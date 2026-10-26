//! Text-only catalogs. Command IDs, paths and user content are never translated.
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock, RwLock},
};

const PT: &str = include_str!("../locales/pt-BR.json");
const EN: &str = include_str!("../locales/en-US.json");
const LIMIT: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Language {
    pub language: String,
    pub name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    language: String,
    name: String,
    #[serde(deserialize_with = "unique_messages")]
    messages: BTreeMap<String, String>,
}

fn unique_messages<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    struct Entries;
    impl<'de> serde::de::Visitor<'de> for Entries {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a translation object with unique keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, String>()? {
                if result.insert(key.clone(), value).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate translation key: {key}"
                    )));
                }
            }
            Ok(result)
        }
    }
    decoder.deserialize_map(Entries)
}

pub struct Catalog {
    language: Language,
    messages: BTreeMap<String, String>,
    patterns: Vec<(Pattern, Pattern)>,
}

#[derive(Clone, Debug)]
struct Pattern {
    literals: Vec<String>,
    fields: Vec<String>,
}
impl Pattern {
    fn parse(text: &str) -> Result<Self> {
        let mut literals = vec![String::new()];
        let mut fields = Vec::new();
        let mut anonymous = 0;
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '{' && chars.peek() == Some(&'{') {
                chars.next();
                literals.last_mut().context("Texto inválido")?.push('{');
            } else if c == '}' && chars.peek() == Some(&'}') {
                chars.next();
                literals.last_mut().context("Texto inválido")?.push('}');
            } else if c == '{' {
                let mut field = String::new();
                let mut closed = false;
                for c in chars.by_ref() {
                    if c == '}' {
                        closed = true;
                        break;
                    }
                    if c == '{' {
                        bail!("Placeholder inválido");
                    }
                    field.push(c);
                }
                if !closed || fields.len() >= 32 {
                    bail!("Placeholder incompleto ou excessivo");
                }
                if field.is_empty() || field.starts_with(':') {
                    field = format!("{anonymous}{field}");
                    anonymous += 1;
                }
                fields.push(field);
                literals.push(String::new());
            } else if c == '}' {
                bail!("Chave de tradução sem abertura");
            } else {
                literals.last_mut().context("Texto inválido")?.push(c);
            }
        }
        Ok(Self { literals, fields })
    }

    fn capture<'a, 'b>(&'a self, text: &'b str) -> Option<BTreeMap<&'a str, &'b str>> {
        // Require application wording, not a generic placeholder or punctuation.
        if self.fields.is_empty()
            || self.literals.iter().map(String::len).sum::<usize>() < 8
            || !self
                .literals
                .iter()
                .any(|s| s.chars().any(char::is_alphabetic))
        {
            return None;
        }
        let mut rest = text.strip_prefix(self.literals.first()?)?;
        let mut values = BTreeMap::new();
        for (i, field) in self.fields.iter().enumerate() {
            let next = self.literals.get(i + 1)?;
            let end = if next.is_empty() {
                if i + 1 != self.fields.len() {
                    return None;
                }
                rest.len()
            } else {
                rest.find(next)?
            };
            let value = rest.get(..end)?;
            if let Some(previous) = values.insert(field.as_str(), value)
                && previous != value
            {
                return None;
            }
            rest = rest.get(end + next.len()..)?;
        }
        rest.is_empty().then_some(values)
    }

    fn render(&self, values: &BTreeMap<&str, &str>) -> Option<String> {
        let mut output = self.literals.first()?.clone();
        for (i, field) in self.fields.iter().enumerate() {
            output.push_str(values.get(field.as_str())?);
            output.push_str(self.literals.get(i + 1)?);
        }
        Some(output)
    }
}

pub fn valid_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag.len() <= 35
        && tag
            .split('-')
            .all(|part| !part.is_empty() && part.bytes().all(|c| c.is_ascii_alphanumeric()))
}

