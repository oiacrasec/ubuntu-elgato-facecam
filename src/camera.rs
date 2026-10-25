use crate::config::Config;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt, fs,
    process::{Command, Output},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Camera {
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Mode {
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
}

#[cfg(feature = "gui")]
impl Mode {
    pub fn label(&self) -> String {
        format!(
            "{}×{} · {} fps · {}",
            self.width, self.height, self.fps, self.format
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Control {
    pub name: String,
    pub kind: String,
    pub min: i64,
    pub max: i64,
    pub step: i64,
    pub default: i64,
    pub value: i64,
    pub inactive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write_error: Option<String>,
    pub menu: BTreeMap<i64, String>,
}

impl Control {
    pub fn writable(&self) -> bool {
        !self.inactive && self.write_error.is_none()
    }
}

#[derive(Debug)]
pub struct V4l2CommandError {
    device: String,
    detail: String,
}

impl fmt::Display for V4l2CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v4l2-ctl ({}): {}", self.device, self.detail)
    }
}

impl std::error::Error for V4l2CommandError {}

fn v4l2_output(device: &str, out: Output) -> Result<String> {
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();
        let detail = if stderr.is_empty() {
            String::from_utf8_lossy(&out.stdout).trim().to_owned()
        } else {
            stderr
        };
        return Err(V4l2CommandError {
            device: device.into(),
            detail,
        }
        .into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn v4l2(device: &str, args: &[&str]) -> Result<String> {
    let out = Command::new("v4l2-ctl")
        .env("LC_ALL", "C")
        .args(["--device", device])
        .args(args)
        .output()
        .context("Instale v4l-utils: v4l2-ctl não está disponível")?;
    v4l2_output(device, out)
}

pub fn discover() -> Result<Vec<Camera>> {
    let mut cameras = Vec::new();
    let entries = match fs::read_dir("/sys/class/video4linux") {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(cameras),
        Err(e) => return Err(e.into()),
    };
    for entry in entries {
        let entry = entry?;
        let name = fs::read_to_string(entry.path().join("name"))?
            .trim()
            .to_owned();
        if !name.to_ascii_lowercase().contains("elgato facecam") {
            continue;
        }
        let path = format!("/dev/{}", entry.file_name().to_string_lossy());
        // Metadata nodes expose the same name but cannot report Video Capture format.
        if v4l2(&path, &["--get-fmt-video"]).is_ok() {
            cameras.push(Camera { name, path });
        }
    }
    cameras.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(cameras)
}

pub fn discover_virtual() -> Vec<Camera> {
    let mut cameras: Vec<_> = fs::read_dir("/sys/class/video4linux")
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = format!("/dev/{}", entry.file_name().to_string_lossy());
            if crate::stream::verify_virtual_device(&path).is_err() {
                return None;
            }
            let name = fs::read_to_string(entry.path().join("name"))
                .ok()?
                .trim()
                .to_owned();
            Some(Camera { name, path })
        })
        .collect();
    cameras.sort_by(|a, b| a.path.cmp(&b.path));
    cameras
}

pub fn parse_modes(text: &str) -> Vec<Mode> {
    let mut modes = Vec::new();
    let mut format = "";
    let mut size = None;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            format = match line.split('\'').nth(1) {
                Some("MJPG") => "mjpeg",
                Some("YUYV") => "yuyv422",
                Some("UYVY") => "uyvy422",
                Some("NV12") => "nv12",
                Some("RGB3") => "rgb24",
                _ => "",
            };
            size = None;
        } else if let Some(s) = line.strip_prefix("Size: Discrete ") {
            size = s
                .split_once('x')
                .and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)));
        } else if let (Some((w, h)), Some((_, rest))) = (size, line.split_once('('))
            && let Some(fps) = rest
                .split_whitespace()
                .next()
                .and_then(|f| f.parse::<f64>().ok())
            && !format.is_empty()
            && fps.is_finite()
            && fps > 0.0
            && w > 0
            && h > 0
        {
            modes.push(Mode {
                format: format.into(),
                width: w,
                height: h,
                fps,
            });
        }
    }
    modes
}

