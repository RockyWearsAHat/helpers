//! PROVEN CONSTRUCTION STATES — the productionization of the PASS-21 measured mechanism (COMPLETION
//! PASS 22). A CONSTRUCTION is a sentence-scale invariant SCAFFOLD with a variant code SLOT (the
//! chrome/attestation invariant-run atom GENERALISED so the scaffold is invariant while a `<code>` slot
//! varies page-to-page). This module MINES constructions from a language's crawled doc pages and PROVES
//! their MEANING against the machine's OWN proven-deprecated subject set, under the frozen ISM law
//! ([`crate::lint_ism`] — ≥ [`REQUIRED_WITNESSES`] genuinely independent witnesses, zero contradiction).
//!
//! Everything here is machine-derived, zero meaning geometry, no word list, no template. The two frozen
//! signals it stands on, exactly as PASS 21 measured them:
//!
//! WALL A — the slot-variance scaffold/chrome disambiguator. The owner's signal made structural:
//! *chrome's content is INVARIANT by definition, so a run flanking a code slot whose filler VARIES
//! page-to-page is definitionally a SCAFFOLD, not chrome.* Mining runs CO-RESIDENT with the strip on the
//! UNSTRIPPED bodies; a construction is a genuine scaffold iff its primary slot's single most-common
//! filler covers no more than [`SCAFFOLD_MODE_FRAC`] of its pages (chrome/boilerplate carry one filler on
//! ~all pages → mode → 1.0; a real scaffold's code is page-specific → mode low). The live prose reader
//! keeps stripping chrome fully; only this miner claims the varying-slot runs back.
//!
//! WALL B — instance-level witnesses. A construction's witnesses are its OWN INSTANCES cross-referenced
//! against the machine's proven-deprecated SUBJECT set (each subject independently proven by the
//! attestation faculty — genuine ISM independence). A single-slot instance whose filler NAMES a deprecated
//! subject witnesses deprecation-direction; a two-slot instance with slotA deprecated and slotB clean
//! witnesses supersession/remedy. Independence = DISTINCT subject. ≥ [`REQUIRED_WITNESSES`] with zero
//! contradiction PROVES; below the floor stays MINED-UNPROVEN (the discovery-starved wall — no floor
//! relitigating).
//!
//! MEASURED (PASS 21, `examples/construct_p21`, reproduced by `examples/construct_prod`): over the real
//! corpus this yields exactly ONE proven state today — `the last version of python that provided the ⟨⟩
//! module was` (24 distinct removed python modules, each independently attested by its own
//! `class="deprecated-removed"` whole-module badge; 24 ≥ 15, zero contradiction). The artifact grows as the
//! corpus and the proven-subject set grow (infinite states); an unproven construction is simply not
//! returned (held as mined-unproven, never consumed).

use crate::lint_ism::REQUIRED_WITNESSES;
use std::collections::{HashMap, HashSet};

/// The cross-page repetition-support floor the whole faculty trusts (`site_chrome`, `lint_attest`): a
/// construction must recur on at least this many distinct pages to be more than an accident.
const SUPPORT_FLOOR: usize = 8;
/// A scaffold below this many non-slot words is too thin to be a construction (a bare `⟨⟩` is not a
/// sentence).
const MIN_SCAFFOLD_WORDS: usize = 3;
/// A construction is a genuine SCAFFOLD (not chrome/boilerplate) iff its primary slot VARIES across pages:
/// no single filler covers more than this fraction of the construction's pages (the mode-fraction test).
const SCAFFOLD_MODE_FRAC: f64 = 0.5;
/// The largest window (bytes) a rendered deprecation badge may reach to attribute its dotted item — the
/// PASS-14 item unit's anchor window, replicated for the subject-basis read.
const MARKER_ANCHOR_WINDOW: usize = 20_000;

const SLOT_OPEN: char = '\u{1}';
const SLOT_CLOSE: char = '\u{2}';

