//! Persistent v4l2loopback writer.
//!
//! With `exclusive_caps=1` the loopback announces capture only while a writer
//! streams. Restarting the capture pipeline used to close that writer, so
//! browsers saw the virtual camera vanish and fell back to the physical
//! Facecam, which the pipeline itself keeps busy. One FFmpeg process owns the
//! device for the controller's lifetime instead: the pipeline submits I420
//! frames, and during restarts or while stopped this module repeats the last
//! frame or a black frame at a low rate.

use anyhow::{Context, Result, bail};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Repetition interval while no new frame arrives (5 FPS).
const IDLE_INTERVAL: Duration = Duration::from_millis(200);
const READY_TIMEOUT: Duration = Duration::from_secs(3);

/// Byte length of one I420 (yuv420p) frame, including odd dimensions.
pub fn i420_len([width, height]: [usize; 2]) -> usize {
    width * height + 2 * width.div_ceil(2) * height.div_ceil(2)
}

/// Limited-range black, the neutral placeholder shown while capture is stopped.
pub fn black_frame(size: [usize; 2]) -> Vec<u8> {
    let luma = size[0] * size[1];
    let mut frame = vec![128; i420_len(size)];
    frame[..luma].fill(16);
    frame
}

#[derive(Default)]
struct Slot {
    pending: Option<Vec<u8>>,
    spare: Option<Vec<u8>>,
    blank: bool,
    closed: bool,
    submitted: u64,
}

/// Frame hand-off between the capture reader and the writer thread. Only the
/// newest frame is kept: a slow device write drops stale frames instead of
/// stalling the pipeline.
#[derive(Default)]
pub struct Input {
    slot: Mutex<Slot>,
    ready: Condvar,
}

impl Input {
    pub fn submit(&self, frame: &[u8]) {
        let mut slot = self.slot.lock().unwrap_or_else(|e| e.into_inner());
        if slot.closed {
            return;
        }
        let mut buffer = slot
            .pending
            .take()
            .or_else(|| slot.spare.take())
            .unwrap_or_default();
        buffer.clear();
        buffer.extend_from_slice(frame);
        slot.pending = Some(buffer);
        slot.blank = false;
        slot.submitted += 1;
        drop(slot);
        self.ready.notify_one();
    }

    #[cfg(test)]
    pub fn submitted(&self) -> u64 {
        self.slot.lock().unwrap().submitted
    }

