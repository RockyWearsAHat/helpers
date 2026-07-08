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

/// Revision of the brain's LEARNED CONTENT (distinct from the container format byte): bumped when
/// the curriculum or the meaning-network binding scheme changes. It is folded into the freshness
/// fingerprint, so every stale `char.global.bin` rebuilds instead of decoding a layout it
/// predates. 1: the dictionary meaning network ([`MeaningNetwork`]) rides the artifact. 2: the
/// learned structural roles ([`StructureRoles`], the page-reading register association) ride it.
/// 3: roles read word English-ness through morphology ([`crate::lint_graph::word_is_english`]).
/// 4: the learned title-shape ceiling rides [`StructureRoles`] beside the roles.
/// 5: the meaning network binds the WHOLE dictionary — multi-word headwords included, all senses
/// folded ([`MeaningNetwork`], owner directive 2026-07-07) — so every stale brain rebinds it.
/// 6: the web curriculum is read DEDUPED ([`novel_blocks`], owner directive 2026-07-08) — repeated
/// crawl chrome is learned once, not 20 000×, so the corpus is representative and the read is
/// seconds; every stale brain rebuilds on the smaller, correct corpus.
const BRAIN_REV: u64 = 6;

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
    /// The dictionary MEANING NETWORK bound during the same read (LINTER.md, "The dictionary
    /// meaning network"): each headword wired to the words of its definition, the comprehension
    /// backbone `meaning_of`/`related` query. PERSISTED with the reader — a loaded brain answers
    /// what a word MEANS, not only how it is spelled.
    meanings: MeaningNetwork,
    /// The learned STRUCTURAL ROLES (LINTER.md, "Reading a page is UNDERSTANDING"): the register
    /// the brain saw follow each markup token across the web curriculum, an association keyed by
    /// the element's own characters. This is what parts a section title from a code example when
    /// meaning alone cannot. PERSISTED with the reader — a loaded brain reads pages by it.
    structure: StructureRoles,
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
        // The meaning network rides the same artifact, after the prediction memory; the learned
        // structural roles follow it.
        self.meanings.enc(e);
        self.structure.enc(e);
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
        let meanings = crate::lint_codec::Bin::dec(d)?;
        let structure = crate::lint_codec::Bin::dec(d)?;
        Some(CharReader { mem, total, meanings, structure })
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
        CharReader {
            mem: HashMap::new(),
            total: 0,
            meanings: MeaningNetwork::new(),
            structure: StructureRoles::new(),
        }
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
        word_vector(text)
    }

    /// The MEANING of `word` from the dictionary meaning network the brain read — see
    /// [`MeaningNetwork::meaning_of`].
    pub fn meaning_of(&self, word: &str) -> Option<Hv> {
        self.meanings.meaning_of(word)
    }

    /// Whether the dictionary meaning network accounts for `word` — the cheap existence query, no
    /// hypervector rebuilt (see [`MeaningNetwork::has`]). Equivalent to `meaning_of(word).is_some()`
    /// but a binary search instead of bundling a dozen 8192-bit vectors; this is what the per-word
    /// English judgment ([`crate::lint_graph::word_is_english`]) calls over the whole corpus.
    pub fn has_meaning(&self, word: &str) -> bool {
        self.meanings.has(word)
    }

    /// The proximity of two words' meanings — see [`MeaningNetwork::related`].
    pub fn related(&self, a: &str, b: &str) -> u32 {
        self.meanings.related(a, b)
    }

    /// The dictionary meaning network the brain read (headword→definition bindings).
    pub fn meanings(&self) -> &MeaningNetwork {
        &self.meanings
    }

    /// The meaning network, mutably — setup binds the dictionary through it, and offline tests
    /// build a small fixture network. The lint path never mutates it.
    pub fn meanings_mut(&mut self) -> &mut MeaningNetwork {
        &mut self.meanings
    }

    /// The LEARNED register role of the markup element whose name hashes to `seed`
    /// (see [`StructureRoles::role_of`]): `Some(true)` a code carrier (its contents are code
    /// however English their words), `Some(false)` a section heading (a boundary), `None` an
    /// element the reading found no decisive role for. The page reader ([`crate::lint_graph`])
    /// asks this before it asks meaning.
    pub fn structure_role(&self, seed: u64) -> Option<bool> {
        self.structure.role_of(seed)
    }

    /// Install the structural roles learned by exposure over the web curriculum
    /// ([`crate::lint_graph::learn_structure_roles`]) — setup only; the lint path never learns.
    pub fn set_structure(&mut self, roles: StructureRoles) {
        self.structure = roles;
    }

    /// How many markup elements carry a learned register role — the read witness for setup.
    pub fn roles_learned(&self) -> usize {
        self.structure.len()
    }

    /// The learned page structure — the committed structure bootstrap is generated from this (a
    /// machine that read the whole web curriculum), and hydrated back on a machine that could not
    /// (the same fallback the english and markup substrates keep).
    pub fn structure(&self) -> &StructureRoles {
        &self.structure
    }

    /// The learned title-shape ceiling in words — the section-heading shape fallback the reader
    /// uses when a heading element earned no role of its own.
    pub fn title_ceiling(&self) -> u32 {
        self.structure.title_ceiling()
    }

    /// Ensure the reader can read a page's structure: if the curriculum read no web pages (an
    /// offline or localhost-only machine, so no roles were learned), hydrate the roles from the
    /// committed bootstrap — the meaning network is still the brain's own local dictionary read.
    /// Setup and the load path both call this, so a role-less brain never reaches the reader.
    pub fn ensure_structure(&mut self) {
        if self.structure.is_empty() {
            if let Some(roles) = structure_bootstrap() {
                self.structure = roles;
            }
        }
    }
}

