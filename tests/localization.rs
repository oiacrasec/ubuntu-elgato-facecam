//! Locale changes use temporary files and a headless controller, never camera writes.
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
        serde_json::from_str(&text).unwrap()
    }
    fn change(&self, language: &str) -> Value {
        self.request(json!({"command":"update_settings","language":language}))
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn language_changes_are_live_persistent_and_refuse_invalid_catalogs_without_state_loss() {
    let controller = Controller::new();
    let initial = controller.request(json!({"command":"status"}));
    assert_eq!(initial["state"]["config"]["ui"]["language"], "pt-BR");
    let en = controller.change("en-US");
    assert_eq!(en["ok"], true);
    assert_eq!(en["state"]["config"]["ui"]["language"], "en-US");
    assert_eq!(
        en["state"]["config"]["effects"],
        initial["state"]["config"]["effects"]
    );
    assert_eq!(en["state"]["config"]["custom"]["preserve"], 7);
    let output = Command::new(BIN)
        .arg("--status")
        .env("XDG_RUNTIME_DIR", controller.root.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let reply: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(reply["message"], "Settings updated");
    let saved = std::fs::read(&controller.config).unwrap();
    for invalid in ["../en-US", "", "xx-Unknown"] {
        assert_eq!(controller.change(invalid)["ok"], false);
        assert_eq!(std::fs::read(&controller.config).unwrap(), saved);
    }
    let locales = controller.root.path().join("locales");
    std::fs::create_dir(&locales).unwrap();
    std::fs::write(locales.join("es-ES.json"), json!({"language":"es-ES","name":"Español","messages":{"Configurações atualizadas":"Preferencias actualizadas"}}).to_string()).unwrap();
    std::fs::write(
        locales.join("es-BAD.json"),
        json!({"language":"es-BAD","name":"Broken","messages":{"Arquivo {path}":"File {error}"}})
            .to_string(),
    )
    .unwrap();
    let refreshed = controller.request(json!({"command":"refresh"}));
    assert!(
        refreshed["state"]["languages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["language"] == "es-ES")
    );
    assert!(
        !refreshed["state"]["languages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["language"] == "es-BAD")
    );
    assert_eq!(controller.change("es-BAD")["ok"], false);
    assert_eq!(std::fs::read(&controller.config).unwrap(), saved);
    assert_eq!(controller.change("es-ES")["ok"], true);
    let output = Command::new(BIN)
        .args(["--config"])
        .arg(&controller.config)
        .arg("--status")
        .env("XDG_RUNTIME_DIR", controller.root.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let reply: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(reply["message"], "Preferencias actualizadas");
    let backup = controller.root.path().join("saved.json");
    std::fs::rename(&controller.config, &backup).unwrap();
    std::fs::create_dir(&controller.config).unwrap();
    let failed = controller.change("pt-BR");
    assert_eq!(failed["ok"], false);
    assert_eq!(failed["state"]["config"]["ui"]["language"], "es-ES");
    std::fs::remove_dir(&controller.config).unwrap();
    std::fs::rename(backup, &controller.config).unwrap();
    assert_eq!(controller.change("pt-BR")["ok"], true);
    assert_eq!(controller.request(json!({"command":"quit"}))["ok"], true);
}

#[test]
fn command_help_uses_text_catalogs_without_changing_flags() {
    let dir = tempfile::tempdir().unwrap();
    for (language, heading, help) in [
        ("pt-BR", "Uso:", "Mostrar ajuda"),
        ("en-US", "Usage:", "Print help"),
    ] {
        let output = Command::new(BIN)
            .arg("--help")
            .env("FACECAM_LANGUAGE", language)
            .env("XDG_CONFIG_HOME", dir.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains(heading), "{text}");
        assert!(text.contains(help), "{text}");
        assert!(text.contains("--prepare-nvidia"));
        assert!(text.contains("--remove-image"));
    }
}