/// What a proven construction ASSERTS about a slot filler that names a proven-deprecated subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstructionKind {
    /// Single-slot, every proving witness is a REMOVED-module subject (`class="deprecated-removed"`): the
    /// slot names a removed feature (`the last version of python that provided the ⟨⟩ module was`).
    Removal,
    /// Single-slot deprecation-direction: the slot names a deprecated subject, not proven removed.
    Prohibition,
    /// Two-slot supersession/remedy: slotA deprecated, slotB clean (the modern replacement).
    Supersession,
}

impl ConstructionKind {
    fn tag(self) -> u64 {
        match self {
            ConstructionKind::Removal => 0,
            ConstructionKind::Prohibition => 1,
            ConstructionKind::Supersession => 2,
        }
    }
    fn from_tag(t: u64) -> Option<ConstructionKind> {
        match t {
            0 => Some(ConstructionKind::Removal),
            1 => Some(ConstructionKind::Prohibition),
            2 => Some(ConstructionKind::Supersession),
            _ => None,
        }
    }
}

/// One PROVEN construction state — the persisted, retain-and-grow artifact the consumer reads. Carries the
/// invariant SCAFFOLD shape (slot positions marked `⟨⟩`), what it asserts, the independent-witness count
/// that proved it, and one rendered example for citation/diagnostics.
#[derive(Clone, Debug)]
pub struct ConstructionState {
    /// The invariant scaffold, lower-cased, with each variant code slot rendered as `⟨⟩` (e.g. `the last
    /// version of python that provided the ⟨⟩ module was`). The identity of the construction.
    pub shape: String,
    /// What the construction asserts about a slot filler naming a proven-deprecated subject.
    pub kind: ConstructionKind,
    /// The number of genuinely independent proven-deprecated subjects that witnessed it (≥
    /// [`REQUIRED_WITNESSES`]).
    pub witnesses: usize,
    /// One rendered instance (`⟨…⟩` slots) — citation and human/measurement readout only.
    pub example: String,
}

impl crate::lint_codec::Bin for ConstructionState {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.str(&self.shape);
        e.u(self.kind.tag());
        e.u(self.witnesses as u64);
        e.str(&self.example);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<ConstructionState> {
        Some(ConstructionState {
            shape: d.str()?,
            kind: ConstructionKind::from_tag(d.u()?)?,
            witnesses: d.u()? as usize,
            example: d.str()?,
        })
    }
}

/// MINE and PROVE the construction states over `pages` (a language's crawled doc corpus). Returns every
/// construction PROVEN under the frozen ≥[`REQUIRED_WITNESSES`]/0-contradiction law; unproven constructions
/// are held (not returned). Pure over `pages` and the frozen faculties — no network, no training.
pub fn mine_and_prove(pages: &[(String, String)]) -> Vec<ConstructionState> {
    if pages.len() < SUPPORT_FLOOR {
        return Vec::new();
    }
    let (subjects, removal_subjects) = proven_deprecated_subjects(pages);
    if subjects.is_empty() {
        return Vec::new();
    }
    // The co-resident design: mine the UNSTRIPPED bodies (scaffolds intact) and keep every varying-slot
    // scaffold ([`is_scaffold`]). The live prose reader keeps stripping chrome fully; only this miner
    // claims the varying-slot runs back — so no `site_chrome` pass is needed here (the strip comparison
    // was PASS-21 reporting only; the mode-fraction test is the structural separator).
    let unstripped = mine(pages);
    let mut out = Vec::new();
    for (shape, s) in &unstripped {
        if !is_scaffold(s) {
            continue;
        }
        let Some(verdict) = prove(s, &subjects, &removal_subjects) else { continue };
        out.push(ConstructionState {
            shape: shape.clone(),
            kind: verdict,
            witnesses: distinct_dep_slot_a(s, &subjects),
            example: s.example.clone(),
        });
    }
    out.sort_by(|a, b| b.witnesses.cmp(&a.witnesses).then_with(|| a.shape.cmp(&b.shape)));
    out
}

/// The per-language PROVEN-CONSTRUCTION artifact path (`<lang>.constructions.bin`, beside the module and
/// the graduated ledger). A SEPARATE sidecar so it survives the module rebuild every retrain does.
fn constructions_path(lang: &str) -> std::path::PathBuf {
    crate::lint_train::model_dir_pub().join(format!("{lang}.constructions.bin"))
}