/// The committed structural-roles bootstrap (`lint-index/char-structure-bootstrap.json`) — the
/// learned register roles, machine-generated by [`generate_char_structure_bootstrap`] on a machine
/// that read the whole web curriculum, so a machine that cannot crawl the web still reads pages by
/// role. Small learned data (a few dozen `seed → ±1` votes), the same committed-fallback pattern
/// the english and markup substrates keep. `None` when the artifact is absent.
fn structure_bootstrap() -> Option<StructureRoles> {
    let text = crate::lint_train::embedded_lint_index_file("char-structure-bootstrap.json")?;
    let roles: StructureRoles = serde_json::from_str(&text).ok()?;
    (!roles.is_empty()).then_some(roles)
}

/// The char-level vector of a string (its spelling centroid): each character's [`char_hv`]
/// rotated by its position and majority-bundled, so order matters (`ab` ≠ `ba`) and strings that
/// share spelling share geometry. PURE — no learned state — which is why the meaning network can
/// rebind a word's meaning identically on every machine. `None` for the empty string.
fn word_vector(text: &str) -> Option<Hv> {
    let mut b = Bundler::new();
    let mut any = false;
    for (i, c) in text.chars().enumerate() {
        // Position-bound so order matters (`ab` ≠ `ba`); rotation is the positional role.
        b.add(&rotate_by(&char_hv(c), i));
        any = true;
    }
    any.then(|| b.finalize())
}

/// The pure spelling centroid of `word` ([`word_vector`]) — exposed so the concept binding
/// ([`crate::lint_probe`]) has an always-available HDC encoder when no dictionary brain is
/// loaded. `None` for the empty string.
pub fn spell_vector(word: &str) -> Option<Hv> {
    word_vector(word)
}

// ── The dictionary meaning network (LINTER.md, "The dictionary meaning network") ──

/// Definition content-word cap per headword — the leading words of a dictionary definition carry
/// its sense (dictionaries front-load the genus); the tail is examples and cross-references. The
/// same bound the word substrate keeps ([`crate::lint_english`]), so the two meaning views agree.
pub const MAX_MEANING_WORDS: usize = 12;

/// The dictionary MEANING NETWORK: every headword the dictionary defines — single words AND
/// multi-word phrases (`give up`) — bound to the words of its own definition, so a word's MEANING
/// can be rebound on demand and two words compared by how much their definitions overlap. This is the comprehension backbone the char substrate reads
/// prohibition off of — negation meaning EMERGES from the dictionary's own definitions (negation
/// words are written in shared negative vocabulary, so their meanings cluster), never a hand list
/// of negation words.
///
/// Storage is bounded and delta-honest: only the compact headword→definition-word list rides the
/// artifact (deflated strings, capped at [`MAX_MEANING_WORDS`] per entry — a few MB, not one 1KB
/// hypervector per headword), and the meaning vector is REBOUND from those words on query.
#[derive(Clone, Default)]
pub struct MeaningNetwork {
    /// Headword token seed → its definition's leading content words. Sorted by seed once
    /// [`seal`](Self::seal)ed, so [`meaning_of`](Self::meaning_of) answers by binary search.
    defs: Vec<(u64, Vec<String>)>,
}

impl MeaningNetwork {
    /// An empty network.
    pub fn new() -> MeaningNetwork {
        MeaningNetwork { defs: Vec::new() }
    }

    /// BIND a headword to the words of its definition (retain-and-grow: appended, folded at
    /// [`seal`](Self::seal); a headword's senses are UNIONED there, primary first). Words are
    /// lowercased and filtered to alphabetic runs of ≥2 characters, capped at
    /// [`MAX_MEANING_WORDS`]. An empty definition binds nothing. Multi-word headwords (`give up`)
    /// are bound too — the caller keys them by their own space-joined token seed.
    pub fn bind(&mut self, headword: &str, def_words: &[&str]) {
        let words: Vec<String> = def_words
            .iter()
            .map(|w| w.to_lowercase())
            .filter(|w| w.chars().count() >= 2 && w.chars().all(char::is_alphabetic))
            .take(MAX_MEANING_WORDS)
            .collect();
        if words.is_empty() {
            return;
        }
        self.defs.push((crate::lint_ai::token_seed(&headword.to_lowercase()), words));
    }

    /// FINISH building: sort by headword seed (stably, so the primary/first-read sense leads) and
    /// FOLD every sense of a duplicated headword into one meaning — the later senses' words are
    /// UNIONED onto the primary's, up to [`MAX_MEANING_WORDS`], so a word's meaning reflects its
    /// whole dictionary range. Queries then answer by binary search over one entry per headword.
    /// Idempotent, and retain-and-grow: the fold only ever ADDS words to a binding (the primary
    /// sense's words stay, in front), never overwrites — so binding more material and sealing
    /// again grows meanings without losing any.
    pub fn seal(&mut self) {
        self.defs.sort_by_key(|(k, _)| *k);
        let mut folded: Vec<(u64, Vec<String>)> = Vec::with_capacity(self.defs.len());
        for (seed, words) in self.defs.drain(..) {
            match folded.last_mut() {
                Some((last_seed, meaning)) if *last_seed == seed => {
                    for w in words {
                        if meaning.len() >= MAX_MEANING_WORDS {
                            break;
                        }
                        if !meaning.contains(&w) {
                            meaning.push(w);
                        }
                    }
                }
                _ => folded.push((seed, words)),
            }
        }
        self.defs = folded;
    }

