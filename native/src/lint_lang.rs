//! The human-language I/O overlay (LINTER.md, "The human-language I/O overlay").
//!
//! Code languages are constant; the INPUT and OUTPUT human languages are malleable. The linter
//! reasons in the language-agnostic CONCEPT graph ([`crate::lint_char::MeaningNetwork`], built by
//! reading the English dictionary). A second human language is an OVERLAY on that same graph — a
//! bilingual LEXICON mapping its words to the SAME concepts, READ from a real bilingual dictionary
//! as pure DATA (no French word list lives in this file; delete the data and the overlay goes dark).
//!
//! - INPUT: [`Lexicon::overlay_into`] binds each foreign headword into the meaning network AS its
//!   English gloss words, so a foreign word lands next to its English synonym on the identical graph
//!   and foreign documentation reads through the SAME rules.
//! - OUTPUT: [`Lexicon::render`] glosses a finding's words back to the foreign language through the
//!   same lexicon (a reverse, primary-sense index). A word the dictionary does not carry stays
//!   English — reported, never faked. This is a CONCEPT/WORD-level overlay, deliberately NOT
//!   full-sentence grammatical translation.
//!
//! English is the zero-config default: with no I/O language selected the overlay never loads and
//! output is byte-for-byte unchanged.

use std::path::{Path, PathBuf};

/// A bilingual human-language overlay: a foreign↔English concept lexicon read from DATA.
///
/// Both directions ride ONE dictionary. `forward` (foreign → English gloss) is the INPUT map that
/// binds foreign words onto the English concept graph; `reverse` (English → foreign, primary-sense)
/// is the OUTPUT map that renders a finding in the foreign language. Both are sorted for
/// binary-search lookup.
pub struct Lexicon {
    /// The I/O language code this lexicon serves (e.g. `"fr"`).
    lang: String,
    /// Foreign headword → its English gloss words (lowercased), sorted by headword.
    forward: Vec<(String, Vec<String>)>,
    /// English word → the foreign headword whose PRIMARY translation it is (lowercased), sorted by
    /// English word. Inherently lossy (two foreign words may share one English sense); the
    /// primary-sense pick is documented as such.
    reverse: Vec<(String, String)>,
    /// Human-readable citation of the DATA source, surfaced on every translated run.
    source: String,
}

impl Lexicon {
    /// Canonicalize an I/O-language SETTING to its lexicon code, or `None` for the English default.
    /// `""`, `"english"`, and `"en"` mean "no overlay" (identical output); `"fr"`/`"french"`/`"fra"`
    /// all map to `"fr"`. Any other non-empty token passes through lowercased so a future
    /// `lang/<code>.tsv` needs no code change.
    pub fn lang_code(setting: &str) -> Option<String> {
        match setting.trim().to_ascii_lowercase().as_str() {
            "" | "english" | "en" | "eng" => None,
            "fr" | "french" | "fra" | "français" | "francais" => Some("fr".to_string()),
            other => Some(other.to_string()),
        }
    }

    /// The selected I/O language for a run: the `HELPERS_LINT_LANG` env override first (for the
    /// demo/one-off), else the project's configured `io_language`. `None` ⇒ English default.
    pub fn selected(config_io_language: &str) -> Option<String> {
        let env = std::env::var("HELPERS_LINT_LANG").unwrap_or_default();
        let pick = if env.trim().is_empty() { config_io_language } else { &env };
        Self::lang_code(pick)
    }

    /// Resolve the lexicon data file for `lang`: the machine cache `<data_root>/lang/<lang>.tsv`
    /// first (where an install or a one-time TEI parse writes it), else the crate's committed asset
    /// `assets/lang/<lang>.tsv` (present in dev/test builds). `None` when neither exists.
    fn data_file(data_root: &Path, lang: &str) -> Option<PathBuf> {
        let cached = data_root.join("lang").join(format!("{lang}.tsv"));
        if cached.exists() {
            return Some(cached);
        }
        let asset = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("lang").join(format!("{lang}.tsv"));
        asset.exists().then_some(asset)
    }

    /// LOAD the overlay for `lang` from its cited data file, or `None` when no lexicon is installed
    /// (the caller then stays in English). Reads the file as DATA exactly as `lint_english` reads
    /// the machine dictionary.
    pub fn load(data_root: &Path, lang: &str) -> Option<Lexicon> {
        let path = Self::data_file(data_root, lang)?;
        let text = std::fs::read_to_string(&path).ok()?;
        let mut lex = Self::parse(&text, lang);
        if lex.forward.is_empty() {
            return None;
        }
        lex.source = format!("{} lexicon ⟨{}⟩", lang, path.file_name().and_then(|n| n.to_str()).unwrap_or("lang.tsv"));
        Some(lex)
    }

