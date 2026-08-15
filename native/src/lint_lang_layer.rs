//! `lint_lang_layer` — the LANGUAGE-DOC reading rung: clean, per-construct governing prose and the
//! docs' own paired bad/good examples, read STRUCTURALLY from real reference/rule pages.
//!
//! Contract: `native/history.dx` → "The language-doc reading rung — structural per-construct governing prose".
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
    /// The page's own worked-example code blocks (`<pre><code>`) — the grammar-referee corpus the
    /// caller uses to SELECT which of a member page's candidate construct SHAPES (qualified
    /// `String.substr`, member `.substr`, bare `substr`) genuinely fires on how the docs use the
    /// subject. Populated for a deprecated reference page (a rule page uses `incorrect`/`correct`).
    pub example_code: Vec<String>,
    /// The construct shapes this page marks deprecated IN PLACE, via a RENDERED marker on the item's own
    /// dotted anchor ([`attested_item_shapes`]) — ONE GROUP PER MARKED ITEM, each group most-specific
    /// first. The rendered-route parallel of the URL naming the subject — except an API page carries MANY
    /// marked items (ssl.html marks 20), so the graduation unit is the ITEM, not the page. A page (MDN)
    /// that names its subject in the URL and marks deprecation with a dotless-id banner yields NOTHING
    /// here, so this is the covenant-clean STRUCTURAL confirmation the caller uses to admit Python/Rust
    /// member subjects the URL cannot spell, without perturbing the URL-subject sites.
    pub marked_deprecated: Vec<Vec<String>>,
    /// The marked item groups whose OWN note COUNTER-ATTESTS them (PASS 28 — an exception clause naming
    /// the item, or a first-sentence usage-form deprecation). Kept SEPARATE from
    /// [`marked_deprecated`](Self::marked_deprecated) rather than dropped: a counter-attested item is
    /// still demonstrated python/rust on its page, so it remains a LANGUAGE WITNESS
    /// ([`crate::lint_module::page_proves_in_lang`]) and a plain (non-revoked) web read node — it only
    /// leaves the ENFORCEMENT view (no proposal, no graded form).
    pub counter_attested: Vec<Vec<String>>,
    /// Whether this page was made a prohibition by BINDING a PROVEN CONSTRUCTION on its own prose (PASS
    /// 23, [`crate::lint_construct::attested_subjects`]): its `constructs` are the construction's slot
    /// subjects (firing-form module names), and the construction's PROOF — not the URL payload or a
    /// per-page grammar demonstration — established both the language and the (removal-strength)
    /// prohibition. The workflow lets such a subject bypass the URL-payload / lead gates and proves it
    /// through the unchanged blind loop. `false` for every page reached the ordinary rule/notecard way.
    pub construction_attested: bool,
}

/// The candidate construct SHAPES a deprecated REFERENCE page proposes for its subject, most-specific
/// first — so the caller keeps the first that FIRES on the page's own example code under the language's
/// grammar (the covenant-clean squeeze; native/history.dx → "QUALIFIED-MEMBER construct extraction"). The
/// subject is the URL's last path segment; whether it is a MEMBER is read from the URL's shape under the
/// reference marker: a member sits under an OWNER segment (`…/Reference/Global_Objects/String/substr` —
/// owner `String`, subject `substr`), a global/keyword sits directly under its category
/// (`…/Reference/Global_Objects/escape`, `…/Reference/Statements/with`). For a member subject the shapes
/// are the RECEIVER-SPECIFIC `Owner.subject` (a static like `RegExp.input`), the RECEIVER-GENERIC member
/// `.subject` (a prototype method like `.substr`), then bare `subject`; for a non-member only bare. This
/// is a per-SOURCE structural marker (the `/reference/` path depth), INTERIM like the page-kind keying,
/// and it names no language: a NON-member language (CSS `@media/-moz-device-pixel-ratio`) simply fails
/// to fire the dotted shapes under its grammar and the caller falls to bare.
pub(crate) fn member_page_shapes(url: &str) -> Vec<String> {
    let lower = url.to_lowercase();
    let after = lower.find("/reference/").map(|i| &url[i + "/reference/".len()..]);
    let segs: Vec<&str> = after
        .unwrap_or("")
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let bare = segs.last().copied().unwrap_or_else(|| url.trim_end_matches('/').rsplit('/').next().unwrap_or(""));
    // A member subject has an OWNER segment between its category and itself (depth ≥ 3 under the
    // reference marker: category / owner / subject). A category-level subject (depth ≤ 2) is a global.
    if segs.len() >= 3 {
        let owner = segs[segs.len() - 2];
        vec![format!("{owner}.{bare}"), format!(".{bare}"), bare.to_string()]
    } else {
        vec![bare.to_string()]
    }
}

/// The QUALIFIED-RECEIVER shapes a deprecated page proposes for its subject, read from the page's OWN
/// example code — for a page WITHOUT the `/reference/` marker whose subject is a member call
/// (`/Web/API/Document/write`: subject `write`, owner segment `Document`). A bare `write` over-fires on
/// every `write`, and the receiver-generic `.write` over-fires on every `.write()`; the clean construct is
/// the qualified `document.write`. We get it covenant-cleanly by linking the URL's OWNER path segment to
/// the example's ACTUAL receiver: an `IDENT.subject` member access whose `IDENT` equals the owner segment
/// case-insensitively (`document` for interface `Document`) yields the actual-case `document.write`. DATA
/// read from the example, owner-to-receiver by identity — no language named, no case convention hardcoded.
fn example_receiver_shapes(url: &str, example_code: &[String]) -> Vec<String> {
    let path = url.split("://").nth(1).unwrap_or(url).trim_end_matches('/');
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segs.len() < 2 {
        return Vec::new();
    }
    let (owner, subject) = (segs[segs.len() - 2], segs[segs.len() - 1]);
    if owner.is_empty() || subject.is_empty() {
        return Vec::new();
    }
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    let mut out: Vec<String> = Vec::new();
    for blk in example_code {
        let bytes = blk.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            // Start of an identifier run (not preceded by an ident char).
            if is_ident(bytes[i] as char) && (i == 0 || !is_ident(bytes[i - 1] as char)) {
                let start = i;
                while i < bytes.len() && is_ident(bytes[i] as char) {
                    i += 1;
                }
                // `IDENT.subject` with IDENT == owner (ci) and subject a whole terminal property.
                let recv = &blk[start..i];
                if recv.eq_ignore_ascii_case(owner)
                    && blk[i..].starts_with('.')
                    && blk[i + 1..].starts_with(subject)
                {
                    let after = i + 1 + subject.len();
                    let bounded = blk[after..].chars().next().map(|c| !is_ident(c)).unwrap_or(true);
                    if bounded {
                        let shape = format!("{recv}.{subject}");
                        if !out.contains(&shape) {
                            out.push(shape);
                        }
                    }
                }
                continue;
            }
            i += 1;
        }
    }
    out
}

