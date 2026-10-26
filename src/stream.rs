use crate::{camera::Mode, config::Config};
use anyhow::{Context, Result, bail};
use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub struct PreviewFrame {
    // Headless controllers can receive/count frames without a display consumer.
    #[cfg_attr(not(feature = "gui"), allow(dead_code))]
    pub size: [usize; 2],
    pub rgb: Vec<u8>,
}
pub type Wake = Arc<Mutex<Option<Box<dyn Fn() + Send>>>>;
#[derive(Default)]
pub struct PreviewFrames {
    pub frame: Mutex<Option<PreviewFrame>>,
    pub wake: Wake,
}
pub type LatestFrame = Arc<PreviewFrames>;

pub fn output_fps(mode: &Mode, effects: &crate::effects::Effects) -> f64 {
    if effects.ai_enabled() {
        mode.fps.min(f64::from(effects.ai_fps))
    } else {
        mode.fps
    }
}

pub struct Stream {
    child: Child,
    upstream: Children,
    reader: Option<JoinHandle<()>>,
    pub started: Instant,
    pub log: PathBuf,
    pub frames_received: Arc<AtomicU64>,
    pub ai_backend: Option<crate::ai::BackendStatus>,
}

#[derive(Default)]
struct Children(Vec<Child>);

impl Drop for Children {
    fn drop(&mut self) {
        for child in self.0.iter_mut().rev() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn ffmpeg_args(
    device: &str,
    mode: &Mode,
    config: &Config,
    virtual_output: bool,
    preview: bool,
) -> Vec<String> {
    let mut args: Vec<String> = [
        "-hide_banner",
        "-loglevel",
        "warning",
        "-nostdin",
        "-filter_threads",
        "1",
        "-filter_complex_threads",
        "1",
        "-threads",
        "2",
        "-thread_queue_size",
        "4",
        "-f",
        "v4l2",
        "-framerate",
        &mode.fps.to_string(),
        "-input_format",
        &mode.format,
        "-video_size",
        &format!("{}x{}", mode.width, mode.height),
        "-i",
        device,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let mut filters = config.effects.filters();
    if filters.is_empty() {
        filters.push("null".into());
    }
    // Share the actual output format too: previewing RGB before YUV420 conversion
    // would hide chroma loss and rounding that virtual-camera consumers receive.
    filters.push("format=yuv420p".into());
    let base = filters.join(",");
    let graph = match (virtual_output, preview) {
        (true, true) => format!("[0:v]{base},split=2[vout][p];[p]format=rgb24[preview]"),
        (true, false) => format!("[0:v]{base}[vout]"),
        _ => format!("[0:v]{base},format=rgb24[preview]"),
    };
    args.extend(["-filter_complex".into(), graph]);
    if virtual_output {
        args.extend(
            [
                "-map",
                "[vout]",
                "-an",
                "-pix_fmt",
                "yuv420p",
                "-f",
                "v4l2",
                &config.virtual_device,
            ]
            .into_iter()
            .map(str::to_owned),
        );
    }
    if preview {
        args.extend(
            [
                "-map",
                "[preview]",
                "-an",
                "-pix_fmt",
                "rgb24",
                "-threads",
                "1",
                "-f",
                "rawvideo",
                "pipe:1",
            ]
            .into_iter()
            .map(str::to_owned),
        );
    }
    args
}

pub fn verify_virtual_device(device: &str) -> Result<()> {
    let info = crate::camera::v4l2(device, &["--info"]);
    let is_loopback = info.as_ref().is_ok_and(|text| {
        text.lines().any(|line| {
            line.split_once(':').is_some_and(|(key, value)| {
                key.trim() == "Driver name" && value.trim() == "v4l2 loopback"
            })
        })
    });
    if !is_loopback {
        bail!(
            "Saída virtual v4l2loopback indisponível em {device}. Abra Configurações › Câmera virtual › Preparar câmera virtual, ou use --prepare-virtual-camera pela CLI"
        );
    }
    Ok(())
}

impl Stream {
    pub fn start(
        device: &str,
        mode: &Mode,
        config: &Config,
        virtual_output: bool,
        preview: bool,
        frames: LatestFrame,
        log: &Path,
    ) -> Result<Self> {
        config.effects.preflight()?;
        let size = [usize::try_from(mode.width)?, usize::try_from(mode.height)?];
        if size[0] == 0 || size[1] == 0 || size[0] > 7680 || size[1] > 4320 {
            bail!("Resolução de vídeo fora dos limites");
        }
        let frame_bytes = size[0]
            .checked_mul(size[1])
            .and_then(|n| n.checked_mul(3))
            .context("Tamanho de quadro inválido")?;
        // SetEffects validates inference before stopping an active capture.
        // Automatic backend selection probes RTX in an isolated child.
        if virtual_output {
            verify_virtual_device(&config.virtual_device)?;
        }
        if fs::metadata(log).is_ok_and(|m| m.len() > 2 * 1024 * 1024) {
            File::create(log)?;
        }
        let err = OpenOptions::new().create(true).append(true).open(log)?;
        let mut upstream = Children::default();
        let mut ai_backend = None;
        let mut args = ffmpeg_args(device, mode, config, virtual_output, preview);
        let stdin = if config.effects.ai_enabled() {
            let (ai_width, ai_height) = (mode.width, mode.height);
            let ai_fps = output_fps(mode, &config.effects);
            let end = args.iter().position(|a| a == "-i").unwrap() + 2;
            let mut capture = args[..end].to_vec();
            capture.extend(["-vf".into(), format!("fps={ai_fps}")]);
            capture.extend(
                [
                    "-an", "-pix_fmt", "rgb24", "-threads", "1", "-f", "rawvideo", "pipe:1",
                ]
                .map(str::to_owned),
            );
            let mut decoder = Command::new("ffmpeg")
                .args(capture)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(err.try_clone()?)
                .spawn()?;
            let source = decoder
                .stdout
                .take()
                .context("Pipe de captura indisponível")?;
            upstream.0.push(decoder);
            let (mut worker_command, status) = crate::ai::command(&config.effects)?;
            ai_backend = Some(status);
            let mut worker = worker_command
                .args(["--ai-size", &format!("{ai_width}x{ai_height}")])
                .args(["--ai-rate", &ai_fps.to_string()])
                .stdin(Stdio::from(source))
                .stdout(Stdio::piped())
                .stderr(err.try_clone()?)
                .spawn()?;
            let processed = worker.stdout.take().context("Pipe de IA indisponível")?;
            upstream.0.push(worker);
            let raw_mode = Mode {
                format: "rgb24".into(),
                width: ai_width,
                height: ai_height,
                fps: ai_fps,
            };
            args = ffmpeg_args("pipe:0", &raw_mode, config, virtual_output, preview);
            let format = args.iter().position(|a| a == "-f").unwrap();
            args[format + 1] = "rawvideo".into();
            let input_format = args.iter().position(|a| a == "-input_format").unwrap();
            args[input_format] = "-pixel_format".into();
            let input = args.iter().position(|a| a == "-i").unwrap();
            args.splice(
                input..input,
                [
                    "-probesize".into(),
                    "32".into(),
                    "-analyzeduration".into(),
                    "0".into(),
                ],
            );
            Stdio::from(processed)
        } else {
            Stdio::null()
        };
        let mut child = Command::new("ffmpeg")
            .args(args)
            .stdin(stdin)
            .stdout(if preview {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stderr(err)
            .spawn()
            .context("Não foi possível iniciar FFmpeg; instale o pacote ffmpeg")?;
        let frames_received = Arc::new(AtomicU64::new(0));
        let counter = frames_received.clone();
        let reader = child.stdout.take().map(|mut stdout| {
            thread::spawn(move || {
                loop {
                    let mut frame = PreviewFrame {
                        size,
                        rgb: vec![0; frame_bytes],
                    };
                    if stdout.read_exact(&mut frame.rgb).is_err() {
                        break;
                    }
                    *frames.frame.lock().unwrap_or_else(|e| e.into_inner()) = Some(frame);
                    counter.fetch_add(1, Ordering::Relaxed);
                    if let Some(wake) = &*frames.wake.lock().unwrap_or_else(|e| e.into_inner()) {
                        wake();
                    }
                }
            })
        });
        Ok(Self {
            child,
            upstream,
            reader,
            started: Instant::now(),
            log: log.into(),
            frames_received,
            ai_backend,
        })
    }

    pub fn poll(&mut self) -> Result<bool> {
        if let Some(status) = self.child.try_wait()? {
            let text = fs::read_to_string(&self.log).unwrap_or_default();
            let tail = text
                .lines()
                .rev()
                .take(8)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            bail!("FFmpeg encerrou ({status}). {tail}");
        }
        Ok(true)
    }

    pub fn stop(&mut self) -> Result<()> {
        if self.child.try_wait()?.is_none() {
            kill(Pid::from_raw(self.child.id() as i32), Signal::SIGTERM).ok();
            let deadline = Instant::now() + Duration::from_secs(2);
            while self.child.try_wait()?.is_none() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(20));
            }
            if self.child.try_wait()?.is_none() {
                self.child.kill()?;
            }
        }
        self.child.wait()?;
        drop(std::mem::take(&mut self.upstream));
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        Ok(())
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const PREVIEW_WIDTH: usize = 640;
    const PREVIEW_HEIGHT: usize = 360;
    #[test]
    fn output_rate_preserves_camera_fps_and_applies_only_the_explicit_ai_limit() {
        let mode = Mode {
            format: "mjpeg".into(),
            width: 1920,
            height: 1080,
            fps: 60.0,
        };
        let effects = crate::effects::Effects::default();
        assert_eq!(output_fps(&mode, &effects), 60.0);
        let effects = crate::effects::Effects {
            background_blur: true,
            ..effects
        };
        assert_eq!(output_fps(&mode, &effects), 30.0);
        let framing = crate::effects::Effects {
            eye_contact: true,
            background_blur: false,
            ..effects.clone()
        };
        assert_eq!(output_fps(&mode, &framing), 30.0);
        let effects = crate::effects::Effects {
            ai_fps: 60,
            ..effects
        };
        assert_eq!(output_fps(&mode, &effects), 60.0);
        let mode = Mode {
            fps: 30000.0 / 1001.0,
            ..mode
        };
        assert_eq!(output_fps(&mode, &effects), mode.fps);
    }
    #[test]
    fn headless_stream_does_not_decode_preview_frames() {
        let mode = Mode {
            format: "mjpeg".into(),
            width: 1280,
            height: 720,
            fps: 30.0,
        };
        let args = ffmpeg_args("/dev/video0", &mode, &Config::default(), true, false);
        assert!(!args.iter().any(|a| a == "rawvideo" || a == "pipe:1"));
        assert_eq!(args.last().unwrap(), "/dev/video10");
    }

    #[test]
    #[ignore = "requires ffmpeg; compares synthetic preview and virtual output, no camera"]
    fn ffmpeg_preview_matches_decoded_virtual_pixels_resolution_and_frame_count() {
        let dir = tempfile::tempdir().unwrap();
        for (width, height, fps) in [(160, 120, 60.0), (320, 180, 30000.0 / 1001.0)] {
            let mode = Mode {
                format: "rgb24".into(),
                width,
                height,
                fps,
            };
            let config = Config {
                effects: crate::effects::Effects {
                    mirror: true,
                    contrast: 1.2,
                    saturation: 0.7,
                    ..Default::default()
                },
                ..Default::default()
            };
            let virtual_file = dir.path().join(format!("{width}.yuv"));
            let render = |virtual_output| {
                let original = ffmpeg_args("unused", &mode, &config, virtual_output, true);
                let filter = original
                    .iter()
                    .position(|a| a == "-filter_complex")
                    .unwrap();
                let mut args: Vec<String> = [
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-filter_complex_threads",
                    "1",
                    "-f",
                    "lavfi",
                    "-i",
                ]
                .map(str::to_owned)
                .into();
                args.push(format!(
                    "testsrc2=size={width}x{height}:rate={fps}:duration=0.2"
                ));
                args.extend_from_slice(&original[filter..]);
                if virtual_output {
                    let sink = args
                        .iter()
                        .position(|a| a == &config.virtual_device)
                        .unwrap();
                    args[sink - 1] = "rawvideo".into();
                    args[sink] = virtual_file.to_str().unwrap().into();
                }
                let output = Command::new("ffmpeg").args(args).output().unwrap();
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                output.stdout
            };
            let preview = render(true);
            let plane = width as usize * height as usize;
            let virtual_bytes = fs::metadata(&virtual_file).unwrap().len() as usize;
            let count = preview.len() / (plane * 3);
            assert_eq!(count, if fps == 60.0 { 12 } else { 6 });
            assert_eq!(preview.len(), count * plane * 3);
            assert_eq!(virtual_bytes, count * plane * 3 / 2);
            let decoded = Command::new("ffmpeg")
                .args([
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-f",
                    "rawvideo",
                    "-pixel_format",
                    "yuv420p",
                    "-video_size",
                    &format!("{width}x{height}"),
                    "-framerate",
                    &fps.to_string(),
                    "-i",
                ])
                .arg(&virtual_file)
                .args([
                    "-pix_fmt", "rgb24", "-threads", "1", "-f", "rawvideo", "pipe:1",
                ])
                .output()
                .unwrap();
            assert!(
                decoded.status.success(),
                "{}",
                String::from_utf8_lossy(&decoded.stderr)
            );
            assert_eq!(
                preview, decoded.stdout,
                "preview must match the decoded virtual frames byte for byte"
            );
            assert_eq!(
                render(false),
                preview,
                "starting virtual output must not change the preview"
            );
        }
    }

    #[test]
    #[ignore = "requires ffmpeg; uses synthetic frames, no camera or display"]
    fn ffmpeg_effects_render() {
        fn render(config: &Config) -> Vec<u8> {
            let mode = Mode {
                format: "mjpeg".into(),
                width: 640,
                height: 360,
                fps: 30.0,
            };
            let original = ffmpeg_args("unused", &mode, config, false, true);
            let end = original
                .iter()
                .position(|a| a == "-filter_complex")
                .unwrap();
            let mut args: Vec<String> = [
                "-hide_banner",
                "-loglevel",
                "error",
                "-filter_complex_threads",
                "1",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=640x360:rate=30:duration=0.2",
            ]
            .map(str::to_owned)
            .into();
            args.extend_from_slice(&original[end..]);
            let output = Command::new("ffmpeg").args(args).output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.stdout.len() >= PREVIEW_WIDTH * PREVIEW_HEIGHT * 3);
            output.stdout[..PREVIEW_WIDTH * PREVIEW_HEIGHT * 3].to_vec()
        }
        let baseline = render(&Config::default());
        let mut config = Config::default();
        config.effects.mirror = true;
        let mirrored = render(&config);
        for y in [10, 100, 300] {
            for x in [10, 100, 500] {
                let original = (y * PREVIEW_WIDTH + x) * 3;
                let reversed = (y * PREVIEW_WIDTH + PREVIEW_WIDTH - 1 - x) * 3;
                assert_eq!(
                    &baseline[original..original + 3],
                    &mirrored[reversed..reversed + 3]
                );
            }
        }
        for brightness in [-0.5, 0.0, 0.5] {
            config.effects = crate::effects::Effects {
                brightness,
                ..Default::default()
            };
            let frame = render(&config);
            let mean =
                |bytes: &[u8]| bytes.iter().map(|&b| b as f64).sum::<f64>() / bytes.len() as f64;
            if brightness < 0.0 {
                assert!(mean(&frame) < mean(&baseline));
            } else if brightness > 0.0 {
                assert!(mean(&frame) > mean(&baseline));
            } else {
                assert_eq!(frame, baseline);
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let lut = dir.path().join("look's \\ path,with:[brackets];.cube");
        let mut content = String::from("LUT_3D_SIZE 2\n");
        // A constant LUT makes incorrect parsing or a bypass observable in the pixels.
        for _ in 0..8 {
            content.push_str("0.25 0.5 0.75\n");
        }
        std::fs::write(&lut, content).unwrap();
        config.effects = crate::effects::Effects {
            lut: lut.to_str().unwrap().into(),
            ..Default::default()
        };
        let lut_frame = render(&config);
        for pixel in lut_frame.as_chunks::<3>().0.iter().step_by(1000) {
            for (actual, expected) in pixel.iter().zip([64i16, 128, 191]) {
                assert!((*actual as i16 - expected).abs() <= 2);
            }
        }
        // Exercise both mapped outputs with a raw-file substitute for the V4L2
        // sink, so this test also verifies delivery without loading a kernel module.
        let mode = Mode {
            format: "mjpeg".into(),
            width: 640,
            height: 360,
            fps: 30.0,
        };
        let original = ffmpeg_args("unused", &mode, &config, true, true);
        let end = original
            .iter()
            .position(|a| a == "-filter_complex")
            .unwrap();
        let mut args: Vec<String> = [
            "-hide_banner",
            "-loglevel",
            "error",
            "-filter_complex_threads",
            "1",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=640x360:rate=30:duration=0.2",
        ]
        .map(str::to_owned)
        .into();
        args.extend_from_slice(&original[end..]);
        let sink = args
            .iter()
            .position(|a| a == &config.virtual_device)
            .unwrap();
        let output_file = dir.path().join("virtual.yuv");
        args[sink - 1] = "rawvideo".into();
        args[sink] = output_file.to_str().unwrap().into();
        let both = Command::new("ffmpeg").args(args).output().unwrap();
        assert!(
            both.status.success(),
            "{}",
            String::from_utf8_lossy(&both.stderr)
        );
        assert!(both.stdout.len() >= lut_frame.len());
        for (actual, expected) in both.stdout[..lut_frame.len()].iter().zip(&lut_frame) {
            assert!((*actual as i16 - *expected as i16).abs() <= 2);
        }
        let virtual_frames = std::fs::read(output_file).unwrap();
        let plane = PREVIEW_WIDTH * PREVIEW_HEIGHT;
        assert!(virtual_frames.len() >= plane * 3 / 2);
        let y = virtual_frames[0];
        assert!((110..=121).contains(&y));
        assert!(virtual_frames[..plane].iter().all(|&v| v.abs_diff(y) <= 1));
        assert!(virtual_frames[plane] > 128 && virtual_frames[plane + plane / 4] < 128);
        config.effects = crate::effects::Effects {
            denoise: 3.0,
            saturation: 0.0,
            contrast: 1.5,
            ..Default::default()
        };
        let gray = render(&config);
        for pixel in gray.as_chunks::<3>().0.iter().step_by(1000) {
            // YUV -> RGB integer conversion can differ by up to four code values.
            assert!((pixel[0] as i16 - pixel[1] as i16).abs() <= 4, "{pixel:?}");
            assert!((pixel[1] as i16 - pixel[2] as i16).abs() <= 4, "{pixel:?}");
        }
        let mode = Mode {
            format: "mjpeg".into(),
            width: 640,
            height: 360,
            fps: 30.0,
        };
        let args = ffmpeg_args("unused", &mode, &config, true, true);
        let graph = &args[args.iter().position(|a| a == "-filter_complex").unwrap() + 1];
        assert_eq!(graph.matches("hqdn3d").count(), 1);
        assert!(graph.find("hqdn3d").unwrap() < graph.find("split=2").unwrap());
    }

    #[test]
    #[ignore = "requires an attached Facecam and ffmpeg; captures frames only in memory"]
    fn hardware_preview_produces_frames_and_reaps_its_process() {
        let cameras = crate::camera::discover().unwrap();
        let camera = cameras.first().expect("connect a Facecam");
        let config = Config::default();
        let modes = crate::camera::parse_modes(
            &crate::camera::v4l2(&camera.path, &["--list-formats-ext"]).unwrap(),
        );
        let mode = crate::camera::choose_mode(&modes, &config).unwrap();
        let frames: LatestFrame = Arc::default();
        let dir = tempfile::tempdir().unwrap();
        let mut stream = Stream::start(
            &camera.path,
            &mode,
            &config,
            false,
            true,
            frames.clone(),
            &dir.path().join("ffmpeg.log"),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(8);
        while stream.frames_received.load(Ordering::Relaxed) < 3 {
            stream.poll().unwrap();
            assert!(Instant::now() < deadline, "no complete preview frames");
            thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(
            frames.frame.lock().unwrap().as_ref().unwrap().rgb.len(),
            mode.width as usize * mode.height as usize * 3
        );
        assert_eq!(
            frames.frame.lock().unwrap().as_ref().unwrap().size,
            [mode.width as usize, mode.height as usize]
        );
        stream.stop().unwrap();
        assert!(stream.child.try_wait().unwrap().is_some());
    }
}
