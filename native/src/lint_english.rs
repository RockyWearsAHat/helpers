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
//! The artifact (`english.global.bin`, an `HLM1` container beside the models) is built once at
//! SETUP time ([`ensure_built`]) and only ever LOADED on the lint path ([`brain`]); machines
//! without a parseable dictionary fall through to the committed bootstrap
//! (`lint-index/english-bootstrap.json` — machine-generated learned data, regenerated with
//! `cargo test --release --lib generate_english_bootstrap -- --ignored`).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::lint_codec::{Bin, SeedSet};

/// The serialized common-language brain: the dictionary-fed reader (frequencies of real
/// English) plus the set of words the dictionary DEFINES (headword token seeds). A word in
/// `defined` is common language whatever its count; a word in neither is not English — it is
/// the thing a rule's sentence is about.
#[derive(Default, Serialize, Deserialize)]
pub struct English {
    /// The reader after reading every definition's prose — real English frequencies.
    pub reader: crate::lint_read::Reader,
    /// Token seeds of every word the dictionary defines (its headwords, tokenized through the
    /// reader's one tokenizer). Frozen sorted array when loaded; see [`SeedSet`].
    pub defined: SeedSet,
    /// Definitions as bindings (LINTER.md, "Common language first"): each SINGLE-WORD
    /// headword's seed → the content-word seeds of its own definition (first
    /// [`MAX_DEFINITION_WORDS`], self excluded), sorted by headword seed. This is the meaning
    /// network the side-count polarity layer diffuses leans through — a word grounding never
    /// met inherits polarity from the words that define it.
    #[serde(default)]
    pub definitions: Vec<(u64, Vec<u64>)>,
    /// The language's DISCOVERED negation primitives (LINTER.md, "The cold floor"): token
    /// seeds that appear in negative-morphology definitions ("invalid" = "not valid") far
    /// above their background definition rate. Sorted; discovered statistically at
    /// dictionary-read time — never enumerated, so any language's dictionary yields its own.
    #[serde(default)]
    pub negators: Vec<u64>,
    /// mtime^len fingerprint of the dictionary body this was read from — rebuild only when the
    /// dictionary itself changed.
    #[serde(default)]
    pub source_fp: u64,
}

/// Definition content-word cap: the leading words of a dictionary definition carry its sense
/// (dictionaries front-load the genus); the tail is example phrases and cross-references.
const MAX_DEFINITION_WORDS: usize = 12;

impl English {
    /// Whether common language accounts for `token`: the dictionary defines it, or definition
    /// prose reads it as common English (scale-free head of the dictionary corpus — catches
    /// inflections and function words that head no entry of their own).
    pub fn knows(&self, token: &str) -> bool {
        self.defined.contains(crate::lint_ai::token_seed(&token.to_lowercase()))
            || self.reader.is_head_word(token)
    }

    /// The definition content-word seeds of the headword with `seed` — the one dictionary hop
    /// polarity diffusion reads through. `None` for a word the dictionary gave no usable
    /// definition (multi-word headword, or no entry).
    pub fn definition_of(&self, seed: u64) -> Option<&[u64]> {
        self.definitions
            .binary_search_by_key(&seed, |(k, _)| *k)
            .ok()
            .map(|i| self.definitions[i].1.as_slice())
    }

    /// Whether `seed` is a NEGATION OPERATOR (LINTER.md, "The cold floor"): a discovered
    /// negator, or a word whose definition is negation COMPOUNDED — it contains a negator
    /// AND another word that is itself negator-defined ("never" = "at NO time … NOT ever").
    /// One negator alone is not enough: litotes runs through dictionaries ("plain" = "NOT
    /// decorated") and a property negated once is a description, not a prohibition operator.
    /// This is the judgment an unready polarity classifier reads overt prohibition with.
    pub fn is_negation(&self, seed: u64) -> bool {
        if self.negators.binary_search(&seed).is_ok() {
            return true;
        }
        let Some(def) = self.definition_of(seed) else { return false };
        let is_negator = |s: &u64| self.negators.binary_search(s).is_ok();
        let negator_defined = |s: &u64| {
            self.definition_of(*s).is_some_and(|d| d.iter().any(is_negator))
        };
        def.iter().any(is_negator) && def.iter().filter(|s| !is_negator(s)).any(|s| negator_defined(s))
    }
}