/// The construct shapes an ATTESTED page proposes for its OWN deprecated items, read STRUCTURALLY from the
/// page's item ANCHORS — the RENDERED-MARKER parallel of the URL-segment subject, for sites (Python's
/// stdlib library, Rust's std API) that render the deprecation marker directly on each item instead of one
/// item per page. For every element whose class carries a prohibition status token
/// (`deprecation-status.json` — the same data the attester joins), the nearest PRECEDING id-valued anchor
/// that is a DOTTED qualified name (`ssl.PROTOCOL_TLS`, rustdoc `method.description`) names the deprecated
/// item; its shapes are proposed most-specific-first — the full qualified id, the receiver-generic `.last`,
/// then the bare last component — and the caller's grammar-refereed selection keeps whichever fires on the
/// page's own example code. A DOTLESS id (an MDN section slug like `browser_compatibility`) is not an item
/// anchor, so the URL-subject sites are never perturbed by this reader. No site, class, or language named:
/// the marker token is data, the dotted-anchor shape is structure.
pub(crate) fn attested_item_shapes(body: &str) -> (Vec<Vec<String>>, Vec<Vec<String>>) {
    let tokens = crate::lint_attest::prohibition_class_tokens();
    if tokens.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let dotted_item = |v: &str| -> bool {
        v.len() >= 3
            && v.contains('.')
            && !v.starts_with('.')
            && !v.ends_with('.')
            && !v.contains("..")
            && v.chars().all(|c| is_ident(c) || c == '.')
            && v.rsplit('.').next().map(|last| last.len() >= 2).unwrap_or(false)
    };
    let attr = |tag: &str, name: &str| -> Option<String> {
        let key = format!("{name}=");
        let i = tag.find(&key)?;
        let rest = &tag[i + key.len()..];
        let q = rest.chars().next()?;
        if q != '"' && q != '\'' {
            return None;
        }
        let rest = &rest[1..];
        let end = rest.find(q)?;
        Some(rest[..end].to_string())
    };
    // Collect the byte position of every dotted item anchor, every deprecation-marker element (carrying
    // the marker element's own id when it has one), and every prose-text position, then attribute each
    // marker by the marker element's TYPOGRAPHY (all three shapes measured on real corpora):
    // - SELF-ANCHORED: the marker element carries the item's own dotted id (rustdoc's
    //   `<section class="method deprecated" id="method.only_v6">`) — the id IS the item.
    // - CONTAINER OPENING: an anchor follows the marker with ONLY MARKUP (no prose text) in between — the
    //   item lives inside the marked element (rustdoc's `<details class="…deprecated">` opens just before
    //   its item's own `<section id=…>`) — attribute FORWARD to that anchor.
    // - TRAILING BADGE: anything else sits inside its item's region (Python's `<div class="deprecated">`
    //   holding the "Deprecated since version…" sentence at the end of the item's `<dd>`; rustdoc's
    //   `stab deprecated` badge after the signature) — attribute BACKWARD to the nearest preceding anchor
    //   (region containment: an item's region runs from its anchor to the next).
    // Nearest-by-byte-distance is NOT sound here (MEASURED): Python's trailing badge is byte-nearer to the
    // NEXT item's anchor than to its own (the description prose sits between), which mis-keyed the
    // deprecations of `utcnow`/`utcfromtimestamp` onto their neighbors `fromtimestamp`/`fromordinal`.
    // EVERY id attribute delimits a region (not only the dotted ones): a marker whose region id is NOT a
    // valid dotted item attests NOTHING rather than sliding past it to an unrelated anchor — the honest
    // abstention for rustdoc's `-1`-suffixed trait-impl duplicates (`method.is_ascii-1`, whose names
    // collide with non-deprecated inherent methods and are unenforceable at token level, MEASURED junk).
    // A marker with no id within [`MARKER_ANCHOR_WINDOW`] is page chrome (a nav badge) and is ignored.
    let mut ids: Vec<(usize, String)> = Vec::new();
    let mut markers: Vec<(usize, Option<String>)> = Vec::new();
    let mut text_positions: Vec<usize> = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let Some(rel) = body[i..].find('>') else { break };
            let tag = &body[i..i + rel + 1];
            let own_id = attr(tag, "id");
            if let Some(id) = &own_id {
                ids.push((i, id.clone()));
            }
            if let Some(class) = attr(tag, "class") {
                if class.split_whitespace().any(|t| tokens.iter().any(|m| m.eq_ignore_ascii_case(t))) {
                    markers.push((i, own_id));
                }
            }
            i += rel + 1;
        } else {
            if !(bytes[i] as char).is_whitespace() {
                text_positions.push(i);
            }
            i += 1;
        }
    }
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut counter: Vec<Vec<String>> = Vec::new();
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (mpos, own_id) in &markers {
        let mpos = *mpos;
        let next = ids.iter().find(|(apos, _)| *apos > mpos);
        // `text_positions` is sorted (built by a forward scan): prose intervenes iff the first text
        // position after the marker falls before the anchor.
        let prose_intervenes = |apos: usize| {
            let k = text_positions.partition_point(|&t| t <= mpos);
            text_positions.get(k).is_some_and(|&t| t < apos)
        };
        let attributed = match (own_id, next) {
            // Self-anchored: the marker element names its own item.
            (Some(id), _) => Some(id),
            // Container opening: an id follows with no prose in between → the marker wraps this item.
            (None, Some((apos, id))) if apos - mpos <= MARKER_ANCHOR_WINDOW && !prose_intervenes(*apos) => {
                Some(id)
            }
            // Trailing badge: the marker sits inside the region of the nearest PRECEDING id.
            _ => ids
                .iter()
                .rev()
                .find(|(apos, _)| *apos < mpos && mpos - apos <= MARKER_ANCHOR_WINDOW)
                .map(|(_, id)| id),
        };
        // Only a valid dotted ITEM id names a construct; any other region id (an MDN section slug, a
        // rustdoc `-1` duplicate) attests nothing for this marker. The proposed shapes are the DOTTED
        // suffixes (receiver-specific `owner.member` chains) plus the receiver-generic member `.member` —
        // NEVER the bare name: the author's own anchor is qualified, and the bare form fires on every
        // unrelated identifier of that name (MEASURED: bare `split` from `re.split` flags every
        // `s.split()` — the exact over-fire the MDN qualified-member work already killed).
        if let Some(id) = attributed.filter(|id| dotted_item(id)) {
            if !seen_ids.insert(id.clone()) {
                continue; // an item marked twice (div + span) is one item
            }
            let parts: Vec<&str> = id.split('.').collect();
            let mut group: Vec<String> = Vec::new();
            for start in 0..parts.len() - 1 {
                group.push(parts[start..].join("."));
            }
            group.push(format!(".{}", parts[parts.len() - 1]));
            // PASS 28 — the NOTE-SCOPE read: the note's OWN sentence can exclude its attributed item
            // (an exception clause) or deprecate only a usage form of it (a first-sentence conditional/
            // gerund register). A counter-attested item leaves the ENFORCEMENT view but stays a
            // structural read — it remains a language witness and a (non-revoked) web read node.
            if note_counter_attests(&marker_note_text(body, mpos), id) {
                counter.push(group);
            } else {
                groups.push(group);
            }
        }
    }
    (groups, counter)
}

