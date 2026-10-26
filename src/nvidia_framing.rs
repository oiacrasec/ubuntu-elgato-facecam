//! GPU image upload and internal face detection for eye-contact correction.
use crate::{
    framing::{HEIGHT, Letterbox, Rect, WIDTH},
    gpu::{Graph, Stage, integer, integers, take},
};
use anyhow::{Context, Result, bail};
use ort::{
    io_binding::IoBinding,
    memory::{AllocationDevice, AllocatorType, MemoryInfo, MemoryType},
    value::{DynValue, Tensor},
};
use std::path::Path;

pub struct Framing {
    upload: Stage,
    prepare: Stage,
    pack: Stage,
    detector: IoBinding,
    host: Tensor<u8>,
    letterbox: Letterbox,
    correction: Option<crate::nvidia_gaze::Correction>,
}

impl Framing {
    pub fn open(
        root: &Path,
        size: [usize; 2],
        fps: f64,
        device: i32,
        eye_contact: bool,
    ) -> Result<Self> {
        let letterbox = Letterbox::new(size)?;
        let installation = crate::nvidia_ar_setup::Installation::load(root)?;
        installation.verify_engines(root)?;
        let session = crate::nvidia_ar::session(root, &crate::nvidia_ar_model::MODELS[0], device)?;
        let mut detector = session.create_binding()?;
        let cpu = MemoryInfo::new(
            AllocationDevice::CPU,
            0,
            AllocatorType::Arena,
            MemoryType::Default,
        )?;
        for name in ["output_bbox", "output_cov/Sigmoid"] {
            detector.bind_output_to_device(name, &cpu)?;
        }
        let [w, h] = size;
        Ok(Self {
            upload: Stage::open(
                &crate::gpu::upload_model(size),
                &["rgb"],
                &[],
                device,
                "ar-upload",
            )?,
            prepare: Stage::open(
                &prepare_model(size, letterbox),
                &["image"],
                &[],
                device,
                "ar-prepare",
            )?,
            pack: Stage::open(&pack_model(size), &[], &["output"], device, "ar-output")?,
            detector,
            host: Tensor::from_array(([1, h, w, 3], vec![0_u8; w * h * 3]))?,
            letterbox,
            correction: if eye_contact {
                Some(crate::nvidia_gaze::Correction::open(
                    root, size, fps, device,
                )?)
            } else {
                None
            },
        })
    }

    pub(crate) fn upload(&mut self, rgb: &[u8]) -> Result<DynValue> {
        let (_, host) = self.host.try_extract_raw_tensor_mut::<u8>()?;
        if host.len() != rgb.len() {
            bail!("Dimensão da imagem GPU incompatível");
        }
        host.copy_from_slice(rgb);
        self.upload.binding.bind_input("bytes", &self.host)?;
        take(&mut self.upload.run()?, "rgb")
    }

    pub(crate) fn detect(&mut self, source: &DynValue) -> Result<Vec<Rect>> {
        self.prepare.binding.bind_input("rgb", source)?;
        let image = take(&mut self.prepare.run()?, "image")?;
        self.detector.bind_input("input", &image)?;
        let outputs = self.detector.run()?;
        let (_, boxes) = outputs
            .get("output_bbox")
            .context("Saída AR ausente")?
            .try_extract_raw_tensor::<f32>()?;
        let (_, confidence) = outputs
            .get("output_cov/Sigmoid")
            .context("Saída AR ausente")?
            .try_extract_raw_tensor::<f32>()?;
        self.letterbox.decode(boxes, confidence)
    }

    pub fn frame(&mut self, rgb: &[u8]) -> Result<DynValue> {
        let source = self.upload(rgb)?;
        let faces = self.detect(&source)?;
        if let Some(correction) = self.correction.as_mut()
            && let Some(corrected) = correction.correct(&source, &faces)?
        {
            return Ok(corrected);
        }
        Ok(source)
    }

    pub fn pack(&mut self, rgb: &DynValue) -> Result<DynValue> {
        self.pack.binding.bind_input("rgb", rgb)?;
        take(&mut self.pack.run()?, "output")
    }
    pub fn reset(&mut self) {
        if let Some(correction) = self.correction.as_mut() {
            correction.reset();
        }
    }
}

