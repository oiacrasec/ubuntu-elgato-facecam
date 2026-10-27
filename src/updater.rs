//! Explicit updates from the repository's production HEAD, off the controller thread.
use crate::installation::{BuildInfo, Installed, REPOSITORY};
use anyhow::{Context, Result, bail};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Idle,
    Checking,
    Current,
    Available,
    Installing,
    RestartRequired,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Candidate {
    pub branch: String,
    pub revision: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Status {
    pub build: BuildInfo,
    pub phase: Phase,
    pub candidate: Option<Candidate>,
    pub installed: bool,
    pub message: String,
    pub log: Option<PathBuf>,
}
impl Default for Status {
    fn default() -> Self {
        let installation = Installed::discover();
        Self {
            build: BuildInfo::default(),
            phase: Phase::Idle,
            candidate: None,
            installed: installation.as_ref().is_ok_and(|i| i.is_some()),
            message: installation
                .err()
                .map_or_else(String::new, |e| format!("{e:#}")),
            log: None,
        }
    }
}
impl Status {
    pub fn busy(&self) -> bool {
        matches!(self.phase, Phase::Checking | Phase::Installing)
    }
    pub fn accept(&mut self, event: Event) {
        match event {
            Event::Progress(text) => self.message = text,
            Event::Checked(result) => match result {
                Ok(candidate) => {
                    let current = candidate.revision == self.build.revision && !self.build.dirty;
                    self.phase = if current {
                        Phase::Current
                    } else {
                        Phase::Available
                    };
                    self.message = if current {
                        "Você está usando a versão mais recente"
                    } else {
                        "Nova atualização disponível"
                    }
                    .into();
                    self.candidate = Some(candidate);
                }
                Err(error) => {
                    self.phase = Phase::Failed;
                    self.message = error;
                }
            },
            Event::Installed(result) => match result {
                Ok(()) => {
                    self.phase = Phase::RestartRequired;
                    self.message =
                        "Atualização instalada. Reinicie para usar a nova versão.".into();
                }
                Err(error) => {
                    self.phase = Phase::Failed;
                    self.message = error;
                }
            },
        }
    }
}

pub enum Event {
    Progress(String),
    Checked(Result<Candidate, String>),
    Installed(Result<(), String>),
}
pub struct Job {
    rx: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl Job {
    pub fn check() -> Self {
        Self::spawn(true, |work| production(work, REPOSITORY).map(Some))
    }
    pub fn install(candidate: Candidate, installed: Installed, log: PathBuf) -> Self {
        Self::spawn(false, move |work| {
            apply(work, REPOSITORY, &candidate, &installed, &log).map(|_| None)
        })
    }
    fn spawn(
        checking: bool,
        run: impl FnOnce(&Work) -> Result<Option<Candidate>> + Send + 'static,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let thread = thread::spawn(move || {
            let work = Work { tx, cancel: flag };
            let result = run(&work);
            let event = match result {
                Ok(Some(candidate)) => Event::Checked(Ok(candidate)),
                Ok(None) => Event::Installed(Ok(())),
                Err(error) if checking => Event::Checked(Err(format!("{error:#}"))),
                Err(error) => Event::Installed(Err(format!("{error:#}"))),
            };
            let _ = work.tx.send(event);
        });
        Self {
            rx,
            cancel,
            thread: Some(thread),
        }
    }
    pub fn poll(&self) -> Option<Event> {
        self.rx.try_recv().ok()
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct Work {
    tx: Sender<Event>,
    cancel: Arc<AtomicBool>,
}
impl Work {
    fn phase(&self, message: &str) {
        let _ = self.tx.send(Event::Progress(message.into()));
    }
    fn run(&self, command: &mut Command, timeout: Duration, log: Option<&Path>) -> Result<String> {
        // File-backed output cannot fill a pipe while Cargo compiles. Only a
        // bounded diagnostic tail returns to the controller after a failure.
        let capture = tempfile::NamedTempFile::new()?;
        let stdout = capture.reopen()?;
        let stderr = if let Some(log) = log {
            fs::OpenOptions::new().create(true).append(true).open(log)?
        } else {
            stdout.try_clone()?
        };
        use std::os::unix::process::CommandExt;
        command.process_group(0);
        command
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .env("GIT_TERMINAL_PROMPT", "0");
        let mut child = command
            .spawn()
            .context("Não foi possível executar a etapa de atualização")?;
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if self.cancel.load(Ordering::Relaxed) || started.elapsed() > timeout {
                let _ = nix::sys::signal::killpg(
                    nix::unistd::Pid::from_raw(child.id() as i32),
                    nix::sys::signal::Signal::SIGKILL,
                );
                let _ = child.wait();
                bail!("Atualização cancelada ou tempo limite atingido");
            }
            thread::sleep(Duration::from_millis(80));
        };
        let mut text = String::new();
        capture
            .reopen()?
            .take(64 * 1024)
            .read_to_string(&mut text)?;
        if !status.success() {
            let detail = if let Some(log) = log {
                tail(log).unwrap_or_default()
            } else {
                text.clone()
            };
            bail!(
                "A atualização não pôde concluir esta etapa: {}",
                detail.trim()
            );
        }
        Ok(text)
    }
}
fn tail(path: &Path) -> Result<String> {
    use std::io::{Seek, SeekFrom};
    let mut file = fs::File::open(path)?;
    let size = file.metadata()?.len();
    file.seek(SeekFrom::Start(size.saturating_sub(4096)))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn parse_production(text: &str) -> Result<Candidate> {
    let branch = text
        .lines()
        .find_map(|line| {
            line.strip_prefix("ref: refs/heads/")
                .and_then(|s| s.strip_suffix("\tHEAD"))
        })
        .filter(|s| matches!(*s, "main" | "master"))
        .context("A branch de produção precisa ser main ou master")?;
    let revision = text
        .lines()
        .find_map(|line| {
            line.strip_suffix("\tHEAD")
                .filter(|s| !s.starts_with("ref:"))
        })
        .filter(|s| valid_revision(s))
        .context("O repositório não informou uma revisão de produção válida")?;
    Ok(Candidate {
        branch: branch.into(),
        revision: revision.into(),
    })
}
fn valid_revision(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|c| c.is_ascii_hexdigit())
}
fn production(work: &Work, repository: &str) -> Result<Candidate> {
    work.phase("Verificando atualizações…");
    let text = work
        .run(
            Command::new("git").args(["ls-remote", "--symref", repository, "HEAD"]),
            Duration::from_secs(45),
            None,
        )
        .context(
            "Não foi possível consultar atualizações. Verifique a conexão e tente novamente.",
        )?;
    parse_production(&text)
}

fn cargo() -> PathBuf {
    let local = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo/bin/cargo"));
    local
        .filter(|p| p.is_file())
        .unwrap_or_else(|| "cargo".into())
}
pub fn log_path() -> Result<PathBuf> {
    Ok(cache()?.join("update.log"))
}
fn cache() -> Result<PathBuf> {
    let root = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .context("HOME não definido")?
        .join("facecam-hub/updates");
    fs::create_dir_all(&root)?;
    Ok(root)
}
fn apply(
    work: &Work,
    repository: &str,
    candidate: &Candidate,
    installed: &Installed,
    log: &Path,
) -> Result<()> {
    if !matches!(candidate.branch.as_str(), "main" | "master")
        || !valid_revision(&candidate.revision)
    {
        bail!("Revisão de atualização inválida");
    }
    let cache = cache()?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(cache.join("update.lock"))?;
    lock.try_lock_exclusive()
        .context("Outra atualização está em andamento")?;
    if let Some(parent) = log.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(log, b"Facecam Hub update\n")?;
    let current = production(work, repository)?;
    if current.branch != candidate.branch {
        bail!("A origem da atualização mudou. Verifique as atualizações novamente.");
    }
    let source = tempfile::Builder::new()
        .prefix("source-")
        .tempdir_in(&cache)?;
    work.phase("Baixando a atualização…");
    work.run(
        Command::new("git")
            .args([
                "clone",
                "--no-checkout",
                "--single-branch",
                "--no-tags",
                "--branch",
                &candidate.branch,
                repository,
            ])
            .arg(source.path()),
        Duration::from_secs(180),
        Some(log),
    )?;
    work.run(
        Command::new("git").arg("-C").arg(source.path()).args([
            "checkout",
            "--detach",
            &candidate.revision,
        ]),
        Duration::from_secs(30),
        Some(log),
    )?;
    if valid_revision(&installed.build.revision) {
        work.run(
            Command::new("git").arg("-C").arg(source.path()).args([
                "merge-base",
                "--is-ancestor",
                &installed.build.revision,
                &candidate.revision,
            ]),
            Duration::from_secs(30),
            Some(log),
        )
        .context("A atualização não pertence à continuação da instalação atual")?;
    }
    work.phase("Preparando a nova versão. Isso pode levar alguns minutos…");
    let target = cache.join("build");
    let mut build_command = Command::new(cargo());
    build_command.current_dir(source.path()).args([
        "build",
        "--locked",
        "--release",
        "--no-default-features",
        "--features",
    ]);
    let mut features = vec!["gui"];
    if installed.build.tray {
        features.push("tray");
    }
    if installed.build.ai {
        features.push("ai");
    }
    build_command
        .arg(features.join(","))
        .arg("--target-dir")
        .arg(&target)
        .env("CARGO_INCREMENTAL", "0");
    work.run(&mut build_command, Duration::from_secs(3600), Some(log))?;
    let binary = target.join("release/elgato-virtualcam");
    let output = work.run(
        Command::new(&binary).arg("--build-info"),
        Duration::from_secs(15),
        Some(log),
    )?;
    let build: BuildInfo = serde_json::from_str(&output)
        .context("A nova versão não informou seus dados corretamente")?;
    if build.application != "facecam-hub"
        || build.revision != candidate.revision
        || build.dirty
        || !build.gui
        || build.tray != installed.build.tray
        || build.ai != installed.build.ai
    {
        bail!("A nova compilação não corresponde à atualização selecionada");
    }
    work.phase("Instalando a atualização…");
    work.run(
        Command::new(&binary)
            .arg("--install-desktop")
            .arg(&installed.prefix)
            .arg("--desktop-resources")
            .arg(source.path().join("assets/backgrounds"))
            .env("XDG_DATA_HOME", &installed.data_home),
        Duration::from_secs(60),
        Some(log),
    )?;
    let published = Installed::for_executable(&installed.launcher().canonicalize()?)?
        .context("A instalação não publicou a nova versão")?;
    if published.build != build
        || crate::installation::checksum(&installed.launcher())? != published.binary_sha256
    {
        bail!("Não foi possível confirmar a integridade da atualização instalada");
    }
    Ok(())
}

pub fn restart(installed: &Installed, config: &Path, gui: bool) -> Result<()> {
    let mut command = Command::new(installed.launcher());
    command
        .arg("--restart-after")
        .arg(std::process::id().to_string())
        .arg("--config")
        .arg(config)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if !gui {
        command.arg("--headless");
    }
    command
        .spawn()
        .context("Não foi possível reiniciar o Facecam Hub")?;
    Ok(())
}
pub fn wait_for_parent(pid: u32) -> Result<()> {
    if pid < 2 || pid > i32::MAX as u32 {
        bail!("Processo de reinicialização inválido");
    }
    let start = Instant::now();
    loop {
        match nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None) {
            Err(nix::errno::Errno::ESRCH) => return Ok(()),
            Ok(()) => {}
            Err(e) => return Err(e.into()),
        }
        if start.elapsed() > Duration::from_secs(20) {
            bail!("A instância anterior não encerrou a tempo");
        }
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_is_the_remote_default_main_or_master_only() {
        let revision = "a".repeat(40);
        for branch in ["main", "master"] {
            let text = format!("ref: refs/heads/{branch}\tHEAD\n{revision}\tHEAD\n");
            assert_eq!(
                parse_production(&text).unwrap(),
                Candidate {
                    branch: branch.into(),
                    revision: revision.clone()
                }
            );
        }
        for text in [
            format!("ref: refs/heads/REA\tHEAD\n{revision}\tHEAD"),
            format!("{revision}\tHEAD"),
            "ref: refs/heads/master\tHEAD\ninvalid\tHEAD".into(),
        ] {
            assert!(parse_production(&text).is_err());
        }
    }

    #[test]
    fn check_uses_local_git_and_failures_leave_the_controller_responsive() {
        let repo = tempfile::tempdir().unwrap();
        assert!(
            Command::new("git")
                .args(["init", "--initial-branch=master"])
                .arg(repo.path())
                .output()
                .unwrap()
                .status
                .success()
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(repo.path())
                .args([
                    "-c",
                    "user.name=Fixture",
                    "-c",
                    "user.email=fixture@example.invalid",
                    "commit",
                    "--allow-empty",
                    "-m",
                    "fixture"
                ])
                .output()
                .unwrap()
                .status
                .success()
        );
        let path = repo.path().to_str().unwrap().to_owned();
        let job = Job::spawn(true, move |work| production(work, &path).map(Some));
        let start = Instant::now();
        let candidate = loop {
            if let Some(Event::Checked(result)) = job.poll() {
                break result.unwrap();
            }
            assert!(start.elapsed() < Duration::from_secs(5));
            thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(candidate.branch, "master");
        let mut state = Status::default();
        state.build.revision = candidate.revision.clone();
        state.build.dirty = false;
        state.accept(Event::Checked(Ok(candidate.clone())));
        assert_eq!(state.phase, Phase::Current);
        state.build.dirty = true;
        state.accept(Event::Checked(Ok(candidate)));
        assert_eq!(state.phase, Phase::Available);
        state.accept(Event::Installed(Err("build failed".into())));
        assert_eq!(state.phase, Phase::Failed);
        state.accept(Event::Installed(Ok(())));
        assert_eq!(state.phase, Phase::RestartRequired);
    }

    #[test]
    fn commands_have_a_bounded_timeout_and_cancel_only_their_process_group() {
        let (tx, _) = mpsc::channel();
        let work = Work {
            tx,
            cancel: Arc::new(AtomicBool::new(false)),
        };
        let start = Instant::now();
        assert!(
            work.run(
                Command::new("sh").args(["-c", "sleep 30 & wait"]),
                Duration::from_millis(100),
                None
            )
            .is_err()
        );
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(wait_for_parent(0).is_err());
        assert!(wait_for_parent(u32::MAX).is_err());
    }
}