impl Catalog {
    pub fn parse(text: &str, expected: &str) -> Result<Self> {
        if text.len() > LIMIT || !valid_tag(expected) {
            bail!("Idioma ou catálogo inválido");
        }
        let file: File = serde_json::from_str(text).context("Catálogo de idioma inválido")?;
        if file.language != expected
            || file.name.trim().is_empty()
            || file.name.len() > 128
            || file.name.chars().any(char::is_control)
            || file.messages.len() > 4096
        {
            bail!("Metadados de idioma inválidos");
        }
        let mut patterns = Vec::new();
        for (source, target) in &file.messages {
            if source.is_empty()
                || source.len() > 4096
                || target.len() > 16384
                || target.trim().is_empty()
                || source
                    .chars()
                    .chain(target.chars())
                    .any(|c| c.is_control() && c != '\n' && c != '\t')
            {
                bail!("Texto de tradução inválido: {source}");
            }
            let original = Pattern::parse(source)
                .with_context(|| format!("Chave de tradução inválida: {source}"))?;
            let translated =
                Pattern::parse(target).with_context(|| format!("Tradução inválida: {source}"))?;
            let mut a = original.fields.clone();
            let mut b = translated.fields.clone();
            a.sort();
            b.sort();
            if a != b {
                bail!("Placeholders incompatíveis na tradução: {source}");
            }
            if !a.is_empty() {
                patterns.push((original, translated));
            }
        }
        // More-specific report prefixes win over generic error wrappers.
        patterns.sort_by_key(|(p, _)| std::cmp::Reverse(p.literals.first().map_or(0, String::len)));
        Ok(Self {
            language: Language {
                language: file.language,
                name: file.name,
            },
            messages: file.messages,
            patterns,
        })
    }

    pub fn text(&self, source: &str) -> String {
        self.report(source, 0)
    }

    fn report(&self, source: &str, depth: u8) -> String {
        if let Some(value) = self.messages.get(source) {
            return value.clone();
        }
        for (original, translated) in &self.patterns {
            if let Some(values) = original.capture(source) {
                let errors: BTreeMap<_, _> = values
                    .iter()
                    .filter(|(name, _)| {
                        depth < 4
                            && matches!(
                                name.split(':').next(),
                                Some("e" | "error" | "restore" | "warning" | "tail" | "detail")
                            )
                    })
                    .map(|(&key, &value)| (key, self.report(value, depth + 1)))
                    .collect();
                let values = values
                    .into_iter()
                    .map(|(key, value)| (key, errors.get(key).map_or(value, String::as_str)))
                    .collect();
                if let Some(output) = translated.render(&values) {
                    return output;
                }
            }
        }
        if depth < 8 {
            if let Some((context, detail)) = source.split_once(": ")
                && let Some(translated) = self.messages.get(context)
            {
                return format!("{translated}: {}", self.report(detail, depth + 1));
            }
            for (suffix, translated) in &self.messages {
                if suffix.starts_with(' ')
                    && suffix.len() > 20
                    && let Some(prefix) = source.strip_suffix(suffix)
                {
                    return self.report(prefix, depth + 1) + translated;
                }
            }
        }
        source.into()
    }

    fn merge_fallback(&mut self, fallback: Self) {
        for (key, value) in fallback.messages {
            self.messages.entry(key).or_insert(value);
        }
        self.patterns.extend(fallback.patterns);
    }
}

fn directories(config: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::var_os("FACECAM_LOCALES_DIR") {
        dirs.push(dir.into());
    }
    if let Some(parent) = config.parent() {
        dirs.push(parent.join("locales"));
    }
    if let Some(data) = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    {
        dirs.push(data.join("facecam-hub/locales"));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        dirs.push(parent.join("locales"));
        if let Some(prefix) = parent.parent() {
            dirs.push(prefix.join("share/facecam-hub/locales"));
        }
    }
    dirs.push(PathBuf::from("locales"));
    dirs.extend(
        [
            "/usr/local/share/facecam-hub/locales",
            "/usr/share/facecam-hub/locales",
        ]
        .map(PathBuf::from),
    );
    dirs
}

pub fn load(language: &str, config: &Path) -> Result<Catalog> {
    if !valid_tag(language) {
        bail!("Código de idioma inválido");
    }
    let mut result = None;
    for directory in directories(config) {
        let path = directory.join(format!("{language}.json"));
        match std::fs::File::open(&path) {
            Ok(file) => {
                let mut text = String::new();
                file.take(LIMIT as u64 + 1).read_to_string(&mut text)?;
                result = Some(
                    Catalog::parse(&text, language)
                        .with_context(|| format!("Idioma indisponível: {}", path.display()))?,
                );
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).context("Não foi possível ler o catálogo de idioma"),
        }
    }
    let fallback = Catalog::parse(PT, "pt-BR")?;
    let mut catalog = match result {
        Some(catalog) => catalog,
        None if language == "pt-BR" => return Ok(fallback),
        None if language == "en-US" => Catalog::parse(EN, "en-US")?,
        None => bail!("Idioma não instalado: {language}"),
    };
    if language == "en-US" {
        catalog.merge_fallback(Catalog::parse(EN, "en-US")?);
    }
    catalog.merge_fallback(fallback);
    Ok(catalog)
}