/// The largest byte distance between a deprecation marker and the item anchor it may belong to. A trailing
/// badge sits at the END of its item's description (Python's `<dd>` prose runs long), so the window spans a
/// long description; a marker farther than this from any dotted anchor is page chrome (a nav badge, a
/// changelog mention), not an item badge — so it attests nothing.
const MARKER_ANCHOR_WINDOW: usize = 20_000;

/// The most note prose read for the scope registers — a rendered badge is a short paragraph; the cap
/// only bounds a pathological unclosed region.
const NOTE_TEXT_CAP: usize = 600;

/// The deprecation note's OWN prose (PASS 28): the whitespace-collapsed text after the marker tag at
/// `mpos`, stopped at the next id-bearing tag (the following item's region) or [`NOTE_TEXT_CAP`] chars —
/// the sentence the author rendered inside the badge, lower-cased for the register reads.
fn marker_note_text(body: &str, mpos: usize) -> String {
    let has_id = |tag: &str| {
        tag.find("id=").is_some_and(|i| matches!(tag.as_bytes().get(i + 3), Some(b'"') | Some(b'\'')))
    };
    let mut out = String::new();
    let mut i = mpos;
    while i < body.len() && out.len() < NOTE_TEXT_CAP {
        if body.as_bytes()[i] == b'<' {
            let Some(rel) = body[i..].find('>') else { break };
            if i > mpos && has_id(&body[i..i + rel + 1]) {
                break;
            }
            i += rel + 1;
            continue;
        }
        let c = body[i..].chars().next().unwrap();
        if c.is_whitespace() {
            if !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
        } else {
            out.extend(c.to_lowercase());
        }
        i += c.len_utf8();
    }
    out
}

/// The NOTE-SCOPE registers (PASS 28, meaning-anchored in PASS 29): whether a deprecation note's own
/// prose COUNTER-ATTESTS its attributed `item` (the marker then attests nothing). The anchors are the
/// ONLY hand data (`deprecation-status.json` — `scope_exception: ["except"]`, `usage_form: ["if"]`);
/// the MEANING NET expands each anchor to every word the dictionary DEFINES VIA it (owner ruling
/// 2026-07-14: registers are meaning, tokens are shims — "unless"/"excluding"/"barring" carry the
/// exception meaning because their own definitions say so, with a function-word POS gate read from the
/// dictionary entry itself so a content word never rides a stray definition mention). `note` is
/// lower-case by contract ([`marker_note_text`]).
/// - EXCEPTION SCOPE: the note names the item AFTER an exception-meaning word — the sentence excludes
///   it ("All TLSVersion members except TLSVersion.TLSv1_3 are deprecated").
/// - CONDITIONAL FORM: the FIRST SENTENCE of the clause after the deprecation head carries a
///   conditional-meaning word — the deprecation is conditional, not the item's ("Deprecation warning is
///   emitted if loop …"). First-sentence-only is load-bearing: later sentences are remedy prose ("Use
///   isinstance(…) to test if …") and must not cut a true deprecation.
/// - USAGE SUBJECT: the first sentence's FIRST word is a gerund of a dictionary VERB — the sentence's
///   subject is an ACTION on the item, not the item ("Passing …", "Setting …", "Accepting … is
///   deprecated"). Verb-hood is the dictionary's own POS word; no gerund list exists anywhere.
/// Without a brain on disk only the literal anchors match (honest narrow fallback); training always
/// runs with the brain, so the learned view is the enforced one.
pub fn note_counter_attests(note: &str, item: &str) -> bool {
    if note.is_empty() {
        return false;
    }
    let net = crate::lint_char::brain().map(|b| b.meanings());
    let item = item.to_lowercase();
    let last = item.rsplit('.').next().unwrap_or(&item);
    let parts: Vec<&str> = item.split('.').collect();
    let two =
        if parts.len() >= 2 { parts[parts.len() - 2..].join(".") } else { item.clone() };
    let exception = crate::lint_attest::scope_exception_tokens();
    for (pos, w) in note_words(note) {
        if word_carries_anchor(w, &exception, net) {
            let after = &note[pos + w.len()..];
            if after.contains(&two) || after.contains(&format!(".{last}")) {
                return true;
            }
        }
    }
    let clause = note.split_once(':').map(|(_, c)| c).unwrap_or(note);
    let first_sentence = clause.find(". ").map(|p| &clause[..p + 1]).unwrap_or(clause);
    let conditional = crate::lint_attest::usage_form_tokens();
    let words = note_words(first_sentence);
    if let Some((_, w0)) = words.first() {
        if gerund_of_known_verb(w0, net) {
            return true;
        }
    }
    words.iter().any(|(_, w)| word_carries_anchor(w, &conditional, net))
}

