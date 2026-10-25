//! Bounded local-video decoding. The decoder dies with its owning AI worker.
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    os::unix::process::CommandExt,
    process::{Child, ChildStdout, Command, Stdio},
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecodeRequest {
    pub path: String,
    pub width: usize,
    pub height: usize,
    pub fps: f64,
    pub parent_pid: u32,
}

impl DecodeRequest {
    fn validate(&self) -> Result<()> {
        if self.width == 0 || self.height == 0 || self.width > 7680 || self.height > 4320 {
            bail!("Resolução do fundo de vídeo fora dos limites");
        }
        if !self.fps.is_finite() || self.fps <= 0.0 || self.fps > 60.0 {
            bail!("FPS do fundo de vídeo deve ser maior que zero e até 60");
        }
        let info = std::fs::metadata(&self.path).context("Vídeo de fundo indisponível")?;
        if !info.is_file() || info.len() == 0 || info.len() > 4 * 1024 * 1024 * 1024 {
            bail!("Vídeo de fundo deve ser um arquivo local de até 4 GiB");
        }
        Ok(())
    }
}

pub fn decode(request: &DecodeRequest) -> Result<()> {
    // Arm this in the single-purpose child before exec, using safe nix APIs.
    // Checking the expected parent also covers its death before prctl was armed.
    nix::sys::prctl::set_pdeathsig(nix::sys::signal::Signal::SIGKILL)?;
    if u32::try_from(nix::unistd::getppid().as_raw()).ok() != Some(request.parent_pid) {
        bail!("O processo responsável pelo fundo de vídeo encerrou");
    }
    request.validate()?;
    // Canonical absolute paths cannot be interpreted as a URL, pipe or option.
    let path = std::fs::canonicalize(&request.path)?;
    let error = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-max_alloc",
            "134217728",
            "-filter_threads",
            "1",
            "-threads",
            "1",
            "-protocol_whitelist",
            "file,pipe",
            "-format_whitelist",
            "mov,matroska,webm",
            "-probesize",
            "4194304",
            "-analyzeduration",
            "2000000",
            "-stream_loop",
            "-1",
            "-i",
        ])
        .arg(path)
        .args([
            "-map",
            "0:v:0",
            "-an",
            "-sn",
            "-dn",
            "-vf",
            &format!(
                "fps={},scale={}:{}:force_original_aspect_ratio=increase,crop={}:{},setsar=1",
                request.fps, request.width, request.height, request.width, request.height
            ),
            "-pix_fmt",
            "rgb24",
            "-threads",
            "1",
            "-f",
            "rawvideo",
            "pipe:1",
        ])
        .exec();
    Err(error).context("Não foi possível iniciar o decoder FFmpeg do fundo de vídeo")
}

pub struct VideoBackground {
    child: Child,
    output: ChildStdout,
    frame: Vec<u8>,
}

impl VideoBackground {
    pub fn open(path: &str, width: usize, height: usize, fps: f64) -> Result<Self> {
        let request = DecodeRequest {
            path: path.into(),
            width,
            height,
            fps,
            parent_pid: std::process::id(),
        };
        request.validate()?;
        let mut child = Command::new(std::env::current_exe()?)
            .args(["--decode-background", &serde_json::to_string(&request)?])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Não foi possível abrir o vídeo de fundo")?;
        let Some(output) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Pipe do fundo de vídeo indisponível");
        };
        Ok(Self {
            child,
            output,
            frame: vec![0; width * height * 3],
        })
    }

    pub fn frame(&mut self) -> Result<&[u8]> {
        self.output.read_exact(&mut self.frame).context(
            "Não foi possível decodificar o vídeo de fundo; use MP4, MOV ou WebM válido",
        )?;
        Ok(&self.frame)
    }
}

impl Drop for VideoBackground {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_dimensions_rates_and_non_files_are_rejected_before_spawn() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("video.mp4");
        std::fs::write(&file, [0]).unwrap();
        let mut request = DecodeRequest {
            path: file.to_string_lossy().into_owned(),
            width: 640,
            height: 360,
            fps: 30.0,
            parent_pid: std::process::id(),
        };
        for fps in [
            -f64::INFINITY,
            -1.0,
            0.0,
            0.5,
            15.0,
            29.97,
            60.0,
            61.0,
            f64::INFINITY,
            f64::NAN,
        ] {
            request.fps = fps;
            assert_eq!(
                request.validate().is_ok(),
                fps.is_finite() && fps > 0.0 && fps <= 60.0
            );
        }
        request.fps = 30.0;
        for width in [0, 1, 640, 7680, 7681, usize::MAX] {
            request.width = width;
            assert_eq!(request.validate().is_ok(), (1..=7680).contains(&width));
        }
        request.width = 640;
        for height in [0, 1, 360, 4320, 4321, usize::MAX] {
            request.height = height;
            assert_eq!(request.validate().is_ok(), (1..=4320).contains(&height));
        }
        request.height = 360;
        request.path = dir.path().to_string_lossy().into_owned();
        assert!(request.validate().is_err());
        request.path = "https://example.com/video.mp4".into();
        assert!(request.validate().is_err());
        request.path = file.to_string_lossy().into_owned();
        std::fs::write(&file, []).unwrap();
        assert!(request.validate().is_err());
        std::fs::File::create(&file)
            .unwrap()
            .set_len(4 * 1024 * 1024 * 1024 + 1)
            .unwrap();
        assert!(request.validate().is_err());
    }
}
