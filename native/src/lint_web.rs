//! THE LANGUAGE WEB — a language's understanding persisted as a SUBGRAPH, from which the rules are
//! read as VIEWS (COMPLETION PASS 24). This is the owner's frame made real: "the english understanding
//! should build a web of the language … LANGUAGE UNDERSTANDING IS WHAT DRIVES LINTING." Where the
//! graduation workflow ([`crate::lint_module`]) used to REDUCE its read to a rule list and discard the
//! rest, this module PERSISTS the whole read as the language's subgraph and DERIVES the rules from it.
//!
//! A [`ConstructNode`] binds, per construct the language read:
//!   * the construct token (the node id, byte-preserved — `var`, `document.write`, `cgi`),
//!   * its GOVERNING PROSE (the understanding sentence, and the advice sentence when one was derived),
//!   * the MEANING LINKS — the distinctive content KEY-WORDS of that prose, each a key into the FROZEN
//!     English web ([`crate::lint_char::MeaningNetwork`]). Never a copy of the English base: the words
//!     are keys and their meaning is REBOUND on query (the same delta pattern the dictionary itself uses
//!     — it stores definition-WORDS, not the meaning hypervectors),
//!   * the doc SOURCE cites, and
//!   * the ATTESTATION state (author-metadata deprecation) plus whether the node is PROVEN (enforced) or
//!     merely READ (present in the web, never fired).
//!
//! **Rules are VIEWS.** A rule is a web node whose state is PROVEN — it carries the compiled/cached rule
//! payload ([`WebRule`]) as a "query plan" for the fast live path. [`derive_rules`] projects exactly the
//! proven nodes back to the `(rule, source)` shape the module build consumes, so deleting the web and
//! re-deriving reproduces the same rules ([`round_trips_and_derives`]).
//!
//! **Everything READ is retained.** An unproven construct is a node with `proven == false` and no rule —
//! present in the web (queryable, cross-linkable) but never enforced. Coverage becomes everything-read;
//! enforcement stays the proven subset.
//!
//! **Webs connect across languages through the shared English base.** A node's meaning links are
//! key-words into the SAME frozen English web, so a cross-language relation is a QUERY: two constructs
//! relate when their governing prose shares distinctive meaning ([`node_meaning`] Hamming proximity), and
//! both traverse to the same English concepts (python's removed `cgi` and JS's deprecated `document.write`
//! both reach the deprecation/removal concepts). `class` in JS and `class` in Python are DIFFERENT nodes
//! (distinct language webs) whose meaning links may traverse to shared concepts — never conflated.

use crate::lint_char::MeaningNetwork;
use std::collections::{HashMap, HashSet};

/// The largest number of meaning-link key-words a node stores per governing sentence — the distinctive
/// content words that carry the sentence's sense (the filler every sentence shares is suppressed by the
/// frozen brain's inverse-document-frequency centrality). Bounded so the subgraph stays a delta, not a
/// copy of the prose.
const MAX_MEANING_LINKS: usize = 12;

/// The compiled/cached rule payload a PROVEN node projects to — the live path's "query plan", byte-for-
/// byte the [`crate::linter::LearnedRule`] the graduation emitted (plus its source cite). The source of
/// truth is the node; this is the derived VIEW the fast path fires.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebRule {
    /// The rule id (`uses-<construct>`), byte-preserved.
    pub id: String,
    /// Severity bucket.
    pub severity: String,
    /// The English understanding / advice shown.
    pub description: String,
    /// A harvested violating block (illustration; the plan rides `construct`).
    pub bad: String,
    /// A clean near-miss block (illustration).
    pub good: String,
    /// The doc URL cited by the finding.
    pub source: String,
}

/// The GRADED (LOW-severity) firing form a REVOKED-role READ node graduated to (COMPLETION PASS 27) — the
/// evidence-graded tier that replaces abstention ("a linter that doesn't do anything isn't a linter"). It
/// is `Some` on an UNPROVEN node iff, at train time, the node carried a revoked doc-role (deprecated /
/// removal / prohibition), was QUALIFIED-SAFE (a real `owner.member`, never a URL basename or rustdoc
/// anchor form), was NOT already covered by a proven rule, AND a flood-safe firing form survived the
/// member-scope usage-death (calibrated against the corpus's own candidate distribution) + clean-near-miss
/// gates ([`crate::lint_module::graded_forms`]). It fires at LOW severity, its message citing the attested
/// deprecation. NEVER present on a proven node (already enforced) and NEVER on a read-roleless node (no
/// revoked fact to grade). Persisted on the node so the gates are computed once, at train time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GradedForm {
    /// The construct token the LOW rule fires `uses_construct` on — either the receiver-generic `.member`
    /// (when the member is USAGE-DEAD in the corpus's own other-page example code AND the corpus's death
    /// verdicts are calibrated, so it flags real `x.member` usage with high recall) or the dotted-literal
    /// `owner.member` (when the member is alive, or the corpus never witnessed a live member — safe, low
    /// recall, fires only on the exact deprecated static text so a live remedy like
    /// `collections.abc.Sequence` is never flagged for `typing.Sequence`).
    pub fire: String,
    /// LOW — the evidence-graded tier. Proven rules stay `medium`; graded findings are always `low`.
    pub severity: String,
    /// The evidence message shown — a prohibition citing the attested deprecation and the usage-death
    /// basis (the RUNG's "documented deprecated ⟨cite⟩" + the corpus-death count).
    pub description: String,
    /// The doc URL the finding cites.
    pub source: String,
}

/// A source's sentence that DENIES a role this web proved (PASS 30 — the self-referee's contradiction
/// record). First-class signal: one side is wrong, and resolving which is learning — so it is persisted
/// on the node, surfaced by `lint_query kind=web`, and it withholds the node's graded form (the
/// evidence-graded tier requires uncontradicted evidence).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contradiction {
    /// The page whose sentence disagrees with the web's proven role.
    pub source: String,
    /// The disagreeing sentence (head-capped at persist time).
    pub sentence: String,
}

