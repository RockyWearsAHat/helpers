//! The MarkupBrain — HTML understanding, read from the web's own documentation (LINTER.md,
//! "Markup second — the MarkupBrain").
//!
//! **No tag is ever consulted by name.** The only programmed piece here is typography from the
//! spec's allowed list: a `<…>` run is ONE MARKUP TOKEN (HTML's word-boundary rule — the same
//! class of mechanism as "whitespace separates words"), and sentence punctuation is what it
//! always was. Everything else is the reading's own judgment, made with the English brain:
//!
//! - **Code vs prose is a register judgment.** The text between markup tokens reads as English
//!   or it doesn't — the fraction of its whitespace words the dictionary accounts for is its
//!   English density, and a low-density run of words is code, whatever tags shredded it
//!   (syntax highlighters split examples into per-token spans; the register survives).
//! - **The split point is LEARNED from the W3 reading** ([`ensure_built`]): the html
//!   language's own documentation contains both registers by nature, so the density
//!   distribution over its word stream is bimodal and Otsu's valley calibrates the judgment.
//!   No html reading ⇒ no calibration ⇒ language pages refuse to be read (the curriculum:
//!   dictionary → W3 HTML → language docs — a hard dependency chain, never a preference).
//! - **A boundary is a TITLE-SHAPED gap**: short, unpunctuated, majority-English — headings,
//!   captions, nav labels in any markup dialect. Governing prose never crosses one and the
//!   title's own words never weld onto the section's first sentence (ledger #22).
//!
//! The artifact (`markup.global.bin`, `HLM1` beside the models) is built at SETUP time after
//! the English brain and before any language trains; machines without a readable html page
//! cache load the committed bootstrap `lint-index/markup-bootstrap.json` (machine-generated
//! learned data — regenerate with
//! `cargo test --release --lib generate_markup_bootstrap -- --ignored`).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::lint_codec::{Bin, Dec, Enc};
use crate::lint_english::English;

/// The serialized MarkupBrain: the html-fed reader (raw pages read WHOLE — tags learned as
/// vocabulary by exposure; ubiquity strips them of meaning-weight) plus the learned register
/// calibration every raw documentation page is read with.
#[derive(Default, Serialize, Deserialize)]
pub struct Markup {
    /// The reader after reading the html documentation corpus raw — markup vocabulary and its
    /// frequencies, learned by exposure exactly as English was.
    pub reader: crate::lint_read::Reader,
    /// The prose/code English-density split, per-mille: Otsu's valley over the W3 corpus's
    /// windowed word-density distribution. A window below this reads as code register.
    pub split_pm: u32,
    /// The title-shape word ceiling: the learned upper word count of a short, unpunctuated,
    /// majority-English gap (the corpus's own heading/caption length distribution).
    pub title_ceiling: u32,
    /// LEARNED tag roles (LINTER.md, "Markup second"): each element whose CONTAINED text
    /// reads, across the W3 corpus, decisively as one register — the seed of the element
    /// name → role (`+1` a code carrier like `<pre>`, `-1` a section heading like `<h1>`). No
    /// tag is enumerated in code; the roles are whatever the reading found. Sorted by seed.
    /// This is what tells `<pre>goto cleanup</pre>` (code) from `<h1>flowlang statements</h1>`
    /// (heading) when the two are textually identical.
    #[serde(default)]
    pub tag_roles: Vec<(u64, i8)>,
    /// Pages of html documentation read into this brain (the read witness).
    pub pages_read: u64,
    /// Fold of the html crawl caches' file states this was calibrated from — rebuild only
    /// when the reading itself changed.
    pub source_fp: u64,
}

impl Markup {
    /// Whether this brain can read pages: it was calibrated from a real corpus (both
    /// registers present). An uncalibrated brain refuses — language documentation is then
    /// reported as unreadable, never hand-parsed.
    pub fn is_ready(&self) -> bool {
        self.split_pm > 0 && self.title_ceiling > 0
    }

    /// The learned role of the element whose name hashes to `seed`: `Some(true)` a code
    /// carrier, `Some(false)` a section heading, `None` an element the reading found no
    /// decisive role for (its contained text mixes registers — read by density like any gap).
    fn tag_role(&self, seed: u64) -> Option<bool> {
        self.tag_roles
            .binary_search_by_key(&seed, |(k, _)| *k)
            .ok()
            .map(|i| self.tag_roles[i].1 > 0)
    }

    /// The role of a gap given the elements that CONTAIN it, innermost last: the innermost
    /// element with a decisive learned role decides (LINTER.md, "Markup second" — a `<span>`
    /// shred inside `<pre>` is code because `pre` contains it; the span itself learned
    /// nothing). `None` when no containing element testifies — density then judges.
    fn stack_role(&self, stack: &[(u32, u64)]) -> Option<bool> {
        stack.iter().rev().find_map(|(_, s)| self.tag_role(*s))
    }
}

/// HLM1 wire form: the reader's arrays ride the RAW stream; calibration is four integers.
impl Bin for Markup {
    fn enc(&self, e: &mut Enc) {
        self.reader.enc(e);
        e.u(u64::from(self.split_pm));
        e.u(u64::from(self.title_ceiling));
        e.raw_u64s(&self.tag_roles.iter().map(|(k, _)| *k).collect::<Vec<_>>());
        e.raw_u64s(&self.tag_roles.iter().map(|(_, r)| *r as u64).collect::<Vec<_>>());
        e.fixed_u64(self.pages_read);
        e.fixed_u64(self.source_fp);
    }
    fn dec(d: &mut Dec) -> Option<Markup> {
        let reader = Bin::dec(d)?;
        let split_pm = d.u()? as u32;
        let title_ceiling = d.u()? as u32;
        let keys = d.raw_u64s()?;
        let roles = d.raw_u64s()?;
        if keys.len() != roles.len() {
            return None;
        }
        let tag_roles = keys.into_iter().zip(roles.into_iter().map(|r| r as i8)).collect();
        Some(Markup {
            reader,
            split_pm,
            title_ceiling,
            tag_roles,
            pages_read: d.fixed_u64()?,
            source_fp: d.fixed_u64()?,
        })
    }
}

