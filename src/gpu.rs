//! CUDA image processing, using owned ONNX graphs and safe ORT I/O binding.
//! Host RGB enters once; intermediate images, masks and composition stay on-device.
use crate::nvidia_model::{bytes, number, text};
use anyhow::{Context, Result, bail};
use ort::{
    execution_providers::CUDAExecutionProvider,
    io_binding::IoBinding,
    memory::{AllocationDevice, AllocatorType, MemoryInfo, MemoryType},
    session::Session,
    value::{DynValue, Tensor},
};
use std::collections::BTreeMap;

pub struct Prepared {
    pub source: DynValue,
    pub small: DynValue,
    pub image: DynValue,
}

pub struct Contract {
    pub size: [usize; 2],
    pub mask: [usize; 2],
    pub mask_shape: Vec<i64>,
    pub native: bool,
    pub device: i32,
}

pub struct Processing {
    prepare: Stage,
    upload: Option<Stage>,
    compose: Stage,
    host: Tensor<u8>,
    background: Option<DynValue>,
    dynamic_background: bool,
    tolerance: f32,
}

impl Processing {
    pub fn open(
        contract: Contract,
        effects: &crate::effects::Effects,
        background: Option<&[u8]>,
    ) -> Result<Self> {
        let Contract {
            size,
            mask,
            mask_shape,
            native,
            device,
        } = contract;
        let dynamic_background = effects.background_mode == crate::effects::BackgroundMode::Video;
        let [w, h] = size;
        let mut result = Self {
            prepare: Stage::open(
                &prepare_model(size, mask, native, effects.eye_contact),
                &["rgb", "small", "image"],
                &[],
                device,
                "prepare",
            )?,
            upload: if background.is_some() || dynamic_background {
                Some(Stage::open(
                    &upload_model(size),
                    &["rgb"],
                    &[],
                    device,
                    "background",
                )?)
            } else {
                None
            },
            compose: Stage::open(
                &compose_model(
                    size,
                    mask,
                    &mask_shape,
                    native,
                    effects.blur_radius,
                    background.is_none() && !dynamic_background,
                ),
                &[],
                &["output", "minimum", "maximum", "invalid"],
                device,
                "compose",
            )?,
            host: Tensor::from_array(([1, h, w, 3], vec![0_u8; w * h * 3]))?,
            background: None,
            dynamic_background,
            tolerance: if native { 0.0001 } else { 0.0 },
        };
        if let Some(rgb) = background {
            result.upload_background(rgb)?;
        }
        Ok(result)
    }

    fn upload_background(&mut self, rgb: &[u8]) -> Result<()> {
        let (_, host) = self.host.try_extract_raw_tensor_mut::<u8>()?;
        if host.len() != rgb.len() {
            bail!("Dimensão do fundo GPU incompatível");
        }
        host.copy_from_slice(rgb);
        let upload = self
            .upload
            .as_mut()
            .context("Upload de fundo indisponível")?;
        upload.binding.bind_input("bytes", &self.host)?;
        self.background = Some(take(&mut upload.run()?, "rgb")?);
        Ok(())
    }

    pub fn prepare(&mut self, rgb: &[u8]) -> Result<Prepared> {
        let (_, host) = self.host.try_extract_raw_tensor_mut::<u8>()?;
        if host.len() != rgb.len() {
            bail!("Dimensão da imagem GPU incompatível");
        }
        host.copy_from_slice(rgb);
        self.prepare.binding.bind_input("bytes", &self.host)?;
        self.prepared()
    }

    pub fn prepare_source(&mut self, source: &DynValue) -> Result<Prepared> {
        self.prepare.binding.bind_input("rgb", source)?;
        self.prepared()
    }

    fn prepared(&mut self) -> Result<Prepared> {
        let mut outputs = self.prepare.run()?;
        Ok(Prepared {
            source: take(&mut outputs, "rgb")?,
            small: take(&mut outputs, "small")?,
            image: take(&mut outputs, "image")?,
        })
    }

