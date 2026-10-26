//! Import the exact locally supplied Windows model; no vendor key or model is embedded.
use anyhow::{Context, Result, bail};
use cbc::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
use sha2::{Digest, Sha256};

pub const SDK_URL: &str = "https://international.download.nvidia.com/Windows/broadcast/sdk/VFX/nvidia_video_effects_sdk_installer_v0.7.6_ada.exe";
pub const SDK_SHA: &str = "4761fe1579e82de6d45efec8dc09492cf2cc601867269955e129707ac8ea8546";
pub const MODEL_SHA: &str = "6636284e5d3d8363c01f83aac90f1ca08ac05952537f38c1e82a39500af87d22";
pub const DLL_SHA: &str = "2a78effae55e9e3791bbe66f312f0bd08a2ede2c94fd8f820933badfa0a6cd8c";
const ENGINE_SHA: &str = "518ff60c08e4900e1fe363807b188cff2215d6a9c38962437fc280c826fdf0d6";
pub const WIDTH: usize = 512;
pub const HEIGHT: usize = 288;
pub const STATES: [(&str, &str, [usize; 4]); 6] = [
    ("rec_ce.1", "rec_ce", [1, 64, 9, 16]),
    ("rec1.1", "rec1", [1, 64, 18, 32]),
    ("rec2.1", "rec2", [1, 32, 36, 64]),
    ("rec3.1", "rec3", [1, 16, 72, 128]),
    ("rec4.1", "rec4", [1, 8, 144, 256]),
    ("rec5.1", "rec5", [1, 4, 288, 512]),
];

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset.checked_add(4).context("Offset de modelo inválido")?;
    let value: [u8; 4] = bytes
        .get(offset..end)
        .context("Cabeçalho de modelo incompleto")?
        .try_into()?;
    Ok(u32::from_le_bytes(value))
}

pub(crate) fn encrypted_payload(package: &[u8]) -> Result<(&[u8], &[u8])> {
    if package.get(..4) != Some(b"NVnn") || u32_at(package, 4)? != 1 || u32_at(package, 8)? != 1 {
        bail!("Contêiner de segmentação NVIDIA incompatível");
    }
    let metadata = usize::try_from(u32_at(package, 24)?)?;
    let iv_start = 28usize
        .checked_add(metadata)
        .context("Metadados de modelo inválidos")?;
    let table = iv_start.checked_add(16).context("IV de modelo inválido")?;
    if u32_at(package, table)? != 1 {
        bail!("Esperado um único engine de segmentação");
    }
    let skip = usize::try_from(u32_at(package, table + 4)?)?;
    let length = usize::try_from(u32_at(package, table + 8)?)?;
    let start = table
        .checked_add(12)
        .and_then(|n| n.checked_add(skip))
        .context("Offset de payload inválido")?;
    let end = start
        .checked_add(length)
        .context("Comprimento de payload inválido")?;
    if length == 0 || length % 16 != 0 || end != package.len() {
        bail!("Payload de segmentação incompleto ou inválido");
    }
    Ok((
        package.get(iv_start..table).context("IV incompleto")?,
        package.get(start..end).context("Payload incompleto")?,
    ))
}

pub fn import(dll: &[u8], package: &[u8]) -> Result<Vec<u8>> {
    if digest(dll) != DLL_SHA || digest(package) != MODEL_SHA {
        bail!("Os arquivos não correspondem ao Video Effects 0.7.6 Ada validado");
    }
    let (iv, ciphertext) = encrypted_payload(package)?;
    let source = dll
        .get(0xc1698..0xc16b8)
        .context("Dados do runtime Windows incompletos")?;
    let mut key = [0_u8; 32];
    for (i, (&byte, value)) in source.iter().zip(&mut key).enumerate() {
        *value = byte ^ (i as u8 + 0x6d);
    }
    let mut buffer = ciphertext.to_vec();
    let decoder = cbc::Decryptor::<aes::Aes256>::new_from_slices(&key, iv)
        .map_err(|_| anyhow::anyhow!("Parâmetros do contêiner inválidos"))?;
    key.fill(0);
    let mut engine = decoder
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|_| anyhow::anyhow!("Não foi possível ler o engine do modelo"))?
        .to_vec();
    if digest(&engine) != ENGINE_SHA {
        bail!("Engine recuperado não corresponde ao modelo validado");
    }
    normalize_platform(&mut engine)?;
    Ok(engine)
}

