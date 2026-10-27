//! User library and original bundled looks. Paths stay inside the controller protocol.
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Lut,
    Image,
    Video,
}

impl Kind {
    fn directory(self) -> &'static str {
        match self {
            Self::Lut => "luts",
            Self::Image => "images",
            Self::Video => "videos",
        }
    }
    fn accepts(self, path: &Path) -> bool {
        let ext = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        match self {
            Self::Lut => ext == "cube",
            Self::Image => matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp"),
            Self::Video => matches!(ext.as_str(), "mp4" | "mov" | "webm" | "mkv"),
        }
    }
    fn limit(self) -> u64 {
        match self {
            Self::Lut => 16 * 1024 * 1024,
            Self::Image => 32 * 1024 * 1024,
            Self::Video => 4 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resource {
    pub kind: Kind,
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub removable: bool,
}

const LOOKS: [(&str, &str); 3] = [
    ("warm.cube", "Quente"),
    ("cool.cube", "Frio"),
    ("soft-contrast.cube", "Contraste suave"),
];

pub fn catalog(root: &Path, effects: &crate::effects::Effects) -> Vec<Resource> {
    let mut found: Vec<_> = LOOKS
        .iter()
        .map(|(file, name)| Resource {
            kind: Kind::Lut,
            name: (*name).into(),
            path: root.join("builtins-v1").join(file),
            removable: false,
        })
        .collect();
    // Keep explicitly selected resources visible even in a large library.
    for (kind, path) in [
        (Kind::Lut, &effects.lut),
        (Kind::Image, &effects.background_image),
        (Kind::Video, &effects.background_video),
    ] {
        if !path.is_empty() {
            add(&mut found, root, kind, path.into());
        }
    }
    let mut roots = vec![root.to_owned()];
    if let Some(data) = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    {
        roots.push(data.join("facecam-hub/resources"));
    }
    roots.extend(
        [
            "/usr/local/share/facecam-hub/resources",
            "/usr/share/facecam-hub/resources",
        ]
        .map(PathBuf::from),
    );
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        roots.push(parent.join("resources"));
        if !parent.join("installation.json").is_file() {
            roots.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/backgrounds"));
        }
    }
    for base in roots {
        for kind in [Kind::Lut, Kind::Image, Kind::Video] {
            let mut files: Vec<_> = fs::read_dir(base.join(kind.directory()))
                .into_iter()
                .flatten()
                .take(256)
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    !p.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .starts_with('.')
                        && kind.accepts(p)
                        && p.is_file()
                })
                .collect();
            files.sort();
            for path in files {
                add(&mut found, root, kind, path);
            }
        }
    }
    found
}

fn add(found: &mut Vec<Resource>, root: &Path, kind: Kind, path: PathBuf) {
    let bytes: usize = found
        .iter()
        .map(|r| r.path.as_os_str().len() + r.name.len() + 128)
        .sum();
    if found.len() < 128
        && bytes + path.as_os_str().len() * 2 + 128 <= 65536
        && path.to_str().is_some()
        && !found.iter().any(|r| r.path == path)
    {
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .replace(['_', '-'], " ");
        let removable = kind == Kind::Image && validate_owned_image(root, &path).is_ok();
        found.push(Resource {
            kind,
            name,
            path,
            removable,
        });
    }
}

/// Only imported copies in this controller's image directory can be removed.
pub fn validate_owned_image(root: &Path, path: &Path) -> Result<()> {
    let directory = root.join(Kind::Image.directory());
    if path.parent() != Some(directory.as_path())
        || !Kind::Image.accepts(path)
        || path
            .file_name()
            .is_none_or(|n| n.to_string_lossy().starts_with('.'))
        || !fs::symlink_metadata(&directory)
            .context("Biblioteca de imagens indisponível")?
            .is_dir()
        || !fs::symlink_metadata(path)
            .context("Imagem indisponível na biblioteca")?
            .is_file()
    {
        bail!("Só é possível remover imagens importadas para a biblioteca deste aplicativo");
    }
    Ok(())
}

pub fn remove_image(root: &Path, path: &Path) -> Result<()> {
    validate_owned_image(root, path)?;
    fs::remove_file(path).context("Não foi possível remover a imagem da biblioteca")
}