/// The SELF-REFEREE verdicts a revoked-role node accumulated at train time (PASS 30): the machine's own
/// web judging every OTHER source's claim about the construct. Coherence = independent corroboration
/// (distinct source URLs whose own sentence asserts the same revocation); contradiction = a source
/// denying it. Both capped; `None` on a node no other source speaks about (the honest sparse state —
/// MEASURED starting point on the python corpora: 2 coherent, 0 contradictions across 208 revoked nodes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Corroboration {
    /// Distinct OTHER-source URLs asserting the node's revoked role.
    pub coherent: Vec<String>,
    /// Sources denying the node's revoked role — the contradiction records.
    pub contradictions: Vec<Contradiction>,
}

/// One construct the language READ — a node in its understanding subgraph. Proven nodes are the rules
/// (via [`WebRule`]); unproven nodes are retained knowledge (present, never enforced) — EXCEPT an unproven
/// node carrying a [`GradedForm`], which fires the evidence-graded LOW tier (PASS 27).
#[derive(Clone, Debug)]
pub struct ConstructNode {
    /// The construct token — the node id, byte-preserved (`var`, `document.write`, `cgi`).
    pub construct: String,
    /// The governing prose: the understanding sentence first, the advice sentence next when one was
    /// derived. Verbatim doc prose — the language's own words that govern the construct.
    pub governing: Vec<String>,
    /// The MEANING LINKS — the distinctive content key-words of the governing prose, keys into the
    /// FROZEN English web. Their meaning is rebound on query, never copied here.
    pub meaning_links: Vec<String>,
    /// The doc source(s) the construct was read from — the finding's citation.
    pub sources: Vec<String>,
    /// Whether the origin STRUCTURALLY ATTESTS the construct deprecated (author-metadata attestation —
    /// [`crate::lint_attest`]).
    pub attested_deprecated: bool,
    /// The DOC-ROLE facts the proven faculties attest for this construct — first-class TRAVERSAL TARGETS
    /// (PASS 25). Each entry is a faculty's OWN proven fact, never a word list: `"deprecated"` (the
    /// author-metadata attestation family, from `attested_deprecated`) and/or `"removal"` /
    /// `"prohibition"` (the KIND of the proven construction that consumed this subject —
    /// [`crate::lint_construct::ConstructionKind::label`]). A web query "what connects to REMOVAL" filters
    /// on these; the removal role is the SPECIFIC fact, deprecation the umbrella.
    pub roles: Vec<String>,
    /// PROVEN (enforced — carries a rule) or merely READ (retained, unproven, never fired).
    pub proven: bool,
    /// The compiled rule VIEW — `Some` iff `proven`, byte-identical to the emitted `(rule, source)`.
    pub rule: Option<WebRule>,
    /// The GRADED (LOW-severity) firing form (PASS 27) — `Some` only on an UNPROVEN, revoked-role node that
    /// passed the train-time safety gates. Fires the evidence-graded tier; proven nodes and read-roleless
    /// nodes carry `None` and never fire from here.
    pub graded: Option<GradedForm>,
    /// The SELF-REFEREE record (PASS 30) — `Some` iff other sources spoke about this revoked-role
    /// construct at train time (corroborating sources and/or contradictions).
    pub referee: Option<Corroboration>,
    /// PASS 35 — the SUPERSESSION edge (owner ruling: supersession is NOT prohibition — no negation,
    /// no template): read by the meaning net from the node's own governing prose. `None` = no
    /// replacement stated (honest). The successor is a LIVING construct the sentence names in the
    /// author's own code typography.
    pub superseded_by: Option<Succession>,
}

/// A documented replacement relation: the node's construct is superseded by `successor`, stated by the
/// docs' own `sentence` (the citation the improvement tier surfaces).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Succession {
    /// The living construct the docs name as the replacement.
    pub successor: String,
    /// The docs' own sentence stating the relation — the finding's citation text.
    pub sentence: String,
}

impl crate::lint_codec::Bin for WebRule {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.str(&self.id);
        e.str(&self.severity);
        e.str(&self.description);
        e.str(&self.bad);
        e.str(&self.good);
        e.str(&self.source);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<WebRule> {
        Some(WebRule {
            id: d.str()?,
            severity: d.str()?,
            description: d.str()?,
            bad: d.str()?,
            good: d.str()?,
            source: d.str()?,
        })
    }
}

impl crate::lint_codec::Bin for GradedForm {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.str(&self.fire);
        e.str(&self.severity);
        e.str(&self.description);
        e.str(&self.source);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<GradedForm> {
        Some(GradedForm { fire: d.str()?, severity: d.str()?, description: d.str()?, source: d.str()? })
    }
}

