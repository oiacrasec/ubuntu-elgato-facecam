//! Guided setup of a system-provided loopback module; never unload another app's device.
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

const OPTIONS_FILE: &str = "/etc/modprobe.d/facecam-hub.conf";
const MODULES_FILE: &str = "/etc/modules-load.d/facecam-hub.conf";
const MARKER: &str = "# Managed by Facecam Hub\n";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Status {
    pub kernel: String,
    pub module_available: bool,
    pub module_loaded: bool,
    pub module_version: Option<String>,
    pub module_signer: Option<String>,
    pub secure_boot: Option<bool>,
}

pub fn discover() -> Result<Status> {
    let kernel = fs::read_to_string("/proc/sys/kernel/osrelease")?
        .trim()
        .to_owned();
    let field = |name| -> Option<String> {
        let output = Command::new("/usr/sbin/modinfo")
            .args(["-F", name, "v4l2loopback"])
            .output()
            .ok()?;
        let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        (output.status.success() && !value.is_empty()).then_some(value)
    };
    let secure_boot = fs::read_dir("/sys/firmware/efi/efivars")
        .ok()
        .and_then(|entries| {
            entries.filter_map(Result::ok).find_map(|entry| {
                if !entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("SecureBoot-")
                {
                    return None;
                }
                fs::read(entry.path())
                    .ok()
                    .and_then(|b| b.get(4).map(|v| *v != 0))
            })
        });
    Ok(Status {
        kernel,
        module_available: field("filename").is_some(),
        module_loaded: Path::new("/sys/module/v4l2loopback").exists(),
        module_version: field("version"),
        module_signer: field("signer"),
        secure_boot,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    pub device: String,
    pub kernel: String,
    pub persistent: bool,
    pub package: Option<String>,
}

impl Setup {
    fn number(&self) -> Result<u32> {
        let number = self
            .device
            .strip_prefix("/dev/video")
            .context("Saída virtual inválida")?
            .parse::<u32>()?;
        if !(2..=255).contains(&number) || self.device != format!("/dev/video{number}") {
            bail!("Número de saída virtual fora do intervalo permitido");
        }
        Ok(number)
    }
    fn files(&self) -> Result<[(&'static str, String); 2]> {
        Ok([
            (
                OPTIONS_FILE,
                format!(
                    "{MARKER}options v4l2loopback video_nr={} card_label=\"{}\" exclusive_caps=1\n",
                    self.number()?,
                    crate::config::VIRTUAL_CAMERA_LABEL
                ),
            ),
            (MODULES_FILE, format!("{MARKER}v4l2loopback\n")),
        ])
    }
    fn validate_inputs(&self) -> Result<()> {
        self.number()?;
        if self.kernel.is_empty()
            || !self
                .kernel
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ".+-_".contains(c))
        {
            bail!("Identificador de kernel inválido");
        }
        if let Some(package) = &self.package
            && package != &format!("linux-main-modules-v4l2loopback-{}", self.kernel)
            && package != &format!("linux-modules-extra-{}", self.kernel)
        {
            bail!("Pacote não permitido para a preparação da câmera virtual");
        }
        Ok(())
    }
    pub fn validate_current(&self) -> Result<()> {
        self.validate_inputs()?;
        let kernel = fs::read_to_string("/proc/sys/kernel/osrelease")?;
        if kernel.trim() != self.kernel {
            bail!("O kernel mudou; prepare e confirme um novo plano");
        }
        if Path::new(&self.device).exists()
            || Path::new(&format!("/sys/class/video4linux/video{}", self.number()?)).exists()
            || Path::new("/sys/module/v4l2loopback").exists()
        {
            bail!(
                "Os dispositivos mudaram desde a preparação. Atualize e prepare um novo plano; nenhum módulo será descarregado"
            );
        }
        if self.persistent {
            for (target, _) in self.files()? {
                check_owned_file(target)?;
            }
        }
        Ok(())
    }
    pub fn verify_result(&self) -> Result<String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if crate::stream::verify_virtual_device(&self.device).is_ok() {
                if self.persistent {
                    for (target, expected) in self.files()? {
                        if fs::read_to_string(target)? != expected {
                            bail!(
                                "Saída disponível nesta sessão, mas a configuração para reinício não foi confirmada"
                            );
                        }
                    }
                }
                return Ok(format!(
                    "Câmera virtual preparada em {}; inicie a saída antes de abrir o consumidor",
                    self.device
                ));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        bail!(
            "O módulo não criou uma saída acessível. Verifique o log, as permissões e a assinatura exigida pelo Secure Boot"
        )
    }
}

fn check_owned_file(target: &str) -> Result<()> {
    match fs::read_to_string(target) {
        Ok(text) if !text.starts_with(MARKER) => {
            bail!("{target} já existe e não pertence ao app; revise a configuração no Ubuntu")
        }
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error.into()),
        _ => {}
    }
    Ok(())
}

impl Setup {
    /// Descriptive steps also executed by the fixed privileged helper below.
    pub fn review_steps(&self) -> Result<Vec<Vec<String>>> {
        self.validate_inputs()?;
        let mut steps: Vec<Vec<String>> = Vec::new();
        if let Some(package) = &self.package {
            steps.push(vec!["/usr/bin/apt-get".into(), "update".into()]);
            steps.push(vec![
                "/usr/bin/apt-get".into(),
                "install".into(),
                "--yes".into(),
                "--no-install-recommends".into(),
                "--".into(),
                package.clone(),
            ]);
        }
        steps.push(vec![
            "/usr/sbin/modprobe".into(),
            "v4l2loopback".into(),
            format!("video_nr={}", self.number()?),
            format!("card_label={}", crate::config::VIRTUAL_CAMERA_LABEL),
            "exclusive_caps=1".into(),
        ]);
        if self.persistent {
            for (target, text) in self.files()? {
                steps.push(vec!["gravar arquivo".into(), target.into(), text]);
            }
            steps.push(vec![
                "/usr/sbin/update-initramfs".into(),
                "-u".into(),
                "-k".into(),
                self.kernel.clone(),
            ]);
        }
        Ok(steps)
    }
}

/// Runs before config, IPC or GUI initialization. No arbitrary commands or destinations.
pub fn run_privileged(payload: &str) -> Result<()> {
    if !nix::unistd::Uid::effective().is_root() {
        bail!("A preparação administrativa exige autenticação pelo Ubuntu");
    }
    let setup: Setup = serde_json::from_str(payload)?;
    setup.validate_current()?;
    for step in setup.review_steps()? {
        let (program, arguments) = step.split_first().context("Etapa vazia")?;
        println!("Executando: {step:?}");
        if program == "gravar arquivo" {
            // Paths and contents are generated here, never loaded from a user script.
            let target = Path::new(&arguments[0]);
            check_owned_file(&arguments[0])?;
            let temporary = target.with_extension(format!("{}.tmp", std::process::id()));
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o644)
                .open(&temporary)?;
            let result = (|| -> Result<()> {
                file.write_all(arguments[1].as_bytes())?;
                file.sync_all()?;
                fs::rename(&temporary, target)?;
                Ok(())
            })();
            if result.is_err() {
                let _ = fs::remove_file(&temporary);
            }
            result?;
        } else {
            if program == "/usr/sbin/modprobe" {
                setup.validate_current()?;
            }
            let status = Command::new(program)
                .args(arguments)
                .env_clear()
                .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
                .env("LC_ALL", "C")
                .stdin(std::process::Stdio::null())
                .status()?;
            if !status.success() {
                bail!("Preparação interrompida em {program}: {status}");
            }
        }
    }
    Ok(())
}

