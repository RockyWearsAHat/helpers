//! `lint_lang_layer` — the LANGUAGE-DOC reading rung: clean, per-construct governing prose and the
//! docs' own paired bad/good examples, read STRUCTURALLY from real reference/rule pages.
//!
//! Contract: `LINTER.md` → "The language-doc reading rung — structural per-construct governing prose".
//! The construct-module workflow ([`crate::lint_module`]) used to PROPOSE from `memory.bindings[].prose`,
//! but on the real crawl those "sentences" are garbled (code fragments and site chrome the word-substrate
//! binding step interleaves with prose). This module supplies the workflow's PROPOSE material instead, by
//! applying the proven [`crate::lint_html_layer`] reading pattern to language-doc pages: one construct per
//! reference/rule page, subject and governing prose read as STRUCTURAL facts of the page.
//!
//! Frozen throughout: this module never judges English and never names a language or a construct — page
//! kind is read from the URL path (a per-SOURCE structural marker, INTERIM like the HTML layer's MDN
//! anchors), the prohibited constructs are DATA read from the page's own prose and its own bad/good
//! examples, and the governing prose is tag-stripped English handed on to the frozen comparator unchanged.

use crate::doc_crawler::{drop_script_style, strip_tags};
use crate::lint_english::English;
use crate::lint_html_layer::sections;
use crate::lint_trace::Bridge;

/// Section anchors whose prose is page FURNITURE, not governing definition/usage/rule text — the union
/// of MDN's reference furniture and a linter rule page's chrome. Each is a URL-fragment slug the page
/// publishes for itself (`<h2 id="…">`), so excluding by it is a STRUCTURAL page-role filter, never a
/// judgement of construct meaning. INTERIM, like the [`crate::lint_html_layer`] anchors.
/// How many backtick-bearing governing sentences a rule page's PROSE construct extraction reads — the
/// lead summary and rule-details name the subject in the first few; a cap that bounds the expensive
/// meaning-alignment ([`Bridge::constructs_named`]) so training stays in seconds.
const PROSE_SENTENCE_LIMIT: usize = 4;

const NON_GOVERNING_ANCHORS: &[&str] = &[
    // MDN reference furniture
    "try_it",
    "examples",
    "specifications",
    "browser_compatibility",
    "see_also",
    "feedback",
    // Linter rule-page chrome
    "options",
    "version",
    "resources",
    "further-reading",
    "related-rules",
    "when-not-to-use-it",
    "known-limitations",
    "handled_by_typescript",
];

/// One language-doc page read into the workflow's PROPOSE material: the clean governing sentences and the
/// construct(s) the page structurally PROHIBITS. `prohibited` records whether the page's ROLE is a
/// prohibition at all (a linter rule page, or a reference page carrying a deprecation notecard); only a
/// prohibited page contributes candidates.
#[derive(Debug, Clone)]
pub struct DocPage {
    /// The documentation url — the finding's citation and the page-of-origin key.
    pub url: String,
    /// Whether the page's ROLE prohibits its subject (rule page, or deprecated reference page).
    pub prohibited: bool,
    /// Whether the page STRUCTURALLY ATTESTS its own subject is DEPRECATED — a reference page carrying a
    /// deprecation notecard (never a rule page). This is the authoritative proof for the NOTECARD
    /// GRADUATION PATH ([`crate::lint_module`]): when a reference site publishes IDENTICAL deprecation
    /// boilerplate for every deprecated construct, the English self-test's foil is degenerate BY
    /// CONSTRUCTION and that referee honestly cannot apply — but the page's own notecard is a STATED
    /// STRUCTURAL FACT that its subject is deprecated, which graduates the rule directly. `false` for a
    /// rule page (its distinguishable prose keeps the English self-test path).
    pub attested_deprecated: bool,
    /// Clean governing sentences of the page (lead definition + usage/rule-details prose), furniture and
    /// example code removed, code typography preserved as backticks so symbol constructs survive.
    pub governing: Vec<String>,
    /// The candidate construct(s) the page names as prohibited — DATA read from its prose and its own
    /// incorrect examples. UNCONFIRMED: the caller (which knows the language) verifies each against the
    /// page's own [`incorrect`](Self::incorrect)/[`correct`](Self::correct) examples with the frozen
    /// firing, so the remedy (`const`/`let`/`===`) and comment-embedded names are excluded soundly.
    pub constructs: Vec<String>,
    /// The page's own "incorrect code" example blocks (the docs' bad examples) — where present.
    pub incorrect: Vec<String>,
    /// The page's own "correct code" example blocks (the docs' good examples) — where present.
    pub correct: Vec<String>,
}

