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
    sync::{Arc, RwLock},
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
    UpdateSettings {
        virtual_device: Option<String>,
        start_minimized: Option<bool>,
        language: Option<String>,
    },
    SetControl {
        name: String,
        value: i64,
    },
    SetEffects {
        effects: crate::effects::Effects,
    },
    SetAutostart {
        enabled: bool,
    },
    PrepareInstall {
        dependency: crate::dependencies::Dependency,
    },
    PrepareUninstall {
        dependency: crate::dependencies::Dependency,
    },
    PrepareVirtualCamera {
        persistent: bool,
    },
    PrepareNvidia,
    ConfirmInstall {
        id: u64,
    },
    CancelInstall,
    ImportResource {
        kind: crate::resources::Kind,
        path: PathBuf,
    },
    RemoveImage {
        path: PathBuf,
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
    pub virtual_cameras: Vec<Camera>,
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
    #[serde(default)]
    pub ai_backend: Option<crate::ai::BackendStatus>,
    #[serde(default)]
    pub nvidia_prepared: bool,
    #[serde(default)]
    pub preparing_nvidia: bool,
    #[serde(default)]
    pub nvidia_progress: String,
    #[serde(default)]
    pub nvidia_log: Option<PathBuf>,
    #[serde(default)]
    pub autostart_enabled: Option<bool>,
    pub installation_plan: Option<crate::dependencies::InstallPlan>,
    pub installation_running: bool,
    pub installation_log: Option<PathBuf>,
    #[serde(default)]
    pub dependencies: Vec<crate::dependencies::DependencyState>,
    pub dependency_error: Option<String>,
    #[serde(default)]
    pub virtual_camera_status: crate::virtual_camera::Status,
    pub resources: Vec<crate::resources::Resource>,
    #[serde(default)]
    pub languages: Vec<crate::i18n::Language>,
    pub importing: bool,
    pub imported: Option<crate::resources::Resource>,
    pub import_revision: u64,
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
    installation: Option<crate::dependencies::Installation>,
    installation_revision: u64,
    import: Option<crate::resources::Import>,
    resource_root: PathBuf,
    last_dependency_refresh: Instant,
    removing_video_tools: bool,
    preparing_virtual_camera: bool,
    #[cfg(feature = "ai")]
    nvidia_preparation: Option<crate::nvidia_setup::Preparation>,
}

