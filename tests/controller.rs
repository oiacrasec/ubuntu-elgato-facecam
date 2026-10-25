use serde_json::Value;
use std::{
    path::Path,
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

const BIN: &str = env!("CARGO_BIN_EXE_elgato-virtualcam");

#[test]
fn default_launch_discovers_ai_and_does_not_overwrite_configuration() {
    for registered in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let config_dir = dir.path().join("config");
        let ai = if registered {
            dir.path().join("custom install")
        } else {
            data.join("facecam-hub/ai")
        };
        let model = ai.join("selfie.onnx");
        let runtime = ai.join(if registered {
            "onnxruntime-linux-x64-1.22.0/lib/libonnxruntime.so"
        } else {
            "onnxruntime-linux-x64-gpu-1.22.0/lib/libonnxruntime.so"
        });
        std::fs::create_dir_all(runtime.parent().unwrap()).unwrap();
        std::fs::write(&model, []).unwrap();
        std::fs::write(&runtime, []).unwrap();
        if registered {
            std::fs::create_dir_all(data.join("facecam-hub")).unwrap();
            std::fs::write(
                data.join("facecam-hub/ai-installations.json"),
                serde_json::json!({"cpu":{"ai_model":model,"ai_runtime":runtime}}).to_string(),
            )
            .unwrap();
        }
        let mut process = Process(
            Command::new(BIN)
                .arg("--headless")
                .env("XDG_RUNTIME_DIR", dir.path())
                .env("XDG_DATA_HOME", &data)
                .env("XDG_CONFIG_HOME", &config_dir)
                .env_remove("FACECAM_AI_MODEL")
                .env_remove("FACECAM_AI_RUNTIME")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let out = cli(dir.path(), &["--status"]);
            if out.status.success() {
                let reply: Value = serde_json::from_slice(&out.stdout).unwrap();
                assert_eq!(
                    reply["state"]["config"]["effects"]["ai_model"],
                    model.to_str().unwrap()
                );
                assert_eq!(
                    reply["state"]["config"]["effects"]["ai_runtime"],
                    runtime.to_str().unwrap()
                );
                assert_eq!(
                    reply["state"]["config"]["effects"]["background_blur"],
                    false
                );
                break;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(25));
        }
        assert!(!config_dir.join("elgato-virtualcam/config.json").exists());
        assert!(cli(dir.path(), &["--quit"]).status.success());
        assert!(process.0.wait().unwrap().success());
    }
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn cli(dir: &Path, args: &[&str]) -> Output {
    Command::new(BIN)
        .env("XDG_RUNTIME_DIR", dir)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn controller_is_single_instance_and_handles_stop_and_shutdown_without_a_display() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.json");
    let mut process = Process(
        Command::new(BIN)
            .args(["--headless", "--config"])
            .arg(&config)
            .env("XDG_RUNTIME_DIR", dir.path())
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let status = cli(dir.path(), &["--status"]);
        if status.status.success() {
            let reply: Value = serde_json::from_slice(&status.stdout).unwrap();
            assert_eq!(reply["state"]["streaming"], false);
            break;
        }
        assert!(Instant::now() < deadline, "controller never became ready");
        thread::sleep(Duration::from_millis(50));
    }
    let duplicate = cli(dir.path(), &["--headless"]);
    assert!(!duplicate.status.success());
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("instância"));
    assert!(cli(dir.path(), &["--stop"]).status.success());
    assert!(cli(dir.path(), &["--quit"]).status.success());
    assert!(process.0.wait().unwrap().success());
    assert!(!cli(dir.path(), &["--status"]).status.success());
    assert!(
        !config.exists(),
        "read-only controller startup must not overwrite config"
    );
}

#[test]
fn malformed_config_is_preserved_and_does_not_leave_a_socket() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.json");
    std::fs::write(&config, "{broken").unwrap();
    let out = Command::new(BIN)
        .args(["--headless", "--config"])
        .arg(&config)
        .env("XDG_RUNTIME_DIR", dir.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(std::fs::read_to_string(config).unwrap(), "{broken");
    assert!(!cli(dir.path(), &["--status"]).status.success());
}

#[test]
fn effects_cli_accepts_signed_values_and_preserves_disk_and_state_on_rejection() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.json");
    let file = dir.path().join("effects.json");
    let mut process = Process(
        Command::new(BIN)
            .args(["--headless", "--config"])
            .arg(&config)
            .env("XDG_RUNTIME_DIR", dir.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while !cli(dir.path(), &["--status"]).status.success() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(25));
    }
    for value in [-0.25, 0.0, 0.25] {
        std::fs::write(&file, serde_json::json!({"brightness":value}).to_string()).unwrap();
        let out = cli(dir.path(), &["--set-effects", file.to_str().unwrap()]);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let reply: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(reply["state"]["config"]["effects"]["brightness"], value);
        let saved: Value = serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
        assert_eq!(saved["effects"]["brightness"], value);
    }
    let before = std::fs::read(&config).unwrap();
    std::fs::write(&file, r#"{"brightness":2.0}"#).unwrap();
    assert!(
        !cli(dir.path(), &["--set-effects", file.to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(std::fs::read(&config).unwrap(), before);
    let status: Value = serde_json::from_slice(&cli(dir.path(), &["--status"]).stdout).unwrap();
    assert_eq!(status["state"]["config"]["effects"]["brightness"], 0.25);
    assert!(cli(dir.path(), &["--quit"]).status.success());
    assert!(process.0.wait().unwrap().success());
}

#[test]
fn failed_background_start_reports_the_error_and_closes_its_controller() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.json");
    std::fs::write(&config, r#"{"virtual_device":"/dev/video999999"}"#).unwrap();
    let out = Command::new(BIN)
        .args(["--start", "--config"])
        .arg(&config)
        .env("XDG_RUNTIME_DIR", dir.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("v4l2loopback"));
    let reply: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(reply["ok"], false);
    let deadline = Instant::now() + Duration::from_secs(3);
    while cli(dir.path(), &["--status"]).status.success() {
        assert!(
            Instant::now() < deadline,
            "failed startup left a controller running"
        );
        thread::sleep(Duration::from_millis(50));
    }
}