    /// How many headwords carry a bound meaning.
    pub fn len(&self) -> usize {
        self.defs.len()
    }

    /// Whether the network bound nothing.
    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    /// The definition words bound to `word`, or `None` when the dictionary bound none (a
    /// construct, a multi-word phrase, an unread word). Requires a [`sealed`](Self::seal) network.
    fn definition(&self, word: &str) -> Option<&[String]> {
        let seed = crate::lint_ai::token_seed(&word.to_lowercase());
        self.defs.binary_search_by_key(&seed, |(k, _)| *k).ok().map(|i| self.defs[i].1.as_slice())
    }

    /// Whether `word` has a bound meaning — the cheap EXISTENCE query (a binary search over the
    /// sealed seeds), with none of the per-word hypervector rebinding [`meaning_of`](Self::meaning_of)
    /// does. Exactly equivalent to `meaning_of(word).is_some()` (a bound definition is never empty —
    /// [`bind`](Self::bind) drops empty ones — so its meaning vector is always non-empty), so a
    /// caller that only asks "does English account for this word" pays a compare, not a bundle of a
    /// dozen 8192-bit vectors. This is the hot query the structure-role learner and construct
    /// selection ask per word over the whole corpus.
    pub fn has(&self, word: &str) -> bool {
        self.definition(word).is_some()
    }

    /// The MEANING of `word`: its definition's content words each [`encode`](CharReader::encode)d
    /// (the char-level spelling centroid) and majority-bundled into one hypervector. PURE and
    /// STABLE — the same word always rebinds the same vector, so a round-tripped network answers
    /// identically. `None` when no definition is bound.
    pub fn meaning_of(&self, word: &str) -> Option<Hv> {
        let words = self.definition(word)?;
        let mut b = Bundler::new();
        for w in words {
            if let Some(v) = word_vector(w) {
                b.add(&v);
            }
        }
        (!b.is_empty()).then(|| b.finalize())
    }

    /// The Hamming PROXIMITY of two words' meanings (0 = identical, ~[`DIM`]/2 = unrelated):
    /// small when their definitions share vocabulary. Returns [`DIM`] (maximally far) when either
    /// word has no bound meaning, so an unknown word reads as "unrelated" without a special case.
    /// This is the pure graph query a later increment reads prohibition-meaning off of — "does
    /// this prose bind to prohibition-meaning" is `related(word, a-known-prohibition-word)`.
    pub fn related(&self, a: &str, b: &str) -> u32 {
        match (self.meaning_of(a), self.meaning_of(b)) {
            (Some(x), Some(y)) => x.distance(&y),
            _ => DIM as u32,
        }
    }
}

/// HLM1 wire form: the headword seeds and per-head word counts ride the RAW stream; the
/// definition words themselves go on the DATA stream (deflated — common words repeat across
/// entries, so they compress hard). Encoded in canonical sorted/deduped order so the artifact is
/// reproducible and a decoded network is already [`sealed`](MeaningNetwork::seal).
impl crate::lint_codec::Bin for MeaningNetwork {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        let mut entries = self.defs.clone();
        entries.sort_by_key(|(k, _)| *k);
        entries.dedup_by(|a, b| a.0 == b.0);
        e.raw_u64s(&entries.iter().map(|(k, _)| *k).collect::<Vec<_>>());
        e.raw_u32s(&entries.iter().map(|(_, w)| w.len() as u32).collect::<Vec<_>>());
        for (_, words) in &entries {
            for w in words {
                e.str(w);
            }
        }
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<MeaningNetwork> {
        let keys = d.raw_u64s()?;
        let lens = d.raw_u32s()?;
        if keys.len() != lens.len() {
            return None;
        }
        let mut defs = Vec::with_capacity(keys.len());
        for (k, len) in keys.into_iter().zip(lens) {
            let mut words = Vec::with_capacity(len as usize);
            for _ in 0..len {
                words.push(d.str()?);
            }
            defs.push((k, words));
        }
        Some(MeaningNetwork { defs })
    }
}

// ── Learned structural roles (LINTER.md, "Reading a page is UNDERSTANDING") ────

/// The register the brain saw follow each markup token across the web curriculum — a code
/// carrier (`+1`, its contained text read as code) or a section heading (`-1`, short
/// title-shaped text). Keyed by the element name's token seed (its own characters — the same
/// typography the scanner tokenizes with, never a tag name enumerated in product code). This is
/// the association the page reader queries to tell `<pre>goto cleanup</pre>` (code) from
/// `<h1>flowlang statements</h1>` (heading) when their words are equally unbound. Learned once at
/// setup ([`crate::lint_graph::learn_structure_roles`]); the lint path only ever reads it.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct StructureRoles {
    /// Element-name seed → learned role (`+1` code carrier, `-1` heading). Sorted by seed so
    /// [`role_of`](Self::role_of) answers by binary search.
    roles: Vec<(u64, i8)>,
    /// The learned title-shape ceiling: the upper word count of a section heading, from the web
    /// curriculum's own headings — the shape fallback the reader bounds a section with when its
    /// heading element earned no role (many pages' `<h1>` is a one-off page title, so headings
    /// are not always learned by name). 0 when nothing was learned.
    #[serde(default)]
    title_ceiling: u32,
}

