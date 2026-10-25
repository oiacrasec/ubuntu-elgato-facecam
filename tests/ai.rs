//! Opt-in native inference test; native runtimes/models are deliberately not CI dependencies.
#[cfg(feature = "ai")]
mod inference {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    const BIN: &str = env!("CARGO_BIN_EXE_elgato-virtualcam");

    #[test]
    #[ignore = "requires Facecam, FFmpeg, FACECAM_AI_RUNTIME and FACECAM_AI_MODEL"]
    fn hardware_ai_preview_stops_all_owned_workers() {
        use std::{
            io::{BufRead, BufReader},
            os::unix::net::UnixStream,
            time::{Duration, Instant},
        };
        let dir = tempfile::tempdir().unwrap();
        let runtime = std::env::var("FACECAM_AI_RUNTIME").expect("set FACECAM_AI_RUNTIME");
        let model = std::env::var("FACECAM_AI_MODEL").expect("set FACECAM_AI_MODEL");
        let provider = std::env::var("FACECAM_AI_PROVIDER").unwrap_or_else(|_| "cpu".into());
        let path = dir.path().join("config.json");
        std::fs::write(&path, serde_json::json!({"effects":{"background_blur":true,"ai_provider":provider,"ai_model":model,"ai_runtime":runtime}}).to_string()).unwrap();
        struct Controller(std::process::Child);
        impl Drop for Controller {
            fn drop(&mut self) {
                let _ = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(self.0.id() as i32),
                    nix::sys::signal::Signal::SIGTERM,
                );
                let _ = self.0.wait();
            }
        }
        let mut child = Controller(
            Command::new(BIN)
                .args(["--headless", "--config"])
                .arg(&path)
                .env("XDG_RUNTIME_DIR", dir.path())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let socket = dir.path().join(format!(
            "elgato-virtualcam-{}/control.sock",
            nix::unistd::Uid::effective()
        ));
        let deadline = Instant::now() + Duration::from_secs(15);
        while !socket.exists() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(25));
        }
        let request = |json: &str| -> serde_json::Value {
            let mut stream = UnixStream::connect(&socket).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(15)))
                .unwrap();
            writeln!(stream, "{json}").unwrap();
            let mut line = String::new();
            BufReader::new(stream).read_line(&mut line).unwrap();
            serde_json::from_str(&line).unwrap()
        };
        let reply = request(r#"{"command":"preview"}"#);
        assert_eq!(reply["ok"], true, "{reply}");
        loop {
            let state = request(r#"{"command":"status"}"#);
            assert!(state["state"]["error"].is_null(), "{state}");
            if state["state"]["preview_frames"].as_u64().unwrap() >= 3 {
                break;
            }
            assert!(Instant::now() < deadline, "no AI preview frames: {state}");
            std::thread::sleep(Duration::from_millis(100));
        }
        fn children(pid: u32) -> std::collections::BTreeSet<u32> {
            // The controller's worker thread owns these children; inspect every task.
            let mut children = std::collections::BTreeSet::new();
            for task in std::fs::read_dir(format!("/proc/{pid}/task"))
                .unwrap()
                .flatten()
            {
                if let Ok(text) = std::fs::read_to_string(task.path().join("children")) {
                    children.extend(text.split_whitespace().map(|id| id.parse::<u32>().unwrap()));
                }
            }
            children
        }
        let workers = children(child.0.id());
        assert_eq!(
            workers.len(),
            3,
            "decoder, AI worker and encoder must be running"
        );
        assert_eq!(request(r#"{"command":"stop"}"#)["ok"], true);
        // Idle metadata refresh can briefly spawn v4l2-ctl; check the owned video
        // workers specifically rather than unrelated, short-lived queries.
        assert!(
            children(child.0.id()).is_disjoint(&workers),
            "workers still alive"
        );
        for pid in workers {
            assert!(
                !std::path::Path::new(&format!("/proc/{pid}")).exists(),
                "worker not reaped: {pid}"
            );
        }
        assert_eq!(request(r#"{"command":"quit"}"#)["ok"], true);
        assert!(child.0.wait().unwrap().success());
    }

    #[test]
    #[ignore = "requires FACECAM_AI_RUNTIME, FACECAM_AI_MODEL; set FACECAM_AI_PROVIDER=cpu or cuda"]
    fn ai_worker_renders_complete_frames_and_rejects_truncation() {
        let runtime = std::env::var("FACECAM_AI_RUNTIME").expect("set FACECAM_AI_RUNTIME");
        let model = std::env::var("FACECAM_AI_MODEL").expect("set FACECAM_AI_MODEL");
        let provider = std::env::var("FACECAM_AI_PROVIDER").unwrap_or_else(|_| "cpu".into());
        let effects = serde_json::json!({"background_blur":true,"ai_provider":provider,"ai_model":model,"ai_runtime":runtime});
        let mut frame = vec![0u8; 64 * 64 * 3];
        for (i, pixel) in frame.as_chunks_mut::<3>().0.iter_mut().enumerate() {
            pixel.fill(if (i % 64 / 4 + i / 64 / 4) % 2 == 0 {
                20
            } else {
                230
            });
        }
        for truncate in [false, true] {
            let mut child = Command::new(BIN)
                .args(["--ai-worker", &effects.to_string(), "--ai-size", "64x64"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let mut input = child.stdin.take().unwrap();
            if truncate {
                input.write_all(&frame[..frame.len() - 1]).unwrap();
            } else {
                input.write_all(&frame).unwrap();
            }
            drop(input);
            let result = child.wait_with_output().unwrap();
            if truncate {
                assert!(!result.status.success());
                assert!(String::from_utf8_lossy(&result.stderr).contains("Frame RGB incompleto"));
            } else {
                assert!(
                    result.status.success(),
                    "{}",
                    String::from_utf8_lossy(&result.stderr)
                );
                assert_eq!(result.stdout.len(), frame.len());
                assert_ne!(result.stdout, frame, "background processing was bypassed");
            }
        }
    }
}