/// Whether the url is a per-construct REFERENCE page — its path names a documentation reference section
/// (`/reference/`). A per-SOURCE structural marker (MDN publishes its reference under `…/Reference/…`),
/// INTERIM; it names no language.
fn is_reference_page(url: &str) -> bool {
    url.to_lowercase().contains("/reference/")
}

/// Whether the url is a linter RULE page — its path names a rule directory (`/rules/`). A rule page's
/// ROLE is a prohibition of the construct it documents; a per-SOURCE structural marker, INTERIM.
fn is_rule_page(url: &str) -> bool {
    url.to_lowercase().contains("/rules/")
}

/// Whether the page carries a DEPRECATION notecard — the structural marker a reference site renders for a
/// discouraged feature (`class="notecard deprecated"`, or the MDN lead line "Deprecated: This feature is
/// no longer recommended"). A page-role signal read from the markup/label, never an English judgement of
/// the prose.
fn has_deprecation_notecard(body: &str) -> bool {
    // The markers appear lowercase in the markup (a class attribute, a notecard label), so a raw
    // substring test suffices — no full-body lowercase allocation on every page.
    body.contains("notecard deprecated") || body.contains("no longer recommended")
}

/// Convert inline code typography `<code>X</code>` → `` `X` `` so [`Bridge::extract_construct`] reads a
/// SYMBOL construct (`==`, `===`, `!=`) that plain tag-stripping would discard. The same backtick
/// convention the extractor already reads — no new judgement. Only the code interior is kept (nested tags
/// dropped), so a highlighter's `<span>`-shredded `<code>` still yields one backticked token.
fn code_to_backtick(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut at = 0usize;
    while at < html.len() {
        let rest = &html[at..];
        if starts_with_ci(rest, "<code") {
            if let Some(gt) = rest.find('>') {
                let inner_start = at + gt + 1;
                if let Some(end) = find_ci(&html[inner_start..], "</code>") {
                    let inner = strip_tags(&html[inner_start..inner_start + end]);
                    if !inner.is_empty() {
                        out.push('`');
                        out.push_str(&inner);
                        out.push('`');
                    }
                    at = inner_start + end + "</code>".len();
                    continue;
                }
            }
        }
        let c = rest.chars().next().unwrap();
        out.push(c);
        at += c.len_utf8();
    }
    out
}

/// ASCII case-insensitive prefix test (tag names are ASCII) — used so a `<CODE>` still keys, without
/// allocating a parallel lowercased string whose byte offsets would drift on multibyte input.
fn starts_with_ci(haystack: &str, needle: &str) -> bool {
    haystack.len() >= needle.len()
        && haystack.as_bytes()[..needle.len()].eq_ignore_ascii_case(needle.as_bytes())
}

/// The byte offset of the first ASCII case-insensitive occurrence of `needle` in `haystack`, on char
/// boundaries (`needle` is ASCII, so a match starts and ends on a boundary).
fn find_ci(haystack: &str, needle: &str) -> Option<usize> {
    let (h, n) = (haystack.as_bytes(), needle.as_bytes());
    if n.is_empty() || h.len() < n.len() {
        return None;
    }
    (0..=h.len() - n.len()).find(|&i| h[i..i + n.len()].eq_ignore_ascii_case(n))
}

/// Remove `<pre …>…</pre>` blocks (worked code examples) from a region, replacing each with a space so
/// prose does not weld across the excision — the [`crate::lint_html_layer`] leak-killer, verbatim.
fn strip_pre_blocks(html: &str) -> String {
    let mut out = html.to_string();
    while let Some(s) = out.find("<pre") {
        match out[s..].find("</pre>") {
            Some(e) => out.replace_range(s..s + e + "</pre>".len(), " "),
            None => break,
        }
    }
    out
}

/// The clean GOVERNING sentences of a reference/rule page body: split at the page's own section anchors,
/// drop furniture regions by anchor id, strip `<pre>` example code, preserve `<code>` typography as
/// backticks, tag-strip to prose, split into sentences. Chrome BEFORE the first governing statement (page
/// title, "Skip to main content", Baseline banner) is left behind because it is not sentence-shaped prose
/// the reader keeps — sentences under [`MIN_WORDS`] are dropped.
fn governing_sentences(body: &str) -> Vec<String> {
    const MIN_WORDS: usize = 4;
    let body = drop_script_style(body);
    // The page's CONTENT begins at its title `<h1>` — everything before (the site's search box, sidebar
    // menu, breadcrumb) is chrome that is NOT inside a `<nav>`/`<header>` and so survives
    // `drop_script_style`, welding a huge menu run-on into the first sentence (measured: ESLint's "Clear
    // search Powered by Use ESLint…" leaked as advice and Contradicted every candidate). Cutting to the
    // first `<h1>` is a site-general structural boundary (MDN and ESLint both open content with the title).
    let body = match find_ci(&body, "<h1") {
        Some(i) => body[i..].to_string(),
        None => body,
    };
    let mut out = Vec::new();
    for (anchor, region) in sections(&body) {
        if NON_GOVERNING_ANCHORS.contains(&anchor.as_str()) {
            continue;
        }
        let region = code_to_backtick(&strip_pre_blocks(&region));
        let prose = strip_tags(&region);
        for s in crate::lint_read::sentences(&prose) {
            if s.split_whitespace().count() >= MIN_WORDS {
                out.push(s.to_string());
            }
        }
    }
    out
}