impl crate::lint_codec::Bin for ConstructNode {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.str(&self.construct);
        self.governing.enc(e);
        self.meaning_links.enc(e);
        self.sources.enc(e);
        e.boolean(self.attested_deprecated);
        self.roles.enc(e);
        e.boolean(self.proven);
        e.boolean(self.rule.is_some());
        if let Some(r) = &self.rule {
            r.enc(e);
        }
        // PASS 27 — the graded form rides at the END, stamp-gated and bounds-safe: an old-format node
        // (written before graded existed) simply lacks these bytes, and `dec` reads the trailing presence
        // flag as `None` (`d.boolean()` past the buffer returns `None` → `unwrap_or_default` → not graded),
        // so an unmigrated web decodes to zero graded findings and rebuilds on the next train.
        e.boolean(self.graded.is_some());
        if let Some(g) = &self.graded {
            g.enc(e);
        }
        // PASS 30 — the self-referee record rides after the graded form, same trailing bounds-safe shape.
        e.boolean(self.referee.is_some());
        if let Some(r) = &self.referee {
            r.coherent.enc(e);
            (r.contradictions.iter().map(|c| c.source.clone()).collect::<Vec<_>>()).enc(e);
            (r.contradictions.iter().map(|c| c.sentence.clone()).collect::<Vec<_>>()).enc(e);
        }
        // PASS 35 — the succession edge rides last, same trailing bounds-safe shape.
        e.boolean(self.superseded_by.is_some());
        if let Some(s) = &self.superseded_by {
            e.str(&s.successor);
            e.str(&s.sentence);
        }
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<ConstructNode> {
        let construct = d.str()?;
        let governing = <Vec<String> as crate::lint_codec::Bin>::dec(d)?;
        let meaning_links = <Vec<String> as crate::lint_codec::Bin>::dec(d)?;
        let sources = <Vec<String> as crate::lint_codec::Bin>::dec(d)?;
        let attested_deprecated = d.boolean()?;
        let roles = <Vec<String> as crate::lint_codec::Bin>::dec(d)?;
        let proven = d.boolean()?;
        let has_rule = d.boolean()?;
        let rule = if has_rule { Some(WebRule::dec(d)?) } else { None };
        // Trailing, back-compatible: absent bytes (old format) read as not-graded rather than failing.
        let graded = match d.boolean() {
            Some(true) => Some(GradedForm::dec(d)?),
            _ => None,
        };
        // PASS 30 trailing referee record — same back-compatible read.
        let referee = match d.boolean() {
            Some(true) => {
                let coherent = <Vec<String> as crate::lint_codec::Bin>::dec(d)?;
                let srcs = <Vec<String> as crate::lint_codec::Bin>::dec(d)?;
                let sents = <Vec<String> as crate::lint_codec::Bin>::dec(d)?;
                let contradictions = srcs
                    .into_iter()
                    .zip(sents)
                    .map(|(source, sentence)| Contradiction { source, sentence })
                    .collect();
                Some(Corroboration { coherent, contradictions })
            }
            _ => None,
        };
        // PASS 35 trailing succession edge — same back-compatible read.
        let superseded_by = match d.boolean() {
            Some(true) => Some(Succession { successor: d.str()?, sentence: d.str()? }),
            _ => None,
        };
        Some(ConstructNode { construct, governing, meaning_links, sources, attested_deprecated, roles, proven, rule, graded, referee, superseded_by })
    }
}