/// Generate our own small color transforms only when a built-in is selected.
pub fn materialize(root: &Path, selected: &str) -> Result<()> {
    for (index, (file, _)) in LOOKS.iter().enumerate() {
        let path = root.join("builtins-v1").join(file);
        if Path::new(selected) == path && !path.is_file() {
            fs::create_dir_all(path.parent().context("Biblioteca sem diretório")?)?;
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)?;
            output.write_all(look(index).as_bytes())?;
        }
    }
    Ok(())
}

fn look(index: usize) -> String {
    let mut cube = String::from("# Facecam Hub — original look\nLUT_3D_SIZE 9\n");
    for b in 0..9 {
        for g in 0..9 {
            for r in 0..9 {
                let mut rgb = [r as f64 / 8.0, g as f64 / 8.0, b as f64 / 8.0];
                match index {
                    0 => {
                        rgb[0] *= 1.06;
                        rgb[2] *= 0.93;
                    }
                    1 => {
                        rgb[0] *= 0.94;
                        rgb[2] *= 1.06;
                    }
                    _ => {
                        for channel in &mut rgb {
                            *channel = (*channel - 0.5) * 1.08 + 0.5;
                        }
                    }
                }
                cube.push_str(&format!(
                    "{:.6} {:.6} {:.6}\n",
                    rgb[0].clamp(0.0, 1.0),
                    rgb[1].clamp(0.0, 1.0),
                    rgb[2].clamp(0.0, 1.0)
                ));
            }
        }
    }
    cube
}

pub fn validate_lut(path: &Path) -> Result<()> {
    let file = fs::File::open(path).context("Não foi possível abrir a LUT")?;
    let mut contents = String::new();
    file.take(Kind::Lut.limit() + 1)
        .read_to_string(&mut contents)
        .context("A LUT deve conter texto UTF-8")?;
    if contents.len() as u64 > Kind::Lut.limit() {
        bail!("LUT excede 16 MiB");
    }
    let mut size = None;
    let mut rows = 0usize;
    let mut domain = [[0.0_f64; 3], [1.0_f64; 3]];
    for line in contents.lines() {
        let fields: Vec<_> = line
            .split('#')
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .collect();
        let Some(first) = fields.first() else {
            continue;
        };
        match *first {
            "TITLE" => {}
            "LUT_3D_SIZE" if fields.len() == 2 && size.is_none() => {
                let n = fields[1]
                    .parse::<usize>()
                    .context("Tamanho de LUT inválido")?;
                if !(2..=65).contains(&n) {
                    bail!("LUT 3D deve ter tamanho entre 2 e 65");
                }
                size = Some(n);
            }
            "DOMAIN_MIN" | "DOMAIN_MAX" if fields.len() == 4 => {
                let index = usize::from(*first == "DOMAIN_MAX");
                for (i, value) in fields[1..].iter().enumerate() {
                    domain[index][i] = value.parse()?;
                }
            }
            _ => {
                if fields.len() != 3
                    || fields
                        .iter()
                        .any(|v| v.parse::<f64>().map_or(true, |n| !n.is_finite()))
                {
                    bail!("LUT 3D inválida: linha {line}");
                }
                rows += 1;
            }
        }
    }
    if (0..3).any(|i| {
        !domain[0][i].is_finite() || !domain[1][i].is_finite() || domain[0][i] >= domain[1][i]
    }) {
        bail!("Domínio de LUT inválido");
    }
    let n = size.context("Arquivo sem LUT_3D_SIZE")?;
    if rows != n * n * n {
        bail!(
            "LUT incompleta: esperadas {} cores, encontradas {rows}",
            n * n * n
        );
    }
    Ok(())
}