/// The construction states PROVEN in past retrains for `lang`, or empty when none is persisted. Stamped
/// with the [`crate::lint_train::train_version`] it was written under and DISCARDED on a mismatch (a
/// semantic bump may change the shape/witness semantics). Never trains; a pure read.
pub fn load(lang: &str) -> Vec<ConstructionState> {
    let Ok(bytes) = std::fs::read(constructions_path(lang)) else {
        return Vec::new();
    };
    let Some((stamp, mut d)) =
        crate::lint_codec::Dec::open(&bytes, crate::lint_codec::kind::CONSTRUCT)
    else {
        return Vec::new();
    };
    if stamp != crate::lint_train::train_version() {
        return Vec::new();
    }
    <Vec<ConstructionState> as crate::lint_codec::Bin>::dec(&mut d).unwrap_or_default()
}

/// Persist the PROVEN construction states for `lang`, retain-and-grow: written ONLY when there are proven
/// states, so a subset crawl that proves nothing never wipes a prior artifact (mirrors
/// `persist_graduated_ledger`). Stamped with the current train version.
pub fn persist(lang: &str, states: &[ConstructionState]) {
    if states.is_empty() {
        return;
    }
    let mut e = crate::lint_codec::Enc::new();
    <Vec<ConstructionState> as crate::lint_codec::Bin>::enc(&states.to_vec(), &mut e);
    let bytes = e.finish(crate::lint_codec::kind::CONSTRUCT, crate::lint_train::train_version());
    if let Some(parent) = constructions_path(lang).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(constructions_path(lang), bytes);
}

/// The machine's proven-deprecated SUBJECT set, keyed clean (item-level), plus the subset that came from a
/// WHOLE-MODULE removal badge (`class="deprecated-removed"`). Three independent, data-keyed routes — the
/// PASS-21 subject basis, faithful:
///   (i)   MDN text-run attestation whose page IS the deprecated feature: subject = URL last segment.
///   (ii)  rendered class badges attributed to their OWN dotted item (PASS-14 item unit): the qualified
///         `owner.member` form, never the bare last component (which collides and manufactures witnesses).
///   (iii) whole-module removal pages (the PASS-22 removal faculty, `deprecated-removed` — a page with NO
///         dotted-item badge): subject = the module basename (extension stripped).
fn proven_deprecated_subjects(pages: &[(String, String)]) -> (HashSet<String>, HashSet<String>) {
    let attest = crate::lint_attest::Attestation::discover(pages);
    let text_markers = attest.markers().to_vec();
    let tokens = crate::lint_attest::prohibition_class_tokens();
    let attests_by_text = |body: &str| -> bool {
        !text_markers.is_empty() && {
            let runs: HashSet<String> = runs_of(body).into_iter().collect();
            text_markers.iter().any(|m| runs.contains(m))
        }
    };
    let mut subjects: HashSet<String> = HashSet::new();
    let mut removal: HashSet<String> = HashSet::new();
    for (url, body) in pages {
        if attests_by_text(body) {
            let subj = url_subject(url);
            if !subj.is_empty() {
                subjects.insert(subj);
            }
        }
        let items = attested_items(body, &tokens);
        for id in &items {
            let low = id.to_lowercase();
            subjects.insert(low.clone());
            let parts: Vec<&str> = low.split('.').collect();
            if parts.len() >= 2 {
                subjects.insert(parts[parts.len() - 2..].join("."));
            }
        }
        // (iii) whole-module removal: the TIGHT compound, ONLY on a page with no dotted-item badge (so the
        // marker is the page's own module-level status, not a stray sidebar cross-ref). PASS-21 precision
        // line — this is the faithful author typography; the loose `deprecated` substring contaminates.
        if items.is_empty() && crate::lint_attest::attests_module_removal(body) {
            let subj = module_subject(url);
            if !subj.is_empty() {
                subjects.insert(subj.clone());
                removal.insert(subj);
            }
        }
    }
    (subjects, removal)
}

