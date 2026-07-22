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
///
/// `end` and `prose_from` (PASS 39) are the RAW byte offsets `at`/`prose` were sliced from —
/// exposed so the label exporter ([`crate::lint_labels`]) can ground its code/governing-prose
/// training spans in the same body this unit was read from, with no new judgment: this struct
/// already carries the facts, these two fields just stop discarding them.
#[derive(Debug)]
pub struct PageUnit {
    pub at: usize,
    pub end: usize,
    pub prose: String,
    pub prose_from: usize,
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
        let gap = text_gap(body, at, end, reader, stack.clone());
        if gap.words > 0 {
            gaps.push(gap);
        }
        at = end;
    }
    Scan { gaps, tags }
}

/// Build the aggregate [`Gap`] for the text run `body[start..end]` under containment `stack`,
/// judging each whitespace-delimited word's English-ness against the meaning network. The shared
/// word-shape former: BOTH the HTML tag scan ([`scan`]) and the markdown marker scan
/// ([`scan_markdown`]) emit gaps through it, so a fenced code block and a `<pre>` block are read by
/// the identical typography — this is the curriculum's transfer point (LINTER.md rung 1).
fn text_gap(body: &str, start: usize, end: usize, reader: &CharReader, stack: Vec<(u32, u64)>) -> Gap {
    let mut gap = Gap {
        start,
        end,
        words: 0,
        english_words: 0,
        punctuated: false,
        terminal: false,
        symbolic: false,
        stack,
    };
    let mut w_start: Option<usize> = None;
    // Walk char-wise so multibyte text never splits; a word closes at whitespace.
    let text = &body[start..end];
    let mut it = text.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        let abs = start + i;
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
    gap
}

// ── Markdown typography (the curriculum precursor to web reading, LINTER.md rung 1) ──

/// The seed of the markdown FENCED-CODE marker — a run of ≥3 backticks or tildes, keyed by the
/// marker's OWN typography INCLUDING its author-supplied INFO-STRING (rung 1): `` ```js ``,
/// `` ```plain ``, and a bare `` ``` `` are DISTINCT markers, exactly as `<pre class="…">` variants
/// would be. The info-string is part of the fence's own tag — the author declares the register the
/// same way an element's attributes ride its tag — so a code fence and an output/`plain` fence learn
/// SEPARATE roles by exposure ([`learn_structure_roles`]), never assigned here. Same
/// [`crate::lint_ai::token_seed`] the HTML scan keys element names with, so a tagged fence and an
/// HTML `<pre>` still occupy the same learned role SPACE. An empty info-string keys the bare
/// `` ``` `` marker, which keeps whatever register it honestly earns.
fn md_fence_seed(info: &str) -> u64 {
    if info.is_empty() {
        crate::lint_ai::token_seed("```")
    } else {
        crate::lint_ai::token_seed(&format!("```{}", info.to_ascii_lowercase()))
    }
}

/// The INFO-STRING of a markdown fence OPEN line — the text after the run of fence characters, its
/// first whitespace-delimited token (`` ```js `` → `js`, `` ```js-nolint `` → `js-nolint`, `` ```
/// title=x `` → the first word, a bare `` ``` `` → empty). Pure typography (the author's own tag on
/// the fence), the markdown analogue of reading an element's attributes; lowercased at the seed.
fn md_fence_info(trimmed: &str) -> &str {
    let c = trimmed.chars().next().unwrap_or('`');
    trimmed.trim_start_matches(c).split_whitespace().next().unwrap_or("")
}

/// The seed of the markdown ATX-HEADING marker — a line-leading run of `#`. Keyed by the marker's
/// own character; the heading ROLE is learned by exposure, never assigned.
fn md_heading_seed() -> u64 {
    crate::lint_ai::token_seed("#")
}

/// Whether a line's leading (whitespace-trimmed) text opens or closes a markdown code FENCE — ≥3
/// of the same fence char (`` ` `` or `~`). Pure typography, no info-string parsing.
fn is_md_fence(trimmed: &str) -> bool {
    let Some(c) = trimmed.chars().next() else { return false };
    (c == '`' || c == '~') && trimmed.chars().take_while(|&x| x == c).count() >= 3
}