// ── The word stream (typography only: `<…>` is a markup token, whitespace splits words) ──

/// One text gap between markup tokens, with the aggregate shape the segmentation reads. Words
/// are counted into the gap, not kept individually: code SPANS are cut at gap edges, not word
/// edges, so a highlighter's per-token shredding is stitched back whole. The English judgment
/// is asked about the WHOLE word with edge punctuation trimmed — `console.log(x)` and
/// `items=[]` are code typography no dictionary defines (LINTER.md, "Common language first").
struct Gap {
    start: usize,
    end: usize,
    words: u32,
    english_words: u32,
    /// Any sentence punctuation (`.;!?` closing a word) — used only to keep a title short and
    /// unpunctuated.
    punctuated: bool,
    /// A prose sentence terminal (`.!?` + whitespace/end) — the delimiter between a prose
    /// sentence and what follows. Deliberately excludes `;`, which ends code statements.
    terminal: bool,
    /// Any non-alphanumeric, non-whitespace character present — code typography.
    symbolic: bool,
    /// Every open element CONTAINING this gap as `(instance, element-name seed)`, outermost
    /// first — seeds key the MarkupBrain's learned tag roles, instance ids make two distinct
    /// elements of the same name distinguishable (so nesting checks and per-element tallies
    /// are exact). Containment is typography (an opening markup token holds the text until
    /// its own name closes it); empty at the top of the body.
    stack: Vec<(u32, u64)>,
}

/// The scanned page: its gaps and every markup token's byte range (kept for author-mark
/// reading — anchors and block language hints are attributes the author wrote, corroborating
/// provenance, never comprehension).
struct Scan {
    gaps: Vec<Gap>,
    tags: Vec<(usize, usize)>,
}

impl Scan {
    /// Whether the page held any word at all — an empty page yields no units.
    fn is_empty(&self) -> bool {
        self.gaps.is_empty()
    }
}

/// The containment stack never grows past this many open elements — a runaway valve against
/// pathologically nested (or endlessly unclosed) markup, not a working limit.
const STACK_CEILING: usize = 64;

/// Tokenize a body into gaps. Pure typography: `<` opens a markup token that runs to its `>`
/// (`<!--` to its `-->`), whitespace separates words in between, and an opening token CONTAINS
/// the text until its own name closes it — names are compared only to each other, never to a
/// list. A repeated unclosed sibling (`<li><li>`) replaces its predecessor on the stack, the
/// same way a new quote ends an unclosed one.
fn scan(body: &str, english: &English) -> Scan {
    let bytes = body.as_bytes();
    let mut gaps = Vec::new();
    let mut tags = Vec::new();
    let mut at = 0usize;
    let mut stack: Vec<(u32, u64)> = Vec::new(); // open elements containing `at`, outermost first
    let mut next_instance: u32 = 0;
    while at < bytes.len() {
        if bytes[at] == b'<' {
            // A comment is one markup token to its own terminator — its interior `>`s are text.
            let close = if body[at..].starts_with("<!--") {
                body[at..].find("-->").map(|i| at + i + 3).unwrap_or(bytes.len())
            } else {
                body[at..].find('>').map(|i| at + i + 1).unwrap_or(bytes.len())
            };
            let tag = &body[at..close];
            tags.push((at, close));
            let seed = element_seed(tag);
            if seed != 0 {
                if tag.starts_with("</") {
                    // Close the nearest open element of this name; unmatched closes are noise.
                    if let Some(i) = stack.iter().rposition(|(_, s)| *s == seed) {
                        stack.truncate(i);
                    }
                } else if !tag.trim_end_matches('>').ends_with('/') {
                    // A self-closing token (`<br/>`) contains nothing and never opens.
                    if stack.last().is_some_and(|(_, s)| *s == seed) {
                        stack.pop(); // sibling auto-close: `<li><li>` — the new one replaces
                    }
                    if stack.len() < STACK_CEILING {
                        stack.push((next_instance, seed));
                        next_instance += 1;
                    }
                }
            }
            at = close;
            continue;
        }
        let end = body[at..].find('<').map(|i| at + i).unwrap_or(bytes.len());
        let mut gap = Gap {
            start: at,
            end,
            words: 0,
            english_words: 0,
            punctuated: false,
            terminal: false,
            symbolic: false,
            stack: stack.clone(),
        };
        let mut w_start: Option<usize> = None;
        // Walk char-wise so multibyte text never splits; a word closes at whitespace.
        let text = &body[at..end];
        let mut it = text.char_indices().peekable();
        while let Some((i, c)) = it.next() {
            let abs = at + i;
            if c.is_whitespace() {
                if let Some(s) = w_start.take() {
                    push_word(body, s, abs, english, &mut gap);
                }
                continue;
            }
            if !c.is_alphanumeric() {
                gap.symbolic = true;
            }
            if w_start.is_none() {
                w_start = Some(abs);
            }
            // Sentence typography (the reader's own rule): `.;!?` followed by whitespace or
            // the gap's end closes a sentence — a `.` between letters is part of a word. A
            // TERMINAL (`.!?`) marks a prose sentence's end; `;` is kept out of `terminal`
            // because it also ends code statements.
            if matches!(c, '.' | ';' | '!' | '?')
                && it.peek().map(|(_, n)| n.is_whitespace()).unwrap_or(true)
            {
                gap.punctuated = true;
                if matches!(c, '.' | '!' | '?') {
                    gap.terminal = true;
                }
            }
        }
        if let Some(s) = w_start {
            push_word(body, s, end, english, &mut gap);
        }
        if gap.words > 0 {
            gaps.push(gap);
        }
        at = end;
    }
    Scan { gaps, tags }
}

