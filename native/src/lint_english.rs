//! The LangBrain — the common-language substrate read from the machine's own dictionary.
//!
//! LINTER.md, "Common language first": a rule written in natural English can only be understood
//! by something that understands natural English, so BEFORE any documentation is read the
//! substrate learns common language from the dictionary installed on this machine (macOS: the
//! New Oxford American Dictionary body). Two things are learned, both pure data: the
//! common-language frequency curve (every definition's prose read through the one
//! [`crate::lint_read::Reader`]) and the DEFINED-WORD set — the vocabulary English itself
//! accounts for, which is the English-knowledge judgment construct selection ranks with.
//!
//! The artifact (`english.global.json`, machine-global beside the models) is built once at
//! SETUP time ([`ensure_built`]) and only ever LOADED on the lint path ([`brain`]); machines
//! without a parseable dictionary fall through to the committed bootstrap
//! (`lint-index/english-bootstrap.json` — machine-generated learned data, regenerated with
//! `cargo test --release --lib generate_english_bootstrap -- --ignored`).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// The serialized common-language brain: the dictionary-fed reader (frequencies of real
/// English) plus the set of words the dictionary DEFINES (headword token seeds). A word in
/// `defined` is common language whatever its count; a word in neither is not English — it is
/// the thing a rule's sentence is about.
#[derive(Default, Serialize, Deserialize)]
pub struct English {
    /// The reader after reading every definition's prose — real English frequencies.
    pub reader: crate::lint_read::Reader,
    /// Token seeds of every word the dictionary defines (its headwords, tokenized through the
    /// reader's one tokenizer).
    pub defined: HashSet<u64>,
    /// mtime^len fingerprint of the dictionary body this was read from — rebuild only when the
    /// dictionary itself changed.
    #[serde(default)]
    pub source_fp: u64,
}

impl English {
    /// Whether common language accounts for `token`: the dictionary defines it, or definition
    /// prose reads it as common English (scale-free head of the dictionary corpus — catches
    /// inflections and function words that head no entry of their own).
    pub fn knows(&self, token: &str) -> bool {
        self.defined.contains(&crate::lint_ai::token_seed(&token.to_lowercase()))
            || self.reader.is_head_word(token)
    }
}

/// Where the machine's built brain lives, beside the shared polarity store.
fn store_path() -> PathBuf {
    crate::lint_train::model_dir_pub().join("english.global.json")
}

/// The loaded common-language brain: the machine's built store first, else the committed
/// bootstrap, else `None` (selection then degrades to the docs-corpus head judgment alone).
/// Memoized for the process — the lint path never builds, only loads (LINTER.md: setup
/// acquires, lint replays).
pub fn brain() -> Option<&'static English> {
    static BRAIN: std::sync::OnceLock<Option<English>> = std::sync::OnceLock::new();
    BRAIN
        .get_or_init(|| {
            let stored = std::fs::read_to_string(store_path())
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok());
            stored.or_else(|| {
                crate::lint_train::embedded_lint_index_file("english-bootstrap.json")
                    .and_then(|s| serde_json::from_str(&s).ok())
            })
        })
        .as_ref()
}

/// SETUP verb: make sure this machine's brain exists and matches its dictionary — build and
/// save when missing or the dictionary changed, replay otherwise. Purely local (the dictionary
/// is a file on disk; no network), so it is safe in every setup path. Returns a one-line
/// human report of what happened, or `None` when no dictionary is parseable here (the
/// committed bootstrap then serves).
pub fn ensure_built() -> Option<String> {
    let (path, fp) = dictionary_body()?;
    let current: Option<English> = std::fs::read_to_string(store_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());
    if current.is_some_and(|e| e.source_fp == fp && !e.defined.is_empty()) {
        return Some("common language: current".to_string());
    }
    let english = read_dictionary(&path, fp)?;
    let report = format!(
        "common language: read {} defined words, {} tokens from {}",
        english.defined.len(),
        english.reader.total_read(),
        path.file_name().and_then(|n| n.to_str()).unwrap_or("dictionary"),
    );
    if let Some(dir) = store_path().parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string(&english) {
        let _ = std::fs::write(store_path(), json);
    }
    Some(report)
}

/// The best English dictionary body on this machine plus its change fingerprint: prefer the
/// New Oxford American, else the largest non-localized `Body.data`. Localized variants
/// (`*.lproj/Body.data`) are other languages — the substrate's baseline is English.
fn dictionary_body() -> Option<(PathBuf, u64)> {
    let mut candidates: Vec<(u64, PathBuf)> = Vec::new();
    let roots = [
        PathBuf::from("/System/Library/AssetsV2/com_apple_MobileAsset_DictionaryServices_dictionaryOSX"),
        PathBuf::from("/Library/Dictionaries"),
    ];
    for root in roots {
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.file_name().is_some_and(|n| n == "Body.data")
                    && !p.to_string_lossy().contains(".lproj")
                {
                    let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    candidates.push((len, p));
                }
            }
        }
    }
    let preferred = candidates
        .iter()
        .filter(|(_, p)| p.to_string_lossy().contains("New Oxford American"))
        .max_by_key(|(len, _)| *len)
        .cloned();
    let (len, path) = preferred.or_else(|| candidates.into_iter().max_by_key(|(len, _)| *len))?;
    let mtime = std::fs::metadata(&path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Some((path, mtime ^ (len << 20)))
}

