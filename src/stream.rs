use crate::virtual_sink::{self, i420_len};
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
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// One I420 (yuv420p) frame, exactly as delivered to the virtual camera.
pub struct PreviewFrame {
    // Headless controllers can receive/count frames without a display consumer.
    #[cfg_attr(not(feature = "gui"), allow(dead_code))]
    pub size: [usize; 2],
    #[cfg_attr(not(feature = "gui"), allow(dead_code))]
    pub yuv: Vec<u8>,
}
pub type Wake = Arc<Mutex<Option<Box<dyn Fn() + Send>>>>;
#[derive(Default)]
pub struct PreviewFrames {
    pub frame: Mutex<Option<PreviewFrame>>,
    pub wake: Wake,
}
pub type LatestFrame = Arc<PreviewFrames>;

/// Converts an I420 frame to RGB24 with the BT.601 limited-range matrix that
/// FFmpeg applies to untagged yuv420p, sharing each chroma sample with its
/// 2×2 block. The GUI converts only frames it actually displays, instead of
/// FFmpeg converting every frame while the preview is hidden.
#[cfg_attr(not(any(feature = "gui", test)), allow(dead_code))]
pub fn i420_to_rgb([width, height]: [usize; 2], yuv: &[u8], rgb: &mut Vec<u8>) -> Result<()> {
    if yuv.len() != i420_len([width, height]) {
        bail!("Quadro I420 com tamanho inesperado");
    }
    let chroma_width = width.div_ceil(2);
    let (luma, chroma) = yuv.split_at(width * height);
    let (u_plane, v_plane) = chroma.split_at(chroma.len() / 2);
    rgb.clear();
    rgb.reserve(width * height * 3);
    for y in 0..height {
        let row = &luma[y * width..(y + 1) * width];
        let chroma_row = (y / 2) * chroma_width;
        for (x, &luma) in row.iter().enumerate() {
            let c = 298 * (i32::from(luma) - 16) + 128;
            let u = i32::from(u_plane[chroma_row + x / 2]) - 128;
            let v = i32::from(v_plane[chroma_row + x / 2]) - 128;
            let clamp = |value: i32| (value >> 8).clamp(0, 255) as u8;
            rgb.extend_from_slice(&[
                clamp(c + 409 * v),
                clamp(c - 100 * u - 208 * v),
                clamp(c + 516 * u),
            ]);
        }
    }
    Ok(())
}

pub fn output_fps(mode: &Mode, effects: &crate::effects::Effects) -> f64 {
    if effects.ai_enabled() {
        mode.fps.min(f64::from(effects.ai_fps))
    } else {
        mode.fps
    }
}

fn ai_capture_filter(mode: &Mode, effects: &crate::effects::Effects) -> String {
    let fps = output_fps(mode, effects);
    let mut filter = if effects.mirror {
        format!("fps={fps},hflip")
    } else {
        format!("fps={fps}")
    };
    // FFmpeg 6.1 converts NV12 to RGB without its vectorized path; going
    // through planar 4:2:0 first measured 37% of a core instead of 71% at
    // 1080p60.
    if mode.format == "nv12" {
        filter.push_str(",format=yuv420p");
    }
    filter
}

pub struct Outputs {
    pub virtual_output: bool,
    pub preview: bool,
}

/// Destination of processed frames for the persistent virtual camera writer;
/// `None` while only the preview is active.
type SinkTarget = Arc<Mutex<Option<Arc<virtual_sink::Input>>>>;

