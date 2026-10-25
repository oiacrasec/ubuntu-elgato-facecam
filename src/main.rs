mod ai;
mod ai_paths;
mod camera;
mod config;
mod desktop;
mod effects;
mod engine;
#[cfg(feature = "gui")]
mod gui;
mod ipc;
mod stream;
#[cfg(feature = "tray")]
mod tray;

use anyhow::{Context, Result, bail};
use clap::Parser;
use engine::Request;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Facecam Hub — native camera controller for Ubuntu 24.04+"
)]
struct Args {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long, help = "Detect Facecam capture devices without opening a window")]
    test_camera: bool,
    #[arg(long, help = "Start the virtual camera in an owned background process")]
    start: bool,
    #[arg(long)]
    stop: bool,
    #[arg(long, help = "Print the running controller state as JSON")]
    status: bool,
    #[arg(long)]
    quit: bool,
    #[arg(long)]
    list_controls: bool,
    #[arg(long, value_name = "NAME=VALUE")]
    set_control: Option<String>,
    #[arg(
        long,
        value_name = "JSON_FILE",
        help = "Apply and save an effects JSON file through the running controller"
    )]
    set_effects: Option<PathBuf>,
    #[arg(
        long,
        help = "Check the configured AI model and runtime, including a real inference"
    )]
    check_ai: bool,
    #[arg(long, hide = true)]
    ai_worker: Option<String>,
    #[arg(long, hide = true)]
    ai_size: Option<String>,
    #[arg(long)]
    install_autostart: bool,
    #[arg(long)]
    remove_autostart: bool,
    #[arg(long, help = "Run the controller in the foreground without a GUI")]
    headless: bool,
    #[arg(long)]
    minimized: bool,
    #[arg(long)]
    debug: bool,
    #[arg(long, hide = true)]
    daemon: bool,
}

fn print_reply(reply: engine::Reply) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&reply)?);
    if !reply.ok {
        bail!("{}", reply.message);
    }
    Ok(())
}

fn background_start(path: &std::path::Path) -> Result<()> {
    if let Ok(reply) = ipc::send(&Request::Start { preview: false }) {
        return print_reply(reply);
    }
    let log_dir = path.parent().context("Configuração sem diretório")?;
    std::fs::create_dir_all(log_dir)?;
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("virtualcam.log"))?;
    let mut child = Command::new(std::env::current_exe()?)
        .arg("--daemon")
        .arg("--config")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log)
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(reply) = ipc::send(&Request::Status) {
            if let Some(error) = reply.state.error.clone() {
                let _ = ipc::send(&Request::Quit);
                return print_reply(engine::Reply {
                    ok: false,
                    message: error,
                    ..reply
                });
            }
            if reply.state.streaming && reply.state.uptime_seconds >= 1 {
                return print_reply(reply);
            }
        }
        if let Some(status) = child.try_wait()? {
            bail!(
                "Controlador encerrou ({status}); consulte {}",
                log_dir.join("virtualcam.log").display()
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    child.kill()?;
    child.wait()?;
    bail!("Controlador não iniciou dentro de 10 segundos")
}

fn run() -> Result<()> {
    let args = Args::parse();
    if let Some(json) = args.ai_worker {
        let effects = serde_json::from_str(&json)?;
        return ai::worker(&effects, args.ai_size.as_deref());
    }
    let path = args
        .config
        .clone()
        .map(Ok)
        .unwrap_or_else(config::default_path)?;
    if args.debug {
        eprintln!(
            "Facecam Hub {} · configuração: {} · runtime: {}",
            env!("CARGO_PKG_VERSION"),
            path.display(),
            ipc::runtime_dir()?.display()
        );
    }
    if args.install_autostart {
        return desktop::install_autostart();
    }
    if args.remove_autostart {
        let path = desktop::autostart_path()?;
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        return Ok(());
    }
    if args.test_camera {
        let cameras = camera::discover()?;
        println!("{}", serde_json::to_string_pretty(&cameras)?);
        if cameras.is_empty() {
            bail!("Nenhuma Facecam encontrada");
        }
        return Ok(());
    }
    if args.status {
        return print_reply(ipc::send(&Request::Status)?);
    }
    if args.check_ai {
        let config = config::Config::load(&path)?;
        ai::check(&config.effects)?;
        println!(
            "IA validada: {:?}; inferência executada",
            config.effects.ai_provider
        );
        return Ok(());
    }
    if let Some(file) = args.set_effects {
        let effects = serde_json::from_slice(&std::fs::read(file)?)?;
        return print_reply(ipc::send(&Request::SetEffects { effects })?);
    }
    if args.stop {
        return print_reply(ipc::send(&Request::Stop)?);
    }
    if args.quit {
        return print_reply(ipc::send(&Request::Quit)?);
    }
    if args.list_controls || args.set_control.is_some() {
        let config = config::Config::load(&path)?;
        let cameras = camera::discover()?;
        let camera = cameras
            .iter()
            .find(|c| c.path == config.camera_device)
            .or_else(|| cameras.first().filter(|_| config.camera_device.is_empty()))
            .context("Facecam não encontrada")?;
        let controls = camera::query_controls(&camera.path)?;
        if let Some(assignment) = args.set_control {
            let (name, value) = assignment
                .split_once('=')
                .context("Use --set-control NOME=VALOR")?;
            let value = value.parse::<i64>()?;
            if let Ok(reply) = ipc::send(&Request::SetControl {
                name: name.into(),
                value,
            }) {
                return print_reply(reply);
            }
            let control = controls
                .iter()
                .find(|c| c.name == name)
                .context("Controle não suportado")?;
            camera::set_control(&camera.path, control, value)?;
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&camera::query_controls(&camera.path)?)?
        );
        return Ok(());
    }
    if args.start && !args.headless {
        return background_start(&path);
    }
    if args.headless || args.daemon {
        let controller = ipc::Controller::spawn(&path, false)?;
        let tx = controller.tx.clone();
        ctrlc::set_handler(move || {
            let _ = tx.send(Request::Quit);
        })?;
        if args.daemon || args.start {
            controller.tx.send(Request::Start { preview: false })?;
        }
        controller.wait();
        return Ok(());
    }
    #[cfg(feature = "gui")]
    {
        let mut resume_stream = false;
        if let Ok(reply) = ipc::send(&Request::Show) {
            if reply.state.gui {
                return Ok(());
            }
            resume_stream = reply.state.streaming;
            ipc::send(&Request::Quit)?;
            let deadline = Instant::now() + Duration::from_secs(5);
            while ipc::send(&Request::Status).is_ok() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        gui::run(path, args.minimized, resume_stream)?;
        Ok(())
    }
    #[cfg(not(feature = "gui"))]
    {
        bail!("Compilação sem interface: use --headless, --start ou --help")
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