/// Every example CODE block inside a `<div class="<class>">` region — the docs' own paired-example markup
/// (`class="incorrect"` / `class="correct"` on ESLint rule pages). A page-role CLASS filter, the exact
/// analog of the [`crate::lint_html_layer`] W3Schools/WHATWG class cuts — structural markup, never English
/// prose. Each region runs until the next paired-example class marker (or a bounded window). The example
/// code is read from the RAW markup (never `code_to_backtick`, which would wrap the whole block in one
/// backtick pair — the block then parses as a single JS template literal and the construct node vanishes)
/// as the INTERIOR of each `<code>…</code>` only ([`code_interiors`]) — so Prism's line-number gutter
/// (`<span class="line-numbers-rows">`, inside the `<pre>` but AFTER `</code>`) is excluded, not welded on.
fn examples_of_class(body: &str, class: &str) -> Vec<String> {
    let needle = format!("class=\"{class}\"");
    let stops = ["class=\"incorrect\"", "class=\"correct\""];
    let mut out = Vec::new();
    let mut at = 0usize;
    while let Some(rel) = body[at..].find(&needle) {
        let start = at + rel + needle.len();
        // The region ends at the next paired-example marker after this one (or a bounded window).
        let end = stops
            .iter()
            .filter_map(|s| body[start..].find(s).map(|i| start + i))
            .min()
            .unwrap_or(body.len())
            .min(start + 8000);
        for block in code_interiors(&body[start..end]) {
            if block.trim().len() >= 3 {
                out.push(block);
            }
        }
        at = end;
    }
    out
}

/// The `strip_code`-decoded INTERIOR of every `<pre>…<code>…</code>…</pre>` example in a markup region —
/// the clean example code with line structure intact and highlight `<span>`s removed. Only a `<code>`
/// INSIDE a `<pre>` is taken: that is the docs' worked example. This deliberately skips INLINE prose
/// `<code>` (a `` `--fix` ``/`` `null` ``/`` `===` `` mentioned in the option text after the example),
/// which is not an example block and would pollute the per-example firing verification. The enclosing
/// `<pre>` itself is never taken, so a sibling line-number gutter is left behind.
fn code_interiors(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while let Some(rel) = find_ci(&html[at..], "<pre") {
        let pre_open = at + rel;
        let pre_end = match find_ci(&html[pre_open..], "</pre>") {
            Some(e) => pre_open + e,
            None => break,
        };
        // The FIRST `<code>` interior inside this `<pre>` is the example; anything after `</pre>` is not.
        if let Some(crel) = find_ci(&html[pre_open..pre_end], "<code") {
            let copen = pre_open + crel;
            if let Some(gt) = html[copen..].find('>') {
                let inner_start = copen + gt + 1;
                if let Some(cend) = find_ci(&html[inner_start..pre_end], "</code>") {
                    out.push(crate::doc_crawler::strip_code(&html[inner_start..inner_start + cend]));
                }
            }
        }
        at = pre_end + "</pre>".len();
    }
    out
}

/// Normalize an extracted construct token to its firing form: strip a trailing empty-call `()` (prose
/// names a callable `eval()` but the AST node in `eval(userInput)` is the identifier `eval`), and trim
/// surrounding punctuation. Symbol constructs (`==`) pass through unchanged.
fn normalize_construct(raw: &str) -> String {
    let mut c = raw.trim().trim_matches(|ch: char| ch == '`' || ch == ',' || ch == '.' || ch == ':').to_string();
    // An HTML element is named `<marquee>` in prose but its AST tag-name node text is `marquee`; strip
    // the angle brackets so the construct fires. Symbol/property/keyword constructs carry no brackets.
    if c.starts_with('<') && c.ends_with('>') {
        c = c[1..c.len() - 1].to_string();
    }
    if let Some(base) = c.strip_suffix("()") {
        c = base.to_string();
    }
    c
}

