//! The character-level substrate (owner directive 2026-07-07: the atom is a UTF-8 character —
//! no token vocabulary, no frequency-by-word table, no cap). The reader ingests a stream of
//! Unicode scalars and learns to PREDICT the next one from the rolling context; the prediction
//! error (SURPRISE) is the one signal everything downstream stands on.
//!
//! This replaces the word-token substrate: "does English account for this?" becomes "does the
//! reader predict this character run with low surprise" (reading the dictionary entirely teaches
//! the model to spell English), and segmentation of a raw documentation page falls out of the
//! same signal — code runs are high-surprise, prose is low-surprise, a section boundary is a
//! surprise spike. No HTML parser, no tag list, no `Gap`.
//!
//! Unlike the old word reader, the prediction memory is PERSISTED (LINTER.md, "Markup second"):
//! a loaded reader must be able to read a page back, so its context→next-char associations ride
//! the artifact. The memory is bounded ([`MEM_CAP`] slots), so the substrate stays small.

use std::collections::HashMap;

use crate::lint_ai::{Bundler, Hv, DIM};

/// Prediction memory capacity — a runaway safety valve, NOT a working limit (owner directive:
/// the brain must REMEMBER IT ALL). The whole dictionary plus the crawled web produces several
/// million distinct backoff contexts; this is sized far above that so the curriculum is never
/// truncated mid-read (measured: 1<<20 filled partway through the dictionary and the web layer
/// learned nothing).
const MEM_CAP: usize = 8 << 20;

/// The neighborhood a character's code-vs-prose vote is taken over (characters). Wide enough to
/// smooth a surprising letter inside a known word, narrow enough to catch a short example.
const SEG_WINDOW: usize = 16;

/// How many characters of calm prose immediately above a code run count as its governing
/// context — the sentence right above the example (interim, dissolves into the sequential read).
const SEG_GOVERN: usize = 320;

/// The longest CALM stretch a code run absorbs — an English-looking fragment inside an
/// identifier (`frobni·cate`) is calm but short; real prose between two examples is longer.
const CODE_GAP: usize = 12;

/// The character-level predictive reader. All operations are 1-bit (XOR / rotate / Hamming);
/// deterministic, no floats in the learned state.
#[derive(Clone, Default)]
pub struct CharReader {
    /// Context address → the SET of next characters seen in that context (capped at
    /// [`SET_CAP`]). Bounded by [`MEM_CAP`]. PERSISTED — a loaded reader reads pages by this
    /// memory. A character is PREDICTED when it is a plausible continuation (in the set), not
    /// only the single last one — English contexts have many valid next characters, so a set is
    /// what makes ordinary prose read calm while novel code (unseen continuations) stays
    /// surprising. Storing chars (not hypervectors) keeps a real brain in the megabytes.
    mem: HashMap<u32, Vec<char>>,
    /// Total characters read — the mass surprise averages are measured against.
    total: u64,
}

/// The most distinct continuations kept per context. A handful captures a context's real
/// branching; an unbounded set would let a short, busy context predict almost anything (every
/// char becomes "plausible") and read code as calm.
const SET_CAP: usize = 12;

/// The shortest backoff context that may decide a prediction. Order 1 (a single preceding
/// character) has so many continuations it predicts nearly everything — too permissive — so
/// backoff stops here: a prediction must rest on real context.
const MIN_ORDER: usize = 2;