/// Close one word: judge it against common language and count it into its gap.
fn push_word(body: &str, start: usize, end: usize, english: &English, g: &mut Gap) {
    let raw = &body[start..end];
    let trimmed = raw.trim_matches(|c: char| !c.is_alphanumeric());
    // The judgment is about the whole word: interior code typography (`a.b`, `x=1`, `f(x)`)
    // keeps it out of English by construction — `knows` is asked with the trimmed surface,
    // which still carries any interior punctuation.
    let english_word = !trimmed.is_empty()
        && trimmed.chars().all(|c| c.is_alphabetic())
        && english.knows(trimmed);
    g.words += 1;
    g.english_words += u32::from(english_word);
}

/// One gap's English density, per-mille — the register signal the learned split thresholds.
fn gap_density(g: &Gap) -> u32 {
    g.english_words * 1000 / g.words.max(1)
}

/// The seed of a markup token's ELEMENT NAME — the word run right after `<` (an optional
/// closing `/` skipped), lowercased, to its first whitespace or `>`. Pure typography (the same
/// class as word boundaries); the name is only ever a hash key, never compared to a list.
fn element_seed(tag: &str) -> u64 {
    let name: String = tag
        .trim_start_matches('<')
        .trim_start_matches('/')
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .flat_map(|c| c.to_lowercase())
        .collect();
    if name.is_empty() {
        0
    } else {
        crate::lint_ai::token_seed(&name)
    }
}

// ── Calibration (learned at setup from the W3 reading, never tuned) ───────────

/// Otsu's method over a 101-bin density histogram: the split that maximizes between-class
/// variance — the valley between the corpus's prose and code modes. Returns per-mille, or
/// `None` when the corpus is NOT genuinely bimodal (one register can't calibrate a
/// two-register judgment). Two guards make it honest: the two class means must be well
/// SEPARATED (a single hump Otsu would happily bisect has means a few bins apart), and both
/// sides must carry real mass. The split is the MIDDLE of the argmax plateau — over an empty
/// valley every cut scores identically, and the low edge would sit against the code mode.
fn otsu_split(hist: &[u64; 101]) -> Option<u32> {
    let total: u64 = hist.iter().sum();
    if total == 0 {
        return None;
    }
    let sum_all: u64 = hist.iter().enumerate().map(|(i, n)| i as u64 * n).sum();
    let (mut w0, mut sum0) = (0u64, 0u64);
    let mut best_var = -1.0f64;
    let (mut lo_t, mut hi_t) = (0usize, 0usize);
    let (mut best_m0, mut best_m1) = (0.0f64, 0.0f64);
    for t in 0..101usize {
        w0 += hist[t];
        sum0 += t as u64 * hist[t];
        let w1 = total - w0;
        if w0 == 0 || w1 == 0 {
            continue;
        }
        let m0 = sum0 as f64 / w0 as f64;
        let m1 = (sum_all - sum0) as f64 / w1 as f64;
        let var = w0 as f64 * w1 as f64 * (m0 - m1) * (m0 - m1);
        if var > best_var + 1e-6 {
            best_var = var;
            lo_t = t;
            hi_t = t;
            best_m0 = m0;
            best_m1 = m1;
        } else if (var - best_var).abs() <= 1e-6 {
            hi_t = t; // extend the plateau
        }
    }
    // Bimodality gate: prose and code modes must be genuinely apart (≥20 bins = 200‰).
    if best_m1 - best_m0 < 20.0 {
        return None;
    }
    let split = (lo_t + hi_t) / 2;
    let below: u64 = hist[..=split].iter().sum();
    if below * 10 < total || (total - below) * 10 < total {
        return None;
    }
    Some((split * 10) as u32)
}

/// The title-shape ceiling: the 90th percentile of the word counts of short-mode candidates —
/// unpunctuated, majority-English gaps (the corpus's own headings and captions). Clamped by a
/// runaway valve, not a working limit.
fn title_ceiling(counts: &mut Vec<u32>) -> Option<u32> {
    if counts.len() < 50 {
        return None;
    }
    counts.sort_unstable();
    Some(counts[counts.len() * 9 / 10].clamp(2, 24))
}

// ── Reading a page (the unit former — the reading's own segmentation) ─────────

/// One read unit of a page: the code span's byte offset in the scanned body (for author-mark
/// anchors), the prose that governs it, the code exactly as served (markup tokens dropped,
/// entities decoded, lines preserved), and the block's own declared language label ("" when
/// the author declared nothing).
pub struct PageUnit {
    pub at: usize,
    pub prose: String,
    pub code: String,
    pub hint: String,
}

/// How much governing prose immediately above a code span counts, in characters — interim
/// mechanism inherited from the window era (LINTER.md marks it for dissolution by the
/// sequential layer). The cap never eats the first word when nothing was cut (ledger #22).
const GOVERNING_CTX: usize = 320;