    /// Waits for the next frame to write. `last` holds the frame currently
    /// shown; it is replaced by a new frame, by black after `blank`, or kept to
    /// be repeated when the idle interval passes. Returns false once closed.
    fn next(&self, last: &mut Vec<u8>, size: [usize; 2], idle: Duration) -> bool {
        let mut slot = self.slot.lock().unwrap_or_else(|e| e.into_inner());
        let deadline = Instant::now() + idle;
        while slot.pending.is_none() && !slot.closed && !slot.blank {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            slot = self
                .ready
                .wait_timeout(slot, deadline - now)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
        if slot.closed {
            return false;
        }
        if let Some(frame) = slot.pending.take() {
            slot.spare = Some(std::mem::replace(last, frame));
        } else if slot.blank {
            slot.blank = false;
            *last = black_frame(size);
        }
        true
    }

    fn blank(&self) {
        let mut slot = self.slot.lock().unwrap_or_else(|e| e.into_inner());
        slot.pending = None;
        slot.blank = true;
        drop(slot);
        self.ready.notify_one();
    }

    fn close(&self) {
        self.slot.lock().unwrap_or_else(|e| e.into_inner()).closed = true;
        self.ready.notify_all();
    }
}

pub fn ffmpeg_args(device: &str, [width, height]: [usize; 2], fps: f64) -> Vec<String> {
    [
        "-hide_banner",
        "-loglevel",
        "warning",
        "-nostdin",
        "-f",
        "rawvideo",
        "-pixel_format",
        "yuv420p",
        "-video_size",
        &format!("{width}x{height}"),
        "-framerate",
        &fps.to_string(),
        "-probesize",
        "32",
        "-analyzeduration",
        "0",
        "-i",
        "pipe:0",
        "-map",
        "0:v",
        "-an",
        // Frames arrive at the pipeline rate, or at the idle rate while
        // stopped; FFmpeg must not duplicate or drop them by timestamp.
        "-fps_mode",
        "passthrough",
        "-c:v",
        "rawvideo",
        "-pix_fmt",
        "yuv420p",
        "-threads",
        "1",
        "-f",
        "v4l2",
        device,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

/// v4l2loopback reports `capture` in sysfs while a writer streams.
fn state_file(device: &str) -> Option<PathBuf> {
    let name = fs::canonicalize(device).ok()?.file_name()?.to_owned();
    let path = Path::new("/sys/class/video4linux").join(name).join("state");
    path.is_file().then_some(path)
}

fn writer_streaming(state: &Path) -> bool {
    fs::read_to_string(state).is_ok_and(|s| s.trim() == "capture")
}

pub struct Sink {
    child: Child,
    input: Arc<Input>,
    writer: Option<JoinHandle<()>>,
    pub device: String,
    pub size: [usize; 2],
    fps: f64,
    log: PathBuf,
    log_start: u64,
}

impl Sink {
    pub fn open(device: &str, size: [usize; 2], fps: f64, log: &Path) -> Result<Self> {
        if size[0] == 0 || size[1] == 0 || size[0] > 7680 || size[1] > 4320 {
            bail!("Resolução da câmera virtual fora dos limites");
        }
        if !fps.is_finite() || fps <= 0.0 || fps > 240.0 {
            bail!("FPS da câmera virtual inválido");
        }
        crate::stream::verify_virtual_device(device)?;
        let state = state_file(device);
        if state.as_deref().is_some_and(writer_streaming) {
            bail!("Outro processo já transmite em {device}; feche-o para usar a câmera virtual");
        }
        if fs::metadata(log).is_ok_and(|m| m.len() > 2 * 1024 * 1024) {
            File::create(log)?;
        }
        let err = OpenOptions::new().create(true).append(true).open(log)?;
        let log_start = err.metadata()?.len();
        let mut child = Command::new("ffmpeg")
            .args(ffmpeg_args(device, size, fps))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(err)
            .spawn()
            .context("Não foi possível iniciar FFmpeg; instale o pacote ffmpeg")?;
        let stdin = child
            .stdin
            .take()
            .context("Pipe da câmera virtual indisponível")?;
        let input = Arc::new(Input::default());
        let writer = Some(spawn_writer(stdin, input.clone(), size));
        let mut sink = Self {
            child,
            input,
            writer,
            device: device.into(),
            size,
            fps: 0.0,
            log: log.into(),
            log_start,
        };
        // FFmpeg opens the device after the first frame; wait until the
        // loopback announces capture so consumers never see a half-open sink.
        // Without the sysfs state (other loopback builds), a writer that is
        // still alive shortly after the first frame is taken as ready.
        let opened = Instant::now();
        loop {
            sink.poll()?;
            let ready = match &state {
                Some(state) => writer_streaming(state),
                None => opened.elapsed() >= Duration::from_millis(300),
            };
            if ready {
                break;
            }
            if opened.elapsed() >= READY_TIMEOUT {
                bail!("A câmera virtual não ficou disponível em {device}");
            }
            thread::sleep(Duration::from_millis(20));
        }
        sink.set_fps(fps);
        Ok(sink)
    }

    pub fn input(&self) -> Arc<Input> {
        self.input.clone()
    }

    /// Announces the real output rate. FFmpeg's V4L2 muxer never sets it, so
    /// the loopback kept its 30 FPS default while 60 FPS were written and
    /// consumers opened the camera with the wrong rate. With a writer
    /// streaming, any opener may set the capture interval.
    pub fn set_fps(&mut self, fps: f64) {
        if fps == self.fps || !fps.is_finite() || fps <= 0.0 {
            return;
        }
        let result = crate::camera::v4l2(&self.device, &["--set-parm", &format!("{fps:.3}")]);
        match result {
            Ok(_) => self.fps = fps,
            Err(e) => {
                if let Ok(mut log) = OpenOptions::new().append(true).open(&self.log) {
                    let _ = writeln!(log, "Falha ao anunciar {fps} FPS na câmera virtual: {e:#}");
                }
            }
        }
    }

    /// Shows black until the next submitted frame, used when capture stops.
    pub fn blank(&self) {
        self.input.blank();
    }

    pub fn poll(&mut self) -> Result<()> {
        if let Some(status) = self.child.try_wait()? {
            let tail = (|| -> std::io::Result<String> {
                let mut file = File::open(&self.log)?;
                let start = self
                    .log_start
                    .max(file.metadata()?.len().saturating_sub(16 * 1024));
                file.seek(SeekFrom::Start(start))?;
                let mut text = String::new();
                file.read_to_string(&mut text)?;
                Ok(text
                    .lines()
                    .rev()
                    .take(6)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join("\n"))
            })()
            .unwrap_or_default();
            bail!("A saída da câmera virtual encerrou ({status}). {tail}");
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn synthetic(device: &str, size: [usize; 2], log: &Path) -> Self {
        let mut child = Command::new("sh")
            .args(["-c", "cat > /dev/null"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let input = Arc::new(Input::default());
        let writer = Some(spawn_writer(
            child.stdin.take().unwrap(),
            input.clone(),
            size,
        ));
        Self {
            child,
            input,
            writer,
            device: device.into(),
            size,
            fps: 0.0,
            log: log.into(),
            log_start: 0,
        }
    }
}

fn spawn_writer(mut stdin: ChildStdin, input: Arc<Input>, size: [usize; 2]) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut last = black_frame(size);
        let expected = last.len();
        // The first black frame makes FFmpeg open the device immediately.
        if stdin.write_all(&last).is_err() {
            return;
        }
        while input.next(&mut last, size, IDLE_INTERVAL) {
            if last.len() != expected {
                last = black_frame(size);
            }
            if stdin.write_all(&last).is_err() {
                break;
            }
        }
    })
}

impl Drop for Sink {
    fn drop(&mut self) {
        self.input.close();
        // The writer drops stdin when it sees the close, letting FFmpeg finish
        // cleanly and release the device. A stuck FFmpeg is killed, which
        // also unblocks a pending write.
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline
            && (self.writer.as_ref().is_some_and(|w| !w.is_finished())
                || matches!(self.child.try_wait(), Ok(None)))
        {
            thread::sleep(Duration::from_millis(20));
        }
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
        }
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_length_and_black_cover_even_odd_and_minimal_sizes() {
        assert_eq!(i420_len([1920, 1080]), 1920 * 1080 * 3 / 2);
        assert_eq!(i420_len([2, 2]), 6);
        assert_eq!(i420_len([1, 1]), 3);
        assert_eq!(i420_len([3, 5]), 15 + 2 * 2 * 3);
        let black = black_frame([4, 2]);
        assert_eq!(black, [16, 16, 16, 16, 16, 16, 16, 16, 128, 128, 128, 128]);
    }

    #[test]
    fn only_the_newest_frame_is_written_and_buffers_are_reused() {
        let input = Input::default();
        let size = [2, 2];
        let mut last = black_frame(size);
        for value in [1, 2, 3] {
            input.submit(&[value; 6]);
        }
        assert!(input.next(&mut last, size, Duration::ZERO));
        assert_eq!(last, [3; 6], "stale frames must be dropped");
        // The replaced buffer becomes the spare used by the next submission.
        let spare = input.slot.lock().unwrap().spare.as_ref().map(Vec::as_ptr);
        input.submit(&[4; 6]);
        assert_eq!(
            input.slot.lock().unwrap().pending.as_ref().map(Vec::as_ptr),
            spare
        );
        assert!(input.next(&mut last, size, Duration::ZERO));
        assert_eq!(last, [4; 6]);
        assert_eq!(input.submitted(), 4);
    }

    #[test]
    fn idle_repeats_last_frame_until_blanked_and_close_stops_the_writer() {
        let input = Input::default();
        let size = [2, 2];
        let mut last = vec![9; 6];
        let started = Instant::now();
        assert!(input.next(&mut last, size, Duration::from_millis(30)));
        assert!(started.elapsed() >= Duration::from_millis(30));
        assert_eq!(last, [9; 6], "a restart keeps showing the last frame");
        input.submit(&[7; 6]);
        input.blank();
        assert!(input.next(&mut last, size, Duration::from_secs(5)));
        assert_eq!(last, black_frame(size), "stopping discards pending frames");
        input.close();
        assert!(!input.next(&mut last, size, Duration::from_secs(5)));
        input.submit(&[1; 6]);
        assert!(input.slot.lock().unwrap().pending.is_none());
    }

    #[test]
    fn writer_never_sends_a_frame_of_the_wrong_length() {
        let dir = tempfile::tempdir().unwrap();
        let mut sink = Sink::synthetic("/dev/video10", [2, 2], &dir.path().join("log"));
        sink.input().submit(&[1; 5]);
        sink.input().submit(&[2; 6]);
        thread::sleep(Duration::from_millis(50));
        sink.poll().unwrap();
        drop(sink);
    }

    #[test]
    fn ffmpeg_writer_keeps_frames_untouched_and_does_not_retime_them() {
        let args = ffmpeg_args("/dev/video10", [1280, 720], 30000.0 / 1001.0);
        let joined = args.join(" ");
        assert!(joined.contains("-pixel_format yuv420p -video_size 1280x720"));
        assert!(joined.contains("-fps_mode passthrough"));
        assert!(!joined.contains("-vf") && !joined.contains("-filter"));
        assert_eq!(&args[args.len() - 3..], ["-f", "v4l2", "/dev/video10"]);
    }

    #[test]
    fn invalid_geometry_and_rates_are_refused_before_touching_devices() {
        let log = tempfile::tempdir().unwrap().path().join("log");
        for (size, fps) in [
            ([0, 720], 30.0),
            ([1280, 0], 30.0),
            ([7681, 720], 30.0),
            ([1280, 720], 0.0),
            ([1280, 720], -30.0),
            ([1280, 720], f64::NAN),
            ([1280, 720], f64::INFINITY),
            ([1280, 720], 241.0),
        ] {
            assert!(Sink::open("/dev/not-a-loopback", size, fps, &log).is_err());
        }
        let error = Sink::open("/dev/not-a-loopback", [1280, 720], 30.0, &log)
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("v4l2loopback"), "{error}");
    }

    #[test]
    #[ignore = "requires ffmpeg, v4l2-ctl and a free v4l2loopback at /dev/video10; writes only to the loopback"]
    fn loopback_stays_capture_capable_and_announces_the_output_rate() {
        let dir = tempfile::tempdir().unwrap();
        let mut sink =
            Sink::open("/dev/video10", [640, 360], 60.0, &dir.path().join("log")).unwrap();
        let info = crate::camera::v4l2("/dev/video10", &["--info"]).unwrap();
        assert!(info.contains("Video Capture"), "{info}");
        let parm = crate::camera::v4l2("/dev/video10", &["--get-parm"]).unwrap();
        assert!(parm.contains("60.000"), "{parm}");
        sink.blank();
        thread::sleep(Duration::from_millis(500));
        sink.poll().unwrap();
        let info = crate::camera::v4l2("/dev/video10", &["--info"]).unwrap();
        assert!(info.contains("Video Capture"), "{info}");
        drop(sink);
        let info = crate::camera::v4l2("/dev/video10", &["--info"]).unwrap();
        assert!(info.contains("Video Output"), "{info}");
    }
}