/// Tokenize a markdown body into gaps by its LINE typography — the markdown analogue of [`scan`]'s
/// `<…>` tokenizer. A fenced block wraps its content lines in the fence marker element; an ATX
/// heading wraps its trailing text in the heading marker element; every other line is plain prose
/// under no element. Byte ranges index the ORIGINAL body so [`read_scan`] slices code and prose
/// verbatim. Pure typography (line-leading markers), never a semantic markdown parser: the markers'
/// ROLES are learned by exposure, this only reads their characters.
fn scan_markdown(md: &str, reader: &CharReader) -> Scan {
    let heading = md_heading_seed();
    let mut gaps = Vec::new();
    let mut inst: u32 = 0;
    // The open fence carries its instance id AND its info-string-keyed seed, so every content line of
    // the block is contained by the marker the AUTHOR tagged (`` ```js `` vs `` ```plain ``).
    let mut fence_open: Option<(u32, u64)> = None;
    let bytes = md.as_bytes();
    let mut at = 0usize;
    while at < bytes.len() {
        let eol = md[at..].find('\n').map(|i| at + i).unwrap_or(bytes.len());
        let line = &md[at..eol];
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if let Some((open, seed)) = fence_open {
            if is_md_fence(trimmed) {
                fence_open = None; // the closing fence ends the block
            } else {
                let g = text_gap(md, at, eol, reader, vec![(open, seed)]);
                if g.words > 0 {
                    gaps.push(g);
                }
            }
        } else if is_md_fence(trimmed) {
            fence_open = Some((inst, md_fence_seed(md_fence_info(trimmed))));
            inst += 1;
        } else if trimmed.starts_with('#') {
            let hashes = trimmed.chars().take_while(|&c| c == '#').count();
            let after = &trimmed[hashes..];
            // An ATX heading is 1–6 `#` followed by whitespace then text; `#word` is prose.
            if hashes <= 6 && after.starts_with(char::is_whitespace) {
                let lead = indent + hashes + (after.len() - after.trim_start().len());
                let g = text_gap(md, at + lead, eol, reader, vec![(inst, heading)]);
                inst += 1;
                if g.words > 0 {
                    gaps.push(g);
                }
            } else {
                let g = text_gap(md, at, eol, reader, Vec::new());
                if g.words > 0 {
                    gaps.push(g);
                }
            }
        } else {
            let g = text_gap(md, at, eol, reader, Vec::new());
            if g.words > 0 {
                gaps.push(g);
            }
        }
        at = if eol < bytes.len() { eol + 1 } else { eol };
    }
    Scan { gaps, tags: Vec::new() }
}

/// READ a markdown body into its [`PageUnit`]s — the markdown counterpart of [`read_page`], standing
/// on the SAME learned roles and the SAME unit former ([`read_scan`]). A brain that learned no page
/// structure reads nothing. This is the curriculum precursor: markdown structure the reader learned
/// by exposure segments a doc into heading-governed code-fence units before the web layer is read.
pub fn read_markdown(md: &str, reader: &CharReader) -> Vec<PageUnit> {
    if reader.structure().is_empty() {
        return Vec::new();
    }
    read_scan(scan_markdown(md, reader), md, reader)
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
    english_lemma(has, word, suffixes).is_some()
}