/// READ one raw page body (script/style/chrome already dropped by the caller) into its units.
/// This is the whole unit former: register decides what is code, title-shaped gaps bound the
/// governing prose, and no tag name is consulted anywhere. The reading is two-scale — a gap
/// with enough words testifies for itself, a shredded fragment is judged over its window —
/// and a code SPAN is a run of whole code-majority gaps, so it can never bleed mid-sentence
/// into the prose around it.
pub fn read_page(body: &str, english: &English, markup: &Markup) -> Vec<PageUnit> {
    if !markup.is_ready() {
        return Vec::new();
    }
    let scan = scan(body, english);
    if scan.is_empty() {
        return Vec::new();
    }
    // Per-gap roles (LINTER.md, "Markup second"). The LEARNED role of the INNERMOST containing
    // element with one decides first: text contained by a code carrier (`<pre>`, learned from
    // W3 — however many `<code>`/`<span>` layers sit between) is code; text contained by a
    // heading element (`<h1>`) is a boundary — this is what tells `<pre>goto cleanup</pre>`
    // from `<h1>flowlang statements</h1>` when the text is identical. With no learned role,
    // register decides: code reads below the W3 English-density split whether or not it
    // carries symbols (`goto cleanup` is two non-English words, no symbol, still code); a LONE
    // symbol-free word below the split is a domain heading the dictionary does not cover, not
    // code, so a code ANCHOR also needs a symbol or two or more words.
    let role: Vec<Option<bool>> = scan.gaps.iter().map(|g| markup.stack_role(&g.stack)).collect();
    let codey: Vec<bool> = scan
        .gaps
        .iter()
        .zip(&role)
        .map(|(g, r)| match r {
            Some(is_code) => *is_code,
            None => gap_density(g) < markup.split_pm && (g.symbolic || g.words >= 2),
        })
        .collect();
    // A heading-role gap never welds two code runs together, however shred-sized its words.
    let shred: Vec<bool> = scan
        .gaps
        .iter()
        .zip(&role)
        .map(|(g, r)| g.words <= SHRED_WORDS && !g.terminal && *r != Some(false))
        .collect();
    // A boundary is decided in two tiers. A LEARNED heading role IS the boundary — the W3
    // corpus already testified, and its judgment survives shapes the heuristics misread (an
    // MDN heading split by its own inline `<code>` mark). Without a role, title SHAPE decides,
    // guarded by sentence flow (LINTER.md, "Markup second"): a gap that belongs to an OPEN
    // prose sentence is never a shape-boundary, however title-shaped its own fragment — open
    // on either side: the previous prose gap ended without a terminal and its containment
    // flows into this one (`the <code>var</code> statement` — the fragment after the mark),
    // or this gap itself ends unterminated and flows into the next (`Never use the` — the
    // fragment before it). Flow is containment typography: adjacent stacks nest (same
    // instances); a real heading never flows into its section's prose.
    let g = &scan.gaps;
    let open_flow: Vec<bool> = (0..g.len())
        .map(|k| {
            let from_prev = k > 0
                && !codey[k - 1]
                && !g[k - 1].terminal
                && stacks_nest(&g[k - 1].stack, &g[k].stack);
            let into_next = !g[k].terminal
                && k + 1 < g.len()
                && stacks_nest(&g[k].stack, &g[k + 1].stack);
            from_prev || into_next
        })
        .collect();
    let titleish: Vec<bool> = scan
        .gaps
        .iter()
        .enumerate()
        .zip(&role)
        .map(|((k, g), r)| match r {
            Some(is_code) => !is_code, // a heading element IS a boundary; a code one never is
            None => {
                g.words <= markup.title_ceiling && !g.punctuated && !g.symbolic && !open_flow[k]
            }
        })
        .collect();

    // Grow code spans from anchors across shred fragments (LINTER.md, "Markup second"): a
    // maximal run of gaps that are each a code anchor or a shred fragment, containing at
    // least one anchor, IS one code example — the highlighter's shredded tokens (`var`,
    // `total`, `=`) rejoin, while a multi-word prose phrase (7 words of lead-in, a full
    // sentence) is neither anchor nor shred and stops the run. Everything outside a code run
    // is prose; its title-shaped gaps are section boundaries.
    let mut in_code = vec![false; scan.gaps.len()];
    let n = scan.gaps.len();
    let mut i = 0usize;
    while i < n {
        if !(codey[i] || shred[i]) {
            i += 1;
            continue;
        }
        let mut j = i;
        while j + 1 < n && (codey[j + 1] || shred[j + 1]) {
            j += 1;
        }
        if (i..=j).any(|k| codey[k]) {
            for k in i..=j {
                in_code[k] = true;
            }
        }
        i = j + 1;
    }
    let boundary: Vec<bool> = (0..n).map(|k| titleish[k] && !in_code[k]).collect();

    // Emit one unit per maximal code run, governed by the prose above it.
    let mut units = Vec::new();
    let mut i = 0usize;
    while i < n {
        if !in_code[i] {
            i += 1;
            continue;
        }
        let mut j = i;
        while j + 1 < n && in_code[j + 1] {
            j += 1;
        }
        let (start, end) = (scan.gaps[i].start, scan.gaps[j].end);
        let code = crate::doc_crawler::strip_code(&body[start..end]);
        let span_words: u32 = scan.gaps[i..=j].iter().map(|g| g.words).sum();
        let plausible =
            span_words >= 2 || code.chars().any(|c| !c.is_alphanumeric() && !c.is_whitespace());
        if code.trim().len() >= 3 && plausible {
            // Governing prose starts after the LAST boundary before the span — the title
            // bounds the section and keeps its OWN words (ledger #22: never welded onto the
            // first sentence).
            let prose_from = (0..i)
                .rev()
                .find(|&k| boundary[k])
                .map(|k| scan.gaps[k].end)
                .unwrap_or(0);
            let prose = governing_tail(&crate::doc_crawler::strip_tags(&body[prose_from..start]));
            units.push(PageUnit {
                at: start,
                prose,
                code,
                hint: hint_before(body, &scan.tags, start),
            });
        }
        i = j + 1;
    }
    units
}

