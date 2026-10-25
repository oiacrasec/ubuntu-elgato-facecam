use crate::{
    camera::{self, Camera, Control, Mode},
    config::Config,
    stream::{LatestFrame, Stream},
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
    time::Instant,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Request {
    Status,
    Refresh,
    Start {
        #[serde(default)]
        preview: bool,
    },
    Preview,
    Stop,
    Configure {
        config: Box<Config>,
        #[serde(default)]
        save: bool,
    },
    SetControl {
        name: String,
        value: i64,
    },
    SetEffects {
        effects: crate::effects::Effects,
    },
    Save,
    Restore,
    Show,
    Quit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub config: Config,
    pub cameras: Vec<Camera>,
    pub camera: Option<Camera>,
    pub modes: Vec<Mode>,
    pub mode: Option<Mode>,
    pub controls: Vec<Control>,
    pub streaming: bool,
    pub preview: bool,
    pub uptime_seconds: u64,
    pub preview_frames: u64,
    pub message: String,
    pub error: Option<String>,
    pub show_revision: u64,
    pub quit: bool,
    pub gui: bool,
    pub ai_compiled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reply {
    pub ok: bool,
    pub message: String,
    pub state: Snapshot,
}

pub type SharedState = Arc<RwLock<Snapshot>>;

pub struct Engine {
    pub state: Snapshot,
    pub shared: SharedState,
    pub frames: LatestFrame,
    path: PathBuf,
    stream: Option<Stream>,
    last_refresh: Instant,
    write_failures: BTreeMap<String, String>,
}

impl Engine {
    pub fn new(path: PathBuf, gui: bool) -> Result<Self> {
        let config = Config::load(&path)?;
        let state = Snapshot {
            config,
            cameras: Vec::new(),
            camera: None,
            modes: Vec::new(),
            mode: None,
            controls: Vec::new(),
            streaming: false,
            preview: false,
            uptime_seconds: 0,
            preview_frames: 0,
            message: "Pronto".into(),
            error: None,
            show_revision: 0,
            quit: false,
            gui,
            ai_compiled: cfg!(feature = "ai"),
        };
        let mut engine = Self {
            shared: Arc::new(RwLock::new(state.clone())),
            state,
            frames: Arc::new(Mutex::new(None)),
            path,
            stream: None,
            last_refresh: Instant::now(),
            write_failures: BTreeMap::new(),
        };
        if let Err(e) = engine.refresh() {
            engine.state.error = Some(format!("{e:#}"));
        }
        engine.publish();
        Ok(engine)
    }

    fn publish(&self) {
        *self.shared.write().unwrap_or_else(|e| e.into_inner()) = self.state.clone();
    }

    fn update_controls(&mut self, mut controls: Vec<Control>) {
        for control in &mut controls {
            control.write_error = self.write_failures.get(&control.name).cloned();
        }
        self.state.controls = controls;
    }

    fn query_controls(&mut self) -> Result<()> {
        let device = &self
            .state
            .camera
            .as_ref()
            .context("Câmera não encontrada")?
            .path;
        let controls = camera::query_controls(device)?;
        self.update_controls(controls);
        Ok(())
    }

    fn write_control(&mut self, name: &str, value: i64) -> Result<()> {
        let device = &self
            .state
            .camera
            .as_ref()
            .context("Câmera não encontrada")?
            .path;
        let control = self
            .state
            .controls
            .iter()
            .find(|c| c.name == name)
            .context("Controle não suportado")?;
        if let Err(error) = camera::set_control(device, control, value) {
            if error.is::<camera::V4l2CommandError>() {
                let reason = format!(
                    "A câmera recusou {name} = {value} nesta configuração. Clique em Atualizar dispositivos e ajustes para tentar novamente. {error}"
                );
                self.write_failures.insert(name.into(), reason.clone());
                // Preserve the last known hardware value even if the readback also fails.
                let controls =
                    camera::query_controls(device).unwrap_or_else(|_| self.state.controls.clone());
                self.update_controls(controls);
                bail!("{reason}");
            }
            return Err(error);
        }
        self.query_controls()
    }

    fn refresh(&mut self) -> Result<()> {
        self.last_refresh = Instant::now();
        let cameras = camera::discover()?;
        let camera = cameras
            .iter()
            .find(|c| c.path == self.state.config.camera_device)
            .or_else(|| {
                cameras
                    .first()
                    .filter(|_| self.state.config.camera_device.is_empty())
            })
            .cloned();
        if self.state.camera != camera {
            self.write_failures.clear();
        }
        self.state.cameras = cameras;
        self.state.camera = camera;
        self.state.controls.clear();
        self.state.modes.clear();
        self.state.mode = None;
        if let Some(camera) = &self.state.camera {
            self.state.modes =
                camera::parse_modes(&camera::v4l2(&camera.path, &["--list-formats-ext"])?);
            self.state.mode = Some(camera::choose_mode(&self.state.modes, &self.state.config)?);
            self.query_controls()?;
        }
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            stream.stop()?;
        }
        self.state.streaming = false;
        self.state.preview = false;
        self.state.uptime_seconds = 0;
        self.state.preview_frames = 0;
        *self.frames.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(())
    }

    fn start(&mut self, virtual_output: bool, preview: bool) -> Result<()> {
        if self.stream.is_some()
            && self.state.streaming == virtual_output
            && self.state.preview == preview
        {
            return Ok(());
        }
        if virtual_output {
            crate::stream::verify_virtual_device(&self.state.config.virtual_device)?;
        }
        self.stop()?;
        self.refresh()?;
        let camera = self
            .state
            .camera
            .as_ref()
            .context("Facecam não encontrada. Conecte a câmera e atualize os dispositivos.")?;
        let mode = self
            .state
            .mode
            .as_ref()
            .context("Nenhum formato compatível")?;
        let parent = self.path.parent().context("Configuração sem diretório")?;
        std::fs::create_dir_all(parent)?;
        self.stream = Some(Stream::start(
            &camera.path,
            mode,
            &self.state.config,
            virtual_output,
            preview,
            self.frames.clone(),
            &parent.join("virtualcam.err.log"),
        )?);
        self.state.streaming = virtual_output;
        self.state.preview = preview;
        Ok(())
    }

    fn execute(&mut self, request: Request) -> Result<String> {
        match request {
            Request::Status => return Ok(self.state.message.clone()),
            Request::Refresh => {
                crate::ai_paths::autofill(&mut self.state.config.effects);
                self.write_failures.clear();
                self.refresh()?;
            }
            Request::Start { preview } => {
                self.start(true, preview)?;
                return Ok("Câmera virtual iniciando".into());
            }
            Request::Preview => {
                self.start(false, true)?;
                return Ok("Preview iniciando".into());
            }
            Request::Stop => {
                self.stop()?;
                return Ok("Captura encerrada".into());
            }
            Request::Configure { mut config, save } => {
                if self.stream.is_some() {
                    bail!("Pare a captura antes de alterar o formato");
                }
                crate::ai_paths::autofill(&mut config.effects);
                config.validate()?;
                self.state.config = *config;
                self.write_failures.clear();
                self.refresh()?;
                if save {
                    self.state.config.save(&self.path)?;
                }
            }
            Request::SetControl { name, value } => {
                self.write_control(&name, value)?;
                let actual = self
                    .state
                    .controls
                    .iter()
                    .find(|c| c.name == name)
                    .context("Controle ausente na leitura de retorno")?
                    .value;
                return Ok(format!("Ajuste aplicado: {name} = {actual}"));
            }
            Request::SetEffects { mut effects } => {
                crate::ai_paths::autofill(&mut effects);
                effects.preflight()?;
                if effects.background_blur {
                    crate::ai::check(&effects)?;
                }
                let previous = self.state.config.clone();
                let outputs = (self.state.streaming, self.state.preview);
                self.stop()?;
                self.state.config.effects = effects;
                let result = (|| {
                    if outputs.0 || outputs.1 {
                        self.start(outputs.0, outputs.1)?;
                    }
                    self.state.config.save(&self.path)
                })();
                if let Err(error) = result {
                    self.stop()?;
                    self.state.config = previous;
                    if (outputs.0 || outputs.1)
                        && let Err(restore) = self.start(outputs.0, outputs.1)
                    {
                        bail!(
                            "Efeitos recusados: {error:#}. Não foi possível retomar a captura anterior: {restore:#}"
                        );
                    }
                    return Err(error);
                }
                return Ok(
                    "Efeitos aplicados ao preview e à câmera virtual; configuração salva".into(),
                );
            }
            Request::Save => {
                if self.state.camera.is_some() {
                    self.query_controls()?;
                    self.state.config.saved_controls = self
                        .state
                        .controls
                        .iter()
                        .filter(|c| c.writable())
                        .map(|c| (c.name.clone(), c.value))
                        .collect();
                }
                self.state.config.save(&self.path)?;
                return Ok("Configuração e ajustes salvos neste computador".into());
            }
            Request::Restore => {
                self.state
                    .camera
                    .as_ref()
                    .context("Câmera não encontrada")?;
                if self.state.config.saved_controls.is_empty() {
                    bail!("Nenhum ajuste salvo neste computador");
                }
                let mut skipped = Vec::new();
                let mut applied = 0;
                for (name, value) in self.state.config.saved_controls.clone() {
                    if self.write_failures.contains_key(&name) {
                        skipped.push(name);
                        continue;
                    }
                    self.query_controls()?;
                    if let Err(error) = self.write_control(&name, value) {
                        if self.write_failures.contains_key(&name) {
                            skipped.push(name);
                            continue;
                        }
                        return Err(error);
                    }
                    applied += 1;
                }
                if !skipped.is_empty() {
                    let unavailable = skipped.join(", ");
                    if applied == 0 {
                        bail!(
                            "Nenhum ajuste aplicado. Indisponíveis nesta configuração: {unavailable}"
                        );
                    }
                    return Ok(format!(
                        "{applied} ajustes restaurados. Indisponíveis nesta configuração: {unavailable}"
                    ));
                }
                return Ok("Ajustes restaurados".into());
            }
            Request::Show => self.state.show_revision = self.state.show_revision.wrapping_add(1),
            Request::Quit => {
                self.stop()?;
                self.state.quit = true;
            }
        }
        Ok("Pronto".into())
    }

    pub fn handle(&mut self, request: Request) -> Reply {
        let status_only = matches!(request, Request::Status);
        let was_streaming = self.state.streaming;
        let result = self.execute(request);
        let ok = result.is_ok();
        let message = match result {
            Ok(m) => m,
            Err(e) => format!("{e:#}"),
        };
        if !status_only {
            if ok {
                self.state.error = None;
                self.state.message = message.clone();
            } else {
                self.state.error = Some(message.clone());
            }
            eprintln!("{message}");
            if self.state.config.ui.show_notifications && was_streaming != self.state.streaming {
                let text = if self.state.streaming {
                    "Câmera virtual iniciada"
                } else {
                    "Câmera virtual encerrada"
                };
                std::thread::spawn(move || {
                    let _ = std::process::Command::new("notify-send")
                        .args(["--app-name=Facecam Hub", "Facecam Hub", text])
                        .status();
                });
            }
        }
        self.publish();
        Reply {
            ok,
            message,
            state: self.state.clone(),
        }
    }

    pub fn tick(&mut self) -> bool {
        let mut changed = false;
        if let Some(stream) = &mut self.stream {
            self.state.uptime_seconds = stream.started.elapsed().as_secs();
            self.state.preview_frames = stream
                .frames_received
                .load(std::sync::atomic::Ordering::Relaxed);
            if let Err(e) = stream.poll() {
                let message = format!("{e:#}");
                let _ = self.stop();
                self.state.error = Some(message);
                changed = true;
            }
        } else if self.last_refresh.elapsed().as_secs() >= 5 {
            changed = true;
            if let Err(e) = self.refresh() {
                self.state.error = Some(format!("{e:#}"));
            }
        }
        self.publish();
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> (tempfile::TempDir, Engine) {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = Engine::new(dir.path().join("config.json"), false).unwrap();
        engine.state.camera = Some(Camera {
            name: "Fixture".into(),
            path: "/dev/not-a-camera".into(),
        });
        engine.state.controls = camera::parse_controls(
            "backlight_compensation 0x0098091c (int) : min=0 max=1 step=1 default=0 value=1",
        );
        (dir, engine)
    }

    #[test]
    fn rejected_write_stays_disabled_after_metadata_refresh_and_does_not_replace_readback() {
        let (_dir, mut engine) = engine();
        engine.write_failures.insert(
            "backlight_compensation".into(),
            "Recusado; atualize para tentar novamente".into(),
        );
        for _ in 0..2 {
            engine.update_controls(camera::parse_controls(
                "backlight_compensation 0x0098091c (int) : min=0 max=1 step=1 default=0 value=1",
            ));
            assert_eq!(engine.state.controls[0].value, 1);
            assert!(!engine.state.controls[0].writable());
        }
        let reply = engine.handle(Request::SetControl {
            name: "backlight_compensation".into(),
            value: 0,
        });
        assert!(!reply.ok);
        assert!(reply.message.contains("Recusado"));
        assert!(!reply.message.contains("v4l2-ctl"));
        engine.write_failures.clear();
        engine.update_controls(camera::parse_controls(
            "backlight_compensation 0x0098091c (int) : min=0 max=1 step=1 default=0 value=1",
        ));
        assert!(engine.state.controls[0].writable());
    }

    #[test]
    fn invalid_input_does_not_mark_a_working_control_as_rejected() {
        let (_dir, mut engine) = engine();
        let reply = engine.handle(Request::SetControl {
            name: "backlight_compensation".into(),
            value: 2,
        });
        assert!(!reply.ok);
        assert!(reply.message.contains("Valor inválido"));
        assert!(engine.write_failures.is_empty());
        assert!(engine.state.controls[0].writable());
    }

    #[test]
    fn effects_round_trip_and_invalid_updates_preserve_saved_configuration() {
        let (dir, mut engine) = engine();
        let effects = crate::effects::Effects {
            brightness: -0.25,
            ai_model: "fixture.onnx".into(),
            ai_runtime: "fixture.so".into(),
            saturation: 0.0,
            contrast: 1.5,
            ..Default::default()
        };
        assert!(
            engine
                .handle(Request::SetEffects {
                    effects: effects.clone()
                })
                .ok
        );
        let before = std::fs::read(dir.path().join("config.json")).unwrap();
        let invalid = crate::effects::Effects {
            brightness: -2.0,
            ..effects.clone()
        };
        assert!(!engine.handle(Request::SetEffects { effects: invalid }).ok);
        assert_eq!(engine.state.config.effects, effects);
        assert_eq!(
            std::fs::read(dir.path().join("config.json")).unwrap(),
            before
        );
    }

    #[test]
    fn restore_of_only_rejected_controls_does_not_retry_or_report_success() {
        let (_dir, mut engine) = engine();
        engine
            .write_failures
            .insert("backlight_compensation".into(), "Recusado".into());
        engine
            .state
            .config
            .saved_controls
            .insert("backlight_compensation".into(), 0);
        let reply = engine.handle(Request::Restore);
        assert!(!reply.ok);
        assert!(reply.message.contains("Nenhum ajuste aplicado"));
        assert!(reply.message.contains("backlight_compensation"));
        assert!(!reply.message.contains("v4l2-ctl"));
    }
}
