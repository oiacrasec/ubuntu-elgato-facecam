use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn main() {
    for path in [
        "src",
        "assets",
        "locales",
        "Cargo.toml",
        "Cargo.lock",
        "build.rs",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    for name in ["HEAD", "packed-refs"] {
        if let Some(path) = git(&["rev-parse", "--git-path", name]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    if let Some(branch) = git(&["symbolic-ref", "-q", "HEAD"])
        && let Some(path) = git(&["rev-parse", "--git-path", &branch])
    {
        println!("cargo:rerun-if-changed={path}");
    }
    let revision = git(&["rev-parse", "--verify", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let clean = Command::new("git")
        .args([
            "diff",
            "--quiet",
            "HEAD",
            "--",
            "src",
            "assets",
            "locales",
            "Cargo.toml",
            "Cargo.lock",
            "build.rs",
        ])
        .status()
        .is_ok_and(|s| s.success());
    println!("cargo:rustc-env=FACECAM_REVISION={revision}");
    println!("cargo:rustc-env=FACECAM_DIRTY={}", !clean);
}