pub struct Stream {
    child: Child,
    upstream: Children,
    reader: Option<JoinHandle<()>>,
    preview_enabled: Arc<AtomicBool>,
    sink: SinkTarget,
    frames: LatestFrame,
    log_start: u64,
    pub size: [usize; 2],
    pub fps: f64,
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

/// Final FFmpeg stage: common effects, then yuv420p on stdout. The controller
/// forwards these frames to the persistent virtual camera writer and, only
/// when visible, to the preview. One branch serves both outputs, so the
/// preview always shows the exact pixels virtual camera consumers receive.
pub fn ffmpeg_args(device: &str, mode: &Mode, config: &Config) -> Vec<String> {
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
    filters.push("format=yuv420p".into());
    args.extend([
        "-filter_complex".into(),
        format!("[0:v]{}[out]", filters.join(",")),
    ]);
    args.extend(
        [
            "-map", "[out]", "-an", "-pix_fmt", "yuv420p", "-threads", "1", "-f", "rawvideo",
            "pipe:1",
        ]
        .into_iter()
        .map(str::to_owned),
    );
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
        outputs: Outputs,
        sink: Option<Arc<virtual_sink::Input>>,
        frames: LatestFrame,
        log: &Path,
    ) -> Result<Self> {
        let Outputs {
            virtual_output,
            preview,
        } = outputs;
        if virtual_output && sink.is_none() {
            bail!("Saída da câmera virtual indisponível");
        }
        config.effects.preflight()?;
        let size = [usize::try_from(mode.width)?, usize::try_from(mode.height)?];
        if size[0] == 0 || size[1] == 0 || size[0] > 7680 || size[1] > 4320 {
            bail!("Resolução de vídeo fora dos limites");
        }
        // SetEffects validates inference before stopping an active capture.
        // Automatic backend selection probes RTX in an isolated child.
        if fs::metadata(log).is_ok_and(|m| m.len() > 2 * 1024 * 1024) {
            File::create(log)?;
        }
        let err = OpenOptions::new().create(true).append(true).open(log)?;
        let log_start = err.metadata()?.len();
        let mut upstream = Children::default();
        let mut ai_backend = None;
        let mut args = ffmpeg_args(device, mode, config);
        let fps = output_fps(mode, &config.effects);
        let stdin = if config.effects.ai_enabled() {
            let (ai_width, ai_height) = (mode.width, mode.height);
            let end = args.iter().position(|a| a == "-i").unwrap() + 2;
            let mut capture = args[..end].to_vec();
            capture.extend(["-vf".into(), ai_capture_filter(mode, &config.effects)]);
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
                .args(["--ai-rate", &fps.to_string()])
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
                fps,
            };
            args = ffmpeg_args("pipe:0", &raw_mode, config);
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
            .stdout(Stdio::piped())
            .stderr(err)
            .spawn()
            .context("Não foi possível iniciar FFmpeg; instale o pacote ffmpeg")?;
        let frames_received = Arc::new(AtomicU64::new(0));
        let preview_enabled = Arc::new(AtomicBool::new(preview));
        let sink: SinkTarget = Arc::new(Mutex::new(sink.filter(|_| virtual_output)));
        let reader = child.stdout.take().map(|stdout| {
            frame_reader(
                stdout,
                size,
                frames.clone(),
                preview_enabled.clone(),
                sink.clone(),
                frames_received.clone(),
            )
        });
        Ok(Self {
            child,
            upstream,
            reader,
            preview_enabled,
            sink,
            frames,
            log_start,
            size,
            fps,
            started: Instant::now(),
            log: log.into(),
            frames_received,
            ai_backend,
        })
    }

    /// Switches the preview without touching processes, so virtual camera
    /// consumers never notice.
    pub fn set_preview(&mut self, enabled: bool) -> bool {
        if enabled && self.reader.is_none() {
            return false;
        }
        let mut frame = self.frames.frame.lock().unwrap_or_else(|e| e.into_inner());
        self.preview_enabled.store(enabled, Ordering::Relaxed);
        if !enabled {
            *frame = None;
        }
        true
    }

    /// Starts or stops forwarding frames to the virtual camera writer without
    /// restarting capture.
    pub fn set_virtual(&mut self, sink: Option<Arc<virtual_sink::Input>>) {
        *self.sink.lock().unwrap_or_else(|e| e.into_inner()) = sink;
    }

