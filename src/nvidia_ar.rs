//! Native AR tensor validation. This is not yet a live gaze-correction pipeline.
use crate::nvidia_ar_model::{DLL_SHA, MODELS, SDK_SHA, SDK_URL};
use anyhow::{Context, Result, bail};
use ort::{
    execution_providers::{CUDAExecutionProvider, TensorRTExecutionProvider},
    session::Session,
    tensor::TensorElementType,
    value::{Tensor, ValueType},
};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn digest_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
fn installer() -> Result<PathBuf> {
    let name = "nvidia_ar_sdk_installer_v0.8.7_ada.exe";
    let mut candidates = Vec::new();
    if let Some(cache) = std::env::var_os("XDG_CACHE_HOME") {
        candidates.push(
            PathBuf::from(cache)
                .join("facecam-hub/nvidia-ar-0.8.7-ada")
                .join(name),
        );
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.push(
            home.join(".cache/facecam-hub/nvidia-ar-0.8.7-ada")
                .join(name),
        );
        for dir in ["Downloads/NVIDEA", "Downloads/NVIDIA", "Downloads"] {
            candidates.push(home.join(dir).join(name));
        }
    }
    for file in candidates {
        if file.is_file() && digest_file(&file)? == SDK_SHA {
            return Ok(file);
        }
    }
    bail!("Pacote AR 0.8.7 Ada não encontrado. Origem: {SDK_URL}")
}

pub fn check() -> Result<()> {
    crate::nvidia::ada_device()?;
    let installation = crate::nvidia::Installation::discover()
        .context("Prepare os componentes RTX antes de verificar os modelos AR")?;
    let installer = installer()?;
    let stage = tempfile::Builder::new()
        .prefix("facecam-ar-check-")
        .tempdir()?;
    let sdk = stage.path().join("sdk");
    std::fs::create_dir(&sdk)?;
    let mut command = Command::new("7z");
    command
        .args(["x", "-y"])
        .arg(format!("-o{}", sdk.display()))
        .arg(installer)
        .arg("nvARPose.dll");
    for model in MODELS {
        command.arg(format!("models/{}", model.source));
    }
    let output = command
        .stdin(Stdio::null())
        .output()
        .context("Não foi possível extrair os modelos AR; instale as ferramentas de IA RTX")?;
    if !output.status.success() {
        bail!("A extração dos modelos AR falhou");
    }
    let dll_path = sdk.join("nvARPose.dll");
    if std::fs::metadata(&dll_path)?.len() > 8 * 1024 * 1024 {
        bail!("Runtime AR excede o limite");
    }
    let dll = std::fs::read(dll_path)?;
    if crate::nvidia_model::digest(&dll) != DLL_SHA {
        bail!("Runtime AR não corresponde à referência validada");
    }
    for model in MODELS {
        let path = sdk.join("models").join(model.source);
        if std::fs::metadata(&path)?.len() > 64 * 1024 * 1024 {
            bail!("Modelo AR excede o limite");
        }
        let engine = model.import(&dll, &std::fs::read(path)?)?;
        std::fs::write(stage.path().join(format!("{}.engine", model.id)), engine)?;
        std::fs::write(
            stage.path().join(format!("{}.onnx", model.id)),
            model.descriptor(),
        )?;
    }
    // Match the already validated ORT/TensorRT/CUDA combination in an isolated child.
    let mut child = Command::new(std::env::current_exe()?);
    let output = child
        .arg("--ar-worker")
        .arg(stage.path())
        .env(
            "LD_LIBRARY_PATH",
            std::env::join_paths(&installation.libraries)?,
        )
        .env("CUDA_MODULE_LOADING", "LAZY")
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        bail!("Validação AR recusada: {error}");
    }
    Ok(())
}

pub fn worker(root: &Path) -> Result<()> {
    let device = crate::nvidia::ada_device()?;
    let install =
        crate::nvidia::Installation::discover().context("Componentes RTX indisponíveis")?;
    ort::init_from(install.runtime.to_string_lossy())
        .with_name("Facecam Hub AR")
        .commit()?;
    let mut encoded: BTreeMap<String, Vec<f32>> = BTreeMap::new();
    for model in MODELS {
        let session = Session::builder()?
            .with_intra_threads(1)?
            .with_config_entry("session.disable_cpu_ep_fallback", "1")?
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
            .commit_from_file(root.join(format!("{}.onnx", model.id)))?;
        if session.inputs.len() != model.inputs.len()
            || session.outputs.len() != model.outputs.len()
        {
            bail!("Contrato AR incompatível: {}", model.id);
        }
        for spec in model.inputs {
            let input = session
                .inputs
                .iter()
                .find(|i| i.name == spec.name)
                .context("Entrada AR ausente")?;
            verify(&input.input_type, spec.shape)?;
        }
        for spec in model.outputs {
            let output = session
                .outputs
                .iter()
                .find(|o| o.name == spec.name)
                .context("Saída AR ausente")?;
            verify(&output.output_type, spec.shape)?;
        }
        let host = model
            .inputs
            .iter()
            .map(|spec| {
                let data = if model.id == "decoder" && spec.name != "gaze_por" {
                    encoded
                        .remove(spec.name)
                        .context("Saída do encoder AR ausente")?
                } else {
                    vec![0_f32; spec.shape.iter().product()]
                };
                Ok((spec.name, Tensor::from_array((spec.shape.to_vec(), data))?))
            })
            .collect::<Result<Vec<_>>>()?;
        let output = session.run(
            host.iter()
                .map(|(name, tensor)| (*name, tensor.view()))
                .collect::<Vec<_>>(),
        )?;
        for spec in model.outputs {
            let value = output.get(spec.name).context("Tensor AR ausente")?;
            let (shape, values) = value.try_extract_raw_tensor::<f32>()?;
            if shape
                .iter()
                .copied()
                .ne(spec.shape.iter().map(|&n| n as i64))
                || values.iter().any(|v| !v.is_finite())
            {
                bail!("Saída AR inválida: {}", spec.name);
            }
            if model.id == "encoder" {
                encoded.insert(spec.name.into(), values.to_vec());
            }
        }
    }
    Ok(())
}
fn verify(value: &ValueType, expected: &[usize]) -> Result<()> {
    if let ValueType::Tensor {
        ty: TensorElementType::Float32,
        dimensions,
        ..
    } = value
        && dimensions
            .iter()
            .copied()
            .eq(expected.iter().map(|&n| n as i64))
    {
        return Ok(());
    }
    bail!("Tipo ou forma do tensor AR incompatível")
}
