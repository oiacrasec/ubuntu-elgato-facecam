//! Eye-contact inference and local GPU blending, with no intermediate image download.
use crate::{
    framing::Rect,
    gaze::{self, Affine, Geometry},
    gpu::{Graph, Stage, integer, integers, string, take},
};
use anyhow::{Context, Result, bail};
use ort::{
    io_binding::IoBinding,
    memory::{AllocationDevice, AllocatorType, MemoryInfo, MemoryType},
    value::{DynValue, Tensor},
};
use std::path::Path;

pub struct Correction {
    landmark_input: Stage,
    landmarks: IoBinding,
    eye_input: Stage,
    encoder: IoBinding,
    decoder: IoBinding,
    validate: Stage,
    compose: Stage,
    size: [usize; 2],
    temporal: gaze::Temporal,
    face: Option<Rect>,
}

impl Correction {
    pub fn open(root: &Path, size: [usize; 2], fps: f64, device: i32) -> Result<Self> {
        let cpu = MemoryInfo::new(
            AllocationDevice::CPU,
            0,
            AllocatorType::Arena,
            MemoryType::Default,
        )?;
        let gpu = MemoryInfo::new(
            AllocationDevice::CUDA,
            device,
            AllocatorType::Device,
            MemoryType::Default,
        )?;
        let mut bindings = Vec::new();
        for model in &crate::nvidia_ar_model::MODELS[1..] {
            let session = crate::nvidia_ar::session(root, model, device)?;
            let mut binding = session.create_binding()?;
            for output in model.outputs {
                binding.bind_output_to_device(
                    output.name,
                    if model.id == "landmarks" { &cpu } else { &gpu },
                )?;
            }
            bindings.push(binding);
        }
        let mut bindings = bindings.into_iter();
        let mut correction = Self {
            landmark_input: Stage::open(
                &sample_model(size, [80, 80], true),
                &["image"],
                &[],
                device,
                "eye-landmarks",
            )?,
            landmarks: bindings
                .next()
                .context("Sessão de landmarks indisponível")?,
            eye_input: Stage::open(
                &sample_model(size, [256, 64], false),
                &["image"],
                &[],
                device,
                "eye-align",
            )?,
            encoder: bindings.next().context("Encoder de olhar indisponível")?,
            decoder: bindings.next().context("Decoder de olhar indisponível")?,
            validate: Stage::open(
                &validation_model(),
                &[],
                &["minimum", "maximum", "invalid"],
                device,
                "eye-validate",
            )?,
            compose: Stage::open(&compose_model(size), &["result"], &[], device, "eye-blend")?,
            size,
            temporal: gaze::Temporal::new(fps)?,
            face: None,
        };
        let black = Tensor::from_array(([1, 3, 64, 256], vec![0_f32; 3 * 64 * 256]))?.into_dyn();
        let decoded = correction.infer(&black, [0.0, 0.0])?;
        correction.check(&decoded)?;
        correction.reset();
        Ok(correction)
    }

    pub fn reset(&mut self) {
        self.temporal.reset();
        self.face = None;
    }

    pub(crate) fn tracked_face(&self) -> Option<Rect> {
        self.face
    }

    fn bind_affine(stage: &mut Stage, transform: Affine, size: [usize; 2]) -> Result<()> {
        for (name, value) in ["a", "b", "c", "d", "e", "f"]
            .into_iter()
            .zip(transform.normalized(size))
        {
            stage.binding.bind_input(
                name,
                &Tensor::from_array((Vec::<usize>::new(), vec![value]))?,
            )?;
        }
        Ok(())
    }