/// The distinctive content KEY-WORDS of `prose` — the meaning links into the frozen English web. A word
/// is a link iff the frozen brain KNOWS it ([`MeaningNetwork::has_meaning`]); the links are ranked by the
/// brain's inverse-document-frequency CENTRALITY (the distinctive words the sentence is about carry the
/// sense; the filler every sentence shares weighs ~1) and capped at [`MAX_MEANING_LINKS`]. Pure over the
/// frozen brain — no word list, no hand gloss; the same centrality every meaning bundle already weighs by.
pub fn meaning_links(m: &MeaningNetwork, prose: &str) -> Vec<String> {
    let mut scored: Vec<(u32, String)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for raw in prose.split(|c: char| !c.is_alphanumeric()) {
        let w = raw.to_lowercase();
        if w.len() < 2 || !m.has(&w) || !seen.insert(w.clone()) {
            continue;
        }
        scored.push((m.centrality(&w), w));
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    scored.into_iter().take(MAX_MEANING_LINKS).map(|(_, w)| w).collect()
}

/// The node's bundled MEANING hypervector — its meaning links each rebound through the frozen English web
/// ([`MeaningNetwork::meaning_of`]) and majority-bundled into one vector. This is what makes cross-language
/// relations a QUERY: two nodes are close when their governing prose shares distinctive meaning, computed
/// against the SAME frozen base for every language. `None` when no link rebinds (an all-jargon prose).
pub fn node_meaning(m: &MeaningNetwork, node: &ConstructNode) -> Option<crate::lint_ai::Hv> {
    let mut b = crate::lint_ai::Bundler::new();
    let mut any = false;
    for w in &node.meaning_links {
        if let Some(hv) = m.meaning_of(w) {
            b.add(&hv);
            any = true;
        }
    }
    any.then(|| b.finalize())
}

/// The Hamming proximity of two construct nodes' governing-prose meanings — the cross-language relation
/// query (lower = more related). `None` when either node's prose does not rebind. Symmetric, pure over the
/// frozen brain. `class` in JS and `class` in Python are distinct nodes; this measures whether their prose
/// MEANS something related, never conflating the constructs themselves.
pub fn relate(m: &MeaningNetwork, a: &ConstructNode, b: &ConstructNode) -> Option<u32> {
    Some(node_meaning(m, a)?.distance(&node_meaning(m, b)?))
}

/// The DOC-ROLES a node carries, from the proven faculties' own facts (PASS 25 rung 2) — never a word
/// list. `attested_deprecated` contributes the author-metadata attestation family role `"deprecated"`
/// (the umbrella); `roles_by_construct` carries the SPECIFIC construction kind
/// ([`crate::lint_construct::subject_roles`], `"removal"`/`"prohibition"`) for a construction-consumed
/// subject. Order is deterministic (specific first, then the umbrella) so the encoding is stable.
fn node_roles(construct: &str, attested_deprecated: bool, roles_by_construct: &HashMap<String, String>) -> Vec<String> {
    let mut roles: Vec<String> = Vec::new();
    if let Some(r) = roles_by_construct.get(construct) {
        roles.push(r.clone());
    }
    if attested_deprecated && !roles.iter().any(|r| r == "deprecated") {
        roles.push("deprecated".to_string());
    }
    roles
}

/// The meaning links bundled from one or more governing sentences, deduped and capped — the frozen brain's
/// distinctive content key-words. Shared by the proven (outcome) and READ (everything-read) node builders.
fn links_of(m: &MeaningNetwork, governing: &[String]) -> Vec<String> {
    let mut links: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for sent in governing {
        for w in meaning_links(m, sent) {
            if seen.insert(w.clone()) {
                links.push(w);
            }
        }
    }
    links.truncate(MAX_MEANING_LINKS);
    links
}

/// Build a language's web subgraph — EVERYTHING READ, retained (PASS 25 rung 1). The pass's `outcomes`
/// become nodes exactly as before (a proven outcome → a PROVEN node carrying its rule view; a proposed
/// candidate that did not graduate → a READ node), and the `read_surface` — every construct the reading
/// layer extracted that was NEVER proposed ([`crate::lint_module::ReadConstruct`]) — is appended as
/// retained-UNPROVEN nodes (coverage = everything-read, enforcement = the proven subset). `roles_by_construct`
/// keys each node's doc-role facts (rung 2). The governing prose is the candidate's understanding (plus its
/// advice when the emitted rule's description differs); meaning links are extracted through the frozen brain.
/// Node order follows outcome order, THEN read-surface order, so [`derive_rules`] (proven nodes only)
/// reproduces the live rule order byte-identically — the read nodes carry no rule and never perturb it.
/// `graded_by_construct` (PASS 27) carries the train-time-computed [`GradedForm`] for each UNPROVEN
/// revoked-role construct that passed the safety gates ([`crate::lint_module::graded_forms`]); a read node
/// whose construct is a key fires the evidence-graded LOW tier, every other node stays abstaining as before.
/// Proven nodes never take a graded form (they already enforce). Empty map ⇒ byte-identical to PASS 25.
/// PASS 35 — the SUCCESSION READ (owner ruling: "use the AI", never a sentence scaffold). A governing
/// sentence states a replacement iff some word of it CARRIES THE REPLACE MEANING — it is the anchor
/// (`deprecation-status.json` → `replacement`, the one-datum covenant), a morphological form of it
/// (`replaced`/`replaces`/`replacement`/`replacing`), or a word the dictionary DEFINES via the anchor
/// with the dictionary's own `verb` POS typography (the PASS-29 register law, verb-gated). The
/// SUCCESSOR is the sentence's own code-typography token (backticked by the author) that is a LIVING
/// subject of the corpus and not the node's own construct — the docs name their replacement in code,
/// so no prose parsing is ever guessed. Returns the first such relation, or `None` (honest).
fn succession_of(
    m: &MeaningNetwork,
    construct: &str,
    source_url: &str,
    node_revoked: bool,
    governing: &[String],
    living_names: &std::collections::HashSet<String>,
) -> Option<Succession> {
    let anchors = crate::lint_attest::replacement_tokens();
    if anchors.is_empty() {
        return None;
    }
    let own_terminal = construct.trim_start_matches('.').rsplit('.').next().unwrap_or(construct);
    // The page's OWNER segment (`/Animation/persist` → `animation`) is the subject's interface, never
    // its successor — measured: `persist → Animation` minted from the owner naming itself.
    let owner_segment = {
        let t = source_url.trim_end_matches('/');
        let mut it = t.rsplit('/');
        it.next();
        it.next().map(|s| s.to_lowercase()).unwrap_or_default()
    };
    for sentence in governing {
        // PROSE ONLY: code-typography spans are tokens, not words — `Symbol.replace` naming itself
        // must never read as the replace MEANING (measured junk class).
        let prose: String = {
            let mut out = String::with_capacity(sentence.len());
            let mut in_code = false;
            for c in sentence.chars() {
                if c == '`' {
                    in_code = !in_code;
                    out.push(' ');
                } else if !in_code {
                    out.push(c);
                }
            }
            out
        };
        let lower = prose.to_lowercase();
        let states = lower
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .filter(|w| w.len() >= anchors.iter().map(|a| a.len()).min().unwrap_or(2))
            .any(|w| word_carries_replace_meaning(w, &anchors, m));
        if !states {
            continue;
        }
        // VERIFICATION (understanding proposes, the claim proves — owner's north star): the edge
        // holds only when the docs REVOKE the subject alongside the replacement — the node already
        // carries the attested/revoked role, or THIS sentence itself asserts revocation (the PASS-30
        // claim atom: prohibition anchor present, not negated). "changes the contents of an array"
        // states change, not succession — measured, it minted `.splice → Array` without this.
        let asserts_revocation = node_revoked || {
            let negated = crate::lint_english::brain()
                .is_some_and(|en| crate::lint_corroborate::is_negated(en, &prose));
            let mut prohibition = crate::lint_attest::prohibition_class_tokens();
            prohibition.extend(crate::lint_attest::removal_class_tokens());
            matches!(
                crate::lint_corroborate::revocation_claim(&lower, negated, &prohibition),
                crate::lint_corroborate::RevocationClaim::Asserts
            )
        };
        if !asserts_revocation {
            continue;
        }
        // The author's own code typography: backtick-quoted tokens, in sentence order.
        let mut rest = sentence.as_str();
        while let Some(open) = rest.find('`') {
            let after = &rest[open + 1..];
            let Some(close) = after.find('`') else { break };
            let token = crate::lint_lang_layer::normalize_construct(&after[..close]);
            rest = &after[close + 1..];
            let terminal = token.trim_start_matches('.').rsplit('.').next().unwrap_or(&token);
            if token.len() >= 2
                && terminal != own_terminal
                && !terminal.is_empty()
                && terminal.to_lowercase() != owner_segment
                && living_names.contains(terminal)
            {
                return Some(Succession { successor: token, sentence: sentence.clone() });
            }
        }
    }
    None
}

/// Whether `word` carries the REPLACE meaning — the PASS-29 register law, nothing hand-listed:
/// a candidate STEM of the word (mechanical prefix cuts, silent-`e` restored — candidates only, never
/// verdicts) counts iff the DICTIONARY ITSELF says so: the stem IS the anchor, or the stem is a real
/// dictionary headword whose own entry opens as a VERB (the dictionary's POS typography) and defines
/// via the anchor ("substitute", "supersede" join by their own definitions; a noun whose long entry
/// merely mentions the anchor does not). The dictionary is the sole referee; the mechanical cuts only
/// generate candidates for it to judge — exactly [`gerund_of_known_verb`]'s covenant.
fn word_carries_replace_meaning(word: &str, anchors: &[String], m: &MeaningNetwork) -> bool {
    if anchors.iter().any(|a| a == word) {
        return true;
    }
    let mut candidates: Vec<String> = vec![word.to_string()];
    for cut in 1..=4usize {
        if word.len() > cut + 2 {
            let b = &word[..word.len() - cut];
            candidates.push(b.to_string());
            candidates.push(format!("{b}e"));
        }
    }
    candidates.iter().any(|c| {
        let is_anchor = anchors.iter().any(|a| a == c);
        let defs = m.definition_words(c);
        let verb_headword = defs.is_some_and(|d| d.iter().take(8).any(|w| w == "verb"));
        let defines_via_anchor =
            defs.is_some_and(|d| d.iter().any(|w| anchors.iter().any(|a| a == w)));
        // An inflected stem must be a REAL verb headword (the dictionary vouches for the cut);
        // a distinct word joins only when its own entry defines it via the anchor as a verb.
        (is_anchor && (c == word || verb_headword)) || (verb_headword && defines_via_anchor)
    })
}

pub fn build(
    m: &MeaningNetwork,
    living_names: &std::collections::HashSet<String>,
    outcomes: &[crate::lint_module::Outcome],
    read_surface: &[crate::lint_module::ReadConstruct],
    roles_by_construct: &HashMap<String, String>,
    graded_by_construct: &HashMap<String, GradedForm>,
    referee_by_construct: &HashMap<String, Corroboration>,
) -> Vec<ConstructNode> {
    let mut nodes = Vec::with_capacity(outcomes.len() + read_surface.len());
    let mut have: HashSet<String> = HashSet::new();
    for o in outcomes {
        let mut governing = vec![o.candidate.understanding.clone()];
        let mut sources = vec![o.candidate.url.clone()];
        let (proven, rule) = match &o.rule {
            Some((r, url)) => {
                // The emitted rule's description is the graduated understanding; carry it as governing
                // prose when it adds a second sentence (the advice the pair reconciled on).
                if r.description != o.candidate.understanding {
                    governing.push(r.description.clone());
                }
                if !sources.contains(url) {
                    sources.push(url.clone());
                }
                (
                    true,
                    Some(WebRule {
                        id: r.id.clone(),
                        severity: r.severity.clone(),
                        description: r.description.clone(),
                        bad: r.bad.clone(),
                        good: r.good.clone(),
                        source: url.clone(),
                    }),
                )
            }
            None => (false, None),
        };
        have.insert(o.candidate.construct.clone());
        nodes.push(ConstructNode {
            meaning_links: links_of(m, &governing),
            roles: node_roles(&o.candidate.construct, o.candidate.attested_deprecated, roles_by_construct),
            superseded_by: succession_of(m, &o.candidate.construct, &o.candidate.url, o.candidate.attested_deprecated, &governing, living_names),
            construct: o.candidate.construct.clone(),
            governing,
            sources,
            attested_deprecated: o.candidate.attested_deprecated,
            proven,
            rule,
            graded: None, // a proven node enforces via its rule; it never takes the graded tier
            referee: referee_by_construct.get(&o.candidate.construct).cloned(),
        });
    }
    // EVERYTHING READ. Every construct the reader saw but the funnel never proposed enters as a retained
    // UNPROVEN node — knowledge, queryable, cross-linkable, never fired. Deduped against the proposed set
    // (a proposed construct is already a node) and against itself (one node per construct token).
    for r in read_surface {
        if !have.insert(r.construct.clone()) {
            continue;
        }
        let governing = vec![r.governing.clone()];
        nodes.push(ConstructNode {
            meaning_links: links_of(m, &governing),
            roles: node_roles(&r.construct, r.attested_deprecated, roles_by_construct),
            graded: graded_by_construct.get(&r.construct).cloned(),
            referee: referee_by_construct.get(&r.construct).cloned(),
            superseded_by: succession_of(m, &r.construct, &r.url, r.attested_deprecated, &governing, living_names),
            construct: r.construct.clone(),
            governing,
            sources: vec![r.url.clone()],
            attested_deprecated: r.attested_deprecated,
            proven: false,
            rule: None,
        });
    }
    nodes
}

/// DERIVE `lang`'s rules as a VIEW over the web: exactly the PROVEN nodes' rule payloads, in node order,
/// as the `(LearnedRule, source)` pairs the module build consumes. The web is the source of truth; this is
/// the projection the fast live path fires. Byte-identical to the graduation's own emitted set (the proven
/// nodes carry those exact rules), so re-deriving from a round-tripped web reproduces them.
pub fn derive_rules(lang: &str, web: &[ConstructNode]) -> Vec<(crate::linter::LearnedRule, String)> {
    web.iter()
        .filter_map(|n| {
            let r = n.rule.as_ref()?;
            Some((
                crate::linter::LearnedRule {
                    language: lang.to_string(),
                    id: r.id.clone(),
                    severity: r.severity.clone(),
                    description: r.description.clone(),
                    bad: r.bad.clone(),
                    good: r.good.clone(),
                    construct: Some(n.construct.clone()),
                },
                r.source.clone(),
            ))
        })
        .collect()
}

/// DERIVE `lang`'s GRADED (LOW-severity) rules as a VIEW over the web (PASS 27): every UNPROVEN node
/// carrying a [`GradedForm`], as `(LearnedRule, source)` pairs the module build compiles into a firing
/// `uses_construct(fire)` detector. The rule id is `graded-<construct>` (never colliding with a proven
/// `uses-<construct>`); `bad`/`good` are empty (the detector rides the plan, not an example diff). Appended
/// AFTER the proven rules by the caller, so the proven set and its order stay byte-identical.
pub fn derive_graded_rules(lang: &str, web: &[ConstructNode]) -> Vec<(crate::linter::LearnedRule, String)> {
    web.iter()
        .filter_map(|n| {
            let g = n.graded.as_ref()?;
            Some((
                crate::linter::LearnedRule {
                    language: lang.to_string(),
                    id: format!("graded-{}", n.construct),
                    severity: g.severity.clone(),
                    description: g.description.clone(),
                    bad: String::new(),
                    good: String::new(),
                    construct: Some(g.fire.clone()),
                },
                g.source.clone(),
            ))
        })
        .collect()
}

/// The construct nodes whose governing prose or attestation CONNECTS to `concept` — a web query. A node
/// connects when one of its meaning links IS the concept word, or its bundled meaning sits within
/// `radius` Hamming of the concept's own meaning (the frozen brain's [`MeaningNetwork::related`] geometry).
/// Pure read; the interpretation is a QUERY over the graph, never a stored label.
pub fn nodes_connecting(m: &MeaningNetwork, web: &[ConstructNode], concept: &str, radius: u32) -> Vec<String> {
    let low = concept.to_lowercase();
    web.iter()
        .filter(|n| {
            n.meaning_links.iter().any(|w| w == &low)
                || n.meaning_links.iter().any(|w| m.related(w, &low) <= radius)
        })
        .map(|n| n.construct.clone())
        .collect()
}

/// The construct nodes in `web` that carry doc-role `role` (PASS 25 rung 2) — a first-class TRAVERSAL over
/// the faculties' own proven facts, case-insensitive. `nodes_with_role(web, "removal")` returns exactly the
/// subjects a Removal construction proved (cgi, telnetlib, …); `"deprecated"` returns the whole
/// author-metadata-attested family. Pure read — the roles are stored proven facts, never re-judged here.
pub fn nodes_with_role(web: &[ConstructNode], role: &str) -> Vec<String> {
    let low = role.to_lowercase();
    web.iter().filter(|n| n.roles.iter().any(|r| r == &low)).map(|n| n.construct.clone()).collect()
}

/// Every construct carrying doc-role `role`, ACROSS every language web on this machine (PASS 25 rung 2) —
/// the cross-language doc-role query: "what connects to REMOVAL" reaches every language's removed subjects
/// through the faculties' shared role vocabulary, regardless of what its prose's index-words say. Loads
/// each language sidecar (the assembly unit) and returns `(language, construct)` pairs, sorted. Pure read.
pub fn roles_across(role: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for lang in languages_with_web() {
        for construct in nodes_with_role(&load(&lang), role) {
            out.push((lang.clone(), construct));
        }
    }
    out.sort();
    out
}

// ── persistence — the per-language web sidecar, delta-stored, retain-and-grow ────────────────────────

/// The per-language web subgraph artifact path (`<lang>.web.bin`, beside the module, the graduated
/// ledger, and the constructions). A SEPARATE sidecar so it survives — and is loaded independently of —
/// the module rebuild (the isolation covenant: a project loads only the webs its languages need).
fn web_path(lang: &str) -> std::path::PathBuf {
    crate::lint_train::model_dir_pub().join(format!("{lang}.web.bin"))
}

/// The web subgraph PERSISTED in past retrains for `lang`, or empty when none is on disk. Stamped with
/// the [`crate::lint_train::train_version`] it was written under and DISCARDED on a mismatch (a semantic
/// bump may change node/rule semantics). Never trains; a pure read — the isolation unit is the per-
/// language file, so loading `lang`'s web never touches another language's.
pub fn load(lang: &str) -> Vec<ConstructNode> {
    let Ok(bytes) = std::fs::read(web_path(lang)) else {
        return Vec::new();
    };
    let Some((stamp, mut d)) = crate::lint_codec::Dec::open(&bytes, crate::lint_codec::kind::WEB) else {
        return Vec::new();
    };
    if stamp != crate::lint_train::train_version() {
        return Vec::new();
    }
    <Vec<ConstructNode> as crate::lint_codec::Bin>::dec(&mut d).unwrap_or_default()
}

/// Persist `lang`'s web subgraph, retain-and-grow: written ONLY when there are nodes, so a subset crawl
/// that reads nothing never wipes a prior web (mirrors [`crate::lint_construct::persist`] and the
/// graduated ledger). Stamped with the current train version; refuses a train-ordinal regression
/// ([`crate::lint_train::stamp_regression`] — an outlived process keeps the newer store).
pub fn persist(lang: &str, web: &[ConstructNode]) {
    if web.is_empty() || crate::lint_train::stamp_regression(&web_path(lang), crate::lint_train::train_version()) {
        return;
    }
    let mut e = crate::lint_codec::Enc::new();
    <Vec<ConstructNode> as crate::lint_codec::Bin>::enc(&web.to_vec(), &mut e);
    let bytes = e.finish(crate::lint_codec::kind::WEB, crate::lint_train::train_version());
    if let Some(parent) = web_path(lang).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(web_path(lang), bytes);
}

/// Every language that has a persisted web on this machine — the basenames of the `*.web.bin` sidecars in
/// the model directory. The assembly unit: a cross-language traversal query loads exactly these, and a
/// project loads only the subset its files need.
pub fn languages_with_web() -> Vec<String> {
    let dir = crate::lint_train::model_dir_pub();
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            if let Some(name) = e.file_name().to_str().and_then(|n| n.strip_suffix(".web.bin")) {
                out.push(name.to_string());
            }
        }
    }
    out.sort();
    out
}