/// The multi-character OPERATOR tokens of a code block — a whole whitespace token that is a run of two
/// or more symbol characters (`==`, `!=`, `===`, `++`). Single-character punctuation (`{`, `(`, `;`, a
/// lone `<`) is excluded — ubiquitous syntax, never a prohibited construct (the measured `}`/`{`/`[`
/// junk class). KEYWORDS are deliberately NOT read from examples: a keyword differing between two
/// examples (a `case`/`if`/`const` that happens to appear in the incorrect block only) is incidental,
/// not the rule's subject; a genuinely prohibited keyword (`var`, `with`) is NAMED in the prohibition
/// prose and enters through [`Bridge::constructs_named`], which reads its meaning. Identifiers likewise
/// come from prose. Only the SYMBOL operator — which prose names as a good form, so prose cannot supply
/// it (e.g. eqeqeq names `===`, never `==`) — is read from the docs' own bad/good here.
fn operator_tokens(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in code.split_whitespace() {
        let t = raw.trim_matches(|c: char| c == '(' || c == ')' || c == ';' || c == '{' || c == '}' || c == ',');
        let is_operator = t.len() >= 2
            && t.chars().all(|c| !c.is_ascii_alphanumeric() && c != '"' && c != '\'' && c != '`' && c != '_');
        if is_operator && !out.contains(&t.to_string()) {
            out.push(t.to_string());
        }
    }
    out
}