/// A shred fragment is at most this many words — a highlighter emits its tokens as one-word
/// spans, so 2 covers `!=`-style two-token gaps without letting a real phrase pose as one.
const SHRED_WORDS: u32 = 2;

/// Cap governing prose to its closing [`GOVERNING_CTX`] characters, snapping to a word
/// boundary ONLY when the cap actually cut (ledger #22: an unconditional snap ate the
/// operative "Never" of every prohibition).
fn governing_tail(prose: &str) -> String {
    let prose = prose.trim();
    let tail_start = prose
        .char_indices()
        .rev()
        .scan(0usize, |seen, (idx, _)| {
            *seen += 1;
            Some((idx, *seen))
        })
        .find(|(_, seen)| *seen >= GOVERNING_CTX)
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    if tail_start == 0 {
        return prose.to_string();
    }
    match prose[tail_start..].find(char::is_whitespace) {
        Some(ws) => prose[tail_start + ws..].trim().to_string(),
        None => prose[tail_start..].trim().to_string(),
    }
}

/// The block's own declared language: the nearest author mark (`brush:`/`language-*` class —
/// ledger #18's gate) among the markup tokens immediately before the span. An author mark is
/// corroborating provenance the reader gets for free — it declares what the block IS, it never
/// decides what is code.
fn hint_before(body: &str, tags: &[(usize, usize)], span_start: usize) -> String {
    let before = tags.partition_point(|(open, _)| *open < span_start);
    for (open, close) in tags[..before].iter().rev().take(4) {
        let label = crate::doc_crawler::lang_label_in_tag(&body[*open..*close].to_ascii_lowercase());
        if !label.is_empty() {
            return label;
        }
    }
    String::new()
}

// ── Build, store, load (setup acquires; lint only ever loads) ─────────────────

/// Where the machine's built brain lives, beside the models.
fn store_path() -> PathBuf {
    crate::lint_train::model_dir_pub().join("markup.global.bin")
}

fn load_store() -> Option<Markup> {
    std::fs::read(store_path())
        .ok()
        .and_then(|b| Dec::open(&b, crate::lint_codec::kind::MARKUP))
        .and_then(|(_, mut d)| Markup::dec(&mut d))
}

fn save_store(markup: &Markup) {
    if let Some(dir) = store_path().parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut e = Enc::new();
    markup.enc(&mut e);
    let bytes = e.finish(crate::lint_codec::kind::MARKUP, &format!("{:016x}", markup.source_fp));
    let _ = std::fs::write(store_path(), bytes);
}

/// The loaded MarkupBrain: the machine's built store first, else the committed bootstrap,
/// else `None` — and `None` means raw pages REFUSE to be read (the curriculum gate), never
/// that some hand parser steps in. Memoized per process; the lint path never builds.
pub fn brain() -> Option<&'static Markup> {
    static BRAIN: std::sync::OnceLock<Option<Markup>> = std::sync::OnceLock::new();
    BRAIN
        .get_or_init(|| {
            load_store().filter(Markup::is_ready).or_else(|| {
                crate::lint_train::embedded_lint_index_file("markup-bootstrap.json")
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .filter(Markup::is_ready)
            })
        })
        .as_ref()
}

/// SETUP verb (curriculum stage two): make sure this machine's MarkupBrain exists and matches
/// the html reading — build and save when missing or the html page caches changed, replay
/// otherwise. Requires the English brain (stage one) and html's raw page caches (crawled here
/// when the setup latch allows). Returns a one-line report, or `None` when nothing readable
/// exists yet (the committed bootstrap then serves).
#[cfg(feature = "crawl")]
pub fn ensure_built(data_root: &std::path::Path) -> Option<String> {
    let english = crate::lint_english::brain()?;
    let (pages, fp) = crate::lint_docs::html_raw_pages(data_root);
    if pages.is_empty() {
        return None;
    }
    // The fingerprint folds the TRAINING VERSION so a build-logic change rebuilds the brain
    // even when the html reading itself is unchanged — a brain is its corpus AND its reader.
    let fp = fp ^ crate::lint_ai::token_seed(crate::lint_train::TRAIN_VERSION);
    if load_store().is_some_and(|m| m.source_fp == fp && m.is_ready()) {
        return Some("markup: current".to_string());
    }
    let markup = build(&pages, english, fp)?;
    let report = format!(
        "markup: read {} html pages whole — register split {}‰, title ceiling {} words",
        markup.pages_read, markup.split_pm, markup.title_ceiling,
    );
    save_store(&markup);
    Some(report)
}

#[cfg(not(feature = "crawl"))]
pub fn ensure_built(_data_root: &std::path::Path) -> Option<String> {
    None
}

