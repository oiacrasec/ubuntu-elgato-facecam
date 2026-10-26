//! Native execution of the locally prepared Ada segmentation model.
use anyhow::{Context, Result, bail};
use ort::{
    execution_providers::{CUDAExecutionProvider, TensorRTExecutionProvider},
    io_binding::IoBinding,
    memory::{AllocationDevice, AllocatorType, MemoryInfo, MemoryType},
    session::Session,
    tensor::TensorElementType,
    value::{DynValue, Tensor, ValueType},
};
use serde::Deserialize;
use std::{io::Read, path::PathBuf, process::Command};

use crate::nvidia_model::STATES;
pub use crate::nvidia_model::{HEIGHT, WIDTH};

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Installation {
    pub format_version: u32,
    pub sdk_version: String,
    pub family: String,
    pub model: PathBuf,
    pub runtime: PathBuf,
    pub libraries: Vec<PathBuf>,
}

impl Installation {
    pub fn discover() -> Option<Self> {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
        let mut roots = Vec::new();
        if let Some(manifest) = std::env::var_os("FACECAM_NVIDIA_MANIFEST") {
            return Self::load(PathBuf::from(manifest).parent()?.to_path_buf()).ok();
        }
        if let Some(data) = data {
            roots.push(data.join("facecam-hub/runtimes/linux-x86_64/vfx/0.7.6/ada"));
        }
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            roots.push(dir.join("nvidia-vfx"));
            if let Some(parent) = dir.parent() {
                roots.push(parent.join("ai-runtime/nvidia-vfx"));
            }
        }
        roots.into_iter().find_map(|root| Self::load(root).ok())
    }

    pub fn load(root: PathBuf) -> Result<Self> {
        let file = std::fs::File::open(root.join("manifest.json"))?;
        let mut text = String::new();
        file.take(65537).read_to_string(&mut text)?;
        if text.len() > 65536 {
            bail!("Manifesto RTX excede o limite de 64 KiB");
        }
        let mut install: Self = serde_json::from_str(&text)?;
        if install.format_version != 1 || install.sdk_version != "0.7.6" || install.family != "ada"
        {
            bail!("Versão de componentes RTX incompatível");
        }
        install.model = root.join(install.model);
        install.runtime = root.join(install.runtime);
        install.libraries = install
            .libraries
            .into_iter()
            .map(|p| root.join(p))
            .collect();
        if !(install.model.is_file()
            && install.runtime.is_file()
            && install.libraries.iter().all(|p| p.is_dir()))
        {
            bail!("Componentes RTX incompletos");
        }
        Ok(install)
    }
}

pub fn ada_device() -> Result<i32> {
    if std::env::consts::ARCH != "x86_64" {
        bail!("A combinação RTX atual requer Linux x86-64");
    }
    let gpu = Command::new("nvidia-smi")
        .args(["--query-gpu=compute_cap", "--format=csv,noheader"])
        .output()
        .context("Não foi possível consultar a GPU NVIDIA")?;
    let device = String::from_utf8_lossy(&gpu.stdout).lines().position(|s| s.trim() == "8.9")
        .filter(|_| gpu.status.success()).context("A preparação atual requer uma GPU Ada (RTX 40); ONNX continua disponível para outras GPUs")?;
    i32::try_from(device).context("Índice de GPU inválido")
}

pub struct Segmentation {
    binding: IoBinding,
    gpu_memory: MemoryInfo,
    pub device: i32,
}

impl Segmentation {
    pub fn open(installation: &Installation) -> Result<Self> {
        let device = ada_device()?;
        ort::init_from(installation.runtime.to_string_lossy())
            .with_name("Facecam Hub RTX")
            .commit()?;
        let session = Session::builder()?
            .with_execution_providers([
                TensorRTExecutionProvider::default()
                    .with_device_id(device)
                    .build()
                    .error_on_failure(),
                CUDAExecutionProvider::default()
                    .with_device_id(device)
                    .build()
                    .error_on_failure(),
            ])?
            .commit_from_file(&installation.model)
            .context("Não foi possível carregar a segmentação RTX")?;
        let expected_inputs = std::iter::once(("input_1", [1, 3, HEIGHT, WIDTH]))
            .chain(STATES.iter().map(|&(i, _, s)| (i, s)));
        if session.inputs.len() != 7 || session.outputs.len() != 7 {
            bail!("Contrato RTX incompatível: esperadas sete entradas e sete saídas");
        }
        for (name, shape) in expected_inputs {
            let input = session
                .inputs
                .iter()
                .find(|i| i.name == name)
                .context("Entrada RTX ausente")?;
            verify_tensor(&input.input_type, shape)?;
        }
        for (name, shape) in std::iter::once(("output_1", [1, 2, HEIGHT, WIDTH]))
            .chain(STATES.iter().map(|&(_, o, s)| (o, s)))
        {
            let output = session
                .outputs
                .iter()
                .find(|o| o.name == name)
                .context("Saída RTX ausente")?;
            verify_tensor(&output.output_type, shape)?;
        }
        let binding = session.create_binding()?;
        let gpu_memory = MemoryInfo::new(
            AllocationDevice::CUDA,
            device,
            AllocatorType::Device,
            MemoryType::Default,
        )?;
        let mut result = Self {
            binding,
            gpu_memory,
            device,
        };
        result.bind_outputs()?;
        result.reset()?;
        Ok(result)
    }