    pub fn compose(
        &mut self,
        prepared: &Prepared,
        mask: &DynValue,
        background: Option<&[u8]>,
    ) -> Result<DynValue> {
        if self.dynamic_background {
            self.upload_background(background.context("Frame do vídeo de fundo ausente")?)?;
        }
        self.compose.binding.bind_input("rgb", &prepared.source)?;
        self.compose.binding.bind_input("mask", mask)?;
        if let Some(bg) = &self.background {
            self.compose.binding.bind_input("background", bg)?;
        } else {
            self.compose.binding.bind_input("small", &prepared.small)?;
        }
        let mut outputs = self.compose.run()?;
        let scalar = |key| -> Result<f32> {
            let (_, v) = outputs
                .get(key)
                .context("Diagnóstico GPU ausente")?
                .try_extract_raw_tensor::<f32>()?;
            v.first().copied().context("Diagnóstico GPU vazio")
        };
        let (min, max, invalid) = (scalar("minimum")?, scalar("maximum")?, scalar("invalid")?);
        if !min.is_finite()
            || !max.is_finite()
            || invalid != 0.0
            || min < -self.tolerance
            || max > 1.0 + self.tolerance
        {
            bail!("Saída de segmentação GPU inválida: probabilidades fora de [0,1] ou não finitas");
        }
        take(&mut outputs, "output")
    }
}

pub(crate) fn take(
    outputs: &mut BTreeMap<&'static str, DynValue>,
    name: &'static str,
) -> Result<DynValue> {
    outputs
        .remove(name)
        .with_context(|| format!("Saída GPU ausente: {name}"))
}