impl Engine {
    pub fn new(path: PathBuf, gui: bool) -> Result<Self> {
        let config = Config::load(&path)?;
        let language = crate::i18n::load(&config.ui.language, &path);
        let language_error = language.as_ref().err().map(|e| format!("{e:#}"));
        crate::i18n::activate(language.or_else(|_| crate::i18n::fallback())?);
        let resource_root = path
            .parent()
            .context("Configuração sem diretório")?
            .join("resources");
        let resources = crate::resources::catalog(&resource_root, &config.effects);
        let state = Snapshot {
            config,
            cameras: Vec::new(),
            virtual_cameras: Vec::new(),
            camera: None,
            modes: Vec::new(),
            mode: None,
            controls: Vec::new(),
            streaming: false,
            preview: false,
            uptime_seconds: 0,
            preview_frames: 0,
            message: "Pronto".into(),
            error: language_error,
            show_revision: 0,
            quit: false,
            gui,
            ai_compiled: cfg!(feature = "ai"),
            ai_backend: None,
            nvidia_prepared: false,
            preparing_nvidia: false,
            nvidia_progress: String::new(),
            nvidia_log: None,
            autostart_enabled: None,
            installation_plan: None,
            installation_running: false,
            installation_log: None,
            dependencies: Vec::new(),
            dependency_error: None,
            virtual_camera_status: Default::default(),
            resources,
            languages: crate::i18n::available(&path),
            importing: false,
            imported: None,
            import_revision: 0,
        };
        let mut engine = Self {
            shared: Arc::new(RwLock::new(state.clone())),
            state,
            frames: Arc::default(),
            path,
            stream: None,
            last_refresh: Instant::now(),
            write_failures: BTreeMap::new(),
            installation: None,
            import: None,
            resource_root,
            last_dependency_refresh: Instant::now(),
            removing_video_tools: false,
            preparing_virtual_camera: false,
            #[cfg(feature = "ai")]
            nvidia_preparation: None,
            installation_revision: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis()
                .try_into()?,
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

    fn refresh_dependencies(&mut self) {
        self.last_dependency_refresh = Instant::now();
        self.state.autostart_enabled = crate::desktop::autostart_enabled().ok();
        if let Ok(status) = crate::virtual_camera::discover() {
            self.state.virtual_camera_status = status;
        }
        match crate::dependencies::discover() {
            Ok(states) => {
                self.state.dependencies = states;
                self.state.dependency_error = None;
            }
            Err(error) => {
                self.state.dependencies.clear();
                self.state.dependency_error = Some(format!("{error:#}"));
            }
        }
    }

    fn prepare_dependency(
        &mut self,
        dependency: crate::dependencies::Dependency,
        action: crate::dependencies::Action,
    ) -> Result<String> {
        use crate::dependencies::{Action, Dependency};
        if self.installation.is_some() {
            bail!("Aguarde a operação APT atual terminar");
        }
        self.state.installation_plan = None;
        if action == Action::Remove && dependency == Dependency::VideoTools && self.stream.is_some()
        {
            bail!("Pare o preview e a câmera virtual antes de desinstalar FFmpeg e V4L2");
        }
        self.installation_revision = self
            .installation_revision
            .checked_add(1)
            .context("Identificador de instalação esgotado")?;
        self.refresh_dependencies();
        let directory = self.path.parent().context("Configuração sem diretório")?;
        let plan = match action {
            Action::Install => {
                crate::dependencies::prepare(dependency, self.installation_revision, directory)?
            }
            Action::Remove => crate::dependencies::prepare_removal(
                dependency,
                self.installation_revision,
                directory,
            )?,
        };
        self.state.installation_plan = Some(plan);
        Ok("Revise os comandos e confirme a operação APT".into())
    }

    fn refresh(&mut self) -> Result<()> {
        self.last_refresh = Instant::now();
        self.state.nvidia_prepared =
            crate::ai::native_available(&crate::effects::Effects::default());
        self.refresh_dependencies();
        self.state.resources =
            crate::resources::catalog(&self.resource_root, &self.state.config.effects);
        self.state.virtual_cameras = camera::discover_virtual();
        if self.stream.is_none()
            && !self
                .state
                .virtual_cameras
                .iter()
                .any(|c| c.path == self.state.config.virtual_device)
            && let Some(camera) = self.state.virtual_cameras.first()
        {
            self.state.config.virtual_device = camera.path.clone();
        }
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
        *self.frames.frame.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(())
    }

    fn start(&mut self, virtual_output: bool, preview: bool) -> Result<()> {
        if self.state.preparing_nvidia {
            bail!("Aguarde a preparação da IA RTX terminar");
        }
        if self.preparing_virtual_camera {
            bail!("Aguarde a preparação da câmera virtual terminar");
        }
        if self.removing_video_tools {
            bail!("Aguarde a desinstalação das ferramentas de vídeo terminar");
        }
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
        self.state.ai_backend = self.stream.as_ref().and_then(|s| s.ai_backend.clone());
        Ok(())
    }

    fn update_image_effects(
        &mut self,
        effects: crate::effects::Effects,
        reconfigure: bool,
    ) -> Result<()> {
        if reconfigure {
            self.execute(Request::SetEffects { effects })?;
        } else {
            let mut config = self.state.config.clone();
            config.effects = effects;
            config.save(&self.path)?;
            self.state.config = config;
        }
        Ok(())
    }

    fn execute(&mut self, request: Request) -> Result<String> {
        match request {
            Request::Status => return Ok(self.state.message.clone()),
            Request::UpdateSettings {
                virtual_device,
                start_minimized,
                language,
            } => {
                if virtual_device.is_some() && self.stream.is_some() {
                    bail!("Pare a captura antes de alterar a câmera virtual");
                }
                if virtual_device.is_none() && start_minimized.is_none() && language.is_none() {
                    return Ok("Configurações sem alterações".into());
                }
                let mut config = self.state.config.clone();
                if let Some(device) = virtual_device {
                    config.virtual_device = device;
                }
                if let Some(enabled) = start_minimized {
                    config.ui.start_minimized = enabled;
                }
                let catalog = language
                    .as_ref()
                    .map(|tag| crate::i18n::load(tag, &self.path))
                    .transpose()?;
                if let Some(language) = language {
                    config.ui.language = language;
                }
                config.save(&self.path)?;
                self.state.config = config;
                if let Some(catalog) = catalog {
                    crate::i18n::activate(catalog);
                }
                return Ok("Configurações atualizadas".into());
            }
            Request::SetAutostart { enabled } => {
                let result = if enabled {
                    crate::desktop::install_autostart()
                } else {
                    crate::desktop::remove_autostart()
                };
                self.state.autostart_enabled = crate::desktop::autostart_enabled().ok();
                result?;
                if self.state.autostart_enabled != Some(enabled) {
                    bail!(
                        "Não foi possível confirmar o início automático; atualize e tente novamente"
                    );
                }
                return Ok(if enabled {
                    "Início ao entrar no desktop ativado"
                } else {
                    "Início ao entrar no desktop desativado"
                }
                .into());
            }
            Request::ImportResource { kind, path } => {
                if self.import.is_some() {
                    bail!("Aguarde a importação atual terminar");
                }
                self.import = Some(crate::resources::Import::start(
                    self.resource_root.clone(),
                    kind,
                    path,
                )?);
                self.state.importing = true;
                return Ok("Adicionando arquivo à biblioteca…".into());
            }
            Request::RemoveImage { path } => {
                if self.import.is_some() {
                    bail!("Aguarde a importação atual terminar antes de remover uma imagem");
                }
                crate::resources::validate_owned_image(&self.resource_root, &path)?;
                let previous = self.state.config.effects.clone();
                let selected = std::path::Path::new(&previous.background_image) == path;
                let using_image =
                    selected && previous.background_mode == crate::effects::BackgroundMode::Image;
                let reconfigure = using_image && previous.background_blur;
                if selected {
                    let mut effects = previous.clone();
                    effects.background_image.clear();
                    if using_image {
                        effects.background_mode = crate::effects::BackgroundMode::Blur;
                    }
                    self.update_image_effects(effects, reconfigure)?;
                }
                if let Err(error) = crate::resources::remove_image(&self.resource_root, &path) {
                    if selected
                        && let Err(restore) = self.update_image_effects(previous, reconfigure)
                    {
                        bail!(
                            "Imagem não removida: {error:#}. Não foi possível restaurar os efeitos anteriores: {restore:#}"
                        );
                    }
                    return Err(error);
                }
                if self.state.imported.as_ref().is_some_and(|r| r.path == path) {
                    self.state.imported = None;
                }
                self.state.resources =
                    crate::resources::catalog(&self.resource_root, &self.state.config.effects);
                return Ok(if using_image {
                    "Imagem removida da biblioteca; fundo alterado para desfoque. O arquivo original foi preservado"
                } else {
                    "Imagem removida da biblioteca. O arquivo original foi preservado"
                }.into());
            }
            Request::PrepareNvidia => {
                #[cfg(not(feature = "ai"))]
                bail!("Esta compilação não inclui IA; use uma compilação com o recurso ai");
                #[cfg(feature = "ai")]
                {
                    if self.stream.is_some()
                        || self.installation.is_some()
                        || self.state.preparing_nvidia
                    {
                        bail!(
                            "Pare a captura e aguarde a operação atual antes de preparar a IA RTX"
                        );
                    }
                    crate::nvidia::ada_device()?;
                    if crate::nvidia_setup::missing_tools() {
                        return self.prepare_dependency(
                            crate::dependencies::Dependency::NvidiaTools,
                            crate::dependencies::Action::Install,
                        );
                    }
                    let log = self
                        .path
                        .parent()
                        .context("Configuração sem diretório")?
                        .join("nvidia-prepare.log");
                    self.nvidia_preparation = Some(crate::nvidia_setup::Preparation::start(
                        self.state.config.effects.clone(),
                        log.clone(),
                    )?);
                    self.state.preparing_nvidia = true;
                    self.state.nvidia_log = Some(log);
                    self.state.nvidia_progress = "Iniciando preparação da IA RTX…".into();
                    return Ok(self.state.nvidia_progress.clone());
                }
            }
            Request::PrepareInstall { dependency } => {
                return self.prepare_dependency(dependency, crate::dependencies::Action::Install);
            }
            Request::PrepareUninstall { dependency } => {
                return self.prepare_dependency(dependency, crate::dependencies::Action::Remove);
            }
            Request::PrepareVirtualCamera { persistent } => {
                if self.installation.is_some() {
                    bail!("Aguarde a operação administrativa atual terminar");
                }
                if self.stream.is_some() {
                    bail!("Pare a captura antes de preparar a câmera virtual");
                }
                self.state.installation_plan = None;
                self.installation_revision = self
                    .installation_revision
                    .checked_add(1)
                    .context("Identificador de plano esgotado")?;
                let directory = self.path.parent().context("Configuração sem diretório")?;
                self.state.installation_plan = Some(crate::virtual_camera::prepare(
                    self.installation_revision,
                    directory,
                    persistent,
                )?);
                return Ok("Revise os comandos e confirme a preparação da câmera virtual".into());
            }
            Request::ConfirmInstall { id } => {
                if self.state.preparing_nvidia {
                    bail!("Aguarde a preparação da IA RTX terminar");
                }
                if self.installation.is_some() {
                    bail!("Aguarde a operação APT atual terminar");
                }
                let plan = self
                    .state
                    .installation_plan
                    .as_ref()
                    .filter(|plan| plan.id == id)
                    .context(
                        "Plano ausente ou expirado; prepare e confirme a instalação novamente",
                    )?;
                let removing_video_tools = plan.action == crate::dependencies::Action::Remove
                    && plan.dependency == crate::dependencies::Dependency::VideoTools;
                if removing_video_tools && self.stream.is_some() {
                    bail!("Pare o preview e a câmera virtual antes de confirmar a desinstalação");
                }
                if plan.virtual_camera.is_some() && self.stream.is_some() {
                    bail!("Pare a captura antes de confirmar a preparação da câmera virtual");
                }
                let installation = crate::dependencies::Installation::start(plan.clone())?;
                self.removing_video_tools = removing_video_tools;
                self.preparing_virtual_camera = plan.virtual_camera.is_some();
                self.state.installation_log = Some(plan.log.clone());
                self.installation = Some(installation);
                self.state.installation_plan = None;
                self.state.installation_running = true;
                if self.preparing_virtual_camera {
                    return Ok("Preparando câmera virtual; confirme a autenticação administrativa do Ubuntu".into());
                }
                return Ok(
                    "Executando operação APT; confirme a autenticação administrativa do Ubuntu"
                        .into(),
                );
            }
            Request::CancelInstall => {
                if self.installation.is_some() {
                    bail!("A operação já foi iniciada; aguarde o APT concluir");
                }
                self.state.installation_plan = None;
                return Ok("Plano cancelado; nenhum comando desse plano foi iniciado".into());
            }
            Request::Refresh => {
                self.state.languages = crate::i18n::available(&self.path);
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
                effects.validate()?;
                crate::resources::materialize(&self.resource_root, &effects.lut)?;
                effects.preflight()?;
                let checked_backend = if effects.background_blur {
                    Some(crate::ai::check(&effects)?)
                } else {
                    None
                };
                let previous = self.state.config.clone();
                let outputs = (self.state.streaming, self.state.preview);
                self.stop()?;
                self.state.config.effects = effects;
                self.state.resources =
                    crate::resources::catalog(&self.resource_root, &self.state.config.effects);
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
                self.state.ai_backend = self
                    .stream
                    .as_ref()
                    .and_then(|s| s.ai_backend.clone())
                    .or(checked_backend);
                let warning = self
                    .state
                    .ai_backend
                    .as_ref()
                    .and_then(|s| s.warning.as_deref());
                return Ok(match warning {
                    Some(warning) => format!("Efeitos aplicados; configuração salva. {warning}"),
                    None => {
                        "Efeitos aplicados ao preview e à câmera virtual; configuração salva".into()
                    }
                });
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
                        "Ajustes restaurados: {applied}. Indisponíveis nesta configuração: {unavailable}"
                    ));
                }
                return Ok("Ajustes restaurados".into());
            }
            Request::Show => self.state.show_revision = self.state.show_revision.wrapping_add(1),
            Request::Quit => {
                if self.state.preparing_nvidia {
                    bail!("Aguarde a preparação da IA RTX terminar antes de encerrar o aplicativo");
                }
                if self.import.is_some() {
                    bail!("Aguarde a importação terminar antes de encerrar o aplicativo");
                }
                if self.installation.is_some() {
                    bail!("Aguarde a operação APT terminar antes de encerrar o aplicativo");
                }
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
        #[cfg(feature = "ai")]
        while let Some(event) = self.nvidia_preparation.as_ref().and_then(|job| job.poll()) {
            changed = true;
            match event {
                crate::nvidia_setup::Update::Progress(message) => {
                    self.state.nvidia_progress = message
                }
                crate::nvidia_setup::Update::Finished(result) => {
                    self.nvidia_preparation = None;
                    self.state.preparing_nvidia = false;
                    self.state.nvidia_prepared =
                        crate::ai::native_available(&crate::effects::Effects::default());
                    match result {
                        Ok(message) => {
                            self.state.nvidia_progress = message.clone();
                            self.state.message = message;
                            self.state.error = None;
                        }
                        Err(message) => {
                            self.state.nvidia_progress = "Preparação RTX não concluída".into();
                            self.state.error = Some(message);
                        }
                    }
                }
            }
        }
        if let Some(import) = &self.import
            && let Some(result) = import.poll()
        {
            self.import = None;
            self.state.importing = false;
            match result {
                Ok(resource) => {
                    self.state.resources =
                        crate::resources::catalog(&self.resource_root, &self.state.config.effects);
                    self.state.message = format!(
                        "{} adicionado à biblioteca; selecione Aplicar e salvar para usar",
                        resource.name
                    );
                    self.state.imported = Some(resource);
                    self.state.import_revision = self.state.import_revision.wrapping_add(1);
                    self.state.error = None;
                }
                Err(message) => self.state.error = Some(message),
            }
            changed = true;
        }
        if let Some(installation) = &self.installation
            && let Some(result) = installation.poll()
        {
            self.installation = None;
            self.removing_video_tools = false;
            self.state.installation_running = false;
            if self.preparing_virtual_camera {
                self.preparing_virtual_camera = false;
                let _ = self.refresh();
            } else {
                self.refresh_dependencies();
            }
            match result {
                Ok(message) => {
                    self.state.message = message;
                    self.state.error = None;
                }
                Err(message) => {
                    self.state.error = Some(message);
                }
            }
            changed = true;
        }
        if self.last_dependency_refresh.elapsed().as_secs() >= 5 {
            self.refresh_dependencies();
            changed = true;
        }
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

    fn library_image(engine: &Engine) -> PathBuf {
        let image = engine.resource_root.join("images/wrong.png");
        std::fs::create_dir_all(image.parent().unwrap()).unwrap();
        std::fs::write(&image, b"synthetic image placeholder").unwrap();
        image
    }

    #[test]
    fn image_removal_clears_saved_selection_and_preserves_other_settings() {
        let (dir, mut engine) = engine();
        let image = library_image(&engine);
        engine.state.config.effects.background_image = image.to_str().unwrap().into();
        engine.state.config.effects.background_mode = crate::effects::BackgroundMode::Image;
        engine.state.config.effects.brightness = -0.25;
        engine.state.config.save(&engine.path).unwrap();
        let reply = engine.handle(Request::RemoveImage {
            path: image.clone(),
        });
        assert!(reply.ok, "{}", reply.message);
        assert!(!image.exists());
        assert!(reply.state.config.effects.background_image.is_empty());
        assert_eq!(
            reply.state.config.effects.background_mode,
            crate::effects::BackgroundMode::Blur
        );
        assert_eq!(reply.state.config.effects.brightness, -0.25);
        assert!(!reply.state.resources.iter().any(|r| r.path == image));
        let saved = Config::load(&dir.path().join("config.json")).unwrap();
        assert_eq!(saved.effects, reply.state.config.effects);
        assert!(!engine.handle(Request::RemoveImage { path: image }).ok);
    }

    #[test]
    fn removing_unapplied_image_does_not_save_or_replace_effects() {
        let (_dir, mut engine) = engine();
        let image = library_image(&engine);
        let effects = engine.state.config.effects.clone();
        let reply = engine.handle(Request::RemoveImage {
            path: image.clone(),
        });
        assert!(reply.ok, "{}", reply.message);
        assert!(!image.exists());
        assert!(!engine.path.exists());
        assert_eq!(reply.state.config.effects, effects);
    }

    #[test]
    fn image_removal_preserves_file_and_selection_when_saving_fails() {
        let (_dir, mut engine) = engine();
        let image = library_image(&engine);
        engine.state.config.effects.background_image = image.to_str().unwrap().into();
        let previous = engine.state.config.effects.clone();
        std::fs::create_dir(&engine.path).unwrap();
        let reply = engine.handle(Request::RemoveImage {
            path: image.clone(),
        });
        assert!(!reply.ok);
        assert!(image.is_file());
        assert_eq!(reply.state.config.effects, previous);
    }

    #[test]
    fn image_removal_restores_configuration_when_unlink_fails() {
        use std::os::unix::fs::PermissionsExt;
        if nix::unistd::Uid::effective().is_root() {
            return;
        }
        let (_dir, mut engine) = engine();
        let image = library_image(&engine);
        engine.state.config.effects.background_image = image.to_str().unwrap().into();
        engine.state.config.save(&engine.path).unwrap();
        let previous = engine.state.config.effects.clone();
        let saved = std::fs::read(&engine.path).unwrap();
        let directory = image.parent().unwrap();
        std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o555)).unwrap();
        let reply = engine.handle(Request::RemoveImage {
            path: image.clone(),
        });
        std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(!reply.ok);
        assert!(image.is_file());
        assert_eq!(reply.state.config.effects, previous);
        assert_eq!(std::fs::read(&engine.path).unwrap(), saved);
    }