/// HLM1 wire form (LINTER.md, "Save"): the reader's frequency arrays and the headword set
/// ride the RAW stream, so loading the ~470k-entry substrate is a header parse plus bulk
/// copies — the frozen reader answers lookups by binary search, no map is ever built.
impl Bin for English {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        self.reader.enc(e);
        self.defined.enc(e);
        e.fixed_u64(self.source_fp);
        // Definitions ride the RAW stream flattened: headword seeds, per-head lengths, then
        // every definition's seeds back to back (decode rebuilds the slices by length).
        e.raw_u64s(&self.definitions.iter().map(|(k, _)| *k).collect::<Vec<_>>());
        e.raw_u32s(&self.definitions.iter().map(|(_, v)| v.len() as u32).collect::<Vec<_>>());
        e.raw_u64s(&self.definitions.iter().flat_map(|(_, v)| v.iter().copied()).collect::<Vec<_>>());
        e.raw_u64s(&self.negators);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<English> {
        let reader = Bin::dec(d)?;
        let defined = Bin::dec(d)?;
        let source_fp = d.fixed_u64()?;
        let heads = d.raw_u64s()?;
        let lens = d.raw_u32s()?;
        let flat = d.raw_u64s()?;
        if heads.len() != lens.len() || lens.iter().map(|l| *l as usize).sum::<usize>() != flat.len() {
            return None;
        }
        let mut definitions = Vec::with_capacity(heads.len());
        let mut at = 0usize;
        for (head, len) in heads.into_iter().zip(lens) {
            let next = at + len as usize;
            definitions.push((head, flat[at..next].to_vec()));
            at = next;
        }
        let negators = d.raw_u64s()?;
        Some(English { reader, defined, definitions, negators, source_fp })
    }
}

/// Where the machine's built brain lives, beside the shared polarity store.
fn store_path() -> PathBuf {
    crate::lint_train::model_dir_pub().join("english.global.bin")
}

/// Load the machine's built brain: the `HLM1` store first, else a legacy JSON store — which is
/// migrated to the container on sight (save `.bin`, delete `.json`) so the next load is a bulk
/// copy (LINTER.md, "Save").
fn load_store() -> Option<English> {
    if let Some(e) = std::fs::read(store_path())
        .ok()
        .and_then(|b| crate::lint_codec::Dec::open(&b, crate::lint_codec::kind::ENGLISH))
        .and_then(|(_, mut d)| English::dec(&mut d))
    {
        return Some(e);
    }
    let legacy = store_path().with_extension("json");
    let english: English = serde_json::from_str(&std::fs::read_to_string(&legacy).ok()?).ok()?;
    save_store(&english);
    let _ = std::fs::remove_file(&legacy);
    Some(english)
}

/// Persist the brain as its `HLM1` container (stamp = the dictionary fingerprint, so a prefix
/// probe can answer staleness without decoding 470k entries).
fn save_store(english: &English) {
    if let Some(dir) = store_path().parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut e = crate::lint_codec::Enc::new();
    english.enc(&mut e);
    let bytes = e.finish(crate::lint_codec::kind::ENGLISH, &format!("{:016x}", english.source_fp));
    let _ = std::fs::write(store_path(), bytes);
}

