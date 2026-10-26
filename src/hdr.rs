//! Facecam MK.2 HDR. Only the verified unit GUID/selector is eligible for writes.
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const GUID: [u8; 16] = [
    0xd0, 0x9e, 0xe4, 0x23, 0x78, 0x11, 0x31, 0x4f, 0xae, 0x52, 0xd2, 0xfb, 0x8a, 0x8d, 0x30, 0x01,
];
const SELECTOR: u8 = 21;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Status {
    pub supported: bool,
    pub available: bool,
    pub pending: bool,
    pub needs_tool: bool,
    pub enabled: Option<bool>,
    pub unit: Option<u8>,
    pub error: Option<String>,
}

fn unit(descriptors: &[u8]) -> Result<Option<u8>> {
    if descriptors.len() > 65536 {
        bail!("Descritores USB excedem o limite");
    }
    let mut bytes = descriptors;
    let mut found = None;
    while !bytes.is_empty() {
        let size = usize::from(*bytes.first().context("Descritor USB incompleto")?);
        if size < 2 || size > bytes.len() {
            bail!("Descritor USB truncado ou inválido");
        }
        let item = &bytes[..size];
        if item.get(1..3) == Some(&[0x24, 6]) && item.get(4..20) == Some(GUID.as_slice()) {
            let pins = usize::from(*item.get(21).context("Unidade USB incompleta")?);
            let width = usize::from(*item.get(22 + pins).context("Controles USB incompletos")?);
            let mask = item
                .get(23 + pins..23 + pins + width)
                .context("Mapa de controles USB truncado")?;
            if item[3] == 0 || found.is_some() {
                bail!("Unidade HDR ambígua ou inválida");
            }
            if mask
                .get(usize::from((SELECTOR - 1) / 8))
                .is_some_and(|b| b & (1 << ((SELECTOR - 1) % 8)) != 0)
            {
                found = Some(item[3]);
            }
        }
        bytes = &bytes[size..];
    }
    Ok(found)
}

fn camera_unit(device: &str) -> Result<Option<u8>> {
    if !crate::config::video_device(device) {
        bail!("Dispositivo de vídeo inválido");
    }
    let name = Path::new(device)
        .file_name()
        .context("Dispositivo de vídeo inválido")?;
    let interface = std::fs::canonicalize(
        Path::new("/sys/class/video4linux")
            .join(name)
            .join("device"),
    )?;
    for parent in interface.ancestors() {
        if let (Ok(vendor), Ok(product)) = (
            std::fs::read_to_string(parent.join("idVendor")),
            std::fs::read_to_string(parent.join("idProduct")),
        ) {
            // Other Facecam revisions need their own confirmed transport contract.
            if vendor.trim() != "0fd9" || !matches!(product.trim(), "0093" | "0094") {
                return Ok(None);
            }
            let file = std::fs::File::open(parent.join("descriptors"))?;
            let mut descriptors = Vec::new();
            file.take(65537).read_to_end(&mut descriptors)?;
            return unit(&descriptors);
        }
    }
    Ok(None)
}

fn run(args: &[String]) -> Result<String> {
    let tool = std::env::var_os("FACECAM_UVC_TOOL").unwrap_or_else(|| "uvcdynctrl".into());
    run_tool(&tool, args)
}

fn run_tool(tool: &std::ffi::OsStr, args: &[String]) -> Result<String> {
    let mut child = Command::new(tool)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .context("Saída da ferramenta HDR indisponível")?;
    let stderr = child
        .stderr
        .take()
        .context("Diagnóstico HDR indisponível")?;
    let read = |reader: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            reader.take(65537).read_to_end(&mut bytes).map(|_| bytes)
        })
    };
    let out = read(Box::new(stdout));
    let err = read(Box::new(stderr));
    let deadline = Instant::now() + Duration::from_secs(1);
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                break Err(anyhow::anyhow!(
                    "A câmera não respondeu ao comando HDR a tempo"
                ));
            }
            Err(e) => break Err(e.into()),
        }
    };
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let stdout = out
        .join()
        .map_err(|_| anyhow::anyhow!("Leitura HDR interrompida"))??;
    let stderr = err
        .join()
        .map_err(|_| anyhow::anyhow!("Diagnóstico HDR interrompido"))??;
    let status = result?;
    if stdout.len() > 65536 || stderr.len() > 65536 {
        bail!("Resposta HDR excede o limite");
    }
    if !status.success() {
        bail!(
            "Falha na ferramenta HDR: {}",
            String::from_utf8_lossy(&stderr)
        );
    }
    String::from_utf8(stdout).context("Resposta HDR não é texto UTF-8")
}

fn byte_value(text: &str, label: &str) -> Result<u8> {
    let line = text
        .lines()
        .find(|line| line.starts_with(label))
        .context("Resposta HDR incompleta")?;
    let value = line
        .split_once("(LE)0x")
        .context("Valor HDR ausente")?
        .1
        .split_whitespace()
        .next()
        .context("Valor HDR ausente")?;
    if value.len() != 2 {
        bail!("Valor HDR não tem um byte");
    }
    Ok(u8::from_str_radix(value, 16)?)
}