/// First local frame, also used to reject invalid media before importing it.
pub fn thumbnail(path: &Path) -> Result<Vec<u8>> {
    let path = fs::canonicalize(path).context("Recurso indisponível")?;
    let output = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-max_alloc",
            "134217728",
            "-threads",
            "1",
            "-protocol_whitelist",
            "file,pipe",
            "-format_whitelist",
            "image2,png_pipe,jpeg_pipe,webp_pipe,bmp_pipe,mov,matroska,webm",
            "-i",
        ])
        .arg(path)
        .args([
            "-map",
            "0:v:0",
            "-frames:v",
            "1",
            "-vf",
            "scale=128:72:force_original_aspect_ratio=decrease,pad=128:72:(ow-iw)/2:(oh-ih)/2",
            "-an",
            "-sn",
            "-dn",
            "-threads",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "pipe:1",
        ])
        .stdin(Stdio::null())
        .output()
        .context("Instale FFmpeg para importar imagens e vídeos")?;
    if !output.status.success() || output.stdout.len() != 128 * 72 * 3 {
        bail!("Não foi possível ler a imagem ou o vídeo; escolha outro arquivo local");
    }
    Ok(output.stdout)
}

pub struct Import {
    result: Receiver<Result<Resource, String>>,
}
impl Import {
    pub fn start(root: PathBuf, kind: Kind, source: PathBuf) -> Result<Self> {
        let (tx, result) = mpsc::channel();
        std::thread::Builder::new()
            .name("resource-import".into())
            .spawn(move || {
                let _ = tx.send(import(&root, kind, &source).map_err(|e| format!("{e:#}")));
            })?;
        Ok(Self { result })
    }
    pub fn poll(&self) -> Option<Result<Resource, String>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(_) => Some(Err(
                "Importação interrompida; tente adicionar o arquivo novamente".into(),
            )),
        }
    }
}