/// A construction's mined support: the pages it appears on, its distinct slot fillers, and its per-slot
/// filler sets — the substrate for the scaffold-variance and witness tests.
struct Shape {
    pages: HashSet<String>,
    fillers: HashSet<String>,
    slot_fillers: Vec<HashSet<String>>,
    /// primary-slot filler → distinct pages carrying it (the invariance/mode-fraction test).
    filler_pages: HashMap<String, HashSet<String>>,
    example: String,
}

/// Mine every slot-bearing sentence shape across `bodies`, accumulating each shape's support.
fn mine(bodies: &[(String, String)]) -> HashMap<String, Shape> {
    let mut m: HashMap<String, Shape> = HashMap::new();
    for (url, body) in bodies {
        let text = flatten(body);
        for sent in sentences(&text) {
            let Some((shape, fillers)) = shape_and_fillers(&sent) else { continue };
            let e = m.entry(shape.clone()).or_insert_with(|| Shape {
                pages: HashSet::new(),
                fillers: HashSet::new(),
                slot_fillers: Vec::new(),
                filler_pages: HashMap::new(),
                example: render_example(&sent),
            });
            e.pages.insert(url.clone());
            e.filler_pages.entry(fillers[0].to_lowercase()).or_default().insert(url.clone());
            for (i, f) in fillers.iter().enumerate() {
                e.fillers.insert(f.clone());
                if e.slot_fillers.len() <= i {
                    e.slot_fillers.push(HashSet::new());
                }
                e.slot_fillers[i].insert(f.clone());
            }
        }
    }
    m
}

fn is_construction(s: &Shape) -> bool {
    s.pages.len() >= SUPPORT_FLOOR && s.fillers.len() >= 2
}

/// The mode fraction of a construction's primary slot: the share of its pages covered by its single
/// most-common filler. Chrome/boilerplate → ~1.0; a real scaffold → low.
fn mode_frac(s: &Shape) -> f64 {
    let pages = s.pages.len().max(1) as f64;
    let mode = s.filler_pages.values().map(HashSet::len).max().unwrap_or(0) as f64;
    mode / pages
}

fn is_scaffold(s: &Shape) -> bool {
    is_construction(s) && mode_frac(s) <= SCAFFOLD_MODE_FRAC
}

/// Whether a slot filler NAMES a proven-deprecated subject: the normalized bare form, or its qualified
/// `owner.member` tail pair, must EQUAL a proven subject. EXACT membership only — a lossy last-component
/// tail collides with common member names and MANUFACTURES witnesses (PASS 21, measured).
fn names_subject(f: &str, subjects: &HashSet<String>) -> bool {
    let low = f.to_lowercase();
    let bare: String = low
        .trim_end_matches("()")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
        .collect();
    if bare.len() < 2 {
        return false;
    }
    if subjects.contains(&bare) {
        return true;
    }
    let parts: Vec<&str> = bare.split('.').collect();
    parts.len() >= 2 && subjects.contains(&parts[parts.len() - 2..].join("."))
}

/// Distinct proven-deprecated subjects named by the construction's PRIMARY slot (the independent
/// deprecation-direction witnesses).
fn distinct_dep_slot_a(s: &Shape, subjects: &HashSet<String>) -> usize {
    s.slot_fillers.first().map(|set| set.iter().filter(|f| names_subject(f, subjects)).count()).unwrap_or(0)
}