    /// PARSE a `foreign⇥english | english …` TSV (with `#` comment/header lines) into the lexicon.
    /// The English gloss column is split on `|` and whitespace into individual concept words; the
    /// reverse index keeps, for each English word, the foreign headword whose PRIMARY (lowest-rank)
    /// gloss it is, tie-broken by the shorter headword for determinism.
    pub fn parse(text: &str, lang: &str) -> Lexicon {
        let mut forward: Vec<(String, Vec<String>)> = Vec::new();
        // English word -> (foreign headword, sense rank) — the running best primary-sense pick.
        let mut reverse_best: std::collections::HashMap<String, (String, usize)> = std::collections::HashMap::new();
        for line in text.lines() {
            let line = line.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((fr, gloss)) = line.split_once('\t') else { continue };
            let fr = fr.trim().to_lowercase();
            if fr.is_empty() {
                continue;
            }
            let english: Vec<String> = gloss
                .split(['|', ' ', ',', ';', '\t'])
                .map(|w| w.trim().to_lowercase())
                .filter(|w| w.chars().count() >= 2 && w.chars().all(|c| c.is_alphabetic()))
                .collect();
            if english.is_empty() {
                continue;
            }
            for (rank, eng) in english.iter().enumerate() {
                let better = match reverse_best.get(eng) {
                    None => true,
                    Some((cur_fr, cur_rank)) => rank < *cur_rank || (rank == *cur_rank && fr.len() < cur_fr.len()),
                };
                if better {
                    reverse_best.insert(eng.clone(), (fr.clone(), rank));
                }
            }
            forward.push((fr, english));
        }
        forward.sort_by(|a, b| a.0.cmp(&b.0));
        forward.dedup_by(|a, b| a.0 == b.0);
        let mut reverse: Vec<(String, String)> = reverse_best.into_iter().map(|(eng, (fr, _))| (eng, fr)).collect();
        reverse.sort_by(|a, b| a.0.cmp(&b.0));
        Lexicon { lang: lang.to_string(), forward, reverse, source: String::new() }
    }

    /// The English gloss words of a foreign headword (the INPUT concept anchor), or `None`.
    pub fn gloss(&self, foreign: &str) -> Option<&[String]> {
        let key = foreign.to_lowercase();
        self.forward.binary_search_by(|(k, _)| k.as_str().cmp(key.as_str())).ok().map(|i| self.forward[i].1.as_slice())
    }

    /// The foreign word whose primary sense is `english` (the OUTPUT rendering), or `None` when the
    /// bilingual dictionary does not carry it (the caller then leaves the word in English).
    pub fn foreign_for(&self, english: &str) -> Option<&str> {
        let key = english.to_lowercase();
        self.reverse.binary_search_by(|(k, _)| k.as_str().cmp(key.as_str())).ok().map(|i| self.reverse[i].1.as_str())
    }

    /// OVERLAY every foreign headword onto the concept graph: bind it into `net` AS its English
    /// gloss words, so the foreign word measures close (via `related`) to its English synonym on the
    /// identical graph. Seals the network afterward (the fold is additive — retain-and-grow — so a
    /// dictionary-loaded network keeps its English bindings and only GAINS the foreign ones).
    pub fn overlay_into(&self, net: &mut crate::lint_char::MeaningNetwork) {
        for (foreign, english) in &self.forward {
            let refs: Vec<&str> = english.iter().map(String::as_str).collect();
            net.bind(foreign, &refs);
        }
        net.seal();
    }

