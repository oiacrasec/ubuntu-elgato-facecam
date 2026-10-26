//! Persistent, user-local AR contracts. This does not enable AR effects in video.
use crate::{
    nvidia::Installation as Runtime,
    nvidia_ar_model::{MODELS, SDK_SHA, SDK_URL},
    nvidia_setup::{Work, checksum, fetch, publish, user_root},
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

const MARKER: &str = ".facecam-hub-ar";
const LICENSES: [&str; 2] = ["NVIDIA Maxine EULA.pdf", "ThirdPartyLicenses.txt"];

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Installation {
    format_version: u32,
    sdk_version: String,
    family: String,
    package_sha256: String,
    runtime_root: PathBuf,
}

pub fn root() -> Result<PathBuf> {
    Ok(user_root("XDG_DATA_HOME", ".local/share")?
        .join("facecam-hub/runtimes/linux-x86_64/ar/0.8.7/ada"))
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("Componentes AR excedem o limite de leitura");
    }
    Ok(bytes)
}

impl Installation {
    fn new(runtime_root: PathBuf) -> Self {
        Self {
            format_version: 1,
            sdk_version: "0.8.7".into(),
            family: "ada".into(),
            package_sha256: SDK_SHA.into(),
            runtime_root,
        }
    }

    /// Cheap discovery validates metadata and descriptors without loading native libraries.
    pub fn load(root: &Path) -> Result<Self> {
        let installation: Self =
            serde_json::from_slice(&read_bounded(&root.join("manifest.json"), 65536)?)?;
        if installation.format_version != 1
            || installation.sdk_version != "0.8.7"
            || installation.family != "ada"
            || installation.package_sha256 != SDK_SHA
            || !installation.runtime_root.is_absolute()
            || read_bounded(&root.join(MARKER), 16)? != b"1\n"
        {
            bail!("Manifesto AR incompatível; prepare os componentes AR novamente");
        }
        installation.runtime()?;
        for model in MODELS {
            let file = fs::metadata(root.join(format!("{}.engine", model.id)))?;
            if !file.is_file()
                || file.len() == 0
                || file.len() > 64 * 1024 * 1024
                || read_bounded(&root.join(format!("{}.onnx", model.id)), 2048)?
                    != model.descriptor()
            {
                bail!("Componentes AR incompatíveis; prepare os componentes AR novamente");
            }
        }
        for license in LICENSES {
            let file = fs::metadata(root.join(license))?;
            if !file.is_file() || file.len() == 0 || file.len() > 8 * 1024 * 1024 {
                bail!("Licenças AR incompletas; prepare os componentes AR novamente");
            }
        }
        Ok(installation)
    }

    pub fn runtime(&self) -> Result<Runtime> {
        Runtime::load(self.runtime_root.clone())
            .context("Prepare a IA RTX antes de preparar ou verificar os componentes AR")
    }

    pub fn verify_engines(&self, root: &Path) -> Result<()> {
        for model in MODELS {
            if checksum(&root.join(format!("{}.engine", model.id)))? != model.native_sha {
                bail!("Componentes AR incompatíveis; prepare os componentes AR novamente");
            }
        }
        Ok(())
    }
}

pub fn available() -> bool {
    root().is_ok_and(|p| Installation::load(&p).is_ok())
}