/// PROVE a scaffold's meaning under the frozen law, returning its [`ConstructionKind`] iff it reaches ≥
/// [`REQUIRED_WITNESSES`] independent deprecated-subject witnesses with zero contradiction; `None`
/// otherwise (held mined-unproven). Classification is behavioral, not word-based:
/// - two-slot, slotA ≥ floor deprecated, slotB has clean fillers and ZERO deprecated (no contradiction) →
///   [`ConstructionKind::Supersession`];
/// - single-slot, slotA ≥ floor deprecated, EVERY witness is a removal subject → [`ConstructionKind::Removal`];
/// - single-slot, slotA ≥ floor deprecated → [`ConstructionKind::Prohibition`].
fn prove(
    s: &Shape,
    subjects: &HashSet<String>,
    removal: &HashSet<String>,
) -> Option<ConstructionKind> {
    let dep_a = distinct_dep_slot_a(s, subjects);
    if dep_a < REQUIRED_WITNESSES {
        return None;
    }
    let two_slot = s.slot_fillers.len() >= 2;
    if two_slot {
        let b = &s.slot_fillers[1];
        let dep_b = b.iter().filter(|f| names_subject(f, subjects)).count();
        let clean_b = b.iter().filter(|f| !names_subject(f, subjects)).count();
        // A supersession must NOT put a deprecated feature in its remedy slot (contradiction).
        if dep_b > 0 || clean_b == 0 {
            return None;
        }
        return Some(ConstructionKind::Supersession);
    }
    // Single-slot: Removal iff every proving witness is a removal-module subject; else Prohibition.
    let all_removal = s
        .slot_fillers
        .first()
        .map(|set| {
            set.iter()
                .filter(|f| names_subject(f, subjects))
                .all(|f| names_subject(f, removal))
        })
        .unwrap_or(false);
    Some(if all_removal { ConstructionKind::Removal } else { ConstructionKind::Prohibition })
}

// ── the module subject key (removal pages) ─────────────────────────────────────────────────────────
/// A removed module's subject: the URL basename with any `.html`/`.htm`/`.php` extension stripped
/// (`aifc.html` → `aifc`) — the form the construction's slot filler carries.
fn module_subject(url: &str) -> String {
    let mut s = url_subject(url);
    for ext in [".html", ".htm", ".php"] {
        if let Some(base) = s.strip_suffix(ext) {
            s = base.to_string();
        }
    }
    s
}

