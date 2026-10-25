//! Local backgrounds. Static resources are prepared once; video stays bounded.
use crate::effects::{BackgroundMode, Effects};
use anyhow::{Context, Result, bail};
use image::ImageDecoder;

pub enum Background {
    Blur,
    Static(Vec<u8>),
    Video(crate::video_background::VideoBackground),
}

impl Background {
    pub fn open(effects: &Effects, w: usize, h: usize, fps: f64) -> Result<Self> {
        if effects.background_mode == BackgroundMode::Video {
            return Ok(Self::Video(crate::video_background::VideoBackground::open(
                &effects.background_video,
                w,
                h,
                fps,
            )?));
        }
        let image = if effects.background_mode == BackgroundMode::Image {
            Some(load_image(&effects.background_image)?)
        } else {
            None
        };
        Ok(match prepare(effects, w, h, image)? {
            Some(rgb) => Self::Static(rgb),
            None => Self::Blur,
        })
    }

    pub fn frame(&mut self) -> Result<Option<&[u8]>> {
        match self {
            Self::Blur => Ok(None),
            Self::Static(rgb) => Ok(Some(rgb)),
            Self::Video(video) => Ok(Some(video.frame()?)),
        }
    }
}

pub fn load_image(path: &str) -> Result<image::DynamicImage> {
    let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::WebP)
    ) {
        bail!("Use uma imagem PNG, JPEG ou WebP");
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .context("Não foi possível decodificar a imagem de fundo")?;
    let orientation = decoder.orientation()?;
    let mut image = image::DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok(image)
}

pub fn prepare(
    effects: &Effects,
    w: usize,
    h: usize,
    image: Option<image::DynamicImage>,
) -> Result<Option<Vec<u8>>> {
    if effects.background_mode == BackgroundMode::Blur {
        return Ok(None);
    }
    if effects.background_mode == BackgroundMode::Image {
        let rgba = image
            .context("Imagem de fundo ausente")?
            .resize_to_fill(w as u32, h as u32, image::imageops::FilterType::Triangle)
            .to_rgba8();
        let mut rgb = Vec::with_capacity(w * h * 3);
        for pixel in rgba.pixels() {
            let alpha = u32::from(pixel[3]);
            for c in 0..3 {
                rgb.push(
                    ((u32::from(pixel[c]) * alpha
                        + u32::from(effects.background_color[c]) * (255 - alpha)
                        + 127)
                        / 255) as u8,
                );
            }
        }
        return Ok(Some(rgb));
    }
    let mut rgb = vec![0; w * h * 3];
    for (i, pixel) in rgb.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        let x = (i % w) as f32 / w as f32;
        let y = (i / w) as f32 / h as f32;
        *pixel = match effects.background_mode {
            BackgroundMode::Solid => effects.background_color,
            BackgroundMode::Studio => {
                let light = (1.0 - ((x - 0.5).hypot(y - 0.4))).clamp(0.0, 1.0);
                [
                    24 + (light * 35.0) as u8,
                    32 + (light * 48.0) as u8,
                    50 + (light * 65.0) as u8,
                ]
            }
            BackgroundMode::Office => {
                if (0.08..0.35).contains(&x) && (0.1..0.6).contains(&y) {
                    if (x - 0.215).abs() < 0.008 || (y - 0.35).abs() < 0.008 {
                        [74, 88, 98]
                    } else {
                        [168, 205, 223]
                    }
                } else if y > 0.78 {
                    [82, 63, 49]
                } else if x > 0.76 && y > 0.25 && y < 0.7 && ((y * 9.0).fract() < 0.08) {
                    [100, 77, 59]
                } else {
                    [208, 202, 190]
                }
            }
            BackgroundMode::Landscape => {
                let ridge = 0.6 + (x * 9.0).sin() * 0.1;
                if y > ridge {
                    [35, (98.0 - y * 35.0) as u8, 67]
                } else if y > 0.5 + (x * 7.0 + 1.0).sin() * 0.12 {
                    [72, 113, 127]
                } else {
                    [135 + (y * 45.0) as u8, 180 + (y * 35.0) as u8, 220]
                }
            }
            _ => unreachable!("blur and image handled above"),
        };
    }
    Ok(Some(rgb))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scenes_are_opaque_different_and_cover_the_output() {
        let mut previous = Vec::new();
        for mode in [
            BackgroundMode::Solid,
            BackgroundMode::Studio,
            BackgroundMode::Office,
            BackgroundMode::Landscape,
        ] {
            let effects = Effects {
                background_mode: mode,
                ..Effects::default()
            };
            let scene = prepare(&effects, 32, 18, None).unwrap().unwrap();
            assert_eq!(scene.len(), 32 * 18 * 3);
            assert_ne!(scene, previous);
            if mode == BackgroundMode::Solid {
                assert!(
                    scene
                        .as_chunks::<3>()
                        .0
                        .iter()
                        .all(|c| c == &effects.background_color)
                );
            }
            previous = scene;
        }
    }
    #[test]
    fn custom_image_decoding_crop_and_failure_are_checked() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("background.png");
        image::RgbImage::from_pixel(8, 4, image::Rgb([10, 20, 30]))
            .save(&path)
            .unwrap();
        let img = load_image(path.to_str().unwrap()).unwrap();
        let effects = Effects {
            background_mode: BackgroundMode::Image,
            ..Effects::default()
        };
        let rgb = prepare(&effects, 3, 5, Some(img)).unwrap().unwrap();
        assert_eq!(rgb, [10, 20, 30].repeat(15));
        for extension in ["jpg", "webp"] {
            let path = dir.path().join(format!("background.{extension}"));
            image::RgbImage::from_pixel(8, 4, image::Rgb([10, 20, 30]))
                .save(&path)
                .unwrap();
            let decoded = load_image(path.to_str().unwrap()).unwrap().to_rgb8();
            assert!(decoded.pixels().all(|p| {
                p.0.iter()
                    .zip([10, 20, 30])
                    .all(|(a, b)| a.abs_diff(b) <= 2)
            }));
        }
        let transparent = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            1,
            1,
            image::Rgba([255, 0, 0, 0]),
        ));
        let rgb = prepare(&effects, 1, 1, Some(transparent)).unwrap().unwrap();
        assert_eq!(rgb, effects.background_color);
        let wide = dir.path().join("too-wide.png");
        image::RgbImage::new(8193, 1).save(&wide).unwrap();
        assert!(load_image(wide.to_str().unwrap()).is_err());
        std::fs::write(&path, b"invalid PNG").unwrap();
        assert!(load_image(path.to_str().unwrap()).is_err());
    }
}