impl StructureRoles {
    /// An empty role set — a brain that has learned no page structure yet reads by meaning alone.
    pub fn new() -> StructureRoles {
        StructureRoles { roles: Vec::new(), title_ceiling: 0 }
    }

    /// Build from `(seed, role)` votes (`+1` code carrier, `-1` heading) and the learned title
    /// ceiling, sorted for binary search. The one constructor the setup learner uses; the lint
    /// path never mutates roles.
    pub fn from_learned(mut roles: Vec<(u64, i8)>, title_ceiling: u32) -> StructureRoles {
        roles.sort_by_key(|(k, _)| *k);
        roles.dedup_by_key(|(k, _)| *k);
        StructureRoles { roles, title_ceiling }
    }

    /// The learned role of the element whose name hashes to `seed`: `Some(true)` a code carrier,
    /// `Some(false)` a heading, `None` an element the reading found no decisive role for.
    pub fn role_of(&self, seed: u64) -> Option<bool> {
        self.roles.binary_search_by_key(&seed, |(k, _)| *k).ok().map(|i| self.roles[i].1 > 0)
    }

    /// The learned votes as `(seed, role)` pairs — how the committed structure bootstrap is
    /// generated and how a role set is inspected.
    pub fn votes(&self) -> &[(u64, i8)] {
        &self.roles
    }

    /// The learned title-shape ceiling in words (0 = none learned) — the section-heading shape
    /// fallback the reader uses when a heading element earned no role.
    pub fn title_ceiling(&self) -> u32 {
        self.title_ceiling
    }

    /// How many elements carry a role.
    pub fn len(&self) -> usize {
        self.roles.len()
    }

    /// Whether no page structure was learned at all (no roles and no title shape).
    pub fn is_empty(&self) -> bool {
        self.roles.is_empty() && self.title_ceiling == 0
    }
}