/// HLM1 wire form: the prediction memory rides the RAW stream as two u32 arrays (context
/// addresses, predicted scalars) in a deterministic key-sorted order so the artifact is
/// reproducible. PERSISTED (unlike the word reader's), because a loaded brain must read pages
/// back.
impl crate::lint_codec::Bin for CharReader {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        let mut entries: Vec<(&u32, &Vec<char>)> = self.mem.iter().collect();
        entries.sort_by_key(|(k, _)| **k);
        e.fixed_u64(self.total);
        // Keys and per-key set lengths on the RAW stream; the flattened continuation chars
        // follow, decoded back into sets by length.
        e.raw_u32s(&entries.iter().map(|(k, _)| **k).collect::<Vec<_>>());
        e.raw_u32s(&entries.iter().map(|(_, s)| s.len() as u32).collect::<Vec<_>>());
        e.raw_u32s(
            &entries.iter().flat_map(|(_, s)| s.iter().map(|c| *c as u32)).collect::<Vec<_>>(),
        );
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<CharReader> {
        let total = d.fixed_u64()?;
        let keys = d.raw_u32s()?;
        let lens = d.raw_u32s()?;
        let flat = d.raw_u32s()?;
        if keys.len() != lens.len()
            || lens.iter().map(|l| *l as usize).sum::<usize>() != flat.len()
        {
            return None;
        }
        let mut mem = HashMap::with_capacity(keys.len());
        let mut at = 0usize;
        for (k, len) in keys.into_iter().zip(lens) {
            let mut set = Vec::with_capacity(len as usize);
            for c in &flat[at..at + len as usize] {
                set.push(char::from_u32(*c)?);
            }
            at += len as usize;
            mem.insert(k, set);
        }
        Some(CharReader { mem, total })
    }
}

