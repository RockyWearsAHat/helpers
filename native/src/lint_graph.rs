//! Reading a documentation page as UNDERSTANDING (LINTER.md, "Reading a page is UNDERSTANDING") —
//! the char-substrate unit former, the sole reader on the read path (it superseded and replaced
//! the deleted word-substrate MarkupBrain).
//!
//! **No tag is ever consulted by name.** The only programmed piece is typography from the spec's
//! allowed list: a `<…>` run is ONE MARKUP TOKEN (HTML's word-boundary rule, the same class as
//! "whitespace separates words"), and sentence punctuation is what it always was. Everything else
//! is the char brain's own judgment ([`crate::lint_char::CharReader`]):
//!
//! - **Prose vs construct is a MEANING query.** A word is English when the dictionary meaning
//!   network binds it (`CharReader::meaning_of`); a text run is PROSE when the MAJORITY of its
//!   words resolve, CONSTRUCT material when they do not — a comparative majority, never an
//!   absolute density threshold.
//! - **A section title and a code example can be equally unbound** (`<h1>flowlang statements</h1>`
//!   vs `<pre>goto cleanup</pre>`), so the brain also learned, BY EXPOSURE over the web curriculum
//!   ([`learn_structure_roles`]), the register that follows each markup token: code carriers and
//!   section headings, keyed by the element's own characters. That learned role decides FIRST — a
//!   code carrier's contents are code however English their words (`blink fast`), a heading opens a
//!   section and its own words never weld into the governing prose (ledger #22). Where no element
//!   testifies, the meaning majority decides.
//!
//! A code example is the maximal run of code-carrier and whole-construct gaps stitched back
//! through a highlighter's shredding; the prose that governs it is the sentences since the last
//! heading; the author's `language-*`/`brush:` mark rides along as the block's hint. No trained
//! brain ⇒ no units (the curriculum gate), never a hand parse.

use crate::lint_char::{CharReader, StructureRoles};

/// One read unit of a page: the code span's byte offset in the scanned body (for author-mark
/// anchors), the prose that governs it, the code exactly as served (markup tokens dropped,
/// entities decoded, lines preserved), and the block's own declared language label ("" when the
/// author declared nothing). The downstream contract both page consumers read.
#[derive(Debug)]
pub struct PageUnit {
    pub at: usize,
    pub prose: String,
    pub code: String,
    pub hint: String,
}

// ── The word stream (typography only: `<…>` is a markup token, whitespace splits words) ──

/// One text gap between markup tokens, with the aggregate shape the reading judges. Words are
/// counted into the gap, not kept individually: code SPANS are cut at gap edges, not word edges,
/// so a highlighter's per-token shredding is stitched back whole. English-ness is the meaning
/// network's judgment of each whole word with edge punctuation trimmed.
struct Gap {
    start: usize,
    end: usize,
    words: u32,
    english_words: u32,
    /// Any sentence punctuation (`.;!?` closing a word) — keeps a title short and unpunctuated.
    punctuated: bool,
    /// A prose sentence terminal (`.!?` + whitespace/end); excludes `;`, which ends code.
    terminal: bool,
    /// Any non-alphanumeric, non-whitespace character — code typography.
    symbolic: bool,
    /// Every open element CONTAINING this gap as `(instance, element-name seed)`, outermost
    /// first — seeds key the learned roles, instance ids distinguish two elements of one name.
    stack: Vec<(u32, u64)>,
}

/// The scanned page: its gaps and every markup token's byte range (kept for author-mark reading).
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
/// pathologically nested or endlessly unclosed markup, not a working limit.
const STACK_CEILING: usize = 64;

