use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    Cpu,
    #[default]
    Cuda,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Effects {
    pub mirror: bool,
    pub brightness: f64,
    pub contrast: f64,
    pub saturation: f64,
    pub denoise: f64,
    pub lut: String,
    pub background_blur: bool,
    pub blur_radius: u32,
    pub ai_provider: AiProvider,
    pub ai_model: String,
    pub ai_runtime: String,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            mirror: false,
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            denoise: 0.0,
            lut: String::new(),
            background_blur: false,
            blur_radius: 12,
            ai_provider: AiProvider::Cuda,
            ai_model: String::new(),
            ai_runtime: String::new(),
        }
    }
}

impl Effects {
    pub fn validate(&self) -> Result<()> {
        for (name, value, min, max) in [
            ("Brilho", self.brightness, -1.0, 1.0),
            ("Contraste", self.contrast, 0.0, 2.0),
            ("Saturação", self.saturation, 0.0, 3.0),
            ("Redução de ruído", self.denoise, 0.0, 10.0),
        ] {
            if !value.is_finite() || !(min..=max).contains(&value) {
                bail!("{name} deve estar entre {min} e {max}");
            }
        }
        if !(1..=32).contains(&self.blur_radius) {
            bail!("Raio do desfoque deve estar entre 1 e 32");
        }
        if self.background_blur && (self.ai_model.is_empty() || self.ai_runtime.is_empty()) {
            bail!("Informe o modelo ONNX e a biblioteca libonnxruntime.so para usar IA");
        }
        Ok(())
    }

    pub fn preflight(&self) -> Result<()> {
        self.validate()?;
        if !self.lut.is_empty() && !std::path::Path::new(&self.lut).is_file() {
            bail!("LUT não encontrada: {}", self.lut);
        }
        Ok(())
    }

    pub fn filters(&self) -> Vec<String> {
        let mut filters = Vec::new();
        if self.mirror {
            filters.push("hflip".into());
        }
        if self.denoise > 0.0 {
            filters.push(format!(
                "hqdn3d={}:{}:0:0",
                self.denoise,
                self.denoise * 0.75
            ));
        }
        if (self.brightness, self.contrast, self.saturation) != (0.0, 1.0, 1.0) {
            filters.push(format!(
                "eq=brightness={}:contrast={}:saturation={}",
                self.brightness, self.contrast, self.saturation
            ));
        }
        if !self.lut.is_empty() {
            filters.push(format!(
                "lut3d=file={}:interp=tetrahedral",
                filter_path(&self.lut)
            ));
        }
        filters
    }
}

// Escape first the option parser, then the filtergraph parser; never invoke a shell.
fn filter_path(path: &str) -> String {
    let option: String = path
        .chars()
        .flat_map(|c| {
            if matches!(c, '\\' | '\'' | ':') {
                vec!['\\', c]
            } else {
                vec![c]
            }
        })
        .collect();
    option
        .chars()
        .flat_map(|c| {
            if matches!(c, '\\' | '\'' | '[' | ']' | ';' | ',') {
                vec!['\\', c]
            } else {
                vec![c]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_effects_cover_negative_zero_positive_and_non_finite_values() {
        for value in [
            -f64::INFINITY,
            -2.0,
            -1.0,
            -0.1,
            0.0,
            0.1,
            1.0,
            2.0,
            f64::INFINITY,
            f64::NAN,
        ] {
            let effect = Effects {
                brightness: value,
                ..Effects::default()
            };
            assert_eq!(
                effect.validate().is_ok(),
                value.is_finite() && (-1.0..=1.0).contains(&value),
                "brightness {value}"
            );
        }
        for value in [-1.0, 0.0, 0.5, 2.0, 3.0, 10.0, 11.0, f64::NAN] {
            for (field, max) in [("contrast", 2.0), ("saturation", 3.0), ("denoise", 10.0)] {
                let mut e = Effects::default();
                match field {
                    "contrast" => e.contrast = value,
                    "saturation" => e.saturation = value,
                    _ => e.denoise = value,
                }
                assert_eq!(
                    e.validate().is_ok(),
                    value.is_finite() && (0.0..=max).contains(&value),
                    "{field} {value}"
                );
            }
        }
        for radius in [0, 1, 12, 32, 33, u32::MAX] {
            assert_eq!(
                Effects {
                    blur_radius: radius,
                    ..Effects::default()
                }
                .validate()
                .is_ok(),
                (1..=32).contains(&radius)
            );
        }
        assert!(serde_json::from_str::<Effects>(r#"{"blur_radius":-1}"#).is_err());
        assert!(serde_json::from_str::<Effects>(r#"{"unknown":1}"#).is_err());
    }

    #[test]
    fn defaults_do_not_add_work_to_the_video_pipeline() {
        assert!(Effects::default().filters().is_empty());
    }
}