/// The fixed basis: one hypervector per Unicode scalar. No vocabulary is stored — the code is a
/// pure function of the character, so any scalar (any language, any symbol) is encodable, and
/// the "basis" is the character set itself.
pub fn char_hv(c: char) -> Hv {
    // Salt keeps char codes clear of token-seed collisions in shared structures.
    Hv::random((c as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ 0xC0FFEE)
}

/// The longest prediction context, in preceding characters. The model is a BACKOFF n-gram: it
/// predicts from the longest context it has actually seen, falling back to shorter ones. This
/// is what makes prose CALM — common English sub-sequences ("th", "ing", "tion") are always
/// seen, so ordinary prose predicts even when its exact 4-gram is novel — while unfamiliar code
/// misses at every order and stays surprising. A single fixed order made even English mispredict
/// ~63% of characters (measured), which no absolute threshold can separate from code.
const ORDER: usize = 5;

/// The address for the `order` characters preceding position `i` — an FNV hash of those
/// character CODES, mixed with the order so contexts of different lengths never collide. The
/// context is just "the last k characters"; hashing the codes directly is the whole job. (The
/// hypervector machinery is for `encode`'s span vectors, never the predictor — building an
/// 8192-bit vector per context made training minutes instead of seconds.)
fn context_key(chars: &[char], i: usize, order: usize) -> u32 {
    let start = i.saturating_sub(order);
    let mut h = 0xCBF29CE484222325u64;
    for c in &chars[start..i] {
        h ^= *c as u64;
        h = h.wrapping_mul(0x100000001B3);
    }
    ((h ^ (h >> 32)) as u32) ^ (order as u32).wrapping_mul(0x9E3779B1)
}

impl CharReader {
    pub fn new() -> CharReader {
        CharReader { mem: HashMap::new(), total: 0 }
    }

    /// Characters read so far.
    pub fn total_read(&self) -> u64 {
        self.total
    }

    /// Learned context slots — how much spelling the reader has comprehended.
    pub fn learned(&self) -> usize {
        self.mem.len()
    }

    /// READ a span and LEARN from it (predictive coding): at each character, when the memory's
    /// prediction for the current context is wrong or absent, rewrite that one slot to expect
    /// this character. Correct predictions touch nothing.
    pub fn learn(&mut self, text: &str) {
        let chars: Vec<char> = text.chars().collect();
        for i in 0..chars.len() {
            // Record the continuation into the set at every order (MIN_ORDER..=ORDER), so
            // backoff always has a shorter, better-populated context to fall back on.
            for order in MIN_ORDER..=ORDER {
                let key = context_key(&chars, i, order);
                let full = self.mem.len() >= MEM_CAP;
                match self.mem.get_mut(&key) {
                    Some(set) => {
                        if !set.contains(&chars[i]) && set.len() < SET_CAP {
                            set.push(chars[i]);
                        }
                    }
                    None if !full => {
                        self.mem.insert(key, vec![chars[i]]);
                    }
                    None => {}
                }
            }
            self.total += 1;
        }
    }

    /// Whether the backoff model PREDICTED `chars[i]`: the LONGEST seen context decides, and the
    /// character is predicted when it is a plausible continuation there (in the set). Prose the
    /// reader has the sub-sequences for is predicted (calm); novel code, unseen at every order
    /// down to [`MIN_ORDER`], is not (surprise).
    fn predicted(&self, chars: &[char], i: usize) -> bool {
        for order in (MIN_ORDER..=ORDER).rev() {
            if let Some(set) = self.mem.get(&context_key(chars, i, order)) {
                return set.contains(&chars[i]);
            }
        }
        false
    }

    /// The SURPRISE of reading `text` under the learned model, in mean bits of prediction error
    /// per character (0 = every character was predicted exactly; ~DIM/2 = wholly unpredictable).
    /// This is the English-vs-code signal: prose the reader learned scores low, code and unseen
    /// identifiers score high. Read-only — never learns.
    pub fn surprise(&self, text: &str) -> u32 {
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return 0;
        }
        let mut sum = 0u64;
        for i in 0..chars.len() {
            // A predicted character (longest seen context) is 0 surprise; a miss at every order
            // is maximally surprising (half the space) — averaged into the span's English-ness.
            let d = if self.predicted(&chars, i) { 0 } else { (DIM / 2) as u32 };
            sum += u64::from(d);
        }
        (sum / chars.len() as u64) as u32
    }

    /// Per-character prediction outcome under the learned model: `true` where the base
    /// PREDICTED the character (calm — known English/markup), `false` where it missed (a
    /// surprise — novel code). The raw signal segmentation reads.
    fn predictions(&self, chars: &[char]) -> Vec<bool> {
        (0..chars.len()).map(|i| self.predicted(chars, i)).collect()
    }

    /// SEGMENT a raw documentation page into `(governing prose, code example)` units by
    /// reading it NOVEL against this base (owner directive 2026-07-07): the brain knows English
    /// and the web delivery layer, so a page's prose and markup read CALM while an example in a
    /// language the base has not learned reads as a SURPRISE SPIKE — the code is the run the
    /// reader could not predict, and the prose that governs it is the calm text just above.
    /// No parser, no tag list; the reading is the segmentation. Markup tokens inside a code run
    /// are stripped (they are calm islands the brain knows), leaving the code as served.
    pub fn segment(&self, page: &str) -> Vec<(String, String)> {
        let idx: Vec<(usize, char)> = page.char_indices().collect();
        let chars: Vec<char> = idx.iter().map(|(_, c)| *c).collect();
        if chars.is_empty() {
            return Vec::new();
        }
        let predicted = self.predictions(&chars);
        // A character reads as CODE when its neighborhood is mostly mispredicted — a windowed
        // vote smooths single surprising letters in prose and single calm letters in code.
        let half = SEG_WINDOW / 2;
        let mut code_char: Vec<bool> = (0..chars.len())
            .map(|i| {
                let lo = i.saturating_sub(half);
                let hi = (i + half + 1).min(chars.len());
                let missed = (lo..hi).filter(|&j| !predicted[j]).count();
                missed * 2 > (hi - lo) // majority mispredicted
            })
            .collect();
        // Close short CALM gaps inside code: an identifier carries English-looking fragments
        // (`frobni-cate`, `-tion`, `-able`) the brain reads calm, which would split one example
        // into several. A calm stretch shorter than [`CODE_GAP`] flanked by code stays code.
        let mut i = 0usize;
        while i < code_char.len() {
            if code_char[i] {
                i += 1;
                continue;
            }
            let start = i;
            while i < code_char.len() && !code_char[i] {
                i += 1;
            }
            let before_code = start > 0 && code_char[start - 1];
            let after_code = i < code_char.len() && code_char[i];
            if before_code && after_code && i - start <= CODE_GAP {
                for c in &mut code_char[start..i] {
                    *c = true;
                }
            }
        }
        // Maximal code runs → one unit each, prose = the calm text since the previous run.
        let byte_end = |i: usize| idx.get(i + 1).map_or(page.len(), |(b, _)| *b);
        let mut units = Vec::new();
        let mut prev_end = 0usize;
        let mut i = 0usize;
        while i < chars.len() {
            if !code_char[i] {
                i += 1;
                continue;
            }
            let start_b = idx[i].0;
            let mut j = i;
            while j + 1 < chars.len() && code_char[j + 1] {
                j += 1;
            }
            let end_b = byte_end(j);
            let code = crate::doc_crawler::strip_code(&page[start_b..end_b]);
            if code.trim().len() >= 3 {
                let prose = crate::doc_crawler::strip_tags(&page[prev_end..start_b]);
                let prose = prose.chars().rev().take(SEG_GOVERN).collect::<Vec<_>>();
                let prose: String = prose.into_iter().rev().collect();
                units.push((prose.trim().to_string(), code));
            }
            prev_end = end_b;
            i = j + 1;
        }
        units
    }

    /// Encode a span into ONE hypervector self-encoded from its characters: the position-bound
    /// bundle of its character codes (a spelling centroid). Any string — word, identifier,
    /// phrase — maps deterministically; strings that share spelling share geometry. This is the
    /// representation polarity and concept matching stand on.
    pub fn encode(&self, text: &str) -> Option<Hv> {
        let mut b = Bundler::new();
        let mut any = false;
        for (i, c) in text.chars().enumerate() {
            // Position-bound so order matters (`ab` ≠ `ba`); rotation is the positional role.
            b.add(&rotate_by(&char_hv(c), i));
            any = true;
        }
        any.then(|| b.finalize())
    }
}

