use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub const VIRTUAL_CAMERA_LABEL: &str = "Elgato Facecam Virtual";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoConfig {
    pub framerate: u32,
    pub input_format: String,
    pub video_size: String,
    pub output_format: String,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            framerate: 30,
            input_format: "auto".into(),
            video_size: "1280x720".into(),
            output_format: "yuv420p".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub show_notifications: bool,
    pub start_minimized: bool,
    pub language: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_notifications: true,
            start_minimized: false,
            language: "pt-BR".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub schema_version: u32,
    pub camera_device: String,
    pub virtual_device: String,
    pub virtual_device_label: String,
    pub ffmpeg_params: VideoConfig,
    pub ui: UiConfig,
    pub effects: crate::effects::Effects,
    pub prompter: crate::prompter::Settings,
    pub saved_controls: BTreeMap<String, i64>,
    pub saved_hdr: Option<bool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: 3,
            camera_device: String::new(),
            virtual_device: "/dev/video10".into(),
            virtual_device_label: VIRTUAL_CAMERA_LABEL.into(),
            ffmpeg_params: VideoConfig::default(),
            ui: UiConfig::default(),
            effects: crate::effects::Effects::default(),
            prompter: crate::prompter::Settings::default(),
            saved_controls: BTreeMap::new(),
            saved_hdr: None,
            extra: BTreeMap::new(),
        }
    }
}

pub fn default_path() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .context("HOME e XDG_CONFIG_HOME não estão definidos")?;
    Ok(base.join("elgato-virtualcam/config.json"))
}

pub fn video_device(path: &str) -> bool {
    path.strip_prefix("/dev/video")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let mut config = match fs::read(path) {
            Ok(bytes) => {
                let config: Self = serde_json::from_slice(&bytes).with_context(|| {
                    format!(
                        "Configuração inválida: {} (arquivo preservado)",
                        path.display()
                    )
                })?;
                Ok(config)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).context("Não foi possível ler a configuração"),
        }?;
        if matches!(
            config.virtual_device_label.as_str(),
            "" | "VirtualCam" | "Virtual Camera"
        ) {
            config.virtual_device_label = VIRTUAL_CAMERA_LABEL.into();
        }
        crate::ai_paths::autofill(&mut config.effects);
        config.validate()?;
        Ok(config)
    }

    pub fn size(&self) -> Result<(u32, u32)> {
        let (w, h) = self
            .ffmpeg_params
            .video_size
            .split_once('x')
            .context("Resolução deve ser LARGURAxALTURA")?;
        let size = (w.parse::<u32>()?, h.parse::<u32>()?);
        if size.0 == 0 || size.1 == 0 || size.0 > 7680 || size.1 > 4320 {
            bail!("Resolução fora dos limites");
        }
        Ok(size)
    }

    pub fn validate(&self) -> Result<()> {
        if !crate::i18n::valid_tag(&self.ui.language) {
            bail!("Código de idioma inválido");
        }
        if !video_device(&self.virtual_device) {
            bail!("Saída virtual deve ser /dev/videoN");
        }
        if !self.camera_device.is_empty() && !video_device(&self.camera_device) {
            bail!("Entrada deve ser /dev/videoN");
        }
        if self.camera_device == self.virtual_device {
            bail!("Entrada e saída não podem ser iguais");
        }
        if !(1..=240).contains(&self.ffmpeg_params.framerate) {
            bail!("FPS deve estar entre 1 e 240");
        }
        if self.ffmpeg_params.output_format != "yuv420p" {
            bail!("A saída virtual suporta yuv420p nesta versão");
        }
        self.size()?;
        self.effects.validate()?;
        self.prompter.validate()?;
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        let parent = path
            .parent()
            .context("Caminho de configuração sem diretório")?;
        fs::create_dir_all(parent)?;
        if path.exists() {
            let backup = path.with_extension("json.backup");
            if !backup.exists() {
                fs::copy(path, backup)?;
            }
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        fs::rename(tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_virtual_camera_label_migrates_without_rewriting_configuration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let original = r#"{"virtual_device_label":"VirtualCam","custom":17}"#;
        fs::write(&path, original).unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config.virtual_device_label, VIRTUAL_CAMERA_LABEL);
        assert_eq!(config.extra["custom"], 17);
        assert_eq!(fs::read_to_string(path).unwrap(), original);
    }
    #[test]
    fn imports_previous_config_without_losing_extensions() {
        let c: Config = serde_json::from_str(r#"{"ffmpeg_params":{"input_format":"uyvy422","video_size":"1280x720"},"logging":{"level":"DEBUG"}}"#).unwrap();
        c.validate().unwrap();
        assert_eq!(c.ffmpeg_params.framerate, 30);
        assert_eq!(c.extra["logging"]["level"], "DEBUG");
    }
    #[test]
    fn rejects_bad_devices_and_sizes() {
        let mut c = Config {
            virtual_device: "/tmp/file".into(),
            ..Config::default()
        };
        assert!(c.validate().is_err());
        c.virtual_device = "/dev/video10".into();
        c.ffmpeg_params.video_size = "99999x99999".into();
        assert!(c.validate().is_err());
    }

    #[test]
    fn saving_preserves_original_file_as_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let original = r#"{"virtual_device":"/dev/video12","logging":{"level":"DEBUG"}}"#;
        fs::write(&path, original).unwrap();
        let mut config = Config::load(&path).unwrap();
        config.virtual_device = "/dev/video13".into();
        config.save(&path).unwrap();
        assert_eq!(
            fs::read_to_string(path.with_extension("json.backup")).unwrap(),
            original
        );
        assert_eq!(Config::load(&path).unwrap().virtual_device, "/dev/video13");
        assert_eq!(
            Config::load(&path).unwrap().extra["logging"]["level"],
            "DEBUG"
        );
    }

    #[test]
    fn fps_and_resolution_cover_negative_zero_positive_limits_and_overflow() {
        for (fps, valid) in [
            (-1i64, false),
            (0, false),
            (1, true),
            (30, true),
            (240, true),
            (241, false),
            (i64::MAX, false),
        ] {
            let json = serde_json::json!({ "ffmpeg_params": { "framerate": fps } });
            let result = serde_json::from_value::<Config>(json)
                .map_err(anyhow::Error::from)
                .and_then(|c| c.validate());
            assert_eq!(result.is_ok(), valid, "fps {fps}");
        }
        for (size, valid) in [
            ("-1x720", false),
            ("1280x-1", false),
            ("0x720", false),
            ("1280x0", false),
            ("1x1", true),
            ("7680x4320", true),
            ("7681x4320", false),
            ("7680x4321", false),
            ("4294967296x720", false),
        ] {
            let mut config = Config::default();
            config.ffmpeg_params.video_size = size.into();
            assert_eq!(config.validate().is_ok(), valid, "{size}");
        }
    }

    #[test]
    fn signed_saved_controls_survive_a_full_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let config = Config {
            saved_controls: [
                ("pan".into(), -360),
                ("brightness".into(), 0),
                ("tilt".into(), 360),
            ]
            .into(),
            ..Config::default()
        };
        config.save(&path).unwrap();
        assert_eq!(
            Config::load(&path).unwrap().saved_controls,
            config.saved_controls
        );
    }
}