/// A cross-language CONNECTION found by traversing the shared English base: `from`'s node in `from_lang`
/// relates to `node`'s construct in `lang` by governing-prose meaning proximity `distance` (lower = more
/// related). The traversal is a QUERY over the frozen web — `class` in two languages stays two distinct
/// nodes; this reports whether their prose MEANS something related.
#[derive(Clone, Debug)]
pub struct CrossLink {
    pub lang: String,
    pub construct: String,
    pub distance: u32,
    pub shared_links: Vec<String>,
}

/// Traverse from one construct node to the NEAREST constructs in OTHER languages' webs, ranked by
/// governing-prose meaning proximity through the shared English base. Excludes `from_lang` itself (this is
/// the CROSS-language query). `top` bounds the result. Pure over the frozen brain and the loaded webs.
pub fn cross_language(m: &MeaningNetwork, from_lang: &str, from: &ConstructNode, top: usize) -> Vec<CrossLink> {
    let Some(_) = node_meaning(m, from) else { return Vec::new() };
    let from_links: HashSet<&str> = from.meaning_links.iter().map(String::as_str).collect();
    let mut out: Vec<CrossLink> = Vec::new();
    for lang in languages_with_web() {
        if lang == from_lang {
            continue;
        }
        for node in load(&lang) {
            if let Some(dist) = relate(m, from, &node) {
                let shared: Vec<String> =
                    node.meaning_links.iter().filter(|w| from_links.contains(w.as_str())).cloned().collect();
                out.push(CrossLink { lang: lang.clone(), construct: node.construct.clone(), distance: dist, shared_links: shared });
            }
        }
    }
    out.sort_by(|a, b| a.distance.cmp(&b.distance).then_with(|| a.construct.cmp(&b.construct)));
    out.truncate(top);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint_codec::Bin;

    fn proven(construct: &str, understanding: &str, url: &str) -> ConstructNode {
        ConstructNode {
            construct: construct.to_string(),
            governing: vec![understanding.to_string()],
            meaning_links: vec!["never".to_string(), "keyword".to_string()],
            sources: vec![url.to_string()],
            attested_deprecated: false,
            roles: Vec::new(),
            proven: true,
            rule: Some(WebRule {
                id: format!("uses-{construct}"),
                severity: "medium".to_string(),
                description: understanding.to_string(),
                bad: format!("{construct} x = 1;"),
                good: "let x = 1;".to_string(),
                source: url.to_string(),
            }),
            graded: None,
            referee: None,
            superseded_by: None,
        }
    }

    fn read_only(construct: &str) -> ConstructNode {
        ConstructNode {
            construct: construct.to_string(),
            governing: vec![format!("The {construct} keyword is documented.")],
            meaning_links: vec!["keyword".to_string()],
            sources: vec!["u".to_string()],
            attested_deprecated: false,
            roles: Vec::new(),
            proven: false,
            rule: None,
            graded: None,
            referee: None,
            superseded_by: None,
        }
    }

    /// The web ROUND-TRIPS byte-identical through the codec, and the rules DERIVED from the round-tripped
    /// web reproduce exactly the proven nodes' rule payloads in order (the source-of-truth contract:
    /// delete the web, re-read it, re-derive — the same rules). An unproven READ node contributes NO rule.
    #[test]
    fn round_trips_and_derives() {
        let mut refereed = read_only("if");
        refereed.referee = Some(Corroboration {
            coherent: vec!["https://docs/other-page".to_string()],
            contradictions: vec![Contradiction {
                source: "https://docs/dissenter".to_string(),
                sentence: "if is not deprecated".to_string(),
            }],
        });
        let web = vec![
            proven("var", "Never use the var keyword.", "https://docs/no-var"),
            refereed, // retained, unproven — present but never a rule; carries a PASS-30 referee record
            proven("eval", "Disallow the eval function.", "https://docs/no-eval"),
        ];
        let mut e = crate::lint_codec::Enc::new();
        web.clone().enc(&mut e);
        let bytes = e.finish(crate::lint_codec::kind::WEB, "test-stamp");
        let (stamp, mut d) = crate::lint_codec::Dec::open(&bytes, crate::lint_codec::kind::WEB).expect("opens");
        assert_eq!(stamp, "test-stamp");
        let back = <Vec<ConstructNode> as Bin>::dec(&mut d).expect("decodes");
        assert_eq!(back.len(), 3);
        // PASS 30 — the referee record round-trips; the un-refereed nodes stay None.
        let r = back[1].referee.as_ref().expect("referee record survives the codec");
        assert_eq!(r.coherent, vec!["https://docs/other-page".to_string()]);
        assert_eq!(r.contradictions.len(), 1);
        assert_eq!(r.contradictions[0].source, "https://docs/dissenter");
        assert!(back[0].referee.is_none() && back[2].referee.is_none());

        // Rules are a VIEW: exactly the proven nodes, in order, byte-identical to the emitted set.
        let derived = derive_rules("javascript", &back);
        assert_eq!(derived.len(), 2, "only the two proven nodes are rules; the READ node is retained-only");
        assert_eq!(derived[0].0.id, "uses-var");
        assert_eq!(derived[0].0.construct.as_deref(), Some("var"));
        assert_eq!(derived[0].1, "https://docs/no-var");
        assert_eq!(derived[1].0.id, "uses-eval");
        // Deriving from the ORIGINAL web equals deriving from the round-tripped web (round-trip is total).
        assert_eq!(derive_rules("javascript", &web), derived, "re-deriving from the round-tripped web reproduces the rules");
    }

    /// Everything READ is retained: an unproven construct is a node in the web (queryable) but never
    /// enforced — coverage is everything-read, enforcement is the proven subset.
    #[test]
    fn read_nodes_are_retained_but_never_enforced() {
        let web = vec![proven("var", "Never use var.", "u"), read_only("const"), read_only("if")];
        assert_eq!(web.iter().filter(|n| !n.proven).count(), 2, "two retained-unproven nodes");
        assert_eq!(derive_rules("javascript", &web).len(), 1, "only the proven node enforces");
    }

    /// Doc-role facts are first-class TRAVERSAL TARGETS (PASS 25 rung 2): a node carries the faculties'
    /// own proven roles, they SURVIVE the codec round-trip, and a role query returns exactly the carriers —
    /// per-web and (the cross-language intent) filtered by role, never by prose index-words.
    #[test]
    fn doc_roles_round_trip_and_query() {
        let mut cgi = read_only("cgi");
        cgi.roles = vec!["removal".to_string(), "deprecated".to_string()];
        cgi.attested_deprecated = true;
        let mut codecs = read_only("codecs.open");
        codecs.roles = vec!["deprecated".to_string()];
        let web = vec![proven("var", "Never use var.", "u"), cgi, codecs];

        // The roles survive the codec round-trip.
        let mut e = crate::lint_codec::Enc::new();
        web.clone().enc(&mut e);
        let bytes = e.finish(crate::lint_codec::kind::WEB, "test-stamp");
        let (_stamp, mut d) = crate::lint_codec::Dec::open(&bytes, crate::lint_codec::kind::WEB).expect("opens");
        let back = <Vec<ConstructNode> as Bin>::dec(&mut d).expect("decodes");
        assert_eq!(back.iter().find(|n| n.construct == "cgi").unwrap().roles, vec!["removal", "deprecated"]);

        // Role query: REMOVAL reaches only the removed subject; DEPRECATED reaches the whole family.
        assert_eq!(nodes_with_role(&back, "removal"), vec!["cgi"]);
        assert_eq!(nodes_with_role(&back, "REMOVAL"), vec!["cgi"], "case-insensitive");
        let dep = nodes_with_role(&back, "deprecated");
        assert!(dep.contains(&"cgi".to_string()) && dep.contains(&"codecs.open".to_string()));
        assert!(!dep.contains(&"var".to_string()), "var carries no doc-role");
    }

    /// `node_roles` derives from the faculties' facts only: the construction kind (specific) then the
    /// attestation umbrella, never a word list — and never duplicates the umbrella.
    #[test]
    fn node_roles_are_specific_then_umbrella_no_duplicate() {
        let mut roles = HashMap::new();
        roles.insert("cgi".to_string(), "removal".to_string());
        assert_eq!(node_roles("cgi", true, &roles), vec!["removal", "deprecated"]);
        assert_eq!(node_roles("codecs.open", true, &roles), vec!["deprecated"]);
        assert_eq!(node_roles("var", false, &roles), Vec::<String>::new());
    }

    /// PASS 27 — a graded READ node SURVIVES the codec round-trip and DERIVES a LOW-severity rule that
    /// fires its `fire` form, while the proven-rule view stays unmoved (graded rules are a SEPARATE tier).
    #[test]
    fn graded_forms_round_trip_and_derive_a_low_rule() {
        let mut blink = read_only("String.blink");
        blink.roles = vec!["deprecated".to_string()];
        blink.attested_deprecated = true;
        blink.graded = Some(GradedForm {
            fire: ".blink".to_string(),
            severity: "low".to_string(),
            description: "Do not use the deprecated `.blink`.".to_string(),
            source: "https://mdn/String/blink".to_string(),
        });
        let web = vec![proven("var", "Never use var.", "u"), blink, read_only("if")];

        let mut e = crate::lint_codec::Enc::new();
        web.clone().enc(&mut e);
        let bytes = e.finish(crate::lint_codec::kind::WEB, "test-stamp");
        let (_stamp, mut d) = crate::lint_codec::Dec::open(&bytes, crate::lint_codec::kind::WEB).expect("opens");
        let back = <Vec<ConstructNode> as Bin>::dec(&mut d).expect("decodes");
        assert_eq!(back.iter().find(|n| n.construct == "String.blink").unwrap().graded.as_ref().unwrap().fire, ".blink");

        // The proven view is unmoved; the graded view is a separate LOW tier firing the `.blink` form.
        assert_eq!(derive_rules("javascript", &back).len(), 1, "only the proven node is a medium rule");
        let graded = derive_graded_rules("javascript", &back);
        assert_eq!(graded.len(), 1, "one graded low rule");
        assert_eq!(graded[0].0.id, "graded-String.blink");
        assert_eq!(graded[0].0.severity, "low");
        assert_eq!(graded[0].0.construct.as_deref(), Some(".blink"), "fires the receiver-generic form");
        assert_eq!(graded[0].1, "https://mdn/String/blink");
    }

    /// Meaning links are the distinctive KEY-WORDS the frozen brain knows, ranked by centrality — no
    /// word list. Skips honestly without a brain (never fakes a pass).
    #[test]
    fn meaning_links_are_known_distinctive_words() {
        let Some(br) = crate::lint_char::brain() else {
            eprintln!("skip: no char brain on disk");
            return;
        };
        let links = meaning_links(br.meanings(), "Never use the deprecated eval function.");
        assert!(!links.is_empty(), "the sentence has known content words");
        assert!(links.iter().all(|w| br.meanings().has(w)), "every link is a known key-word");
    }
}