/// Read one Apple dictionary body into a brain: walk its zlib chunks, strip each entry's XML
/// down to definition prose the reader learns from, and collect every `d:title` headword's
/// tokens as the defined-word set.
fn read_dictionary(path: &std::path::Path, source_fp: u64) -> Option<English> {
    let data = std::fs::read(path).ok()?;
    let mut english = English { source_fp, ..Default::default() };
    // Chunk heap layout (verified against the shipped New Oxford American body):
    // 0x60-byte header, then [outer size u32][inner size u32][raw size u32][zlib stream of
    // outer-8 bytes] repeated. The walk stops at the first malformed chunk — a partial read
    // of a foreign layout must not poison the brain with garbage tokens.
    let mut pos = 0x60usize;
    let mut chunks = 0usize;
    while pos + 12 < data.len() {
        let outer = u32::from_le_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
        if outer < 8 || pos + 4 + outer > data.len() {
            break;
        }
        let Ok(xml_bytes) = miniz_oxide::inflate::decompress_to_vec_zlib(&data[pos + 12..pos + 4 + outer])
        else {
            break;
        };
        let xml = String::from_utf8_lossy(&xml_bytes);
        let (prose, titles) = strip_entry_xml(&xml);
        english.reader.learn_span(&prose);
        for title in titles {
            for tok in crate::lint_read::tokens(&title) {
                english.defined.insert(crate::lint_ai::token_seed(&tok));
            }
        }
        chunks += 1;
        pos += 4 + outer;
    }
    (chunks > 0 && !english.defined.is_empty()).then_some(english)
}

/// Reduce dictionary entry XML to (definition prose, headword titles): tags drop, `d:title`
/// attribute values are captured, character entities become spaces (they are punctuation-class
/// for the reader's word runs). No XML library — the format is a flat span soup and the reader
/// only needs word runs.
fn strip_entry_xml(xml: &str) -> (String, Vec<String>) {
    let mut prose = String::with_capacity(xml.len() / 4);
    let mut titles = Vec::new();
    let mut rest = xml;
    while let Some(open) = rest.find('<') {
        prose.push_str(&rest[..open]);
        prose.push(' ');
        let Some(close) = rest[open..].find('>') else { break };
        let tag = &rest[open + 1..open + close];
        if let Some(t) = tag.split("d:title=\"").nth(1).and_then(|a| a.split('"').next()) {
            titles.push(t.to_string());
        }
        rest = &rest[open + close + 1..];
    }
    prose.push_str(rest);
    let prose = prose.replace('&', " ");
    (prose, titles)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DEV TOOL — regenerates `lint-index/english-bootstrap.json` from this machine's
    /// dictionary so machines without one still get the common-language substrate. Run
    /// whenever the tokenizer or the dictionary parser changes:
    /// `cargo test --release --lib generate_english_bootstrap -- --ignored`
    #[test]
    #[ignore]
    fn generate_english_bootstrap() {
        let (path, _) = dictionary_body().expect("this machine has a dictionary");
        // The bootstrap is a portable substrate, not a machine snapshot: fingerprint zero so
        // any machine that CAN read its own dictionary always rebuilds over it, and the
        // frequency tail (single reads — scan noise, hapax typos) is dropped as pure size.
        let mut english = read_dictionary(&path, 0).expect("dictionary parses");
        english.reader.retain_read_at_least(2);
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../lint-index/english-bootstrap.json");
        std::fs::write(&out, serde_json::to_string(&english).expect("serializes"))
            .expect("bootstrap written");
        println!(
            "wrote {} — {} defined words, {} tokens read",
            out.display(),
            english.defined.len(),
            english.reader.total_read(),
        );
    }

    /// The committed bootstrap must answer the judgment selection ranks with: register and
    /// code-register English are KNOWN, code constructs are NOT. This is the exact word set
    /// ledger #17 was measured on — the invariant, as a table.
    #[test]
    fn bootstrap_separates_english_from_code_constructs() {
        let english: English = serde_json::from_str(
            &crate::lint_train::embedded_lint_index_file("english-bootstrap.json")
                .expect("bootstrap committed"),
        )
        .expect("bootstrap parses");
        for known in ["never", "import", "module", "use", "print", "the", "write", "document"] {
            assert!(english.knows(known), "common language must account for {known:?}");
        }
        for construct in ["telnetlib", "xmlhttprequest", "dbg", "paramiko"] {
            assert!(!english.knows(construct), "{construct:?} is not English — it is a construct");
        }
    }
}
