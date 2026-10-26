//! Script settings and playback shared by GUI and headless controller.
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// JSON escaping can expand each byte sixfold; keep requests below the 64 KiB IPC cap.
pub const MAX_SCRIPT_BYTES: usize = 8 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub text: String,
    pub words_per_minute: u32,
    pub font_size: u32,
    pub mirror: bool,
    /// None presents a movable window; Some targets fullscreen in winit monitor order.
    pub monitor: Option<usize>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            text: String::new(),
            words_per_minute: 120,
            font_size: 48,
            mirror: false,
            monitor: None,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        if self.text.len() > MAX_SCRIPT_BYTES
            || self
                .text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            bail!("Roteiro inválido: use texto de até 8 KiB, sem caracteres de controle");
        }
        if !(30..=400).contains(&self.words_per_minute) {
            bail!("Velocidade de leitura deve estar entre 30 e 400 palavras por minuto");
        }
        if !(16..=128).contains(&self.font_size) {
            bail!("Tamanho do texto deve estar entre 16 e 128 pontos");
        }
        if self.monitor.is_some_and(|m| m >= 32) {
            bail!("Monitor do Prompter fora dos limites");
        }
        Ok(())
    }
    pub fn words(&self) -> usize {
        self.text.split_whitespace().count()
    }
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.words() as f64 * 60.0 / self.words_per_minute.max(1) as f64)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub playing: bool,
    pub presenting: bool,
    /// Normalized reading progress. Does not depend on monitor pixels or DPI.
    pub position: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Play,
    Pause,
    Rewind,
    Show,
    Hide,
}

pub fn advance(state: &mut State, settings: &Settings, elapsed: Duration) -> bool {
    if !state.playing {
        return false;
    }
    // Suspend or a stalled controller pauses reading instead of skipping the script.
    let duration = settings.duration().as_secs_f64();
    if elapsed > Duration::from_secs(2) || duration <= 0.0 {
        state.playing = false;
        return true;
    }
    state.position = (state.position + elapsed.as_secs_f64() / duration).min(1.0);
    if state.position >= 1.0 {
        state.playing = false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_refuse_invalid_limits_and_keep_utf8() {
        let mut settings = Settings::default();
        for value in [0, 29, 401, u32::MAX] {
            settings.words_per_minute = value;
            assert!(settings.validate().is_err());
        }
        settings.words_per_minute = 120;
        for value in [0, 15, 129, u32::MAX] {
            settings.font_size = value;
            assert!(settings.validate().is_err());
        }
        settings.font_size = 48;
        for text in ["a\0b".into(), "ç".repeat(MAX_SCRIPT_BYTES / 2 + 1)] {
            settings.text = text;
            assert!(settings.validate().is_err());
        }
        settings.text = "Olá, câmera!\nSegunda linha.\tFinal".into();
        assert!(settings.validate().is_ok());
        assert_eq!(settings.words(), 5);
        assert_eq!(settings.duration(), Duration::from_millis(2500));
        settings.monitor = Some(32);
        assert!(settings.validate().is_err());
    }
    #[test]
    fn playback_pauses_at_end_and_after_suspend_without_skipping() {
        let settings = Settings {
            text: "one two three four".into(),
            ..Default::default()
        };
        let mut state = State {
            playing: true,
            ..Default::default()
        };
        assert!(advance(&mut state, &settings, Duration::from_millis(500)));
        assert_eq!(state.position, 0.25);
        assert!(advance(&mut state, &settings, Duration::from_secs(8)));
        assert!(!state.playing);
        assert_eq!(state.position, 0.25);
        state.playing = true;
        advance(&mut state, &settings, Duration::from_millis(1500));
        assert_eq!(state.position, 1.0);
        assert!(!state.playing);
        assert!(!advance(&mut state, &settings, Duration::from_secs(1)));
    }
}