/// A page's subject key: the last URL path segment, case-folded, fragment/trailing-slash trimmed — the
/// same key [`crate::lint_attest`]'s subject reader uses.
fn url_subject(url: &str) -> String {
    url.split('#')
        .next()
        .unwrap_or(url)
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

// ── item-level deprecation attestation (the PASS-14 unit, subject-basis read) ───────────────────────
fn attr(tag: &str, name: &str) -> Option<String> {
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
}

/// Whether a value is a dotted API item id (`socket.close`, `method.only_v6`) — the qualified subject a
/// badge anchors, distinct from a bare region id.
fn dotted_item(v: &str) -> bool {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    v.len() >= 3
        && v.contains('.')
        && !v.starts_with('.')
        && !v.ends_with('.')
        && !v.contains("..")
        && v.chars().all(|c| is_ident(c) || c == '.')
        && v.rsplit('.').next().map(|last| last.len() >= 2).unwrap_or(false)
}

/// The dotted item ids a body marks deprecated, each attributed by the marker element's typography
/// (self-anchored / container-opening / trailing-badge) — the PASS-14 item unit.
fn attested_items(body: &str, tokens: &[String]) -> Vec<String> {
    if tokens.is_empty() {
        return Vec::new();
    }
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
    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (mpos, own_id) in &markers {
        let mpos = *mpos;
        let next = ids.iter().find(|(apos, _)| *apos > mpos);
        let prose_intervenes = |apos: usize| {
            let k = text_positions.partition_point(|&t| t <= mpos);
            text_positions.get(k).is_some_and(|&t| t < apos)
        };
        let attributed = match (own_id, next) {
            (Some(id), _) => Some(id),
            (None, Some((apos, id))) if apos - mpos <= MARKER_ANCHOR_WINDOW && !prose_intervenes(*apos) => {
                Some(id)
            }
            _ => ids
                .iter()
                .rev()
                .find(|(apos, _)| *apos < mpos && mpos - apos <= MARKER_ANCHOR_WINDOW)
                .map(|(_, id)| id),
        };
        if let Some(id) = attributed.filter(|id| dotted_item(id)) {
            if seen.insert(id.clone()) {
                out.push(id.clone());
            }
        }
    }
    out
}

// ── flatten / sentence / shape (the PASS-21 slot atom) ──────────────────────────────────────────────
/// Flatten an HTML body to text, replacing each short inline `<code>…</code>` with a SLOT sentinel
/// carrying its filler (`\u{1}filler\u{2}`) — the variant code slot the scaffold flanks. Non-code tags
/// become spaces; entities decode.
fn flatten(body: &str) -> String {
    let b = body.as_bytes();
    let mut out = String::with_capacity(body.len() / 2);
    let mut i = 0usize;
    while i < b.len() {
        if b[i] == b'<' {
            let close = body[i..].find('>').map(|k| i + k).unwrap_or(b.len().saturating_sub(1));
            let tag = &body[i + 1..close.min(body.len())];
            let name: String =
                tag.chars().take_while(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
            if name == "code" {
                let after = (close + 1).min(body.len());
                if let Some(rel) = body[after..].find("</code>") {
                    let filler: String =
                        strip_tags(&body[after..after + rel]).split_whitespace().collect::<Vec<_>>().join(" ");
                    if !filler.is_empty() && filler.len() <= 60 {
                        out.push(SLOT_OPEN);
                        out.push_str(&filler);
                        out.push(SLOT_CLOSE);
                    } else {
                        out.push(' ');
                    }
                    i = after + rel + "</code>".len();
                    continue;
                }
            }
            out.push(' ');
            i = close + 1;
            continue;
        }
        let ch = body[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    decode_entities(&out)
}

fn strip_tags(frag: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in frag.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    decode_entities(&out)
}

fn decode_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

/// Split flattened text into sentences, never breaking inside a slot sentinel.
fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_slot = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            SLOT_OPEN => in_slot = true,
            SLOT_CLOSE => in_slot = false,
            _ => {}
        }
        cur.push(c);
        let boundary = !in_slot
            && matches!(c, '.' | '!' | '?' | '\n')
            && chars.peek().is_none_or(|n| n.is_whitespace());
        if boundary {
            push_sentence(&mut out, &cur);
            cur.clear();
        }
    }
    push_sentence(&mut out, &cur);
    out
}

fn push_sentence(out: &mut Vec<String>, cur: &str) {
    let s: String = cur.split_whitespace().collect::<Vec<_>>().join(" ");
    if !s.is_empty() {
        out.push(s);
    }
}

/// Derive a sentence's SHAPE (lower-cased words with each slot as `⟨⟩`) and its ordered slot fillers, iff
/// it carries at least one slot and at least [`MIN_SCAFFOLD_WORDS`] non-slot words.
fn shape_and_fillers(sentence: &str) -> Option<(String, Vec<String>)> {
    if !sentence.contains(SLOT_OPEN) {
        return None;
    }
    let mut shape = String::new();
    let mut fillers = Vec::new();
    let mut words = 0usize;
    let mut chars = sentence.chars().peekable();
    let mut word = String::new();
    let flush = |word: &mut String, shape: &mut String, words: &mut usize| {
        if !word.is_empty() {
            shape.push_str(&word.to_lowercase());
            shape.push(' ');
            *words += 1;
            word.clear();
        }
    };
    while let Some(c) = chars.next() {
        match c {
            SLOT_OPEN => {
                flush(&mut word, &mut shape, &mut words);
                let mut filler = String::new();
                for c2 in chars.by_ref() {
                    if c2 == SLOT_CLOSE {
                        break;
                    }
                    filler.push(c2);
                }
                shape.push_str("⟨⟩ ");
                fillers.push(filler);
            }
            c if c.is_whitespace() => flush(&mut word, &mut shape, &mut words),
            c => word.push(c),
        }
    }
    flush(&mut word, &mut shape, &mut words);
    (words >= MIN_SCAFFOLD_WORDS && !fillers.is_empty()).then(|| (shape.trim().to_string(), fillers))
}

fn render_example(sentence: &str) -> String {
    sentence.replace(SLOT_OPEN, "⟨").replace(SLOT_CLOSE, "⟩")
}

/// Whitespace-collapse a text span; a run of ≥ 2 words and ≥ 6 chars is a comparison atom (the same rule
/// [`crate::lint_attest`]'s site-invariance atom uses).
fn norm_run(text: &str) -> Option<String> {
    let norm: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if norm.len() < 6 || norm.split(' ').count() < 2 {
        return None;
    }
    Some(norm)
}

/// The invariant text runs of an HTML body (structure only — the text between tags).
fn runs_of(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let b = body.as_bytes();
    let (mut in_tag, mut start) = (false, 0usize);
    for i in 0..b.len() {
        match b[i] {
            b'<' => {
                if !in_tag {
                    if let Some(r) = norm_run(&body[start..i]) {
                        out.push(r);
                    }
                }
                in_tag = true;
            }
            b'>' => {
                in_tag = false;
                start = i + 1;
            }
            _ => {}
        }
    }
    if let Some(r) = norm_run(&body[start..]) {
        out.push(r);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint_codec::Bin;

    /// A synthetic python-shaped corpus: ≥15 removed-module pages, each carrying the removal construction
    /// sentence with its OWN module in the slot and its own `class="deprecated-removed"` whole-module
    /// badge. The construction must PROVE as a Removal (24-style, here 16 distinct modules ≥ 15).
    #[test]
    fn removal_construction_proves_over_synthetic_python_pages() {
        let modules =
            ["aifc", "audioop", "asynchat", "asyncore", "cgi", "cgitb", "chunk", "crypt", "imghdr",
             "mailcap", "msilib", "nis", "nntplib", "ossaudiodev", "pipes", "sndhdr", "telnetlib", "uu"];
        let mut pages = Vec::new();
        for m in modules {
            let body = format!(
                "<div class=\"deprecated-removed\"><p>Deprecated since 3.11, removed in 3.13.</p></div>\
                 <p>The last version of Python that provided the <code>{m}</code> module was Python 3.12.</p>",
            );
            pages.push((format!("https://docs.python.org/3/library/{m}.html"), body));
        }
        let proven = mine_and_prove(&pages);
        let removal = proven
            .iter()
            .find(|c| c.shape.contains("last version of python") && c.shape.contains("module was"));
        let c = removal.expect("the python-removal construction must prove");
        assert_eq!(c.kind, ConstructionKind::Removal);
        assert!(c.witnesses >= REQUIRED_WITNESSES, "witnesses {} < floor", c.witnesses);
    }

    /// Below the floor (only a handful of removed modules), NOTHING proves — the discovery-starved wall,
    /// no floor relitigating.
    #[test]
    fn below_the_witness_floor_nothing_proves() {
        let modules = ["aifc", "audioop", "cgi"];
        let mut pages = Vec::new();
        for m in modules {
            let body = format!(
                "<div class=\"deprecated-removed\">removed</div>\
                 <p>The last version of Python that provided the <code>{m}</code> module was Python 3.12.</p>",
            );
            pages.push((format!("https://docs.python.org/3/library/{m}.html"), body));
        }
        assert!(mine_and_prove(&pages).is_empty(), "3 subjects < 15 ⇒ mined-unproven");
    }

    /// The round-trip codec contract: a proven state survives encode → decode byte-identical.
    #[test]
    fn construction_state_round_trips() {
        let state = ConstructionState {
            shape: "the last version of python that provided the ⟨⟩ module was".to_string(),
            kind: ConstructionKind::Removal,
            witnesses: 24,
            example: "The last version of Python that provided the ⟨aifc⟩ module was 3.12".to_string(),
        };
        let mut e = crate::lint_codec::Enc::new();
        state.enc(&mut e);
        let bytes = e.finish(crate::lint_codec::kind::CONSTRUCT, "test-stamp");
        let (stamp, mut d) =
            crate::lint_codec::Dec::open(&bytes, crate::lint_codec::kind::CONSTRUCT).expect("opens");
        assert_eq!(stamp, "test-stamp");
        let back = ConstructionState::dec(&mut d).expect("decodes");
        assert_eq!(back.shape, state.shape);
        assert_eq!(back.kind, state.kind);
        assert_eq!(back.witnesses, state.witnesses);
        assert_eq!(back.example, state.example);
    }
}
