//! Locate optional AI assets without loading a native runtime or changing providers.
use crate::effects::{AiProvider, Effects};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Default, Deserialize)]
struct Installation {
    ai_model: String,
    ai_runtime: String,
}

pub fn autofill(effects: &mut Effects) {
    if !effects.ai_model.is_empty() && !effects.ai_runtime.is_empty() {
        return;
    }
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
    let mut roots = Vec::new();
    // Explicit configuration wins; environment values precede installation hints.
    for (target, name) in [
        (&mut effects.ai_model, "FACECAM_AI_MODEL"),
        (&mut effects.ai_runtime, "FACECAM_AI_RUNTIME"),
    ] {
        if let Some(path) = std::env::var_os(name) {
            fill_file(target, Path::new(&path));
        }
    }
    if let Some(data) = data {
        // The preparer registers custom directories here as well as its default.
        if let Ok(file) = std::fs::File::open(data.join("facecam-hub/ai-installations.json")) {
            let mut json = String::new();
            if file.take(65536).read_to_string(&mut json).is_ok()
                && let Ok(registry) = serde_json::from_str::<BTreeMap<String, Installation>>(&json)
                && let Some(location) = registry
                    .get(match effects.ai_provider {
                        AiProvider::Cuda => "cuda",
                        AiProvider::Cpu => "cpu",
                    })
                    .or_else(|| {
                        registry.get(match effects.ai_provider {
                            AiProvider::Cuda => "cpu",
                            AiProvider::Cpu => "cuda",
                        })
                    })
            {
                fill_file(&mut effects.ai_model, Path::new(&location.ai_model));
                fill_file(&mut effects.ai_runtime, Path::new(&location.ai_runtime));
            }
        }
        roots.push(data.join("facecam-hub/ai"));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        roots.push(dir.join("ai"));
        roots.push(dir.to_path_buf());
        if let Some(parent) = dir.parent() {
            roots.push(parent.join("ai-runtime"));
            roots.push(parent.join("share/facecam-hub/ai"));
            roots.push(parent.join("lib"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.join("target/ai-runtime"));
        roots.push(cwd.join("ai"));
    }
    roots.extend(
        [
            "/usr/local/share/facecam-hub/ai",
            "/usr/share/facecam-hub/ai",
            "/opt/onnxruntime",
            "/usr/local/lib",
            "/usr/lib/x86_64-linux-gnu",
            "/usr/lib",
        ]
        .map(PathBuf::from),
    );
    discover_in(effects, &roots);
}

fn fill_file(target: &mut String, path: &Path) {
    if target.is_empty()
        && path.is_file()
        && let Some(path) = path.to_str()
    {
        *target = path.to_owned();
    }
}

fn version(path: &Path) -> Vec<u64> {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

fn runtime_in(dir: &Path) -> Option<PathBuf> {
    for name in ["libonnxruntime.so", "libonnxruntime.so.1"] {
        let file = dir.join(name);
        if file.is_file() {
            return Some(file);
        }
    }
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .take(256)
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|s| s.to_string_lossy().starts_with("libonnxruntime.so."))
                && p.is_file()
        })
        .collect();
    files.sort_by_key(|p| std::cmp::Reverse(version(p)));
    files.into_iter().next()
}

fn discover_in(effects: &mut Effects, roots: &[PathBuf]) {
    for root in roots {
        for name in ["selfie.onnx", "models/selfie.onnx", "onnx/model.onnx"] {
            fill_file(&mut effects.ai_model, &root.join(name));
        }
        if effects.ai_runtime.is_empty() {
            let mut packages: Vec<_> = std::fs::read_dir(root)
                .into_iter()
                .flatten()
                .take(256)
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    p.is_dir()
                        && p.file_name()
                            .is_some_and(|s| s.to_string_lossy().starts_with("onnxruntime-"))
                })
                .collect();
            packages.sort_by_key(|p| {
                let gpu = p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .contains("-gpu-");
                (
                    gpu != (effects.ai_provider == AiProvider::Cuda),
                    std::cmp::Reverse(version(p)),
                )
            });
            let dirs = [root.join("lib"), root.clone()]
                .into_iter()
                .chain(packages.into_iter().map(|p| p.join("lib")));
            for dir in dirs {
                if let Some(file) = runtime_in(&dir) {
                    fill_file(&mut effects.ai_runtime, &file);
                    break;
                }
            }
        }
        if !effects.ai_model.is_empty() && !effects.ai_runtime.is_empty() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn file(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, []).unwrap();
    }
    #[test]
    fn finds_model_and_prefers_the_selected_provider_without_enabling_ai() {
        let dir = tempfile::tempdir().unwrap();
        file(&dir.path().join("selfie.onnx"));
        let cpu = dir
            .path()
            .join("onnxruntime-linux-x64-1.22.0/lib/libonnxruntime.so");
        let gpu = dir
            .path()
            .join("onnxruntime-linux-x64-gpu-1.22.0/lib/libonnxruntime.so");
        file(&cpu);
        file(&gpu);
        for (provider, expected) in [(AiProvider::Cpu, cpu), (AiProvider::Cuda, gpu)] {
            let mut e = Effects {
                ai_provider: provider,
                ..Default::default()
            };
            discover_in(&mut e, &[dir.path().into()]);
            assert_eq!(Path::new(&e.ai_runtime), expected);
            assert_eq!(Path::new(&e.ai_model), dir.path().join("selfie.onnx"));
            assert!(!e.background_blur);
            assert_eq!(e.ai_provider, provider);
        }
    }
    #[test]
    fn preserves_explicit_paths_even_when_missing_and_fills_only_empty_fields() {
        let dir = tempfile::tempdir().unwrap();
        file(&dir.path().join("selfie.onnx"));
        file(&dir.path().join("lib/libonnxruntime.so"));
        for (model, runtime) in [
            ("custom.onnx", ""),
            ("", "custom.so"),
            ("custom.onnx", "custom.so"),
        ] {
            let mut e = Effects {
                ai_model: model.into(),
                ai_runtime: runtime.into(),
                ..Default::default()
            };
            discover_in(&mut e, &[dir.path().into()]);
            if !model.is_empty() {
                assert_eq!(e.ai_model, model);
            } else {
                assert!(e.ai_model.ends_with("selfie.onnx"));
            }
            if !runtime.is_empty() {
                assert_eq!(e.ai_runtime, runtime);
            } else {
                assert!(e.ai_runtime.ends_with("libonnxruntime.so"));
            }
        }
    }
    #[test]
    fn handles_absence_directories_and_versioned_libraries_deterministically() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("selfie.onnx")).unwrap();
        let mut e = Effects::default();
        discover_in(&mut e, &[dir.path().into()]);
        assert!(e.ai_model.is_empty() && e.ai_runtime.is_empty());
        file(&dir.path().join("lib/libonnxruntime.so.1.9.0"));
        let latest = dir.path().join("lib/libonnxruntime.so.1.22.0");
        file(&latest);
        discover_in(&mut e, &[dir.path().into()]);
        assert_eq!(Path::new(&e.ai_runtime), latest);
        assert!(e.ai_model.is_empty());
    }
}
