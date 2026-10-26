use crate::effects::Effects;
use crate::effects::{AiBackend, AiProvider};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackendStatus {
    pub backend: AiBackend,
    pub warning: Option<String>,
}

pub fn native_available(effects: &Effects) -> bool {
    if effects.ai_provider != AiProvider::Cuda || effects.ai_backend == AiBackend::Onnx {
        return false;
    }
    #[cfg(feature = "ai")]
    {
        crate::nvidia::Installation::discover().is_some()
    }
    #[cfg(not(feature = "ai"))]
    {
        false
    }
}

pub fn command(effects: &Effects) -> Result<(std::process::Command, BackendStatus)> {
    let mut warning = None;
    let backend = if effects.eye_contact || effects.ai_backend == AiBackend::Nvidia {
        AiBackend::Nvidia
    } else if native_available(effects) {
        let output = command_for(effects, AiBackend::Nvidia)?
            .stdin(std::process::Stdio::null())
            .output()?;
        if output.status.success() {
            AiBackend::Nvidia
        } else {
            warning = Some(format!(
                "RTX indisponível; usando ONNX com o provider selecionado: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
            AiBackend::Onnx
        }
    } else {
        AiBackend::Onnx
    };
    Ok((
        command_for(effects, backend)?,
        BackendStatus { backend, warning },
    ))
}

fn command_for(effects: &Effects, backend: AiBackend) -> Result<std::process::Command> {
    let mut effective = effects.clone();
    effective.ai_backend = backend;
    let mut command = std::process::Command::new(std::env::current_exe()?);
    command.args(["--ai-worker", &serde_json::to_string(&effective)?]);
    // The preparer keeps CUDA libraries beside the extracted runtime. Pass their
    // paths only to the native worker; no process-global environment mutation.
    let runtime = std::path::Path::new(&effects.ai_runtime);
    let mut libraries = Vec::new();
    #[cfg(feature = "ai")]
    if backend == AiBackend::Nvidia {
        let installation = if effects.eye_contact {
            crate::nvidia_ar_setup::Installation::load(&crate::nvidia_ar_setup::root()?)?
                .runtime()?
        } else {
            crate::nvidia::Installation::discover()
                .ok_or_else(|| anyhow::anyhow!("Componentes da segmentação RTX não encontrados"))?
        };
        libraries.extend(installation.libraries);
    }
    if let Some(lib) = runtime.parent() {
        libraries.push(lib.to_path_buf());
        if let Some(root) = lib.parent().and_then(|p| p.parent())
            && let Ok(entries) = std::fs::read_dir(root.join("cuda/nvidia"))
        {
            let mut paths: Vec<_> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path().join("lib"))
                .filter(|p| p.is_dir())
                .collect();
            paths.sort();
            libraries.extend(paths);
        }
    }
    if let Some(existing) = std::env::var_os("LD_LIBRARY_PATH") {
        libraries.extend(std::env::split_paths(&existing));
    }
    command.env("LD_LIBRARY_PATH", std::env::join_paths(libraries)?);
    Ok(command)
}

pub fn check(effects: &Effects) -> Result<BackendStatus> {
    effects.preflight()?;
    if !effects.ai_enabled() {
        bail!("Ative um efeito de IA para verificar o processamento");
    }
    let (mut command, status) = command(effects)?;
    let output = command.stdin(std::process::Stdio::null()).output()?;
    if !output.status.success() {
        bail!(
            "IA indisponível: {}{}",
            status
                .warning
                .as_ref()
                .map(|s| format!("{s}; "))
                .unwrap_or_default(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(status)
}

#[cfg(not(feature = "ai"))]
pub fn worker(_effects: &Effects, _size: Option<&str>, _rate: Option<f64>) -> Result<()> {
    bail!("Esta compilação não inclui IA; recompile com --features ai")
}

#[cfg(feature = "ai")]
enum Segmentation {
    Onnx(Box<OnnxSegmentation>),
    Nvidia(Box<crate::nvidia::Segmentation>),
}

#[cfg(feature = "ai")]
impl Segmentation {
    fn open(effects: &Effects) -> Result<Self> {
        if effects.ai_backend == AiBackend::Nvidia
            || (effects.ai_backend == AiBackend::Auto && native_available(effects))
        {
            let installation = crate::nvidia::Installation::discover()
                .ok_or_else(|| anyhow::anyhow!("Componentes RTX não encontrados"))?;
            Ok(Self::Nvidia(Box::new(crate::nvidia::Segmentation::open(
                &installation,
            )?)))
        } else {
            Ok(Self::Onnx(Box::new(OnnxSegmentation::open(effects)?)))
        }
    }

    fn size(&self) -> (usize, usize) {
        match self {
            Self::Onnx(_) => (256, 256),
            Self::Nvidia(_) => (crate::nvidia::WIDTH, crate::nvidia::HEIGHT),
        }
    }

    fn infer(&mut self, rgb: &[u8]) -> Result<&[f32]> {
        match self {
            Self::Onnx(model) => model.infer(rgb),
            Self::Nvidia(_) => bail!("A segmentação RTX requer o processamento GPU"),
        }
    }

    fn gpu_contract(&self, size: [usize; 2]) -> crate::gpu::Contract {
        let (w, h) = self.size();
        let (native, device, mask_shape) = match self {
            Self::Nvidia(model) => (true, model.device, vec![1, 2, h as i64, w as i64]),
            Self::Onnx(model) => (false, 0, model.output_shape.clone()),
        };
        crate::gpu::Contract {
            size,
            mask: [w, h],
            mask_shape,
            native,
            device,
        }
    }

    fn infer_gpu(&mut self, image: &ort::value::DynValue) -> Result<ort::value::DynValue> {
        use anyhow::Context;
        match self {
            Self::Nvidia(model) => model.infer_gpu(image),
            Self::Onnx(model) => {
                let binding = model.gpu.as_mut().context("Sessão CUDA indisponível")?;
                binding.bind_input(&model.input_name, image)?;
                let mut output = binding.run()?;
                output
                    .remove(&model.output_name)
                    .context("Máscara CUDA ausente")
            }
        }
    }

    fn reset(&mut self) -> Result<()> {
        match self {
            Self::Nvidia(model) => model.reset(),
            Self::Onnx(_) => Ok(()),
        }
    }
}

#[cfg(feature = "ai")]
struct OnnxSegmentation {
    session: ort::session::Session,
    image: ort::value::Tensor<f32>,
    input_name: String,
    mask: Vec<f32>,
    output_shape: Vec<i64>,
    output_name: String,
    gpu: Option<ort::io_binding::IoBinding>,
}

#[cfg(feature = "ai")]
impl OnnxSegmentation {
    fn open(effects: &Effects) -> Result<Self> {
        use anyhow::Context;
        use ort::{
            execution_providers::CUDAExecutionProvider,
            session::Session,
            tensor::TensorElementType,
            value::{Tensor, ValueType},
        };
        if !std::path::Path::new(&effects.ai_runtime).is_file() {
            bail!(
                "Biblioteca ONNX Runtime não encontrada: {}",
                effects.ai_runtime
            );
        }
        ort::init_from(&effects.ai_runtime)
            .with_name("Facecam Hub AI")
            .commit()?;
        let mut builder = Session::builder()?.with_intra_threads(2)?;
        if effects.ai_provider == AiProvider::Cuda {
            builder = builder.with_execution_providers([CUDAExecutionProvider::default()
                .build()
                .error_on_failure()])?;
        }
        let session = builder
            .commit_from_file(&effects.ai_model)
            .context("Não foi possível carregar a segmentação ONNX")?;
        let shape = match session.inputs.first().map(|i| &i.input_type) {
            Some(ValueType::Tensor {
                ty: TensorElementType::Float32,
                dimensions,
                ..
            }) => dimensions,
            _ => bail!("O modelo deve receber RGB float32 NCHW 1×3×256×256"),
        };
        if session.inputs.len() != 1 || !matches!(&shape[..], [1 | -1, 3, 256, 256]) {
            bail!("Modelo incompatível: esperado RGB NCHW 1×3×256×256; recebido {shape:?}");
        }
        let input_name = session.inputs[0].name.clone();
        let output_name = session
            .outputs
            .first()
            .context("Modelo ONNX sem saída")?
            .name
            .clone();
        let mut model = Self {
            session,
            input_name,
            image: Tensor::from_array(([1usize, 3, 256, 256], vec![0_f32; 3 * 256 * 256]))?,
            mask: vec![0_f32; 256 * 256],
            output_shape: Vec::new(),
            output_name,
            gpu: None,
        };
        model.infer(&vec![0; 256 * 256 * 3])?;
        if effects.ai_provider == AiProvider::Cuda {
            use ort::memory::{AllocationDevice, AllocatorType, MemoryInfo, MemoryType};
            let mut binding = model.session.create_binding()?;
            let memory = MemoryInfo::new(
                AllocationDevice::CUDA,
                0,
                AllocatorType::Device,
                MemoryType::Default,
            )?;
            binding.bind_output_to_device(&model.output_name, &memory)?;
            model.gpu = Some(binding);
        }
        Ok(model)
    }

    fn infer(&mut self, rgb: &[u8]) -> Result<&[f32]> {
        let (_, tensor) = self.image.try_extract_raw_tensor_mut::<f32>()?;
        for (i, pixel) in rgb.as_chunks::<3>().0.iter().enumerate() {
            for c in 0..3 {
                tensor[c * 256 * 256 + i] = f32::from(pixel[c]) / 255.0;
            }
        }
        let outputs = self
            .session
            .run(vec![(self.input_name.as_str(), self.image.view())])?;
        let (shape, mask) = outputs[0].try_extract_raw_tensor::<f32>()?;
        validate_mask(shape, mask)?;
        self.output_shape = shape.to_vec();
        self.mask.copy_from_slice(mask);
        Ok(&self.mask)
    }
}

#[cfg(feature = "ai")]
pub fn worker(effects: &Effects, size: Option<&str>, rate: Option<f64>) -> Result<()> {
    use anyhow::Context;
    use std::io::{Read, Write};
    effects.preflight()?;
    let mut model = if effects.background_blur {
        Some(if effects.eye_contact {
            let installation =
                crate::nvidia_ar_setup::Installation::load(&crate::nvidia_ar_setup::root()?)?;
            Segmentation::Nvidia(Box::new(crate::nvidia::Segmentation::open(
                &installation.runtime()?,
            )?))
        } else {
            Segmentation::open(effects)?
        })
    } else {
        None
    };
    if effects.eye_contact && model.is_none() {
        let installation =
            crate::nvidia_ar_setup::Installation::load(&crate::nvidia_ar_setup::root()?)?;
        ort::init_from(installation.runtime()?.runtime.to_string_lossy())
            .with_name("Facecam Hub AR")
            .commit()?;
    }
    let (mask_width, mask_height) = model.as_ref().map(|m| m.size()).unwrap_or((368, 208));
    let fps = rate.unwrap_or(f64::from(effects.ai_fps));
    if !fps.is_finite() || fps <= 0.0 || fps > 60.0 {
        bail!("FPS de processamento da IA inválido");
    }
    let (w, h) = if let Some(size) = size {
        let (w, h) = size.split_once('x').context("Resolução IA inválida")?;
        (w.parse::<usize>()?, h.parse::<usize>()?)
    } else {
        (mask_width, mask_height)
    };
    if w == 0 || h == 0 || w > 7680 || h > 4320 {
        bail!("Resolução IA fora dos limites");
    }
    if effects.ai_provider == AiProvider::Cuda {
        return gpu_worker(model.as_mut(), effects, [w, h], fps, size.is_none());
    }
    let mut model = model.context("Ative um efeito de IA para verificar o processamento")?;
    if size.is_none() {
        crate::background::Background::open(effects, w, h, fps)?.frame()?;
        return Ok(());
    }
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    let mut frame = vec![0; w * h * 3];
    let mut background = crate::background::Background::open(effects, w, h, fps)?;
    let mut compositor = Compositor::for_mask(w, h, mask_width, mask_height);
    loop {
        if input.read(&mut frame[..1])? == 0 {
            break;
        }
        input
            .read_exact(&mut frame[1..])
            .context("Frame RGB incompleto")?;
        let small = resize_rgb_to(&frame, w, h, mask_width, mask_height);
        let mask = model.infer(&small)?;
        let background = background.frame()?;
        let blurred = if background.is_none() {
            Some(blur(
                &small,
                mask_width,
                mask_height,
                effects.blur_radius as usize,
            ))
        } else {
            None
        };
        compositor.apply(&mut frame, mask, blurred.as_deref(), background);
        output.write_all(&frame)?;
    }
    Ok(())
}

#[cfg(feature = "ai")]
fn gpu_worker(
    mut model: Option<&mut Segmentation>,
    effects: &Effects,
    [w, h]: [usize; 2],
    fps: f64,
    checking: bool,
) -> Result<()> {
    use anyhow::Context;
    use std::io::{Read, Write};
    let mut background = if effects.background_blur {
        Some(crate::background::Background::open(effects, w, h, fps)?)
    } else {
        None
    };
    let initial = match &background {
        Some(crate::background::Background::Static(rgb)) => Some(rgb.as_slice()),
        _ => None,
    };
    let mut gpu = model
        .as_ref()
        .map(|model| crate::gpu::Processing::open(model.gpu_contract([w, h]), effects, initial))
        .transpose()?;
    let mut framing = if effects.eye_contact {
        Some(crate::nvidia_framing::Framing::open(
            &crate::nvidia_ar_setup::root()?,
            [w, h],
            fps,
            crate::nvidia::ada_device()?,
            true,
        )?)
    } else {
        None
    };
    let mut frame = vec![0; w * h * 3];
    let warm_background = (effects.background_mode == crate::effects::BackgroundMode::Video)
        .then_some(frame.as_slice());
    process_gpu_frame(&mut model, &mut gpu, &mut framing, &frame, warm_background)?;
    if let Some(model) = model.as_mut() {
        model.reset()?;
    }
    if let Some(framing) = framing.as_mut() {
        framing.reset();
    }
    if checking {
        // Validate real image/video decoding as well as the GPU path without
        // advancing the live video's first frame during the worker warm-up.
        if let Some(background) = background.as_mut() {
            background.frame()?;
        }
        return Ok(());
    }
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    loop {
        if input.read(&mut frame[..1])? == 0 {
            break;
        }
        input
            .read_exact(&mut frame[1..])
            .context("Frame RGB incompleto")?;
        let bg = background
            .as_mut()
            .map(|b| b.frame())
            .transpose()?
            .flatten();
        let result = process_gpu_frame(&mut model, &mut gpu, &mut framing, &frame, bg)?;
        let (shape, rgb) = result.try_extract_raw_tensor::<u8>()?;
        if shape != [1, h as i64, w as i64, 3] || rgb.len() != frame.len() {
            bail!("Quadro de composição GPU incompatível");
        }
        output.write_all(rgb)?;
    }
    Ok(())
}

#[cfg(feature = "ai")]
fn process_gpu_frame(
    model: &mut Option<&mut Segmentation>,
    gpu: &mut Option<crate::gpu::Processing>,
    framing: &mut Option<crate::nvidia_framing::Framing>,
    frame: &[u8],
    background: Option<&[u8]>,
) -> Result<ort::value::DynValue> {
    use anyhow::Context;
    let source = framing.as_mut().map(|ar| ar.frame(frame)).transpose()?;
    if let Some(gpu) = gpu.as_mut() {
        let prepared = if let Some(source) = &source {
            gpu.prepare_source(source)?
        } else {
            gpu.prepare(frame)?
        };
        let mask = model
            .as_mut()
            .context("Máscara RTX ausente")?
            .infer_gpu(&prepared.image)?;
        gpu.compose(&prepared, &mask, background)
    } else {
        framing
            .as_mut()
            .context("Ative um efeito de IA para verificar o processamento")?
            .pack(&source.context("Ative um efeito de IA para verificar o processamento")?)
    }
}

#[cfg(feature = "ai")]
fn validate_mask(shape: &[i64], mask: &[f32]) -> Result<()> {
    if !matches!(shape, [1, 1, 256, 256] | [1, 256, 256, 1] | [1, 256, 256])
        || mask.len() != 256 * 256
        || mask
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
    {
        bail!("Saída de segmentação incompatível; esperado alpha 256×256 entre 0 e 1");
    }
    Ok(())
}

#[cfg(feature = "ai")]
fn sample(
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    out_w: usize,
    out_h: usize,
) -> (usize, usize, f32, f32) {
    let sx = ((x as f32 + 0.5) * w as f32 / out_w as f32 - 0.5).clamp(0.0, (w - 1) as f32);
    let sy = ((y as f32 + 0.5) * h as f32 / out_h as f32 - 0.5).clamp(0.0, (h - 1) as f32);
    (sx as usize, sy as usize, sx.fract(), sy.fract())
}

#[cfg(feature = "ai")]
fn interpolate(
    data: &[u8],
    size: (usize, usize),
    point: (usize, usize, f32, f32),
    c: usize,
) -> f32 {
    let (w, h) = size;
    let (x, y, dx, dy) = point;
    let a = data[(y * w + x) * 3 + c] as f32;
    let b = data[(y * w + (x + 1).min(w - 1)) * 3 + c] as f32;
    let d = data[(((y + 1).min(h - 1)) * w + x) * 3 + c] as f32;
    let e = data[(((y + 1).min(h - 1)) * w + (x + 1).min(w - 1)) * 3 + c] as f32;
    (a + (b - a) * dx) * (1.0 - dy) + (d + (e - d) * dx) * dy
}

#[cfg(feature = "ai")]
pub(crate) fn resize_rgb_to(
    frame: &[u8],
    w: usize,
    h: usize,
    width: usize,
    height: usize,
) -> Vec<u8> {
    let mut out = vec![0; width * height * 3];
    for y in 0..height {
        for x in 0..width {
            let (sx, sy, dx, dy) = sample(x, y, w, h, width, height);
            for c in 0..3 {
                out[(y * width + x) * 3 + c] =
                    interpolate(frame, (w, h), (sx, sy, dx, dy), c).round() as u8;
            }
        }
    }
    out
}

#[cfg(feature = "ai")]
pub(crate) fn blur(rgb: &[u8], w: usize, h: usize, radius: usize) -> Vec<u8> {
    let mut tmp = vec![0; rgb.len()];
    let mut out = vec![0; rgb.len()];
    // Two sliding-window passes: O(pixels), independent of the radius.
    blur_pass(rgb, &mut tmp, h, w, w * 3, 3, radius);
    blur_pass(&tmp, &mut out, w, h, 3, w * 3, radius);
    out
}

#[cfg(feature = "ai")]
fn blur_pass(
    input: &[u8],
    output: &mut [u8],
    rows: usize,
    cols: usize,
    stride: usize,
    step: usize,
    radius: usize,
) {
    for row in 0..rows {
        for c in 0..3 {
            let mut sum = 0u32;
            let mut left = 0;
            let mut right = 0;
            for col in 0..cols {
                let end = (col + radius + 1).min(cols);
                while right < end {
                    sum += input[row * stride + right * step + c] as u32;
                    right += 1;
                }
                let start = col.saturating_sub(radius);
                while left < start {
                    sum -= input[row * stride + left * step + c] as u32;
                    left += 1;
                }
                output[row * stride + col * step + c] = (sum / (right - left) as u32) as u8;
            }
        }
    }
}

#[cfg(feature = "ai")]
pub(crate) struct Compositor {
    width: usize,
    mask_width: usize,
    x: Vec<(usize, usize, f32)>,
    y: Vec<(usize, usize, f32)>,
    horizontal: Vec<[f32; 4]>,
}

#[cfg(feature = "ai")]
impl Compositor {
    pub(crate) fn for_mask(
        width: usize,
        height: usize,
        mask_width: usize,
        mask_height: usize,
    ) -> Self {
        let axis = |length, source| {
            (0..length)
                .map(|i| {
                    let (a, _, t, _) = sample(i, 0, source, 1, length, 1);
                    (a, (a + 1).min(source - 1), t)
                })
                .collect()
        };
        Self {
            width,
            mask_width,
            x: axis(width, mask_width),
            y: axis(height, mask_height),
            horizontal: vec![[0.0; 4]; width * mask_height],
        }
    }

    pub(crate) fn apply(
        &mut self,
        frame: &mut [u8],
        mask: &[f32],
        blurred: Option<&[u8]>,
        background: Option<&[u8]>,
    ) {
        // Resample each mask row once. Adjacent output rows reuse these
        // values, avoiding divisions and four source reads per output channel.
        for (sy, row) in self.horizontal.chunks_exact_mut(self.width).enumerate() {
            for (pixel, &(a, b, t)) in row.iter_mut().zip(&self.x) {
                let left = sy * self.mask_width + a;
                let right = sy * self.mask_width + b;
                pixel[0] = mask[left] + (mask[right] - mask[left]) * t;
                if let Some(rgb) = blurred {
                    for c in 0..3 {
                        let l = rgb[left * 3 + c] as f32;
                        pixel[c + 1] = l + (rgb[right * 3 + c] as f32 - l) * t;
                    }
                }
            }
        }
        for (y, row) in frame.chunks_exact_mut(self.width * 3).enumerate() {
            let (a, b, t) = self.y[y];
            let top = &self.horizontal[a * self.width..(a + 1) * self.width];
            let bottom = &self.horizontal[b * self.width..(b + 1) * self.width];
            let static_row =
                background.map(|rgb| &rgb[y * self.width * 3..(y + 1) * self.width * 3]);
            for (x, ((rgb, l), r)) in row
                .as_chunks_mut::<3>()
                .0
                .iter_mut()
                .zip(top)
                .zip(bottom)
                .enumerate()
            {
                let alpha = (l[0] * (1.0 - t) + r[0] * t).clamp(0.0, 1.0);
                for c in 0..3 {
                    let bg = if let Some(background) = static_row {
                        background[x * 3 + c] as f32
                    } else {
                        l[c + 1] * (1.0 - t) + r[c + 1] * t
                    };
                    // All terms are nonnegative. Add-half then truncate avoids
                    // a scalar roundf call on the generic x86-64 target. The
                    // scalar-reference test allows at most one u8 level.
                    rgb[c] = (rgb[c] as f32 * alpha + bg * (1.0 - alpha) + 0.5) as u8;
                }
            }
        }
    }
}

#[cfg(all(test, feature = "ai"))]
fn composite(
    frame: &mut [u8],
    w: usize,
    h: usize,
    mask: &[f32],
    background: &[u8],
    shape: (usize, usize),
) {
    let (mw, mh) = shape;
    for y in 0..h {
        for x in 0..w {
            let (sx, sy, dx, dy) = sample(x, y, mw, mh, w, h);
            let a = mask[sy * mw + sx];
            let b = mask[sy * mw + (sx + 1).min(mw - 1)];
            let d = mask[(sy + 1).min(mh - 1) * mw + sx];
            let e = mask[(sy + 1).min(mh - 1) * mw + (sx + 1).min(mw - 1)];
            let alpha = ((a + (b - a) * dx) * (1.0 - dy) + (d + (e - d) * dx) * dy).clamp(0.0, 1.0);
            for c in 0..3 {
                let i = (y * w + x) * 3 + c;
                let bg = interpolate(background, shape, (sx, sy, dx, dy), c);
                frame[i] = (frame[i] as f32 * alpha + bg * (1.0 - alpha)).round() as u8;
            }
        }
    }
}

#[cfg(all(test, feature = "ai"))]
mod tests {
    use super::*;
    #[test]
    fn alpha_endpoints_preserve_foreground_and_replace_background() {
        let original = vec![250, 30, 20, 10, 240, 50];
        let bg = vec![60; 256 * 256 * 3];
        for alpha in [0.0, 0.5, 1.0] {
            let mut frame = original.clone();
            Compositor::for_mask(2, 1, 256, 256).apply(
                &mut frame,
                &vec![alpha; 256 * 256],
                Some(&bg),
                None,
            );
            for (actual, source) in frame.iter().zip(&original) {
                assert_eq!(
                    *actual,
                    (*source as f32 * alpha + 60.0 * (1.0 - alpha)).round() as u8
                );
            }
        }
    }
    #[test]
    fn cached_composition_matches_scalar_reference_and_static_privacy_background() {
        for (mw, mh) in [(256, 256), (512, 288)] {
            let mask: Vec<_> = (0..mw * mh).map(|i| (i % 101) as f32 / 100.0).collect();
            let bg: Vec<_> = (0..mw * mh * 3).map(|i| (i * 17 % 256) as u8).collect();
            for (w, h) in [(1, 1), (3, 2), (17, 11), (128, 73)] {
                let source: Vec<_> = (0..w * h * 3).map(|i| (i * 11 % 256) as u8).collect();
                let mut expected = source.clone();
                composite(&mut expected, w, h, &mask, &bg, (mw, mh));
                let mut actual = source;
                let mut renderer = Compositor::for_mask(w, h, mw, mh);
                renderer.apply(&mut actual, &mask, Some(&bg), None);
                assert!(actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1));
                let privacy = vec![42; w * h * 3];
                renderer.apply(&mut actual, &vec![0.0; mw * mh], None, Some(&privacy));
                assert_eq!(actual, privacy);
            }
        }
    }
    #[test]
    fn rejects_invalid_masks_and_partial_shapes() {
        for value in [-1.0, 0.0, 0.5, 1.0, 2.0, f32::NAN] {
            let mask = vec![value; 256 * 256];
            assert_eq!(
                validate_mask(&[1, 1, 256, 256], &mask).is_ok(),
                (0.0..=1.0).contains(&value)
            );
        }
        assert!(validate_mask(&[256, 256], &vec![0.0; 256 * 256]).is_err());
    }
    #[test]
    fn blur_keeps_constant_images_and_spreads_an_impulse() {
        for radius in [1, 12, 32] {
            assert_eq!(blur(&[50; 27], 3, 3, radius), vec![50; 27]);
        }
        let mut impulse = vec![0; 75];
        impulse[36..39].fill(255);
        let blurred = blur(&impulse, 5, 5, 1);
        assert_eq!(&blurred[36..39], &[28; 3]);
        assert!(blurred.iter().filter(|&&v| v > 0).count() > 3);
    }
}