/// Build the brain from raw html pages: read every page WHOLE into the reader (tags as
/// vocabulary), fold every page's word stream into the density histogram and the title-shape
/// distribution, then calibrate. Pure over its inputs — offline, deterministic, testable.
fn build(pages: &[(String, String)], english: &English, source_fp: u64) -> Option<Markup> {
    let mut markup = Markup { source_fp, ..Default::default() };
    let mut hist = [0u64; 101];
    let mut title_counts: Vec<u32> = Vec::new();
    // Tag-role tallies: per element-name seed, how many following gaps read code-register vs
    // heading-shaped. The split is needed to judge register, so pass 1 calibrates it, pass 2
    // tallies roles. Both are one cheap scan of the corpus.
    let mut scans: Vec<Vec<Gap>> = Vec::with_capacity(pages.len());
    for (_, html) in pages {
        markup.reader.learn_span(html);
        markup.pages_read += 1;
        let body = crate::doc_crawler::drop_script_style(html);
        let gaps = scan(&body, english).gaps;
        for (k, g) in gaps.iter().enumerate() {
            // Weight each gap's density by its words, so the histogram reflects word-level
            // mass — the split then separates the corpus's prose gaps from its code gaps.
            let bin = (gap_density(g) / 10).min(100) as usize;
            hist[bin] += u64::from(g.words);
            // A title-shape candidate must be majority-English and must not CONTINUE an open
            // sentence — the same two guards the role tally uses, keeping mid-sentence link
            // text out of the heading-length distribution.
            if !g.punctuated && g.english_words * 2 >= g.words && !gap_open_flow(&gaps, k) {
                title_counts.push(g.words);
            }
        }
        scans.push(gaps);
    }
    markup.split_pm = otsu_split(&hist)?;
    markup.title_ceiling = title_ceiling(&mut title_counts)?;
    // Pass 2 — learn tag roles (LINTER.md, "Markup second"): every element INSTANCE is judged
    // over the WHOLE text it contains — `<pre>` receives the code its highlighter spans wrap,
    // `<h1>` receives its full title even when an inline `<code>` mark splits it into
    // fragments (judged per fragment, a split heading reads as an open sentence and never
    // testifies — measured: `h1`–`h3` learned nothing until the tally aggregated instances).
    // Code evidence is the register judgment itself; heading evidence needs title shape,
    // majority-English words, and a first fragment that does not continue an open sentence —
    // without those guards, highlighter shreds (`<span>var</span>`) and mid-sentence links
    // tally as headings (measured: the first bootstrap learned `a`/`em`/`span`/`kbd` as
    // boundaries and never found `pre`). Heading shape deliberately tolerates symbols — real
    // headings carry section numbers and API punctuation ("4.5.1 The code element"). Only
    // elements with real support and a ¾ majority earn a role; everything else stays
    // density-judged. No element name is enumerated — the seeds are whatever the corpus used.
    let mut tally: std::collections::HashMap<u64, (u32, u32, u32)> = Default::default();
    for gaps in &scans {
        // Aggregate each element instance's contained text: (seed, words, english, punctuated,
        // symbolic, first gap index, depth of the instance in that gap's stack), keyed by
        // instance id.
        let mut inst: std::collections::HashMap<u32, (u64, u32, u32, bool, bool, usize, usize)> =
            Default::default();
        for (k, g) in gaps.iter().enumerate() {
            for (depth, (id, seed)) in g.stack.iter().enumerate() {
                let e = inst.entry(*id).or_insert((*seed, 0, 0, false, false, k, depth));
                e.1 += g.words;
                e.2 += g.english_words;
                e.3 |= g.punctuated;
                e.4 |= g.symbolic;
            }
        }
        for (_, (seed, words, english, punctuated, symbolic, first, depth)) in inst {
            let density = english * 1000 / words.max(1);
            let code = density < markup.split_pm && (symbolic || words >= 2);
            let heading = words <= markup.title_ceiling
                && !punctuated
                && english * 2 >= words
                && !instance_mid_sentence(gaps, first, depth);
            let e = tally.entry(seed).or_insert((0, 0, 0));
            e.0 += 1;
            e.1 += u32::from(code);
            e.2 += u32::from(heading);
        }
    }
    for (seed, (n, code, heading)) in tally {
        if n < TAG_ROLE_SUPPORT {
            continue;
        }
        // A clear ¾ majority earns the role; a mixed element stays density-judged.
        if code * 4 >= n * 3 {
            markup.tag_roles.push((seed, 1));
        } else if heading * 4 >= n * 3 {
            markup.tag_roles.push((seed, -1));
        }
    }
    markup.tag_roles.sort_by_key(|(k, _)| *k);
    Some(markup)
}

/// Whether two adjacent gaps' containment stacks NEST — one is a prefix of the other (same
/// element INSTANCES, not merely same names), so a sentence can flow between them (`[p]` into
/// `[p, code]` and back out). A heading and its section's prose never nest (`[h2]`, `[p]`),
/// so flow stops exactly where sections do.
fn stacks_nest(a: &[(u32, u64)], b: &[(u32, u64)]) -> bool {
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    long.starts_with(short)
}

/// Whether gap `k` belongs to an OPEN prose sentence (the gap-level flow test pass 1 uses
/// for title-shape candidates): the previous gap reads as majority-English prose, ended
/// without a terminal, and flows in — or this gap ends unterminated and flows into the next.
fn gap_open_flow(gaps: &[Gap], k: usize) -> bool {
    let g = &gaps[k];
    let from_prev = k > 0 && {
        let p = &gaps[k - 1];
        !p.terminal && p.english_words * 2 >= p.words && stacks_nest(&p.stack, &g.stack)
    };
    let into_next =
        !g.terminal && k + 1 < gaps.len() && stacks_nest(&g.stack, &gaps[k + 1].stack);
    from_prev || into_next
}

