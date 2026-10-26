//! User-local packages. A complete immutable bundle is published by one symlink.
use anyhow::{Context, Result, bail};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};

pub const REPOSITORY: &str = "https://github.com/oiacrasec/ubuntu-elgato-facecam.git";
pub const ICON: &str = "facecam-hub";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildInfo {
    pub application: String,
    pub version: String,
    pub revision: String,
    pub dirty: bool,
    pub gui: bool,
    pub tray: bool,
    pub ai: bool,
}

impl Default for BuildInfo {
    fn default() -> Self {
        Self {
            application: "facecam-hub".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            revision: env!("FACECAM_REVISION").into(),
            dirty: env!("FACECAM_DIRTY") == "true",
            gui: cfg!(feature = "gui"),
            tray: cfg!(feature = "tray"),
            ai: cfg!(feature = "ai"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Installed {
    pub format: u32,
    pub prefix: PathBuf,
    pub data_home: PathBuf,
    pub build: BuildInfo,
    pub binary_sha256: String,
}

impl Installed {
    pub fn root(&self) -> PathBuf {
        self.prefix.join("lib/facecam-hub")
    }
    pub fn launcher(&self) -> PathBuf {
        self.prefix.join("bin/elgato-virtualcam")
    }
    pub fn discover() -> Result<Option<Self>> {
        Self::for_executable(&std::env::current_exe()?)
    }
    pub fn for_executable(executable: &Path) -> Result<Option<Self>> {
        let parent = executable.parent().context("Executável sem diretório")?;
        let path = parent.join("installation.json");
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e).context("Não foi possível ler o registro de instalação"),
        };
        if bytes.len() > 16 * 1024 {
            bail!("Registro de instalação inválido");
        }
        let installed: Self =
            serde_json::from_slice(&bytes).context("Registro de instalação inválido")?;
        if installed.format != 1
            || installed.build.application != "facecam-hub"
            || !installed.prefix.is_absolute()
            || installed.prefix.parent().is_none()
            || parent.parent() != Some(installed.root().join("versions").as_path())
        {
            bail!("Registro de instalação incompatível com o executável");
        }
        Ok(Some(installed))
    }
}

pub fn checksum(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub fn install_id(prefix: &Path) -> String {
    format!(
        "{:x}",
        Sha256::digest(prefix.as_os_str().as_encoded_bytes())
    )
}

pub fn exec_quote(path: &Path) -> Result<String> {
    let text = path
        .to_str()
        .context("O caminho de instalação precisa usar UTF-8")?;
    if text.chars().any(char::is_control) {
        bail!("Caminho de instalação inválido");
    }
    // Escape the Exec argument, then the Desktop Entry string value itself.
    let argument = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('%', "%%");
    Ok(format!("\"{}\"", argument.replace('\\', "\\\\")))
}

pub fn desktop_entry(launcher: &Path, prefix: &Path, autostart: bool) -> Result<String> {
    let exec = exec_quote(launcher)?;
    Ok(format!(
        "[Desktop Entry]\nType=Application\nName=Facecam Hub\nComment=Controle da Facecam e câmera virtual\nExec={exec}{}\nIcon={ICON}\nTerminal=false\nCategories=AudioVideo;Video;\nStartupWMClass=elgato-virtualcam\nStartupNotify={}\nX-Facecam-Hub-Install-Id={}\n{}",
        if autostart { " --minimized" } else { "" },
        !autostart,
        install_id(prefix),
        if autostart {
            "X-GNOME-Autostart-enabled=true\n"
        } else {
            ""
        }
    ))
}

pub fn data_home() -> Result<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
        .context("HOME não definido")
}

enum Original {
    Missing,
    File(Vec<u8>, fs::Permissions),
    Link(PathBuf),
}
struct Changes {
    originals: Vec<(PathBuf, Original)>,
    committed: bool,
}
impl Changes {
    fn new() -> Self {
        Self {
            originals: Vec::new(),
            committed: false,
        }
    }
    fn remember(&mut self, path: &Path) -> Result<()> {
        let original = match fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_symlink() => Original::Link(fs::read_link(path)?),
            Ok(meta) if meta.is_file() && meta.len() <= 128 * 1024 * 1024 => {
                Original::File(fs::read(path)?, meta.permissions())
            }
            Ok(_) => bail!("Destino de instalação ocupado: {}", path.display()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Original::Missing,
            Err(e) => return Err(e.into()),
        };
        self.originals.push((path.into(), original));
        Ok(())
    }
    fn file(&mut self, path: &Path, bytes: &[u8]) -> Result<()> {
        let parent = path.parent().context("Destino sem diretório")?;
        fs::create_dir_all(parent)?;
        self.remember(path)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        use std::io::Write;
        temp.write_all(bytes)?;
        temp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o644))?;
        temp.persist(path).map_err(|e| e.error)?;
        Ok(())
    }
    fn link(&mut self, path: &Path, target: &Path) -> Result<()> {
        let parent = path.parent().context("Destino sem diretório")?;
        fs::create_dir_all(parent)?;
        self.remember(path)?;
        let temp = tempfile::tempdir_in(parent)?;
        let link = temp.path().join("link");
        symlink(target, &link)?;
        fs::rename(link, path)?;
        Ok(())
    }
}
impl Drop for Changes {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        for (path, original) in self.originals.iter().rev() {
            let _ = fs::remove_file(path);
            let restored = match original {
                Original::Missing => Ok(()),
                Original::Link(target) => symlink(target, path),
                Original::File(bytes, permissions) => fs::write(path, bytes)
                    .and_then(|_| fs::set_permissions(path, permissions.clone())),
            };
            if let Err(e) = restored {
                eprintln!("Não foi possível restaurar {}: {e}", path.display());
            }
        }
    }
}