fn parse(text: &str) -> Result<Option<bool>> {
    let size = text
        .lines()
        .find_map(|line| line.strip_prefix("query control size of : "))
        .context("Tamanho HDR ausente")?
        .trim()
        .parse::<u32>()?;
    let flags = text
        .lines()
        .find_map(|line| line.strip_prefix("query control flags of: 0x"))
        .context("Flags HDR ausentes")?
        .trim();
    let flags = u8::from_str_radix(flags, 16)?;
    if size != 1
        || flags & 7 != 3
        || byte_value(text, "query minimum value of:")? != 0
        || byte_value(text, "query maximum value of:")? != 1
        || byte_value(text, "query step size of")? != 1
    {
        bail!("Contrato HDR da câmera incompatível");
    }
    match byte_value(text, "query current value of:")? {
        0 => Ok(Some(false)),
        1 => Ok(Some(true)),
        0xff => Ok(None),
        _ => bail!("Estado HDR da câmera inválido"),
    }
}

fn query(device: &str, unit: u8) -> Result<Option<bool>> {
    parse(&run(&[
        format!("--device={device}"),
        format!("--get_raw={unit}:{SELECTOR}"),
    ])?)
}

pub fn discover(device: &str) -> Status {
    let mut status = Status::default();
    let unit = match camera_unit(device) {
        Ok(Some(unit)) => {
            status.supported = true;
            status.unit = Some(unit);
            unit
        }
        Ok(None) => return status,
        Err(error) => {
            status.error = Some(format!("{error:#}"));
            return status;
        }
    };
    match query(device, unit) {
        Ok(Some(enabled)) => {
            status.available = true;
            status.enabled = Some(enabled);
        }
        Ok(None) => {
            status.pending = true;
            status.error =
                Some("Aguardando a câmera confirmar o HDR; mantenha o preview ativo".into());
        }
        Err(error) => {
            status.needs_tool = error
                .root_cause()
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound);
            status.error = Some(if status.needs_tool {
                "Instale o suporte HDR para controlar esta câmera".into()
            } else {
                format!("{error:#}")
            });
        }
    }
    status
}

fn wait(device: &str, unit: u8, wanted: Option<bool>, timeout: Duration) -> Result<bool> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(actual) = query(device, unit)?
            && wanted.is_none_or(|value| value == actual)
        {
            return Ok(actual);
        }
        if Instant::now() >= deadline {
            bail!("A câmera não confirmou a alteração de HDR");
        }
        thread::sleep(Duration::from_millis(80));
    }
}

pub fn set(device: &str, enabled: bool) -> Result<bool> {
    let unit = camera_unit(device)?.context("HDR não confirmado para este modelo de câmera")?;
    if wait(device, unit, None, Duration::from_millis(1500))? == enabled {
        return Ok(enabled);
    }
    run(&[
        format!("--device={device}"),
        format!("--set_raw={unit}:{SELECTOR}"),
        format!("0x{:02x}", u8::from(enabled)),
    ])?;
    wait(device, unit, Some(enabled), Duration::from_secs(2))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn descriptor() -> Vec<u8> {
        let mut bytes = vec![29, 0x24, 6, 7];
        bytes.extend(GUID);
        bytes.extend([32, 1, 3, 4, 0x83, 0x26, 0xb3, 0, 0]);
        bytes
    }
    #[test]
    fn usb_contract_refuses_truncation_wrong_units_and_missing_selectors() {
        let bytes = descriptor();
        assert_eq!(unit(&bytes).unwrap(), Some(7));
        for length in 1..bytes.len() {
            assert!(unit(&bytes[..length]).is_err());
        }
        let mut changed = bytes.clone();
        changed[4] ^= 1;
        assert_eq!(unit(&changed).unwrap(), None);
        changed = bytes.clone();
        changed[26] &= !0x10;
        assert_eq!(unit(&changed).unwrap(), None);
        changed = bytes.clone();
        changed[3] = 0;
        assert!(unit(&changed).is_err());
        changed = bytes.clone();
        changed.extend(&bytes);
        assert!(unit(&changed).is_err());
        assert!(unit(&vec![0; 65537]).is_err());
    }
    fn reply(value: &str) -> String {
        format!(
            "query control size of : 1\nquery control flags of: 0x3\nquery minimum value of: (LE)0x00 (BE)0x00\nquery maximum value of: (LE)0x01 (BE)0x01\nquery step size of    : (LE)0x01 (BE)0x01\nquery current value of: (LE)0x{value} (BE)0x{value}\n"
        )
    }
    #[test]
    fn hdr_reply_requires_boolean_writable_contract_and_confirmed_byte() {
        assert_eq!(parse(&reply("00")).unwrap(), Some(false));
        assert_eq!(parse(&reply("01")).unwrap(), Some(true));
        assert_eq!(parse(&reply("ff")).unwrap(), None);
        for invalid in [
            reply("02"),
            reply("-1"),
            reply("0001"),
            reply("01").replace("size of : 1", "size of : 2"),
            reply("01").replace("flags of: 0x3", "flags of: 0x7"),
            reply("01").replace("(LE)0x01", "(LE)0x02"),
            "unknown response".into(),
        ] {
            assert!(parse(&invalid).is_err(), "{invalid}");
        }
    }
    #[test]
    fn hdr_tool_is_bounded_and_rejects_non_utf8_output() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let tool = dir.path().join("tool");
        std::fs::write(&tool, "#!/bin/sh\nexec /bin/sleep 5\n").unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o700)).unwrap();
        let start = Instant::now();
        let error = run_tool(tool.as_os_str(), &[]).unwrap_err();
        assert!(error.to_string().contains("a tempo"), "{error}");
        assert!(start.elapsed() < Duration::from_secs(3));
        std::fs::write(&tool, "#!/bin/sh\nprintf '\\377'\n").unwrap();
        assert!(run_tool(tool.as_os_str(), &[]).is_err());
    }
}