pub(crate) fn prepare(work: &mut Work<'_>) -> Result<String> {
    crate::nvidia::ada_device()?;
    let runtime = Runtime::discover()
        .context("Prepare a IA RTX antes de preparar ou verificar os componentes AR")?;
    // Resolved VFX paths identify the exact native runtime reused by this preparation.
    let runtime_root = runtime
        .model
        .parent()
        .context("Destino RTX inválido")?
        .canonicalize()?;
    let root = root()?;
    let parent = root.parent().context("Destino RTX inválido")?;
    fs::create_dir_all(parent)?;
    let stage = tempfile::Builder::new()
        .prefix(".ada-ar-preparing-")
        .tempdir_in(parent)?;
    let cache = user_root("XDG_CACHE_HOME", ".cache")?.join("facecam-hub/nvidia-ar-0.8.7-ada");
    fs::create_dir_all(&cache)?;
    let installer = cache.join("nvidia_ar_sdk_installer_v0.8.7_ada.exe");
    work.phase("Conferindo o pacote AR 0.8.7 Ada")?;
    if !checksum(&installer).is_ok_and(|s| s == SDK_SHA)
        && let Ok(source) = crate::nvidia_ar::installer()
        && source != installer
    {
        fs::copy(source, &installer)?;
    }
    fetch(work, SDK_URL, &installer, SDK_SHA, false)?;
    let sdk = stage.path().join("sdk");
    fs::create_dir(&sdk)?;
    work.phase("Importando os quatro contratos AR para o usuário")?;
    let mut args = vec![
        "x".into(),
        "-y".into(),
        format!("-o{}", sdk.display()).into(),
        installer.into_os_string(),
        "nvARPose.dll".into(),
    ];
    for model in MODELS {
        args.push(format!("models/{}", model.source).into());
    }
    args.extend(LICENSES.map(Into::into));
    work.run("7z", &args)?;
    crate::nvidia_ar::import_models(&sdk, stage.path())?;
    for license in LICENSES {
        let bytes = read_bounded(&sdk.join(license), 8 * 1024 * 1024)?;
        if bytes.is_empty() {
            bail!("Licenças AR incompletas; prepare os componentes AR novamente");
        }
        fs::write(stage.path().join(license), bytes)?;
    }
    fs::remove_dir_all(&sdk)?;
    fs::write(
        stage.path().join("manifest.json"),
        serde_json::to_vec_pretty(&Installation::new(runtime_root))?,
    )?;
    fs::write(stage.path().join(MARKER), b"1\n")?;
    let prepared = Installation::load(stage.path())?;
    work.phase("Validando os quatro contratos AR na GPU")?;
    let executable = format!("/proc/{}/exe", std::process::id());
    let status = work
        .command(&executable)?
        .arg("--ar-worker")
        .arg(stage.path())
        .env(
            "LD_LIBRARY_PATH",
            std::env::join_paths(prepared.runtime()?.libraries)?,
        )
        .env("CUDA_MODULE_LOADING", "LAZY")
        .status()?;
    if !status.success() {
        bail!("A validação AR falhou; os componentes anteriores foram preservados");
    }
    publish(stage.path(), &root, MARKER)?;
    Ok("Contratos AR preparados e validados; efeitos no vídeo ainda em implementação".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(root: &Path) {
        let runtime = root.join("runtime");
        fs::create_dir(&runtime).unwrap();
        fs::write(runtime.join("model.onnx"), []).unwrap();
        fs::write(runtime.join("runtime.so"), []).unwrap();
        fs::write(
            runtime.join("manifest.json"),
            serde_json::json!({"format_version":1,"sdk_version":"0.7.6","family":"ada",
                "model":"model.onnx","runtime":"runtime.so","libraries":["."]})
            .to_string(),
        )
        .unwrap();
        fs::write(root.join(MARKER), b"1\n").unwrap();
        fs::write(
            root.join("manifest.json"),
            serde_json::to_vec(&Installation::new(runtime)).unwrap(),
        )
        .unwrap();
        for model in MODELS {
            fs::write(root.join(format!("{}.engine", model.id)), b"fixture").unwrap();
            fs::write(root.join(format!("{}.onnx", model.id)), model.descriptor()).unwrap();
        }
        for license in LICENSES {
            fs::write(root.join(license), b"fixture").unwrap();
        }
    }

    #[test]
    fn manifest_checks_versions_limits_descriptors_licenses_and_engine_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        assert!(Installation::load(root).is_err());
        fixture(root);
        let bytes = fs::read(root.join("manifest.json")).unwrap();
        let valid = Installation::load(root).unwrap();
        assert!(valid.verify_engines(root).is_err());
        for (field, value) in [
            ("format_version", serde_json::json!(2)),
            ("sdk_version", serde_json::json!("0.8.2")),
            ("family", serde_json::json!("ampere")),
            ("package_sha256", serde_json::json!("invalid")),
            ("runtime_root", serde_json::json!("relative")),
            ("unknown", serde_json::json!(true)),
        ] {
            let mut manifest: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            manifest[field] = value;
            fs::write(root.join("manifest.json"), manifest.to_string()).unwrap();
            assert!(Installation::load(root).is_err(), "{field}");
        }
        for data in [b"{".to_vec(), vec![b' '; 65537]] {
            fs::write(root.join("manifest.json"), data).unwrap();
            assert!(Installation::load(root).is_err());
        }
        fs::write(root.join("manifest.json"), bytes).unwrap();
        fs::write(root.join("face.onnx"), b"invalid").unwrap();
        assert!(Installation::load(root).is_err());
        fs::write(root.join("face.onnx"), MODELS[0].descriptor()).unwrap();
        fs::remove_file(root.join(LICENSES[0])).unwrap();
        assert!(Installation::load(root).is_err());
    }

    #[test]
    fn publication_preserves_unowned_paths_and_rolls_back_failed_rename() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("ada");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("previous"), b"preserved").unwrap();
        let stage = dir.path().join("stage");
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join(MARKER), b"1\n").unwrap();
        assert!(publish(&stage, &root, MARKER).is_err());
        assert!(root.join("previous").is_file());
        fs::write(root.join(MARKER), b"1\n").unwrap();
        assert!(publish(&dir.path().join("missing"), &root, MARKER).is_err());
        assert!(root.join("previous").is_file());
        publish(&stage, &root, MARKER).unwrap();
        assert!(!root.join("previous").exists());
        assert!(root.join(MARKER).is_file());
        assert!(!stage.exists());
    }
}