/// Whether the element instance whose first contained gap is `gaps[first]` (sitting at
/// `depth` in that gap's stack) is MID-SENTENCE: the gap just before it is unterminated
/// majority-English prose whose containment nests with the instance's PARENT stack — a
/// sentence open OUTSIDE flows in across the element boundary (`see the <a>var statement</a>`).
/// Flow INSIDE the instance (a heading's own inline `<code>` mark splitting its title) is the
/// element's own text and never disqualifies it — the tally judges the contained text WHOLE.
fn instance_mid_sentence(gaps: &[Gap], first: usize, depth: usize) -> bool {
    first > 0 && {
        let p = &gaps[first - 1];
        let parent = &gaps[first].stack[..depth];
        !p.terminal && p.english_words * 2 >= p.words && stacks_nest(&p.stack, parent)
    }
}

/// A tag needs at least this many occurrences across the corpus before its role is trusted —
/// a rarely-seen tag cannot testify.
const TAG_ROLE_SUPPORT: u32 = 8;

#[cfg(test)]
mod tests {
    use super::*;

    fn test_english() -> English {
        serde_json::from_str(
            &crate::lint_train::embedded_lint_index_file("english-bootstrap.json")
                .expect("bootstrap committed"),
        )
        .expect("bootstrap parses")
    }

    /// A calibrated brain for offline tests — the calibration values are in the band the W3
    /// reading lands in; unit tests exercise the READING mechanism, not the calibration.
    fn test_markup() -> Markup {
        Markup { split_pm: 600, title_ceiling: 8, pages_read: 1, ..Default::default() }
    }

    #[test]
    fn otsu_finds_the_valley_of_a_bimodal_corpus() {
        let mut hist = [0u64; 101];
        for b in 10..30usize {
            hist[b] = 500; // the code mode
        }
        for b in 75..95usize {
            hist[b] = 800; // the prose mode
        }
        let split = otsu_split(&hist).expect("bimodal calibrates");
        assert!((300..=750).contains(&split), "split lands between the modes: {split}");
    }

    #[test]
    fn a_single_register_corpus_refuses_to_calibrate() {
        let mut hist = [0u64; 101];
        for b in 80..95usize {
            hist[b] = 1000;
        }
        assert_eq!(otsu_split(&hist), None, "one register cannot calibrate a two-register judgment");
    }

    #[test]
    fn a_shredded_code_block_reads_as_one_span_and_prose_governs_it() {
        let english = test_english();
        let markup = test_markup();
        // A syntax highlighter shreds the example into per-token spans; the register run
        // reads through the shredding, and no tag name is consulted.
        let body = r#"<h2>Variables</h2>
            <p>Never use the var statement to declare a variable; it hoists silently.</p>
            <div><span>var</span> <span>total</span> <span>=</span> <span>compute(items);</span>
            <span>var</span> <span>flag</span> <span>=</span> <span>!!getOpts(cfg);</span></div>
            <p>Use let or const instead, which scope to the block.</p>"#;
        let units = read_page(body, &english, &markup);
        assert_eq!(units.len(), 1, "one merged code span: {:?}", units.iter().map(|u| &u.code).collect::<Vec<_>>());
        assert!(units[0].code.contains("var total = compute(items);"), "code survives verbatim: {:?}", units[0].code);
        assert!(units[0].prose.contains("Never use the var statement"), "the governing prose is the section body: {:?}", units[0].prose);
        assert!(!units[0].prose.contains("Variables"), "the title's own words never weld into the prose (ledger #22)");
    }

    #[test]
    fn the_authors_language_mark_is_read_as_a_hint() {
        let english = test_english();
        let markup = test_markup();
        let body = r#"<p>The following example shows the failing call, run it and observe the error.</p>
            <pre class="language-python">raise ValueError(f"bad {x!r}") if x else compute_totals(rows[0:12], key=lambda r: r.total)</pre>"#;
        let units = read_page(body, &english, &markup);
        assert_eq!(units.len(), 1, "units: {:?}", units.iter().map(|u| &u.code).collect::<Vec<_>>());
        assert_eq!(units[0].hint, "python", "the author's own mark declares the block's language");
    }

    /// A brain that learned roles (as the W3 reading does): `pre` carries code, `h2` bounds
    /// sections. Seeds are hashed here in the fixture — the product code never names a tag.
    fn test_markup_with_roles() -> Markup {
        let mut m = test_markup();
        m.tag_roles = vec![
            (crate::lint_ai::token_seed("pre"), 1),
            (crate::lint_ai::token_seed("h2"), -1),
        ];
        m.tag_roles.sort_by_key(|(k, _)| *k);
        m
    }

    #[test]
    fn a_learned_code_role_reads_english_words_inside_pre_as_code() {
        let english = test_english();
        let markup = test_markup_with_roles();
        // "goto cleanup" vs "code here": both read English-ish, but `pre` CONTAINS them (even
        // through a highlighter span), so the learned role decides — and a same-register
        // heading stays a boundary, so the intro prose above it never governs the block.
        let body = r#"<p>Sections are introduced here with a full sentence of prose.</p>
            <h2>error handling</h2>
            <p>Always release resources before returning from the function.</p>
            <pre><code><span>goto</span> <span>cleanup;</span></code></pre>"#;
        let units = read_page(body, &english, &markup);
        assert_eq!(units.len(), 1, "the pre block is one unit: {:?}", units.iter().map(|u| &u.code).collect::<Vec<_>>());
        assert!(units[0].code.contains("goto cleanup;"), "code: {:?}", units[0].code);
        assert!(units[0].prose.contains("release resources"), "prose: {:?}", units[0].prose);
        assert!(!units[0].prose.contains("introduced here"), "the h2 role bounds the section: {:?}", units[0].prose);
    }

