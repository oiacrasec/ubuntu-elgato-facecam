use crate::engine::{Engine, Reply, Request};
#[cfg(feature = "gui")]
use crate::{engine::SharedState, stream::LatestFrame};
use anyhow::{Context, Result, bail};
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

pub fn runtime_dir() -> Result<PathBuf> {
    let uid = nix::unistd::Uid::current();
    let root = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = root.join(format!("elgato-virtualcam-{uid}"));
    fs::create_dir_all(&dir)?;
    use std::os::unix::fs::MetadataExt;
    if fs::symlink_metadata(&dir)?.uid() != uid.as_raw()
        || fs::symlink_metadata(&dir)?.file_type().is_symlink()
    {
        bail!("Diretório de execução não pertence ao usuário");
    }
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
    Ok(dir)
}

pub fn send(request: &Request) -> Result<Reply> {
    let mut stream = UnixStream::connect(runtime_dir()?.join("control.sock"))
        .context("Aplicativo não está em execução")?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    serde_json::to_writer(&mut stream, request)?;
    stream.write_all(b"\n")?;
    let mut line = String::new();
    BufReader::new(stream)
        .take(256 * 1024)
        .read_line(&mut line)?;
    Ok(serde_json::from_str(&line)?)
}

struct SocketGuard {
    path: PathBuf,
    _lock: File,
}
impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

use crate::stream::Wake;

pub struct Controller {
    pub tx: Sender<Request>,
    #[cfg(feature = "gui")]
    pub shared: SharedState,
    #[cfg(feature = "gui")]
    pub frames: LatestFrame,
    #[cfg(feature = "gui")]
    pub wake: Wake,
    thread: Option<JoinHandle<()>>,
}

impl Controller {
    pub fn spawn(config_path: &Path, gui: bool) -> Result<Self> {
        let dir = runtime_dir()?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(dir.join("instance.lock"))?;
        lock.try_lock_exclusive()
            .context("Outra instância já está em execução")?;
        let path = dir.join("control.sock");
        if path.exists() {
            fs::remove_file(&path)?;
        }
        let listener = UnixListener::bind(&path)?;
        listener.set_nonblocking(true)?;
        let guard = SocketGuard { path, _lock: lock };
        let engine = Engine::new(config_path.into(), gui)?;
        #[cfg(feature = "gui")]
        let shared = engine.shared.clone();
        #[cfg(feature = "gui")]
        let frames = engine.frames.clone();
        let (tx, rx) = mpsc::channel();
        let wake = engine.frames.wake.clone();
        let worker_wake = wake.clone();
        let thread = thread::spawn(move || run(engine, rx, listener, guard, worker_wake));
        Ok(Self {
            tx,
            #[cfg(feature = "gui")]
            shared,
            #[cfg(feature = "gui")]
            frames,
            #[cfg(feature = "gui")]
            wake,
            thread: Some(thread),
        })
    }

    pub fn wait(mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        let _ = self.tx.send(Request::Quit);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(mut stream: UnixStream, engine: &mut Engine) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_millis(200)))?;
    stream.set_write_timeout(Some(Duration::from_millis(200)))?;
    let mut line = String::new();
    BufReader::new(&mut stream)
        .take(64 * 1024)
        .read_line(&mut line)?;
    let request = serde_json::from_str(&line)?;
    serde_json::to_writer(&mut stream, &engine.handle(request))?;
    stream.write_all(b"\n")?;
    Ok(())
}

fn wake_gui(wake: &Wake) {
    if let Some(callback) = &*wake.lock().unwrap_or_else(|e| e.into_inner()) {
        callback();
    }
}

fn run(
    mut engine: Engine,
    rx: Receiver<Request>,
    listener: UnixListener,
    _guard: SocketGuard,
    wake: Wake,
) {
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(request) => {
                engine.handle(request);
                wake_gui(&wake);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        for _ in 0..8 {
            match listener.accept() {
                Ok((stream, _)) => {
                    if let Err(e) = serve(stream, &mut engine) {
                        eprintln!("Controle local: {e}");
                    }
                    wake_gui(&wake);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    eprintln!("Socket: {e}");
                    break;
                }
            }
        }
        if engine.tick() {
            wake_gui(&wake);
        }
        if engine.state.quit {
            break;
        }
    }
}