fn import(root: &Path, kind: Kind, source: &Path) -> Result<Resource> {
    if !kind.accepts(source) {
        bail!("Formato de arquivo incompatível com este recurso");
    }
    let info = fs::metadata(source).context("Não foi possível abrir o arquivo escolhido")?;
    if !info.is_file() || info.len() == 0 || info.len() > kind.limit() {
        bail!("Arquivo vazio, indisponível ou maior que o limite permitido");
    }
    let filename = source
        .file_name()
        .context("Arquivo sem nome")?
        .to_str()
        .context("Nome de arquivo deve ser UTF-8")?;
    let directory = root.join(kind.directory());
    fs::create_dir_all(&directory)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let staging = directory.join(format!(
        ".import-{}-{stamp}.{}",
        std::process::id(),
        source.extension().unwrap_or_default().to_string_lossy()
    ));
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&staging)?;
    let result = (|| {
        let mut input = fs::File::open(source)?.take(kind.limit() + 1);
        let copied = std::io::copy(&mut input, &mut output)?;
        if copied == 0 || copied > kind.limit() {
            bail!("O arquivo mudou durante a importação ou excedeu o limite");
        }
        output.flush()?;
        if kind == Kind::Lut {
            validate_lut(&staging)?;
        } else {
            thumbnail(&staging)?;
        }
        // Publish the complete validated file atomically, without overwriting anything.
        let mut destination = None;
        for n in 0..1000 {
            let name = if n == 0 {
                filename.to_owned()
            } else {
                format!(
                    "{} ({n}).{}",
                    source.file_stem().unwrap_or_default().to_string_lossy(),
                    source.extension().unwrap_or_default().to_string_lossy()
                )
            };
            let path = directory.join(name);
            match fs::hard_link(&staging, &path) {
                Ok(()) => {
                    destination = Some(path);
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
        let path = destination
            .context("Há muitos arquivos com este nome; renomeie o arquivo e tente novamente")?;
        Ok(Resource {
            kind,
            name: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .replace(['_', '-'], " "),
            path: path.clone(),
            removable: kind == Kind::Image && validate_owned_image(root, &path).is_ok(),
        })
    })();
    let _ = fs::remove_file(&staging);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_removal_only_accepts_regular_imported_copies() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("resources");
        let images = root.join("images");
        fs::create_dir_all(&images).unwrap();
        let original = dir.path().join("original.png");
        fs::write(&original, b"original remains untouched").unwrap();
        let copy = images.join("copy.png");
        fs::hard_link(&original, &copy).unwrap();
        let link = images.join("link.png");
        symlink(&original, &link).unwrap();
        let folder = images.join("folder.png");
        fs::create_dir(&folder).unwrap();
        for path in [
            &original,
            &link,
            &folder,
            &images.join("../images/copy.png"),
        ] {
            assert!(remove_image(&root, path).is_err(), "accepted {path:?}");
        }
        let items = catalog(&root, &Default::default());
        assert!(items.iter().any(|r| r.path == copy && r.removable));
        assert!(!items.iter().any(|r| r.path == link && r.removable));
        remove_image(&root, &copy).unwrap();
        assert!(!copy.exists());
        assert_eq!(fs::read(&original).unwrap(), b"original remains untouched");
        assert!(remove_image(&root, &copy).is_err());
        fs::rename(&images, root.join("saved-images")).unwrap();
        symlink(dir.path(), &images).unwrap();
        assert!(remove_image(&root, &images.join("original.png")).is_err());
        assert!(original.exists());
    }

    #[test]
    #[ignore = "requires FFmpeg; uses synthetic media, no camera"]
    fn ffmpeg_library_imports_images_and_videos_and_rejects_invalid_content() {
        let dir = tempfile::tempdir().unwrap();
        for ext in ["png", "jpg", "webp", "bmp", "mp4", "mov", "webm", "mkv"] {
            let source = dir.path().join(format!("own '{ext}, resource.{ext}"));
            assert!(
                Command::new("ffmpeg")
                    .args([
                        "-hide_banner",
                        "-loglevel",
                        "error",
                        "-f",
                        "lavfi",
                        "-i",
                        "color=c=red:s=64x48:r=2",
                        "-frames:v",
                        "1",
                        "-threads",
                        "1"
                    ])
                    .arg(&source)
                    .status()
                    .unwrap()
                    .success()
            );
            let kind = if matches!(ext, "png" | "jpg" | "webp" | "bmp") {
                Kind::Image
            } else {
                Kind::Video
            };
            let item = import(&dir.path().join("library"), kind, &source).unwrap();
            fs::remove_file(source).unwrap();
            let rgb = thumbnail(&item.path).unwrap();
            let middle = (36 * 128 + 64) * 3;
            assert!(rgb[middle] > 200 && rgb[middle + 1] < 30 && rgb[middle + 2] < 30);
        }
        let invalid = dir.path().join("invalid.mp4");
        fs::write(&invalid, b"not video").unwrap();
        assert!(import(&dir.path().join("invalid-library"), Kind::Video, &invalid).is_err());
        assert_eq!(
            fs::read_dir(dir.path().join("invalid-library/videos"))
                .unwrap()
                .count(),
            0
        );
    }
    #[test]
    fn own_looks_import_and_discovery_survive_original_removal_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("resources");
        let source = dir.path().join("my look.cube");
        fs::write(&source, look(0)).unwrap();
        let first = import(&root, Kind::Lut, &source).unwrap();
        let second = import(&root, Kind::Lut, &source).unwrap();
        assert_ne!(first.path, second.path);
        fs::remove_file(source).unwrap();
        validate_lut(&first.path).unwrap();
        let items = catalog(&root, &Default::default());
        assert!(items.iter().any(|r| r.path == first.path));
        for resource in items
            .iter()
            .filter(|r| r.path.parent().unwrap().ends_with("builtins-v1"))
        {
            assert!(!resource.path.exists());
            materialize(&root, resource.path.to_str().unwrap()).unwrap();
            validate_lut(&resource.path).unwrap();
        }
    }
    #[test]
    fn lut_values_accept_negative_zero_positive_but_reject_non_finite_and_invalid_grids() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("look.cube");
        for value in ["-0.1", "0", "1.1", "NaN", "inf"] {
            fs::write(
                &source,
                format!("LUT_3D_SIZE 2\n{}", format!("{value} 0 1\n").repeat(8)),
            )
            .unwrap();
            assert_eq!(
                validate_lut(&source).is_ok(),
                !matches!(value, "NaN" | "inf")
            );
        }
        for text in [
            "LUT_3D_SIZE -1\n",
            "LUT_3D_SIZE 0\n",
            "LUT_3D_SIZE 66\n",
            "LUT_3D_SIZE 2\n0 0 0\n",
            "LUT_1D_SIZE 2\n",
            "LUT_3D_SIZE 2\nDOMAIN_MIN 0 0 0\nDOMAIN_MAX 0 1 1\n",
        ] {
            fs::write(&source, text).unwrap();
            assert!(import(dir.path(), Kind::Lut, &source).is_err());
            assert_eq!(fs::read_dir(dir.path().join("luts")).unwrap().count(), 0);
        }
    }
}