pub fn install(
    binary: &Path,
    build: &BuildInfo,
    prefix: &Path,
    data: &Path,
    autostart: bool,
) -> Result<Installed> {
    if !build.gui {
        bail!("O instalador desktop requer uma compilação com interface gráfica");
    }
    for path in [prefix, data] {
        if !path.is_absolute()
            || path.parent().is_none()
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            bail!("Use um diretório de instalação absoluto e dedicado");
        }
        exec_quote(path)?;
    }
    fs::create_dir_all(prefix)?;
    fs::create_dir_all(data)?;
    let prefix = prefix.canonicalize()?;
    let data = data.canonicalize()?;
    let root = prefix.join("lib/facecam-hub");
    let owner = format!(
        "facecam-hub-v1\n{}\n{}\n",
        install_id(&prefix),
        prefix.display()
    );
    if root.exists() && !fs::read_to_string(root.join("owner")).is_ok_and(|text| text == owner) {
        bail!("O diretório de instalação já contém arquivos sem registro do Facecam Hub");
    }
    fs::create_dir_all(&root)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(root.join("install.lock"))?;
    lock.try_lock_exclusive()
        .context("Outra instalação está em andamento")?;
    fs::write(root.join("owner"), &owner)?;
    let versions = root.join("versions");
    fs::create_dir_all(&versions)?;
    let hash = checksum(binary)?;
    let bundle = tempfile::Builder::new()
        .prefix(&format!("v-{}-", &hash[..16]))
        .tempdir_in(&versions)?;
    fs::copy(binary, bundle.path().join("elgato-virtualcam"))?;
    fs::set_permissions(
        bundle.path().join("elgato-virtualcam"),
        fs::Permissions::from_mode(0o755),
    )?;
    fs::create_dir(bundle.path().join("locales"))?;
    fs::write(
        bundle.path().join("locales/pt-BR.json"),
        include_bytes!("../locales/pt-BR.json"),
    )?;
    fs::write(
        bundle.path().join("locales/en-US.json"),
        include_bytes!("../locales/en-US.json"),
    )?;
    fs::create_dir(bundle.path().join("icons"))?;
    for (name, bytes) in [
        (
            "facecam-hub.png",
            include_bytes!("../assets/icons/facecam-hub.png").as_slice(),
        ),
        (
            "facecam-hub-tray.png",
            include_bytes!("../assets/icons/facecam-hub-tray.png").as_slice(),
        ),
        (
            "facecam-hub-tray-update.png",
            include_bytes!("../assets/icons/facecam-hub-tray-update.png").as_slice(),
        ),
        (
            "README.md",
            include_bytes!("../assets/README.md").as_slice(),
        ),
        (
            "provenance.json",
            include_bytes!("../assets/icons/provenance.json").as_slice(),
        ),
    ] {
        fs::write(bundle.path().join("icons").join(name), bytes)?;
    }
    let installed = Installed {
        format: 1,
        prefix: prefix.clone(),
        data_home: data.clone(),
        build: build.clone(),
        binary_sha256: hash,
    };
    fs::write(
        bundle.path().join("installation.json"),
        serde_json::to_vec_pretty(&installed)?,
    )?;
    let launcher = installed.launcher();
    let mut changes = Changes::new();
    changes.file(
        &root.join("data-home"),
        data.to_str()
            .context("Diretório de dados inválido")?
            .as_bytes(),
    )?;
    changes.link(&launcher, &root.join("current/elgato-virtualcam"))?;
    for (size, category, name) in [
        ("256x256", "apps", "facecam-hub.png"),
        ("64x64", "status", "facecam-hub-tray.png"),
        ("64x64", "status", "facecam-hub-tray-update.png"),
    ] {
        changes.link(
            &data
                .join("icons/hicolor")
                .join(size)
                .join(category)
                .join(name),
            &root.join("current/icons").join(name),
        )?;
    }
    // Remove only links from the previous installer: a stale scalable icon
    // otherwise takes priority over the original PNG in theme lookup.
    for (category, name) in [
        ("apps", "facecam-hub.svg"),
        ("status", "facecam-hub-symbolic.svg"),
        ("status", "facecam-hub-active-symbolic.svg"),
    ] {
        let link = data
            .join("icons/hicolor/scalable")
            .join(category)
            .join(name);
        if fs::read_link(&link).is_ok_and(|target| target == root.join("current/icons").join(name))
        {
            changes.remember(&link)?;
            fs::remove_file(&link)?;
        }
    }
    changes.file(
        &data.join("applications/elgato-virtualcam.desktop"),
        desktop_entry(&launcher, &prefix, false)?.as_bytes(),
    )?;
    if autostart {
        let path = crate::desktop::autostart_path()?;
        let backup = path.with_extension("desktop.backup");
        if path.is_file() && !backup.exists() {
            changes.file(&backup, &fs::read(&path)?)?;
        }
        changes.file(
            &crate::desktop::autostart_path()?,
            desktop_entry(&launcher, &prefix, true)?.as_bytes(),
        )?;
    }
    changes.link(&root.join("current"), bundle.path())?;
    let _ = bundle.keep();
    changes.committed = true;
    // Integration caches are optional. Publication is already complete; a
    // missing desktop utility does not invalidate an installed package.
    for (tool, arguments, directory) in [
        (
            "gtk-update-icon-cache",
            &["--force", "--ignore-theme-index"][..],
            data.join("icons/hicolor"),
        ),
        (
            "update-desktop-database",
            &[][..],
            data.join("applications"),
        ),
    ] {
        let _ = std::process::Command::new(tool)
            .args(arguments)
            .arg(directory)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    Ok(installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build() -> BuildInfo {
        BuildInfo {
            gui: true,
            ..Default::default()
        }
    }

    #[test]
    fn upgrade_keeps_running_bundle_and_restores_entries_after_publish_failure() {
        let dir = tempfile::tempdir().unwrap();
        let prefix = dir.path().join("prefix with spaces % $ ` ç");
        let data = dir.path().join("data");
        let source = dir.path().join("binary");
        fs::write(&source, b"old executable").unwrap();
        let installed = install(&source, &build(), &prefix, &data, false).unwrap();
        let first = installed.launcher().canonicalize().unwrap();
        let legacy = data.join("icons/hicolor/scalable/apps/facecam-hub.svg");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        let legacy_target = installed.root().join("current/icons/facecam-hub.svg");
        symlink(&legacy_target, &legacy).unwrap();
        fs::write(&source, b"new executable").unwrap();
        install(&source, &build(), &prefix, &data, false).unwrap();
        assert!(legacy.symlink_metadata().is_err());
        assert!(
            data.join("icons/hicolor/256x256/apps/facecam-hub.png")
                .is_file()
        );
        assert_eq!(fs::read(&first).unwrap(), b"old executable");
        assert_eq!(fs::read(installed.launcher()).unwrap(), b"new executable");
        let second = installed.launcher().canonicalize().unwrap();
        let metadata = Installed::for_executable(&second).unwrap().unwrap();
        assert_eq!(checksum(&second).unwrap(), metadata.binary_sha256);
        assert!(
            second
                .parent()
                .unwrap()
                .join("locales/en-US.json")
                .is_file()
        );
        let entry = data.join("applications/elgato-virtualcam.desktop");
        assert!(
            fs::read_to_string(&entry)
                .unwrap()
                .contains("Icon=facecam-hub")
        );
        // A directory where the final symlink belongs must leave the prior
        // desktop/launcher intact, including their original file permissions.
        fs::remove_file(installed.root().join("current")).unwrap();
        fs::create_dir(installed.root().join("current")).unwrap();
        let original_entry = fs::read(&entry).unwrap();
        symlink(&legacy_target, &legacy).unwrap();
        fs::write(&source, b"failed upgrade").unwrap();
        assert!(install(&source, &build(), &prefix, &data, false).is_err());
        assert_eq!(fs::read_link(&legacy).unwrap(), legacy_target);
        assert_eq!(fs::read(&entry).unwrap(), original_entry);
        assert_eq!(fs::read(&second).unwrap(), b"new executable");
        fs::remove_dir(installed.root().join("current")).unwrap();
        symlink(second.parent().unwrap(), installed.root().join("current")).unwrap();
        assert_eq!(fs::read(installed.launcher()).unwrap(), b"new executable");
    }

    #[test]
    fn headless_and_unowned_destinations_are_rejected_without_overwriting_files() {
        let dir = tempfile::tempdir().unwrap();
        let prefix = dir.path().join("prefix");
        let data = dir.path().join("data");
        let source = dir.path().join("binary");
        fs::write(&source, b"binary").unwrap();
        assert!(
            install(
                &source,
                &BuildInfo {
                    gui: false,
                    ..Default::default()
                },
                &prefix,
                &data,
                false
            )
            .is_err()
        );
        assert!(!prefix.exists());
        fs::create_dir_all(prefix.join("lib/facecam-hub")).unwrap();
        fs::write(prefix.join("lib/facecam-hub/keep"), b"personal").unwrap();
        assert!(install(&source, &build(), &prefix, &data, false).is_err());
        assert_eq!(
            fs::read(prefix.join("lib/facecam-hub/keep")).unwrap(),
            b"personal"
        );
        assert!(exec_quote(Path::new("/tmp/a\nb")).is_err());
    }
}
