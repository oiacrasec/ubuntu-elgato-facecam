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

pub fn autostart_enabled() -> Result<bool> {
    let text = match fs::read_to_string(autostart_path()?) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error).context("Não foi possível consultar o início automático"),
    };
    let mut desktop_entry = false;
    let mut found = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            desktop_entry = line == "[Desktop Entry]";
            found |= desktop_entry;
        } else if desktop_entry
            && let Some((key, value)) = line.split_once('=')
            && matches!(
                (key.trim(), value.trim()),
                ("Hidden", "true") | ("X-GNOME-Autostart-enabled", "false")
            )
        {
            return Ok(false);
        }
    }
    Ok(found)
}

pub fn remove_autostart() -> Result<()> {
    match fs::remove_file(autostart_path()?) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("Não foi possível desativar o início automático"),
    }
}
