mod ai;
mod ai_paths;
#[cfg(feature = "ai")]
mod background;
mod camera;
mod config;
mod dependencies;
mod desktop;
mod effects;
mod engine;
#[cfg(feature = "ai")]
mod gpu;
#[cfg(feature = "gui")]
mod gui;
#[cfg(all(feature = "gui", debug_assertions))]
mod gui_test;
mod i18n;
mod ipc;
#[cfg(feature = "ai")]
mod nvidia;
#[cfg(feature = "ai")]
mod nvidia_model;
#[cfg(feature = "ai")]
mod nvidia_setup;
mod prompter;
#[cfg(feature = "gui")]
mod prompter_gui;
mod resources;
mod stream;
#[cfg(feature = "tray")]
mod tray;
#[cfg(feature = "ai")]
mod video_background;
mod virtual_camera;

use anyhow::{Context, Result, bail};
use clap::{Arg, ArgAction, CommandFactory, FromArgMatches, Parser};
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
    #[arg(long, hide = true)]
    virtual_camera_helper: Option<String>,
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
        value_name = "JSON_FILE",
        help = "Save Prompter script and reading settings"
    )]
    prompter_settings: Option<PathBuf>,
    #[arg(long, value_enum, help = "Control script playback and presentation")]
    prompter_action: Option<prompter::Action>,
    #[arg(
        long,
        value_name = "PERCENT",
        help = "Seek the script from 0 to 100 percent"
    )]
    prompter_position: Option<f64>,
    #[arg(
        long,
        help = "Check the configured AI model and runtime, including a real inference"
    )]
    check_ai: bool,
    #[arg(
        long,
        help = "Download and validate user-local NVIDIA RTX segmentation components"
    )]
    prepare_nvidia: bool,
    #[arg(
        long,
        value_enum,
        help = "Prepare an APT installation plan without running it"
    )]
    prepare_install: Option<dependencies::Dependency>,
    #[arg(
        long,
        value_enum,
        help = "Prepare a reviewed APT removal plan without uninstalling anything"
    )]
    prepare_uninstall: Option<dependencies::Dependency>,
    #[arg(
        long,
        help = "Review virtual-camera setup commands without executing them"
    )]
    prepare_virtual_camera: bool,
    #[arg(
        long,
        requires = "prepare_virtual_camera",
        help = "Prepare the virtual camera for this boot only"
    )]
    virtual_camera_session_only: bool,
    #[arg(
        long,
        value_name = "PLAN_ID",
        help = "Explicitly confirm a previously reviewed installation plan"
    )]
    confirm_install: Option<u64>,
    #[arg(long, help = "Discard the pending installation plan")]
    cancel_install: bool,
    #[arg(
        long,
        value_enum,
        requires = "import_file",
        help = "Import a LUT, image or video into the application library"
    )]
    import_resource: Option<resources::Kind>,
    #[arg(long, requires = "import_resource", value_name = "FILE")]
    import_file: Option<PathBuf>,
    #[arg(
        long,
        value_name = "FILE",
        help = "Remove an imported image from the application library"
    )]
    remove_image: Option<PathBuf>,
    #[arg(long, hide = true)]
    ai_worker: Option<String>,
    #[arg(long, hide = true)]
    ai_size: Option<String>,
    #[arg(long, hide = true)]
    ai_rate: Option<f64>,
    #[arg(long, hide = true)]
    decode_background: Option<String>,
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

fn print_reply(mut reply: engine::Reply, path: &std::path::Path) -> Result<()> {
    let language = std::env::var("FACECAM_LANGUAGE")
        .unwrap_or_else(|_| reply.state.config.ui.language.clone());
    i18n::activate(i18n::load(&language, path).or_else(|_| i18n::fallback())?);
    reply.message = i18n::tr(&reply.message);
    reply.state.message = i18n::tr(&reply.state.message);
    reply.state.error = reply.state.error.as_deref().map(i18n::tr);
    println!("{}", serde_json::to_string_pretty(&reply)?);
    if !reply.ok {
        bail!("{}", reply.message);
    }
    Ok(())
}

fn background_start(path: &std::path::Path) -> Result<()> {
    if let Ok(reply) = ipc::send(&Request::Start { preview: false }) {
        return print_reply(reply, path);
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
                return print_reply(
                    engine::Reply {
                        ok: false,
                        message: error,
                        ..reply
                    },
                    path,
                );
            }
            if reply.state.streaming && reply.state.uptime_seconds >= 1 {
                return print_reply(reply, path);
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

fn localized_args() -> Result<Args> {
    let raw: Vec<_> = std::env::args_os().collect();
    if raw.iter().any(|arg| {
        arg == "--virtual-camera-helper" || arg == "--ai-worker" || arg == "--decode-background"
    }) {
        return Ok(Args::try_parse_from(raw)?);
    }
    let selected = raw
        .windows(2)
        .find(|pair| pair[0] == "--config")
        .map(|pair| PathBuf::from(&pair[1]))
        .or_else(|| {
            raw.iter()
                .filter_map(|s| s.to_str())
                .find_map(|s| s.strip_prefix("--config=").map(PathBuf::from))
        });
    let path = selected
        .or_else(|| config::default_path().ok())
        .unwrap_or_else(|| PathBuf::from("config.json"));
    let language = std::env::var("FACECAM_LANGUAGE")
        .ok()
        .or_else(|| config::Config::load(&path).ok().map(|c| c.ui.language))
        .unwrap_or_else(|| "pt-BR".into());
    i18n::activate(i18n::load(&language, &path).or_else(|_| i18n::fallback())?);
    let mut command = Args::command()
        .disable_help_flag(true)
        .disable_version_flag(true);
    if let Some(about) = command.get_about() {
        let about = i18n::tr(&about.to_string());
        command = command.about(about);
    }
    let labels: Vec<_> = command
        .get_arguments()
        .filter_map(|arg| {
            arg.get_help()
                .map(|help| (arg.get_id().clone(), i18n::tr(&help.to_string())))
        })
        .collect();
    for (id, label) in labels {
        command = command.mut_arg(id, |arg| arg.help(label));
    }
    command = command
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .action(ArgAction::Help)
                .help(i18n::tr("Print help")),
        )
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::Version)
                .help(i18n::tr("Print version")),
        );
    let ids: Vec<_> = command
        .get_arguments()
        .map(|arg| arg.get_id().clone())
        .collect();
    for id in ids {
        command = command.mut_arg(id, |arg| arg.help_heading(i18n::tr("Options")));
    }
    let template = "{about-with-newline}\n{usage-heading} {usage}\n\n{all-args}"
        .replace("{usage-heading}", &i18n::tr("Usage:"));
    let matches = command.help_template(template).get_matches();
    Ok(Args::from_arg_matches(&matches)?)
}

