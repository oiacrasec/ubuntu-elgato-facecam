use crate::effects::Effects;
use anyhow::{Result, bail};

pub fn command(effects: &Effects) -> Result<std::process::Command> {
    let mut command = std::process::Command::new(std::env::current_exe()?);
    command.args(["--ai-worker", &serde_json::to_string(effects)?]);
    // The preparer keeps CUDA libraries beside the extracted runtime. Pass their
    // paths only to the native worker; no process-global environment mutation.
    let runtime = std::path::Path::new(&effects.ai_runtime);
    let mut libraries = Vec::new();
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

pub fn check(effects: &Effects) -> Result<()> {
    effects.preflight()?;
    if !effects.background_blur {
        bail!("Ative background_blur para verificar a IA");
    }
    let output = command(effects)?
        .stdin(std::process::Stdio::null())
        .output()?;
    if !output.status.success() {
        bail!(
            "IA indisponível: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

#[cfg(not(feature = "ai"))]
pub fn worker(_effects: &Effects, _size: Option<&str>) -> Result<()> {
    bail!("Esta compilação não inclui IA; recompile com --features ai")
}

#[cfg(feature = "ai")]
pub fn worker(effects: &Effects, size: Option<&str>) -> Result<()> {
    use crate::effects::AiProvider;
    use anyhow::Context;
    use ort::{
        execution_providers::CUDAExecutionProvider,
        session::Session,
        tensor::TensorElementType,
        value::{Tensor, ValueType},
    };
    use std::io::{Read, Write};

    effects.preflight()?;
    if !std::path::Path::new(&effects.ai_runtime).is_file() {
        bail!(
            "Biblioteca ONNX Runtime não encontrada: {}",
            effects.ai_runtime
        );
    }
    // This process isolates native runtime failures from the controller and GUI.
    ort::init_from(&effects.ai_runtime)
        .with_name("Facecam Hub AI")
        .commit()?;
    let mut builder = Session::builder()?.with_intra_threads(2)?;
    if effects.ai_provider == AiProvider::Cuda {
        builder = builder.with_execution_providers([CUDAExecutionProvider::default()
            .build()
            .error_on_failure()])?;
    }
    let mut session = builder
        .commit_from_file(&effects.ai_model)
        .context("Não foi possível carregar a segmentação ONNX")?;
    let shape = match session.inputs.first().map(|i| &i.input_type) {
        Some(ValueType::Tensor {
            ty: TensorElementType::Float32,
            shape,
            ..
        }) => shape,
        _ => bail!("O modelo deve receber RGB float32 NCHW 1×3×256×256"),
    };
    if session.inputs.len() != 1 || !matches!(&shape[..], [1 | -1, 3, 256, 256]) {
        bail!("Modelo incompatível: esperado RGB NCHW 1×3×256×256; recebido {shape:?}");
    }
    // Warm up both providers and verify the output, including in --check-ai.
    let warm = Tensor::from_array(([1usize, 3, 256, 256], vec![0.0_f32; 3 * 256 * 256]))?;
    let outputs = session.run(ort::inputs![warm])?;
    let (shape, mask) = outputs[0].try_extract_tensor::<f32>()?;
    validate_mask(shape, mask)?;
    drop(outputs);
    let Some(size) = size else {
        return Ok(());
    };
    let (w, h) = size.split_once('x').context("Resolução IA inválida")?;
    let (w, h) = (w.parse::<usize>()?, h.parse::<usize>()?);
    if w == 0 || h == 0 || w > 7680 || h > 4320 {
        bail!("Resolução IA fora dos limites");
    }
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    let mut frame = vec![0; w * h * 3];
    loop {
        // A partial frame is an error; a clean EOF ends the owned worker.
        if input.read(&mut frame[..1])? == 0 {
            break;
        }
        input
            .read_exact(&mut frame[1..])
            .context("Frame RGB incompleto")?;
        let small = resize_rgb(&frame, w, h);
        let mut tensor = vec![0.0_f32; 3 * 256 * 256];
        for (i, rgb) in small.as_chunks::<3>().0.iter().enumerate() {
            for c in 0..3 {
                tensor[c * 256 * 256 + i] = rgb[c] as f32 / 255.0;
            }
        }
        let value = Tensor::from_array(([1usize, 3, 256, 256], tensor))?;
        let outputs = session.run(ort::inputs![value])?;
        let (shape, mask) = outputs[0].try_extract_tensor::<f32>()?;
        validate_mask(shape, mask)?;
        let blurred = blur(&small, 256, 256, effects.blur_radius as usize);
        composite(&mut frame, w, h, mask, &blurred);
        output.write_all(&frame)?;
    }
    Ok(())
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
fn resize_rgb(frame: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = vec![0; 256 * 256 * 3];
    for y in 0..256 {
        for x in 0..256 {
            let (sx, sy, dx, dy) = sample(x, y, w, h, 256, 256);
            for c in 0..3 {
                out[(y * 256 + x) * 3 + c] =
                    interpolate(frame, (w, h), (sx, sy, dx, dy), c).round() as u8;
            }
        }
    }
    out
}

#[cfg(feature = "ai")]
fn blur(rgb: &[u8], w: usize, h: usize, radius: usize) -> Vec<u8> {
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
fn composite(frame: &mut [u8], w: usize, h: usize, mask: &[f32], background: &[u8]) {
    for y in 0..h {
        for x in 0..w {
            let (sx, sy, dx, dy) = sample(x, y, 256, 256, w, h);
            let a = mask[sy * 256 + sx];
            let b = mask[sy * 256 + (sx + 1).min(255)];
            let d = mask[(sy + 1).min(255) * 256 + sx];
            let e = mask[(sy + 1).min(255) * 256 + (sx + 1).min(255)];
            let alpha = ((a + (b - a) * dx) * (1.0 - dy) + (d + (e - d) * dx) * dy).clamp(0.0, 1.0);
            for c in 0..3 {
                let i = (y * w + x) * 3 + c;
                let bg = interpolate(background, (256, 256), (sx, sy, dx, dy), c);
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
            composite(&mut frame, 2, 1, &vec![alpha; 256 * 256], &bg);
            for (actual, source) in frame.iter().zip(&original) {
                assert_eq!(
                    *actual,
                    (*source as f32 * alpha + 60.0 * (1.0 - alpha)).round() as u8
                );
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