    fn locate(&mut self, source: &DynValue, face: Rect) -> Result<Option<Geometry>> {
        let crop = gaze::landmark_crop(face, self.size)?;
        Self::bind_affine(&mut self.landmark_input, crop, self.size)?;
        self.landmark_input.binding.bind_input("rgb", source)?;
        let image = take(&mut self.landmark_input.run()?, "image")?;
        self.landmarks.bind_input("input", &image)?;
        let outputs = self.landmarks.run()?;
        let values = |name| -> Result<&[f32]> {
            Ok(outputs
                .get(name)
                .context("Saída AR ausente")?
                .try_extract_raw_tensor::<f32>()?
                .1)
        };
        gaze::geometry(
            crop,
            values("kpts")?,
            values("confidence")?,
            values("headpose")?,
            self.size,
        )
    }

    fn infer(&mut self, image: &DynValue, target: [f32; 2]) -> Result<DynValue> {
        self.encoder.bind_input("input_image", image)?;
        let features = self.encoder.run()?;
        for name in ["embeddings_flat", "pseudo_labels_flat"] {
            self.decoder.bind_input(
                name,
                features.get(name).context("Saída do encoder AR ausente")?,
            )?;
        }
        self.decoder
            .bind_input("gaze_por", &Tensor::from_array(([1, 2], target.to_vec()))?)?;
        self.decoder
            .run()?
            .remove("gaze_redirected_image")
            .context("Saída AR ausente")
    }

    fn check(&mut self, image: &DynValue) -> Result<()> {
        self.validate.binding.bind_input("image", image)?;
        let outputs = self.validate.run()?;
        let scalar = |name| -> Result<f32> {
            outputs
                .get(name)
                .context("Diagnóstico GPU ausente")?
                .try_extract_raw_tensor::<f32>()?
                .1
                .first()
                .copied()
                .context("Diagnóstico GPU vazio")
        };
        let (min, max, invalid) = (scalar("minimum")?, scalar("maximum")?, scalar("invalid")?);
        if !min.is_finite() || !max.is_finite() || invalid != 0.0 || min < -1.0 || max > 2.0 {
            bail!(
                "Saída de correção de olhar inválida: mínimo {min}, máximo {max}, não finitos {invalid}"
            );
        }
        Ok(())
    }

    fn blend(
        &mut self,
        source: &DynValue,
        aligned: &DynValue,
        patch: &DynValue,
        geometry: &Geometry,
    ) -> Result<DynValue> {
        Self::bind_affine(&mut self.compose, geometry.to_patch, [256, 64])?;
        self.compose.binding.bind_input("rgb", source)?;
        self.compose.binding.bind_input("aligned", aligned)?;
        self.compose.binding.bind_input("patch", patch)?;
        for (i, ([x, y, rx, ry], visibility)) in
            geometry.eyes.iter().zip(geometry.visibility).enumerate()
        {
            let names = if i == 0 {
                ["ex0", "ey0", "rx0", "ry0", "gain0"]
            } else {
                ["ex1", "ey1", "rx1", "ry1", "gain1"]
            };
            for (name, value) in names.into_iter().zip([
                x / 128.0 - 1.0,
                y / 32.0 - 1.0,
                rx / 128.0,
                ry / 32.0,
                visibility * 0.85,
            ]) {
                self.compose.binding.bind_input(
                    name,
                    &Tensor::from_array((Vec::<usize>::new(), vec![value]))?,
                )?;
            }
        }
        take(&mut self.compose.run()?, "result")
    }

    pub fn correct(&mut self, source: &DynValue, faces: &[Rect]) -> Result<Option<DynValue>> {
        let Some(face) = self
            .face
            .and_then(|previous| {
                faces
                    .iter()
                    .copied()
                    .filter(|face| previous.overlap(*face) > 0.1)
                    .max_by(|a, b| previous.overlap(*a).total_cmp(&previous.overlap(*b)))
            })
            .or_else(|| {
                faces
                    .iter()
                    .copied()
                    .max_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height)))
            })
        else {
            self.reset();
            return Ok(None);
        };
        if self
            .face
            .is_some_and(|previous| previous.overlap(face) <= 0.1)
        {
            self.reset();
        }
        let Some(geometry) = self.locate(source, face)? else {
            self.reset();
            return Ok(None);
        };
        self.face = Some(face);
        let geometry = self.temporal.update(geometry)?;
        if geometry.visibility.iter().all(|v| *v == 0.0) {
            return Ok(None);
        }
        Self::bind_affine(&mut self.eye_input, geometry.to_source, self.size)?;
        self.eye_input.binding.bind_input("rgb", source)?;
        let eyes = take(&mut self.eye_input.run()?, "image")?;
        let decoded = self.infer(&eyes, [0.0, 0.0])?;
        self.check(&decoded)?;
        Ok(Some(self.blend(source, &eyes, &decoded, &geometry)?))
    }
}

