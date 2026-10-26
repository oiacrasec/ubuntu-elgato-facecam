//! Explicit, user-local preparation. Downloads never install or replace system packages.
use crate::{effects::Effects, nvidia::Installation, nvidia_model};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
};

const ORT: &str = "onnxruntime-linux-x64-gpu-1.20.1";
const ORT_SHA: &str = "6bfb87c6ebe55367a94509b8ef062239e188dccf8d5caac8d6909b2344893bf0";
const COMPUTE: [(&str, &str); 3] = [
    (
        "libnvinfer10",
        "3c59c3f185e32298f3c9394cfac5ad71457931a37af0ae4dd94c9ace5caf4b31",
    ),
    (
        "libnvinfer-plugin10",
        "a61576e0efa55e55dc3fe530ccc65be3f4f286c62314a828f22ef93980c1767d",
    ),
    (
        "libnvonnxparsers10",
        "a0fa02db1e9e4b421b4622b6b71433f7b51b272d5810bb7ee3d2c8438c5cb9e6",
    ),
];

pub fn missing_tools() -> bool {
    use std::os::unix::fs::PermissionsExt;
    let paths: Vec<_> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    ["7z", "curl", "unzip"].iter().any(|name| {
        !paths.iter().any(|dir| {
            fs::metadata(dir.join(name))
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
    })
}

pub enum Update {
    Progress(String),
    Finished(std::result::Result<String, String>),
}
pub struct Preparation {
    events: Receiver<Update>,
}
impl Preparation {
    pub fn start(effects: Effects, log: PathBuf) -> Result<Self> {
        if let Some(parent) = log.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(log)?;
        let (tx, events) = mpsc::channel();
        std::thread::spawn(move || {
            let mut work = Work {
                log: file,
                events: &tx,
            };
            let result = prepare(&effects, &mut work).map_err(|e| format!("{e:#}"));
            let _ = tx.send(Update::Finished(result));
        });
        Ok(Self { events })
    }
    pub fn poll(&self) -> Option<Update> {
        match self.events.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Update::Finished(Err(
                "A preparação RTX foi interrompida".into(),
            ))),
        }
    }
}
struct Work<'a> {
    log: File,
    events: &'a Sender<Update>,
}
impl Work<'_> {
    fn phase(&mut self, text: &str) -> Result<()> {
        writeln!(self.log, "{text}")?;
        let _ = self.events.send(Update::Progress(text.into()));
        Ok(())
    }
    fn command(&self, program: &str) -> Result<Command> {
        let mut command = Command::new("/usr/bin/setpriv");
        command.args(["--pdeathsig", "KILL", "--", program]);
        command
            .stdin(Stdio::null())
            .stdout(self.log.try_clone()?)
            .stderr(self.log.try_clone()?);
        Ok(command)
    }
    fn run(&self, program: &str, args: &[OsString]) -> Result<()> {
        let status = self
            .command(program)?
            .args(args)
            .status()
            .with_context(|| format!("Não foi possível iniciar {program}"))?;
        if !status.success() {
            bail!("{program} falhou ({status}); consulte o log da preparação RTX");
        }
        Ok(())
    }
}
fn user_root(var: &str, suffix: &str) -> Result<PathBuf> {
    std::env::var_os(var)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(suffix)))
        .context("Diretório do usuário indisponível")
}
pub fn root() -> Result<PathBuf> {
    Ok(user_root("XDG_DATA_HOME", ".local/share")?
        .join("facecam-hub/runtimes/linux-x86_64/vfx/0.7.6/ada"))
}
fn checksum(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn fetch(work: &Work<'_>, url: &str, path: &Path, sha: &str, apt: bool) -> Result<()> {
    if checksum(path).is_ok_and(|s| s == sha) {
        return Ok(());
    }
    let part = path.with_extension("part");
    if apt {
        work.run(
            "/usr/lib/apt/apt-helper",
            &[
                "download-file".into(),
                url.into(),
                part.as_os_str().into(),
                format!("SHA256:{sha}").into(),
            ],
        )?;
    } else {
        work.run(
            "curl",
            &[
                "--fail".into(),
                "--location".into(),
                "--retry".into(),
                "2".into(),
                "--connect-timeout".into(),
                "20".into(),
                "--max-time".into(),
                "900".into(),
                "--output".into(),
                part.as_os_str().into(),
                "--".into(),
                url.into(),
            ],
        )?;
    }
    if checksum(&part)? != sha {
        bail!(
            "O download não passou na conferência SHA-256: {}",
            path.display()
        );
    }
    fs::rename(part, path)?;
    Ok(())
}
fn copy_tree(source: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_tree(&from, &to)?;
        } else if ty.is_symlink() {
            std::os::unix::fs::symlink(fs::read_link(&from)?, &to)?;
        } else if ty.is_file() && fs::hard_link(&from, &to).is_err() {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
fn extract_compute(work: &Work<'_>, package: &Path, dest: &Path, core: bool) -> Result<()> {
    fs::create_dir_all(dest)?;
    if !core {
        return work.run(
            "dpkg-deb",
            &[
                "--extract".into(),
                package.as_os_str().into(),
                dest.as_os_str().into(),
            ],
        );
    }
    // Extract only the inference runtime, omitting the large builder resource.
    let mut producer = work
        .command("dpkg-deb")?
        .args(["--fsys-tarfile".as_ref(), package.as_os_str()])
        .stdout(Stdio::piped())
        .spawn()?;
    let result = (|| -> Result<()> {
        let source = producer
            .stdout
            .take()
            .context("Pipe de extração indisponível")?;
        let status = work
            .command("tar")?
            .args(["-x", "--wildcards", "--no-same-owner", "-C"])
            .arg(dest)
            .arg("./usr/lib/x86_64-linux-gnu/libnvinfer.so*")
            .stdin(Stdio::from(source))
            .status()?;
        if !status.success() {
            bail!("Não foi possível extrair o runtime TensorRT");
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = producer.kill();
    }
    let status = producer.wait()?;
    result?;
    if !status.success() {
        bail!("A leitura do pacote TensorRT falhou");
    }
    Ok(())
}
fn cuda(
    work: &mut Work<'_>,
    effects: &Effects,
    cache: &Path,
    stage: &Path,
) -> Result<Vec<PathBuf>> {
    let mut roots = vec![user_root("XDG_DATA_HOME", ".local/share")?.join("facecam-hub/ai/cuda")];
    if let Some(ai) = Path::new(&effects.ai_runtime)
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
    {
        roots.push(ai.join("cuda"));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent().and_then(Path::parent)
    {
        roots.push(parent.join("ai-runtime/cuda"));
    }
    if let Some(source) = roots.into_iter().find(|p| {
        p.join("nvidia/cudnn/lib/libcudnn.so.9").is_file()
            && p.join("nvidia/cublas/lib/libcublas.so.12").is_file()
    }) {
        work.phase("Reutilizando bibliotecas CUDA já disponíveis")?;
        copy_tree(&source, &stage.join("cuda"))?;
    } else {
        for (package, version) in [
            ("nvidia-cudnn-cu12", "9.8.0.87"),
            ("nvidia-cublas-cu12", "12.8.4.1"),
            ("nvidia-cuda-runtime-cu12", "12.8.90"),
            ("nvidia-curand-cu12", "10.3.9.90"),
            ("nvidia-cufft-cu12", "11.3.3.83"),
            ("nvidia-cuda-nvrtc-cu12", "12.8.93"),
        ] {
            work.phase(&format!("Preparando {package}"))?;
            let metadata = cache.join(format!("{package}-{version}.json"));
            work.run(
                "curl",
                &[
                    "--fail".into(),
                    "--location".into(),
                    "--max-filesize".into(),
                    "4194304".into(),
                    "--max-time".into(),
                    "60".into(),
                    "--output".into(),
                    metadata.as_os_str().into(),
                    format!("https://pypi.org/pypi/{package}/{version}/json").into(),
                ],
            )?;
            if fs::metadata(&metadata)?.len() > 4194304 {
                bail!("Metadados CUDA excedem o limite");
            }
            let json: serde_json::Value = serde_json::from_reader(File::open(metadata)?)?;
            let file = json["urls"]
                .as_array()
                .and_then(|a| {
                    a.iter().find(|f| {
                        f["filename"]
                            .as_str()
                            .is_some_and(|s| s.contains("manylinux") && s.contains("x86_64"))
                    })
                })
                .context("Biblioteca CUDA x86-64 indisponível")?;
            let url = file["url"].as_str().context("URL CUDA ausente")?;
            let hash = file["digests"]["sha256"]
                .as_str()
                .context("Hash CUDA ausente")?;
            if !url.starts_with("https://files.pythonhosted.org/")
                || hash.len() != 64
                || !hash.bytes().all(|c| c.is_ascii_hexdigit())
            {
                bail!("Origem da biblioteca CUDA inválida");
            }
            let wheel = cache.join(format!("{package}-{version}.whl"));
            fetch(work, url, &wheel, hash, false)?;
            work.run(
                "unzip",
                &[
                    "-oq".into(),
                    wheel.as_os_str().into(),
                    "-d".into(),
                    stage.join("cuda").into_os_string(),
                ],
            )?;
        }
    }
    let mut dirs = Vec::new();
    for entry in fs::read_dir(stage.join("cuda/nvidia"))? {
        let path = entry?.path().join("lib");
        if path.is_dir() {
            dirs.push(path.strip_prefix(stage)?.to_path_buf());
        }
    }
    dirs.sort();
    Ok(dirs)
}

fn prepare(effects: &Effects, work: &mut Work<'_>) -> Result<String> {
    crate::nvidia::ada_device()?;
    work.phase("Preparando segmentação RTX para a família Ada")?;
    let root = root()?;
    let parent = root.parent().context("Destino RTX inválido")?;
    fs::create_dir_all(parent)?;
    let stage = tempfile::Builder::new()
        .prefix(".ada-preparing-")
        .tempdir_in(parent)?;
    let stage_path = stage.path();
    let cache = user_root("XDG_CACHE_HOME", ".cache")?.join("facecam-hub/nvidia-vfx-0.7.6-ada");
    fs::create_dir_all(&cache)?;
    let installer = cache.join("nvidia_video_effects_sdk_installer_v0.7.6_ada.exe");
    if !installer.exists()
        && let Some(home) = std::env::var_os("HOME")
    {
        for sub in ["Downloads/NVIDEA", "Downloads/NVIDIA", "Downloads"] {
            let source = PathBuf::from(&home).join(sub).join(
                installer
                    .file_name()
                    .context("Nome do instalador ausente")?,
            );
            if checksum(&source).is_ok_and(|s| s == nvidia_model::SDK_SHA) {
                fs::hard_link(&source, &installer)
                    .or_else(|_| fs::copy(&source, &installer).map(|_| ()))?;
                break;
            }
        }
    }
    work.phase("Conferindo o pacote Windows de referência")?;
    fetch(
        work,
        nvidia_model::SDK_URL,
        &installer,
        nvidia_model::SDK_SHA,
        false,
    )?;
    let sdk = cache.join("sdk");
    fs::create_dir_all(&sdk)?;
    work.run(
        "7z",
        &[
            "x".into(),
            "-y".into(),
            format!("-o{}", sdk.display()).into(),
            installer.into_os_string(),
            "NVVideoEffects.dll".into(),
            "models/AIGS_288x512_89_m1.engine.trtpkg".into(),
            "NVIDIA Maxine EULA.pdf".into(),
            "ThirdPartyLicenses.txt".into(),
        ],
    )?;
    let dll_path = sdk.join("NVVideoEffects.dll");
    let model_path = sdk.join("models/AIGS_288x512_89_m1.engine.trtpkg");
    if fs::metadata(&dll_path)?.len() > 32 * 1024 * 1024
        || fs::metadata(&model_path)?.len() > 16 * 1024 * 1024
    {
        bail!("Arquivo de referência excede o limite");
    }
    work.phase("Preparando o modelo de segmentação local")?;
    let engine = nvidia_model::import(&fs::read(dll_path)?, &fs::read(model_path)?)?;
    fs::write(stage_path.join("segmentation.engine"), &engine)?;
    fs::write(
        stage_path.join("segmentation.ctx.onnx"),
        nvidia_model::context_model(),
    )?;
    for license in ["NVIDIA Maxine EULA.pdf", "ThirdPartyLicenses.txt"] {
        fs::copy(sdk.join(license), stage_path.join(license))?;
    }
    work.phase("Preparando o runtime de inferência")?;
    let ort_archive = cache.join(format!("{ORT}.tgz"));
    fetch(
        work,
        &format!("https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/{ORT}.tgz"),
        &ort_archive,
        ORT_SHA,
        false,
    )?;
    work.run(
        "tar",
        &[
            "-xzf".into(),
            ort_archive.into_os_string(),
            "-C".into(),
            stage_path.as_os_str().into(),
        ],
    )?;
    for (i, (name, hash)) in COMPUTE.into_iter().enumerate() {
        work.phase(&format!("Preparando TensorRT: {name}"))?;
        let filename = format!("{name}_10.4.0.26-1+cuda12.6_amd64.deb");
        let package = cache.join(&filename);
        fetch(
            work,
            &format!(
                "https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/{filename}"
            ),
            &package,
            hash,
            true,
        )?;
        extract_compute(work, &package, &stage_path.join("compute"), i == 0)?;
    }
    let mut libraries = cuda(work, effects, &cache, stage_path)?;
    libraries.push("compute/usr/lib/x86_64-linux-gnu".into());
    libraries.push(format!("{ORT}/lib").into());
    let install = Installation {
        format_version: 1,
        sdk_version: "0.7.6".into(),
        family: "ada".into(),
        model: "segmentation.ctx.onnx".into(),
        runtime: format!("{ORT}/lib/libonnxruntime.so").into(),
        libraries,
    };
    fs::write(
        stage_path.join("manifest.json"),
        serde_json::to_vec_pretty(&install)?,
    )?;
    fs::write(stage_path.join(".facecam-hub-nvidia"), b"1\n")?;
    work.phase("Testando uma inferência real na GPU")?;
    let resolved = Installation::load(stage_path.to_path_buf())?;
    let request =
        serde_json::json!({"background_blur":true,"ai_backend":"nvidia","ai_provider":"cuda"});
    // Keep testing this running build even if an update replaces its file
    // during a long download. /proc/self/exe would refer to setpriv itself.
    let executable = format!("/proc/{}/exe", std::process::id());
    let status = work
        .command(&executable)?
        .args(["--ai-worker", &request.to_string()])
        .env("FACECAM_NVIDIA_MANIFEST", stage_path.join("manifest.json"))
        .env(
            "LD_LIBRARY_PATH",
            std::env::join_paths(&resolved.libraries)?,
        )
        .status()?;
    if !status.success() {
        bail!("A inferência RTX falhou; a instalação anterior foi preservada");
    }
    let previous = tempfile::Builder::new()
        .prefix(".ada-previous-")
        .tempdir_in(parent)?;
    let had_previous = root.exists();
    if had_previous {
        if !root.join(".facecam-hub-nvidia").is_file() {
            bail!("O destino RTX já existe sem identificação do aplicativo; arquivos preservados");
        }
        fs::rename(&root, previous.path())?;
    }
    if let Err(error) = fs::rename(stage_path, &root) {
        if had_previous {
            fs::rename(previous.path(), &root)
                .context("Não foi possível restaurar os componentes RTX anteriores")?;
        }
        return Err(error.into());
    }
    work.phase("Segmentação RTX preparada e inferência validada")?;
    Ok("IA RTX preparada: segmentação temporal, desfoque e troca de fundo disponíveis".into())
}
