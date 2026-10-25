//! Opt-in camera acceptance. Never loaded by the synthetic `ffmpeg_` check filter.
use serde_json::{Value, json};
use std::{
    io::Read,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const BIN: &str = env!("CARGO_BIN_EXE_elgato-virtualcam");

struct OwnedProcess(Child);
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(self.0.id() as i32),
            nix::sys::signal::Signal::SIGTERM,
        );
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if self.0.try_wait().ok().flatten().is_some() {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn cli(runtime: &Path, arguments: &[&str]) -> Value {
    let output = Command::new(BIN)
        .env("XDG_RUNTIME_DIR", runtime)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reply: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(reply["ok"], true, "{reply}");
    reply
}

fn consume(device: &str) -> (usize, u8) {
    let mut process = OwnedProcess(
        Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "v4l2",
                "-i",
                device,
                "-an",
                "-vf",
                "scale=32:18:flags=neighbor,format=yuv420p",
                "-frames:v",
                "30",
                "-f",
                "rawvideo",
                "pipe:1",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let output = process.0.stdout.take().unwrap();
    let errors = process.0.stderr.take().unwrap();
    let reader = thread::spawn(move || {
        let mut pixels = Vec::new();
        output.take(864 * 30 + 1).read_to_end(&mut pixels).unwrap();
        pixels
    });
    let error_reader = thread::spawn(move || {
        let mut text = String::new();
        errors.take(65536).read_to_string(&mut text).unwrap();
        text
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(status) = process.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "Consumer timed out; owned child will be terminated"
        );
        thread::sleep(Duration::from_millis(50));
    };
    let errors = error_reader.join().unwrap();
    assert!(status.success(), "{errors}");
    let pixels = reader.join().unwrap();
    assert_eq!(pixels.len(), 864 * 30);
    // Sample without interpolation; bicubic downscaling introduces chroma ringing
    // even for a synthetic neutral frame in FFmpeg. Check chroma before RGB rounding.
    // YUV420P at 32x18 has 576 luma bytes followed by 288 chroma bytes.
    let difference = pixels
        .as_chunks::<864>()
        .0
        .iter()
        .flat_map(|frame| &frame[576..])
        .map(|chroma| chroma.abs_diff(128))
        .max()
        .unwrap();
    (pixels.len() / 864, difference)
}

#[test]
#[ignore = "opens a physical camera and an existing loopback output; close other producers first; no control writes, module setup or recordings"]
fn hardware_virtual_camera_delivers_effects_and_restarts() {
    let device =
        std::env::var("FACECAM_TEST_VIRTUAL_DEVICE").unwrap_or_else(|_| "/dev/video10".into());
    assert!(
        Path::new(&device).exists(),
        "Prepare the virtual camera in the app first"
    );
    let info = Command::new("v4l2-ctl")
        .args(["-d", &device, "--info"])
        .output()
        .unwrap();
    assert!(
        info.status.success()
            && String::from_utf8_lossy(&info.stdout)
                .to_ascii_lowercase()
                .contains("v4l2 loopback"),
        "The selected consumer device must be a v4l2loopback output"
    );
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.json");
    std::fs::write(
        &config,
        serde_json::to_vec(&json!({
            "virtual_device":device, "effects":{"saturation":0.0,"background_blur":false}
        }))
        .unwrap(),
    )
    .unwrap();
    let mut controller = OwnedProcess(
        Command::new(BIN)
            .args(["--headless", "--config"])
            .arg(&config)
            .env("XDG_RUNTIME_DIR", directory.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let status = Command::new(BIN)
            .arg("--status")
            .env("XDG_RUNTIME_DIR", directory.path())
            .output()
            .unwrap();
        if status.status.success() {
            break;
        }
        assert!(Instant::now() < deadline, "Controller did not start");
        thread::sleep(Duration::from_millis(50));
    }
    for round in 1..=2 {
        cli(directory.path(), &["--start"]);
        thread::sleep(Duration::from_millis(1000));
        let status = cli(directory.path(), &["--status"]);
        assert!(status["state"]["error"].is_null(), "{status}");
        assert_eq!(status["state"]["streaming"], true);
        assert_eq!(status["state"]["preview"], false);
        let (frames, difference) = consume(&device);
        assert!(
            difference <= 1,
            "Saturation zero did not reach the consumer: {difference}"
        );
        println!(
            "round={round}, frames={frames}, max_chroma_distance_from_128={difference}; no frames saved"
        );
        let stopped = cli(directory.path(), &["--stop"]);
        assert_eq!(stopped["state"]["streaming"], false);
    }
    cli(directory.path(), &["--quit"]);
    assert!(controller.0.wait().unwrap().success());
}