/// The word tokens of a lower-case note with their byte positions — identifier-ish runs, so `if` never
/// matches inside `shift` and a dotted mention stays one probe target per component.
fn note_words(note: &str) -> Vec<(usize, &str)> {
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in note.char_indices() {
        match (is_word(c), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((s, &note[s..i]));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, &note[s..]));
    }
    out
}

/// Whether `word` carries an anchor's MEANING: it IS an anchor, or the dictionary DEFINES it via one —
/// its definition words contain an anchor AND its entry opens as a function word (conjunction /
/// preposition, the dictionary's own POS typography), so "unless" (conjunction: "except if …") and
/// "excluding" (preposition: "… apart from except") join the register while a content word whose long
/// definition merely mentions the anchor does not. Anchor equality needs no brain.
fn word_carries_anchor(
    word: &str,
    anchors: &[String],
    net: Option<&crate::lint_char::MeaningNetwork>,
) -> bool {
    if anchors.iter().any(|a| a == word) {
        return true;
    }
    let Some(net) = net else { return false };
    let Some(defs) = net.definition_words(word) else { return false };
    defs.iter().take(3).any(|d| d == "conjunction" || d == "preposition")
        && defs.iter().any(|d| anchors.iter().any(|a| a == d))
}

/// Whether `word` is the GERUND of a dictionary VERB — morphological `-ing` plus the stem's own entry
/// opening with the dictionary's `verb` POS word (candidates: drop `ing`, un-double the final
/// consonant for `setting`→`set`, restore the silent `e` for `encoding`→`encode`). This is the
/// usage-subject register with zero vocabulary: the dictionary's verb knowledge is the whole test.
fn gerund_of_known_verb(word: &str, net: Option<&crate::lint_char::MeaningNetwork>) -> bool {
    let Some(net) = net else { return false };
    let Some(base) = word.strip_suffix("ing").filter(|b| b.len() >= 2) else { return false };
    let mut candidates: Vec<String> = vec![base.to_string(), format!("{base}e")];
    let b = base.as_bytes();
    if b.len() >= 2 && b[b.len() - 1] == b[b.len() - 2] {
        candidates.push(base[..base.len() - 1].to_string());
    }
    candidates.iter().any(|c| {
        net.definition_words(c).is_some_and(|defs| defs.iter().take(8).any(|d| d == "verb"))
    })
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
pub(crate) fn governing_sentences(body: &str) -> Vec<String> {
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
/// as each `<pre>` example's interior ([`code_interiors`], the nested `<code>` preferred) — so Prism's
/// line-number gutter (`<span class="line-numbers-rows">`, inside the `<pre>` but AFTER `</code>`) is
/// excluded, not welded on.
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

/// The JavaScript interior of a code block that IS a single `<script>…</script>` element — the one way
/// an HTML page embeds JS (MDN demonstrates `document.write` as `<script>document.write(…)</script>`).
/// `None` when the block is not a lone script element (ordinary code, or HTML with other content), so an
/// HTML example is never stripped of its markup. Structural: keys on the `<script>` element, names no
/// language. The interior arrives already entity-decoded (`strip_code`), so `<script>` is a real tag here.
fn script_interior(code: &str) -> Option<String> {
    let t = code.trim();
    if !starts_with_ci(t, "<script") || !t.to_ascii_lowercase().ends_with("</script>") {
        return None;
    }
    let open_end = t.find('>')?;
    let inner = &t[open_end + 1..t.len() - "</script>".len()];
    // Only a NON-EMPTY interior is JS to surface (a `<script src=…></script>` reference has none, and
    // there must be no nested `<script>` — a lone element only).
    let inner = inner.trim();
    if inner.is_empty() || find_ci(inner, "<script").is_some() {
        return None;
    }
    Some(inner.to_string())
}

/// The `strip_code`-decoded INTERIOR of every `<pre>` example block in a markup region — the clean
/// example code with line structure intact and highlight `<span>`s removed. A nested `<code>` INSIDE
/// the `<pre>` is PREFERRED (the rustdoc/MDN/ESLint worked-example shape — taking only the `<code>`
/// interior leaves a sibling line-number gutter behind); a bare `<pre>` with no `<code>` child is the
/// example ITSELF (many real sites and the fixture pages author examples that way — PASS 36: reading
/// only `<pre><code>` left those pages with an EMPTY own-example corpus, blinding the grammar
/// partition and blanket-minting junk "no example spells the subject" rows). This deliberately skips
/// INLINE prose `<code>` (a `` `--fix` ``/`` `null` ``/`` `===` `` mentioned in the option text after
/// the example), which is not an example block and would pollute the per-example firing verification.
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
        let block = if let Some(crel) = find_ci(&html[pre_open..pre_end], "<code") {
            let copen = pre_open + crel;
            html[copen..].find('>').and_then(|gt| {
                let inner_start = copen + gt + 1;
                find_ci(&html[inner_start..pre_end], "</code>")
                    .map(|cend| crate::doc_crawler::strip_code(&html[inner_start..inner_start + cend]))
            })
        } else {
            // Bare `<pre>` — no `<code>` child anywhere inside: the `<pre>` interior IS the example.
            html[pre_open..pre_end]
                .find('>')
                .map(|gt| crate::doc_crawler::strip_code(&html[pre_open + gt + 1..pre_end]))
                .filter(|b| !b.trim().is_empty())
        };
        if let Some(block) = block {
            // A `<script>` ELEMENT INTERIOR is JavaScript — the one way an HTML page embeds JS.
            // An example that IS a lone script element (MDN's `<script>document.write(…)</script>`
            // demo) is surfaced as its JS interior so the JS grammar can parse+fire it; keying on
            // the `<script>` element is web-platform structure the reader understands, not a
            // language name. A non-script block passes through unchanged.
            out.push(script_interior(&block).unwrap_or(block));
        }
        at = pre_end + "</pre>".len();
    }
    out
}