/// Rotate an Hv left by `k` bits (positional binding) — `k` capped to the dimension.
fn rotate_by(hv: &Hv, k: usize) -> Hv {
    let mut v = *hv;
    for _ in 0..(k % DIM) {
        v = v.rotl1_pub();
    }
    v
}

// ── The cumulative global brain (setup trains it; lint loads it) ──────────────

/// Where the machine's character brain lives, beside the models.
fn store_path() -> std::path::PathBuf {
    crate::lint_train::model_dir_pub().join("char.global.bin")
}

/// Load the machine's character brain (`HLM1`), or `None` when it has not been trained yet.
/// Memoized for the process — the lint path only ever loads.
pub fn brain() -> Option<&'static CharReader> {
    use crate::lint_codec::{Bin, Dec};
    static BRAIN: std::sync::OnceLock<Option<CharReader>> = std::sync::OnceLock::new();
    BRAIN
        .get_or_init(|| {
            std::fs::read(store_path())
                .ok()
                .and_then(|b| Dec::open(&b, crate::lint_codec::kind::CHARBRAIN))
                .and_then(|(_, mut d)| CharReader::dec(&mut d))
        })
        .as_ref()
}

/// Persist a character brain as its `HLM1` container (stamp = characters read, a cheap prefix
/// probe of how much it has learned).
pub fn save(reader: &CharReader) {
    use crate::lint_codec::{Bin, Enc};
    if let Some(dir) = store_path().parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut e = Enc::new();
    reader.enc(&mut e);
    let bytes = e.finish(crate::lint_codec::kind::CHARBRAIN, &reader.total.to_string());
    let _ = std::fs::write(store_path(), bytes);
}

/// Train the brain CUMULATIVELY over a curriculum, in order (owner directive 2026-07-07):
/// English first (the general base), then each web language layered on — one brain that
/// retains what it read and gains what is new. Returns the trained reader; the caller saves it.
/// Each corpus is a raw character stream (dictionary prose, then whole raw pages of html, css,
/// js documentation); the reader reads them all into one memory.
pub fn train_curriculum<'a>(corpora: impl IntoIterator<Item = &'a str>) -> CharReader {
    let mut r = CharReader::new();
    for corpus in corpora {
        r.learn(corpus);
    }
    r
}

/// The web languages the brain reads AFTER English, in curriculum order (owner directive): the
/// page's own delivery language first (html), then how it is styled and scripted (css, js), so
/// that by the time it reads any documentation it already understands the website it arrives
/// in. Data, not a hardcoded truth about the world — these are the languages a documentation
/// PAGE is built from.
const WEB_CURRICULUM: [&str; 3] = ["html", "css", "javascript"];