pub fn prepare(
    id: u64,
    directory: &Path,
    persistent: bool,
) -> Result<crate::dependencies::InstallPlan> {
    let status = discover()?;
    let occupied: Vec<_> = (2..=255)
        .filter(|n| {
            Path::new(&format!("/dev/video{n}")).exists()
                || Path::new(&format!("/sys/class/video4linux/video{n}")).exists()
        })
        .collect();
    build_plan(id, directory, persistent, &status, &occupied, |package| {
        let result = Command::new("/usr/bin/apt-cache")
            .args(["policy", package])
            .env("LC_ALL", "C")
            .output()?;
        Ok(result.status.success()
            && String::from_utf8_lossy(&result.stdout).lines().any(|s| {
                s.trim()
                    .strip_prefix("Candidate:")
                    .is_some_and(|v| !v.trim().is_empty() && v.trim() != "(none)")
            }))
    })
}

fn build_plan(
    id: u64,
    directory: &Path,
    persistent: bool,
    status: &Status,
    occupied: &[u32],
    mut candidate: impl FnMut(&str) -> Result<bool>,
) -> Result<crate::dependencies::InstallPlan> {
    if status.module_loaded {
        bail!(
            "v4l2loopback já está carregado. Atualize os dispositivos; o app não descarrega módulos de outros aplicativos"
        );
    }
    if !status
        .kernel
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || ".+-_".contains(c))
        || status.kernel.is_empty()
    {
        bail!("Identificador de kernel inválido");
    }
    let number = (10..=255)
        .find(|n| !occupied.contains(n))
        .context("Nenhum número livre para a câmera virtual")?;
    let mut setup = Setup {
        device: format!("/dev/video{number}"),
        kernel: status.kernel.clone(),
        persistent,
        package: None,
    };
    if !status.module_available {
        let preferred = format!("linux-main-modules-v4l2loopback-{}", status.kernel);
        let extra = format!("linux-modules-extra-{}", status.kernel);
        let package = if candidate(&preferred)? {
            preferred
        } else if candidate(&extra)? {
            extra
        } else {
            bail!(
                "Não há módulo pré-compilado para este kernel nos repositórios. Prepare v4l2loopback no Ubuntu e atualize os dispositivos"
            );
        };
        setup.package = Some(package);
    }
    let commands = vec![vec![
        "/usr/bin/pkexec".into(),
        "--disable-internal-agent".into(),
        std::env::current_exe()?
            .to_str()
            .context("Binário deve ter caminho UTF-8")?
            .into(),
        "--virtual-camera-helper".into(),
        serde_json::to_string(&setup)?,
    ]];
    let mut explanation = format!(
        "Prepara Elgato Facecam Virtual em {}, com compatibilidade para navegadores. Uma autorização do Ubuntu executa as etapas abaixo. Aproveita o módulo do kernel quando disponível; caso necessário, instala o pacote oficial do kernel por APT. Não altera a câmera física nem descarrega módulos.",
        setup.device
    );
    if persistent {
        explanation.push_str(" Cadastra os dois arquivos facecam-hub.conf exibidos no plano para disponibilizar a saída após reiniciar o Ubuntu.");
    }
    if status.secure_boot == Some(true) {
        explanation.push_str(" Secure Boot está ativo: o Ubuntu só carregará um módulo aceito pela política de assinatura.");
    }
    Ok(crate::dependencies::InstallPlan {
        id,
        title: "Preparar câmera virtual".into(),
        explanation,
        commands,
        log: directory.join(format!("virtual-camera-{id}.log")),
        verified_file: None,
        sha256: None,
        bootstrap: None,
        repository: None,
        dependency: crate::dependencies::Dependency::VirtualCamera,
        action: crate::dependencies::Action::Install,
        removals: Vec::new(),
        virtual_camera: Some(setup),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn status() -> Status {
        Status {
            kernel: "7.0.0-38-generic".into(),
            module_available: true,
            ..Default::default()
        }
    }
    #[test]
    fn native_module_plan_avoids_dkms_and_does_not_write_before_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        let plan = build_plan(1, dir.path(), true, &status(), &[0, 1, 10], |_| {
            panic!("should not query APT when module exists")
        })
        .unwrap();
        assert_eq!(plan.virtual_camera.as_ref().unwrap().device, "/dev/video11");
        assert_eq!(plan.commands.len(), 1);
        let setup = plan.virtual_camera.as_ref().unwrap();
        assert_eq!(setup.review_steps().unwrap().len(), 4);
        assert!(
            setup.review_steps().unwrap()[0].contains(&"card_label=Elgato Facecam Virtual".into())
        );
        assert!(
            setup.files().unwrap()[0]
                .1
                .contains("card_label=\"Elgato Facecam Virtual\"")
        );
        assert!(
            setup.review_steps().unwrap()[0]
                .iter()
                .any(|v| v == "video_nr=11")
        );
        assert_eq!(plan.commands[0][3], "--virtual-camera-helper");
        let payload: Setup = serde_json::from_str(&plan.commands[0][4]).unwrap();
        assert_eq!(
            payload.review_steps().unwrap(),
            setup.review_steps().unwrap()
        );
        assert!(
            plan.commands
                .iter()
                .all(|args| !args.iter().any(|v| v.contains("dkms") || v == "-r"))
        );
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
    #[test]
    fn missing_module_uses_matching_kernel_package_or_returns_an_actionable_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = Status {
            module_available: false,
            ..status()
        };
        let plan = build_plan(1, dir.path(), false, &missing, &[], |p| {
            assert_eq!(p, "linux-main-modules-v4l2loopback-7.0.0-38-generic");
            Ok(true)
        })
        .unwrap();
        assert_eq!(plan.commands.len(), 1);
        let steps = plan
            .virtual_camera
            .as_ref()
            .unwrap()
            .review_steps()
            .unwrap();
        assert_eq!(steps.len(), 3);
        assert!(
            steps[1]
                .iter()
                .any(|v| v == "linux-main-modules-v4l2loopback-7.0.0-38-generic")
        );
        assert!(build_plan(1, dir.path(), false, &missing, &[], |_| Ok(false)).is_err());
        assert!(
            build_plan(
                1,
                dir.path(),
                false,
                &Status {
                    module_loaded: true,
                    ..status()
                },
                &[],
                |_| Ok(true)
            )
            .is_err()
        );
    }

    #[test]
    fn helper_rejects_invalid_devices_kernels_packages_and_extra_fields() {
        let mut setup = Setup {
            device: "/dev/video10".into(),
            kernel: "fixture".into(),
            persistent: true,
            package: None,
        };
        for device in [
            "/dev/video0",
            "/dev/video-1",
            "/dev/video256",
            "/dev/video+10",
            "/dev/video010",
            "/dev/video10;other",
            "/tmp/video10",
        ] {
            setup.device = device.into();
            assert!(setup.review_steps().is_err());
        }
        setup.device = "/dev/video10".into();
        setup.kernel = "kernel;other".into();
        assert!(setup.review_steps().is_err());
        setup.kernel = "7.0.0-38-generic".into();
        setup.package = Some("other-package".into());
        assert!(setup.review_steps().is_err());
        let mut payload = serde_json::to_value(&setup).unwrap();
        payload["commands"] = serde_json::json!(["other"]);
        assert!(serde_json::from_value::<Setup>(payload).is_err());
    }
}