/// HLM1 wire form: the element seeds and their signed roles ride the RAW stream as two parallel
/// arrays in canonical sorted order, then the title ceiling, so the artifact is reproducible.
impl crate::lint_codec::Bin for StructureRoles {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        let mut roles = self.roles.clone();
        roles.sort_by_key(|(k, _)| *k);
        e.raw_u64s(&roles.iter().map(|(k, _)| *k).collect::<Vec<_>>());
        e.raw_u64s(&roles.iter().map(|(_, r)| *r as u64).collect::<Vec<_>>());
        e.fixed_u64(u64::from(self.title_ceiling));
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<StructureRoles> {
        let keys = d.raw_u64s()?;
        let vals = d.raw_u64s()?;
        if keys.len() != vals.len() {
            return None;
        }
        let roles = keys.into_iter().zip(vals.into_iter().map(|r| r as i8)).collect();
        let title_ceiling = d.fixed_u64()? as u32;
        Some(StructureRoles { roles, title_ceiling })
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

// ── Rules from UNDERSTANDING (owner directive: never a surprise threshold) ────

/// Read a language's documentation PROSE and produce rules from UNDERSTANDING. Each sentence is
/// understood through the dictionary's meaning network ([`crate::lint_english`], distilled from
/// the same dictionary this brain read at character level): a sentence whose MEANING is a
/// prohibition — it carries a word the dictionary DEFINES by negation ("never", "avoid",
/// "deprecated") — names a CONSTRUCT, the word the sentence is about that English cannot account
/// for, and that pairing is a rule. No surprise, no threshold: comprehension decides, and a rule
/// is a construct the docs' own words forbid.
pub fn rules_from_understanding(lang: &str, prose: &str) -> Vec<crate::linter::LearnedRule> {
    let Some(eng) = crate::lint_english::brain() else { return Vec::new() };
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for sentence in crate::lint_read::sentences(prose) {
        let words: Vec<String> = sentence
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|w| w.len() >= 2)
            .collect();
        // UNDERSTANDING — does this sentence STATE a prohibition (LINTER.md, "Entry gates")? A
        // negation operator commands it ("Never use X") or a word names disapproval of the
        // construct ("X is incorrect") — discovered from the dictionary's meaning network, never
        // a keyword list. A negation merely buried in a description ("it is not allowed to move
        // fields …") states nothing and is skipped.
        if !eng.sentence_states_prohibition(sentence) {
            continue;
        }
        // The CONSTRUCT — the word the sentence is ABOUT that English cannot account for: a
        // dictionary non-word that is code-shaped (carries a letter). The most distinctive
        // (longest) such word is the named construct ("goto" among ordinary English).
        // MIGRATION (LINTER.md, "retiring word-level `english.knows`"): `!eng.knows` becomes
        // `!lint_graph::word_is_english(char_brain, w)` once the char brain is threaded here and
        // this test carries a meaning-bound brain — LEFT until then so understanding stays intact.
        let Some(construct) = words
            .iter()
            .filter(|w| !eng.knows(w) && w.chars().any(|c| c.is_alphabetic()))
            .max_by_key(|w| w.len())
        else {
            continue;
        };
        let id = construct.to_lowercase();
        if !seen.insert(id.clone()) {
            continue;
        }
        out.push(crate::linter::LearnedRule {
            language: lang.to_string(),
            id,
            severity: "medium".to_string(),
            description: sentence.trim().to_string(),
            bad: construct.clone(),
            good: String::new(),
        });
    }
    out
}

// ── The cumulative global brain (setup trains it; lint loads it) ──────────────

/// Where the machine's character brain lives, beside the models.
fn store_path() -> std::path::PathBuf {
    crate::lint_train::model_dir_pub().join("char.global.bin")
}

/// Load the machine's character brain (`HLM1`), or `None` when it has not been trained yet. The
/// SUCCESSFUL load is memoized for the process; a MISS is not — during setup the same process
/// writes the brain after a consumer first asked for it (site discovery reads pages right after
/// `ensure_brain` saves), so caching an early `None` would blind every later read. Once loaded,
/// every call returns the same cached reader; the lint path only ever loads.
pub fn brain() -> Option<&'static CharReader> {
    use crate::lint_codec::{Bin, Dec};
    static BRAIN: std::sync::OnceLock<CharReader> = std::sync::OnceLock::new();
    if let Some(b) = BRAIN.get() {
        return Some(b);
    }
    let loaded = std::fs::read(store_path())
        .ok()
        .and_then(|b| Dec::open(&b, crate::lint_codec::kind::CHARBRAIN))
        .and_then(|(_, mut d)| CharReader::dec(&mut d))
        .map(|mut r| {
            // A brain trained where the web could not be crawled learned no roles; hydrate them
            // from the committed bootstrap so the reader still works.
            r.ensure_structure();
            r
        })?;
    // Set only on success (ignore a losing race — the winner's reader is equivalent).
    let _ = BRAIN.set(loaded);
    BRAIN.get()
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

/// Corpus dedup block size (characters). A crawled page is split into blocks at newlines (a block
/// longer than this is further chunked, so minified single-line pages still dedup), and a block
/// whose exact text was already learned THIS build is skipped. Repeated page chrome — the nav
/// sidebar and footer duplicated across a 20 000-page crawl — is thus learned ONCE; unique prose,
/// code, and tag structure are all kept verbatim. Raw HTML is preserved (owner guardrail: the
/// structure-role learner needs real markup in context); only the DUPLICATION dies. This is a
/// correctness fix as much as a speed one: re-reading identical chrome 20 000× teaches an order-5
/// predictor nothing after the first exposures and only skews the frequency curve.
const DEDUP_BLOCK: usize = 512;

/// Novel-content ceiling per crawl source (characters) — a runaway safety valve, NOT a working
/// limit. Representative raw HTML of a language (its tags, real prose, real code) fits easily; this
/// exists only so a pathological source of genuinely-unique boilerplate cannot re-inflate the
/// corpus the dedup just shrank. The dictionary base is exempt (it is unique content by nature).
const LANG_CORPUS_CAP: usize = 24 << 20;

/// The NOVEL raw-HTML of `body` under `seen`: its blocks (newline-delimited, long blocks chunked
/// at [`DEDUP_BLOCK`]) that have not been learned this build, concatenated in order and bounded by
/// `budget` (decremented as content is kept). Blank blocks are dropped; duplicated chrome collapses
/// to its first occurrence while unique content and tag structure are retained verbatim. Keeping
/// blocks in place (not stripping tags) preserves the markup-in-context the structure-role learner
/// reads (owner guardrail).
fn novel_blocks(
    body: &str,
    seen: &mut std::collections::HashSet<u64>,
    budget: &mut usize,
) -> String {
    let mut out = String::new();
    for line in body.split_inclusive('\n') {
        let mut rest = line;
        while !rest.is_empty() {
            if *budget == 0 {
                return out;
            }
            // Chunk overlong blocks on a char boundary so a minified single-line page still dedups.
            let end = rest.char_indices().nth(DEDUP_BLOCK).map_or(rest.len(), |(i, _)| i);
            let (block, tail) = rest.split_at(end);
            rest = tail;
            if block.trim().is_empty() {
                continue;
            }
            if seen.insert(crate::lint_ai::token_seed(block)) {
                out.push_str(block);
                *budget = budget.saturating_sub(block.chars().count());
            }
        }
    }
    out
}

/// SETUP verb (curriculum): build this machine's character brain if missing or its inputs
/// changed, cumulatively — the whole dictionary (English base), then every raw page of the web
/// curriculum in order, DEDUPED to representative content ([`novel_blocks`]: repeated crawl chrome
/// is learned once, not 20 000×). Purely reads what setup already cached/crawled; saves
/// `char.global.bin`. Returns a one-line report, or `None` when there is no dictionary and no
/// cached web pages to learn from. Online only through the shared crawl cache (same latch as every
/// setup read).
#[cfg(feature = "crawl")]
pub fn ensure_brain(data_root: &std::path::Path) -> Option<String> {
    // Freshness: the brain is keyed by the dictionary fingerprint folded with each web
    // language's raw-page fingerprint. Unchanged inputs ⇒ the saved brain is current.
    let english = crate::lint_english::dictionary_prose(None);
    let mut web: Vec<(String, Vec<(String, String)>)> = Vec::new();
    // BRAIN_REV seeds the fingerprint so a binding-scheme change alone forces every brain to
    // rebuild, even when the dictionary and web inputs are byte-for-byte unchanged.
    let mut fp = crate::lint_ai::token_seed(english.as_deref().unwrap_or("")) ^ BRAIN_REV.rotate_left(29);
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
    let trace = std::env::var_os("HELPERS_LINT_TRACE").is_some();
    let mut clock = std::time::Instant::now();
    let mut lap = |clock: &mut std::time::Instant, name: &str| {
        if trace {
            eprintln!("[char-brain] {name}: {:.1}ms", clock.elapsed().as_secs_f64() * 1e3);
        }
        *clock = std::time::Instant::now();
    };
    let mut r = CharReader::new();
    let mut order: Vec<String> = Vec::new();
    if let Some(prose) = &english {
        let before = r.total;
        r.learn(prose);
        order.push(format!("english {}c", r.total - before));
    }
    lap(&mut clock, "english-learn");
    // Bind the dictionary's headword→definition meaning network from the SAME dictionary
    // (LINTER.md, "The dictionary meaning network") — the char substrate's comprehension graph.
    if let Some(defs) = crate::lint_english::dictionary_definitions(None, MAX_MEANING_WORDS) {
        for (head, words) in &defs {
            let refs: Vec<&str> = words.iter().map(String::as_str).collect();
            r.meanings.bind(head, &refs);
        }
        r.meanings.seal();
        order.push(format!("meanings {}w", r.meanings.len()));
    }
    lap(&mut clock, "meanings-bind");
    // Read the web curriculum DEDUPED: a global block set collapses the chrome repeated across a
    // crawl to its first occurrence, so the reader sees each language's real structure and content
    // once instead of thousands of near-identical copies. The deduped raw-HTML pages are kept for
    // the structure-role learner (it needs markup in context, and identical chrome adds no
    // discriminating instances anyway).
    let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut web_bodies: Vec<String> = Vec::new();
    for (lang, pages) in &web {
        let before = r.total;
        let mut budget = LANG_CORPUS_CAP;
        for (_, body) in pages {
            let novel = novel_blocks(body, &mut seen, &mut budget);
            if novel.is_empty() {
                continue;
            }
            r.learn(&novel);
            web_bodies.push(novel);
        }
        if r.total > before {
            order.push(format!("{lang} {}c", r.total - before));
        }
    }
    lap(&mut clock, "web-dedup+learn");
    // Learn the page STRUCTURE by exposure (LINTER.md, "Reading a page is UNDERSTANDING"): over
    // the same (deduped) web curriculum, which register followed each markup token — code carriers
    // vs section headings — read with the meaning network just sealed. This is what lets the reader
    // tell a title from an example when their words are equally unbound.
    let bodies: Vec<&str> = web_bodies.iter().map(String::as_str).collect();
    r.set_structure(crate::lint_graph::learn_structure_roles(&r, &bodies));
    lap(&mut clock, "structure-roles");
    // No web to read (offline or localhost-only) ⇒ hydrate roles from the committed bootstrap, so
    // the saved brain reads pages by role even where the curriculum could not crawl the web.
    r.ensure_structure();
    if r.roles_learned() > 0 {
        order.push(format!("roles {}", r.roles_learned()));
    }
    save(&r);
    save_brain_fp(fp);
    lap(&mut clock, "save");
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
        // The meaning network rode the artifact and answers on real data — the WHOLE dictionary is
        // bound (multi-word headwords included), a floor near the measured ~103k, not the old 69k.
        assert!(loaded.meanings().len() > 95_000, "bound the whole dictionary: {} meanings", loaded.meanings().len());
        // Report the meaning network's BYTE COST — the whole brain vs the brain with the network
        // stripped, so the artifact-size delta is visible on real data.
        let mut bare = loaded.clone();
        bare.meanings = MeaningNetwork::new();
        let bare_bytes = { let mut e = crate::lint_codec::Enc::new(); bare.enc(&mut e); e.finish(crate::lint_codec::kind::CHARBRAIN, "t").len() };
        eprintln!(
            "char.global.bin {} bytes; meaning network adds {} bytes over {} meanings ({} without it)",
            bytes.len(), bytes.len().saturating_sub(bare_bytes), loaded.meanings().len(), bare_bytes,
        );
        // Second call replays.
        let second = super::ensure_brain(data).expect("still trainable");
        assert_eq!(second, "character brain: current", "unchanged inputs replay: {second}");
        std::env::remove_var("HELPERS_LINT_OFFLINE");
        std::env::remove_var("HELPERS_LINT_MODELS");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// END-TO-END, understanding-driven (owner directive): documentation PROSE → the dictionary
    /// meaning network understands the prohibition and names the construct → a rule → it FIRES
    /// on real code. No surprise anywhere. Hermetic — the dictionary understanding loads from
    /// the committed english bootstrap.
    #[test]
    fn understanding_extracts_a_rule_that_fires_on_real_code() {
        let prose = "Never use the goto statement anywhere; it is deprecated and will be removed. \
                     Prefer a structured loop instead.";
        let rules = super::rules_from_understanding("flowlang", prose);
        assert!(
            rules.iter().any(|r| r.bad == "goto"),
            "understanding names the forbidden construct: {:?}",
            rules.iter().map(|r| (r.bad.clone(), r.description.clone())).collect::<Vec<_>>()
        );
        // Compile through the REAL matcher and fire on REAL code — the whole path.
        let tuples: Vec<(String, String, String, String, String, String)> = rules
            .iter()
            .map(|r| {
                (
                    r.id.clone(),
                    r.severity.clone(),
                    r.bad.clone(),
                    r.good.clone(),
                    r.description.clone(),
                    "doc".to_string(),
                )
            })
            .collect();
        let rs = crate::lint_match::RuleSet::build(
            "flowlang",
            &tuples,
            &crate::lint_match::Grounding::default(),
        );
        let findings = rs.flag("start:\n    goto cleanup\n    emit(\"done\")\n");
        assert!(
            findings.iter().any(|f| f.rule == "goto" && f.line == 2),
            "the understood rule fires on the goto line: {:?}",
            findings.iter().map(|f| (f.rule.clone(), f.line)).collect::<Vec<_>>()
        );
        // And it does NOT fire on clean code with no goto.
        let clean = rs.flag("start:\n    emit(\"done\")\n");
        assert!(
            clean.is_empty(),
            "no false positive on clean code: {:?}",
            clean.iter().map(|f| f.rule.clone()).collect::<Vec<_>>()
        );
    }

    /// UNIFORM over the WHOLE Unicode scalar set (owner directive 2026-07-08 "whole charset"):
    /// reading privileges no subset. The reader is uniform BY CONSTRUCTION — [`char_hv`] seeds off
    /// the full scalar (a bijection, so high planes never collide with ASCII), context addressing
    /// hashes char CODES not bytes (no multibyte mis-slice), and no `as u8`/ascii-only path exists.
    /// This proves it: Latin, CJK, Arabic (RTL), an astral-plane emoji, a mathematical symbol, and
    /// a decomposed combining diacritic go through every reader path with NO loss and NO panic.
    #[test]
    fn reading_is_uniform_over_the_whole_charset() {
        // (1) `char_hv` is collision-free across planes — the seed is a bijection of the scalar,
        // so two DISTINCT scalars (however high their code point) get distinct vectors. A hash
        // that folded high planes onto ASCII would fail here.
        let scalars = [
            'A',         // Latin
            'é',         // Latin-1 precomposed
            '中',        // CJK unified ideograph
            'ا',         // Arabic (RTL)
            '😀',        // U+1F600 — astral plane (emoji)
            '×',         // mathematical symbol
            '\u{0301}',  // combining acute accent
            '\u{1F4A9}', // another astral scalar
        ];
        for i in 0..scalars.len() {
            for j in (i + 1)..scalars.len() {
                assert!(
                    char_hv(scalars[i]).distance(&char_hv(scalars[j])) > 0,
                    "distinct scalars must not collide: {:?} vs {:?}",
                    scalars[i],
                    scalars[j]
                );
            }
        }

        // (2) `encode` maps mixed-script text without loss and stays order-sensitive across
        // scripts — a spelling centroid that dropped non-Latin characters would tie these.
        let mixed = "中文 عربى 😀 café x×y n\u{0301}";
        assert!(CharReader::new().encode(mixed).is_some(), "mixed-script text encodes");
        let ab = CharReader::new().encode("中a").unwrap();
        let ba = CharReader::new().encode("a中").unwrap();
        assert!(ab.distance(&ba) > 0, "encode is order-sensitive across scripts (中a ≠ a中)");

        // (3) LEARN then PREDICT round-trips every scalar: after one read, each position's exact
        // continuation is retained in its context set — CJK, RTL, emoji, and combining marks
        // included. A stored ASCII-only path would leave the non-Latin positions unpredicted.
        let line = "中文文档 goto 説明 عربية نص 😀🎉 café n\u{0301}o\u{0301} x×÷y ";
        let mut r = CharReader::new();
        r.learn(line);
        let chars: Vec<char> = line.chars().collect();
        assert_eq!(r.total_read(), chars.len() as u64, "every scalar was read, none dropped");
        for i in MIN_ORDER..chars.len() {
            assert!(
                r.predicted(&chars, i),
                "scalar {:?} at {i} must be retained and predicted after reading it",
                chars[i]
            );
        }

        // (4) The gauge paths never panic on the full charset (combining marks, RTL, astral).
        let _ = r.surprise(line);
        let _ = r.segment(&format!("<p>{line}</p><pre>中: goto 😀</pre>"));
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

    /// DEV TOOL — writes `lint-index/char-structure-bootstrap.json` from THIS machine's char
    /// brain (which must have read the whole web curriculum, so its roles are real), so machines
    /// that can only reach localhost still read pages by role. Asserts the learned roles are
    /// sensible — a code carrier and section headings — then commits them. Online prerequisite is
    /// a trained machine brain; run after a full `lint_config action=train`:
    /// `cargo test --release --lib --features crawl generate_char_structure_bootstrap -- --ignored --nocapture`
    #[cfg(feature = "crawl")]
    #[test]
    #[ignore = "dev tool: writes the committed structure-roles bootstrap from the machine brain"]
    fn generate_char_structure_bootstrap() {
        let brain = super::brain().expect("a trained machine char brain (run lint_config train)");
        let structure = brain.structure().clone();
        assert!(
            !structure.is_empty(),
            "the machine brain must have learned page structure from the web curriculum"
        );
        // Sanity: the discrimination depends on a code carrier and a section-heading shape.
        let role = |name: &str| structure.role_of(crate::lint_ai::token_seed(name));
        assert_eq!(role("pre"), Some(true), "`pre` must read as a code carrier");
        assert!(structure.title_ceiling() > 0, "a title-shape ceiling must be learned");
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("lint-index/char-structure-bootstrap.json");
        std::fs::write(&out, serde_json::to_string(&structure).expect("serializes"))
            .expect("bootstrap written");
        eprintln!(
            "wrote {} — {} roles, title ceiling {}",
            out.display(),
            structure.len(),
            structure.title_ceiling()
        );
    }

    // ── The dictionary meaning network ────────────────────────────────────────

    /// A tiny fixture dictionary (hermetic — no machine dictionary, no network): negation
    /// headwords written in SHARED negative vocabulary, neutral headwords in their own. The
    /// cluster the tests below assert is a property of THESE definitions alone — the product code
    /// ([`MeaningNetwork`]) names no negation word anywhere, so nothing is smuggled in by a list.
    fn negation_fixture() -> MeaningNetwork {
        let mut m = MeaningNetwork::new();
        m.bind("never", &["not", "negation", "negative"]);
        m.bind("no", &["negation", "negative", "refusal"]);
        m.bind("not", &["negation", "negative", "deny"]);
        m.bind("avoid", &["negation", "negative", "prevent"]);
        m.bind("banana", &["yellow", "curved", "tropical", "fruit"]);
        m.bind("table", &["furniture", "flat", "surface", "legs"]);
        m.seal();
        m
    }

    /// (a) A word's meaning retrieves, is pure/stable, and survives the HLM1 round trip carried
    /// inside the whole brain — the property a loaded `char.global.bin` stands on.
    #[test]
    fn meaning_retrieves_and_survives_the_hlm1_round_trip() {
        use crate::lint_codec::{Bin, Dec, Enc};
        let mut r = CharReader::new();
        r.meanings = negation_fixture();
        let before = r.meaning_of("never").expect("never has a bound meaning");
        assert_eq!(r.meaning_of("never").unwrap().distance(&before), 0, "meaning is pure/stable");
        assert!(r.meaning_of("frobnicate").is_none(), "an unbound word has no meaning");
        let mut e = Enc::new();
        r.enc(&mut e);
        let bytes = e.finish(crate::lint_codec::kind::CHARBRAIN, "t");
        let (_, mut d) = Dec::open(&bytes, crate::lint_codec::kind::CHARBRAIN).expect("opens");
        let loaded = CharReader::dec(&mut d).expect("decodes");
        assert_eq!(
            loaded.meaning_of("never").expect("survives the round trip").distance(&before),
            0,
            "a loaded network rebinds the identical meaning"
        );
        assert_eq!(loaded.meanings().len(), r.meanings().len(), "every binding survives");
    }

    /// (b) Words whose definitions share vocabulary measure CLOSER than unrelated ones; an
    /// unknown word reads as maximally far without a special case.
    #[test]
    fn shared_definitions_measure_closer_than_unrelated_ones() {
        let m = negation_fixture();
        let shared = m.related("never", "no");
        let unrelated = m.related("never", "banana");
        assert!(shared < unrelated, "shared meaning ({shared}) must beat unrelated ({unrelated})");
        assert_eq!(m.related("never", "zzzznope"), DIM as u32, "an unknown word is maximally far");
    }

    /// (c) Retain-and-grow with sense-folding: binding MORE material adds new headwords, and a
    /// repeated headword FOLDS its further senses onto the primary (additive — the primary sense's
    /// words stay in front, the new sense's words append) rather than being dropped or overwritten.
    #[test]
    fn binding_more_material_folds_senses_and_retains_the_network() {
        let mut m = negation_fixture();
        let primary = m.definition("never").expect("bound").to_vec();
        let n = m.len();
        m.bind("orange", &["round", "citrus", "fruit"]);
        m.bind("never", &["completely", "different", "sense"]); // second sense — folded in
        m.seal();
        assert!(m.len() > n, "new headwords were gained: {} -> {}", n, m.len());
        let folded = m.definition("never").expect("still bound");
        assert!(
            folded.starts_with(&primary),
            "the primary sense stays in front: {folded:?} vs {primary:?}"
        );
        assert!(
            folded.contains(&"completely".to_string()),
            "the later sense is folded in, not dropped: {folded:?}"
        );
        assert!(m.meaning_of("orange").is_some(), "the newly read word is queryable");
    }

    /// (d) The negation cluster EMERGES from the dictionary's own definitions: never/no/not/avoid
    /// measure mutually closer than any of them to the neutral words — with NO negation word
    /// named in product code (only in the test fixture). This is the covenant's payoff — a later
    /// increment reads prohibition-meaning as `related(word, prohibition-word)`, never a list.
    #[test]
    fn negation_words_cluster_without_any_negation_list_in_product_code() {
        let m = negation_fixture();
        let neg = ["never", "no", "not", "avoid"];
        let neutral = ["banana", "table"];
        let mut worst_intra = 0u32;
        for i in 0..neg.len() {
            for j in (i + 1)..neg.len() {
                worst_intra = worst_intra.max(m.related(neg[i], neg[j]));
            }
        }
        let mut best_cross = u32::MAX;
        for a in neg {
            for b in neutral {
                best_cross = best_cross.min(m.related(a, b));
            }
        }
        assert!(
            worst_intra < best_cross,
            "negation words must cluster: worst intra {worst_intra} < best cross {best_cross}"
        );
    }
}