    /// RENDER English finding text in the foreign language, word by word: each alphabetic word is
    /// replaced by its foreign primary-sense translation when the lexicon carries it, preserving the
    /// original leading-capital shape; punctuation, numbers, backtick-quoted code spans, and words
    /// the dictionary lacks pass through unchanged. Concept/word-level only — foreign word order and
    /// agreement are not modeled (an honest, documented limit).
    pub fn render(&self, english_text: &str) -> String {
        let mut out = String::with_capacity(english_text.len());
        let mut in_code = false;
        let mut word = String::new();
        let flush = |word: &mut String, out: &mut String, lex: &Lexicon| {
            if word.is_empty() {
                return;
            }
            let lower = word.to_lowercase();
            match lex.foreign_for(&lower) {
                Some(fr) if fr != lower => {
                    let leads_upper = word.chars().next().is_some_and(char::is_uppercase);
                    if leads_upper {
                        let mut cs = fr.chars();
                        if let Some(first) = cs.next() {
                            out.extend(first.to_uppercase());
                            out.push_str(cs.as_str());
                        }
                    } else {
                        out.push_str(fr);
                    }
                }
                _ => out.push_str(word),
            }
            word.clear();
        };
        for c in english_text.chars() {
            if c == '`' {
                flush(&mut word, &mut out, self);
                in_code = !in_code;
                out.push(c);
                continue;
            }
            if in_code {
                out.push(c);
                continue;
            }
            if c.is_alphabetic() {
                word.push(c);
            } else {
                flush(&mut word, &mut out, self);
                out.push(c);
            }
        }
        flush(&mut word, &mut out, self);
        out
    }

    /// The I/O language code this lexicon serves.
    pub fn lang(&self) -> &str {
        &self.lang
    }

    /// How many foreign headwords the lexicon carries.
    pub fn len(&self) -> usize {
        self.forward.len()
    }

    /// Whether the lexicon bound nothing.
    pub fn is_empty(&self) -> bool {
        self.forward.is_empty()
    }

    /// The DATA-source citation for this lexicon (surfaced on a translated run).
    pub fn source(&self) -> &str {
        &self.source
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint_char::MeaningNetwork;

    /// A tiny hermetic English "dictionary" (headword⇥definition words) plus a French overlay, so
    /// the overlay's graph binding is proven WITHOUT the machine dictionary.
    fn english_net() -> MeaningNetwork {
        let mut net = MeaningNetwork::new();
        // English headwords bound to their definition content words (as ensure_brain does).
        for (head, def) in [
            ("avoid", "keep away from prevent shun evade"),
            ("evade", "avoid escape elude slip away"),
            ("dog", "domestic carnivore animal pet bark"),
            ("stone", "hard mineral rock solid"),
        ] {
            let words: Vec<&str> = def.split_whitespace().collect();
            net.bind(head, &words);
        }
        net.seal();
        net
    }

    fn french_overlay() -> Lexicon {
        // A hermetic slice of the FreeDict shape: french⇥english gloss.
        Lexicon::parse("éviter\tavoid | evade\nchien\tdog\npierre\tstone | rock\n", "fr")
    }

    #[test]
    fn overlay_binds_french_to_english_concept() {
        let mut net = english_net();
        french_overlay().overlay_into(&mut net);
        // The French word now measures FAR closer to its English synonym than to an unrelated word.
        let near = net.related("éviter", "avoid");
        let far = net.related("éviter", "dog");
        assert!(near < far, "éviter↔avoid ({near}) should beat éviter↔dog ({far})");
        // And closer to avoid than chien is to avoid (the overlay put éviter in avoid's neighborhood).
        assert!(near < net.related("chien", "avoid"), "éviter should be nearer avoid than chien is");
    }

    #[test]
    fn forward_and_reverse_lookup() {
        let lex = french_overlay();
        assert_eq!(lex.gloss("éviter").unwrap(), ["avoid".to_string(), "evade".to_string()]);
        assert_eq!(lex.foreign_for("avoid"), Some("éviter"));
        assert_eq!(lex.foreign_for("dog"), Some("chien"));
        assert_eq!(lex.foreign_for("nonexistent"), None);
    }

    #[test]
    fn render_glosses_known_words_and_keeps_the_rest() {
        let lex = Lexicon::parse("jamais\tnever\néviter\tavoid\nerreur\terror | mistake\n", "fr");
        // Known words translate; unknown words and `code` spans stay; leading capital preserved.
        let out = lex.render("Never avoid the `error` handler xyz.");
        assert!(out.starts_with("Jamais éviter"), "got: {out}");
        assert!(out.contains("`error`"), "code span must be preserved: {out}");
        assert!(out.contains("xyz"), "unknown word must survive: {out}");
    }

    #[test]
    fn english_default_selects_no_overlay() {
        assert_eq!(Lexicon::lang_code(""), None);
        assert_eq!(Lexicon::lang_code("english"), None);
        assert_eq!(Lexicon::lang_code("fr").as_deref(), Some("fr"));
        assert_eq!(Lexicon::lang_code("french").as_deref(), Some("fr"));
    }
}