pub fn fallback() -> Result<Catalog> {
    Catalog::parse(PT, "pt-BR")
}

pub fn available(config: &Path) -> Vec<Language> {
    let mut languages = BTreeMap::new();
    for (tag, text) in [("pt-BR", PT), ("en-US", EN)] {
        if let Ok(c) = Catalog::parse(text, tag) {
            languages.insert(tag.to_owned(), c.language);
        }
    }
    for dir in directories(config).into_iter().rev() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.take(128).flatten() {
                let path = entry.path();
                if let Some(tag) = path.file_stem().and_then(|s| s.to_str())
                    && path.extension().is_some_and(|s| s == "json")
                    && let Ok(c) = load(tag, config)
                {
                    languages.insert(tag.to_owned(), c.language);
                }
            }
        }
    }
    languages.into_values().collect()
}

fn current() -> &'static RwLock<Option<Arc<Catalog>>> {
    static CURRENT: OnceLock<RwLock<Option<Arc<Catalog>>>> = OnceLock::new();
    CURRENT.get_or_init(|| RwLock::new(None))
}
pub fn activate(catalog: Catalog) {
    *current().write().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(catalog));
}
pub fn tr(source: &str) -> String {
    if let Some(catalog) = current().read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        catalog.text(source)
    } else {
        source.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogs_validate_metadata_placeholders_and_preserve_user_values() {
        let catalog = Catalog::parse(r#"{"language":"en-US","name":"English","messages":{"Olá":"Hello","Arquivo {path} recusado: {error}":"Rejected {path}: {error}"}}"#, "en-US").unwrap();
        assert_eq!(catalog.text("Olá"), "Hello");
        assert_eq!(catalog.text("Filename unchanged"), "Filename unchanged");
        assert_eq!(
            catalog.text("Arquivo /tmp/Olá recusado: bad bytes"),
            "Rejected /tmp/Olá: bad bytes"
        );
        assert!(Catalog::parse(r#"{"language":"en-US","name":"English","messages":{"Arquivo {path}":"File {error}"}}"#, "en-US").is_err());
        assert!(
            Catalog::parse(
                r#"{"language":"en-US","name":"English","messages":{"Olá":""}}"#,
                "en-US"
            )
            .is_err()
        );
        assert!(Catalog::parse(EN, "../en-US").is_err());
        assert!(Catalog::parse(EN, "pt-BR").is_err());
        assert!(Catalog::parse(&" ".repeat(LIMIT + 1), "en-US").is_err());
    }
    #[test]
    fn shipped_catalogs_have_identical_keys_and_safe_templates() {
        let pt = Catalog::parse(PT, "pt-BR").unwrap();
        let en = Catalog::parse(EN, "en-US").unwrap();
        assert_eq!(
            pt.messages.keys().collect::<Vec<_>>(),
            en.messages.keys().collect::<Vec<_>>()
        );
        assert_eq!(pt.messages.len(), en.messages.len());
    }

    #[test]
    fn duplicate_keys_and_malformed_placeholders_are_refused() {
        for messages in [
            r#"{"Camera":"Câmera","Camera":"Outra"}"#,
            r#"{"Arquivo {path}":"File {path"}"#,
            r#"{"Arquivo {path}":"File }path{"}"#,
            r#"{"Arquivo {path:?}":"File {path}"}"#,
        ] {
            let text = format!(r#"{{"language":"en-US","name":"English","messages":{messages}}}"#);
            assert!(Catalog::parse(&text, "en-US").is_err(), "{text}");
        }
    }

    #[test]
    fn templates_reorder_named_values_and_translate_only_error_contexts() {
        let catalog = Catalog::parse(
            r#"{"language":"en-US","name":"English","messages":{
            "Operação {status} no arquivo {}: {error}":"File {}: {error} ({status})",
            "Falha interna":"Internal failure",
            "Não encontrado":"Not found",
            "Abrindo arquivo":"Opening file"
        }}"#,
            "en-US",
        )
        .unwrap();
        assert_eq!(
            catalog.text("Operação Não encontrado no arquivo /tmp/Falha interna: Falha interna"),
            "File /tmp/Falha interna: Internal failure (Não encontrado)"
        );
        assert_eq!(
            catalog.text("Abrindo arquivo: Falha interna: detail"),
            "Opening file: Internal failure: detail"
        );
        assert_eq!(catalog.text("/tmp/Falha interna"), "/tmp/Falha interna");
    }
}