/// The English LEMMA `word` reduces to under `has`, or `None` when nothing accounts for it — the
/// base form [`english_inflection`] answers a bool from (`english_inflection == english_lemma…().
/// is_some()`, so the read-path judgment is bit-identical). Returned so a caller that must QUERY the
/// resolved lemma (the meaning net's `related`, not just its `has`) can key a keyed singular from a
/// plural (`attributes → attribute`). Pure suffix morphology, same set and order as the bool form:
/// the word itself if known, else `-ies → -y`, else each suffix stripped (undoing a doubled final
/// consonant before a vowel suffix). Never a word list.
pub(crate) fn english_lemma(has: impl Fn(&str) -> bool, word: &str, suffixes: &[&str]) -> Option<String> {
    let w = word.to_lowercase();
    if has(&w) {
        return Some(w);
    }
    let resolved = |stem: &str| (stem.chars().count() >= 3 && has(stem)).then(|| stem.to_string());
    if let Some(base) = w.strip_suffix("ies") {
        if let Some(lemma) = resolved(&format!("{base}y")) {
            return Some(lemma);
        }
    }
    for suffix in suffixes {
        if let Some(stem) = w.strip_suffix(suffix) {
            if let Some(lemma) = resolved(stem) {
                return Some(lemma);
            }
            // A doubled final consonant before a vowel suffix drops one (`running`, `hotter`).
            if matches!(*suffix, "ing" | "ed" | "er" | "est") {
                let chars: Vec<char> = stem.chars().collect();
                if chars.len() >= 2 && chars[chars.len() - 1] == chars[chars.len() - 2] {
                    let deduped: String = chars[..chars.len() - 1].iter().collect();
                    if let Some(lemma) = resolved(&deduped) {
                        return Some(lemma);
                    }
                }
            }
        }
    }
    None
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
    read_scan(scan(body, reader), body, reader)
}