fn run() -> Result<()> {
    let args = localized_args()?;
    if let Some(payload) = &args.virtual_camera_helper {
        return virtual_camera::run_privileged(payload);
    }
    if let Some(json) = args.decode_background {
        #[cfg(feature = "ai")]
        return video_background::decode(&serde_json::from_str(&json)?);
        #[cfg(not(feature = "ai"))]
        {
            let _ = json;
            bail!("Esta compilação não inclui fundos com IA");
        }
    }
    if let Some(json) = args.ai_worker {
        let effects = serde_json::from_str(&json)?;
        return ai::worker(&effects, args.ai_size.as_deref(), args.ai_rate);
    }
    let path = args
        .config
        .clone()
        .map(Ok)
        .unwrap_or_else(config::default_path)?;
    if args.debug {
        eprintln!(
            "{}",
            i18n::tr(&format!(
                "Facecam Hub {} · configuração: {} · runtime: {}",
                env!("CARGO_PKG_VERSION"),
                path.display(),
                ipc::runtime_dir()?.display()
            ))
        );
    }
    if args.install_autostart || args.remove_autostart {
        if let Ok(reply) = ipc::send(&Request::SetAutostart {
            enabled: args.install_autostart,
        }) {
            return print_reply(reply, &path);
        }
        return if args.install_autostart {
            desktop::install_autostart()
        } else {
            desktop::remove_autostart()
        };
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
        return print_reply(ipc::send(&Request::Status)?, &path);
    }
    if args.prepare_nvidia {
        return print_reply(ipc::send(&Request::PrepareNvidia)?, &path);
    }
    if let Some(dependency) = args.prepare_install {
        return print_reply(ipc::send(&Request::PrepareInstall { dependency })?, &path);
    }
    if let Some(dependency) = args.prepare_uninstall {
        return print_reply(ipc::send(&Request::PrepareUninstall { dependency })?, &path);
    }
    if args.prepare_virtual_camera {
        return print_reply(
            ipc::send(&Request::PrepareVirtualCamera {
                persistent: !args.virtual_camera_session_only,
            })?,
            &path,
        );
    }
    if let Some(id) = args.confirm_install {
        return print_reply(ipc::send(&Request::ConfirmInstall { id })?, &path);
    }
    if args.cancel_install {
        return print_reply(ipc::send(&Request::CancelInstall)?, &path);
    }
    if let Some(image) = args.remove_image {
        return print_reply(ipc::send(&Request::RemoveImage { path: image })?, &path);
    }
    if let (Some(kind), Some(file)) = (args.import_resource, args.import_file) {
        return print_reply(
            ipc::send(&Request::ImportResource { kind, path: file })?,
            &path,
        );
    }
    if let Some(file) = args.prompter_settings {
        let settings = serde_json::from_slice(&std::fs::read(file)?)?;
        return print_reply(ipc::send(&Request::UpdatePrompter { settings })?, &path);
    }
    if let Some(action) = args.prompter_action {
        return print_reply(ipc::send(&Request::ControlPrompter { action })?, &path);
    }
    if let Some(percent) = args.prompter_position {
        if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
            bail!("Posição do roteiro deve estar entre 0 e 100%");
        }
        return print_reply(
            ipc::send(&Request::SeekPrompter {
                position: percent / 100.0,
            })?,
            &path,
        );
    }
    if args.check_ai {
        let config = config::Config::load(&path)?;
        let status = ai::check(&config.effects)?;
        println!(
            "{}",
            i18n::tr(&format!(
                "IA validada: {:?}, {:?}; inferência executada",
                status.backend, config.effects.ai_provider
            ))
        );
        if let Some(warning) = status.warning {
            eprintln!("{}", i18n::tr(&warning));
        }
        return Ok(());
    }
    if let Some(file) = args.set_effects {
        let effects = serde_json::from_slice(&std::fs::read(file)?)?;
        return print_reply(ipc::send(&Request::SetEffects { effects })?, &path);
    }
    if args.stop {
        return print_reply(ipc::send(&Request::Stop)?, &path);
    }
    if args.quit {
        return print_reply(ipc::send(&Request::Quit)?, &path);
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
                return print_reply(reply, &path);
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
        eprintln!("{}", i18n::tr(&format!("{error:#}")));
        std::process::exit(1);
    }
}
