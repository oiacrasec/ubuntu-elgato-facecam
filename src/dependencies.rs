//! Reviewed APT plans. Only the controller can start a prepared installation.
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, TryRecvError},
};

const KEYRING_URL: &str =
    "https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/3bf863cc.pub";
const KEYRING_SHA256: &str = "7eb71103e32e813ea3e1c06bdb01b143f36feb8d83ecac2b0c11c9273f9e6822";
const REPOSITORY: &str = "Types: deb\nURIs: https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/\nSuites: /\nArchitectures: amd64\n";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Dependency {
    #[default]
    VideoTools,
    SdkManager,
    NvidiaTools,
    VirtualCamera,
}

impl Dependency {
    pub fn name(self) -> &'static str {
        match self {
            Self::VideoTools => "FFmpeg e V4L2",
            Self::SdkManager => "NVIDIA SDK Manager",
            Self::NvidiaTools => "Ferramentas de IA RTX",
            Self::VirtualCamera => "Câmera virtual",
        }
    }
    fn packages(self) -> &'static [&'static str] {
        match self {
            Self::VideoTools => &["ffmpeg", "v4l-utils"],
            Self::SdkManager => &["sdkmanager"],
            Self::NvidiaTools => &["curl", "p7zip-full", "unzip"],
            Self::VirtualCamera => &[],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    #[default]
    Install,
    Remove,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageState {
    pub name: String,
    pub version: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DependencyState {
    pub dependency: Dependency,
    pub packages: Vec<PackageState>,
}

impl DependencyState {
    pub fn installed(&self) -> bool {
        !self.packages.is_empty() && self.packages.iter().all(|p| p.version.is_some())
    }
}

/// Query the package database, including a package installed before this app existed.
pub fn discover() -> Result<Vec<DependencyState>> {
    let output = Command::new("/usr/bin/dpkg-query")
        .args([
            "--show",
            "--showformat=${Package}\t${db:Status-Status}\t${Version}\n",
            "--",
            "ffmpeg",
            "v4l-utils",
            "sdkmanager",
            "curl",
            "p7zip-full",
            "unzip",
        ])
        .env("LC_ALL", "C")
        .output()
        .context("Não foi possível consultar pacotes pelo dpkg-query")?;
    // Exit 1 also means that one of the requested optional packages is absent.
    if !matches!(output.status.code(), Some(0 | 1)) {
        bail!(
            "dpkg-query falhou: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(parse_packages(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_packages(text: &str) -> Vec<DependencyState> {
    [
        Dependency::VideoTools,
        Dependency::SdkManager,
        Dependency::NvidiaTools,
    ]
    .into_iter()
    .map(|dependency| DependencyState {
        dependency,
        packages: dependency
            .packages()
            .iter()
            .map(|name| {
                let entry = text.lines().find_map(|line| {
                    let mut fields = line.split('\t');
                    if fields.next()? != *name {
                        return None;
                    }
                    let status = fields.next()?;
                    let version = fields.next()?.trim();
                    Some((status.to_owned(), version.to_owned()))
                });
                let version = entry.as_ref().and_then(|(status, version)| {
                    (status == "installed" && !version.is_empty()).then(|| version.clone())
                });
                PackageState {
                    name: (*name).into(),
                    version,
                    status: entry.map(|(status, _)| status),
                }
            })
            .collect(),
    })
    .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstallPlan {
    pub id: u64,
    pub title: String,
    pub explanation: String,
    pub commands: Vec<Vec<String>>,
    pub log: PathBuf,
    pub verified_file: Option<PathBuf>,
    pub sha256: Option<String>,
    pub bootstrap: Option<PathBuf>,
    pub repository: Option<String>,
    #[serde(default)]
    pub dependency: Dependency,
    #[serde(default)]
    pub action: Action,
    #[serde(default)]
    pub removals: Vec<String>,
    #[serde(default)]
    pub virtual_camera: Option<crate::virtual_camera::Setup>,
}

fn apt(arguments: &[&str]) -> Vec<String> {
    [
        "/usr/bin/pkexec",
        "--disable-internal-agent",
        "/usr/bin/apt-get",
    ]
    .into_iter()
    .chain(arguments.iter().copied())
    .map(str::to_owned)
    .collect()
}

#[cfg(feature = "gui")]
pub fn display_command(command: &[String]) -> String {
    command
        .iter()
        .map(|argument| {
            if !argument.is_empty()
                && argument
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "/._-=:".contains(c))
            {
                argument.clone()
            } else {
                format!("'{}'", argument.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn prepare(dependency: Dependency, id: u64, directory: &Path) -> Result<InstallPlan> {
    let release = std::fs::read_to_string("/etc/os-release")?;
    if !release
        .lines()
        .any(|line| matches!(line, "ID=ubuntu" | "ID=\"ubuntu\""))
    {
        bail!("A instalação assistida por APT requer Ubuntu");
    }
    let mut plan = InstallPlan {
        id,
        title: String::new(),
        explanation: String::new(),
        commands: vec![apt(&["update"])],
        log: directory.join(format!("install-{id}.log")),
        verified_file: None,
        sha256: None,
        bootstrap: None,
        repository: None,
        dependency,
        action: Action::Install,
        removals: Vec::new(),
        virtual_camera: None,
    };
    match dependency {
        Dependency::VideoTools => {
            plan.title = "Instalar ferramentas de vídeo".into();
            plan.explanation =
                "Instala FFmpeg e v4l-utils pelos repositórios configurados no Ubuntu.".into();
            plan.commands.push(apt(&[
                "install",
                "--yes",
                "--no-install-recommends",
                "--",
                "ffmpeg",
                "v4l-utils",
            ]));
        }
        Dependency::NvidiaTools => {
            plan.title = "Preparar ferramentas de IA RTX".into();
            plan.explanation = "Instala as ferramentas de download e extração pelo APT. Depois de concluir, clique novamente em Preparar IA RTX para obter e validar os componentes no seu usuário.".into();
            plan.commands.push(apt(&[
                "install",
                "--yes",
                "--no-install-recommends",
                "--",
                "curl",
                "p7zip-full",
                "unzip",
            ]));
        }
        Dependency::SdkManager => {
            plan.title = "Instalar NVIDIA SDK Manager".into();
            plan.explanation = "Instala o gerenciador NVIDIA. A preparação dos efeitos de fundo fica em Efeitos NVIDIA RTX nas configurações.".into();
            if std::env::consts::ARCH != "x86_64" {
                bail!("A instalação assistida do SDK Manager requer Ubuntu x86-64");
            }
            let candidate = Command::new("/usr/bin/apt-cache")
                .args(["policy", "sdkmanager"])
                .env("LC_ALL", "C")
                .output()?;
            let available = candidate.status.success()
                && String::from_utf8_lossy(&candidate.stdout)
                    .lines()
                    .any(|line| {
                        line.trim()
                            .strip_prefix("Candidate:")
                            .is_some_and(|v| !v.trim().is_empty() && v.trim() != "(none)")
                    });
            if available {
                plan.commands.push(apt(&[
                    "install",
                    "--yes",
                    "--no-install-recommends",
                    "--",
                    "sdkmanager",
                ]));
            } else {
                if !release
                    .lines()
                    .any(|line| matches!(line, "VERSION_ID=24.04" | "VERSION_ID=\"24.04\""))
                {
                    bail!(
                        "Configure o repositório NVIDIA para esta versão do Ubuntu; o cadastro assistido está disponível para Ubuntu 24.04"
                    );
                }
                let folder = directory.join(format!("nvidia-repository-{id}"));
                let keyring = folder.join("nvidia-cuda.asc");
                let source = folder.join("nvidia-cuda.sources");
                let path = keyring
                    .to_str()
                    .context("O caminho do instalador deve ser UTF-8")?;
                plan.commands.insert(
                    0,
                    vec![
                        "/usr/bin/curl".into(),
                        "--fail".into(),
                        "--location".into(),
                        "--proto".into(),
                        "=https".into(),
                        "--proto-redir".into(),
                        "=https".into(),
                        "--connect-timeout".into(),
                        "15".into(),
                        "--max-time".into(),
                        "120".into(),
                        "--output".into(),
                        path.into(),
                        KEYRING_URL.into(),
                    ],
                );
                plan.commands.insert(
                    1,
                    [
                        "/usr/bin/pkexec",
                        "--disable-internal-agent",
                        "/usr/bin/install",
                        "-m",
                        "0644",
                        "-T",
                        "--",
                        source
                            .to_str()
                            .context("O caminho do repositório deve ser UTF-8")?,
                        "/etc/apt/sources.list.d/facecam-hub-nvidia-cuda.sources",
                    ]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                );
                plan.commands.push(apt(&[
                    "install",
                    "--yes",
                    "--no-install-recommends",
                    "--",
                    "sdkmanager",
                ]));
                plan.verified_file = Some(keyring);
                plan.sha256 = Some(KEYRING_SHA256.into());
                plan.bootstrap = Some(folder);
                plan.repository = Some(REPOSITORY.into());
                plan.explanation.push_str(" Cadastra o repositório oficial NVIDIA para Ubuntu 24.04 com chave de assinatura restrita a essa fonte. Não usa instaladores .deb locais. Não instala CUDA Toolkit nem solicita atualização do driver.");
            }
        }
        Dependency::VirtualCamera => return crate::virtual_camera::prepare(id, directory, true),
    }
    Ok(plan)
}

pub fn prepare_removal(dependency: Dependency, id: u64, directory: &Path) -> Result<InstallPlan> {
    if dependency == Dependency::VirtualCamera {
        bail!(
            "A preparação não descarrega módulos nem remove pacotes de kernel; gerencie isso no Ubuntu"
        );
    }
    let packages: Vec<_> = discover()?
        .into_iter()
        .find(|s| s.dependency == dependency)
        .context("Estado da dependência indisponível")?
        .packages
        .into_iter()
        .filter(|p| p.version.is_some())
        .map(|p| p.name)
        .collect();
    if packages.is_empty() {
        bail!(
            "{} já está desinstalado; atualize o estado",
            dependency.name()
        );
    }
    let removals = simulate_removal(&packages)?;
    Ok(InstallPlan {
        id, title: format!("Desinstalar {}", dependency.name()),
        explanation: "Remove os pacotes de sistema listados abaixo. Outros aplicativos que usam essas ferramentas também serão afetados. A remoção não usa autoremove nem purge. Configurações e recursos do Facecam Hub são preservados.".into(),
        commands: vec![removal_command(&packages)], log: directory.join(format!("uninstall-{id}.log")),
        verified_file: None, sha256: None, bootstrap: None, repository: None,
        dependency, action: Action::Remove, removals, virtual_camera: None,
    })
}

fn removal_command(packages: &[String]) -> Vec<String> {
    let mut command = apt(&["remove", "--yes", "--no-auto-remove", "--"]);
    command.extend(packages.iter().cloned());
    command
}

fn simulate_removal(packages: &[String]) -> Result<Vec<String>> {
    let output = Command::new("/usr/bin/apt-get")
        .args(["--simulate", "--no-auto-remove", "remove", "--"])
        .args(packages)
        .env("LC_ALL", "C")
        .output()
        .context("Não foi possível simular a desinstalação pelo APT")?;
    if !output.status.success() {
        bail!(
            "O APT recusou a simulação de remoção: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    validate_removals(&String::from_utf8_lossy(&output.stdout), packages)
}

fn validate_removals(output: &str, allowed: &[String]) -> Result<Vec<String>> {
    let mut removed: Vec<String> = output
        .lines()
        .filter_map(|line| {
            line.strip_prefix("Remv ")?
                .split_whitespace()
                .next()
                .map(str::to_owned)
        })
        .collect();
    removed.sort();
    removed.dedup();
    if removed.iter().any(|name| !allowed.contains(name)) {
        bail!(
            "A remoção afetaria outros pacotes: {}. Use o gerenciador de pacotes do Ubuntu para revisar essa operação",
            removed.join(", ")
        );
    }
    if removed.is_empty() {
        bail!("Nenhum pacote seria removido; atualize o estado antes de tentar novamente");
    }
    Ok(removed)
}

fn verify_result(
    dependency: Dependency,
    action: Action,
    states: &[DependencyState],
) -> Result<String> {
    let state = states
        .iter()
        .find(|s| s.dependency == dependency)
        .context("Não foi possível verificar o resultado no dpkg")?;
    match action {
        Action::Install if state.installed() => Ok(format!(
            "{} instalado e verificado no sistema",
            dependency.name()
        )),
        Action::Remove
            if state.packages.iter().all(|p| {
                matches!(
                    p.status.as_deref(),
                    None | Some("not-installed" | "config-files")
                )
            }) =>
        {
            Ok(format!(
                "{} desinstalado e verificado no sistema",
                dependency.name()
            ))
        }
        _ => bail!(
            "O APT terminou, mas o estado dos pacotes não corresponde à operação; consulte o log e atualize o estado"
        ),
    }
}

fn sha256(path: &Path) -> Result<String> {
    let output = Command::new("/usr/bin/sha256sum")
        .arg("--")
        .arg(path)
        .output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let digest = text
        .trim_start_matches('\\')
        .split_whitespace()
        .next()
        .unwrap_or("");
    if !output.status.success()
        || digest.len() != 64
        || !digest.bytes().all(|c| c.is_ascii_hexdigit())
    {
        bail!("Não foi possível verificar SHA-256 de {}", path.display());
    }
    Ok(digest.into())
}

pub struct Installation {
    result: Receiver<std::result::Result<String, String>>,
}

impl Installation {
    pub fn start(plan: InstallPlan) -> Result<Self> {
        let (tx, result) = mpsc::channel();
        std::thread::Builder::new()
            .name("apt-install".into())
            .spawn(move || {
                let result = run(&plan)
                    .and_then(|()| match &plan.virtual_camera {
                        Some(setup) => setup.verify_result(),
                        None => verify_result(plan.dependency, plan.action, &discover()?),
                    })
                    .map_err(|error| format!("{error:#}. Log: {}", plan.log.display()));
                let _ = tx.send(result);
            })?;
        Ok(Self { result })
    }

    pub fn poll(&self) -> Option<std::result::Result<String, String>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(
                "A operação APT encerrou sem resultado; consulte o log antes de repetir".into(),
            )),
        }
    }
}

fn run(plan: &InstallPlan) -> Result<()> {
    std::fs::create_dir_all(plan.log.parent().context("Log sem diretório")?)?;
    let mut log = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&plan.log)?;
    if let Some(folder) = &plan.bootstrap {
        std::fs::DirBuilder::new().mode(0o700).create(folder)?;
    }
    if plan.action == Action::Remove && simulate_removal(&plan.removals)? != plan.removals {
        bail!("O plano de remoção mudou; revise e confirme um novo plano");
    }
    for command in &plan.commands {
        if let Some(setup) = &plan.virtual_camera {
            setup.validate_current()?;
        }
        // The first bootstrap command downloads the public key. Every later
        // privileged command checks it against the reviewed, pinned digest.
        if command.first().is_some_and(|v| v == "/usr/bin/pkexec")
            && let (Some(file), Some(expected)) = (&plan.verified_file, &plan.sha256)
            && sha256(file)? != *expected
        {
            bail!(
                "O arquivo verificado mudou ou o hash não confere. Prepare e confirme um novo plano"
            );
        }
        writeln!(log, "Executando: {command:?}")?;
        let (program, arguments) = command.split_first().context("Comando APT vazio")?;
        let status = Command::new(program)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log.try_clone()?)
            .status()
            .with_context(|| format!("Não foi possível executar {program}"))?;
        if !status.success() {
            let mut tail = File::open(&plan.log)?;
            let size = tail.metadata()?.len();
            tail.seek(SeekFrom::Start(size.saturating_sub(4096)))?;
            let mut bytes = Vec::new();
            tail.take(4096).read_to_end(&mut bytes)?;
            bail!(
                "Operação APT interrompida ({status}): {}",
                String::from_utf8_lossy(&bytes)
            );
        }
        if program == "/usr/bin/curl"
            && let (Some(folder), Some(repository), Some(file), Some(expected)) = (
                &plan.bootstrap,
                &plan.repository,
                &plan.verified_file,
                &plan.sha256,
            )
        {
            if sha256(file)? != *expected {
                bail!("SHA-256 da chave NVIDIA não confere");
            }
            let key = std::fs::read_to_string(file)?;
            let contents = repository_with_key(repository, &key);
            let mut source = OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(folder.join("nvidia-cuda.sources"))?;
            source.write_all(contents.as_bytes())?;
        }
    }
    Ok(())
}

fn repository_with_key(repository: &str, key: &str) -> String {
    let mut contents = format!("{repository}Signed-By:\n");
    for line in key.lines() {
        if line.is_empty() {
            contents.push_str(" .\n");
        } else {
            contents.push(' ');
            contents.push_str(line);
            contents.push('\n');
        }
    }
    contents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_database_distinguishes_installed_partial_removed_and_broken_states() {
        let installed = parse_packages(
            "ffmpeg\tinstalled\t7:6.1.1\nv4l-utils\tinstalled\t1.26.1\nsdkmanager\tconfig-files\t26.1.0\n",
        );
        assert!(installed[0].installed());
        assert_eq!(installed[0].packages[0].version.as_deref(), Some("7:6.1.1"));
        assert!(!installed[1].installed());
        for state in [
            "not-installed",
            "unpacked",
            "half-configured",
            "config-files",
            "half-installed",
        ] {
            let partial = parse_packages(&format!(
                "ffmpeg\tinstalled\t6.1\nv4l-utils\t{state}\t1.26\n"
            ));
            assert!(!partial[0].installed());
            assert!(partial[0].packages[1].version.is_none());
        }
        let absent = parse_packages("");
        assert!(absent.iter().all(|s| !s.installed()));
        assert!(!parse_packages("ffmpeg\tinstalled\t\nv4l-utils\tinstalled\t1\n")[0].installed());
    }

    #[test]
    fn successful_process_exit_requires_the_requested_package_state() {
        let present = parse_packages("ffmpeg\tinstalled\t6.1\nv4l-utils\tinstalled\t1.26\n");
        let absent = parse_packages("");
        assert!(verify_result(Dependency::VideoTools, Action::Install, &present).is_ok());
        assert!(verify_result(Dependency::VideoTools, Action::Install, &absent).is_err());
        assert!(verify_result(Dependency::VideoTools, Action::Remove, &present).is_err());
        assert!(verify_result(Dependency::VideoTools, Action::Remove, &absent).is_ok());
        let incomplete = parse_packages("ffmpeg\thalf-installed\t6.1\n");
        assert!(verify_result(Dependency::VideoTools, Action::Remove, &incomplete).is_err());
    }

    #[test]
    fn removal_refuses_extra_packages_and_never_adds_autoremove_or_purge() {
        let allowed = vec!["ffmpeg".into(), "v4l-utils".into()];
        assert_eq!(
            validate_removals("Remv v4l-utils [1.26]\nRemv ffmpeg [6.1]\n", &allowed).unwrap(),
            allowed
        );
        assert!(validate_removals("Remv ffmpeg [6.1]\nRemv desktop-app [1]\n", &allowed).is_err());
        assert!(validate_removals("Nothing to remove\n", &allowed).is_err());
        let command = removal_command(&allowed);
        assert!(command.iter().any(|v| v == "remove"));
        assert!(command.iter().any(|v| v == "--no-auto-remove"));
        assert!(
            !command
                .iter()
                .any(|v| matches!(v.as_str(), "autoremove" | "purge"))
        );
    }

    #[test]
    #[ignore = "requires HTTPS and APT; downloads signed metadata into a private temporary directory, never installs packages"]
    fn apt_repository_accepts_the_pinned_key_and_offers_sdkmanager() {
        let dir = tempfile::tempdir().unwrap();
        let key = PathBuf::from(
            std::env::var_os("FACECAM_NVIDIA_PUBLIC_KEY")
                .expect("set FACECAM_NVIDIA_PUBLIC_KEY to 3bf863cc.pub"),
        );
        assert_eq!(sha256(&key).unwrap(), KEYRING_SHA256);
        let etc = dir.path().join("etc");
        for directory in [
            &etc,
            &dir.path().join("state/lists/partial"),
            &dir.path().join("cache/archives/partial"),
            &dir.path().join("log"),
        ] {
            std::fs::create_dir_all(directory).unwrap();
        }
        std::fs::write(
            etc.join("nvidia.sources"),
            repository_with_key(REPOSITORY, &std::fs::read_to_string(key).unwrap()),
        )
        .unwrap();
        let config = dir.path().join("apt.conf");
        std::fs::write(&config, format!(
            "Dir::Etc \"{}\";\nDir::Etc::main \"-\";\nDir::Etc::parts \"-\";\nDir::Etc::sourceparts \"-\";\nDir::Etc::sourcelist \"nvidia.sources\";\nDir::State \"{}\";\nDir::Cache \"{}\";\nDir::Log \"{}\";\nAcquire::Retries \"0\";\nAcquire::https::Timeout \"30\";\n",
            etc.display(), dir.path().join("state").display(), dir.path().join("cache").display(), dir.path().join("log").display()
        )).unwrap();
        let update = Command::new("/usr/bin/apt-get")
            .arg("update")
            .env("APT_CONFIG", &config)
            .env("LC_ALL", "C")
            .output()
            .unwrap();
        assert!(
            update.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&update.stdout),
            String::from_utf8_lossy(&update.stderr)
        );
        let policy = Command::new("/usr/bin/apt-cache")
            .args(["policy", "sdkmanager"])
            .env("APT_CONFIG", config)
            .env("LC_ALL", "C")
            .output()
            .unwrap();
        let output = String::from_utf8_lossy(&policy.stdout);
        assert!(
            policy.status.success()
                && output.lines().any(|line| line
                    .trim()
                    .strip_prefix("Candidate:")
                    .is_some_and(|v| !v.trim().is_empty() && v.trim() != "(none)")),
            "{output}"
        );
        println!(
            "APT verified NVIDIA's signed repository in isolated state; no installation:\n{output}"
        );
    }

    #[test]
    fn public_key_change_aborts_before_any_administrative_command() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a ' ; [public key].asc");
        std::fs::write(&path, b"reviewed").unwrap();
        let digest = sha256(&path).unwrap();
        let plan = InstallPlan {
            id: 1,
            title: String::new(),
            explanation: String::new(),
            commands: vec![apt(&["install", "--yes", "--", path.to_str().unwrap()])],
            log: dir.path().join("install.log"),
            verified_file: Some(path.clone()),
            sha256: Some(digest),
            bootstrap: None,
            repository: None,
            dependency: Dependency::VideoTools,
            action: Action::Install,
            removals: Vec::new(),
            virtual_camera: None,
        };
        std::fs::write(path, b"changed after review").unwrap();
        let error = run(&plan).unwrap_err();
        assert!(error.to_string().contains("arquivo verificado mudou"));
        assert!(std::fs::read(plan.log).unwrap().is_empty());
    }

    #[test]
    fn failed_step_stops_the_plan_and_reports_its_log() {
        let dir = tempfile::tempdir().unwrap();
        let plan = InstallPlan {
            id: 1,
            title: String::new(),
            explanation: String::new(),
            commands: vec![
                vec!["/usr/bin/false".into()],
                vec!["/missing-never-executed".into()],
            ],
            log: dir.path().join("install.log"),
            verified_file: None,
            sha256: None,
            bootstrap: None,
            repository: None,
            dependency: Dependency::VideoTools,
            action: Action::Install,
            removals: Vec::new(),
            virtual_camera: None,
        };
        let error = run(&plan).unwrap_err();
        assert!(error.to_string().contains("Operação APT interrompida"));
        let log = std::fs::read_to_string(plan.log).unwrap();
        assert!(log.contains("/usr/bin/false"));
        assert!(!log.contains("/missing-never-executed"));
    }
}