    pub fn reset(&mut self) -> Result<()> {
        for &(input, _, shape) in &STATES {
            let state = Tensor::from_array((shape, vec![0_f32; shape.iter().product()]))?;
            self.binding.bind_input(input, &state)?;
        }
        Ok(())
    }

    fn bind_outputs(&mut self) -> Result<()> {
        self.binding.clear_outputs();
        for &(_, output, _) in &STATES {
            self.binding
                .bind_output_to_device(output, &self.gpu_memory)?;
        }
        self.binding
            .bind_output_to_device("output_1", &self.gpu_memory)?;
        Ok(())
    }

    pub fn infer_gpu(&mut self, image: &DynValue) -> Result<DynValue> {
        self.binding.bind_input("input_1", image)?;
        let mut outputs = self.binding.run()?;
        let alpha = outputs.remove("output_1").context("Máscara RTX ausente")?;
        let mut states = Vec::with_capacity(STATES.len());
        for &(input, output, _) in &STATES {
            states.push((
                input,
                outputs
                    .remove(output)
                    .context("Estado temporal RTX ausente")?,
            ));
        }
        drop(outputs);
        for (name, state) in states {
            self.binding.bind_input(name, &state)?;
        }
        // Keep the previous GPU outputs alive as inputs, then allocate distinct
        // outputs. Reusing the same output buffers would alias recurrent state.
        self.bind_outputs()?;
        Ok(alpha)
    }
}

fn verify_tensor(ty: &ValueType, expected: [usize; 4]) -> Result<()> {
    match ty {
        ValueType::Tensor {
            ty: TensorElementType::Float32,
            dimensions,
            ..
        } if dimensions.iter().copied().eq(expected.map(|n| n as i64)) => Ok(()),
        _ => bail!("Tensor RTX incompatível: esperado float32 {expected:?}, recebido {ty:?}"),
    }
}

#[cfg(test)]
fn foreground_mask(shape: &[i64], values: &[f32], out: &mut [f32]) -> Result<()> {
    let pixels = WIDTH * HEIGHT;
    if shape != [1, 2, HEIGHT as i64, WIDTH as i64]
        || values.len() != pixels * 2
        || out.len() != pixels
        || values
            .iter()
            .any(|v| !v.is_finite() || !(-0.0001..=1.0001).contains(v))
    {
        bail!("Máscara RTX inválida: esperadas duas probabilidades por pixel");
    }
    for (alpha, &value) in out.iter_mut().zip(&values[pixels..]) {
        *alpha = value.clamp(0.0, 1.0);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installation_rejects_missing_files_unknown_versions_and_oversized_manifests() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = dir.path().join("manifest.json");
        let mut data = serde_json::json!({"format_version":1,"sdk_version":"0.7.6","family":"ada","model":"segmentation.ctx.onnx","runtime":"libonnxruntime.so","libraries":["lib"]});
        std::fs::write(&manifest, data.to_string()).unwrap();
        assert!(Installation::load(dir.path().into()).is_err());
        std::fs::write(dir.path().join("segmentation.ctx.onnx"), []).unwrap();
        std::fs::write(dir.path().join("libonnxruntime.so"), []).unwrap();
        std::fs::create_dir(dir.path().join("lib")).unwrap();
        let installation = Installation::load(dir.path().into()).unwrap();
        assert_eq!(installation.model, dir.path().join("segmentation.ctx.onnx"));
        for (field, value) in [
            ("format_version", serde_json::json!(0)),
            ("sdk_version", serde_json::json!("0.8")),
            ("family", serde_json::json!("ampere")),
        ] {
            let mut bad = data.clone();
            bad[field] = value;
            std::fs::write(&manifest, bad.to_string()).unwrap();
            assert!(Installation::load(dir.path().into()).is_err());
        }
        data["unknown"] = serde_json::json!(true);
        std::fs::write(&manifest, data.to_string()).unwrap();
        assert!(Installation::load(dir.path().into()).is_err());
        std::fs::write(&manifest, vec![b' '; 65537]).unwrap();
        assert!(Installation::load(dir.path().into()).is_err());
    }
    #[test]
    fn foreground_uses_second_plane_and_rejects_invalid_probabilities() {
        let mut values = vec![0_f32; WIDTH * HEIGHT * 2];
        let mut out = vec![0_f32; WIDTH * HEIGHT];
        let shape = [1, 2, HEIGHT as i64, WIDTH as i64];
        for v in [0.0, 0.25, 1.0] {
            values[WIDTH * HEIGHT] = v;
            foreground_mask(&shape, &values, &mut out).unwrap();
            assert_eq!(out[0], v);
        }
        for v in [-1.0, 1.1, f32::NAN, f32::INFINITY] {
            values[WIDTH * HEIGHT] = v;
            assert!(foreground_mask(&shape, &values, &mut out).is_err());
        }
        assert!(foreground_mask(&[1, 1, HEIGHT as i64, WIDTH as i64], &values, &mut out).is_err());
        assert!(foreground_mask(&shape, &values[..10], &mut out).is_err());
    }
}
