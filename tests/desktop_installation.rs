//! Real installer/launcher/remover contracts, isolated from the user's desktop.
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
const BIN: &str = env!("CARGO_BIN_EXE_elgato-virtualcam");

fn command(root: &Path, program: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_RUNTIME_DIR", root)
        .env("XDG_CACHE_HOME", root.join("cache"));
    command
}
fn succeeded(output: Output) -> Output {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn desktop_installation_preserves_data_and_removes_orphaned_autostart() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let prefix = root.join("install ç % $ ` \\");
    let launcher = prefix.join("bin/elgato-virtualcam");
    let source_info = succeeded(Command::new(BIN).arg("--build-info").output().unwrap());
    let info: serde_json::Value = serde_json::from_slice(&source_info.stdout).unwrap();
    if !cfg!(feature = "gui") {
        assert_eq!(info["gui"], false);
        assert!(
            !command(root, Path::new(BIN))
                .arg("--install-desktop")
                .arg(&prefix)
                .output()
                .unwrap()
                .status
                .success()
        );
        assert!(!prefix.exists());
        return;
    }
    // Exercise the public script without rebuilding or touching the real prefix.
    let target = root.join("target");
    fs::create_dir_all(target.join("release")).unwrap();
    std::os::unix::fs::symlink(BIN, target.join("release/elgato-virtualcam")).unwrap();
    let install = Path::new(env!("CARGO_MANIFEST_DIR")).join("install-desktop-app.sh");
    succeeded(
        command(root, &install)
            .env("CARGO_TARGET_DIR", &target)
            .args(["--no-build", "--autostart", "--prefix"])
            .arg(&prefix)
            .output()
            .unwrap(),
    );
    let entry = root.join("data/applications/elgato-virtualcam.desktop");
    let autostart = root.join("config/autostart/elgato-virtualcam.desktop");
    assert!(autostart.is_file());
    assert!(
        fs::read_to_string(&entry)
            .unwrap()
            .contains("Icon=facecam-hub\n")
    );
    if Command::new("desktop-file-validate")
        .arg("--version")
        .output()
        .is_ok()
    {
        succeeded(
            Command::new("desktop-file-validate")
                .arg(&entry)
                .output()
                .unwrap(),
        );
        succeeded(
            Command::new("desktop-file-validate")
                .arg(&autostart)
                .output()
                .unwrap(),
        );
    }
    let first = launcher.canonicalize().unwrap();
    let resources = first.parent().unwrap().join("resources");
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(resources.join("provenance.json")).unwrap()).unwrap();
    let media = manifest["files"].as_array().unwrap();
    assert_eq!(media.len(), 22);
    for item in media {
        let relative = item["path"].as_str().unwrap();
        assert_eq!(
            fs::read(resources.join(relative)).unwrap(),
            fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("assets/backgrounds")
                    .join(relative)
            )
            .unwrap(),
        );
    }
    assert_eq!(
        succeeded(
            command(root, &launcher)
                .arg("--build-info")
                .output()
                .unwrap()
        )
        .stdout,
        source_info.stdout
    );
    let personal = root.join("config/facecam-hub/config.json");
    let model = root.join("data/facecam-hub/ai/model.onnx");
    let backup = autostart.with_extension("desktop.backup");
    for path in [&personal, &model, &backup] {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"preserve").unwrap();
    }
    // A running old binary remains executable after an upgrade (no ETXTBSY).
    let mut old = command(root, &launcher)
        .args(["--headless", "--config"])
        .arg(root.join("controller.json"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let start = std::time::Instant::now();
    loop {
        if command(root, &launcher)
            .arg("--status")
            .output()
            .unwrap()
            .status
            .success()
        {
            break;
        }
        assert!(start.elapsed() < std::time::Duration::from_secs(10));
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let upgrade = command(root, &launcher)
        .arg("--install-desktop")
        .arg(&prefix)
        .output()
        .unwrap();
    // Always stop the owned process even if the assertion below fails.
    let _ = command(root, &launcher).arg("--quit").output();
    let _ = old.wait();
    succeeded(upgrade);
    assert_ne!(launcher.canonicalize().unwrap(), first);
    assert!(first.is_file());
    // Upgrading an old install can leave its unmarked autostart unchanged.
    let legacy_exec = launcher
        .to_str()
        .unwrap()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
        .replace('%', "%%");
    fs::write(
        &autostart,
        format!("[Desktop Entry]\nName=Facecam Hub\nExec=\"{legacy_exec}\" --minimized\n"),
    )
    .unwrap();
    // Removal must work even when the stable executable was lost.
    fs::remove_file(&launcher).unwrap();
    let uninstall = Path::new(env!("CARGO_MANIFEST_DIR")).join("uninstall-desktop-app.sh");
    succeeded(
        command(root, &uninstall)
            .arg("--prefix")
            .arg(&prefix)
            .output()
            .unwrap(),
    );
    assert!(!entry.exists());
    assert!(!autostart.exists());
    assert!(!prefix.join("lib/facecam-hub").exists());
    assert!(
        !root
            .join("data/icons/hicolor/256x256/apps/facecam-hub.png")
            .symlink_metadata()
            .is_ok()
    );
    for path in [&personal, &model, &backup] {
        assert_eq!(fs::read(path).unwrap(), b"preserve");
    }
    succeeded(
        command(root, &uninstall)
            .arg("--prefix")
            .arg(&prefix)
            .output()
            .unwrap(),
    );
}

#[cfg(feature = "gui")]
#[test]
fn uninstall_does_not_remove_another_prefix_desktop_entries() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("one");
    let second = dir.path().join("two");
    for prefix in [&first, &second] {
        let mut installer = command(dir.path(), Path::new(BIN));
        installer
            .arg("--install-desktop")
            .arg(prefix)
            .arg("--desktop-autostart");
        // The previous production updater does not pass --desktop-resources.
        if prefix == &first {
            installer
                .arg("--desktop-resources")
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/backgrounds"));
        }
        succeeded(installer.output().unwrap());
    }
    let desktop = dir
        .path()
        .join("data/applications/elgato-virtualcam.desktop");
    let original = fs::read(&desktop).unwrap();
    let uninstall = Path::new(env!("CARGO_MANIFEST_DIR")).join("uninstall-desktop-app.sh");
    succeeded(
        command(dir.path(), &uninstall)
            .arg("--prefix")
            .arg(&first)
            .output()
            .unwrap(),
    );
    assert_eq!(fs::read(&desktop).unwrap(), original);
    assert!(
        dir.path()
            .join("config/autostart/elgato-virtualcam.desktop")
            .is_file()
    );
    assert!(
        dir.path()
            .join("data/icons/hicolor/256x256/apps/facecam-hub.png")
            .exists()
    );
    assert!(second.join("bin/elgato-virtualcam").exists());
}