/// Every `<code>…</code>` interior on a page (nested highlighter tags stripped), INCLUDING the bare inline
/// `<code>` a `<pre>` does not wrap. This complements [`code_interiors`] (which reads only `<pre>`
/// example blocks) for RENDERED-MARKER sites whose demonstrated
/// usage lives elsewhere: Python renders worked examples as `<pre>` DOCTESTS (`>>>`-prefixed, which do not
/// parse cleanly) and names each item as bare inline `<code>datetime.utcfromtimestamp</code>` — so its
/// clean-parsing usages are exactly the bare `<code>` refs this reads. Used only to ENRICH the example
/// corpus of a page that carries in-place item-anchor markers ([`DocPage::marked_deprecated`]); a
/// URL-subject site (MDN) has no such markers, so its corpus is never widened by this.
fn bare_code_interiors(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while let Some(rel) = find_ci(&html[at..], "<code") {
        let copen = at + rel;
        let Some(gt) = html[copen..].find('>') else { break };
        let inner = copen + gt + 1;
        let Some(cend) = find_ci(&html[inner..], "</code>") else { break };
        let block = crate::doc_crawler::strip_code(&html[inner..inner + cend]);
        let trimmed = block.trim();
        if trimmed.len() >= 2 {
            out.push(trimmed.to_string());
        }
        at = inner + cend + "</code>".len();
    }
    out
}

/// The example-code corpus a page's own worked usage is read from — [`code_interiors`] (the `<pre><code>`
/// example shape) for every page, WIDENED with [`bare_code_interiors`] when the page carries in-place
/// rendered item-anchor markers (`marked` non-empty), so a Python/Rust item whose only clean usage is a
/// bare inline `<code>` reference is visible to the grammar referee. Structural gate, no site named.
pub(crate) fn page_example_corpus(body: &str, rendered_marker: bool) -> Vec<String> {
    let mut out = code_interiors(body);
    if rendered_marker {
        let mut seen: std::collections::HashSet<String> = out.iter().cloned().collect();
        for blk in bare_code_interiors(body) {
            if seen.insert(blk.clone()) {
                out.push(blk);
            }
        }
    }
    out
}

/// The `<pre><code>` worked-example code of `pages` — every example block ([`code_interiors`], the same
/// extractor the reader uses), deduped and bounded. NO URL/language attribution is applied (owner
/// directive 2026-07-12: language emerges from understanding/verification, never from the URL): the caller
/// decides scope. Two callers, both grammar-refereed downstream so an off-language block is harmless here:
/// (a) [`crate::lint_module::page_proves_in_lang`] passes ONE page and checks whether its subject fires on
/// this code under `lang`'s grammar — the verification-decided partition; (b)
/// [`crate::lint_module::graduated_rules`] passes the WHOLE-SITE corpus as the OFFLINE-ROBUSTNESS harvest
/// (native/history.dx → "the recommended unlock") when this machine's read [`Memory`] is too sparse to reach the
/// rep floor — a block that does not contain a candidate's construct simply never fires for it. Never
/// fetches; `lang` is unused (grammar judging happens in the caller).
pub(crate) fn page_code_corpus(pages: &[(String, String)], _lang: &str, cap: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (_url, body) in pages {
        for block in code_interiors(body) {
            let trimmed = block.trim();
            if trimmed.len() >= 3 && seen.insert(trimmed.to_string()) {
                out.push(trimmed.to_string());
                if out.len() >= cap {
                    return out;
                }
            }
        }
    }
    out
}

/// The `<pre><code>` worked-example blocks of `pages` WITH their source-page url attached (PASS 27) —
/// `(url, block)` pairs, deduped by block text and bounded. The graded tier's usage-death and clean-near-
/// miss gates ([`crate::lint_module::graded_forms`]) need to EXCLUDE a construct's OWN page when measuring
/// whether its member is dead in the corpus's OTHER current example code (the PASS-26 measurement), so the
/// url must ride each block — unlike [`page_code_corpus`], which drops it. Never fetches.
pub(crate) fn page_code_blocks_by_url(pages: &[(String, String)], cap: usize) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (url, body) in pages {
        for block in code_interiors(body) {
            let trimmed = block.trim();
            if trimmed.len() >= 3 && seen.insert(trimmed.to_string()) {
                out.push((url.clone(), trimmed.to_string()));
                if out.len() >= cap {
                    return out;
                }
            }
        }
    }
    out
}