pub fn choose_mode(modes: &[Mode], config: &Config) -> Result<Mode> {
    let (w, h) = config.size()?;
    let fps = f64::from(config.ffmpeg_params.framerate);
    modes
        .iter()
        .min_by_key(|m| {
            let exact_size = m.width == w && m.height == h;
            let size_diff = u64::from(m.width.abs_diff(w)) + u64::from(m.height.abs_diff(h));
            let format = if m.format == config.ffmpeg_params.input_format {
                0
            } else if m.format == "mjpeg" {
                1
            } else {
                2
            };
            (
                !exact_size,
                size_diff,
                ((m.fps - fps).abs() * 1000.0) as u64,
                format,
            )
        })
        .cloned()
        .context("A câmera não expõe formatos discretos compatíveis")
}

fn number(line: &str, key: &str) -> Option<i64> {
    line.split_whitespace()
        .find_map(|s| s.strip_prefix(key)?.parse().ok())
}

pub fn parse_controls(text: &str) -> Vec<Control> {
    let mut controls: Vec<Control> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.contains(" 0x") && line.contains(" : ") {
            let name = line.split_whitespace().next().unwrap_or_default();
            let kind = line
                .split('(')
                .nth(1)
                .and_then(|s| s.split(')').next())
                .unwrap_or_default();
            if !matches!(kind, "int" | "bool" | "menu" | "intmenu") {
                continue;
            }
            controls.push(Control {
                name: name.into(),
                kind: kind.into(),
                min: number(line, "min=").unwrap_or(0),
                max: number(line, "max=").unwrap_or(1),
                step: number(line, "step=").unwrap_or(1).max(1),
                default: number(line, "default=").unwrap_or(0),
                value: number(line, "value=").unwrap_or(0),
                inactive: line.contains("inactive")
                    || line.contains("read-only")
                    || line.contains("disabled"),
                write_error: None,
                menu: BTreeMap::new(),
            });
        } else if let Some((value, label)) = line.split_once(':')
            && let Ok(value) = value.parse::<i64>()
            && let Some(control) = controls.last_mut()
        {
            control.menu.insert(value, label.trim().into());
        }
    }
    controls
}

pub fn query_controls(device: &str) -> Result<Vec<Control>> {
    Ok(parse_controls(&v4l2(device, &["--list-ctrls-menus"])?))
}

pub fn set_control(device: &str, control: &Control, value: i64) -> Result<()> {
    validate_control(control, value)?;
    v4l2(
        device,
        &["--set-ctrl", &format!("{}={value}", control.name)],
    )?;
    Ok(())
}