/// SETUP verb (curriculum): build this machine's character brain if missing or its inputs
/// changed, cumulatively — the whole dictionary (English base), then every raw page of the web
/// curriculum in order. Purely reads what setup already cached/crawled; saves `char.global.bin`.
/// Returns a one-line report, or `None` when there is no dictionary and no cached web pages to
/// learn from. Online only through the shared crawl cache (same latch as every setup read).
#[cfg(feature = "crawl")]
pub fn ensure_brain(data_root: &std::path::Path) -> Option<String> {
    // Freshness: the brain is keyed by the dictionary fingerprint folded with each web
    // language's raw-page fingerprint. Unchanged inputs ⇒ the saved brain is current.
    let english = crate::lint_english::dictionary_prose(None);
    let mut web: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut fp = crate::lint_ai::token_seed(english.as_deref().unwrap_or(""));
    for lang in WEB_CURRICULUM {
        let (pages, lang_fp) = crate::lint_docs::raw_pages(data_root, lang);
        fp ^= lang_fp.rotate_left(7);
        web.push((lang.to_string(), pages));
    }
    if english.is_none() && web.iter().all(|(_, p)| p.is_empty()) {
        return None;
    }
    // Freshness reads the fingerprint sidecar and the file directly (not the memoized
    // `brain()`, which caches its cold-load result for the process): unchanged inputs replay.
    if brain_fp() == Some(fp) && store_path().exists() {
        return Some("character brain: current".to_string());
    }
    let mut r = CharReader::new();
    let mut order: Vec<String> = Vec::new();
    if let Some(prose) = &english {
        let before = r.total;
        r.learn(prose);
        order.push(format!("english {}c", r.total - before));
    }
    for (lang, pages) in &web {
        let before = r.total;
        for (_, body) in pages {
            r.learn(body);
        }
        if r.total > before {
            order.push(format!("{lang} {}c", r.total - before));
        }
    }
    save(&r);
    save_brain_fp(fp);
    Some(format!(
        "character brain: read {} chars, {} contexts — curriculum: {}",
        r.total_read(),
        r.learned(),
        order.join(" → "),
    ))
}

#[cfg(not(feature = "crawl"))]
pub fn ensure_brain(_data_root: &std::path::Path) -> Option<String> {
    None
}