/// Normalize an extracted construct token to its firing form: strip a trailing empty-call `()` (prose
/// names a callable `eval()` but the AST node in `eval(userInput)` is the identifier `eval`), and trim
/// surrounding punctuation. Symbol constructs (`==`) pass through unchanged.
pub(crate) fn normalize_construct(raw: &str) -> String {
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
pub fn read_doc_page(
    url: &str,
    body: &str,
    _en: &English,
    bridge: &Bridge,
    attested: &std::collections::HashSet<String>,
    construction: &std::collections::HashMap<String, Vec<String>>,
) -> DocPage {
    let reference = is_reference_page(url);
    let rule = is_rule_page(url);
    // Only a PROHIBITION page contributes — a linter rule page, or a reference page the LEARNED attester
    // ([`crate::lint_attest::Attestation`]) marks deprecated by the author's own metadata typography. Every
    // check here is cheap (URL path + a run-membership test), so the ~thousands of ordinary reference/guide
    // pages are skipped BEFORE the expensive governing-prose extraction, and the pool stays the clean
    // per-construct prohibition reading. This keeps training in seconds.
    let empty = DocPage {
        url: url.to_string(),
        prohibited: false,
        attested_deprecated: false,
        governing: Vec::new(),
        constructs: Vec::new(),
        incorrect: Vec::new(),
        correct: Vec::new(),
        example_code: Vec::new(),
        marked_deprecated: Vec::new(),
        counter_attested: Vec::new(),
        construction_attested: false,
    };
    // A deprecation NOTECARD makes a page a prohibition regardless of the `/reference/` URL marker: MDN
    // renders the same notecard on a `/Web/API/Document/write`-style API page that has no `/reference/`
    // segment. The notecard is a STATED STRUCTURAL FACT (markup), and the grammar-verification partition
    // ([`crate::lint_module::page_proves_in_lang`]) is the real guard against a wrong-language leak — so
    // dropping the URL-marker requirement cannot cross the partition ∅. Reference vs non-reference only
    // decides HOW the subject's construct SHAPE is derived below.
    //
    // A page can ALSO become a prohibition by BINDING A PROVEN CONSTRUCTION on its own prose (PASS 23):
    // the caller-supplied `construction` map carries, per url, the construction's slot subjects (already
    // firing-form). Such a page attests its subject deprecated exactly as a notecard does — the
    // construction's PROOF stands in for the notecard's structural fact — but its subject bypasses the
    // URL-payload / lead gates downstream (the construction proved the subject, not the URL shape).
    let existing_attested = !rule && attested.contains(url);
    let construction_subjects: Vec<String> = construction.get(url).cloned().unwrap_or_default();
    let construction_attested = !rule && !existing_attested && !construction_subjects.is_empty();
    let attested_deprecated = existing_attested || construction_attested;
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
    let mut example_code: Vec<String> = Vec::new();
    let mut marked_deprecated: Vec<Vec<String>> = Vec::new();
    let mut counter_attested: Vec<Vec<String>> = Vec::new();
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
    } else if construction_attested {
        // CONSTRUCTION-BOUND page (PASS 23): the subject(s) the proven construction named in this page's
        // own prose ARE the constructs — already firing-form module names (`cgi`, `telnetlib`). No
        // URL/item/member shape derivation and no example corpus: the construction's proof IS the subject
        // confirmation, so the caller admits the subject directly and proves it through the blind loop.
        for c in &construction_subjects {
            if !constructs.contains(c) {
                constructs.push(c.clone());
            }
        }
    } else {
        // Deprecated reference page: the SUBJECT is the page's own URL last segment — MDN names the
        // element/property in its path (`/Element/marquee`, `/Properties/box-orient`). This is far more
        // reliable than the definition prose (whose first backticked token is often a SIBLING the
        // deprecation banner mentions — `css`/`color`/`src`, MEASURED) and covenant-clean: the URL is
        // DATA, a per-SOURCE structural marker exactly like the page-kind keying. The caller's URL-payload
        // subject gate then trivially confirms it, and the deprecation notecard is the prohibition proof.
        //
        // For a MEMBER subject (a prototype method / static property) the bare segment would fire on every
        // ordinary identifier of that name (`const link = 1` fires `link`); so propose the QUALIFIED and
        // RECEIVER-GENERIC-MEMBER shapes too ([`member_page_shapes`], most-specific first) and let the
        // caller keep the first that fires on this page's own example code under the language's grammar.
        // These shapes are already firing-form, so they bypass `normalize_construct` (which would strip a
        // leading `.`). The page's example code is carried for that grammar-refereed selection.
        // RENDERED-MARKER sites (Python/Rust) mark each deprecated item in place with its own dotted
        // anchor, not one item per URL — so read the subjects from the page's item anchors, most-specific
        // first. Dotless-id (URL-subject) sites like MDN yield nothing here, so they are unperturbed.
        (marked_deprecated, counter_attested) = attested_item_shapes(body);
        // The example corpus is [`code_interiors`] for a URL-subject page, WIDENED with the bare inline
        // `<code>` refs for a rendered-marker page (Python names its items only in inline code, not
        // `<pre><code>`). MDN has no rendered markers, so its corpus is byte-identical. Whether a page IS
        // a rendered-marker page is structural, so counter-attested items still count for the widening.
        example_code =
            page_example_corpus(body, !marked_deprecated.is_empty() || !counter_attested.is_empty());
        // A `/reference/` page names its owner in the path (`…/String/substr`), so URL-derived shapes
        // suffice. A NON-reference notecard page (`/Web/API/Document/write`) names the owner as a plain
        // path segment whose CASE differs from the code receiver (`Document` vs `document`); its clean
        // qualified shape is read from the page's OWN example receiver ([`example_receiver_shapes`]),
        // prepended most-specific-first so the caller's grammar-refereed selection keeps `document.write`
        // over the over-firing bare `write`.
        for shape in marked_deprecated.iter().chain(counter_attested.iter()).flatten() {
            if !constructs.contains(shape) {
                constructs.push(shape.clone());
            }
        }
        if !reference {
            for shape in example_receiver_shapes(url, &example_code) {
                if !constructs.contains(&shape) {
                    constructs.push(shape);
                }
            }
        }
        for shape in member_page_shapes(url) {
            if shape.len() >= 2 && !constructs.contains(&shape) {
                constructs.push(shape);
            }
        }
    }

    DocPage { url: url.to_string(), prohibited, attested_deprecated, governing, constructs, incorrect, correct, example_code, marked_deprecated, counter_attested, construction_attested }
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
    fn a_bare_pre_block_is_read_as_the_pages_own_example() {
        // PASS 36 — F3: many real sites (and the census fixtures) author examples as a bare
        // `<pre>` with no nested `<code>`. Reading only `<pre><code>` left such pages with an
        // EMPTY own-example corpus, blinding the grammar partition and blanket-minting junk
        // "no example spells the subject" rows. The `<pre>` interior IS the example there;
        // a nested `<code>` interior stays PREFERRED when present (gutter siblings excluded).
        let body = r#"<h1>zap()</h1><p>Never use zap.</p>
            <pre>zap(payload)
zap(other)</pre>
            <pre><span class="gutter">1</span><code>preferred(interior)</code></pre>
            <pre>   </pre>"#;
        let own = page_example_corpus(body, false);
        assert_eq!(own.len(), 2, "bare pre + pre>code, whitespace-only pre skipped: {own:?}");
        assert!(own[0].contains("zap(payload)") && own[0].contains("zap(other)"), "the bare <pre> interior is the example unit: {:?}", own[0]);
        assert_eq!(own[1], "preferred(interior)", "a nested <code> stays preferred; the gutter sibling is left behind");
    }

    #[test]
    fn attested_item_shapes_reads_the_three_marker_typographies() {
        // TRAILING BADGE (Python): the deprecation div sits at the END of its item's <dd>, byte-nearer to
        // the NEXT item's anchor — region containment must key it to the PRECEDING anchor.
        let py = r#"<dl><dt id="mod.klass.olditem"></dt><dd><p>Long description prose here.</p>
            <div class="deprecated"><p>Deprecated since version 9.9: gone.</p></div></dd>
            <dt id="mod.klass.newitem"></dt><dd><p>The replacement.</p></dd></dl>"#;
        let (groups, _) = attested_item_shapes(py);
        assert_eq!(groups.len(), 1, "one marked item: {groups:?}");
        assert_eq!(groups[0], vec!["mod.klass.olditem", "klass.olditem", ".olditem"], "no bare shape");

        // CONTAINER OPENING (rustdoc details): marker element opens BEFORE the item's own id, only markup
        // between them — attributes FORWARD.
        let rs = r#"<details class="toggle method-toggle deprecated"><summary>
            <section id="method.olditem" class="method"><h4>fn olditem()</h4></section></summary></details>"#;
        let (groups, _) = attested_item_shapes(rs);
        assert_eq!(groups.len(), 1, "{groups:?}");
        assert_eq!(groups[0][0], "method.olditem");

        // SELF-ANCHORED (rustdoc section): id and marker class on the SAME element.
        let rs2 = r#"<section class="method deprecated" id="method.only_v6"><h4>sig</h4></section>"#;
        assert_eq!(attested_item_shapes(rs2).0[0][0], "method.only_v6");

        // A dotless region id (an MDN section slug) and a `-1` duplicate id attest NOTHING.
        let mdn = r#"<h2 id="syntax"></h2><div class="notecard deprecated"><p>Deprecated.</p></div>"#;
        assert!(attested_item_shapes(mdn).0.is_empty(), "dotless region id abstains");
        let dup = r#"<section id="method.is_ascii-1"></section><div class="stab deprecated">note</div>"#;
        assert!(attested_item_shapes(dup).0.is_empty(), "invalid dotted id abstains, never slides past");
    }

    #[test]
    fn note_scope_registers_counter_attest_exception_and_usage_form_notes() {
        // PASS 28 — EXCEPTION SCOPE: the note names its attributed item AFTER "except", so the sentence
        // EXCLUDES it (the real ssl.TLSVersion.TLSv1_3 shape). The marker attests nothing.
        let except = r#"<dl><dt id="ssl.TLSVersion.TLSv1_3"></dt><dd><p>Enum member prose.</p>
            <div class="deprecated"><p>Deprecated since version 3.10: All TLSVersion members except
            TLSVersion.TLSv1_2 and TLSVersion.TLSv1_3 are deprecated.</p></div></dd></dl>"#;
        let (enforce, counter) = attested_item_shapes(except);
        assert!(enforce.is_empty(), "excepted item leaves the enforcement view");
        assert_eq!(counter.len(), 1, "…but stays a structural read (language witness): {counter:?}");
        assert_eq!(counter[0][0], "ssl.TLSVersion.TLSv1_3");

        // PASS 28 — CONDITIONAL FORM: the first sentence deprecates a call/argument form ("… if
        // loop …"), not the item (the real asyncio.shield shape). Literal anchor — no brain needed.
        let cond = r#"<dl><dt id="asyncio.shield"></dt><dd><p>Prose.</p>
            <div class="deprecated"><p>Deprecated since version 3.10: Deprecation warning is emitted if
            aw is not Future-like object and there is no running event loop.</p></div></dd></dl>"#;
        let (enforce, counter) = attested_item_shapes(cond);
        assert!(enforce.is_empty() && counter.len() == 1, "conditional-form note is counter-attested");

        // FIRST-SENTENCE-ONLY is load-bearing: "if" in a LATER remedy sentence must not cut a true
        // deprecation (the real SourceLoader.path_mtime shape).
        let keep = r#"<dl><dt id="abc.SourceLoader.path_mtime"></dt><dd>
            <div class="deprecated"><p>Deprecated since version 3.3: This method is deprecated in favour
            of path_stats(). Raise OSError if the path cannot be handled.</p></div></dd></dl>"#;
        let (groups, counter) = attested_item_shapes(keep);
        assert_eq!(groups.len(), 1, "remedy-sentence 'if' keeps the true deprecation: {groups:?}");
        assert_eq!(groups[0][0], "abc.SourceLoader.path_mtime");
        assert!(counter.is_empty());

        // Bounded-word: "if" inside a longer identifier-ish word never matches; a plain note attests.
        let plain = r#"<dl><dt id="mod.olditem"></dt><dd>
            <div class="deprecated"><p>Deprecated since version 9.9: use shift() or newitem() instead.
            </p></div></dd></dl>"#;
        assert_eq!(attested_item_shapes(plain).0.len(), 1, "'shift' does not read as the 'if' register");
    }

    #[test]
    fn note_scope_registers_are_meaning_anchored_not_token_lists() {
        // PASS 29 — the registers ride the MEANING NET (owner ruling: registers are meaning, tokens are
        // shims). Needs the frozen brain on disk; a brainless machine matches literal anchors only.
        if crate::lint_char::brain().is_none() {
            eprintln!("skip: no frozen brain on disk");
            return;
        }
        // GERUND USAGE-SUBJECT with zero vocabulary: "Passing"/"Setting" are gerunds of dictionary
        // VERBS (`pass`, `set` via un-doubling) — no gerund appears in any data file.
        assert!(
            note_counter_attests("deprecated since version 3.13: passing maxsplit as positional arguments is deprecated.", "re.split"),
            "gerund of a dictionary verb reads as the usage-subject register"
        );
        assert!(
            note_counter_attests("deprecated since version 3.13: setting an attribute by two positional arguments is deprecated.", "tkinter.Wm.attributes"),
            "consonant-doubled gerund resolves to its verb stem"
        );
        // An unlisted gerund generalizes the same way ("calling" was never in any list).
        assert!(
            note_counter_attests("deprecated since version 3.15: calling this with a loop argument is deprecated.", "mod.thing"),
            "unseen gerund of a known verb fires the register"
        );
        // EXCEPTION MEANING beyond the anchor: "unless" is a conjunction the dictionary defines via
        // "except", so an item named after it is excluded — no "unless" token exists anywhere.
        assert!(
            note_counter_attests("deprecated since version 3.10: all members are deprecated unless they are tlsversion.tlsv1_2 or tlsversion.tlsv1_3.", "ssl.TLSVersion.TLSv1_3"),
            "'unless' carries the exception meaning through its own definition"
        );
        // A content word whose definition merely mentions an anchor does NOT ride the register (the
        // function-word POS gate), and a first word that is not a verb gerund stays plain.
        assert!(
            !note_counter_attests("deprecated since version 3.12: this method is deprecated in favour of path_stats().", "abc.SourceLoader.path_mtime"),
            "a plain deprecation stays attested under the meaning-anchored read"
        );
    }

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
        let page = read_doc_page(url, body, en, &bridge, &std::collections::HashSet::new(), &std::collections::HashMap::new());
        assert!(page.prohibited, "a /rules/ page is a prohibition by its role");
        assert!(page.constructs.contains(&"~~".to_string()), "the operator candidate is read: {:?}", page.constructs);
        assert!(page.incorrect.iter().any(|b| b.contains("~~")), "the page's own bad example is captured");
        assert!(page.correct.iter().any(|b| b.contains("~~~")), "the page's own good example is captured");
    }

    #[test]
    fn script_interior_unwraps_a_lone_script_element_only() {
        assert_eq!(script_interior("<script>obj.qux(1);</script>").as_deref(), Some("obj.qux(1);"));
        assert_eq!(script_interior("  <SCRIPT>\n obj.qux(1);\n</SCRIPT>  ").as_deref(), Some("obj.qux(1);"));
        assert_eq!(script_interior("obj.qux(1);"), None, "ordinary code is not a script element");
        assert_eq!(script_interior("<script src=\"x.js\"></script>"), None, "empty interior is not JS to surface");
        assert_eq!(
            script_interior("<p>hi</p><script>obj.qux(1)</script>"),
            None,
            "HTML with other content is left whole"
        );
    }

    #[test]
    fn example_receiver_shapes_reads_the_qualified_receiver_from_the_example() {
        // A non-reference page: subject `qux`, owner segment `Obj`; the example's actual receiver is `obj`.
        let url = "https://example.org/en-US/docs/Web/API/Obj/qux";
        let ex = vec!["obj.qux(1);\nfoo.bar();".to_string()];
        assert_eq!(example_receiver_shapes(url, &ex), vec!["obj.qux".to_string()], "owner-linked, actual case");
        // A DIFFERENT receiver (a local var) is NOT the owner, so no qualified shape is minted.
        let ex2 = vec!["thing.qux(1);".to_string()];
        assert!(example_receiver_shapes(url, &ex2).is_empty(), "a non-owner receiver mints no shape");
    }

    #[test]
    fn a_non_reference_notecard_page_prohibits_its_qualified_subject() {
        let (Some(br), Some(en)) = (crate::lint_char::brain(), crate::lint_english::brain()) else {
            eprintln!("skip: no frozen brains on disk");
            return;
        };
        let bridge = Bridge::new(br.meanings(), en);
        // A NON-`/reference/` API page carrying a deprecation notecard, whose lone-script example shows the
        // subject called on its owner receiver — the `document.write` shape, opaque here (`obj.qux`). The
        // LEARNED attester is fed the page's own banner run (the corpus-discovered P=R=1.000 markers live in
        // `examples/metajoin`); here we exercise the attestation → construct-shape path deterministically.
        let url = "https://example.org/en-US/docs/Web/API/Obj/qux";
        let body = r#"<html><body><h1>Obj: qux() method</h1>
            <div class="notecard deprecated"><p>Deprecated: no longer recommended.</p></div>
            <pre class="brush: html"><code>&lt;script&gt;obj.qux("x");&lt;/script&gt;</code></pre>
            </body></html>"#;
        // The caller attested this url from the page's own metadata banner (the corpus-discovered P=R=1.000
        // markers live in `examples/metajoin`); here we exercise the attestation → construct-shape path.
        let attested = std::collections::HashSet::from([url.to_string()]);
        let page = read_doc_page(url, body, en, &bridge, &attested, &std::collections::HashMap::new());
        assert!(page.prohibited && page.attested_deprecated, "a notecard page is a prohibition without /reference/");
        assert!(page.constructs.contains(&"obj.qux".to_string()), "qualified shape proposed: {:?}", page.constructs);
        assert!(
            page.example_code.iter().any(|b| b.contains("obj.qux") && !b.contains("<script")),
            "the script interior is surfaced as JS example code: {:?}",
            page.example_code
        );
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
        let page = read_doc_page("https://example.org/reference/operators/qux", body, en, &bridge, &std::collections::HashSet::new(), &std::collections::HashMap::new());
        assert!(!page.prohibited, "a reference page with no deprecation notecard is not a prohibition");
        assert!(page.constructs.is_empty(), "no construct proposed from a non-prohibition page");
    }
}