    #[test]
    fn a_mid_sentence_inline_mark_never_cuts_the_governing_prose() {
        let english = test_english();
        let markup = test_markup_with_roles();
        // `var` sits mid-sentence inside an inline tag with no learned role: title-shaped on
        // its own, but the sentence around it is still open — it must not become a boundary.
        let body = r#"<p>Never use the <code>var</code> statement to declare anything.</p>
            <pre>var total = compute(items);</pre>"#;
        let units = read_page(body, &english, &markup);
        assert_eq!(units.len(), 1, "units: {:?}", units.iter().map(|u| &u.code).collect::<Vec<_>>());
        assert!(
            units[0].prose.contains("Never use the") && units[0].prose.contains("statement"),
            "the whole sentence governs, uncut by the inline mark: {:?}",
            units[0].prose
        );
    }

    /// DEV TOOL — regenerates `lint-index/markup-bootstrap.json` by crawling the registered
    /// html documentation RAW (bounded — a representative sample is all the register split and
    /// title-shape distribution need) and calibrating, so machines without a local html read
    /// still get an HTML-literate substrate. Online. Run after tokenizer/scanner changes:
    /// `cargo test --release --lib generate_markup_bootstrap -- --ignored`
    #[cfg(feature = "crawl")]
    #[test]
    #[ignore = "dev tool: regenerates the committed markup bootstrap by crawling html docs"]
    fn generate_markup_bootstrap() {
        let data_root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
        // The html sources exactly as registered (WHATWG standard + MDN html tree).
        let mut sources = crate::lint_train::registered_docs_sources(&data_root, "html");
        assert!(!sources.is_empty(), "html has no registered docs source in sources.json");
        // Bounded per source — calibration is a distribution, not a corpus; a few hundred
        // pages across both registers is plenty and keeps regeneration to seconds.
        const BOOTSTRAP_PAGES: usize = 400;
        let mut pages: Vec<(String, String)> = Vec::new();
        for src in sources.drain(..) {
            for p in crate::doc_crawler::crawl(&[&src.url], BOOTSTRAP_PAGES, 0) {
                pages.push((p.url, p.html));
            }
        }
        assert!(!pages.is_empty(), "html crawl returned nothing — check connectivity");
        let english = test_english();
        // Portable substrate, not a machine snapshot: fingerprint zero so any machine that
        // can read html itself rebuilds over it; the frequency tail is dropped as pure size.
        let mut markup = build(&pages, &english, 0).expect("html corpus calibrates");
        markup.reader.retain_read_at_least(2);
        let out = data_root.join("lint-index/markup-bootstrap.json");
        std::fs::write(&out, serde_json::to_string(&markup).expect("serializes"))
            .expect("bootstrap written");
        println!(
            "wrote {} — {} pages, split {}‰, title ceiling {}",
            out.display(),
            markup.pages_read,
            markup.split_pm,
            markup.title_ceiling,
        );
    }
}

#[cfg(all(test, feature = "crawl"))]
mod debug_tally {
    use super::*;

    /// TEMP diagnostic — prints per-element tally numbers over the bootstrap corpus.
    #[test]
    #[ignore = "dev diagnostic"]
    fn print_heading_tallies() {
        let data_root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
        let mut sources = crate::lint_train::registered_docs_sources(&data_root, "html");
        let mut pages: Vec<(String, String)> = Vec::new();
        for src in sources.drain(..) {
            for p in crate::doc_crawler::crawl(&[&src.url], 400, 0) {
                pages.push((p.url, p.html));
            }
        }
        let english: English = serde_json::from_str(
            &crate::lint_train::embedded_lint_index_file("english-bootstrap.json").unwrap(),
        )
        .unwrap();
        let split = 530u32;
        let ceiling = 6u32;
        let mut tally: std::collections::HashMap<u64, (u32, u32, u32)> = Default::default();
        for (_, html) in &pages {
            let body = crate::doc_crawler::drop_script_style(html);
            let gaps = scan(&body, &english).gaps;
            let mut inst: std::collections::HashMap<u32, (u64, u32, u32, bool, bool, usize, usize)> =
                Default::default();
            for (k, g) in gaps.iter().enumerate() {
                for (depth, (id, seed)) in g.stack.iter().enumerate() {
                    let e = inst.entry(*id).or_insert((*seed, 0, 0, false, false, k, depth));
                    e.1 += g.words;
                    e.2 += g.english_words;
                    e.3 |= g.punctuated;
                    e.4 |= g.symbolic;
                }
            }
            for (_, (seed, words, english_w, punctuated, symbolic, first, depth)) in inst {
                let density = english_w * 1000 / words.max(1);
                let code = density < split && (symbolic || words >= 2);
                let heading = words <= ceiling
                    && !punctuated
                    && english_w * 2 >= words
                    && !instance_mid_sentence(&gaps, first, depth);
                let e = tally.entry(seed).or_insert((0, 0, 0));
                e.0 += 1;
                e.1 += u32::from(code);
                e.2 += u32::from(heading);
            }
        }
        for name in ["h1", "h2", "h3", "h4", "h5", "h6", "pre", "code", "dt", "a", "p"] {
            let seed = crate::lint_ai::token_seed(name);
            let (n, c, h) = tally.get(&seed).copied().unwrap_or((0, 0, 0));
            println!("{name}: n={n} code={c} heading={h}");
        }
    }
}