    pub fn poll(&mut self) -> Result<bool> {
        if let Some(status) = self.child.try_wait()? {
            use std::io::{Seek, SeekFrom};
            let text = (|| -> std::io::Result<String> {
                let mut file = File::open(&self.log)?;
                let start = self
                    .log_start
                    .max(file.metadata()?.len().saturating_sub(64 * 1024));
                file.seek(SeekFrom::Start(start))?;
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;
                Ok(String::from_utf8_lossy(&bytes).into_owned())
            })()
            .unwrap_or_default();
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
        self.set_virtual(None);
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

/// Reads complete I420 frames from the final FFmpeg stage. Each frame goes to
/// the virtual camera writer when enabled, and to the preview slot only while
/// the preview is published. Neither consumer can stall capture: both keep
/// just the newest frame.
fn frame_reader(
    mut stdout: impl Read + Send + 'static,
    size: [usize; 2],
    delivery: LatestFrame,
    publishing: Arc<AtomicBool>,
    sink: SinkTarget,
    counter: Arc<AtomicU64>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let frame_bytes = i420_len(size);
        let mut yuv = vec![0; frame_bytes];
        while stdout.read_exact(&mut yuv).is_ok() {
            if let Some(input) = &*sink.lock().unwrap_or_else(|e| e.into_inner()) {
                input.submit(&yuv);
            }
            let mut slot = delivery.frame.lock().unwrap_or_else(|e| e.into_inner());
            if !publishing.load(Ordering::Relaxed) {
                continue;
            }
            let next = slot
                .take()
                .filter(|frame| frame.yuv.len() == frame_bytes)
                .map(|frame| frame.yuv)
                .unwrap_or_else(|| vec![0; frame_bytes]);
            *slot = Some(PreviewFrame {
                size,
                yuv: std::mem::replace(&mut yuv, next),
            });
            counter.fetch_add(1, Ordering::Relaxed);
            drop(slot);
            if let Some(wake) = &*delivery.wake.lock().unwrap_or_else(|e| e.into_inner()) {
                wake();
            }
        }
    })
}

#[cfg(test)]
impl Stream {
    /// A fake capture producing 2×2 I420 frames (6 bytes) every 20 ms.
    pub(crate) fn synthetic(log: &Path, frames: LatestFrame) -> Self {
        let mut child = Command::new("sh")
            .args(["-c", "while :; do printf 012345; sleep 0.02; done"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let frames_received = Arc::new(AtomicU64::new(0));
        let preview_enabled = Arc::new(AtomicBool::new(false));
        let sink: SinkTarget = Arc::default();
        let reader = Some(frame_reader(
            child.stdout.take().unwrap(),
            [2, 2],
            frames.clone(),
            preview_enabled.clone(),
            sink.clone(),
            frames_received.clone(),
        ));
        Self {
            child,
            upstream: Children::default(),
            reader,
            preview_enabled,
            sink,
            frames,
            log: log.into(),
            log_start: 0,
            size: [2, 2],
            fps: 50.0,
            started: Instant::now(),
            frames_received,
            ai_backend: None,
        }
    }
    pub(crate) fn pid(&self) -> u32 {
        self.child.id()
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
    fn stream_failure_reports_only_the_current_capture_log() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("stream.log");
        fs::write(&log, "Old shutdown: Broken pipe\n").unwrap();
        let mut stream = Stream::synthetic(&log, Arc::default());
        stream.log_start = fs::metadata(&log).unwrap().len();
        writeln!(
            OpenOptions::new().append(true).open(&log).unwrap(),
            "Current capture: device busy"
        )
        .unwrap();
        stream.child.kill().unwrap();
        stream.child.wait().unwrap();
        let error = stream.poll().unwrap_err().to_string();
        assert!(error.contains("Current capture: device busy"));
        assert!(!error.contains("Old shutdown"));
    }
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
    fn final_stage_emits_one_yuv_branch_and_never_opens_the_virtual_device() {
        let mode = Mode {
            format: "mjpeg".into(),
            width: 1280,
            height: 720,
            fps: 30.0,
        };
        let args = ffmpeg_args("/dev/video0", &mode, &Config::default());
        // The persistent writer owns the loopback; the capture pipeline must not
        // open it, or restarting capture would make consumers lose the camera.
        assert!(!args.iter().any(|a| a == "/dev/video10"));
        assert_eq!(
            args.iter().filter(|a| *a == "v4l2").count(),
            1,
            "only the camera input"
        );
        assert!(
            !args
                .iter()
                .any(|a| a.contains("rgb24") || a.contains("split"))
        );
        assert_eq!(
            &args[args.len() - 5..],
            ["-threads", "1", "-f", "rawvideo", "pipe:1"]
        );
        assert_eq!(args.iter().filter(|a| *a == "-map").count(), 1);
        assert_eq!(args.iter().filter(|a| *a == "yuv420p").count(), 1);
    }

    #[test]
    fn i420_conversion_matches_bt601_limited_range_and_rejects_bad_frames() {
        let mut rgb = Vec::new();
        // Black, white and the limited-range primaries.
        for (yuv, expected) in [
            ([16, 128, 128], [0, 0, 0]),
            ([235, 128, 128], [255, 255, 255]),
            ([81, 90, 240], [255, 0, 0]),
            ([145, 54, 34], [0, 255, 1]),
            ([41, 240, 110], [0, 0, 255]),
            // Out-of-range codes clamp instead of wrapping.
            ([0, 0, 0], [0, 135, 0]),
            ([255, 255, 255], [255, 125, 255]),
        ] {
            let frame = [yuv[0], yuv[1], yuv[2]];
            i420_to_rgb([1, 1], &frame, &mut rgb).unwrap();
            for (actual, wanted) in rgb.iter().zip(expected) {
                assert!(actual.abs_diff(wanted) <= 1, "{yuv:?} -> {rgb:?}");
            }
        }
        // Odd sizes share the last chroma column/row.
        let size = [3, 3];
        let mut frame = vec![128; i420_len(size)];
        frame[..9].fill(126);
        frame[9..13].copy_from_slice(&[128, 128, 128, 240]);
        i420_to_rgb(size, &frame, &mut rgb).unwrap();
        assert_eq!(rgb.len(), 27);
        assert_eq!(&rgb[..3], [128, 128, 128]);
        assert!(rgb[(2 * 3 + 2) * 3] < rgb[(2 * 3 + 2) * 3 + 2]);
        for bad in [0, 1, i420_len(size) - 1, i420_len(size) + 1] {
            assert!(i420_to_rgb(size, &vec![0; bad], &mut rgb).is_err());
        }
    }

    #[test]
    fn reader_forwards_to_the_virtual_writer_without_publishing_hidden_previews() {
        let dir = tempfile::tempdir().unwrap();
        let frames: LatestFrame = Arc::default();
        let mut stream = Stream::synthetic(&dir.path().join("log"), frames.clone());
        let sink = virtual_sink::Sink::synthetic("/dev/video10", [2, 2], &dir.path().join("sink"));
        stream.set_virtual(Some(sink.input()));
        let deadline = Instant::now() + Duration::from_secs(2);
        while sink.input().submitted() < 3 {
            assert!(
                Instant::now() < deadline,
                "virtual output received no frames"
            );
            thread::sleep(Duration::from_millis(10));
        }
        assert!(frames.frame.lock().unwrap().is_none());
        assert_eq!(stream.frames_received.load(Ordering::Relaxed), 0);
        assert!(stream.set_preview(true));
        while stream.frames_received.load(Ordering::Relaxed) == 0 {
            assert!(Instant::now() < deadline, "preview received no frames");
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            frames.frame.lock().unwrap().as_ref().unwrap().yuv,
            b"012345"
        );
        stream.set_virtual(None);
        thread::sleep(Duration::from_millis(30));
        let forwarded = sink.input().submitted();
        thread::sleep(Duration::from_millis(80));
        assert_eq!(sink.input().submitted(), forwarded);
        stream.stop().unwrap();
    }

    #[test]
    fn ai_mirror_is_applied_to_capture_and_never_to_the_composed_frame() {
        let mode = Mode {
            format: "rgb24".into(),
            width: 64,
            height: 32,
            fps: 30.0,
        };
        for (background_blur, eye_contact) in [(true, false), (false, true), (true, true)] {
            let effects = crate::effects::Effects {
                mirror: true,
                background_blur,
                eye_contact,
                ai_fps: 24,
                ..Default::default()
            };
            assert_eq!(ai_capture_filter(&mode, &effects), "fps=24,hflip");
            let nv12 = Mode {
                format: "nv12".into(),
                ..mode.clone()
            };
            assert_eq!(
                ai_capture_filter(&nv12, &effects),
                "fps=24,hflip,format=yuv420p"
            );
            assert!(!effects.filters().iter().any(|f| f == "hflip"));
        }
        let effects = crate::effects::Effects {
            mirror: true,
            ..Default::default()
        };
        assert!(effects.filters().iter().any(|f| f == "hflip"));
    }

    #[test]
    #[ignore = "requires FFmpeg; synthetic camera and asymmetric background, no model or hardware"]
    fn ffmpeg_mirror_keeps_background_orientation_and_preview_matches_virtual_pixels() {
        let dir = tempfile::tempdir().unwrap();
        let mode = Mode {
            format: "rgb24".into(),
            width: 64,
            height: 32,
            fps: 5.0,
        };
        let camera_file = dir.path().join("camera.rgb");
        let mut camera = vec![0_u8; 64 * 32 * 3];
        for y in 0..16 {
            for x in 8..16 {
                camera[(y * 64 + x) * 3] = 255;
            }
        }
        fs::write(&camera_file, camera).unwrap();
        for mirror in [false, true] {
            let config = Config {
                effects: crate::effects::Effects {
                    mirror,
                    background_blur: true,
                    ..Default::default()
                },
                ..Default::default()
            };
            let captured = Command::new("ffmpeg")
                .args([
                    "-v",
                    "error",
                    "-filter_threads",
                    "1",
                    "-f",
                    "rawvideo",
                    "-pixel_format",
                    "rgb24",
                    "-video_size",
                    "64x32",
                    "-framerate",
                    "5",
                    "-i",
                ])
                .arg(&camera_file)
                .args([
                    "-vf",
                    &ai_capture_filter(&mode, &config.effects),
                    "-frames:v",
                    "1",
                    "-pix_fmt",
                    "rgb24",
                    "-threads",
                    "1",
                    "-f",
                    "rawvideo",
                    "pipe:1",
                ])
                .output()
                .unwrap();
            assert!(
                captured.status.success(),
                "{}",
                String::from_utf8_lossy(&captured.stderr)
            );
            assert_eq!(captured.stdout.len(), 64 * 32 * 3);
            let mut composed = captured.stdout;
            // A deterministic mask isolates the top half as camera/person.
            // The replacement background has distinct left and right colors.
            for y in 16..32 {
                for x in 0..64 {
                    composed[(y * 64 + x) * 3..(y * 64 + x) * 3 + 3].copy_from_slice(if x < 32 {
                        &[0, 0, 255]
                    } else {
                        &[255, 255, 0]
                    });
                }
            }
            let input = dir.path().join("composed.rgb");
            fs::write(&input, composed).unwrap();
            let mut args = ffmpeg_args("unused", &mode, &config);
            let filter = args.iter().position(|a| a == "-filter_complex").unwrap();
            args.drain(..filter);
            let output = Command::new("ffmpeg")
                .args([
                    "-v",
                    "error",
                    "-filter_complex_threads",
                    "1",
                    "-f",
                    "rawvideo",
                    "-pixel_format",
                    "rgb24",
                    "-video_size",
                    "64x32",
                    "-framerate",
                    "5",
                    "-i",
                ])
                .arg(&input)
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            // Preview and virtual camera now share this exact I420 frame.
            assert_eq!(output.stdout.len(), i420_len([64, 32]));
            let mut rgb = Vec::new();
            i420_to_rgb([64, 32], &output.stdout, &mut rgb).unwrap();
            let pixel = |x: usize, y: usize| &rgb[(y * 64 + x) * 3..(y * 64 + x) * 3 + 3];
            let red = pixel(if mirror { 52 } else { 12 }, 8);
            assert!(red[0] > 240 && red[1] < 15 && red[2] < 15);
            assert!(
                pixel(if mirror { 12 } else { 52 }, 8)
                    .iter()
                    .all(|v| *v < 15)
            );
            let blue = pixel(12, 24);
            let yellow = pixel(52, 24);
            assert!(blue[0] < 15 && blue[1] < 15 && blue[2] > 240);
            assert!(yellow[0] > 240 && yellow[1] > 240 && yellow[2] < 15);
        }
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
            let original = ffmpeg_args("unused", &mode, &config);
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
            let output = Command::new("ffmpeg").args(args).output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let size = [width as usize, height as usize];
            let frame = i420_len(size);
            let count = output.stdout.len() / frame;
            assert_eq!(count, if fps == 60.0 { 12 } else { 6 });
            assert_eq!(output.stdout.len(), count * frame);
            let virtual_file = dir.path().join(format!("{width}.yuv"));
            fs::write(&virtual_file, &output.stdout).unwrap();
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
            // The GUI converts the same I420 frames the virtual camera receives;
            // integer rounding may differ slightly from FFmpeg's converter.
            let mut preview = Vec::new();
            let mut worst = 0;
            for (yuv, reference) in output
                .stdout
                .chunks_exact(frame)
                .zip(decoded.stdout.chunks_exact(size[0] * size[1] * 3))
            {
                i420_to_rgb(size, yuv, &mut preview).unwrap();
                for (a, b) in preview.iter().zip(reference) {
                    worst = worst.max(a.abs_diff(*b));
                }
            }
            assert!(
                worst <= 3,
                "preview differs from FFmpeg by {worst} code values"
            );
        }
    }

    #[test]
    #[ignore = "requires ffmpeg; uses synthetic frames, no camera or display"]
    fn ffmpeg_effects_render() {
        fn render_yuv(config: &Config) -> Vec<u8> {
            let mode = Mode {
                format: "mjpeg".into(),
                width: 640,
                height: 360,
                fps: 30.0,
            };
            let original = ffmpeg_args("unused", &mode, config);
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
            let frame = i420_len([PREVIEW_WIDTH, PREVIEW_HEIGHT]);
            assert!(output.stdout.len() >= frame);
            output.stdout[..frame].to_vec()
        }
        // The preview shows the GUI conversion of the exact virtual frame.
        fn render(config: &Config) -> Vec<u8> {
            let mut rgb = Vec::new();
            i420_to_rgb(
                [PREVIEW_WIDTH, PREVIEW_HEIGHT],
                &render_yuv(config),
                &mut rgb,
            )
            .unwrap();
            rgb
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
        // The single I420 branch is what the virtual camera writer receives.
        let virtual_frames = render_yuv(&config);
        let plane = PREVIEW_WIDTH * PREVIEW_HEIGHT;
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
        let args = ffmpeg_args("unused", &mode, &config);
        let graph = &args[args.iter().position(|a| a == "-filter_complex").unwrap() + 1];
        assert_eq!(graph.matches("hqdn3d").count(), 1);
        assert!(graph.find("hqdn3d").unwrap() < graph.find("format=yuv420p").unwrap());
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
            Outputs {
                virtual_output: false,
                preview: true,
            },
            None,
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
            frames.frame.lock().unwrap().as_ref().unwrap().yuv.len(),
            i420_len([mode.width as usize, mode.height as usize])
        );
        assert_eq!(
            frames.frame.lock().unwrap().as_ref().unwrap().size,
            [mode.width as usize, mode.height as usize]
        );
        stream.stop().unwrap();
        assert!(stream.child.try_wait().unwrap().is_some());
    }
}