/// The input fingerprint the saved brain was trained under (a sidecar beside the brain), so an
/// unchanged curriculum replays instead of retraining.
fn fp_path() -> std::path::PathBuf {
    store_path().with_extension("fp")
}
fn brain_fp() -> Option<u64> {
    std::fs::read_to_string(fp_path()).ok()?.trim().parse().ok()
}
fn save_brain_fp(fp: u64) {
    let _ = std::fs::write(fp_path(), fp.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// English prose the reader has read must be far LESS surprising than code it has not —
    /// the hermetic floor of the whole rewrite (the real-dictionary gate is the ignored test
    /// below). The reader trains on English sentences, then judges unseen English vs code.
    #[test]
    fn english_reads_calmer_than_code() {
        let mut r = CharReader::new();
        // A small English corpus — ordinary words, ordinary letter sequences.
        let corpus = "the quick brown fox jumps over the lazy dog. \
             a function returns a value to the caller when it is done. \
             the reader learns to predict the next letter in a word. \
             common english words have common letter sequences and endings. \
             she sells sea shells by the sea shore in the summer season. ";
        // Read it enough times that the letter statistics settle.
        for _ in 0..40 {
            r.learn(corpus);
        }
        let prose = r.surprise("the reader returns a common value to the caller");
        let code = r.surprise("xq7_frobnicate(items[0], cfg={k:v}); z9$->w=~q;");
        assert!(
            prose + prose / 2 < code,
            "English prose ({prose} bits) must read far calmer than code ({code} bits)"
        );
        // A single English-shaped word vs a symbol-laden identifier.
        assert!(
            r.surprise("season") < r.surprise("z9$_qx=[]"),
            "an English word ({}) reads calmer than a code token ({})",
            r.surprise("season"),
            r.surprise("z9$_qx=[]"),
        );
    }

    /// The payoff, on the REAL curriculum (no hand-fed corpora — owner directive): read the
    /// whole dictionary (English), then CRAWL the web standards (W3/WHATWG + MDN html) to learn
    /// the delivery layer, then segment a page whose example is in a language the brain never
    /// read — the novel code is the run it cannot predict, and no parser runs. Ignored (reads
    /// the local dictionary and crawls). Run:
    /// `cargo test --release --lib base_segments_novel_code -- --ignored --nocapture`
    #[cfg(feature = "crawl")]
    #[test]
    #[ignore = "reads the local dictionary and CRAWLS W3/MDN; the real end-to-end segmentation gate"]
    fn base_segments_novel_code_out_of_a_raw_page_by_surprise() {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let prose = crate::lint_english::dictionary_prose(None).expect("a readable dictionary");
        let mut r = CharReader::new();
        r.learn(&prose); // English base — the whole dictionary
        eprintln!("english base: {} chars, {} contexts", r.total_read(), r.learned());
        // Learn the web by CRAWLING the standards — bounded for the gate, real pages.
        for lang in ["html", "css"] {
            let mut n = 0usize;
            for src in crate::lint_train::registered_docs_sources(data, lang) {
                for p in crate::doc_crawler::crawl(&[&src.url], 300, 0) {
                    r.learn(&p.html);
                    n += 1;
                }
            }
            eprintln!("web: read {n} {lang} pages → {} contexts", r.learned());
        }
        // A page in the delivery layer the brain knows, with an example in a language it never
        // read — its identifiers and operators are the surprise the reader segments out.
        let page = "<h1>Statements</h1>\
            <p>Never use the goto statement; prefer a structured loop instead.</p>\
            <pre>@zblorp$ qux := frobnicate(&items[0x1F], ~cfg._k) |&gt; wibble;;</pre>\
            <p>the value is returned to the caller when the work is complete</p>";
        let units = r.segment(page);
        eprintln!("segments: {units:#?}");
        assert_eq!(units.len(), 1, "one code example segments out: {units:?}");
        let (prose, code) = &units[0];
        assert!(code.contains("frobnicate"), "the novel code is the surprise run: {code:?}");
        assert!(prose.to_lowercase().contains("goto"), "the calm prose above governs it: {prose:?}");
    }

    /// END-TO-END setup: `ensure_brain` reads this machine's dictionary (English base — no web
    /// sources registered in the temp root, so the curriculum is English alone here), trains,
    /// saves `char.global.bin`, and on a second call REPLAYS ("current") instead of retraining.
    /// Proves the whole setup verb runs on real data. Ignored (needs the local dictionary).
    #[test]
    #[ignore = "reads the local dictionary; exercises the ensure_brain setup verb end to end"]
    fn ensure_brain_trains_saves_and_replays() {
        use crate::lint_codec::{Dec, Bin};
        let dir = std::env::temp_dir().join(format!("char-brain-e2e-{}", std::process::id()));
        let _env = crate::test_env_lock();
        std::env::set_var("HELPERS_LINT_MODELS", &dir);
        std::env::set_var("HELPERS_LINT_OFFLINE", "1"); // no web crawl — dictionary only
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let first = super::ensure_brain(data).expect("a dictionary exists to train from");
        eprintln!("{first}");
        assert!(first.contains("read") && first.contains("english"), "trained report: {first}");
        // The saved brain decodes and has really read.
        let bytes = std::fs::read(super::store_path()).expect("brain saved");
        let (_, mut d) = Dec::open(&bytes, crate::lint_codec::kind::CHARBRAIN).expect("opens");
        let loaded = CharReader::dec(&mut d).expect("decodes");
        assert!(loaded.total_read() > 1_000_000, "read the dictionary: {} chars", loaded.total_read());
        // Second call replays.
        let second = super::ensure_brain(data).expect("still trainable");
        assert_eq!(second, "character brain: current", "unchanged inputs replay: {second}");
        std::env::remove_var("HELPERS_LINT_OFFLINE");
        std::env::remove_var("HELPERS_LINT_MODELS");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The persisted brain round-trips exactly — a loaded brain reads pages identically to the
    /// one that trained (the property that lets segmentation stand on a saved reader).
    #[test]
    fn brain_round_trips_through_hlm1() {
        use crate::lint_codec::{Bin, Dec, Enc};
        let mut r = CharReader::new();
        for _ in 0..20 {
            r.learn("the reader predicts the next character in a common english word. ");
        }
        let probe = "the reader predicts a common word";
        let before = r.surprise(probe);
        let mut e = Enc::new();
        r.enc(&mut e);
        let bytes = e.finish(crate::lint_codec::kind::CHARBRAIN, "t");
        let (_, mut d) = Dec::open(&bytes, crate::lint_codec::kind::CHARBRAIN).expect("opens");
        let loaded = CharReader::dec(&mut d).expect("decodes");
        assert_eq!(loaded.total_read(), r.total_read());
        assert_eq!(loaded.surprise(probe), before, "a loaded brain reads identically");
    }

    /// The CURRICULUM property (owner directive 2026-07-07): one brain, trained cumulatively —
    /// reading HTML must RETAIN the English it already learned while GAINING HTML. Because
    /// learning only adds context→prediction slots, prior knowledge is never overwritten; new
    /// structure layers on top. This is what lets the brain read English → HTML → CSS → JS and
    /// then read any documentation directly.
    #[test]
    fn reading_html_retains_english_and_gains_html() {
        let english = "the quick brown fox jumps over the lazy dog. a function returns a \
            value to the caller. common english words share common letter sequences. she \
            sells sea shells by the sea shore every single summer season without fail. ";
        let html = "<div class=\"box\"><p>the value</p></div><span id=\"x\">text</span>\
            <ul><li>one</li><li>two</li></ul><a href=\"/page\">link</a><h2>Title</h2>";
        let mut r = CharReader::new();
        for _ in 0..40 {
            r.learn(english);
        }
        let e0 = r.surprise("a common function returns the value to the caller");
        let h0 = r.surprise("<div class=\"row\"><p>hello</p></div>");
        // Now layer HTML on top — English is never re-read.
        for _ in 0..40 {
            r.learn(html);
        }
        let e1 = r.surprise("a common function returns the value to the caller");
        let h1 = r.surprise("<div class=\"row\"><p>hello</p></div>");
        // RETAINS: English surprise barely moves (knowledge is not overwritten).
        assert!(e1 <= e0 + e0 / 8, "English retained: was {e0}, now {e1}");
        // GAINS: HTML surprise drops materially (new structure learned).
        assert!(h1 + h1 / 4 < h0, "HTML learned: was {h0}, now {h1}");
    }

    /// THE CURRICULUM, on real data (owner directive 2026-07-07): reading is uniform character
    /// prediction, English is the general base, and each language is the SAME method continued
    /// from there. Code is surprising only UNTIL the reader reads it — so the property to prove
    /// is not a permanent English/code split but that training on a language DROPS its surprise
    /// while English is RETAINED. Ignored (needs the local dictionary); run:
    /// `cargo test --release --lib char_curriculum_on_real_dictionary -- --ignored --nocapture`
    #[test]
    #[ignore = "reads the local dictionary; demonstrates the cumulative curriculum on real data"]
    fn char_curriculum_on_real_dictionary() {
        let prose = crate::lint_english::dictionary_prose_sample()
            .expect("this machine has a readable dictionary");
        let mut r = CharReader::new();
        r.learn(&prose); // the general base: English
        eprintln!("English base: {} chars, {} slots", r.total_read(), r.learned());
        let english = "the value is returned to the caller when the function is done";
        let code_sample = "let x = arr[0]; for (const k of items) { obj[k] = fn(k) ?? 0; } \
            function step(a, b) { return a === b ? a : b; } const y = eval(cfg._k);";
        let e_before = r.surprise(english);
        let c_before = r.surprise("const z = arr[1] ?? fn(items[0]);");
        eprintln!("before code: english ~{e_before}  code ~{c_before}");
        // Continue the SAME training on code — the specific layered onto the general.
        for _ in 0..200 {
            r.learn(code_sample);
        }
        let e_after = r.surprise(english);
        let c_after = r.surprise("const z = arr[1] ?? fn(items[0]);");
        eprintln!("after code:  english ~{e_after}  code ~{c_after}");
        assert!(c_after + c_after / 8 < c_before, "the language was learned: code {c_before} -> {c_after}");
        assert!(e_after <= e_before + e_before / 8, "English retained: {e_before} -> {e_after}");
    }
}