fn prepare_model([w, h]: [usize; 2], letterbox: Letterbox) -> Vec<u8> {
    let mut g = Graph::default();
    g.value("rgb", 1, &[1, 3, h as i64, w as i64], true);
    let [rw, rh] = letterbox.size;
    let [ox, oy] = letterbox.offset;
    g.ints("target", &[1, 3, rh as i64, rw as i64]);
    g.resize("rgb", "resized", "target");
    g.scalar("half", 0.5);
    g.scalar("divisor", 255.0);
    g.node("Add", &["resized", "half"], "rounded", &[]);
    g.node("Floor", &["rounded"], "pixels", &[]);
    g.ints(
        "pads",
        &[
            0,
            0,
            oy as i64,
            ox as i64,
            0,
            0,
            (HEIGHT - rh - oy) as i64,
            (WIDTH - rw - ox) as i64,
        ],
    );
    g.node("Pad", &["pixels", "pads"], "padded", &[]);
    g.ints("channels", &[2, 1, 0]);
    g.node(
        "Gather",
        &["padded", "channels"],
        "bgr",
        &[integer("axis", 1)],
    );
    g.node("Div", &["bgr", "divisor"], "image", &[]);
    g.value("image", 1, &[1, 3, HEIGHT as i64, WIDTH as i64], false);
    g.finish()
}

fn pack_model([w, h]: [usize; 2]) -> Vec<u8> {
    let mut g = Graph::default();
    g.value("rgb", 1, &[1, 3, h as i64, w as i64], true);
    g.scalar("half", 0.5);
    g.node("Add", &["rgb", "half"], "rounded", &[]);
    g.node("Cast", &["rounded"], "bytes", &[integer("to", 2)]);
    g.node(
        "Transpose",
        &["bytes"],
        "output",
        &[integers("perm", &[0, 2, 3, 1])],
    );
    g.value("output", 2, &[1, h as i64, w as i64, 3], false);
    g.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn init() {
        let install =
            crate::nvidia_ar_setup::Installation::load(&crate::nvidia_ar_setup::root().unwrap())
                .unwrap();
        ort::init_from(install.runtime().unwrap().runtime.to_string_lossy())
            .commit()
            .unwrap();
    }
    fn download(value: &DynValue, shape: &[i64], device: i32) -> Vec<f32> {
        let mut g = Graph::default();
        g.value("input", 1, shape, true);
        g.node("Identity", &["input"], "output", &[]);
        g.value("output", 1, shape, false);
        let mut stage = Stage::open(&g.finish(), &[], &["output"], device, "ar-oracle").unwrap();
        stage.binding.bind_input("input", value).unwrap();
        stage.run().unwrap()["output"]
            .try_extract_raw_tensor::<f32>()
            .unwrap()
            .1
            .to_vec()
    }
    #[test]
    #[ignore = "requires prepared AR Ada and its CUDA library paths; synthetic pixels only"]
    fn gpu_face_input_matches_cpu_oracle_and_no_face_preserves_the_frame() {
        init();
        let device = crate::nvidia::ada_device().unwrap();
        for (w, h) in [(37, 23), (23, 37), (64, 64)] {
            let mut ar = Framing::open(
                &crate::nvidia_ar_setup::root().unwrap(),
                [w, h],
                30.0,
                device,
                false,
            )
            .unwrap();
            let frame: Vec<u8> = (0..w * h * 3).map(|i| (i * 31 % 256) as u8).collect();
            let source = ar.upload(&frame).unwrap();
            ar.prepare.binding.bind_input("rgb", &source).unwrap();
            let image = take(&mut ar.prepare.run().unwrap(), "image").unwrap();
            let values = download(&image, &[1, 3, HEIGHT as i64, WIDTH as i64], device);
            let letterbox = Letterbox::new([w, h]).unwrap();
            let [rw, rh] = letterbox.size;
            let [ox, oy] = letterbox.offset;
            let resized = crate::ai::resize_rgb_to(&frame, w, h, rw, rh);
            for c in 0..3 {
                for y in 0..HEIGHT {
                    for x in 0..WIDTH {
                        let expected = if x >= ox && x < ox + rw && y >= oy && y < oy + rh {
                            resized[((y - oy) * rw + x - ox) * 3 + 2 - c] as f32 / 255.0
                        } else {
                            0.0
                        };
                        assert!(
                            (values[(c * HEIGHT + y) * WIDTH + x] - expected).abs()
                                <= 1.001 / 255.0
                        );
                    }
                }
            }
            let black = vec![0_u8; w * h * 3];
            let source = ar.upload(&black).unwrap();
            assert!(ar.detect(&source).unwrap().is_empty());
            let result = ar.frame(&black).unwrap();
            assert!(
                ar.pack(&result)
                    .unwrap()
                    .try_extract_raw_tensor::<u8>()
                    .unwrap()
                    .1
                    .iter()
                    .all(|v| *v == 0)
            );
        }
    }
}