/// Tokenize a body into gaps. Pure typography: `<` opens a markup token that runs to its `>`
/// (`<!--` to its `-->`), whitespace separates words in between, and an opening token CONTAINS the
/// text until its own name closes it — names are compared only to each other, never to a list. A
/// repeated unclosed sibling (`<li><li>`) replaces its predecessor, as a new quote ends an
/// unclosed one. English-ness of each word is the meaning network's judgment.
fn scan(body: &str, reader: &CharReader) -> Scan {
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
                    push_word(body, s, abs, reader, &mut gap);
                }
                continue;
            }
            if !c.is_alphanumeric() {
                gap.symbolic = true;
            }
            if w_start.is_none() {
                w_start = Some(abs);
            }
            // Sentence typography: `.;!?` followed by whitespace or the gap's end closes a
            // sentence — a `.` between letters is part of a word. A TERMINAL (`.!?`) marks a prose
            // sentence's end; `;` is kept out because it also ends code statements.
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
            push_word(body, s, end, reader, &mut gap);
        }
        if gap.words > 0 {
            gaps.push(gap);
        }
        at = end;
    }
    Scan { gaps, tags }
}

/// Close one word: judge it against the meaning network and count it into its gap. The judgment
/// is about the WHOLE word — interior code typography (`a.b`, `x=1`, `f(x)`) keeps it out of
/// English by construction, since only an all-alphabetic surface can bind a dictionary meaning.
fn push_word(body: &str, start: usize, end: usize, reader: &CharReader, g: &mut Gap) {
    let raw = &body[start..end];
    let trimmed = raw.trim_matches(|c: char| !c.is_alphanumeric());
    let english_word = !trimmed.is_empty()
        && trimmed.chars().all(|c| c.is_alphabetic())
        && word_is_english(reader, trimmed);
    g.words += 1;
    g.english_words += u32::from(english_word);
}

/// Whether the dictionary meaning network accounts for `word` — directly, or through a common
/// English inflection reduced to its lemma. The meaning network binds single-word HEADWORDS, so a
/// plural or tensed surface (`statements`, `combines`, `getting`) resolves through its lemma
/// (`statement`, `combine`, `get`) rather than reading as a construct. This is English morphology
/// (the same class of programmed language typography as word boundaries), not an enumerated
/// vocabulary: the words themselves still come from the dictionary the brain read.
pub(crate) fn word_is_english(reader: &CharReader, word: &str) -> bool {
    english_inflection(|w| reader.has_meaning(w), word, READ_SUFFIXES)
}

/// The regular-inflection suffixes the READ path reduces (`statements`, `combined`, `quickly`) —
/// the set [`word_is_english`] has always used. Comparatives/superlatives are deliberately EXCLUDED
/// here so the page-reading judgment (and thus segmentation and `TRAIN_VERSION`) is bit-identical;
/// the trace bridge's construct filter uses a wider set ([`CONSTRUCT_SUFFIXES`]) where more English
/// coverage only ever SUPPRESSES a junk `uses_construct`, never changes what a page reads as.
pub(crate) const READ_SUFFIXES: &[&str] = &["s", "es", "ed", "ing", "d", "ly"];

/// The suffixes the trace bridge's construct filter reduces — the read set plus comparative/
/// superlative `-er/-est` (`harder`, `largest`), so a common English word in ANY inflection is
/// recognised as English and never mistaken for a dictionary-unknown code construct. Wider than
/// [`READ_SUFFIXES`] on purpose and safe to widen: it only makes the construct filter ABSTAIN more.
pub(crate) const CONSTRUCT_SUFFIXES: &[&str] = &["s", "es", "ed", "ing", "d", "ly", "er", "est"];

