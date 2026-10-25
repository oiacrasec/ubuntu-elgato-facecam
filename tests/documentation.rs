use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn markdown_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            markdown_files(&path, files);
        } else if path.extension().is_some_and(|e| e == "md") {
            files.push(path);
        }
    }
}

#[test]
fn maintained_documentation_has_no_broken_local_file_links() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = ["README.md", "CLAUDE.md", "AGENTS.md"]
        .map(|p| root.join(p))
        .to_vec();
    markdown_files(&root.join("docs"), &mut files);
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        for tail in text.split("](").skip(1) {
            let Some((target, _)) = tail.split_once(')') else {
                continue;
            };
            let target = target.trim().trim_matches(['<', '>']);
            if target.contains("://") || target.starts_with('#') || target.starts_with("mailto:") {
                continue;
            }
            let target = target.split('#').next().unwrap();
            assert!(
                file.parent().unwrap().join(target).is_file(),
                "{}: missing link {target}",
                file.display()
            );
        }
    }
}

#[derive(Deserialize)]
struct Coverage {
    source_documents: usize,
    source_markdown_documents: usize,
    source_revision: String,
    entries: Vec<Entry>,
}
#[derive(Deserialize)]
struct Entry {
    id: String,
    subject: String,
    destination: String,
    adaptation: String,
    source_sha256: String,
    source_bytes: usize,
}

#[test]
fn documentary_inventory_accounts_for_every_reviewed_file() {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs");
    let coverage: Coverage =
        serde_json::from_slice(&fs::read(docs.join("cobertura-documental.json")).unwrap()).unwrap();
    assert_eq!(
        coverage.source_documents, 152,
        "a new source inventory requires an explicit coverage review"
    );
    assert_eq!(coverage.source_markdown_documents, 137);
    assert_eq!(coverage.entries.len(), coverage.source_documents);
    assert_eq!(coverage.source_revision.len(), 40);
    assert!(
        coverage
            .source_revision
            .bytes()
            .all(|c| c.is_ascii_hexdigit())
    );
    let catalog = fs::read_to_string(docs.join("COBERTURA_DOCUMENTAL.md")).unwrap();
    let mut ids = BTreeSet::new();
    for (index, entry) in coverage.entries.iter().enumerate() {
        assert_eq!(entry.id, format!("D{:03}", index + 1));
        assert!(ids.insert(&entry.id));
        assert!(!entry.subject.is_empty() && !entry.adaptation.is_empty());
        assert!(entry.source_bytes > 0);
        assert_eq!(entry.source_sha256.len(), 64);
        assert!(entry.source_sha256.bytes().all(|c| c.is_ascii_hexdigit()));
        assert!(
            docs.join(&entry.destination).is_file(),
            "{}: {}",
            entry.id,
            entry.destination
        );
        assert!(catalog.contains(&format!("| {} |", entry.id)));
        assert!(catalog.contains(&entry.source_sha256[..12]));
    }
}
