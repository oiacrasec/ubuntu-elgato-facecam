use anyhow::{Context, Result};
use std::{fs, path::PathBuf};

pub fn autostart_path() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .context("HOME não definido")?;
    Ok(base.join("autostart/elgato-virtualcam.desktop"))
}

pub fn install_autostart() -> Result<()> {
    let path = autostart_path()?;
    let executable = std::env::current_exe()?
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('%', "%%");
    fs::create_dir_all(path.parent().context("Autostart sem diretório")?)?;
    if path.exists() && !path.with_extension("desktop.backup").exists() {
        fs::copy(&path, path.with_extension("desktop.backup"))?;
    }
    fs::write(
        &path,
        format!(
            "[Desktop Entry]\nType=Application\nName=Facecam Hub\nComment=Facecam controller and virtual camera\nExec=\"{executable}\" --minimized\nIcon=camera-web\nTerminal=false\nStartupNotify=false\nX-GNOME-Autostart-enabled=true\n"
        ),
    )?;
    println!("{}", path.display());
    Ok(())
}