/// Form units from an already-tokenized [`Scan`] over `body` — shared by the HTML reader
/// ([`read_page`]) and the markdown reader ([`read_markdown`]), so both typographies segment into
/// heading-governed code units by the identical register logic. `body` is the ORIGINAL source the
/// scan indexes; code and prose are sliced from it verbatim.
fn read_scan(scan: Scan, body: &str, reader: &CharReader) -> Vec<PageUnit> {
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
                end,
                prose,
                prose_from,
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

// ── Cross-page-invariance chrome discovery (site-scoped, setup only) ───────────

/// A text run counts as site chrome once it recurs on at least this many DISTINCT pages of the same
/// site — the north-star's own law ("an element whose structure and style and content is invariant
/// across a site's pages is navigation/boilerplate with zero meaning"), approximated by CONTENT-run
/// recurrence. This is the PROTOTYPE's measured separation floor (`examples/reader_grade`): at ≥8
/// same-site pages the site-invariant text mass separates cleanly — W3Schools 70.1% (its menu/
/// breadcrumb/footer) vs MDN reference 19.8% / API 23.8% (reference furniture). It is the SAME
/// repetition-support floor as [`TAG_ROLE_SUPPORT`]: a signal is trusted-by-repetition only once at
/// least this many independent instances testify, never on one sighting.
const CHROME_PAGE_SUPPORT: usize = 8;

/// The invariance key of one tag-separated text run — the atom the chrome detector counts across a
/// site's pages. Pure typography: whitespace-collapse the run, and only a run of at least two words
/// and six characters is a comparison atom (a lone word or a stray symbol never keys as chrome). The
/// key is the run's content hash ([`crate::lint_ai::token_seed`]), so two byte-identical runs on two
/// pages collide and a paraphrase does not — invariance is EXACT recurrence, never similarity.
fn run_key(text: &str) -> Option<u64> {
    let norm: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if norm.len() < 6 || norm.split(' ').count() < 2 {
        return None;
    }
    Some(crate::lint_ai::token_seed(&norm))
}

/// Per-HOST site chrome: the set of text-run keys that recur across a site's pages (navigation,
/// breadcrumb, footer, sidebar menu). Keyed by host because invariance is a property of ONE site —
/// a run shared by MDN and W3Schools is not chrome, and each site's furniture is discarded only
/// against its own pages. Built once over a whole-site corpus at setup ([`site_chrome`]); the strip
/// ([`SiteChrome::strip`]) removes chrome text before any reader (learned [`read_page`] or the hand
/// anatomy) forms prose or units, so boilerplate never welds into a page's governing meaning.
pub struct SiteChrome {
    /// host → {run key → distinct-page support count}. The count is kept (PASS 36 — the recall
    /// census): a fact eaten as chrome must be AUDITABLE, so each stripped run's page support
    /// stands on the chrome itself ([`SiteChrome::page_support`]) — key hashes and counts only,
    /// never the run text (the multi-GB corpus law). Membership alone still decides the strip.
    chrome: std::collections::HashMap<String, std::collections::HashMap<u64, usize>>,
}

impl SiteChrome {
    /// Whether no host contributed any chrome — a corpus too small or too varied to find invariance.
    pub fn is_empty(&self) -> bool {
        self.chrome.values().all(std::collections::HashMap::is_empty)
    }

    /// The number of distinct chrome runs discovered across all hosts — a measurement handle.
    pub fn len(&self) -> usize {
        self.chrome.values().map(std::collections::HashMap::len).sum()
    }

    /// PASS 36 — the chrome AUDIT surface: per-host, per-run-key distinct-page support counts, as
    /// discovered by the build fold. A fact suspected eaten as chrome is checked here (key its run
    /// with the same normalization the detector used and look it up), without the strip ever
    /// recording per-run text. Read-only; empty for a corpus that discovered no chrome.
    pub fn page_support(
        &self,
    ) -> &std::collections::HashMap<String, std::collections::HashMap<u64, usize>> {
        &self.chrome
    }

    /// STRIP `url`'s site chrome from `body`: every tag-separated text run whose key is invariant on
    /// this page's host is blanked to a space, tags and attributes preserved verbatim (so a
    /// `class="notecard deprecated"` marker, an `id=` anchor, and `<pre><code>` example code all
    /// survive — only recurring PROSE text is removed). A page whose host contributed no chrome, or
    /// a url with no host, is returned unchanged. Pure typography: `<…>` is a markup token whose
    /// interior is copied as-is, text between tokens is the run the key is computed over.
    pub fn strip(&self, url: &str, body: &str) -> String {
        let Some(host) = crate::lint_docs::url_host(url) else { return body.to_string() };
        let Some(set) = self.chrome.get(&host) else { return body.to_string() };
        if set.is_empty() {
            return body.to_string();
        }
        let bytes = body.as_bytes();
        let mut out = String::with_capacity(body.len());
        let mut at = 0usize;
        let mut run_start = 0usize;
        while at < bytes.len() {
            if bytes[at] == b'<' {
                let run = &body[run_start..at];
                if run_key(run).is_some_and(|k| set.contains_key(&k)) {
                    out.push(' ');
                } else {
                    out.push_str(run);
                }
                let close = body[at..].find('>').map(|i| at + i + 1).unwrap_or(bytes.len());
                out.push_str(&body[at..close]);
                at = close;
                run_start = at;
                continue;
            }
            at += 1;
        }
        let tail = &body[run_start..];
        if run_key(tail).is_some_and(|k| set.contains_key(&k)) {
            out.push(' ');
        } else {
            out.push_str(tail);
        }
        out
    }
}

/// DISCOVER each site's chrome by cross-page text-run invariance over a whole-site `pages` corpus
/// (LINTER.md, "Cross-page invariance = chrome, discarded"). Groups pages by host, counts on how
/// many DISTINCT pages of that host each text-run key appears (deduped within a page), and keeps the
/// keys recurring on ≥ [`CHROME_PAGE_SUPPORT`] pages. No element name and no site name is consulted:
/// the discriminator is purely how often a run's exact content recurs across the same site. Pure
/// over the corpus; called once at setup, never on the lint path.
pub fn site_chrome(pages: &[(String, String)]) -> SiteChrome {
    use std::collections::{HashMap, HashSet};
    let mut counts: HashMap<String, HashMap<u64, usize>> = HashMap::new();
    for (url, body) in pages {
        let Some(host) = crate::lint_docs::url_host(url) else { continue };
        let mut on_page: HashSet<u64> = HashSet::new();
        let mut in_tag = false;
        let mut run_start = 0usize;
        let b = body.as_bytes();
        for i in 0..b.len() {
            match b[i] {
                b'<' => {
                    if !in_tag {
                        if let Some(k) = run_key(&body[run_start..i]) {
                            on_page.insert(k);
                        }
                    }
                    in_tag = true;
                }
                b'>' => {
                    in_tag = false;
                    run_start = i + 1;
                }
                _ => {}
            }
        }
        if let Some(k) = run_key(&body[run_start..]) {
            on_page.insert(k);
        }
        let host_counts = counts.entry(host).or_default();
        for k in on_page {
            *host_counts.entry(k).or_insert(0) += 1;
        }
    }
    // Keep each surviving key's page-support count (PASS 36): the strip's membership test is
    // byte-identical in behavior, and the counts make a chrome-eaten fact auditable for free.
    let chrome = counts
        .into_iter()
        .map(|(host, ks)| {
            let kept: HashMap<u64, usize> =
                ks.into_iter().filter(|(_, n)| *n >= CHROME_PAGE_SUPPORT).collect();
            (host, kept)
        })
        .collect();
    SiteChrome { chrome }
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
pub fn learn_structure_roles(reader: &CharReader, bodies: &[&str], md_bodies: &[&str]) -> StructureRoles {
    // Read every page's structure once; the title-shape ceiling is learned from the corpus's own
    // short gaps so "heading-shaped" is the corpus's word-length, not a hand constant. The markdown
    // curriculum (LINTER.md rung 1) is read through its OWN line typography ([`scan_markdown`]) and
    // its marker instances tally into the SAME role space — a fence learns "code carrier", an ATX
    // heading learns "section heading", exactly as `<pre>`/`<h1>` do, so the roles transfer.
    let mut scans: Vec<Vec<Gap>> = bodies
        .iter()
        .map(|b| scan(&crate::doc_crawler::drop_script_style(b), reader).gaps)
        .collect();
    scans.extend(md_bodies.iter().map(|b| scan_markdown(b, reader).gaps));
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
    // Diagnostic (rung 1/2 measurement, off unless HELPERS_ROLE_TRACE is set): report the raw vote
    // tally for the markdown markers so register purity is visible before the ¾ bar decides. The
    // fence FAMILY is now info-string-keyed (rung 1), so the labels are drawn from the corpus's own
    // opening-fence info-strings (data, not a code list) plus the bare fence and the heading. No
    // effect on the learned roles.
    if std::env::var_os("HELPERS_ROLE_TRACE").is_some() {
        let mut markers: Vec<String> = vec!["```".to_string(), "#".to_string()];
        for body in md_bodies {
            let mut open = false;
            for line in body.lines() {
                let trimmed = line.trim_start();
                if is_md_fence(trimmed) {
                    if !open {
                        let info = md_fence_info(trimmed);
                        if !info.is_empty() {
                            let m = format!("```{}", info.to_ascii_lowercase());
                            if !markers.contains(&m) {
                                markers.push(m);
                            }
                        }
                    }
                    open = !open;
                }
            }
        }
        for marker in &markers {
            let seed = crate::lint_ai::token_seed(marker);
            let label = if marker == "#" { "heading" } else { "fence" };
            if let Some((support, code, heading)) = tally.get(&seed).copied() {
                let pct = |n: u32| if support > 0 { 100 * n / support } else { 0 };
                eprintln!(
                    "[role-trace] {label} ({marker}): support {support} / code {code} ({}%) / heading {heading} ({}%)",
                    pct(code),
                    pct(heading)
                );
            } else {
                eprintln!("[role-trace] {label} ({marker}): no instances tallied");
            }
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

    /// RUNG 1 (LINTER.md, the curriculum precursor): a real markdown document segments into a
    /// heading-governed code-fence unit. The fence marker carries the learned CODE role, the ATX
    /// heading marker the learned HEADING (boundary) role — the SAME role space HTML `<pre>`/`<h2>`
    /// use — so the same unit former reads markdown as it reads a web page. Markdown-marker roles are
    /// keyed by the marker's OWN characters (`` ``` ``/`#`), never a role name.
    /// The fence marker is keyed by the author's own INFO-STRING (rung 1): `` ```js ``, `` ```plain ``,
    /// and a bare `` ``` `` are DISTINCT markers that can learn separate roles, exactly as `<pre
    /// class="…">` variants would. Pure typography — the seed is a hash of the fence literal plus its
    /// info-string, case-folded like an element name, never compared to a list.
    #[test]
    fn fence_markers_are_keyed_by_their_info_string() {
        assert_eq!(md_fence_info("```js"), "js");
        assert_eq!(md_fence_info("```js-nolint  title=x"), "js-nolint");
        assert_eq!(md_fence_info("~~~python"), "python");
        assert_eq!(md_fence_info("```"), "", "a bare fence has no info-string");
        assert_ne!(md_fence_seed("js"), md_fence_seed("plain"), "tagged fences are different markers");
        assert_ne!(md_fence_seed("js"), md_fence_seed(""), "a tagged fence differs from the bare fence");
        assert_eq!(md_fence_seed(""), crate::lint_ai::token_seed("```"), "the bare fence keeps its own seed");
        assert_eq!(md_fence_seed("JS"), md_fence_seed("js"), "the info-string case-folds like an element name");
    }

    #[test]
    fn a_markdown_doc_segments_into_heading_prose_and_code_fence_units() {
        // The fence carries the `js` info-string, so its learned role is keyed `` ```js `` (rung 1).
        let brain = test_brain(&[("```js", 1), ("#", -1)]);
        let body = "# Introduced sections\n\
            \n\
            Sections are introduced here with a full sentence of prose.\n\
            \n\
            ## error handling\n\
            \n\
            Never use the var statement to declare a variable; it hoists silently.\n\
            \n\
            ```js\n\
            var total = compute(items);\n\
            ```\n\
            \n\
            Use let or const instead, which scope to the block.\n";
        let units = read_markdown(body, &brain);
        assert_eq!(
            units.len(),
            1,
            "one fenced code unit: {:?}",
            units.iter().map(|u| &u.code).collect::<Vec<_>>()
        );
        assert!(
            units[0].code.contains("var total = compute(items);"),
            "the fenced code survives verbatim: {:?}",
            units[0].code
        );
        assert!(
            units[0].prose.contains("Never use the var statement"),
            "the section body governs the fence: {:?}",
            units[0].prose
        );
        assert!(
            !units[0].prose.contains("introduced here"),
            "the `## error handling` heading bounds the section, so the intro prose never governs: {:?}",
            units[0].prose
        );
    }

    /// A text run recurring across a site's pages (the menu) is discovered as chrome and blanked,
    /// while a run UNIQUE to one page (its governing prose) survives — the cross-page-invariance
    /// filter, site-scoped and learned purely from recurrence, no element or site name consulted.
    #[test]
    fn site_invariance_discovers_and_strips_the_recurring_menu_but_keeps_unique_prose() {
        // A shared navigation run on every page; each page's own body prose is unique. The menu run
        // is well over the two-word / six-char comparison-atom floor, so it keys.
        let menu = "<div id=\"m\">Home About Contact Reference Guide</div>";
        let pages: Vec<(String, String)> = (0..CHROME_PAGE_SUPPORT)
            .map(|i| {
                let url = format!("https://site.test/page{i}");
                let body = format!("{menu}<p>Unique page {i} governing sentence about widgets.</p>");
                (url, body)
            })
            .collect();
        let chrome = site_chrome(&pages);
        assert!(!chrome.is_empty(), "the menu recurs on >= the support floor of pages ⇒ chrome");

        let stripped = chrome.strip(&pages[0].0, &pages[0].1);
        assert!(
            !stripped.contains("Home About Contact Reference Guide"),
            "the invariant menu text is blanked: {stripped}"
        );
        assert!(
            stripped.contains("Unique page 0 governing sentence about widgets."),
            "the page's own unique prose survives: {stripped}"
        );
        assert!(stripped.contains("<div id=\"m\">"), "tags and attributes are preserved verbatim");
    }

    /// A run seen on only a FEW pages of a site (below the support floor) is NOT chrome — a genuine
    /// recurring sentence on two pages is content, not boilerplate.
    #[test]
    fn a_run_below_the_page_support_floor_is_not_chrome() {
        let shared = "<p>This appears on just a couple of pages only.</p>";
        let pages: Vec<(String, String)> = (0..3)
            .map(|i| (format!("https://site.test/p{i}"), format!("{shared}<p>body {i}</p>")))
            .collect();
        let chrome = site_chrome(&pages);
        let stripped = chrome.strip(&pages[0].0, &pages[0].1);
        assert!(
            stripped.contains("This appears on just a couple of pages only."),
            "below the support floor ⇒ kept as content: {stripped}"
        );
    }
}