pub(crate) fn normalize_platform(engine: &mut [u8]) -> Result<()> {
    if engine.get(..4) != Some(b"ftrt")
        || engine.get(24..28) != Some(&[10, 4, 0, 26])
        || u32_at(engine, 28)? != 0x74727471
    {
        bail!("Cabeçalho TensorRT incompatível com esta conversão");
    }
    engine[28..32].copy_from_slice(&0x74727472_u32.to_le_bytes());
    Ok(())
}

fn varint(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let byte = (value & 127) as u8;
        value >>= 7;
        out.push(byte | if value == 0 { 0 } else { 128 });
        if value == 0 {
            return out;
        }
    }
}
pub(crate) fn number(field: u64, value: u64) -> Vec<u8> {
    [varint(field << 3), varint(value)].concat()
}
pub(crate) fn bytes(field: u64, value: &[u8]) -> Vec<u8> {
    [
        varint((field << 3) | 2),
        varint(value.len() as u64),
        value.to_vec(),
    ]
    .concat()
}
pub(crate) fn text(field: u64, value: &str) -> Vec<u8> {
    bytes(field, value.as_bytes())
}
fn tensor(name: &str, shape: [usize; 4], input: bool) -> Vec<u8> {
    let dimensions = shape
        .into_iter()
        .flat_map(|n| bytes(1, &number(1, n as u64)))
        .collect::<Vec<_>>();
    let ty = bytes(1, &[number(1, 1), bytes(2, &dimensions)].concat());
    bytes(
        if input { 11 } else { 12 },
        &[text(1, name), bytes(2, &ty)].concat(),
    )
}

/// ONNX EPContext metadata wrapping a local engine; the network is not embedded.
pub fn context_model() -> Vec<u8> {
    let inputs = std::iter::once(("input_1", [1, 3, HEIGHT, WIDTH]))
        .chain(STATES.iter().map(|&(i, _, s)| (i, s)))
        .collect::<Vec<_>>();
    let outputs = std::iter::once(("output_1", [1, 2, HEIGHT, WIDTH]))
        .chain(STATES.iter().map(|&(_, o, s)| (o, s)))
        .collect::<Vec<_>>();
    let mut node = Vec::new();
    for &(name, _) in &inputs {
        node.extend(text(1, name));
    }
    for &(name, _) in &outputs {
        node.extend(text(2, name));
    }
    node.extend(
        [
            text(3, "aigs-native"),
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
            text(4, "segmentation.engine"),
            number(20, 3),
        ]
        .concat(),
    ));
    let mut graph = [bytes(1, &node), text(2, "aigs-native")].concat();
    for (name, shape) in inputs {
        graph.extend(tensor(name, shape, true));
    }
    for (name, shape) in outputs {
        graph.extend(tensor(name, shape, false));
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn container_rejects_truncation_overflow_and_unexpected_block_layout() {
        for length in [0, 3, 4, 12, 27, 28, 43, 44, 55] {
            assert!(encrypted_payload(&vec![0; length]).is_err());
        }
        let mut p = vec![0; 72];
        p[..4].copy_from_slice(b"NVnn");
        p[4] = 1;
        p[8] = 1;
        p[44] = 1;
        p[52] = 16;
        assert!(encrypted_payload(&p).is_ok());
        for field in [24, 48, 52] {
            let mut bad = p.clone();
            bad[field..field + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(encrypted_payload(&bad).is_err());
        }
        p[44] = 2;
        assert!(encrypted_payload(&p).is_err());
        assert!(import(&[0; 32], &p).is_err());
    }
    #[test]
    fn conversion_is_limited_to_the_verified_header_version_and_preserves_payload() {
        let mut e = vec![42; 48];
        e[..4].copy_from_slice(b"ftrt");
        e[24..28].copy_from_slice(&[10, 4, 0, 26]);
        e[28..32].copy_from_slice(&0x74727471_u32.to_le_bytes());
        let original = e.clone();
        normalize_platform(&mut e).unwrap();
        assert_eq!(&e[..28], &original[..28]);
        assert_eq!(&e[32..], &original[32..]);
        assert_eq!(e[28], b'r');
        assert!(normalize_platform(&mut e).is_err());
        assert!(normalize_platform(&mut [0; 8]).is_err());
    }
}
