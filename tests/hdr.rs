//! HDR writes are opt-in. Common checks only prepare APT or refuse a missing camera.
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
const BIN: &str = env!("CARGO_BIN_EXE_elgato-virtualcam");
struct Controller {
    child: Child,
    root: tempfile::TempDir,
    restore: Option<bool>,
}
impl Controller {
    fn new(config: Value) -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("config.json");
        std::fs::write(&path, config.to_string()).unwrap();
        let child = Command::new(BIN)
            .args(["--headless", "--config"])
            .arg(path)
            .env("XDG_RUNTIME_DIR", root.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let c = Self {
            child,
            root,
            restore: None,
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        while !c.socket().exists() {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(20));
        }
        c
    }
    fn socket(&self) -> PathBuf {
        self.root.path().join(format!(
            "elgato-virtualcam-{}/control.sock",
            nix::unistd::Uid::effective()
        ))
    }
    fn try_request(&self, data: Value) -> std::io::Result<Value> {
        let mut socket = UnixStream::connect(self.socket())?;
        socket.set_read_timeout(Some(Duration::from_secs(10)))?;
        writeln!(socket, "{data}")?;
        let mut line = String::new();
        BufReader::new(socket).read_line(&mut line)?;
        serde_json::from_str(&line).map_err(std::io::Error::other)
    }
    fn request(&self, data: Value) -> Value {
        self.try_request(data).unwrap()
    }
    fn cli(&self, args: &[&str]) -> Value {
        let output = Command::new(BIN)
            .args(args)
            .env("XDG_RUNTIME_DIR", self.root.path())
            .output()
            .unwrap();
        serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stderr)))
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        // Restore through the same controller even if an assertion panics.
        if let Some(enabled) = self.restore {
            let _ = self.try_request(json!({"command":"preview"}));
            match self.try_request(json!({"command":"set_hdr","enabled":enabled})) {
                Ok(reply) if reply["ok"] == true => {}
                other => eprintln!("HDR restore was not confirmed: {other:?}"),
            }
            let _ = self.try_request(json!({"command":"stop"}));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
#[test]
fn hdr_without_camera_is_refused_and_apt_is_only_a_reviewed_plan() {
    let c = Controller::new(json!({"camera_device":"/dev/video999","effects":{"brightness":-0.2}}));
    let path = c.root.path().join("config.json");
    let before = std::fs::read(&path).unwrap();
    for value in ["false", "true"] {
        assert_eq!(c.cli(&["--set-hdr", value])["ok"], false);
    }
    let reply = c.cli(&["--prepare-install", "hdr-tools"]);
    assert_eq!(reply["ok"], true);
    let plan = &reply["state"]["installation_plan"];
    assert_eq!(plan["dependency"], "hdr_tools");
    assert_eq!(plan["repository"], Value::Null);
    assert!(plan["commands"].as_array().unwrap().iter().any(|command| {
        command
            .as_array()
            .unwrap()
            .iter()
            .any(|arg| arg == "uvcdynctrl")
    }));
    assert_eq!(reply["state"]["installation_running"], false);
    assert_eq!(c.cli(&["--cancel-install"])["ok"], true);
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
#[ignore = "changes real Facecam MK.2 HDR and restores it; close other capture apps first; requires uvcdynctrl or FACECAM_UVC_TOOL"]
fn hardware_hdr_toggles_reads_back_restores_saved_value_and_preserves_other_controls() {
    let device = std::env::var("FACECAM_HDR_DEVICE").unwrap_or_else(|_| "/dev/video0".into());
    let rate = std::env::var("FACECAM_HDR_RATE")
        .unwrap_or_else(|_| "30".into())
        .parse::<u32>()
        .unwrap();
    let mut c = Controller::new(
        json!({"camera_device":device,"ffmpeg_params":{"video_size":"1920x1080","framerate":rate},"effects":{"background_blur":false},"saved_controls":{}}),
    );
    let before = c.request(json!({"command":"status"}));
    assert_eq!(before["state"]["hdr"]["available"], true, "{before}");
    let original = before["state"]["hdr"]["enabled"].as_bool().unwrap();
    c.restore = Some(original);
    let controls = before["state"]["controls"].clone();
    assert_eq!(c.request(json!({"command":"preview"}))["ok"], true);
    let deadline = Instant::now() + Duration::from_secs(6);
    loop {
        let preview = c.request(json!({"command":"status"}));
        if preview["state"]["preview_frames"].as_u64().unwrap() > 0 {
            break;
        }
        assert!(Instant::now() < deadline, "{preview}");
        thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(c.request(json!({"command":"stop"}))["ok"], true);
    assert_eq!(
        c.cli(&["--set-hdr", if original { "false" } else { "true" }])["ok"],
        false
    );
    let mut config = before["state"]["config"].clone();
    config["saved_hdr"] = original.into();
    assert_eq!(
        c.request(json!({"command":"configure","config":config,"save":true}))["ok"],
        true
    );
    assert_eq!(c.request(json!({"command":"preview"}))["ok"], true);
    thread::sleep(Duration::from_millis(250));
    let toggled = c.cli(&["--set-hdr", if original { "false" } else { "true" }]);
    assert_eq!(toggled["ok"], true, "{toggled}");
    assert_eq!(toggled["state"]["hdr"]["enabled"], !original);
    let restored = c.request(json!({"command":"restore"}));
    assert_eq!(restored["ok"], true, "{restored}");
    assert_eq!(restored["state"]["hdr"]["enabled"], original);
    let fresh = c.request(json!({"command":"refresh"}));
    for control in controls.as_array().unwrap() {
        let name = &control["name"];
        // Some exposure values can change autonomously; the test makes no writes to them.
        if [
            "brightness",
            "contrast",
            "saturation",
            "sharpness",
            "backlight_compensation",
            "zoom_absolute",
            "pan_absolute",
            "tilt_absolute",
        ]
        .iter()
        .any(|n| name == n)
        {
            let after = fresh["state"]["controls"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == *name)
                .unwrap();
            assert_eq!(after["value"], control["value"], "{name}");
        }
    }
    assert_eq!(
        fresh["state"]["config"]["effects"],
        before["state"]["config"]["effects"]
    );
    let frames = fresh["state"]["preview_frames"].as_u64().unwrap();
    thread::sleep(Duration::from_millis(200));
    assert!(
        c.request(json!({"command":"status"}))["state"]["preview_frames"]
            .as_u64()
            .unwrap()
            > frames
    );
    let saved = c.request(json!({"command":"save"}));
    assert_eq!(saved["ok"], true);
    let disk: Value =
        serde_json::from_slice(&std::fs::read(c.root.path().join("config.json")).unwrap()).unwrap();
    assert_eq!(disk["saved_hdr"], original);
}
