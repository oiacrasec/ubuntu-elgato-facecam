//! Public Prompter commands, using a private headless controller and synthetic text.
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
    config: PathBuf,
}
impl Controller {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config.json");
        std::fs::write(
            &config,
            json!({"effects":{"brightness":-0.15,"mirror":true},"custom":{"preserve":7}})
                .to_string(),
        )
        .unwrap();
        let child = Command::new(BIN)
            .args(["--headless", "--config"])
            .arg(&config)
            .env("XDG_RUNTIME_DIR", root.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let controller = Self {
            child,
            root,
            config,
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        while !controller.socket().exists() {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(20));
        }
        controller
    }
    fn socket(&self) -> PathBuf {
        self.root.path().join(format!(
            "elgato-virtualcam-{}/control.sock",
            nix::unistd::Uid::effective()
        ))
    }
    fn request(&self, request: Value) -> Value {
        let mut socket = UnixStream::connect(self.socket()).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        writeln!(socket, "{request}").unwrap();
        let mut text = String::new();
        BufReader::new(socket).read_line(&mut text).unwrap();
        if text.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap()
        }
    }
    fn cli(&self, args: &[&str]) -> Value {
        let output = Command::new(BIN)
            .args(args)
            .env("XDG_RUNTIME_DIR", self.root.path())
            .output()
            .unwrap();
        serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&output.stderr)))
    }
    fn action(&self, action: &str) -> Value {
        self.cli(&["--prompter-action", action])
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn script_playback_cli_is_persistent_independent_of_camera_and_rejects_invalid_changes() {
    let c = Controller::new();
    let initial = c.request(json!({"command":"status"}));
    assert_eq!(c.action("play")["ok"], false);
    assert_eq!(c.action("show")["ok"], false);
    let settings = c.root.path().join("script.json");
    std::fs::write(&settings, json!({"text":"Olá câmera. Este é um roteiro sintético.","words_per_minute":120,"font_size":48,"mirror":true}).to_string()).unwrap();
    let saved = c.cli(&["--prompter-settings", settings.to_str().unwrap()]);
    assert_eq!(saved["ok"], true);
    assert_eq!(
        saved["state"]["config"]["effects"],
        initial["state"]["config"]["effects"]
    );
    assert_eq!(saved["state"]["config"]["custom"]["preserve"], 7);
    assert_eq!(c.action("play")["state"]["prompter"]["playing"], true);
    thread::sleep(Duration::from_millis(300));
    let paused = c.action("pause");
    let position = paused["state"]["prompter"]["position"].as_f64().unwrap();
    assert!(position > 0.0 && position < 1.0, "{paused}");
    thread::sleep(Duration::from_millis(150));
    assert_eq!(
        c.request(json!({"command":"status"}))["state"]["prompter"]["position"],
        position
    );
    for percent in ["-1", "101", "NaN", "inf"] {
        let output = Command::new(BIN)
            .arg(format!("--prompter-position={percent}"))
            .env("XDG_RUNTIME_DIR", c.root.path())
            .output()
            .unwrap();
        assert!(!output.status.success(), "{percent}");
    }
    for (percent, expected) in [("0", 0.0), ("50", 0.5), ("100", 1.0)] {
        let reply = c.cli(&["--prompter-position", percent]);
        assert_eq!(reply["ok"], true, "{reply}");
        assert_eq!(reply["state"]["prompter"]["position"], expected);
    }
    assert_eq!(c.action("play")["state"]["prompter"]["position"], 0.0);
    assert_eq!(c.action("rewind")["state"]["prompter"]["position"], 0.0);
    let before = std::fs::read(&c.config).unwrap();
    for settings in [
        json!({"text":"ok","words_per_minute":0}),
        json!({"text":"ok","font_size":-1}),
        json!({"text":"x".repeat(8193)}),
        json!({"text":"bad\u{0}text"}),
        json!({"text":"ok","monitor":32}),
    ] {
        assert_ne!(
            c.request(json!({"command":"update_prompter","settings":settings}))["ok"],
            true
        );
        assert_eq!(std::fs::read(&c.config).unwrap(), before);
    }
    assert_eq!(
        c.request(json!({"command":"report_prompter_displays","displays":["Fake"]}))["ok"],
        false
    );
    let backup = c.root.path().join("saved.json");
    std::fs::rename(&c.config, &backup).unwrap();
    std::fs::create_dir(&c.config).unwrap();
    let failed = c.request(json!({"command":"update_prompter","settings":{"text":"new"}}));
    assert_eq!(failed["ok"], false);
    assert_eq!(
        failed["state"]["config"]["prompter"],
        saved["state"]["config"]["prompter"]
    );
    std::fs::remove_dir(&c.config).unwrap();
    std::fs::rename(&backup, &c.config).unwrap();
    let final_state = c.request(json!({"command":"status"}));
    assert_eq!(final_state["state"]["streaming"], false);
    assert_eq!(final_state["state"]["preview"], false);
    assert_eq!(c.action("hide")["ok"], true);
}