/// Whether `word` is accounted for by English — directly, or by reducing a regular inflection to a
/// known lemma via `has` (the caller's existence query: the reader's meaning network, or the trace
/// bridge's dictionary). `suffixes` is the inflection set to try (see [`READ_SUFFIXES`],
/// [`CONSTRUCT_SUFFIXES`]). Pure suffix morphology — never a word list, just typography: strip the
/// suffix, re-query the lemma; `-ies → -y`; undo a doubled final consonant before a vowel suffix
/// (`running → run`, `hotter → hot`). Shared by [`word_is_english`] and the construct filter so both
/// judge English inflection identically.
pub(crate) fn english_inflection(has: impl Fn(&str) -> bool, word: &str, suffixes: &[&str]) -> bool {
    let w = word.to_lowercase();
    if has(&w) {
        return true;
    }
    let lemma_resolves = |stem: &str| stem.chars().count() >= 3 && has(stem);
    if let Some(base) = w.strip_suffix("ies") {
        if lemma_resolves(&format!("{base}y")) {
            return true;
        }
    }
    for suffix in suffixes {
        if let Some(stem) = w.strip_suffix(suffix) {
            if lemma_resolves(stem) {
                return true;
            }
            // A doubled final consonant before a vowel suffix drops one (`running`, `hotter`).
            if matches!(*suffix, "ing" | "ed" | "er" | "est") {
                let chars: Vec<char> = stem.chars().collect();
                if chars.len() >= 2 && chars[chars.len() - 1] == chars[chars.len() - 2] {
                    let deduped: String = chars[..chars.len() - 1].iter().collect();
                    if lemma_resolves(&deduped) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Whether the MAJORITY of a gap's words resolve to a dictionary meaning — the prose judgment.
fn prose_majority(g: &Gap) -> bool {
    g.english_words * 2 >= g.words
}

/// The seed of a markup token's ELEMENT NAME — the word run right after `<` (an optional closing
/// `/` skipped), lowercased, to its first whitespace or `>`. Pure typography (the same class as
/// word boundaries); the name is only ever a hash key, never compared to a list.
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

/// The learned role of a gap given the elements that CONTAIN it, innermost last: the innermost
/// element with a decisive role decides (a `<span>` shred inside `<pre>` is code because `pre`
/// contains it; the span itself learned nothing). `None` when no containing element testifies.
fn stack_role(reader: &CharReader, stack: &[(u32, u64)]) -> Option<bool> {
    stack.iter().rev().find_map(|(_, s)| reader.structure_role(*s))
}

// ── Reading a page (the unit former) ──────────────────────────────────────────

/// A shred fragment is at most this many words — a highlighter emits its tokens as one-word
/// spans, so 2 covers `!=`-style two-token gaps without letting a real phrase pose as one.
const SHRED_WORDS: u32 = 2;

/// How much governing prose immediately above a code span counts, in characters — an interim
/// window inherited from the word substrate (LINTER.md marks it for dissolution). The cap never
/// eats the first word when nothing was cut (ledger #22).
const GOVERNING_CTX: usize = 320;

/// READ one raw page body (script/style/chrome already dropped by the caller) into its units. The
/// learned role of the innermost containing element decides register first — code inside a code
/// carrier, a boundary at a heading — and where no element testifies the meaning majority judges.
/// A code SPAN is a run of code-carrier and whole-construct gaps stitched through a highlighter's
/// shredding, governed by the prose since the last heading. No trained brain ⇒ no units.
pub fn read_page(body: &str, reader: &CharReader) -> Vec<PageUnit> {
    // The curriculum gate: a brain that learned no page structure cannot read a page's register.
    if reader.structure().is_empty() {
        return Vec::new();
    }
    let scan = scan(body, reader);
    if scan.is_empty() {
        return Vec::new();
    }
    let n = scan.gaps.len();
    let role: Vec<Option<bool>> =
        scan.gaps.iter().map(|g| stack_role(reader, &g.stack)).collect();
    // A gap is CODE when a code-carrier role says so, else when it is meaning-minority AND carries
    // a symbol or two-plus words (a lone symbol-free unbound word is a domain heading, not code).
    let codey: Vec<bool> = scan
        .gaps
        .iter()
        .zip(&role)
        .map(|(g, r)| match r {
            Some(is_code) => *is_code,
            None => !prose_majority(g) && (g.symbolic || g.words >= 2),
        })
        .collect();
    // A shred fragment welds into an adjacent code run — a highlighter's one/two-word spans; a
    // heading never welds, however shred-sized its fragment.
    let shred: Vec<bool> = scan
        .gaps
        .iter()
        .zip(&role)
        .map(|(g, r)| g.words <= SHRED_WORDS && !g.terminal && *r != Some(false))
        .collect();
    // A section boundary is decided in two tiers (LINTER.md, "Reading a page is UNDERSTANDING"): a
    // LEARNED heading role IS the boundary — the web corpus already testified. Without a role,
    // title SHAPE decides (short, unpunctuated, symbol-free, at most the learned ceiling of
    // words), guarded by sentence flow so a gap belonging to an OPEN prose sentence is never a
    // shape-boundary (a mid-sentence inline `<code>var</code>` mark), and never inside code.
    let ceiling = reader.title_ceiling();
    let g = &scan.gaps;
    let open_flow: Vec<bool> = (0..n)
        .map(|k| {
            let from_prev = k > 0
                && !codey[k - 1]
                && !g[k - 1].terminal
                && stacks_nest(&g[k - 1].stack, &g[k].stack);
            let into_next =
                !g[k].terminal && k + 1 < n && stacks_nest(&g[k].stack, &g[k + 1].stack);
            from_prev || into_next
        })
        .collect();
    let titleish: Vec<bool> = scan
        .gaps
        .iter()
        .enumerate()
        .zip(&role)
        .map(|((k, gap), r)| match r {
            Some(is_code) => !is_code, // a heading element IS a boundary; a code one never is
            None => {
                ceiling > 0
                    && gap.words <= ceiling
                    && !gap.punctuated
                    && !gap.symbolic
                    && !open_flow[k]
            }
        })
        .collect();

    // Grow code spans from anchors across shred fragments: a maximal run of gaps each a code
    // anchor or shred fragment, containing at least one anchor, IS one code example — the
    // highlighter's shredded tokens rejoin, while a full prose sentence stops the run.
    let mut in_code = vec![false; n];
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
            for slot in &mut in_code[i..=j] {
                *slot = true;
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
            // Governing prose starts after the LAST boundary before the span — the heading bounds
            // the section and keeps its OWN words (ledger #22: never welded onto the first
            // sentence).
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

/// Cap governing prose to its closing [`GOVERNING_CTX`] characters, snapping to a word boundary
/// ONLY when the cap actually cut (ledger #22: an unconditional snap ate the operative "Never"
/// of every prohibition).
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

/// The block's own declared language: the nearest author mark (`brush:`/`language-*` class) among
/// the markup tokens immediately before the span. An author mark is corroborating provenance the
/// reader gets for free — it declares what the block IS, it never decides what is code.
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

// ── Learning the structural roles by exposure (setup only) ────────────────────

/// A markup element needs at least this many occurrences across the corpus before its role is
/// trusted — a rarely-seen tag cannot testify (a support floor, not a classification threshold).
const TAG_ROLE_SUPPORT: u32 = 8;

/// LEARN the register role of each markup element by EXPOSURE over the web curriculum (LINTER.md,
/// "Reading a page is UNDERSTANDING"): read every page's structure, judge each element instance's
/// contained text against the meaning network `reader` just sealed, and tally — an element whose
/// contained text reads decisively as code becomes a code carrier, a short title-shaped one a
/// section heading. No element name is enumerated; the seeds are whatever the corpus used. Called
/// once at setup with the curriculum bodies; the lint path only ever reads the result.
pub fn learn_structure_roles(reader: &CharReader, bodies: &[&str]) -> StructureRoles {
    // Read every page's structure once; the title-shape ceiling is learned from the corpus's own
    // short gaps so "heading-shaped" is the corpus's word-length, not a hand constant.
    let scans: Vec<Vec<Gap>> = bodies
        .iter()
        .map(|b| scan(&crate::doc_crawler::drop_script_style(b), reader).gaps)
        .collect();
    let mut title_counts: Vec<u32> = Vec::new();
    for gaps in &scans {
        for (k, g) in gaps.iter().enumerate() {
            if !g.punctuated && prose_majority(g) && !gap_open_flow(gaps, k) {
                title_counts.push(g.words);
            }
        }
    }
    let Some(ceiling) = title_ceiling(&mut title_counts) else {
        return StructureRoles::new();
    };
    // Tally each element INSTANCE over the WHOLE text it contains — `<pre>` receives the code its
    // highlighter spans wrap, `<h1>` its full title even when an inline mark splits it. Code
    // evidence is the register judgment itself; heading evidence needs title shape, majority
    // English, and a first fragment that does not continue an open sentence.
    let mut tally: std::collections::HashMap<u64, (u32, u32, u32)> = Default::default();
    for gaps in &scans {
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
            let english_majority = english * 2 >= words;
            let code = !english_majority && (symbolic || words >= 2);
            let heading = words <= ceiling
                && !punctuated
                && english * 2 >= words
                && !instance_mid_sentence(gaps, first, depth);
            let e = tally.entry(seed).or_insert((0, 0, 0));
            e.0 += 1;
            e.1 += u32::from(code);
            e.2 += u32::from(heading);
        }
    }
    let mut votes = Vec::new();
    for (seed, (support, code, heading)) in tally {
        if support < TAG_ROLE_SUPPORT {
            continue;
        }
        // A clear ¾ majority earns the role; a mixed element stays meaning-judged.
        if code * 4 >= support * 3 {
            votes.push((seed, 1));
        } else if heading * 4 >= support * 3 {
            votes.push((seed, -1));
        }
    }
    StructureRoles::from_learned(votes, ceiling)
}

/// The title-shape ceiling: the 90th percentile of the word counts of short-mode candidates —
/// unpunctuated, majority-English gaps (the corpus's own headings and captions). Clamped by a
/// runaway valve, not a working limit. `None` on too little evidence to calibrate.
fn title_ceiling(counts: &mut Vec<u32>) -> Option<u32> {
    if counts.len() < 50 {
        return None;
    }
    counts.sort_unstable();
    Some(counts[counts.len() * 9 / 10].clamp(2, 24))
}

/// Whether two adjacent gaps' containment stacks NEST — one is a prefix of the other (same element
/// INSTANCES, not merely names), so a sentence can flow between them. A heading and its section's
/// prose never nest, so flow stops exactly where sections do.
fn stacks_nest(a: &[(u32, u64)], b: &[(u32, u64)]) -> bool {
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    long.starts_with(short)
}

/// Whether gap `k` belongs to an OPEN prose sentence (the candidate filter role-learning uses for
/// title shapes): the previous gap is unterminated majority-English prose that flows in, or this
/// gap ends unterminated and flows into the next.
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

/// Whether the element instance whose first contained gap is `gaps[first]` (at `depth` in that
/// gap's stack) is MID-SENTENCE: the gap just before it is unterminated majority-English prose
/// whose containment nests with the instance's PARENT stack — a sentence open OUTSIDE flows in
/// across the element boundary. Flow INSIDE the instance never disqualifies it (its text is judged
/// whole).
fn instance_mid_sentence(gaps: &[Gap], first: usize, depth: usize) -> bool {
    first > 0 && {
        let p = &gaps[first - 1];
        let parent = &gaps[first].stack[..depth];
        !p.terminal && p.english_words * 2 >= p.words && stacks_nest(&p.stack, parent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint_char::{CharReader, StructureRoles};

    /// A tiny char brain for offline tests: it binds the English words the test prose uses into
    /// its meaning network (so `prose_majority` is real), and carries hand-set roles for the
    /// element names the fixtures use — the product code never names a tag, only the fixture does.
    fn test_brain(roles: &[(&str, i8)]) -> CharReader {
        let mut r = CharReader::new();
        // The dictionary words the fixtures' prose is built from — each bound to a throwaway
        // definition so `meaning_of` resolves it. Construct words (var, goto, blink…) are left
        // unbound, exactly as a real dictionary leaves them.
        let english = [
            "never", "use", "the", "statement", "to", "declare", "variable", "it", "hoists",
            "silently", "let", "const", "instead", "which", "scope", "block", "always", "release",
            "resources", "before", "returning", "from", "function", "sections", "are", "introduced",
            "here", "with", "full", "sentence", "of", "prose", "error", "handling", "anything",
            "example", "following", "shows", "failing", "call", "run", "and", "observe",
        ];
        for w in english {
            r.meanings_mut().bind(w, &["thing"]);
        }
        r.meanings_mut().seal();
        let votes: Vec<(u64, i8)> =
            roles.iter().map(|(name, role)| (crate::lint_ai::token_seed(name), *role)).collect();
        // A modest learned ceiling so shape-based boundaries are exercised alongside roles.
        r.set_structure(StructureRoles::from_learned(votes, 8));
        r
    }

    /// A shredded highlighter block reads as ONE code span, its code survives verbatim, the
    /// section body governs it, and the title's own words never weld in (ledger #22).
    #[test]
    fn a_shredded_code_block_reads_as_one_span_and_prose_governs_it() {
        let brain = test_brain(&[("h2", -1)]);
        let body = r#"<h2>Variables</h2>
            <p>Never use the var statement to declare a variable; it hoists silently.</p>
            <div><span>var</span> <span>total</span> <span>=</span> <span>compute(items);</span>
            <span>var</span> <span>flag</span> <span>=</span> <span>!!getOpts(cfg);</span></div>
            <p>Use let or const instead, which scope to the block.</p>"#;
        let units = read_page(body, &brain);
        assert_eq!(
            units.len(),
            1,
            "one merged code span: {:?}",
            units.iter().map(|u| &u.code).collect::<Vec<_>>()
        );
        assert!(
            units[0].code.contains("var total = compute(items);"),
            "code survives verbatim: {:?}",
            units[0].code
        );
        assert!(
            units[0].prose.contains("Never use the var statement"),
            "the governing prose is the section body: {:?}",
            units[0].prose
        );
        assert!(
            !units[0].prose.contains("Variables"),
            "the title's own words never weld into the prose (ledger #22)"
        );
    }

    /// The author's `language-*` class is read as the block's hint.
    #[test]
    fn the_authors_language_mark_is_read_as_a_hint() {
        let brain = test_brain(&[("pre", 1)]);
        let body = r#"<p>The following example shows the failing call, run it and observe the error.</p>
            <pre class="language-python">raise ValueError(f"bad {x!r}") if x else compute_totals(rows[0:12], key=lambda r: r.total)</pre>"#;
        let units = read_page(body, &brain);
        assert_eq!(
            units.len(),
            1,
            "units: {:?}",
            units.iter().map(|u| &u.code).collect::<Vec<_>>()
        );
        assert_eq!(units[0].hint, "python", "the author's own mark declares the block's language");
    }

    /// A learned code role reads ordinary English words inside `<pre>` as CODE, and a same-register
    /// heading stays a boundary so the intro prose above it never governs the block. This is the
    /// h1-vs-pre discrimination the old layer needed, now on the char substrate.
    #[test]
    fn a_learned_code_role_reads_english_words_inside_pre_as_code() {
        let brain = test_brain(&[("pre", 1), ("h2", -1)]);
        let body = r#"<p>Sections are introduced here with a full sentence of prose.</p>
            <h2>error handling</h2>
            <p>Always release resources before returning from the function.</p>
            <pre><code><span>goto</span> <span>cleanup;</span></code></pre>"#;
        let units = read_page(body, &brain);
        assert_eq!(
            units.len(),
            1,
            "the pre block is one unit: {:?}",
            units.iter().map(|u| &u.code).collect::<Vec<_>>()
        );
        assert!(units[0].code.contains("goto cleanup;"), "code: {:?}", units[0].code);
        assert!(units[0].prose.contains("release resources"), "prose: {:?}", units[0].prose);
        assert!(
            !units[0].prose.contains("introduced here"),
            "the h2 role bounds the section: {:?}",
            units[0].prose
        );
    }

    /// A mid-sentence inline mark with no learned role never cuts the governing prose.
    #[test]
    fn a_mid_sentence_inline_mark_never_cuts_the_governing_prose() {
        let brain = test_brain(&[("pre", 1), ("h2", -1)]);
        let body = r#"<p>Never use the <code>var</code> statement to declare anything.</p>
            <pre>var total = compute(items);</pre>"#;
        let units = read_page(body, &brain);
        assert_eq!(
            units.len(),
            1,
            "units: {:?}",
            units.iter().map(|u| &u.code).collect::<Vec<_>>()
        );
        assert!(
            units[0].prose.contains("Never use the") && units[0].prose.contains("statement"),
            "the whole sentence governs, uncut by the inline mark: {:?}",
            units[0].prose
        );
    }

    /// The curriculum gate: a brain that learned no page structure refuses to read a page, never
    /// hand-parses it.
    #[test]
    fn an_untrained_brain_reads_no_units() {
        let brain = CharReader::new(); // no page structure learned at all
        let body = "<h2>Variables</h2><pre>goto cleanup</pre>";
        assert!(
            read_page(body, &brain).is_empty(),
            "no structure ⇒ no units (the curriculum gate)"
        );
    }
}