pub(crate) struct Stage {
    pub(crate) binding: IoBinding,
    session: Session,
    gpu: MemoryInfo,
    cpu: MemoryInfo,
    gpu_outputs: Vec<&'static str>,
    cpu_outputs: Vec<&'static str>,
    profiling: bool,
}
impl Stage {
    pub(crate) fn open(
        model: &[u8],
        gpu_outputs: &[&'static str],
        cpu_outputs: &[&'static str],
        device: i32,
        name: &str,
    ) -> Result<Self> {
        let mut builder = Session::builder()?
            .with_intra_threads(1)?
            .with_config_entry("session.disable_cpu_ep_fallback", "1")?
            .with_execution_providers([CUDAExecutionProvider::default()
                .with_device_id(device)
                .build()
                .error_on_failure()])?;
        let profile = std::env::var_os("FACECAM_GPU_PROFILE_DIR");
        if let Some(dir) = &profile {
            std::fs::create_dir_all(dir)?;
            builder = builder.with_profiling(std::path::PathBuf::from(dir).join(name))?;
        }
        let session = builder
            .commit_from_memory(model)
            .with_context(|| format!("Não foi possível preparar {name} na GPU"))?;
        Ok(Self {
            binding: session.create_binding()?,
            session,
            gpu: MemoryInfo::new(
                AllocationDevice::CUDA,
                device,
                AllocatorType::Device,
                MemoryType::Default,
            )?,
            cpu: MemoryInfo::new(
                AllocationDevice::CPU,
                0,
                AllocatorType::Arena,
                MemoryType::Default,
            )?,
            gpu_outputs: gpu_outputs.to_vec(),
            cpu_outputs: cpu_outputs.to_vec(),
            profiling: profile.is_some(),
        })
    }
    pub(crate) fn run(&mut self) -> Result<BTreeMap<&'static str, DynValue>> {
        // Each result can remain bound as an input to another session. Rebind to
        // device memory instead of overwriting an earlier frame/background.
        self.binding.clear_outputs();
        for &name in &self.gpu_outputs {
            self.binding.bind_output_to_device(name, &self.gpu)?;
        }
        for &name in &self.cpu_outputs {
            self.binding.bind_output_to_device(name, &self.cpu)?;
        }
        let mut output = self.binding.run()?;
        self.gpu_outputs
            .iter()
            .chain(&self.cpu_outputs)
            .map(|&n| {
                Ok((
                    n,
                    output
                        .remove(n)
                        .with_context(|| format!("Saída GPU ausente: {n}"))?,
                ))
            })
            .collect()
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        if self.profiling {
            let _ = self.session.end_profiling();
        }
    }
}

// Small, fixed-shape graphs owned by this application. No model weights,
// downloaded code or Python runtime are used to build these descriptors.
#[derive(Default)]
pub(crate) struct Graph(Vec<u8>);
pub(crate) fn integer(name: &str, value: u64) -> Vec<u8> {
    bytes(
        5,
        &[text(1, name), number(3, value), number(20, 2)].concat(),
    )
}
pub(crate) fn integers(name: &str, values: &[u64]) -> Vec<u8> {
    bytes(
        5,
        &[
            text(1, name),
            values.iter().flat_map(|&n| number(8, n)).collect(),
            number(20, 7),
        ]
        .concat(),
    )
}
pub(crate) fn string(name: &str, value: &str) -> Vec<u8> {
    bytes(5, &[text(1, name), text(4, value), number(20, 3)].concat())
}
impl Graph {
    pub(crate) fn node(&mut self, op: &str, inputs: &[&str], output: &str, attrs: &[Vec<u8>]) {
        let mut n = inputs.iter().flat_map(|s| text(1, s)).collect::<Vec<_>>();
        n.extend(text(2, output));
        n.extend(text(3, output));
        n.extend(text(4, op));
        for a in attrs {
            n.extend(a);
        }
        self.0.extend(bytes(1, &n));
    }
    pub(crate) fn value(&mut self, name: &str, ty: u64, shape: &[i64], input: bool) {
        let dims = shape
            .iter()
            .flat_map(|&n| bytes(1, &number(1, n as u64)))
            .collect::<Vec<_>>();
        let tensor = bytes(1, &[number(1, ty), bytes(2, &dims)].concat());
        self.0.extend(bytes(
            if input { 11 } else { 12 },
            &[text(1, name), bytes(2, &tensor)].concat(),
        ));
    }
    pub(crate) fn constant(&mut self, name: &str, ty: u64, shape: &[u64], raw: &[u8]) {
        let t = [
            shape.iter().flat_map(|&n| number(1, n)).collect(),
            number(2, ty),
            text(8, name),
            bytes(9, raw),
        ]
        .concat();
        self.0.extend(bytes(5, &t));
    }
    pub(crate) fn ints(&mut self, name: &str, values: &[i64]) {
        self.constant(
            name,
            7,
            &[values.len() as u64],
            &values
                .iter()
                .flat_map(|n| n.to_le_bytes())
                .collect::<Vec<_>>(),
        );
    }
    pub(crate) fn scalar(&mut self, name: &str, value: f32) {
        self.constant(name, 1, &[], &value.to_le_bytes());
    }
    pub(crate) fn finish(mut self) -> Vec<u8> {
        self.0.extend(text(2, "Facecam GPU"));
        [
            number(1, 9),
            text(2, "Facecam Hub"),
            bytes(7, &self.0),
            bytes(8, &number(2, 17)),
        ]
        .concat()
    }
    pub(crate) fn resize(&mut self, input: &str, output: &str, target: &str) {
        self.node(
            "Resize",
            &[input, "", "", target],
            output,
            &[
                string("mode", "linear"),
                string("coordinate_transformation_mode", "half_pixel"),
            ],
        );
    }
}
fn upload_graph([w, h]: [usize; 2]) -> Graph {
    let mut g = Graph::default();
    g.value("bytes", 2, &[1, h as i64, w as i64, 3], true);
    g.node("Cast", &["bytes"], "floats", &[integer("to", 1)]);
    g.node(
        "Transpose",
        &["floats"],
        "rgb",
        &[integers("perm", &[0, 3, 1, 2])],
    );
    g
}
pub(crate) fn upload_model(size: [usize; 2]) -> Vec<u8> {
    let mut g = upload_graph(size);
    g.value("rgb", 1, &[1, 3, size[1] as i64, size[0] as i64], false);
    g.finish()
}
fn prepare_model(size: [usize; 2], [mw, mh]: [usize; 2], native: bool, source: bool) -> Vec<u8> {
    let mut g = if source {
        let mut g = Graph::default();
        g.value("rgb", 1, &[1, 3, size[1] as i64, size[0] as i64], true);
        g
    } else {
        upload_graph(size)
    };
    g.ints("target", &[1, 3, mh as i64, mw as i64]);
    g.scalar("half", 0.5);
    g.scalar("divisor", 255.0);
    g.resize("rgb", "resized", "target");
    g.node("Add", &["resized", "half"], "rounded", &[]);
    g.node("Floor", &["rounded"], "small", &[]);
    let channels = if native {
        g.ints("channels", &[2, 1, 0]);
        g.node(
            "Gather",
            &["small", "channels"],
            "bgr",
            &[integer("axis", 1)],
        );
        "bgr"
    } else {
        "small"
    };
    g.node("Div", &[channels, "divisor"], "image", &[]);
    g.value("rgb", 1, &[1, 3, size[1] as i64, size[0] as i64], false);
    for n in ["small", "image"] {
        g.value(n, 1, &[1, 3, mh as i64, mw as i64], false);
    }
    g.finish()
}
fn compose_model(
    [w, h]: [usize; 2],
    [mw, mh]: [usize; 2],
    mask_shape: &[i64],
    native: bool,
    radius: u32,
    blur: bool,
) -> Vec<u8> {
    let mut g = Graph::default();
    let full = [1, 3, h as i64, w as i64];
    g.value("rgb", 1, &full, true);
    g.value("mask", 1, mask_shape, true);
    g.scalar("half", 0.5);
    g.scalar("one", 1.0);
    g.scalar("zero", 0.0);
    for (op, out) in [("ReduceMin", "minimum"), ("ReduceMax", "maximum")] {
        g.node(op, &["mask"], out, &[integer("keepdims", 0)]);
        g.value(out, 1, &[], false);
    }
    g.node("IsNaN", &["mask"], "nan", &[]);
    g.node("IsInf", &["mask"], "inf", &[]);
    g.node("Or", &["nan", "inf"], "bad", &[]);
    g.node("Cast", &["bad"], "bad_float", &[integer("to", 1)]);
    g.node(
        "ReduceSum",
        &["bad_float"],
        "invalid",
        &[integer("keepdims", 0)],
    );
    g.value("invalid", 1, &[], false);
    let alpha = if native {
        g.ints("person", &[1]);
        g.node(
            "Gather",
            &["mask", "person"],
            "person_mask",
            &[integer("axis", 1)],
        );
        "person_mask"
    } else {
        g.ints("mask_shape", &[1, 1, mh as i64, mw as i64]);
        g.node("Reshape", &["mask", "mask_shape"], "person_mask", &[]);
        "person_mask"
    };
    g.node("Clip", &[alpha, "zero", "one"], "alpha_clipped", &[]);
    g.ints("alpha_size", &[1, 1, h as i64, w as i64]);
    g.resize("alpha_clipped", "alpha", "alpha_size");
    let bg = if blur {
        g.value("small", 1, &[1, 3, mh as i64, mw as i64], true);
        let r = u64::from(radius);
        g.node(
            "AveragePool",
            &["small"],
            "horizontal_mean",
            &[
                integers("kernel_shape", &[1, 2 * r + 1]),
                integers("pads", &[0, r, 0, r]),
                integer("count_include_pad", 0),
            ],
        );
        g.node("Floor", &["horizontal_mean"], "horizontal", &[]);
        g.node(
            "AveragePool",
            &["horizontal"],
            "vertical_mean",
            &[
                integers("kernel_shape", &[2 * r + 1, 1]),
                integers("pads", &[r, 0, r, 0]),
                integer("count_include_pad", 0),
            ],
        );
        g.node("Floor", &["vertical_mean"], "blurred", &[]);
        g.ints("full_size", &full);
        g.resize("blurred", "background_full", "full_size");
        "background_full"
    } else {
        g.value("background", 1, &full, true);
        "background"
    };
    g.node("Sub", &["one", "alpha"], "inverse", &[]);
    g.node("Mul", &["rgb", "alpha"], "foreground", &[]);
    g.node("Mul", &[bg, "inverse"], "hidden", &[]);
    g.node("Add", &["foreground", "hidden"], "composed", &[]);
    g.node("Add", &["composed", "half"], "rounded", &[]);
    g.node("Cast", &["rounded"], "rgb_u8", &[integer("to", 2)]);
    g.node(
        "Transpose",
        &["rgb_u8"],
        "output",
        &[integers("perm", &[0, 2, 3, 1])],
    );
    g.value("output", 2, &[1, h as i64, w as i64, 3], false);
    g.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ai::{Compositor, blur, resize_rgb_to},
        effects::{BackgroundMode, Effects},
    };