    #[test]
    fn nvidia_preparation_blocks_capture_and_quit_without_touching_configuration() {
        let (dir, mut engine) = engine();
        engine.state.preparing_nvidia = true;
        for request in [
            Request::Preview,
            Request::Start { preview: false },
            Request::Quit,
        ] {
            let reply = engine.handle(request);
            assert!(!reply.ok);
            assert!(reply.message.contains("preparação da IA RTX"));
            assert!(!reply.state.quit);
        }
        assert!(!dir.path().join("config.json").exists());
    }

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
    fn live_settings_persist_only_requested_fields_and_keep_state_on_failure() {
        let (dir, mut engine) = engine();
        engine.state.config.effects.brightness = -0.25;
        engine
            .state
            .config
            .extra
            .insert("custom".into(), serde_json::json!("preserve"));
        for enabled in [true, false] {
            assert!(
                engine
                    .handle(Request::UpdateSettings {
                        language: None,
                        virtual_device: None,
                        start_minimized: Some(enabled),
                    })
                    .ok
            );
            let saved = Config::load(&dir.path().join("config.json")).unwrap();
            assert_eq!(saved.ui.start_minimized, enabled);
            assert_eq!(saved.effects.brightness, -0.25);
            assert_eq!(saved.extra["custom"], "preserve");
        }
        assert!(
            engine
                .handle(Request::UpdateSettings {
                    language: None,
                    virtual_device: Some("/dev/video234".into()),
                    start_minimized: None,
                })
                .ok
        );
        let saved = std::fs::read(dir.path().join("config.json")).unwrap();
        assert_eq!(
            Config::load(&engine.path).unwrap().virtual_device,
            "/dev/video234"
        );
        let before = serde_json::to_value(&engine.state.config).unwrap();
        assert!(
            !engine
                .handle(Request::UpdateSettings {
                    language: None,
                    virtual_device: Some("/tmp/invalid-output".into()),
                    start_minimized: Some(true),
                })
                .ok
        );
        assert_eq!(serde_json::to_value(&engine.state.config).unwrap(), before);
        assert_eq!(std::fs::read(&engine.path).unwrap(), saved);
        engine.path = dir.path().join("not-a-config.json");
        std::fs::create_dir(&engine.path).unwrap();
        assert!(
            !engine
                .handle(Request::UpdateSettings {
                    language: None,
                    virtual_device: None,
                    start_minimized: Some(true),
                })
                .ok
        );
        assert_eq!(serde_json::to_value(&engine.state.config).unwrap(), before);
        assert!(engine.path.is_dir());
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
