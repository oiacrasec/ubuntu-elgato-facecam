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
fn library_import_is_async_persistent_and_does_not_apply_or_save_effects() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.json");
    let source = dir.path().join("User look.cube");
    std::fs::write(
        &source,
        format!("LUT_3D_SIZE 2\n{}", "-0.1 0 1.1\n".repeat(8)),
    )
    .unwrap();
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
    assert!(!config.exists());
    let imported = cli(
        dir.path(),
        &[
            "--import-resource",
            "lut",
            "--import-file",
            source.to_str().unwrap(),
        ],
    );
    assert!(
        imported.status.success(),
        "{}",
        String::from_utf8_lossy(&imported.stderr)
    );
    let wait = || -> Value {
        loop {
            let state: Value =
                serde_json::from_slice(&cli(dir.path(), &["--status"]).stdout).unwrap();
            if state["state"]["importing"] == false {
                return state["state"].clone();
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(25));
        }
    };
    let state = wait();
    assert_eq!(state["import_revision"], 1);
    assert_eq!(state["config"]["effects"]["lut"], "");
    assert!(!config.exists());
    let resource = &state["imported"];
    let copy = Path::new(resource["path"].as_str().unwrap());
    assert_ne!(copy, source);
    assert_eq!(
        std::fs::read(copy).unwrap(),
        std::fs::read(&source).unwrap()
    );
    std::fs::remove_file(&source).unwrap();
    assert!(copy.is_file());
    std::fs::write(&source, "LUT_3D_SIZE 0\n").unwrap();
    assert!(
        cli(
            dir.path(),
            &[
                "--import-resource",
                "lut",
                "--import-file",
                source.to_str().unwrap()
            ]
        )
        .status
        .success()
    );
    let rejected = wait();
    assert!(rejected["error"].as_str().unwrap().contains("entre 2 e 65"));
    assert_eq!(rejected["import_revision"], 1);
    assert!(!config.exists());
    assert_eq!(
        std::fs::read_dir(copy.parent().unwrap()).unwrap().count(),
        1
    );
    assert!(cli(dir.path(), &["--quit"]).status.success());
    assert!(process.0.wait().unwrap().success());
    let mut restarted = Process(
        Command::new(BIN)
            .args(["--headless", "--config"])
            .arg(&config)
            .env("XDG_RUNTIME_DIR", dir.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    loop {
        let out = cli(dir.path(), &["--status"]);
        if out.status.success() {
            let reply: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert!(
                reply["state"]["resources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["path"] == resource["path"])
            );
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(25));
    }
    assert!(cli(dir.path(), &["--quit"]).status.success());
    assert!(restarted.0.wait().unwrap().success());
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
fn dependency_status_follows_the_package_database_without_saving_configuration() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.json");
    let database = dir.path().join("dpkg");
    std::fs::create_dir(&database).unwrap();
    let contents = "Package: ffmpeg\nStatus: install ok installed\nArchitecture: amd64\nVersion: 6.1\nDescription: test fixture\n\nPackage: v4l-utils\nStatus: install ok installed\nArchitecture: amd64\nVersion: 1.26\nDescription: test fixture\n\n";
    std::fs::write(database.join("status"), contents).unwrap();
    let mut process = Process(
        Command::new(BIN)
            .args(["--headless", "--config"])
            .arg(&config)
            .env("XDG_RUNTIME_DIR", dir.path())
            .env("DPKG_ADMINDIR", &database)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let out = cli(dir.path(), &["--status"]);
        if out.status.success() {
            let reply: Value = serde_json::from_slice(&out.stdout).unwrap();
            let packages = &reply["state"]["dependencies"][0]["packages"];
            assert_eq!(packages[0]["name"], "ffmpeg");
            assert_eq!(packages[0]["version"], "6.1");
            assert_eq!(packages[1]["version"], "1.26");
            assert!(reply["state"]["dependency_error"].is_null());
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(25));
    }
    std::fs::write(database.join("next-status"), "").unwrap();
    std::fs::rename(database.join("next-status"), database.join("status")).unwrap();
    loop {
        let reply: Value = serde_json::from_slice(&cli(dir.path(), &["--status"]).stdout).unwrap();
        if reply["state"]["dependencies"][0]["packages"][0]["version"].is_null() {
            assert!(reply["state"]["dependencies"][0]["packages"][1]["version"].is_null());
            break;
        }
        assert!(
            Instant::now() < deadline,
            "package state was not refreshed automatically"
        );
        thread::sleep(Duration::from_millis(100));
    }
    assert!(!config.exists());
    assert!(cli(dir.path(), &["--quit"]).status.success());
    assert!(process.0.wait().unwrap().success());
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
fn apt_plans_require_explicit_confirmation_and_cancel_invalidates_the_identifier() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.json");
    let original = b"{\"custom\":{\"keep\":true}}";
    std::fs::write(&config, original).unwrap();
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
    assert!(
        !cli(dir.path(), &["--confirm-install", "1"])
            .status
            .success()
    );
    let mut previous = None;
    for _ in 0..2 {
        let prepared = cli(dir.path(), &["--prepare-install", "video-tools"]);
        assert!(
            prepared.status.success(),
            "{}",
            String::from_utf8_lossy(&prepared.stderr)
        );
        let reply: Value = serde_json::from_slice(&prepared.stdout).unwrap();
        let plan = &reply["state"]["installation_plan"];
        let id = plan["id"].as_u64().unwrap();
        assert_ne!(previous, Some(id));
        if let Some(previous) = previous {
            assert!(
                !cli(dir.path(), &["--confirm-install", &previous.to_string()])
                    .status
                    .success()
            );
        }
        assert_eq!(reply["state"]["installation_running"], false);
        assert!(!Path::new(plan["log"].as_str().unwrap()).exists());
        assert!(
            plan["commands"]
                .as_array()
                .unwrap()
                .iter()
                .all(|command| command[2] == "/usr/bin/apt-get")
        );
        previous = Some(id);
    }
    assert!(cli(dir.path(), &["--cancel-install"]).status.success());
    assert!(
        !cli(
            dir.path(),
            &["--confirm-install", &previous.unwrap().to_string()]
        )
        .status
        .success()
    );
    let status: Value = serde_json::from_slice(&cli(dir.path(), &["--status"]).stdout).unwrap();
    assert!(status["state"]["installation_plan"].is_null());
    assert_eq!(status["state"]["installation_running"], false);
    assert_eq!(std::fs::read(config).unwrap(), original);
    let sdk = cli(dir.path(), &["--prepare-install", "sdk-manager"]);
    assert!(
        sdk.status.success(),
        "{}",
        String::from_utf8_lossy(&sdk.stderr)
    );
    let sdk: Value = serde_json::from_slice(&sdk.stdout).unwrap();
    let commands = sdk["state"]["installation_plan"]["commands"]
        .as_array()
        .unwrap();
    assert!(
        commands
            .iter()
            .flat_map(|c| c.as_array().unwrap())
            .all(|a| !a.as_str().unwrap().ends_with(".deb"))
    );
    assert_eq!(
        commands.last().unwrap().as_array().unwrap().last().unwrap(),
        "sdkmanager"
    );
    assert_eq!(sdk["state"]["installation_running"], false);
    assert!(cli(dir.path(), &["--cancel-install"]).status.success());
    assert!(cli(dir.path(), &["--quit"]).status.success());
    assert!(process.0.wait().unwrap().success());
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
        std::fs::write(&file, serde_json::json!({"brightness":value,"background_mode":"studio","background_color":[10,20,30],"ai_fps":24}).to_string()).unwrap();
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
        assert_eq!(saved["effects"]["background_mode"], "studio");
        assert_eq!(
            saved["effects"]["background_color"],
            serde_json::json!([10, 20, 30])
        );
        assert_eq!(saved["effects"]["ai_fps"], 24);
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

#[cfg(feature = "ai")]
#[test]
#[ignore = "requires ffmpeg; decodes local synthetic media, no camera or model"]
fn ffmpeg_video_background_loops_at_processing_rate_and_rejects_invalid_media() {
    use std::{io::Read, sync::mpsc};

    let dir = tempfile::tempdir().unwrap();
    let video = dir.path().join("video ' ; [loop].mp4");
    let generated = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=32x24:rate=2:duration=1",
            "-threads",
            "1",
            "-c:v",
            "mpeg4",
            "-y",
        ])
        .arg(&video)
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let request = serde_json::json!({
        "path":video, "width":32, "height":24, "fps":4.0, "parent_pid":std::process::id(),
    });
    let mut decoder = Process(
        Command::new(BIN)
            .args(["--decode-background", &request.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut output = decoder.0.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut frames = vec![vec![0; 32 * 24 * 3]; 12];
        for frame in &mut frames {
            output.read_exact(frame).unwrap();
        }
        let _ = tx.send(frames);
    });
    let frames = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("background decoder stalled");
    decoder.0.kill().unwrap();
    decoder.0.wait().unwrap();
    reader.join().unwrap();
    // Two 2fps source frames become four 4fps frames, repeating every second.
    assert_eq!(frames[0], frames[1]);
    assert_ne!(frames[0], frames[2]);
    assert_eq!(frames[2], frames[3]);
    for i in 4..12 {
        assert_eq!(frames[i], frames[i % 4], "loop frame {i}");
    }

    for invalid in [
        dir.path().join("broken.mp4"),
        dir.path().join("playlist.mp4"),
    ] {
        std::fs::write(&invalid, "file 'https://example.com/video.mp4'\n").unwrap();
        let mut request = request.clone();
        request["path"] = serde_json::json!(invalid);
        let out = Command::new(BIN)
            .args(["--decode-background", &request.to_string()])
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
    }
    let mut request = request;
    request["parent_pid"] = serde_json::json!(0);
    let out = Command::new(BIN)
        .args(["--decode-background", &request.to_string()])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("processo responsável"));
}