    fn download(value: &DynValue, shape: &[i64]) -> Vec<f32> {
        let mut graph = Graph::default();
        graph.value("input", 1, shape, true);
        graph.node("Identity", &["input"], "output", &[]);
        graph.value("output", 1, shape, false);
        let mut stage =
            Stage::open(&graph.finish(), &[], &["output"], 0, "oracle-download").unwrap();
        stage.binding.bind_input("input", value).unwrap();
        let outputs = stage.run().unwrap();
        let (_, values) = outputs["output"].try_extract_raw_tensor::<f32>().unwrap();
        values.to_vec()
    }

    #[test]
    #[ignore = "requires FACECAM_AI_RUNTIME and CUDA libraries; synthetic pixels only, no camera"]
    fn gpu_processing_matches_cpu_oracle_and_rejects_invalid_masks() {
        ort::init_from(std::env::var("FACECAM_AI_RUNTIME").expect("set FACECAM_AI_RUNTIME"))
            .commit()
            .unwrap();
        for (w, h, mw, mh) in [(37, 23, 80, 72), (101, 79, 80, 72)] {
            let frame: Vec<u8> = (0..w * h * 3).map(|i| (i * 31 % 256) as u8).collect();
            let bg: Vec<u8> = (0..w * h * 3).map(|i| (i * 13 % 256) as u8).collect();
            let small = resize_rgb_to(&frame, w, h, mw, mh);
            for native in [false, true] {
                let planes = if native { 2 } else { 1 };
                let shape = vec![1, planes, mh as i64, mw as i64];
                for radius in [1, 4, 10, 32] {
                    for use_blur in [true, false] {
                        let effects = Effects {
                            blur_radius: radius,
                            background_mode: if use_blur {
                                BackgroundMode::Blur
                            } else {
                                BackgroundMode::Solid
                            },
                            ..Default::default()
                        };
                        let contract = Contract {
                            size: [w, h],
                            mask: [mw, mh],
                            mask_shape: shape.clone(),
                            native,
                            device: 0,
                        };
                        let mut gpu = Processing::open(
                            contract,
                            &effects,
                            (!use_blur).then_some(bg.as_slice()),
                        )
                        .unwrap();
                        let prepared = gpu.prepare(&frame).unwrap();
                        let image = download(&prepared.image, &[1, 3, mh as i64, mw as i64]);
                        for c in 0..3 {
                            for i in 0..mw * mh {
                                let expected =
                                    f32::from(small[i * 3 + if native { 2 - c } else { c }])
                                        / 255.0;
                                assert!(
                                    (image[c * mw * mh + i] - expected).abs() <= 1.0 / 255.0 + 1e-6,
                                    "input channels/normalization mismatch"
                                );
                            }
                        }
                        for alpha in [0.0, 0.5, 1.0, -1.0, -2.0] {
                            let mask: Vec<f32> = (0..mw * mh)
                                .map(|i| {
                                    if alpha == -2.0 {
                                        (i * 17 % 997) as f32 / 996.0
                                    } else if alpha < 0.0 {
                                        (i % mw + i / mw) as f32 / (mw + mh - 2) as f32
                                    } else {
                                        alpha
                                    }
                                })
                                .collect();
                            let values = if native {
                                [mask.iter().map(|a| 1.0 - a).collect(), mask.clone()].concat()
                            } else {
                                mask.clone()
                            };
                            let tensor = Tensor::from_array((shape.clone(), values))
                                .unwrap()
                                .into_dyn();
                            let result = gpu.compose(&prepared, &tensor, None).unwrap();
                            let (_, actual) = result.try_extract_raw_tensor::<u8>().unwrap();
                            let mut expected = frame.clone();
                            let blurred = use_blur.then(|| blur(&small, mw, mh, radius as usize));
                            Compositor::for_mask(w, h, mw, mh).apply(
                                &mut expected,
                                &mask,
                                blurred.as_deref(),
                                (!use_blur).then_some(bg.as_slice()),
                            );
                            let max = actual
                                .iter()
                                .zip(&expected)
                                .map(|(a, b)| a.abs_diff(*b))
                                .max()
                                .unwrap();
                            assert!(
                                max <= 2,
                                "GPU/CPU mismatch: {max}, native={native}, radius={radius}, blur={use_blur}, alpha={alpha}"
                            );
                        }
                        for bad in [-1.0, 1.1, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                            let mut values = vec![0.0; mw * mh * planes as usize];
                            let middle = values.len() / 2;
                            values[middle] = bad;
                            let tensor = Tensor::from_array((shape.clone(), values))
                                .unwrap()
                                .into_dyn();
                            assert!(
                                gpu.compose(&prepared, &tensor, None).is_err(),
                                "invalid mask accepted: {bad}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    #[ignore = "requires FACECAM_AI_RUNTIME and CUDA libraries; synthetic pixels only, no camera"]
    fn gpu_video_background_changes_each_frame_without_overwriting_foreground() {
        ort::init_from(std::env::var("FACECAM_AI_RUNTIME").expect("set FACECAM_AI_RUNTIME"))
            .commit()
            .unwrap();
        let effects = Effects {
            background_mode: BackgroundMode::Video,
            ..Default::default()
        };
        let contract = Contract {
            size: [8, 8],
            mask: [4, 4],
            mask_shape: vec![1, 1, 4, 4],
            native: false,
            device: 0,
        };
        let mut gpu = Processing::open(contract, &effects, None).unwrap();
        let mask = Tensor::from_array(([1, 1, 4, 4], vec![0.5_f32; 16]))
            .unwrap()
            .into_dyn();
        for (foreground, background) in [(20, 100), (80, 200), (200, 20)] {
            let prepared = gpu.prepare(&[foreground; 8 * 8 * 3]).unwrap();
            let output = gpu
                .compose(&prepared, &mask, Some(&[background; 8 * 8 * 3]))
                .unwrap();
            let (_, actual) = output.try_extract_raw_tensor::<u8>().unwrap();
            assert!(
                actual
                    .iter()
                    .all(|&p| p == (foreground / 2 + background / 2))
            );
        }
    }
}