fn grid(g: &mut Graph, [w, h]: [usize; 2]) {
    for name in ["a", "b", "c", "d", "e", "f"] {
        g.value(name, 1, &[], true);
    }
    g.constant(
        "x",
        1,
        &[1, 1, w as u64, 1],
        &(0..w)
            .flat_map(|x| (x as f32 + 0.5).to_le_bytes())
            .collect::<Vec<_>>(),
    );
    g.constant(
        "y",
        1,
        &[1, h as u64, 1, 1],
        &(0..h)
            .flat_map(|y| (y as f32 + 0.5).to_le_bytes())
            .collect::<Vec<_>>(),
    );
    for (out, a, b, c) in [("sx", "a", "b", "c"), ("sy", "d", "e", "f")] {
        g.node("Mul", &["x", a], &format!("{out}_x"), &[]);
        g.node("Mul", &["y", b], &format!("{out}_y"), &[]);
        g.node(
            "Add",
            &[&format!("{out}_x"), &format!("{out}_y")],
            &format!("{out}_xy"),
            &[],
        );
        g.node("Add", &[&format!("{out}_xy"), c], out, &[]);
    }
    g.node("Concat", &["sx", "sy"], "grid", &[integer("axis", 3)]);
}

fn sample_model([w, h]: [usize; 2], size: [usize; 2], gray: bool) -> Vec<u8> {
    let mut g = Graph::default();
    g.value("rgb", 1, &[1, 3, h as i64, w as i64], true);
    grid(&mut g, size);
    g.node(
        "GridSample",
        &["rgb", "grid"],
        "sample",
        &[
            integer("align_corners", 0),
            string("mode", "bilinear"),
            string("padding_mode", "border"),
        ],
    );
    if gray {
        g.constant(
            "luma",
            1,
            &[1, 3, 1, 1],
            &[0.299_f32, 0.587, 0.114]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        g.node("Mul", &["sample", "luma"], "weighted", &[]);
        g.ints("axis", &[1]);
        g.node(
            "ReduceSum",
            &["weighted", "axis"],
            "image",
            &[integer("keepdims", 1)],
        );
    } else {
        g.scalar("scale", 255.0);
        g.node("Div", &["sample", "scale"], "image", &[]);
    }
    g.value(
        "image",
        1,
        &[1, if gray { 1 } else { 3 }, size[1] as i64, size[0] as i64],
        false,
    );
    g.finish()
}

fn validation_model() -> Vec<u8> {
    let mut g = Graph::default();
    g.value("image", 1, &[1, 3, 64, 256], true);
    for (op, name) in [("ReduceMin", "minimum"), ("ReduceMax", "maximum")] {
        g.node(op, &["image"], name, &[integer("keepdims", 0)]);
        g.value(name, 1, &[], false);
    }
    g.node("IsNaN", &["image"], "nan", &[]);
    g.node("IsInf", &["image"], "inf", &[]);
    g.node("Or", &["nan", "inf"], "bad", &[]);
    g.node("Cast", &["bad"], "float_bad", &[integer("to", 1)]);
    g.node(
        "ReduceSum",
        &["float_bad"],
        "invalid",
        &[integer("keepdims", 0)],
    );
    g.value("invalid", 1, &[], false);
    g.finish()
}

fn compose_model(size: [usize; 2]) -> Vec<u8> {
    let [w, h] = size;
    let mut g = Graph::default();
    g.value("rgb", 1, &[1, 3, h as i64, w as i64], true);
    g.value("patch", 1, &[1, 3, 64, 256], true);
    g.value("aligned", 1, &[1, 3, 64, 256], true);
    grid(&mut g, size);
    g.scalar("zero", 0.0);
    g.scalar("one", 1.0);
    g.scalar("two", 2.0);
    g.scalar("three", 3.0);
    g.scalar("feather", 0.55);
    g.scalar("scale", 255.0);
    g.node("Clip", &["patch", "zero", "one"], "bounded", &[]);
    // Warp only the model's change. Re-warping the original eye texture would
    // blur/shimmer even for a decoder that returns its input unchanged.
    g.node("Sub", &["bounded", "aligned"], "delta", &[]);
    g.node(
        "GridSample",
        &["delta", "grid"],
        "warped",
        &[
            integer("align_corners", 0),
            string("mode", "bilinear"),
            string("padding_mode", "border"),
        ],
    );
    for (i, [ex, ey, rx, ry, gain]) in [
        ["ex0", "ey0", "rx0", "ry0", "gain0"],
        ["ex1", "ey1", "rx1", "ry1", "gain1"],
    ]
    .into_iter()
    .enumerate()
    {
        for name in [ex, ey, rx, ry, gain] {
            g.value(name, 1, &[], true);
        }
        let n = |s: &str| format!("{s}{i}");
        g.node("Sub", &["sx", ex], &n("dx"), &[]);
        g.node("Div", &[&n("dx"), rx], &n("nx"), &[]);
        g.node("Sub", &["sy", ey], &n("dy"), &[]);
        g.node("Div", &[&n("dy"), ry], &n("ny"), &[]);
        for axis in ["nx", "ny"] {
            g.node("Mul", &[&n(axis), &n(axis)], &n(&format!("{axis}2")), &[]);
        }
        g.node("Add", &[&n("nx2"), &n("ny2")], &n("radius"), &[]);
        g.node("Sub", &["one", &n("radius")], &n("edge"), &[]);
        g.node("Div", &[&n("edge"), "feather"], &n("ramp"), &[]);
        g.node("Clip", &[&n("ramp"), "zero", "one"], &n("t"), &[]);
        g.node("Mul", &[&n("t"), &n("t")], &n("square"), &[]);
        g.node("Mul", &[&n("t"), "two"], &n("twice"), &[]);
        g.node("Sub", &["three", &n("twice")], &n("curve"), &[]);
        g.node("Mul", &[&n("square"), &n("curve")], &n("smooth"), &[]);
        g.node("Mul", &[&n("smooth"), gain], &n("mask"), &[]);
    }
    g.node("Max", &["mask0", "mask1"], "mask_hwc", &[]);
    g.node(
        "Transpose",
        &["mask_hwc"],
        "mask",
        &[integers("perm", &[0, 3, 1, 2])],
    );
    g.node("Mul", &["warped", "scale"], "difference", &[]);
    g.node("Mul", &["difference", "mask"], "correction", &[]);
    g.node("Add", &["rgb", "correction"], "unbounded_result", &[]);
    g.node(
        "Clip",
        &["unbounded_result", "zero", "scale"],
        "result",
        &[],
    );
    g.value("result", 1, &[1, 3, h as i64, w as i64], false);
    g.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn download(value: &DynValue, shape: &[i64], device: i32) -> Vec<f32> {
        let mut g = Graph::default();
        g.value("input", 1, shape, true);
        g.node("Identity", &["input"], "output", &[]);
        g.value("output", 1, shape, false);
        let mut stage = Stage::open(&g.finish(), &[], &["output"], device, "gaze-oracle").unwrap();
        stage.binding.bind_input("input", value).unwrap();
        stage.run().unwrap()["output"]
            .try_extract_raw_tensor::<f32>()
            .unwrap()
            .1
            .to_vec()
    }
    fn pixels(values: &[f32], w: u32, h: u32) -> image::RgbImage {
        image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb(std::array::from_fn(|c| {
                (values[c * (w * h) as usize + (y * w + x) as usize] * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8
            }))
        })
    }
    #[test]
    #[ignore = "requires prepared Ada, CUDA paths and the public astronaut fixture"]
    fn public_face_runs_aligned_encoder_decoder_and_local_blend() {
        let root = crate::nvidia_ar_setup::root().unwrap();
        let install = crate::nvidia_ar_setup::Installation::load(&root).unwrap();
        ort::init_from(install.runtime().unwrap().runtime.to_string_lossy())
            .commit()
            .unwrap();
        let device = crate::nvidia::ada_device().unwrap();
        let image_path = std::env::var_os("FACECAM_AR_TEST_IMAGE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "target/ar-face-contract/astronaut.png".into());
        assert_eq!(
            crate::nvidia_setup::checksum(&image_path).unwrap(),
            "88431cd9653ccd539741b555fb0a46b61558b301d4110412b5bc28b5e3ea6cb5"
        );
        let input = crate::background::load_image(image_path.to_str().unwrap())
            .unwrap()
            .to_rgb8();
        let size = [input.width() as usize, input.height() as usize];
        let mut ar =
            crate::nvidia_framing::Framing::open(&root, size, 30.0, device, false).unwrap();
        let source = ar.upload(input.as_raw()).unwrap();
        let faces = ar.detect(&source).unwrap();
        assert!(!faces.is_empty());
        let mut correction = Correction::open(&root, size, 30.0, device).unwrap();
        let geometry = correction
            .locate(&source, faces[0])
            .unwrap()
            .expect("valid open eyes in public fixture");
        eprintln!("geometry={geometry:?}");
        Correction::bind_affine(&mut correction.eye_input, geometry.to_source, size).unwrap();
        correction
            .eye_input
            .binding
            .bind_input("rgb", &source)
            .unwrap();
        let eyes = take(&mut correction.eye_input.run().unwrap(), "image").unwrap();
        let original = download(&eyes, &[1, 3, 64, 256], device);
        let identity = correction.blend(&source, &eyes, &eyes, &geometry).unwrap();
        let unchanged = download(&identity, &[1, 3, size[1] as i64, size[0] as i64], device);
        let original_nchw: Vec<f32> = (0..3)
            .flat_map(|c| input.pixels().map(move |p| p[c] as f32))
            .collect();
        assert_eq!(
            unchanged, original_nchw,
            "identity decoder must not resample source texture"
        );
        let a = correction.infer(&eyes, [0.0, 0.0]).unwrap();
        correction.check(&a).unwrap();
        let frontal = download(&a, &[1, 3, 64, 256], device);
        let b = correction.infer(&eyes, [0.0, 0.3]).unwrap();
        correction.check(&b).unwrap();
        let turned = download(&b, &[1, 3, 64, 256], device);
        let delta = frontal
            .iter()
            .zip(&turned)
            .map(|(a, b)| (a - b).abs())
            .sum::<f32>()
            / frontal.len() as f32;
        assert!(delta > 0.001, "target gaze must change the decoded eyes");
        std::fs::create_dir_all("target/ar-gaze-contract").unwrap();
        let mut comparison = image::RgbImage::new(256, 64 * 3);
        for (row, values) in [original, frontal, turned].iter().enumerate() {
            image::imageops::replace(
                &mut comparison,
                &pixels(values, 256, 64),
                0,
                row as i64 * 64,
            );
        }
        image::imageops::resize(&comparison, 1024, 768, image::imageops::FilterType::Nearest)
            .save("target/ar-gaze-contract/eye-strip-comparison.png")
            .unwrap();
        let mut corrected = None;
        for _ in 0..12 {
            corrected = correction.correct(&source, &faces).unwrap();
        }
        let output = corrected.expect("real correction output");
        let blended = download(&output, &[1, 3, size[1] as i64, size[0] as i64], device);
        let normalized = blended.iter().map(|v| v / 255.0).collect::<Vec<_>>();
        pixels(&normalized, input.width(), input.height())
            .save("target/ar-gaze-contract/corrected-public-face.png")
            .unwrap();
        let changed = blended
            .iter()
            .zip(&original_nchw)
            .filter(|(a, b)| (*a - *b).abs() > 0.5)
            .count();
        assert!(
            changed > 20 && changed < 30_000,
            "only eyes should change: {changed}"
        );
        for y in 0..size[1] {
            for x in 0..size[0] {
                let p = geometry.to_patch.map([x as f32 + 0.5, y as f32 + 0.5]);
                let outside = geometry.eyes.iter().all(|[cx, cy, rx, ry]| {
                    ((p[0] - cx) / rx).powi(2) + ((p[1] - cy) / ry).powi(2) > 1.001
                });
                if outside {
                    for c in 0..3 {
                        let i = (c * size[1] + y) * size[0] + x;
                        assert_eq!(blended[i], original_nchw[i], "pixel outside eye at {x},{y}");
                    }
                }
            }
        }
        for value in [f32::NAN, f32::INFINITY, 1000.0] {
            let bad = Tensor::from_array(([1, 3, 64, 256], vec![value; 3 * 64 * 256]))
                .unwrap()
                .into_dyn();
            assert!(correction.check(&bad).is_err());
        }
        assert!(correction.correct(&source, &[]).unwrap().is_none());
        eprintln!("decoded_target_difference={delta:.5}, changed_channels={changed}");
    }

    #[test]
    #[ignore = "requires prepared Ada, CUDA paths and the public astronaut fixture"]
    fn public_face_small_photometric_changes_keep_detection() {
        let root = crate::nvidia_ar_setup::root().unwrap();
        let install = crate::nvidia_ar_setup::Installation::load(&root).unwrap();
        ort::init_from(install.runtime().unwrap().runtime.to_string_lossy())
            .commit()
            .unwrap();
        let device = crate::nvidia::ada_device().unwrap();
        let path = std::env::var_os("FACECAM_AR_TEST_IMAGE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "target/ar-face-contract/astronaut.png".into());
        assert_eq!(
            crate::nvidia_setup::checksum(&path).unwrap(),
            "88431cd9653ccd539741b555fb0a46b61558b301d4110412b5bc28b5e3ea6cb5"
        );
        let input = crate::background::load_image(path.to_str().unwrap())
            .unwrap()
            .to_rgb8();
        let size = [input.width() as usize, input.height() as usize];
        let mut ar = crate::nvidia_framing::Framing::open(&root, size, 30.0, device, true).unwrap();
        for i in 0..6 {
            let image = image::RgbImage::from_fn(input.width(), input.height(), |x, y| {
                let p = input.get_pixel(x, y);
                image::Rgb(std::array::from_fn(|c| {
                    if i == 0 {
                        p[c]
                    } else {
                        (i32::from(p[c])
                            + if i % 2 == 0 { -1 } else { 1 }
                            + ((x * 13 + y * 7 + c as u32 + i) % 3) as i32
                            - 1)
                        .clamp(0, 255) as u8
                    }
                }))
            });
            let output = ar.frame(image.as_raw()).unwrap();
            let output = download(&output, &[1, 3, size[1] as i64, size[0] as i64], device);
            let original: Vec<f32> = (0..3)
                .flat_map(|c| image.pixels().map(move |p| p[c] as f32))
                .collect();
            let changed = output
                .iter()
                .zip(original)
                .filter(|(a, b)| (*a - *b).abs() > 0.5)
                .count();
            eprintln!("phase={i}, changed_channels={changed}");
            assert!(
                changed > 20 && changed < 30_000,
                "correction dropped out in phase {i}: {changed}"
            );
        }
        // Losing the face must discard the lower continuation threshold.
        ar.frame(&vec![0; size[0] * size[1] * 3]).unwrap();
        let weak = image::RgbImage::from_fn(input.width(), input.height(), |x, y| {
            let p = input.get_pixel(x, y);
            image::Rgb(std::array::from_fn(|c| {
                (i32::from(p[c]) + 1 + ((x * 13 + y * 7 + c as u32 + 1) % 3) as i32 - 1)
                    .clamp(0, 255) as u8
            }))
        });
        let output = ar.frame(weak.as_raw()).unwrap();
        let output = download(&output, &[1, 3, size[1] as i64, size[0] as i64], device);
        let original: Vec<f32> = (0..3)
            .flat_map(|c| weak.pixels().map(move |p| p[c] as f32))
            .collect();
        assert_eq!(
            output, original,
            "weak detection must not reacquire after face loss"
        );
    }

    #[test]
    #[ignore = "requires prepared Ada and CUDA paths; synthetic pixels only"]
    fn gpu_eye_warp_matches_scalar_bilinear_reference() {
        let root = crate::nvidia_ar_setup::root().unwrap();
        let install = crate::nvidia_ar_setup::Installation::load(&root).unwrap();
        ort::init_from(install.runtime().unwrap().runtime.to_string_lossy())
            .commit()
            .unwrap();
        let device = crate::nvidia::ada_device().unwrap();
        let [w, h] = [37, 23];
        let frame: Vec<u8> = (0..w * h * 3).map(|i| (i * 31 % 256) as u8).collect();
        let mut ar =
            crate::nvidia_framing::Framing::open(&root, [w, h], 30.0, device, false).unwrap();
        let source = ar.upload(&frame).unwrap();
        for gray in [false, true] {
            for transform in [
                Affine([0.6, -0.1, 4.0, 0.1, 0.6, 3.0]),
                Affine([1.0, 0.0, -4.0, 0.0, 1.0, -2.0]),
            ] {
                let mut stage = Stage::open(
                    &sample_model([w, h], [19, 13], gray),
                    &["image"],
                    &[],
                    device,
                    "eye-warp-oracle",
                )
                .unwrap();
                Correction::bind_affine(&mut stage, transform, [w, h]).unwrap();
                stage.binding.bind_input("rgb", &source).unwrap();
                let result = take(&mut stage.run().unwrap(), "image").unwrap();
                let channels = if gray { 1 } else { 3 };
                let actual = download(&result, &[1, channels, 13, 19], device);
                for y in 0..13 {
                    for x in 0..19 {
                        let p = transform.map([x as f32 + 0.5, y as f32 + 0.5]);
                        let sx = (p[0] - 0.5).clamp(0.0, (w - 1) as f32);
                        let sy = (p[1] - 0.5).clamp(0.0, (h - 1) as f32);
                        let x0 = sx.floor() as usize;
                        let y0 = sy.floor() as usize;
                        let x1 = (x0 + 1).min(w - 1);
                        let y1 = (y0 + 1).min(h - 1);
                        let fx = sx - x0 as f32;
                        let fy = sy - y0 as f32;
                        let pixel = |c| {
                            let p = |x, y| frame[(y * w + x) * 3 + c] as f32;
                            (p(x0, y0) * (1.0 - fx) + p(x1, y0) * fx) * (1.0 - fy)
                                + (p(x0, y1) * (1.0 - fx) + p(x1, y1) * fx) * fy
                        };
                        if gray {
                            let expected = pixel(0) * 0.299 + pixel(1) * 0.587 + pixel(2) * 0.114;
                            assert!((actual[y * 19 + x] - expected).abs() < 0.001);
                        } else {
                            for c in 0..3 {
                                assert!(
                                    (actual[(c * 13 + y) * 19 + x] - pixel(c) / 255.0).abs()
                                        < 0.00001
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