/// READ one language-doc page into the workflow's PROPOSE material. Returns a [`DocPage`] whose
/// `constructs` are the UNCONFIRMED candidate construct(s) the page names as prohibited, DATA-read two
/// covenant-clean ways, plus the page's own bad/good example blocks the CALLER verifies each candidate
/// against (with the frozen firing, since only the caller knows the language):
/// - **Rule page:** candidates = the prose-named constructs ([`Bridge::constructs_named`], catching the
///   keyword/identifier `var`/`with`/`eval`) ∪ the multi-char OPERATOR tokens of the incorrect examples
///   (catching the symbol `==`/`!=` that prose names only as its good form). `incorrect`/`correct` carry
///   the page's own `class="incorrect"`/`class="correct"` example blocks.
/// - **Deprecated reference page:** candidate = the construct its definition sentence names (the page's
///   subject), prohibited by the deprecation notecard; no paired examples.
pub fn read_doc_page(url: &str, body: &str, _en: &English, bridge: &Bridge) -> DocPage {
    let reference = is_reference_page(url);
    let rule = is_rule_page(url);
    // Only a PROHIBITION page contributes — a linter rule page, or a reference page with a deprecation
    // notecard. Every check here is cheap (URL path + one substring scan of the body), so the ~thousands
    // of ordinary reference/guide pages are skipped BEFORE the expensive governing-prose extraction, and
    // the pool stays the clean per-construct prohibition reading. This keeps training in seconds.
    let empty = DocPage {
        url: url.to_string(),
        prohibited: false,
        attested_deprecated: false,
        governing: Vec::new(),
        constructs: Vec::new(),
        incorrect: Vec::new(),
        correct: Vec::new(),
    };
    let attested_deprecated = !rule && reference && has_deprecation_notecard(body);
    if !rule && !attested_deprecated {
        return empty;
    }
    let prohibited = true;
    let governing = governing_sentences(body);

    let mut constructs: Vec<String> = Vec::new();
    let push = |c: String, out: &mut Vec<String>| {
        let c = normalize_construct(&c);
        if c.len() >= 2 && !out.contains(&c) {
            out.push(c);
        }
    };

    let (mut incorrect, mut correct) = (Vec::new(), Vec::new());
    if rule {
        // Examples are read from the RAW markup (only script/style dropped) — NOT `code_to_backtick`,
        // which wraps each example's `<code>` in one backtick pair and makes the whole block parse as a
        // single template literal (the construct node vanishes → the docs'-own-example verification fires
        // on nothing). `examples_of_class` pulls the clean `<code>` interior.
        let raw = drop_script_style(body);
        incorrect = examples_of_class(&raw, "incorrect");
        correct = examples_of_class(&raw, "correct");
        // Prose-named constructs (reads their MEANING — catches a keyword/identifier the token diff
        // cannot: `var`, `with`, `eval`). Only the first few backtick-bearing sentences are read (the
        // lead summary + rule-details name the subject there), which bounds the expensive alignment.
        for s in governing.iter().filter(|s| s.contains('`')).take(PROSE_SENTENCE_LIMIT) {
            for (c, _) in bridge.constructs_named(s) {
                push(c, &mut constructs);
            }
        }
        // Multi-char OPERATOR tokens of the incorrect examples — the docs' own bad/good, which is the
        // ONLY place the banned SYMBOL appears (prose names the good form: eqeqeq names `===`, never
        // `==`). Keywords are deliberately NOT harvested here (the incidental-keyword junk class).
        for block in &incorrect {
            for t in operator_tokens(block) {
                push(t, &mut constructs);
            }
        }
    } else {
        // Deprecated reference page: the SUBJECT is the page's own URL last segment — MDN names the
        // element/property in its path (`/Element/marquee`, `/Properties/box-orient`). This is far more
        // reliable than the definition prose (whose first backticked token is often a SIBLING the
        // deprecation banner mentions — `css`/`color`/`src`, MEASURED) and covenant-clean: the URL is
        // DATA, a per-SOURCE structural marker exactly like the page-kind keying. The caller's URL-payload
        // subject gate then trivially confirms it, and the deprecation notecard is the prohibition proof.
        let seg = url.trim_end_matches('/').rsplit('/').next().unwrap_or("");
        push(seg.to_string(), &mut constructs);
    }

    DocPage { url: url.to_string(), prohibited, attested_deprecated, governing, constructs, incorrect, correct }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests exercise the STRUCTURAL reading with ZERO embedded language knowledge: synthetic pages whose
    // constructs are opaque (`qux`, an operator `~~`), a synthetic rule/reference URL shape, and
    // synthetic prose. What is under test is page-role keying, furniture/example stripping, code-typography
    // preservation, and candidate extraction — never any fact about a real language. The bad/good FIRING
    // confirmation lives in `lint_module` (it needs the language) and is tested there.

    #[test]
    fn operator_tokens_keeps_operators_drops_punctuation_and_keywords() {
        let toks = operator_tokens("if foo ~~ bar { baz }");
        assert!(toks.contains(&"~~".to_string()), "multi-char operator kept");
        assert!(!toks.iter().any(|t| t == "{" || t == "}"), "single-char punctuation dropped (the brace-junk class)");
        assert!(!toks.iter().any(|t| t == "if"), "a keyword is NOT harvested from examples (prose names it)");
    }

    #[test]
    fn normalize_strips_trailing_empty_call() {
        assert_eq!(normalize_construct("`qux()`"), "qux");
        assert_eq!(normalize_construct("~~"), "~~");
    }

    #[test]
    fn a_rule_page_reads_operator_candidates_and_its_own_bad_good_examples() {
        let (Some(br), Some(en)) = (crate::lint_char::brain(), crate::lint_english::brain()) else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let bridge = Bridge::new(br.meanings(), en);
        // A rule page: an incorrect example uses the operator `~~`, the correct one the remedy `~~~`.
        // The reader proposes the operator candidate `~~` and carries the page's own bad/good blocks; the
        // firing confirmation that keeps `~~` and drops the remedy `~~~` is `lint_module`'s job.
        let body = r#"<html><body>
            <h1>no-loose</h1>
            <p>This rule disallows the loose <code>~~</code> operator.</p>
            <p>Examples of <strong>incorrect</strong> code for this rule:</p>
            <div class="incorrect"><pre class="language-js"><code>a ~~ b</code></pre></div>
            <p>Examples of <strong>correct</strong> code for this rule:</p>
            <div class="correct"><pre class="language-js"><code>a ~~~ b</code></pre></div>
            </body></html>"#;
        let url = "https://example.org/docs/latest/rules/no-loose";
        let page = read_doc_page(url, body, en, &bridge);
        assert!(page.prohibited, "a /rules/ page is a prohibition by its role");
        assert!(page.constructs.contains(&"~~".to_string()), "the operator candidate is read: {:?}", page.constructs);
        assert!(page.incorrect.iter().any(|b| b.contains("~~")), "the page's own bad example is captured");
        assert!(page.correct.iter().any(|b| b.contains("~~~")), "the page's own good example is captured");
    }

    #[test]
    fn a_plain_reference_page_without_deprecation_proposes_nothing() {
        let (Some(br), Some(en)) = (crate::lint_char::brain(), crate::lint_english::brain()) else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let bridge = Bridge::new(br.meanings(), en);
        let body = r#"<html><body><h1>qux</h1>
            <p>The <code>qux</code> operator combines two values into one clearly.</p></body></html>"#;
        let page = read_doc_page("https://example.org/reference/operators/qux", body, en, &bridge);
        assert!(!page.prohibited, "a reference page with no deprecation notecard is not a prohibition");
        assert!(page.constructs.is_empty(), "no construct proposed from a non-prohibition page");
    }
}