/// The loaded common-language brain: the machine's built store first, else the committed
/// bootstrap, else `None` (selection then degrades to the docs-corpus head judgment alone).
/// Memoized for the process — the lint path never builds, only loads (LINTER.md: setup
/// acquires, lint replays).
pub fn brain() -> Option<&'static English> {
    static BRAIN: std::sync::OnceLock<Option<English>> = std::sync::OnceLock::new();
    BRAIN
        .get_or_init(|| {
            load_store().or_else(|| {
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
    if load_store().is_some_and(|e| e.source_fp == fp && !e.defined.is_empty()) {
        return Some("common language: current".to_string());
    }
    let (english, chunks) = read_dictionary(&path, fp)?;
    let report = format!(
        "common language: read ENTIRE dictionary — {} chunks, {} defined words, {} tokens from {}",
        chunks,
        english.defined.len(),
        english.reader.total_read(),
        path.file_name().and_then(|n| n.to_str()).unwrap_or("dictionary"),
    );
    save_store(&english);
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

/// A sample of the machine dictionary's raw definition PROSE — the leading chunks, for the
/// fast validation gate. `None` when no dictionary is parseable here.
pub fn dictionary_prose_sample() -> Option<String> {
    dictionary_prose(Some(200))
}

/// The machine dictionary's raw definition PROSE, as one character stream — the character-level
/// reader's English base ([`crate::lint_char`]). `max_chunks` bounds the read (`None` = the
/// whole dictionary, the production curriculum). `None` return when no dictionary is parseable.
pub fn dictionary_prose(max_chunks: Option<usize>) -> Option<String> {
    let (path, _) = dictionary_body()?;
    let data = std::fs::read(path).ok()?;
    let mut out = String::new();
    let mut pos = 0x60usize;
    let mut chunks = 0usize;
    let cap = max_chunks.unwrap_or(usize::MAX);
    while pos + 12 < data.len() && chunks < cap {
        let outer = u32::from_le_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
        if outer < 8 || pos + 4 + outer > data.len() {
            break;
        }
        let Ok(xml_bytes) =
            miniz_oxide::inflate::decompress_to_vec_zlib(&data[pos + 12..pos + 4 + outer])
        else {
            break;
        };
        let xml = String::from_utf8_lossy(&xml_bytes);
        for piece in xml.split("<d:entry").skip(1) {
            let Some(gt) = piece.find('>') else { continue };
            let (prose, _) = strip_entry_xml(&piece[gt + 1..]);
            out.push_str(&prose);
            out.push(' ');
        }
        chunks += 1;
        pos += 4 + outer;
    }
    (!out.is_empty()).then_some(out)
}

/// Read one Apple dictionary body into a brain ENTIRELY, witnessed (LINTER.md, "Common
/// language first"): walk its zlib chunks, strip each entry's XML down to definition prose the
/// reader learns from, and collect every `d:title` headword's tokens as the defined-word set.
/// Returns the brain plus the chunk count for the setup report; `None` when the walk did NOT
/// consume the whole body (tail not zero padding) — a partial read is refused, never saved,
/// because half a frequency curve is a silently wrong baseline for every judgment downstream.
fn read_dictionary(path: &std::path::Path, source_fp: u64) -> Option<(English, usize)> {
    let data = std::fs::read(path).ok()?;
    let mut english = English { source_fp, ..Default::default() };
    // Chunk heap layout (verified against the shipped New Oxford American body):
    // 0x60-byte header, then [outer size u32][inner size u32][raw size u32][zlib stream of
    // outer-8 bytes] repeated. The walk stops at the first malformed chunk — a partial read
    // of a foreign layout must not poison the brain with garbage tokens.
    let mut pos = 0x60usize;
    let mut chunks = 0usize;
    // Negator discovery (LINTER.md, "The cold floor"): count every definition token's
    // appearances overall and inside NEGATIVE-MORPHOLOGY entries (headword = prefix + a word
    // of its own definition, "invalid" = "not valid"). The language's negation primitive is
    // whatever token rides those entries far above its background rate — discovered, never
    // enumerated.
    let mut def_count: std::collections::HashMap<u64, u32> = Default::default();
    let mut neg_count: std::collections::HashMap<u64, u32> = Default::default();
    let mut neg_entries = 0u32;
    let mut def_entries = 0u32;
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
        // One chunk holds MANY `<d:entry>` elements; a headword's definition is its OWN
        // entry's prose — entry-boundary slicing is the same tag-boundary law the crawler
        // follows (never byte offsets, never one blob for a hundred entries).
        for piece in xml.split("<d:entry").skip(1) {
        // The entry tag's own attributes carry the headword; its body carries the prose.
        let Some(gt) = piece.find('>') else { continue };
        let entry_title =
            piece[..gt].split("d:title=\"").nth(1).and_then(|a| a.split('"').next()).map(str::to_string);
        let (prose, mut titles) = strip_entry_xml(&piece[gt + 1..]);
        if let Some(t) = entry_title {
            titles.insert(0, t);
        }
        english.reader.learn_span(&prose);
        // Definitions as bindings: a SINGLE-WORD headword keeps its definition's leading
        // content-word seeds (multi-word titles are phrases — their parts are entries of
        // their own). First title of the entry owns the prose; sub-titles share it.
        for title in &titles {
            let toks = crate::lint_read::tokens(title);
            let [head] = toks.as_slice() else { continue };
            let head_seed = crate::lint_ai::token_seed(head);
            let mut def: Vec<u64> = Vec::new();
            let mut def_words: Vec<String> = Vec::new();
            // The DEFINITION is the text before the entry's first example separator
            // (" : " — the dictionary's own typography): example sentences quote arbitrary
            // usage ("do not use the phone — write instead") and must never leak their
            // vocabulary into the headword's meaning.
            let def_src = prose.split(" : ").next().unwrap_or(prose.as_str());
            for tok in crate::lint_read::tokens(def_src) {
                // Words only: entry prose also carries pronunciation (which tokenizes to
                // char shrapnel) and sense numbers — neither is definition MEANING. Any
                // language's letters pass; length is in characters, not bytes.
                if tok.chars().count() < 2 || !tok.chars().all(char::is_alphabetic) {
                    continue;
                }
                let s = crate::lint_ai::token_seed(&tok);
                if s != head_seed && !def.contains(&s) {
                    def.push(s);
                    def_words.push(tok);
                    if def.len() >= MAX_DEFINITION_WORDS {
                        break;
                    }
                }
            }
            if def.is_empty() {
                continue;
            }
            // Negative morphology: the headword is a BOUND prefix (1–2 chars — longer
            // "prefixes" are compound first-elements like "bread"+"box") over one of its own
            // definition words ("in"+"valid", "un"+"safe"), and the definition states the
            // negation as NEGATOR-then-root ("not valid") — so the word immediately BEFORE
            // the root is the negator candidate. Counted, never assumed: the lift test below
            // keeps only candidates that ride these entries far above their background rate.
            def_entries += 1;
            for s in &def {
                *def_count.entry(*s).or_insert(0) += 1;
            }
            let root_at = def_words.iter().position(|root| {
                root.chars().count() >= 3
                    && head.len() > root.len()
                    && head.len() <= root.len() + 2
                    && head.ends_with(root.as_str())
            });
            if let Some(at) = root_at {
                neg_entries += 1;
                if at > 0 {
                    *neg_count.entry(def[at - 1]).or_insert(0) += 1;
                }
            }
            english.definitions.push((head_seed, def));
        }
        for title in titles {
            for tok in crate::lint_read::tokens(&title) {
                english.defined.insert(crate::lint_ai::token_seed(&tok));
            }
        }
        }
        chunks += 1;
        pos += 4 + outer;
    }
    // Sorted by headword seed for the binary-search hop; the FIRST entry of a duplicated
    // headword wins (dictionaries put the primary sense first).
    english.definitions.sort_by_key(|(k, _)| *k);
    english.definitions.dedup_by(|a, b| a.0 == b.0);
    // A negator rides negative-morphology definitions at ≥3× its background rate with real
    // support — scale-free (rates, not counts), so any dictionary size and any language's
    // prefix system yields its own primitives.
    if neg_entries >= 50 && def_entries > 0 {
        let base = f64::from(neg_entries) / f64::from(def_entries);
        for (seed, n) in &neg_count {
            let total = def_count.get(seed).copied().unwrap_or(*n).max(*n);
            let rate = f64::from(*n) / f64::from(total);
            if *n >= 100 && rate >= 3.0 * base {
                english.negators.push(*seed);
            }
        }
        english.negators.sort_unstable();
    }
    // The ENTIRETY witness (LINTER.md): the walk ended either at the file's end or at the
    // start of its zero padding — anything else means a chunk mid-file failed to decode and
    // the dictionary was only PARTIALLY read. A partial brain is refused: the committed
    // bootstrap (a complete brain) serves until the dictionary reads whole again.
    let whole = data[pos.min(data.len())..].iter().all(|b| *b == 0);
    (whole && chunks > 0 && !english.defined.is_empty()).then_some((english, chunks))
}

/// Reduce dictionary entry XML to (definition prose, headword titles): tags drop, `d:title`
/// attribute values are captured, character entities become spaces (they are punctuation-class
/// for the reader's word runs). No XML library — the format is a flat span soup and the reader
/// only needs word runs.
fn strip_entry_xml(xml: &str) -> (String, Vec<String>) {
    let mut prose = String::with_capacity(xml.len() / 4);
    let mut titles = Vec::new();
    let mut rest = xml;
    // Headword/pronunciation spans are the entry's TYPOGRAPHY, not its meaning — their text
    // (syllabified headwords, respellings) would sit in front of every definition and poison
    // the leading content words the bindings keep. The format marks them; skip their whole
    // subtree by span depth.
    let mut skip_depth = 0usize;
    while let Some(open) = rest.find('<') {
        if skip_depth == 0 {
            prose.push_str(&rest[..open]);
            prose.push(' ');
        }
        let Some(close) = rest[open..].find('>') else { break };
        let tag = &rest[open + 1..open + close];
        if let Some(t) = tag.split("d:title=\"").nth(1).and_then(|a| a.split('"').next()) {
            titles.push(t.to_string());
        }
        let opens_span = tag.starts_with("span");
        let closes_span = tag.starts_with("/span");
        if skip_depth > 0 {
            if opens_span {
                skip_depth += 1;
            } else if closes_span {
                skip_depth -= 1;
            }
        } else if opens_span
            && (tag.contains("class=\"prx") || tag.contains("class=\"syl_txt") || tag.contains("class=\"hw"))
        {
            skip_depth = 1;
        }
        rest = &rest[open + close + 1..];
    }
    if skip_depth == 0 {
        prose.push_str(rest);
    }
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
        let (mut english, _chunks) = read_dictionary(&path, 0).expect("dictionary parses");
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
