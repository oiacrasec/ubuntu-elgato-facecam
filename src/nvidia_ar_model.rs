//! Exact AR 0.8.7 Ada contracts. Vendor bytes and keys are never embedded.
use crate::nvidia_model::{bytes, number, text};
use anyhow::{Context, Result, bail};
use cbc::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};

pub const SDK_URL: &str = "https://international.download.nvidia.com/Windows/broadcast/sdk/AR/nvidia_ar_sdk_installer_v0.8.7_ada.exe";
pub const SDK_SHA: &str = "0e2e1110065ed58b9c4a844c674fe5f50ed5fe37a0d170bd229a4bbc48861ca5";
pub const DLL_SHA: &str = "90f8b1b4747349651621bc9a991c8f856664843f5315870677c485dbaadcc9a4";

#[derive(Clone, Copy, Debug)]
pub struct TensorSpec {
    pub name: &'static str,
    pub shape: &'static [usize],
}
#[derive(Clone, Copy, Debug)]
pub struct Model {
    pub id: &'static str,
    pub source: &'static str,
    pub package_sha: &'static str,
    pub engine_sha: &'static str,
    pub inputs: &'static [TensorSpec],
    pub outputs: &'static [TensorSpec],
}

pub const MODELS: [Model; 4] = [
    Model {
        id: "face",
        source: "face_detection_89.engine.trtpkg",
        package_sha: "1e5fbb250e06095f4a3726bb9d1ce1cd8d52a475cba1f405cb1a3af728c95eee",
        engine_sha: "b2e9a3add120a67ebfc63ec0b413a9ef95d33589af992606bf53ba5f3f18cc5c",
        inputs: &[TensorSpec {
            name: "input",
            shape: &[1, 3, 208, 368],
        }],
        outputs: &[
            TensorSpec {
                name: "output_bbox",
                shape: &[1, 4, 13, 23],
            },
            TensorSpec {
                name: "output_cov/Sigmoid",
                shape: &[1, 1, 13, 23],
            },
        ],
    },
    Model {
        id: "landmarks",
        source: "faceland_126_maxbatch1_mode0_89.engine.trtpkg",
        package_sha: "41a116dcb86135323180f42fe808bea15dfa631c00115a181222f3f4ef9d0674",
        engine_sha: "d60a8f81aedca36101830d869ec3455074f836938fad6251db215278375da7b9",
        inputs: &[TensorSpec {
            name: "input",
            shape: &[1, 1, 80, 80],
        }],
        outputs: &[
            TensorSpec {
                name: "kpts",
                shape: &[1, 252],
            },
            TensorSpec {
                name: "confidence",
                shape: &[1, 126],
            },
            TensorSpec {
                name: "headpose",
                shape: &[1, 3],
            },
        ],
    },
    Model {
        id: "encoder",
        source: "gazeredir_encoder_fp16_89.engine.trtpkg",
        package_sha: "8a1224b1a2f97a46d89a984177b6a3d43e65812a9b84f0e4701d003ac3d87a7c",
        engine_sha: "579a448be4a4aecf03660dabe02646804c66ce16d7b97d0632c41b81fa9bbbfa",
        inputs: &[TensorSpec {
            name: "input_image",
            shape: &[1, 3, 64, 256],
        }],
        outputs: &[
            TensorSpec {
                name: "pseudo_labels_flat",
                shape: &[1, 12],
            },
            TensorSpec {
                name: "embeddings_flat",
                shape: &[1, 1344],
            },
        ],
    },
    Model {
        id: "decoder",
        source: "gazeredir_decoder_fp16_89.engine.trtpkg",
        package_sha: "ef3b4bbd06d27052edd458317248c933d0ec1ddb8a05a48c4ae47bca24cb824a",
        engine_sha: "6c6334f415c56dcdb5baeeceb48440e8239e339db897eebf5a7014bb837c5a70",
        inputs: &[
            TensorSpec {
                name: "embeddings_flat",
                shape: &[1, 1344],
            },
            TensorSpec {
                name: "pseudo_labels_flat",
                shape: &[1, 12],
            },
            TensorSpec {
                name: "gaze_por",
                shape: &[1, 2],
            },
        ],
        outputs: &[
            TensorSpec {
                name: "gaze_redirected_image",
                shape: &[1, 3, 64, 256],
            },
            TensorSpec {
                name: "gaze_landmarks",
                shape: &[1, 2, 12, 1],
            },
        ],
    },
];