fn validate_control(control: &Control, value: i64) -> Result<()> {
    if let Some(reason) = &control.write_error {
        bail!("{reason}");
    }
    if control.inactive {
        bail!("{} está inativo; confira o modo automático", control.name);
    }
    if control.step <= 0
        || value < control.min
        || value > control.max
        || (i128::from(value) - i128::from(control.min)) % i128::from(control.step) != 0
    {
        bail!("Valor inválido para {}", control.name);
    }
    if !control.menu.is_empty() && !control.menu.contains_key(&value) {
        bail!("Opção de menu inválida");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    #[test]
    fn signed_controls_accept_negative_zero_positive_and_reject_outside_range_or_step() {
        let controls = parse_controls(
            "pan_absolute 0x009a0908 (int) : min=-3600 max=3600 step=360 default=0 value=-360",
        );
        let control = &controls[0];
        assert_eq!(control.value, -360);
        for (value, valid) in [
            (i64::MIN, false),
            (-3960, false),
            (-3600, true),
            (-360, true),
            (-1, false),
            (0, true),
            (1, false),
            (360, true),
            (3600, true),
            (3960, false),
            (i64::MAX, false),
        ] {
            assert_eq!(validate_control(control, value).is_ok(), valid, "{value}");
        }
    }

    #[test]
    fn validation_handles_extreme_signed_ranges_and_malformed_steps_without_panicking() {
        let mut control =
            parse_controls("brightness 0x00980900 (int) : min=0 max=10 step=1 default=0 value=0")
                .remove(0);
        control.min = i64::MIN;
        control.max = i64::MAX;
        for value in [i64::MIN, -1, 0, 1, i64::MAX] {
            assert!(validate_control(&control, value).is_ok());
        }
        for step in [i64::MIN, -1, 0] {
            control.step = step;
            assert!(validate_control(&control, 0).is_err());
        }
    }

    #[test]
    fn boolean_and_sparse_signed_menus_do_not_accept_other_in_range_values() {
        let boolean = parse_controls("privacy 0x009a0910 (bool) : default=0 value=0").remove(0);
        for value in [-1, 0, 1, 2] {
            assert_eq!(
                validate_control(&boolean, value).is_ok(),
                (0..=1).contains(&value)
            );
        }
        let menu = parse_controls("mode 0x009a0901 (menu) : min=-2 max=2 default=0 value=0\n  -2: Low\n  0: Normal\n  2: High").remove(0);
        for value in [-3, -2, -1, 0, 1, 2, 3] {
            assert_eq!(
                validate_control(&menu, value).is_ok(),
                [-2, 0, 2].contains(&value)
            );
        }
    }

    #[test]
    fn classifies_device_rejection_without_losing_the_driver_diagnostic() {
        let error = v4l2_output(
            "/dev/video0",
            Output {
                status: std::process::ExitStatus::from_raw(255 << 8),
                stdout: b"VIDIOC_S_CTRL: failed".to_vec(),
                stderr: b"backlight_compensation: Numerical result out of range\n".to_vec(),
            },
        )
        .unwrap_err();
        assert!(error.is::<V4l2CommandError>());
        assert!(
            error
                .to_string()
                .contains("backlight_compensation: Numerical result out of range")
        );
    }

    #[test]
    fn advertised_range_does_not_override_an_observed_write_rejection() {
        let mut controls = parse_controls(
            "backlight_compensation 0x0098091c (int) : min=0 max=1 step=1 default=0 value=1",
        );
        let control = &mut controls[0];
        assert!(control.writable());
        control.write_error =
            Some("A câmera recusou este ajuste; atualize para tentar novamente".into());
        assert!(!control.writable());
        for value in [0, 1] {
            let error = set_control("/dev/not-a-camera", control, value).unwrap_err();
            assert!(error.to_string().contains("atualize"));
            assert!(
                !error.is::<V4l2CommandError>(),
                "cached rejection must not invoke v4l2-ctl"
            );
        }
    }
    #[test]
    fn negotiates_mjpeg_instead_of_unsupported_legacy_uyvy() {
        let modes = parse_modes(
            "[0]: 'YUYV' (YUYV)\nSize: Discrete 640x480\nInterval: Discrete 0.033s (30.000 fps)\n[1]: 'MJPG'\nSize: Discrete 1280x720\nInterval: Discrete 0.017s (60.000 fps)\nInterval: Discrete 0.033s (30.000 fps)",
        );
        let mut config = Config::default();
        config.ffmpeg_params.input_format = "uyvy422".into();
        let selected = choose_mode(&modes, &config).unwrap();
        assert_eq!(selected.format, "mjpeg");
        assert_eq!(selected.width, 1280);
        assert_eq!(selected.fps, 30.0);
    }
    #[test]
    fn keeps_inactive_flags_and_sparse_menu_options() {
        let c = parse_controls(
            "auto_exposure 0x009a0901 (menu) : min=0 max=3 default=3 value=3\n 1: Manual Mode\n 3: Aperture Priority Mode\nexposure_time_absolute 0x009a0902 (int) : min=1 max=2500 step=1 default=156 value=156 flags=inactive, 0x00001000",
        );
        assert_eq!(c[0].menu.len(), 2);
        assert!(!c[0].menu.contains_key(&2));
        assert!(c[1].inactive);
    }
}