impl Model {
    pub fn import(&self, dll: &[u8], package: &[u8]) -> Result<Vec<u8>> {
        if dll.len() > 8 * 1024 * 1024
            || package.len() > 64 * 1024 * 1024
            || crate::nvidia_model::digest(dll) != DLL_SHA
            || crate::nvidia_model::digest(package) != self.package_sha
        {
            bail!("Os arquivos não correspondem ao AR 0.8.7 Ada validado");
        }
        let (iv, ciphertext) = crate::nvidia_model::encrypted_payload(package)?;
        let source = dll
            .get(0x11ece8..0x11ed08)
            .context("Dados do runtime AR incompletos")?;
        let mut key = [0_u8; 32];
        for (i, (&byte, value)) in source.iter().zip(&mut key).enumerate() {
            *value = byte ^ (i as u8 + 0x6d);
        }
        let decoder = cbc::Decryptor::<aes::Aes256>::new_from_slices(&key, iv)
            .map_err(|_| anyhow::anyhow!("Parâmetros do contêiner AR inválidos"))?;
        key.fill(0);
        let mut buffer = ciphertext.to_vec();
        let mut engine = decoder
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|_| anyhow::anyhow!("Não foi possível ler o engine AR"))?
            .to_vec();
        if crate::nvidia_model::digest(&engine) != self.engine_sha {
            bail!("Engine AR não corresponde ao modelo validado");
        }
        crate::nvidia_model::normalize_platform(&mut engine)?;
        Ok(engine)
    }

    /// Small owned descriptor referring to a local engine, without model weights.
    pub fn descriptor(&self) -> Vec<u8> {
        let mut node = Vec::new();
        for input in self.inputs {
            node.extend(text(1, input.name));
        }
        for output in self.outputs {
            node.extend(text(2, output.name));
        }
        node.extend(
            [
                text(3, "ar-native"),
                text(4, "EPContext"),
                text(7, "com.microsoft"),
            ]
            .concat(),
        );
        node.extend(bytes(
            5,
            &[text(1, "embed_mode"), number(3, 0), number(20, 2)].concat(),
        ));
        node.extend(bytes(
            5,
            &[
                text(1, "ep_cache_context"),
                text(4, &format!("{}.engine", self.id)),
                number(20, 3),
            ]
            .concat(),
        ));
        let mut graph = [bytes(1, &node), text(2, "ar-native")].concat();
        for (specs, field) in [(self.inputs, 11), (self.outputs, 12)] {
            for tensor in specs {
                let dims = tensor
                    .shape
                    .iter()
                    .flat_map(|&n| bytes(1, &number(1, n as u64)))
                    .collect::<Vec<_>>();
                let ty = bytes(1, &[number(1, 1), bytes(2, &dims)].concat());
                graph.extend(bytes(
                    field,
                    &[text(1, tensor.name), bytes(2, &ty)].concat(),
                ));
            }
        }
        [
            number(1, 9),
            text(2, "Facecam Hub"),
            bytes(7, &graph),
            bytes(8, &number(2, 17)),
            bytes(8, &[text(1, "com.microsoft"), number(2, 1)].concat()),
        ]
        .concat()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ar_descriptors_have_fixed_contracts_without_models_or_vendor_keys() {
        for model in MODELS {
            assert!(model.import(&[], &[]).is_err());
            let descriptor = model.descriptor();
            assert!(descriptor.len() < 2048);
            assert!(descriptor.windows(9).any(|s| s == b"EPContext"));
            for tensor in model.inputs.iter().chain(model.outputs) {
                assert!(
                    descriptor
                        .windows(tensor.name.len())
                        .any(|s| s == tensor.name.as_bytes())
                );
                assert!(tensor.shape.iter().product::<usize>() <= 3 * 208 * 368);
            }
            assert_eq!(model.package_sha.len(), 64);
            assert_eq!(model.engine_sha.len(), 64);
        }
    }
}
