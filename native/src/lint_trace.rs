//! `lint_trace` — the understanding→trace bridge (owner directive 2026-07-08). The rule IS the
//! understanding: there is NO compiled per-principle detector anywhere in this module. A principle
//! written in prose is enforced by (1) reading its meaning, (2) ALIGNING its salient concepts — by
//! meaning, in the separating dictionary space (`lint_char::MeaningNetwork::related`) — to a small
//! set of GENERIC tracing primitives over the tree-sitter AST, and (3) composing those primitives
//! by ONE general rule into a query that yields the violating nodes.
//!
//! Nothing here knows what "dead code" or "DRY" is. It knows a fixed vocabulary of general senses —
//! node PREDICATES ("is a statement", "transfers control away", "is a public item", "is
//! documented") and structural RELATIONS ("follows in the same block", "is a duplicate subtree") —
//! each carrying a MEANING DESCRIPTOR (ordinary words). A principle names concepts; each concept
//! binds to the primitive whose descriptor it is DECISIVELY nearest to (a comparative
//! nearest-neighbor with a relative margin — never an absolute distance threshold); the bound
//! primitives compose. Adding a new principle sentence to the corpus, whose concepts already align
//! to these primitives, produces new enforcement with ZERO code change — that open-endedness is the
//! whole point. When a principle's concepts do NOT align, the bridge ABSTAINS (produces no rule)
//! rather than misfiring on a bad guess: silent-and-correct beats loud-and-wrong.
//!
//! This supersedes the hardcoded per-principle detectors in `lint_probe` (kept as the committed
//! fallback until this path passes every gate).

use crate::lint_char::MeaningNetwork;
use crate::lint_english::English;
use crate::lint_ai::DIM;
use tree_sitter::Node;

/// A node predicate's recogniser: reads one node (and source bytes) and answers whether the
/// property holds. It is also handed the predicate's own meaning descriptor `words`, so a
/// construct-recognising predicate (an `unwrap` call, a secret-named binding, a shell exec) draws
/// the tokens it looks for from THAT SINGLE declared vocabulary — never a second, hidden enumerated
/// list. Pure-structural predicates ignore it.
type PredFn = for<'a, 'b, 'c> fn(Node<'a>, &'b [u8], &'c [&'c str]) -> bool;

/// A relation's pair generator: yields the `(a, b)` node pairs standing in the relation.
type RelFn = for<'a, 'b> fn(Node<'a>, &'b [u8]) -> Vec<(Node<'a>, Node<'a>)>;

/// A generic NODE PREDICATE — a pure structural/semantic property of one AST node the trace can
/// RECOGNISE, carrying no policy. `words` is its meaning descriptor: the ordinary vocabulary a
/// principle concept aligns to when it MEANS this property. General and reusable — many principles
/// compose from the same predicates; none is per-principle.
struct Predicate {
    /// Machine name, for the plan's readable form and debugging.
    name: &'static str,
    /// The meaning descriptor — the words a sentence concept aligns to to select this predicate.
    words: &'static [&'static str],
    /// Whether this property is a COMPLETE defect on its own (a magic number, an over-long body, an
    /// unwrap) versus a ROLE/qualifier that only means something in a relation ("a statement", "a
    /// return", "public", "documented"). A unary (relation-less) rule fires on self-bad predicates
    /// only, so an incidental role concept the sentence also names ("…in the CODE", "never WRITE …")
    /// cannot silently AND itself onto the defect and make it un-fireable. A property of the
    /// primitive, not of any principle.
    self_bad: bool,
    /// Recognise the property on one node (source bytes for text/line inspection).
    test: PredFn,
}

/// A generic structural RELATION — yields ordered `(a, b)` node pairs standing in a relation, where
/// `a` fills endpoint A and `b` fills endpoint B. `words` selects the relation from a concept;
/// `endpoint_a`/`endpoint_b` are the two endpoints' meaning descriptors, so a role concept can be
/// assigned to the endpoint it is nearest to (meaning-driven direction, LINTER.md "the bridge").
struct Relation {
    name: &'static str,
    words: &'static [&'static str],
    endpoint_a: &'static [&'static str],
    endpoint_b: &'static [&'static str],
    pairs: RelFn,
}

/// The generic predicate vocabulary. GENERAL senses over the AST — never one entry per principle.
const PREDICATES: &[Predicate] = &[
    Predicate {
        name: "statement",
        words: &["statement", "code", "instruction", "expression", "logic", "block", "line"],
        self_bad: false,
        test: is_statement,
    },
    Predicate {
        name: "control_exit",
        words: &["return", "exit", "leave", "stop", "halt", "terminate", "break"],
        self_bad: false,
        test: is_control_exit,
    },
    Predicate {
        name: "public_item",
        words: &["public", "exposed", "expose", "exported", "function", "type", "interface"],
        self_bad: false,
        test: is_public_item,
    },
    Predicate {
        name: "documented",
        words: &["documentation", "comment", "documented", "describe", "description"],
        self_bad: false,
        test: is_documented,
    },
    Predicate {
        name: "single_letter_name",
        words: &["name", "letter", "variable", "identifier", "single", "character"],
        self_bad: true,
        test: is_single_letter_name,
    },
    Predicate {
        name: "unwrap_call",
        words: &["unwrap", "expect", "fallible", "result", "panic"],
        self_bad: true,
        test: is_unwrap_call,
    },
    Predicate {
        name: "magic_number",
        words: &["number", "literal", "numeric", "constant", "magic"],
        self_bad: true,
        test: is_magic_number,
    },
    Predicate {
        name: "long_body",
        words: &["enormous", "large", "long", "many", "statements", "responsibilities"],
        self_bad: true,
        test: is_long_body,
    },
    Predicate {
        name: "hardcoded_secret",
        words: &["secret", "password", "credential", "key", "token"],
        self_bad: true,
        test: is_hardcoded_secret,
    },
    Predicate {
        name: "shell_injection",
        words: &["shell", "command", "execute", "injection", "inject"],
        self_bad: true,
        test: is_shell_injection,
    },
    Predicate {
        name: "discarded_fallible",
        // The DISTINCTIVE swallowed-error vocabulary. "discard" is deliberately excluded: as a bare
        // verb it equally means discarding a variable/resource ("Unused variables — explicitly
        // discard"), a sense collision that would shadow a more specific rule; the distinctive
        // "swallow"/"ignore"/"suppress" (+ "error"/"exception") carry the defect without it. This
        // word list is a FALLBACK organ (owner doctrine: hardcoding to be retired): the learned
        // usage substrate (`lint_char::MeaningNetwork::usage`) proved it can grow "swallow" a
        // programming sense from real prose, but binding it structurally waits on broad-corpus scale
        // and, ultimately, on the reasoning path — the list stays only until understanding replaces it.
        words: &["ignore", "swallow", "error", "exception", "suppress"],
        self_bad: true,
        test: is_discarded_fallible,
    },
];

/// The generic relation vocabulary. GENERAL structural relations — never one per principle.
const RELATIONS: &[Relation] = &[
    Relation {
        name: "follows_in_block",
        words: &["after", "following", "follows", "subsequent", "next", "later", "then"],
        endpoint_a: &["later", "following", "subsequent", "after"],
        endpoint_b: &["earlier", "preceding", "before", "prior"],
        pairs: follows_in_block,
    },
    Relation {
        name: "duplicate_subtree",
        words: &["duplicate", "duplicated", "copy", "identical", "repeat", "same", "replicate"],
        endpoint_a: &["code", "block", "logic"],
        endpoint_b: &["code", "block", "logic"],
        pairs: duplicate_subtree,
    },
];

/// A concept's binding target — one generic primitive it aligned to by meaning.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Primitive {
    Pred(usize),
    Rel(usize),
}

/// The composition a principle compiles to — the general shape of a violation query, derived from
/// which primitives the sentence's concepts aligned to. NOT a per-principle object: it is just the
/// small set of bound primitive indices and their roles.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Plan {
    /// A single node satisfying every listed predicate — a self-bad shape ("a single-letter
    /// variable name").
    Unary(Vec<usize>),
    /// A node `a` (endpoint A of the relation) that stands in the relation to some node `b`
    /// (endpoint B), where predicate `a_pred` holds on `a` and `b_pred` on `b`.
    Relational { rel: usize, a_pred: usize, b_pred: usize },
    /// A node satisfying every `present` predicate but NONE of the `absent` predicates — the
    /// "X WITHOUT Y" shape ("a public item WITHOUT a documentation comment" → public_item ∧
    /// ¬documented). Produced when an inner-negation operator separates the present role concepts
    /// from the absent ones. General over any "present-thing lacking a property" principle.
    PresentWithout { present: Vec<usize>, absent: Vec<usize> },
    /// AST USAGE of a specific named code construct — the shape of a real language rule ("never
    /// use `var`", "never call `eval`", "avoid `==`"). `construct` is DATA extracted from the
    /// prohibition's own prose (a backticked name, or a token English cannot account for), never a
    /// coded token list. Fired by [`run_plan`] as an exact whole-token match over leaf token nodes,
    /// skipping string/char/comment interiors — the covenant-clean successor to the retired
    /// `tokens `X`` detector. Produced only when a prohibition names a construct AND aligns to no
    /// CS primitive (understanding a defect always beats matching a token).
    UsesConstruct { construct: String },
}

impl Plan {
    /// A readable form of the plan — the primitives it composed, for the checkpoint's raw verdict.
    pub fn describe(&self) -> String {
        match self {
            Plan::Unary(preds) => {
                let parts: Vec<&str> = preds.iter().map(|i| PREDICATES[*i].name).collect();
                format!("unary({})", parts.join(" & "))
            }
            Plan::Relational { rel, a_pred, b_pred } => format!(
                "relational({}: A={} B={})",
                RELATIONS[*rel].name, PREDICATES[*a_pred].name, PREDICATES[*b_pred].name
            ),
            Plan::PresentWithout { present, absent } => {
                let names = |ids: &[usize]| {
                    ids.iter().map(|i| PREDICATES[*i].name).collect::<Vec<_>>().join(" & ")
                };
                format!("present_without({} \\ {})", names(present), names(absent))
            }
            Plan::UsesConstruct { construct } => format!("uses_construct({construct})"),
        }
    }
}

/// One salient concept's alignment, as the `explain` query reports it: the nearest primitive and
/// the distances behind the decision. `aligned` is `Some(name)` only when the concept bound (its
/// nearest was decisively closer than the runner-up); a filler word has `aligned: None` even though
/// a `nearest` primitive is always named.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct ConceptAlign {
    pub word: String,
    pub nearest: String,
    pub distance: u32,
    pub runner_up: u32,
    pub ratio: f64,
    pub aligned: Option<String>,
    /// The concept's CENTRALITY to the prohibition — its dictionary distinctiveness
    /// ([`MeaningNetwork::centrality`]). A high-centrality word is what the sentence is ABOUT; a
    /// low one is incidental. The unary composition reads this so a peripheral word cannot drive a
    /// rule alone (see [`Bridge::compose_unary`]).
    pub centrality: u32,
}

/// The full step trace of understanding applied to one principle — the honest, structured answer
/// the `lint_query explain` interrogation returns. Shows exactly why understanding did or did not
/// shape a rule.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Explanation {
    /// The first sentence understanding read (a principle is stated once).
    pub sentence: String,
    /// Did the meaning-based prohibition gate fire?
    pub prohibition: bool,
    /// Negation OPERATORS found and set aside (they command; they are not concepts).
    pub operators: Vec<String>,
    /// INNER-NEGATION operators found ("without") — each flips the following role concepts into the
    /// `absent` set of a [`Plan::PresentWithout`]. Named so the `explain` query shows the fix at work.
    pub inner_negations: Vec<String>,
    /// Every salient concept and where it aligned.
    pub concepts: Vec<ConceptAlign>,
    /// The rule understanding shaped, when it did.
    pub plan: Option<Plan>,
    /// Why no rule was shaped, when none was.
    pub abstain: Option<String>,
}

/// The understanding the bridge reads a principle through: the separating dictionary meaning
/// network (for concept→primitive alignment) and the English brain (for the prohibition gate and
/// the discovered negators that set predicate polarity). Both are the loaded global brains in
/// production; a test builds them from the local dictionary.
pub struct Bridge<'a> {
    meanings: &'a MeaningNetwork,
    english: &'a English,
}

/// A concept the sentence names, aligned to a primitive — its word (for endpoint-meaning role
/// assignment) and its sentence position (for the object-adjacency tiebreak).
struct BoundConcept {
    word: String,
    position: usize,
    primitive: Primitive,
    /// Dictionary distinctiveness ([`MeaningNetwork::centrality`]) — how central this concept is to
    /// what the sentence prohibits. Read by [`Bridge::compose_unary`] so an incidental peripheral
    /// word cannot drive a unary defect alone.
    centrality: u32,
}

/// The tie-band (fraction of `DIM`) within which the two role concepts' endpoint preferences count
/// as INDISTINGUISHABLE — the endpoints are meaning-symmetric for these roles (a positional
/// relation's "later"/"earlier" carry no bias for "statement" or "return"), so direction falls to
/// the sentence-structure tiebreak. Outside the band, meaning decides (an asymmetric relation like
/// operand-of, whose endpoints "operand"/"operator" a role aligns to). A coarse band, not a
/// classification constant.
const ENDPOINT_TIE: f64 = 0.05;

/// How much nearer than the runner-up primitive a concept's best match must be to BIND — a
/// relative margin (a ratio, comparative), never an absolute distance the covenant forbids. A
/// genuine synonym is far nearer to its primitive's vocabulary than to any other's; a filler word
/// sits at the noise floor to every primitive, so no primitive is decisively nearest and it
/// abstains. This is what lets non-matching words fall away without a hand-picked cutoff.
const BIND_MARGIN: f64 = 0.60;

/// The meaning descriptor for an INNER-NEGATION operator — a preposition asserting that the concept
/// it precedes is ABSENT ("public function WITHOUT documentation" reads as public-item ∧
/// ¬documented). Distinct from the base negators [`Bridge::is_negator`] catches, which command the
/// WHOLE rule: this one flips the polarity of the role predicates that follow it. It is a MEANING
/// descriptor aligned through the same `related()` metric as a primitive's descriptor — never a
/// token-match rule — and it is applied COMPARATIVELY (nearer to absence than to any primitive), so
/// a content word that merely grazes the absence sense ("secret" = "not known") stays a concept
/// because it binds a real primitive decisively closer.
const ABSENCE: &[&str] = &["absence", "without", "lacking", "lack", "missing", "absent"];

impl<'a> Bridge<'a> {
    /// Read a principle through this understanding.
    pub fn new(meanings: &'a MeaningNetwork, english: &'a English) -> Bridge<'a> {
        Bridge { meanings, english }
    }

    /// Whether `word` is a NEGATION OPERATOR — a commanding negator ("never", "no", "not"), by the
    /// compounded [`English::is_negation`] test. Such a word commands the rule rather than naming a
    /// concept, so it is excluded from alignment. The COMPOUNDED test is deliberate: a looser
    /// "definition contains a negator" hop wrongly swept in absence-DEFINED content words ("secret"
    /// = "NOT known", "meaningless" = "having NO meaning") and excluded them as concepts. The cost
    /// is that a negation PREPOSITION whose dictionary definition never reaches a base negator
    /// ("without" = "in the absence of") is not caught — the honest inner-negation gap reported for
    /// undocumented-public (see LINTER.md).
    fn is_negator(&self, word: &str) -> bool {
        self.english.is_negation(crate::lint_ai::token_seed(word))
    }

    /// Whether `word` is an INNER-NEGATION operator (see [`ABSENCE`]) — DECISIVELY nearer to the
    /// absence sense than to any generic primitive. Comparative (a ratio against the word's best
    /// primitive distance), never an absolute cutoff: "without" (def "in the absence of") clears it
    /// while a content word that binds a primitive at ~0 never can. This is what lets "public
    /// WITHOUT documentation" read as an absence without a hand-listed preposition.
    fn is_inner_negation(&self, word: &str) -> bool {
        let absence = self.score(word, ABSENCE) as f64;
        let (_, best_primitive, _) = self.align_scored(word);
        absence <= best_primitive as f64 * BIND_MARGIN
    }

    /// The min meaning distance from `concept` to any of a primitive's descriptor `words` — the
    /// concept aligns to a primitive when it is a near-synonym of ANY word the primitive is
    /// described by. Two independent readings of "near", taking the BEST (min) of:
    ///   * DICTIONARY relatedness ([`related`]) — definitions share vocabulary. Binds a concept whose
    ///     general-English sense already IS the code shape ("duplicate" → `duplicate_subtree`).
    ///   * LEARNED-USAGE relatedness ([`context_related`]) — the two words are USED ALIKE in the docs
    ///     the brain read. This is what lets an inference concept the dictionary defines only in
    ///     general English ("unreachable" = "unable to be reached") align through its PROGRAMMING
    ///     sense — `unreachable` sits in docs beside `statements`/`unused`/`disallow`, so it reads as
    ///     near the `statement`/`control_exit` descriptors even though no definition connects them.
    /// Owner directive 2026-07-09 ("docs + dictionary understanding is enough"): the docs prose is
    /// now folded into the usage graph ([`ensure_brain`]), and this is where that learning enters
    /// the rule-shaping decision. Neither reading is spelling overlap — both are set overlap of
    /// distinctive words in an 8192-bit space.
    fn score(&self, concept: &str, words: &[&str]) -> u32 {
        words
            .iter()
            .map(|w| self.meanings.related(concept, w).min(self.meanings.context_related(concept, w)))
            .min()
            .unwrap_or(DIM as u32)
    }

    /// The name of a primitive (predicate or relation) — for readable query output.
    fn primitive_name(p: Primitive) -> &'static str {
        match p {
            Primitive::Pred(i) => PREDICATES[i].name,
            Primitive::Rel(i) => RELATIONS[i].name,
        }
    }

    /// Score `concept` against EVERY primitive and return the nearest, the winner distance, and the
    /// runner-up distance — the raw material both alignment and the `explain` query read. Sorted
    /// ascending by distance (0 = an exact descriptor-word match).
    fn align_scored(&self, concept: &str) -> (Primitive, u32, u32) {
        let mut scored: Vec<(Primitive, u32)> = Vec::new();
        for (i, p) in PREDICATES.iter().enumerate() {
            scored.push((Primitive::Pred(i), self.score(concept, p.words)));
        }
        for (i, r) in RELATIONS.iter().enumerate() {
            scored.push((Primitive::Rel(i), self.score(concept, r.words)));
        }
        scored.sort_by_key(|(_, d)| *d);
        let (best_prim, best) = scored[0];
        let runner_up = scored.get(1).map(|(_, d)| *d).unwrap_or(DIM as u32);
        (best_prim, best, runner_up)
    }

    /// UNDERSTANDING SHAPES A RULE: read a principle's prose and produce the composition
    /// understanding shapes from it, or `None` when the sentence states no prohibition or its
    /// concepts do not align to a usable set of primitives (ABSTAIN). The rule is what understanding
    /// PRODUCES here; the machinery is the same [`explain`](Self::explain) step trace, minus the
    /// bookkeeping. No per-principle branch.
    ///
    /// This is the GENERAL (language-doc) reading: a prohibition that names a code construct but
    /// aligns no CS primitive shapes a [`Plan::UsesConstruct`]. The language-AGNOSTIC canon reads
    /// through [`understand_canon`](Self::understand_canon) instead, which never mints a construct.
    pub fn understand(&self, description: &str) -> Option<Plan> {
        self.explain_scoped(description, true).plan
    }

    /// UNDERSTANDING SHAPES A RULE for the LANGUAGE-AGNOSTIC CANON — identical to
    /// [`understand`](Self::understand) except the construct fallback is SUPPRESSED. A canon
    /// principle is a language-agnostic design rule ("prefer deterministic behaviour"); it enforces
    /// through a structural primitive or ABSTAINS honestly, and it never names a code construct whose
    /// USE is the violation. Suppressing the fallback here is what keeps a canon sentence that merely
    /// MENTIONS a construct (`HashMap` in "prefer deterministic behaviour", `bugs` in a rationale)
    /// from minting `uses_construct(<that noun>)` — the junk class this scope exists to forbid.
    pub fn understand_canon(&self, description: &str) -> Option<Plan> {
        self.explain_scoped(description, false).plan
    }

    /// The step-by-step trace of understanding APPLIED to `description` — the debugger behind the
    /// `lint_query explain` interrogation. SCANS EVERY sentence (a principle states its enforceable
    /// clause once, often not in the opening line) and returns the trace of the FIRST sentence that
    /// both commands a prohibition AND yields a plan. When no sentence yields a rule, returns the
    /// most informative abstain: the first sentence whose prohibition gate fired, else the first
    /// sentence — so the reason stays honest. `understand` is this with only the plan kept.
    pub fn explain(&self, description: &str) -> Explanation {
        self.explain_scoped(description, true)
    }

    /// The step trace of understanding APPLIED to `description` at a given SCOPE. `allow_construct`
    /// is the only scope lever: `true` is the general (language-doc) reading where a prohibition
    /// naming a code construct shapes a [`Plan::UsesConstruct`]; `false` is the language-agnostic
    /// canon reading where the construct fallback is suppressed, so a canon principle enforces
    /// through a structural primitive or abstains and never mints a construct on an incidental noun.
    /// [`explain`](Self::explain) is this at general scope.
    pub fn explain_scoped(&self, description: &str, allow_construct: bool) -> Explanation {
        let sentences = crate::lint_read::sentences(description);
        if sentences.is_empty() {
            let mut ex = Explanation::default();
            ex.abstain = Some("empty input (no sentence)".to_string());
            return ex;
        }
        let mut fallback: Option<Explanation> = None;
        for sentence in sentences {
            let ex = self.explain_sentence(sentence, allow_construct);
            if ex.plan.is_some() {
                return ex;
            }
            // Keep the most honest abstain: upgrade to the first prohibition-gating sentence.
            let upgrade = fallback.as_ref().map_or(true, |f| !f.prohibition && ex.prohibition);
            if upgrade {
                fallback = Some(ex);
            }
        }
        fallback.unwrap_or_default()
    }

    /// The step trace for ONE sentence — the per-sentence core [`explain`] calls across a whole
    /// principle. Records the prohibition-gate result, every salient concept and the primitive it
    /// aligned to, and the plan understanding shaped from this sentence: a CS-primitive composition
    /// when its concepts align, else a [`Plan::UsesConstruct`] when the sentence NAMES a code
    /// construct, else the precise abstain reason.
    fn explain_sentence(&self, sentence: &str, allow_construct: bool) -> Explanation {
        let mut ex = Explanation::default();
        ex.sentence = sentence.to_string();
        ex.prohibition = self.english.sentence_states_prohibition(sentence);
        let tokens = tokenize(sentence);
        let mut bound: Vec<BoundConcept> = Vec::new();
        let mut inner_neg_pos: Option<usize> = None;
        for (pos, tok) in &tokens {
            if tok.len() < 3 || !tok.chars().all(|c| c.is_ascii_alphabetic()) {
                continue;
            }
            // An inner-negation ("without", "missing", "lacking") asserts the FOLLOWING concept is
            // ABSENT (present ∧ ¬absent) — set aside, its position remembered. Checked BEFORE the
            // base-negator test because a word can be both by definition ("missing" = "NOT able to
            // be found"): absence of a SPECIFIC concept is the more specific, truer reading, so it
            // flips one role rather than commanding the whole rule. A general base negator ("never",
            // "not") is not DECISIVELY an absence word (temporal/logical negation), so it fails the
            // comparative [`is_inner_negation`] test and falls through to the operator branch.
            if self.is_inner_negation(tok) {
                inner_neg_pos.get_or_insert(*pos);
                ex.inner_negations.push(tok.clone());
                continue;
            }
            // A base negator ("never", "no") is an OPERATOR that commands the whole rule — never a
            // concept naming a primitive.
            if self.is_negator(tok) {
                ex.operators.push(tok.clone());
                continue;
            }
            let (prim, best, runner_up) = self.align_scored(tok);
            let ratio = best as f64 / runner_up.max(1) as f64;
            let centrality = self.meanings.centrality(tok);
            let aligned = (ratio <= BIND_MARGIN).then(|| Self::primitive_name(prim).to_string());
            if aligned.is_some() {
                bound.push(BoundConcept {
                    word: tok.clone(),
                    position: *pos,
                    primitive: prim,
                    centrality,
                });
            }
            ex.concepts.push(ConceptAlign {
                word: tok.clone(),
                nearest: Self::primitive_name(prim).to_string(),
                distance: best,
                runner_up,
                ratio: (ratio * 1000.0).round() / 1000.0,
                aligned,
                centrality,
            });
        }
        // The prohibition gate. A construct is not inherently bad (docs name `let`/`const`/`map()`
        // too), so naming one shapes a rule ONLY when the sentence takes a negative stance toward it.
        // MEASURED 2026-07-10: dropping this gate made the reader flag every construct the docs
        // mention (`uses_construct(let)`, `(const)`, `(map())`) — over-generation, not understanding.
        // KNOWN LIMITATION: the gate is a positional-negation heuristic (a lead "never"/"not"), so it
        // misses MDN's dominant phrasings ("deprecated", "avoid", "should not") — recall is low. The
        // planned replacement is PROPOSE-then-VERIFY (LINTER.md): understanding proposes the rule,
        // reality proves it against the docs' own bad/good examples, and only a PROVEN rule is
        // learned — verification, not a gate, is the filter. Until that lands the gate stays, trading
        // recall for precision.
        if !ex.prohibition {
            ex.abstain = Some("states no prohibition — the meaning-based gate did not fire".to_string());
            return ex;
        }
        let relations: Vec<&BoundConcept> =
            bound.iter().filter(|b| matches!(b.primitive, Primitive::Rel(_))).collect();
        let predicates: Vec<&BoundConcept> =
            bound.iter().filter(|b| matches!(b.primitive, Primitive::Pred(_))).collect();
        // The sentence's CENTRAL baseline — the median distinctiveness of its content concepts.
        // A single-concept unary defect must clear this to shape a rule (see [`compose_unary`]), so
        // the CORE prohibited concept drives the rule, never an incidental peripheral word. Words
        // the meaning network has NO vector for (nearest primitive at the max `DIM` distance — an
        // unbound plural like "exceptions", a filler like "does") are excluded: they carry no
        // meaning to reason about, so counting their rarity-driven centrality would inflate the
        // baseline and wrongly suppress a genuine aligning concept ("swallow" in "Never Swallow
        // Exceptions"). The baseline measures distinctiveness AMONG UNDERSTOOD words, comparatively.
        let baseline = median(
            ex.concepts.iter().filter(|c| c.distance < DIM as u32).map(|c| c.centrality).collect(),
        );
        let plan = if let Some(rel_concept) = relations.first() {
            let Primitive::Rel(rel) = rel_concept.primitive else { unreachable!() };
            self.compose_relational(rel, rel_concept.position, &predicates)
        } else if let Some(neg_pos) = inner_neg_pos {
            self.compose_present_without(neg_pos, &predicates)
        } else {
            self.compose_unary(&predicates, baseline)
        };
        match plan {
            // The CS primitive ALWAYS wins when one composed — understanding a defect beats matching
            // a token. Only when NO primitive composed does a named construct carry the prohibition,
            // and only in the GENERAL (language-doc) scope — the language-agnostic canon never mints
            // a construct (`allow_construct == false`), enforcing structurally or abstaining.
            Some(p) => ex.plan = Some(p),
            None => match allow_construct.then(|| self.extract_construct(sentence, baseline)).flatten() {
                Some(construct) => ex.plan = Some(Plan::UsesConstruct { construct }),
                None => {
                    ex.abstain =
                        Some(self.abstain_reason(&relations, &predicates, inner_neg_pos, baseline));
                }
            },
        }
        ex
    }

    /// Extract the CODE CONSTRUCT a prohibition names, reusing the construct-ranking PRINCIPLE of
    /// [`crate::lint_match::select`] (never duplicating its word list) without needing that path's
    /// code grounding, which is unavailable at understand-time. Two covenant-clean signals identify
    /// a construct token:
    ///
    /// * BACKTICKED — the author naming a construct explicitly (kept EXACTLY: code is case- and
    ///   symbol-sensitive, `mem::uninitialized`, `==`). Highest priority.
    /// * A construct WORD — a token that reads as language syntax rather than prose: either some
    ///   bundled grammar lexes it as a keyword/primitive type ([`crate::lint_match::is_construct_keyword`]
    ///   — grammar-driven, never a keyword list) OR the dictionary meaning network cannot account for
    ///   it ([`MeaningNetwork::has`] — a token like `eval` no grammar flags as syntax). Either way the
    ///   token must be at least as CENTRAL as the sentence's `baseline` (its median content-word
    ///   distinctiveness): the same comparative gate [`compose_unary`] uses, and what separates the
    ///   distinctive `var` (centrality 92) from an incidental keyword-shaped word like `use` or `or`
    ///   that is common English. `!has` alone is NOT enough — the real dictionary knows `var` as an
    ///   obscure abbreviation, so the grammar signal carries it; the centrality gate carries the
    ///   quality.
    ///
    /// Among construct words the most CENTRAL wins (ties → earliest), so a distinctive construct is
    /// chosen over an incidental one. Read ONLY from this NAMING sentence, so a remedy alternative
    /// stated in a later sentence ("use `let` instead") can never be mistaken for the construct.
    /// `None` when nothing reads as a named construct — a bare English prohibition stays silent
    /// rather than mint junk.
    fn extract_construct(&self, sentence: &str, baseline: u32) -> Option<String> {
        let mut backticked: Vec<(usize, String)> = Vec::new();
        // (centrality, position, token) for each construct word — ranked most-central first.
        let mut words: Vec<(u32, usize, String)> = Vec::new();
        for (pos, raw) in sentence.split_whitespace().enumerate() {
            if let Some(open) = raw.find('`') {
                let inner = raw[open + 1..].split('`').next().unwrap_or("");
                if !inner.is_empty() {
                    backticked.push((pos, inner.to_string()));
                }
                continue;
            }
            let tok: String = raw.trim_matches(|c: char| !c.is_ascii_alphanumeric()).to_lowercase();
            if tok.len() < 2 || self.is_negator(&tok) || !tok.chars().any(|c| c.is_ascii_alphabetic()) {
                continue;
            }
            // A bare (non-backticked) construct name must read as a SINGLE code lexeme: an
            // unbroken identifier/keyword run. A token carrying interior punctuation ("hash-table",
            // "inner/private") is English prose joined by a symbol, not one construct — a genuine
            // multi-symbol construct (`mem::uninitialized`, `==`) arrives BACKTICKED (handled above).
            if !tok.chars().all(|c| c.is_ascii_alphanumeric()) {
                continue;
            }
            // English cannot account for it — MORPHOLOGY-AWARE, so a common word in any inflection
            // (`bugs`→bug, `maps`→map, `harder`→hard) is recognised as English and rejected, closing
            // the leak where `!has` alone treated every plural/comparative as unknown syntax. The
            // grammar-keyword signal still carries a real reserved word the dictionary also knows as
            // an obscure abbreviation (`var`).
            let english = crate::lint_graph::english_inflection(
                |w| self.meanings.has(w),
                &tok,
                crate::lint_graph::CONSTRUCT_SUFFIXES,
            );
            let reads_as_syntax = crate::lint_match::is_construct_keyword(&tok) || !english;
            let centrality = self.meanings.centrality(&tok);
            if reads_as_syntax && centrality >= baseline {
                words.push((centrality, pos, tok));
            }
        }
        if let Some((_, construct)) = backticked.into_iter().min_by_key(|(pos, _)| *pos) {
            return Some(construct);
        }
        // Most central construct word; ties broken by the earliest position (its naming order).
        words.into_iter().max_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1))).map(|(_, _, t)| t)
    }

    /// Compose a PRESENT-WITHOUT plan: an inner-negation operator at `neg_pos` asserts its OBJECT —
    /// the role concept nearest AFTER it ("…WITHOUT documentation", "MISSING documentation…") — is
    /// ABSENT; every OTHER role is the PRESENT context the absence is checked against. The rule flags
    /// a node satisfying every present predicate but not the absent one. Nearest-after (else
    /// nearest-before) is the same meaning-symmetric sentence-structure tiebreak
    /// [`compose_relational`](Self::compose_relational) uses, so "a public item WITHOUT a doc
    /// comment" and "MISSING documentation on a public API" both read as public-present ∧
    /// doc-absent regardless of which side the present role sits on. General over any "present thing
    /// lacking a property" principle; abstains when there is no object or no other present role (an
    /// inner negation needs something present to flag and something absent to check for).
    fn compose_present_without(&self, neg_pos: usize, predicates: &[&BoundConcept]) -> Option<Plan> {
        let object = predicates
            .iter()
            .filter(|b| b.position > neg_pos)
            .min_by_key(|b| b.position)
            .or_else(|| predicates.iter().filter(|b| b.position < neg_pos).max_by_key(|b| b.position))?;
        let Primitive::Pred(absent) = object.primitive else { return None };
        let mut present: Vec<usize> = Vec::new();
        for b in predicates {
            if let Primitive::Pred(i) = b.primitive {
                if i != absent && !present.contains(&i) {
                    present.push(i);
                }
            }
        }
        (!present.is_empty()).then_some(Plan::PresentWithout { present, absent: vec![absent] })
    }

    /// Why composition produced no rule — the precise application-failure reason the `explain` query
    /// reports (never a vague "abstained"). `inner_neg` carries the inner-negation operator's
    /// position when one was found, so an "X without Y" that failed to split is explained as such.
    fn abstain_reason(
        &self,
        relations: &[&BoundConcept],
        predicates: &[&BoundConcept],
        inner_neg: Option<usize>,
        baseline: u32,
    ) -> String {
        if let Some(pos) = inner_neg {
            let before = predicates.iter().any(|b| b.position < pos);
            let after = predicates.iter().any(|b| b.position > pos);
            if !before || !after {
                return format!(
                    "an inner-negation was found but a role predicate is missing on {} of it \
                     (an 'X without Y' needs a present thing and an absent property)",
                    if !before { "the present side" } else { "the absent side" }
                );
            }
        }
        if relations.is_empty() && predicates.is_empty() {
            return "no salient concept aligned to any primitive".to_string();
        }
        if let Some(rel) = relations.first() {
            return format!(
                "relation `{}` aligned but no role predicate did (a relation needs two endpoints)",
                Self::primitive_name(rel.primitive)
            );
        }
        let self_bad: Vec<&&BoundConcept> = predicates
            .iter()
            .filter(|b| matches!(b.primitive, Primitive::Pred(i) if PREDICATES[i].self_bad))
            .collect();
        if self_bad.is_empty() {
            let names: Vec<&str> =
                predicates.iter().map(|b| Self::primitive_name(b.primitive)).collect();
            return format!(
                "only role/qualifier concepts aligned ({}), none a self-contained defect",
                names.join(", ")
            );
        }
        // A self-bad defect DID align, yet composition abstained — the aligning concepts were all
        // PERIPHERAL (each a lone incidental word below the sentence's central baseline, its
        // descriptor grazed by a word the prohibition is not about). Name them, honestly.
        let peripheral: Vec<String> = self_bad
            .iter()
            .map(|b| format!("{} (centrality {} < baseline {baseline})", b.word, b.centrality))
            .collect();
        format!(
            "only an incidental concept aligned to a defect [{}] while the prohibition's central \
             concepts aligned to no primitive — abstaining rather than shaping a rule from a \
             peripheral word",
            peripheral.join(", ")
        )
    }

    /// Compose a RELATIONAL plan: assign the two role predicates to the relation's endpoints. The
    /// endpoint B (the reference the relation points AT — "a return", the earlier one) is the role
    /// concept nearest the relation word in the sentence (its object); the other role is endpoint
    /// A. This leans on the sentence's own structure only as the tiebreak the endpoints are
    /// meaning-symmetric for (a positional relation's "later"/"earlier" carry no bias for a role
    /// like "statement" or "return"). One distinct role → both endpoints are it (a symmetric
    /// relation like duplicate-subtree). Abstains on zero or more-than-two distinct roles.
    fn compose_relational(
        &self,
        rel: usize,
        rel_pos: usize,
        predicates: &[&BoundConcept],
    ) -> Option<Plan> {
        // All role bindings as (predicate index, sentence position, concept word) — NOT deduped, so
        // the position of the concept that is the relation's object survives even when another
        // concept aligns to the same predicate.
        let roles: Vec<(usize, usize, &str)> = predicates
            .iter()
            .filter_map(|b| match b.primitive {
                Primitive::Pred(i) => Some((i, b.position, b.word.as_str())),
                _ => None,
            })
            .collect();
        let first = *roles.first()?;
        // The two DISTINCT role predicates (there may be extra concepts aligning to the same one).
        let other = roles.iter().copied().find(|(i, _, _)| *i != first.0);
        let Some(second) = other else {
            // One role → the relation is symmetric (duplicate-subtree): both endpoints are it.
            return Some(Plan::Relational { rel, a_pred: first.0, b_pred: first.0 });
        };
        // MEANING-DRIVEN endpoint assignment (primary): each role concept prefers the endpoint its
        // meaning is nearer to (`endpoint_a` vs `endpoint_b`). When the two roles prefer opposite
        // endpoints decisively (an asymmetric relation like operand-of), meaning alone fixes the
        // direction.
        let r = &RELATIONS[rel];
        let pref = |w: &str| self.score(w, r.endpoint_b) as f64 - self.score(w, r.endpoint_a) as f64;
        let (pa, pb) = (pref(first.2), pref(second.2));
        let tie = ENDPOINT_TIE * DIM as f64;
        let (a_pred, b_pred) = if (pa - pb).abs() > tie {
            // `pref = dist(endpoint_b) - dist(endpoint_a)`, so a HIGHER pref means NEARER endpoint_a.
            // The role nearer endpoint_a is endpoint A; the other is endpoint B. (This was inverted:
            // it sent the endpoint_a-leaning role to B, so "a statement that follows a return"
            // compiled with `return` as the LATER endpoint — `follows_in_block` yields (later,
            // earlier) pairs, so the rule then looked for a control-exit AFTER a statement and never
            // fired on real dead code. Getting the direction right is what makes dead-code fire.)
            if pa >= pb {
                (first.0, second.0)
            } else {
                (second.0, first.0)
            }
        } else {
            // Meaning-symmetric endpoints → the sentence-structure TIEBREAK: endpoint B is the
            // relation's OBJECT, the nearest role AFTER the relation word ("after a RETURN"), else
            // the nearest before it.
            let b = roles
                .iter()
                .filter(|(_, p, _)| *p > rel_pos)
                .min_by_key(|(_, p, _)| *p)
                .or_else(|| roles.iter().filter(|(_, p, _)| *p < rel_pos).max_by_key(|(_, p, _)| *p))
                .copied()
                .unwrap_or(second);
            let a = roles.iter().copied().find(|(i, _, _)| *i != b.0).unwrap_or(first);
            (a.0, b.0)
        };
        Some(Plan::Relational { rel, a_pred, b_pred })
    }

    /// Compose a UNARY plan: the SELF-BAD defect the sentence is about. Only self-bad predicates
    /// (a magic number, an unwrap, an over-long body) are eligible — a role/qualifier concept the
    /// sentence incidentally names ("…in the code", "never write …") is dropped.
    ///
    /// A defect QUALIFIES to shape the rule only when its alignment is TRUSTWORTHY — the rule must
    /// be driven by the principle's CORE prohibited concept, never an incidental word:
    /// - CORROBORATED — two or more of the sentence's concepts align to it (a self-validating match:
    ///   "unwrap … expect … result … fallible" all point at `unwrap_call`); or
    /// - CENTRAL — a single aligning concept whose centrality is at least the sentence's `baseline`
    ///   (its median content-word distinctiveness). This is the fix for the tangential-word class:
    ///   in "Never ignore or discard an error RESULT", only the incidental noun `result` grazes a
    ///   descriptor (`unwrap_call`'s "result") while the prohibition's central concepts (`ignore`,
    ///   `discard`) align to nothing — `result` is below the sentence median, so the defect does not
    ///   qualify and the principle ABSTAINS honestly rather than shaping a wrong unwrap rule.
    ///
    /// Among the qualifying defects, the winner is the one the MOST concepts vote for (plurality):
    /// a principle names ONE defect, and ANDing a second self-bad predicate an incidental word
    /// aligned to would make the conjunction un-fireable (a node is rarely two defects at once).
    /// Ties keep the tied set. Abstains when no self-bad predicate qualifies. The `baseline` is
    /// comparative (the sentence's own median), never an absolute distinctiveness cutoff.
    fn compose_unary(&self, predicates: &[&BoundConcept], baseline: u32) -> Option<Plan> {
        // Per self-bad predicate: how many concepts vote for it, and the most-central one's weight.
        let mut votes: Vec<(usize, usize, u32)> = Vec::new(); // (predicate index, count, max centrality)
        for p in predicates {
            if let Primitive::Pred(i) = p.primitive {
                if PREDICATES[i].self_bad {
                    match votes.iter_mut().find(|(j, _, _)| *j == i) {
                        Some(v) => {
                            v.1 += 1;
                            v.2 = v.2.max(p.centrality);
                        }
                        None => votes.push((i, 1, p.centrality)),
                    }
                }
            }
        }
        // Keep only the defects that are corroborated OR carried by a central concept.
        votes.retain(|(_, count, centrality)| *count >= 2 || *centrality >= baseline);
        let top = votes.iter().map(|(_, n, _)| *n).max()?;
        let winners: Vec<usize> =
            votes.iter().filter(|(_, n, _)| *n == top).map(|(i, _, _)| *i).collect();
        Some(Plan::Unary(winners))
    }

    /// ENFORCE a principle on `code` of language `lang`: the 1-based lines its plan flags. Empty
    /// when the principle abstains or the language has no bundled grammar (a trace needs a tree).
    pub fn enforce(&self, description: &str, lang: &str, code: &str) -> Vec<usize> {
        let Some(plan) = self.understand(description) else { return Vec::new() };
        run_plan(&plan, lang, code)
    }

    /// PROPOSE-then-VERIFY — reason a principle's structural check by TESTING candidate senses
    /// against the principle's own bad/good evidence, keeping the one that actually catches the bad
    /// shape and spares the good. This is the M3-scale stand-in for Ornith's frozen-LLM judge: with
    /// no model to referee, REALITY referees — a check earns the rule only by working. It is
    /// cheat-proof by construction, three ways:
    ///
    ///   * the candidates are ONLY the general structural senses ([`PREDICATES`], self-bad) — never a
    ///     snippet matcher, so a winner cannot overfit the literal example;
    ///   * a winner MUST fire on `bad` AND stay clean on `good` — a lazy check that flags everything
    ///     is rejected by the good example (the anti-cheat the good half exists for);
    ///   * the plan is the SIMPLEST shape (one unary sense), and when several senses survive,
    ///     UNDERSTANDING breaks the tie (the survivor whose meaning is nearest the principle's own
    ///     central concepts) — reality picks the checks that work, comprehension picks which working
    ///     check the principle is ABOUT.
    ///
    /// Returns the verified plan, or `None` when no candidate both fires-bad and stays-clean — an
    /// honest abstain (no evidence that any check is right beats guessing one). Unlike
    /// [`understand`](Self::understand), this needs no descriptor WORD LIST to reach the right sense:
    /// the swallowed-error sense is selected because it CATCHES the swallowed error, not because a
    /// word matched — the path by which the hand-written lists burn down as understanding rises.
    pub fn understand_verified(
        &self,
        description: &str,
        lang: &str,
        bad: &str,
        good: &str,
    ) -> Option<Plan> {
        // Only a stated prohibition enforces — the same meaning-based gate `understand` uses.
        let ex = self.explain(description);
        if !ex.prohibition {
            return None;
        }
        // The COMPREHENSION GUARD: the primitives the principle's own concepts aligned to. A
        // candidate check may use ONLY these — a shape that merely happens to fire on a tiny
        // example (`present_without(control_exit \ documented)` on an unreachable-code example, say)
        // is rejected because the principle never NAMES `documented`. Reality proves a check works;
        // understanding proves it is the one the principle is ABOUT. Empty ⇒ nothing to enforce.
        let named: std::collections::HashSet<&str> =
            ex.concepts.iter().filter_map(|c| c.aligned.as_deref()).collect();
        if named.is_empty() {
            return None;
        }
        let allowed = |names: &[&str]| names.iter().all(|n| named.contains(n));
        // VERIFY: a check earns the rule only by FIRING on the bad shape and staying CLEAN on good.
        let fires = |plan: &Plan| {
            !run_plan(plan, lang, bad).is_empty() && run_plan(plan, lang, good).is_empty()
        };
        // GROUNDS-EXERCISE guard: a non-unary check is only validated when the GOOD example is a real
        // NEAR-MISS — it must contain the very construct being checked, done RIGHT. Without this, a
        // thin good example that simply OMITS the construct (a good with no `return` at all) fails to
        // rule out an over-broad shape ("flags any return"), and the verifier would mint that junk.
        // Requiring the good to EXERCISE the class turns poor grounds into an honest ABSTAIN, not a
        // bad rule. `unary` needs no such guard: the bad exhibits the self-bad defect and clean-good
        // alone suffices. This is the structural half of the two-way loop's demand that generated
        // grounds isolate the defect (good = the same shape minus only the fault).
        let exercised = |i: usize| !run_plan(&Plan::Unary(vec![i]), lang, good).is_empty();
        // PROPOSE the WHOLE primitive vocabulary as candidate shapes (updated 2026-07-09 — unary
        // alone cannot reach relational defects). Each carries its arity (primitive count) so the
        // SIMPLEST verified shape wins, and the descriptor words of its primitives so UNDERSTANDING
        // breaks any remaining tie (the survivor nearest the principle's own concepts).
        let concepts: Vec<&str> =
            ex.concepts.iter().filter(|c| c.distance < DIM as u32).map(|c| c.word.as_str()).collect();
        let nearness = |words: &[&str]| {
            concepts.iter().map(|w| self.score(w, words)).min().unwrap_or(DIM as u32)
        };
        let mut candidates: Vec<(Plan, usize, u32)> = Vec::new();
        // Unary — one self-bad predicate ("an unwrap", "a swallowed error").
        for (i, p) in PREDICATES.iter().enumerate() {
            let names = [p.name];
            if p.self_bad && allowed(&names) {
                let plan = Plan::Unary(vec![i]);
                if fires(&plan) {
                    candidates.push((plan, 1, nearness(p.words)));
                }
            }
        }
        // Present-without — a node with one property but lacking another ("public WITHOUT docs").
        // The good must EXERCISE the present class (a documented public item), or the check is not a
        // near-miss and cannot be trusted.
        for (pi, pp) in PREDICATES.iter().enumerate() {
            for (ai, ap) in PREDICATES.iter().enumerate() {
                if pi == ai || !allowed(&[pp.name, ap.name]) || !exercised(pi) {
                    continue;
                }
                let plan = Plan::PresentWithout { present: vec![pi], absent: vec![ai] };
                if fires(&plan) {
                    candidates.push((plan, 2, nearness(pp.words).min(nearness(ap.words))));
                }
            }
        }
        // Relational — two nodes standing in a structural relation ("a statement AFTER a return").
        // The good must EXERCISE BOTH endpoint classes (a real near-miss: the return present, just
        // nothing dead after it), or the pair cannot be trusted to isolate the defect.
        for (ri, r) in RELATIONS.iter().enumerate() {
            for (ai, ap) in PREDICATES.iter().enumerate() {
                for (bi, bp) in PREDICATES.iter().enumerate() {
                    if !allowed(&[r.name, ap.name, bp.name]) || !exercised(ai) || !exercised(bi) {
                        continue;
                    }
                    let plan = Plan::Relational { rel: ri, a_pred: ai, b_pred: bi };
                    if fires(&plan) {
                        let near =
                            nearness(r.words).min(nearness(ap.words)).min(nearness(bp.words));
                        candidates.push((plan, 3, near));
                    }
                }
            }
        }
        // SIMPLEST verified shape wins; UNDERSTANDING (lower nearness = closer to the principle's
        // concepts) breaks the tie. A single survivor needs no tie-break.
        candidates
            .into_iter()
            .min_by_key(|(_, arity, near)| (*arity, *near))
            .map(|(plan, _, _)| plan)
    }
}

/// UNDERSTAND a principle through the MACHINE'S LOADED BRAINS — the live entry the lint walk binds a
/// corpus principle with (`lint_match`). Reads the char brain's separating meaning network and the
/// English brain; `None` when a brain is unavailable or the principle abstains (no rule). Zero
/// per-principle logic: whatever prose the corpus holds is read through the one generic mechanism.
pub fn understand(description: &str) -> Option<Plan> {
    // REMEMBER what worked: a principle the AI has already REASONED to a verified check (by testing
    // it against real bad/good evidence, [`learn_verified`]) is recalled directly — the learned
    // policy that lets the word lists burn down. Only when nothing was learned does it fall back to
    // the descriptor-alignment path.
    if let Some(plan) = recall_verified(description) {
        return Some(plan);
    }
    let char_brain = crate::lint_char::brain()?;
    let english = crate::lint_english::brain()?;
    Bridge::new(char_brain.meanings(), english).understand(description)
}

/// The verified-rule memory file — principle → the check the AI PROVED correct against evidence.
fn verified_store_path() -> std::path::PathBuf {
    crate::lint_train::model_dir_pub().join("verified_rules.json")
}

/// The stable key for a principle's text — whitespace-collapsed, lowercased, seeded — so the same
/// principle recalls its learned check regardless of incidental spacing/case.
fn principle_key(description: &str) -> String {
    crate::lint_ai::token_seed(&description.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase())
        .to_string()
}

/// Load the verified-rule memory (principle key → plan), or an empty map when none is saved yet.
fn load_verified() -> std::collections::HashMap<String, Plan> {
    std::fs::read_to_string(verified_store_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Recall the check the AI already PROVED correct for this principle, if any.
pub fn recall_verified(description: &str) -> Option<Plan> {
    load_verified().remove(&principle_key(description))
}

/// LEARN a principle's check by PROPOSE-then-VERIFY against real evidence, then REMEMBER it: the AI
/// tests candidate structural senses on `bad`/`good`, keeps the one that catches the bad shape and
/// spares the good ([`Bridge::understand_verified`]), and PERSISTS it so later runs recall it
/// without re-deriving — reward flowing to the plan, Ornith-style, on an M3 with no judge model.
/// Returns the verified plan (also persisted), or `None` when no candidate verified (honest
/// abstain). The reward is reality's: a check earns the rule only by working.
pub fn learn_verified(description: &str, lang: &str, bad: &str, good: &str) -> Option<Plan> {
    let char_brain = crate::lint_char::brain()?;
    let english = crate::lint_english::brain()?;
    let plan = Bridge::new(char_brain.meanings(), english)
        .understand_verified(description, lang, bad, good)?;
    let mut store = load_verified();
    store.insert(principle_key(description), plan.clone());
    if let Some(parent) = verified_store_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&store) {
        let _ = std::fs::write(verified_store_path(), json);
    }
    Some(plan)
}

/// UNDERSTAND a LANGUAGE-AGNOSTIC CANON principle through the machine's loaded brains — the live
/// entry the lint walk binds a `corpus/*.md` principle with. The construct fallback is SUPPRESSED
/// (see [`Bridge::understand_canon`]): a canon principle enforces through a structural primitive or
/// abstains honestly, and never mints `uses_construct` on an incidental noun it merely mentions.
pub fn understand_canon(description: &str) -> Option<Plan> {
    let char_brain = crate::lint_char::brain()?;
    let english = crate::lint_english::brain()?;
    Bridge::new(char_brain.meanings(), english).understand_canon(description)
}

/// EXPLAIN understanding applied to `description` through the machine's loaded brains — the step
/// trace the `lint_query explain` interrogation returns. `canon` selects the scope: `true` reads it
/// as a language-agnostic canon principle (no construct fallback), `false` as general/language-doc
/// prose (a construct-naming prohibition may shape `uses_construct`). `None` when a brain is
/// unavailable.
pub fn explain(description: &str, canon: bool) -> Option<Explanation> {
    let char_brain = crate::lint_char::brain()?;
    let english = crate::lint_english::brain()?;
    Some(Bridge::new(char_brain.meanings(), english).explain_scoped(description, !canon))
}

/// The distance from `word` to EVERY generic primitive (min over the primitive's descriptor words),
/// sorted nearest-first — what the `lint_query define` interrogation reports as "nearest concepts
/// in the meaning space". `None` when a brain is unavailable.
pub fn concept_alignment(word: &str) -> Option<Vec<(String, u32)>> {
    let char_brain = crate::lint_char::brain()?;
    let english = crate::lint_english::brain()?;
    let bridge = Bridge::new(char_brain.meanings(), english);
    let mut out: Vec<(String, u32)> = PREDICATES
        .iter()
        .map(|p| (p.name.to_string(), bridge.score(word, p.words)))
        .chain(RELATIONS.iter().map(|r| (r.name.to_string(), bridge.score(word, r.words))))
        .collect();
    out.sort_by_key(|(_, d)| *d);
    Some(out)
}

/// The MEDIAN of a set of centralities — the sentence's typical content-word distinctiveness, the
/// comparative baseline a single-concept unary defect must clear ([`Bridge::compose_unary`]). An
/// empty set has no baseline (0), so nothing to compare against lets a lone concept through.
fn median(mut vals: Vec<u32>) -> u32 {
    if vals.is_empty() {
        return 0;
    }
    vals.sort_unstable();
    let n = vals.len();
    if n % 2 == 1 {
        vals[n / 2]
    } else {
        (vals[n / 2 - 1] + vals[n / 2]) / 2
    }
}

/// Split a sentence into `(position, lowercased-word)` tokens, punctuation trimmed — the position
/// is the token index, which the negation window and endpoint-object tiebreak read.
fn tokenize(sentence: &str) -> Vec<(usize, String)> {
    sentence
        .split_whitespace()
        .enumerate()
        .filter_map(|(i, w)| {
            let t: String = w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
            (!t.is_empty()).then_some((i, t))
        })
        .collect()
}

/// Evaluate a plan over parsed `code` — shared by [`Bridge::enforce`] and the tests. The plan is
/// the only per-principle state, and it is just primitive indices, so this runs identically for
/// every principle.
pub fn run_plan(plan: &Plan, lang: &str, code: &str) -> Vec<usize> {
    let Some(language) = crate::lint_match::language(lang) else { return Vec::new() };
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&language).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(code, None) else { return Vec::new() };
    let src = code.as_bytes();
    let root = tree.root_node();
    let mut hits: Vec<usize> = Vec::new();
    match plan {
        Plan::Unary(preds) => walk(root, &mut |node| {
            if preds.iter().all(|i| (PREDICATES[*i].test)(node, src, PREDICATES[*i].words)) {
                hits.push(row(node));
            }
        }),
        Plan::Relational { rel, a_pred, b_pred } => {
            let (ap, bp) = (&PREDICATES[*a_pred], &PREDICATES[*b_pred]);
            for (a, b) in (RELATIONS[*rel].pairs)(root, src) {
                if (ap.test)(a, src, ap.words) && (bp.test)(b, src, bp.words) {
                    hits.push(row(a));
                }
            }
        }
        Plan::PresentWithout { present, absent } => walk(root, &mut |node| {
            let holds = |i: &usize| (PREDICATES[*i].test)(node, src, PREDICATES[*i].words);
            if present.iter().all(&holds) && !absent.iter().any(&holds) {
                hits.push(row(node));
            }
        }),
        Plan::UsesConstruct { construct } => scan_construct(root, src, construct, &mut hits),
    }
    hits.sort_unstable();
    hits.dedup();
    hits
}

// ── Generic tree helpers ───────────────────────────────────────────────────────

/// The 1-based row a node starts on.
fn row(node: Node) -> usize {
    node.start_position().row + 1
}

/// Depth-first visit of every node. Lifetime-generic so collected nodes keep the tree's lifetime.
fn walk<'a>(node: Node<'a>, f: &mut impl FnMut(Node<'a>)) {
    f(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, f);
    }
}

/// Whether a node's kind is a STRING/CHAR/COMMENT interior — a lexical region whose text is not
/// code. [`scan_construct`] neither matches nor descends into it, so a construct's name discussed
/// in a comment or embedded in a string is never a usage. Read from the node kind by substring (the
/// blessed generic probe, as [`is_statement`] reads `comment`), never a grammar-specific list.
fn is_lexical_text(kind: &str) -> bool {
    kind.contains("string") || kind.contains("comment") || kind.contains("char")
}

/// Record every AST USAGE of the exact whole-token `construct`: a LEAF token node (an identifier,
/// type, field, keyword, or operator — whatever the grammar lexes as one token) whose utf8 text
/// equals `construct` EXACTLY (never a substring). Skips string/char/comment interiors entirely, so
/// the construct's name in prose or a literal is not a usage. The 1-based row of each usage is
/// pushed to `hits`. AST-grained by construction: only childless token nodes can match.
fn scan_construct(node: Node, src: &[u8], construct: &str, hits: &mut Vec<usize>) {
    if is_lexical_text(node.kind()) {
        return;
    }
    if node.child_count() == 0 && node.utf8_text(src).map(|t| t == construct).unwrap_or(false) {
        hits.push(row(node));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        scan_construct(child, src, construct, hits);
    }
}

// ── The generic node predicates (read node kinds / text / structure) ────────────

/// A node is a STATEMENT — a named, non-comment code node. The general grain a positional relation
/// flags and a duplicate compares.
fn is_statement(node: Node, _src: &[u8], _words: &[&str]) -> bool {
    node.is_named() && !node.kind().contains("comment")
}

/// A node TRANSFERS CONTROL AWAY — its kind is a control-transfer construct (a return, break,
/// continue, throw, or raise), read from the node KIND (the blessed generic probe), directly or as
/// the inner expression of an expression statement. Not a source-token list: these are AST
/// control-flow node kinds, universal across the grammars that model control flow.
fn is_control_exit(node: Node, _src: &[u8], _words: &[&str]) -> bool {
    fn kind_is_exit(kind: &str) -> bool {
        ["return", "break", "continue", "throw", "raise"].iter().any(|k| kind.contains(k))
    }
    if kind_is_exit(node.kind()) {
        return true;
    }
    node.kind() == "expression_statement"
        && node.named_child(0).is_some_and(|c| kind_is_exit(c.kind()))
}

/// A node is a PUBLIC ITEM — a function/type/module item carrying a visibility modifier. Read from
/// the node kind plus the presence of a `visibility_modifier` child (structural, grammar-driven).
fn is_public_item(node: Node, _src: &[u8], _words: &[&str]) -> bool {
    let is_item = matches!(
        node.kind(),
        "function_item"
            | "struct_item"
            | "enum_item"
            | "trait_item"
            | "type_item"
            | "mod_item"
            | "const_item"
            | "static_item"
    );
    if !is_item {
        return false;
    }
    if node.child_by_field_name("visibility_modifier").is_some() {
        return true;
    }
    let mut cursor = node.walk();
    let first_is_vis =
        node.children(&mut cursor).next().is_some_and(|c| c.kind() == "visibility_modifier");
    first_is_vis
}

/// A node is a SINGLE-LETTER NAME binding — a `let` binding or parameter whose name is one
/// alphabetic character (`x`, `n`). Read from the binding pattern's text; `_` and multi-letter
/// names never match.
fn is_single_letter_name(node: Node, src: &[u8], _words: &[&str]) -> bool {
    if !matches!(node.kind(), "let_declaration" | "parameter") {
        return false;
    }
    let Some(pat) = node.child_by_field_name("pattern") else { return false };
    let name = pat.utf8_text(src).unwrap_or("").trim();
    name.len() == 1 && name.chars().all(|c| c.is_ascii_alphabetic())
}

/// A node is DOCUMENTED — a doc comment opens the line immediately above it (skipping attribute
/// lines). Read from the source text so it is grammar-independent.
fn is_documented(node: Node, src: &[u8], _words: &[&str]) -> bool {
    let Ok(text) = std::str::from_utf8(src) else { return false };
    let lines: Vec<&str> = text.lines().collect();
    let mut i = node.start_position().row;
    while i > 0 {
        let prev = lines.get(i - 1).map(|l| l.trim_start()).unwrap_or("");
        if prev.starts_with("#[") || prev.starts_with("#!") {
            i -= 1;
            continue;
        }
        return prev.starts_with("///") || prev.starts_with("//!") || prev.starts_with("/**");
    }
    false
}

/// A node UNWRAPS A FALLIBLE VALUE — a method call whose method name carries one of this
/// predicate's own descriptor words (`unwrap`, `expect`): `x.unwrap()`, `y.expect(..)`. The tokens
/// it recognises are exactly the meaning descriptor, so there is no separate hidden list.
fn is_unwrap_call(node: Node, src: &[u8], words: &[&str]) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    let Some(func) = node.child_by_field_name("function") else { return false };
    if func.kind() != "field_expression" {
        return false;
    }
    let Some(field) = func.child_by_field_name("field") else { return false };
    let name = field.utf8_text(src).unwrap_or("");
    words.iter().any(|w| name.contains(w))
}

/// The small, self-explanatory numeric values that are never "magic" (a size hyperparameter, not
/// policy — the shape of a magic number, not which principle asks about it).
fn small_magnitude(raw: &str) -> bool {
    let digits: String = raw.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    matches!(digits.as_str(), "0" | "1" | "2" | "")
        || digits.parse::<f64>().map(|v| v <= 2.0).unwrap_or(false)
}

/// A node is a MAGIC NUMBER — a numeric literal that is neither small nor already named (not inside
/// a `const`/`static` initializer, an array length, or an attribute). Reads node kinds and the
/// literal's own text; no descriptor tokens needed.
fn is_magic_number(node: Node, src: &[u8], _words: &[&str]) -> bool {
    if !matches!(node.kind(), "integer_literal" | "float_literal") {
        return false;
    }
    if small_magnitude(node.utf8_text(src).unwrap_or("")) {
        return false;
    }
    let mut cur = node.parent();
    while let Some(p) = cur {
        if matches!(p.kind(), "const_item" | "static_item" | "attribute_item" | "attribute" | "array_type") {
            return false; // already named / structural — not a bare magic number
        }
        if matches!(p.kind(), "function_item" | "block") {
            break; // reached the enclosing body without a naming context
        }
        cur = p.parent();
    }
    true
}

/// The statement count above which a function body has outgrown one responsibility. A size
/// hyperparameter (the shape of an over-long body), not policy about which principle enforces it.
const LONG_BODY_STATEMENTS: usize = 25;

/// A node is a LONG BODY — a function whose body block holds more than [`LONG_BODY_STATEMENTS`]
/// statements. Pure structure; no descriptor tokens needed.
fn is_long_body(node: Node, _src: &[u8], _words: &[&str]) -> bool {
    if !matches!(node.kind(), "function_item" | "function_signature_item") {
        return false;
    }
    let Some(body) = node.child_by_field_name("body") else { return false };
    let mut cursor = body.walk();
    let statements = body
        .children(&mut cursor)
        .filter(|c| c.is_named() && !c.kind().contains("comment"))
        .count();
    statements > LONG_BODY_STATEMENTS
}

/// A node is a HARDCODED SECRET — a `let`/`const`/`static` binding whose NAME carries one of this
/// predicate's descriptor words (`secret`, `password`, `key`, `token`, `credential`) and whose
/// value is a non-trivial string literal. The recognised name words ARE the meaning descriptor —
/// one declared vocabulary, not a hidden list.
fn is_hardcoded_secret(node: Node, src: &[u8], words: &[&str]) -> bool {
    if !matches!(node.kind(), "let_declaration" | "static_item" | "const_item") {
        return false;
    }
    let name = node
        .child_by_field_name("pattern")
        .or_else(|| node.child_by_field_name("name"))
        .map(|p| p.utf8_text(src).unwrap_or("").to_lowercase())
        .unwrap_or_default();
    if !words.iter().any(|w| name.contains(w)) {
        return false;
    }
    let Some(val) = node.child_by_field_name("value") else { return false };
    val.kind() == "string_literal" && val.utf8_text(src).unwrap_or("").trim_matches('"').len() >= 8
}

/// A node is a SHELL INJECTION — the outermost call in a chain that both names a shell execution
/// (its text carries a descriptor word such as `command`/`shell`/`exec`) and builds an argument by
/// interpolation (`format!`). The shell-naming vocabulary is the meaning descriptor itself.
fn is_shell_injection(node: Node, src: &[u8], words: &[&str]) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    if node.parent().is_some_and(|p| matches!(p.kind(), "field_expression" | "call_expression")) {
        return false; // only examine the whole chain once, at its outermost call
    }
    let chain = node.utf8_text(src).unwrap_or("").to_lowercase();
    let names_shell = words.iter().any(|w| chain.contains(w));
    let interpolates = chain.contains("format!") || chain.contains("format !");
    names_shell && interpolates
}

/// A node DISCARDS A FALLIBLE VALUE — an error/result thrown away instead of handled: a
/// `let _ = <expr>` binding a non-trivial value (a call result, or a variable that held one) to
/// the throwaway `_` pattern, or an `Err(..) => {}` / `() ` match arm whose error branch does
/// nothing. The generic shape of a swallowed error — pure structure, no descriptor tokens (the
/// descriptor `words` name the CONCEPT a principle aligns to select this predicate; the SHAPE it
/// recognises is fixed). LINTER.md, "discarded_fallible": the generic primitive that lets
/// swallowed_error bind from the canon's real "Never Swallow Exceptions" prose via `unary`.
fn is_discarded_fallible(node: Node, src: &[u8], _words: &[&str]) -> bool {
    match node.kind() {
        "let_declaration" => {
            let is_wild = node
                .child_by_field_name("pattern")
                .and_then(|p| p.utf8_text(src).ok())
                .is_some_and(|t| t.trim() == "_");
            // A discarded RESULT is non-trivial: `let _ = 5;` deliberately drops a literal, but
            // `let _ = fallible();` and `let _ = d;` throw a real value (a failure) away.
            let discards = node
                .child_by_field_name("value")
                .is_some_and(|v| !is_trivial_value(v.kind()));
            is_wild && discards
        }
        "match_arm" => {
            let names_err = node
                .child_by_field_name("pattern")
                .and_then(|p| p.utf8_text(src).ok())
                .is_some_and(|t| t.contains("Err"));
            let empty_body = node
                .child_by_field_name("value")
                .and_then(|v| v.utf8_text(src).ok())
                .is_some_and(|t| matches!(t.trim(), "{}" | "()"));
            names_err && empty_body
        }
        _ => false,
    }
}

/// Whether an expression node is a TRIVIAL literal — a bare number, string, char, boolean, or
/// unit. A `let _ =` of such a value is a deliberate discard, not a swallowed fallible result.
fn is_trivial_value(kind: &str) -> bool {
    matches!(
        kind,
        "integer_literal"
            | "float_literal"
            | "string_literal"
            | "char_literal"
            | "boolean_literal"
            | "unit_expression"
    )
}

// ── The generic structural relations ────────────────────────────────────────────

/// FOLLOWS-IN-BLOCK: for every block, each later named sibling paired with each earlier named
/// sibling — `(later, earlier)`, matching endpoint A = the following one, endpoint B = the earlier
/// one. A general positional relation; "code after a return" is one composition over it.
fn follows_in_block<'a>(root: Node<'a>, _src: &[u8]) -> Vec<(Node<'a>, Node<'a>)> {
    let mut out = Vec::new();
    walk(root, &mut |node| {
        if node.kind() != "block" {
            return;
        }
        let mut cursor = node.walk();
        let sibs: Vec<Node> =
            node.children(&mut cursor).filter(|c| c.is_named() && !c.kind().contains("comment")).collect();
        for later in 0..sibs.len() {
            for earlier in 0..later {
                out.push((sibs[later], sibs[earlier]));
            }
        }
    });
    out
}

/// The smallest subtree size (node count) that can count as a real duplicate — below this, two
/// tiny bodies coinciding is coincidence, not a copy.
const DUP_MIN_NODES: usize = 12;

/// DUPLICATE-SUBTREE: every pair of function bodies with an identical structural shape (identical
/// node-kind sequence), in both orders so each duplicate site is an endpoint A once. A general
/// structural relation; DRY is one composition over it.
fn duplicate_subtree<'a>(root: Node<'a>, _src: &[u8]) -> Vec<(Node<'a>, Node<'a>)> {
    use std::collections::HashMap;
    let mut by_shape: HashMap<u64, Vec<Node>> = HashMap::new();
    walk(root, &mut |node| {
        if node.kind() != "function_item" {
            return;
        }
        let Some(body) = node.child_by_field_name("body") else { return };
        let mut kinds: Vec<u16> = Vec::new();
        walk(body, &mut |n| kinds.push(n.kind_id()));
        if kinds.len() < DUP_MIN_NODES {
            return;
        }
        let mut h = 0xcbf29ce484222325u64;
        for k in &kinds {
            h ^= u64::from(*k);
            h = h.wrapping_mul(0x100000001b3);
        }
        by_shape.entry(h).or_default().push(node);
    });
    let mut out = Vec::new();
    for group in by_shape.values() {
        for i in 0..group.len() {
            for j in 0..group.len() {
                if i != j {
                    out.push((group[i], group[j]));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build the understanding a real machine loads: the separating dictionary meaning network
    /// (whole dictionary) plus the English brain (bootstrap fallback is fine). Ignored — reads the
    /// local dictionary.
    fn understanding() -> MeaningNetwork {
        let defs = crate::lint_english::dictionary_definitions(None, crate::lint_char::MAX_MEANING_WORDS)
            .expect("a readable dictionary");
        let mut m = MeaningNetwork::new();
        for (head, words) in &defs {
            m.bind(head, &words.iter().map(String::as_str).collect::<Vec<_>>());
        }
        m.seal();
        m
    }

    /// THE CHECKPOINT PROOF (owner directive 2026-07-08): three DIFFERENTLY-SHAPED principles —
    /// their REAL corpus prose — enforce end to end through ONE mechanism with ZERO per-principle
    /// code: (a) dead-code-after-return (relation + two roles + direction — the crux), (b)
    /// undocumented-public-item (unary, single self-bad predicate with an inner negation), (c) DRY
    /// (a different relation, symmetric). Each flags the bad shape and is clean on good code. Raw
    /// verdicts printed. Ignored (reads the local dictionary). Run:
    /// `cargo test --release --lib bridge_enforces_three_shapes -- --ignored --nocapture`
    #[test]
    #[ignore = "reads the local dictionary; the Step-3 three-shape bridge proof"]
    fn bridge_enforces_three_shapes() {
        let meanings = understanding();
        let english = crate::lint_english::brain().expect("English brain (bootstrap fallback)");
        let bridge = Bridge::new(&meanings, &english);

        // (a) dead-code-after-return — the real corpus prose.
        let dead = "Never write code after a return statement. A statement that follows a return \
                    is unreachable dead code.";
        let plan = bridge.understand(dead).expect("dead-code understood");
        eprintln!("(a) dead-code plan: {}", plan.describe());
        assert!(matches!(plan, Plan::Relational { .. }), "dead-code is relational: {}", plan.describe());
        let bad_a = "fn f(x: i32) -> i32 {\n    if x > 0 {\n        return x;\n        let z = x;\n        drop(z);\n    }\n    x\n}\n";
        let good_a = "fn f(x: i32) -> i32 {\n    if x > 0 {\n        return x;\n    }\n    x + 1\n}\n";
        let hits_bad_a = bridge.enforce(dead, "rust", bad_a);
        let hits_good_a = bridge.enforce(dead, "rust", good_a);
        eprintln!("    bad flags lines {hits_bad_a:?}; good flags {hits_good_a:?}");
        assert!(hits_bad_a.contains(&4), "the dead `let z` (line 4) flags: {hits_bad_a:?}");
        assert!(hits_good_a.is_empty(), "clean code is not flagged: {hits_good_a:?}");

        // (b) non-descriptive-name — the UNARY shape: a single self-bad predicate, no relation,
        // no inner negation (undocumented-public needs inner-negation of "without", a Step-4 gap
        // reported to main — the dictionary clusters that preposition with positional ones).
        let naming = "Never name a variable with a single meaningless letter.";
        let plan_b = bridge.understand(naming).expect("naming understood");
        eprintln!("(b) naming plan: {}", plan_b.describe());
        assert!(matches!(plan_b, Plan::Unary(_)), "naming is unary: {}", plan_b.describe());
        let bad_b = "fn f(count: i32) -> i32 {\n    let x = count + 1;\n    x\n}\n";
        let good_b = "fn compute(count: i32) -> i32 {\n    let total = count + 1;\n    total\n}\n";
        let hits_bad_b = bridge.enforce(naming, "rust", bad_b);
        let hits_good_b = bridge.enforce(naming, "rust", good_b);
        eprintln!("    bad flags lines {hits_bad_b:?}; good flags {hits_good_b:?}");
        assert!(hits_bad_b.contains(&2), "the single-letter `let x` (line 2) flags: {hits_bad_b:?}");
        assert!(hits_good_b.is_empty(), "descriptive names are not flagged: {hits_good_b:?}");

        // (c) DRY — a different relation (duplicate-subtree), symmetric.
        let dry = "Never duplicate the same code in two places.";
        let plan_c = bridge.understand(dry).expect("DRY understood");
        eprintln!("(c) DRY plan: {}", plan_c.describe());
        assert!(matches!(plan_c, Plan::Relational { .. }), "DRY is relational: {}", plan_c.describe());
        let bad_c = "fn alpha(a: i32, b: i32) -> i32 {\n    let s = a + b;\n    let t = s * 2;\n    let u = t - 1;\n    u\n}\nfn beta(a: i32, b: i32) -> i32 {\n    let s = a + b;\n    let t = s * 2;\n    let u = t - 1;\n    u\n}\n";
        let good_c = "fn alpha(a: i32) -> i32 {\n    a + 1\n}\nfn beta(a: i32, b: i32) -> i32 {\n    let s = a + b;\n    let t = s * 2;\n    s - t\n}\n";
        let hits_bad_c = bridge.enforce(dry, "rust", bad_c);
        let hits_good_c = bridge.enforce(dry, "rust", good_c);
        eprintln!("    bad flags lines {hits_bad_c:?}; good flags {hits_good_c:?}");
        assert!(hits_bad_c.len() >= 2, "both duplicated functions flag: {hits_bad_c:?}");
        assert!(hits_good_c.is_empty(), "distinct functions are not flagged: {hits_good_c:?}");

        // ABSTAIN, never misfire: a sentence that states no prohibition, and a prohibition whose
        // concepts align to NO primitive, both produce no rule.
        assert!(
            bridge.understand("A public function should carry a documentation comment.").is_none(),
            "a non-prohibition states no rule"
        );
        // A prohibition that NAMES a code construct but aligns to no CS primitive now shapes a
        // `uses_construct` rule (the covenant-clean successor to `tokens `X``) — understanding
        // reads the construct out of the prose rather than abstaining.
        assert!(
            matches!(
                bridge.understand("Never use the var keyword to declare a variable.").as_ref(),
                Some(Plan::UsesConstruct { construct }) if construct == "var"
            ),
            "a construct-naming prohibition shapes uses_construct(var)"
        );
        eprintln!("ABSTAIN: non-prohibition produced no rule; var-prohibition shapes uses_construct.");
    }

    /// INNER-NEGATION: undocumented_public_item — the real corpus prose "…public function or type
    /// WITHOUT a documentation comment" — composes `present_without(public_item \ documented)` and
    /// FIRES on an undocumented public item while staying clean on a documented one, through the same
    /// understand()/enforce() path with zero per-principle code. The companion abstain check proves
    /// the fix did not turn the honest swallowed_error abstain into a misfire. Ignored (dictionary).
    #[test]
    #[ignore = "reads the local dictionary; the inner-negation firing check"]
    fn inner_negation_enforces_undocumented_public() {
        let meanings = understanding();
        let english = crate::lint_english::brain().expect("English brain");
        let bridge = Bridge::new(&meanings, &english);

        let undoc = "Never expose a public function or type without a documentation comment. A public \
                     item is an API other code depends on; document every public item with a comment.";
        let plan = bridge.understand(undoc).expect("undocumented-public understood");
        eprintln!("undoc plan: {}", plan.describe());
        assert!(matches!(plan, Plan::PresentWithout { .. }), "present-without shape: {}", plan.describe());

        let bad = "pub fn total(items: &[i32]) -> i32 {\n    items.iter().sum()\n}\n";
        let good = "/// Sum every item in the slice.\npub fn total(items: &[i32]) -> i32 {\n    items.iter().sum()\n}\n";
        let hits_bad = bridge.enforce(undoc, "rust", bad);
        let hits_good = bridge.enforce(undoc, "rust", good);
        eprintln!("    bad flags lines {hits_bad:?}; good flags {hits_good:?}");
        assert!(hits_bad.contains(&1), "the undocumented `pub fn` (line 1) flags: {hits_bad:?}");
        assert!(hits_good.is_empty(), "a documented public item is not flagged: {hits_good:?}");

        // REVERSED ROLE ORDER (owner directive 2026-07-09): the CANON states the same rule as
        // "Missing documentation on public APIs — write it" — the absence word "missing" leads and
        // its object (documentation) is ABSENT while the present role (public API) trails it. The
        // generalized object-based split reads this as the SAME public-present ∧ doc-absent shape,
        // so understanding binds the rule from the canon's REAL phrasing, not only the "X without
        // Y" order. ("missing" is an absence word even though its definition compounds "not", so it
        // is an inner-negation, not a whole-rule operator.)
        let missing = "Missing documentation on public APIs — write it.";
        let plan_m = bridge.understand_canon(missing).expect("missing-phrasing undoc understood");
        eprintln!("missing-phrasing plan: {}", plan_m.describe());
        assert!(
            matches!(plan_m, Plan::PresentWithout { .. }),
            "reversed 'missing Y on X' reads as present-without: {}",
            plan_m.describe()
        );
        assert!(bridge.enforce(missing, "rust", bad).contains(&1), "missing-phrasing flags the undocumented pub fn");
        assert!(bridge.enforce(missing, "rust", good).is_empty(), "missing-phrasing clean on documented");

        // swallowed_error now ENFORCES through the generic `discarded_fallible` primitive
        // (LINTER.md): "ignore"/"discard"/"swallow" align to it, so the prohibition shapes a
        // unary(discarded_fallible) rule that flags a `let _ = <fallible>` and stays clean on a
        // handled result.
        let swallowed = "Never ignore, discard, or swallow an error.";
        assert!(
            matches!(bridge.understand(swallowed), Some(Plan::Unary(ref p)) if p.iter().any(|i| PREDICATES[*i].name == "discarded_fallible")),
            "swallowed_error shapes unary(discarded_fallible): {:?}",
            bridge.understand(swallowed).map(|p| p.describe())
        );
        let bad_sw = "fn f() { let d = std::fs::read_to_string(\"x\"); let _ = d; }";
        let good_sw = "fn f() -> std::io::Result<()> { std::fs::read_to_string(\"x\")?; Ok(()) }";
        assert!(!bridge.enforce(swallowed, "rust", bad_sw).is_empty(), "flags the discarded result");
        assert!(bridge.enforce(swallowed, "rust", good_sw).is_empty(), "clean on the handled result");
        eprintln!("swallowed_error: enforces via unary(discarded_fallible).");
    }

    /// REASONING beats word-matching: the AI selects a principle's structural check by PROPOSE-then-
    /// VERIFY — testing candidate senses against the principle's own bad/good evidence — and reaches
    /// the right one where the descriptor-word path abstains. "Never Swallow Exceptions" (a bare
    /// title) trips the word-path's centrality gate and shapes NO rule; verification enforces it by
    /// finding the sense that actually catches the swallowed error and spares the handled one. This
    /// is the Ornith method on an M3 with no judge model: reality referees. Ignored (dictionary +
    /// rust grammar).
    #[test]
    #[ignore = "reads the local dictionary + rust grammar; the propose-then-verify reasoning proof"]
    fn reasoning_selects_check_by_verification() {
        let char_brain = crate::lint_char::brain().expect("char brain");
        let english = crate::lint_english::brain().expect("english brain");
        let bridge = Bridge::new(char_brain.meanings(), english);
        let principle = "Never Swallow Exceptions";
        // Isolated evidence: the bad shape carries ONLY the swallowed-result defect (a bare `let _`,
        // no single-letter name to confound), so verification has a UNIQUE survivor.
        let bad = "fn f() { let _ = std::fs::read_to_string(\"x\"); }";
        let good = "fn f() -> std::io::Result<()> { std::fs::read_to_string(\"x\")?; Ok(()) }";
        // The descriptor-word path abstains on this bare title (centrality gate) — reasoning must not.
        let word_path = bridge.understand(principle);
        let verified = bridge
            .understand_verified(principle, "rust", bad, good)
            .expect("propose-then-verify selects a check");
        eprintln!(
            "word-path: {:?}; verified: {}",
            word_path.as_ref().map(Plan::describe),
            verified.describe()
        );
        assert!(
            matches!(verified, Plan::Unary(ref p) if p.iter().any(|i| PREDICATES[*i].name == "discarded_fallible")),
            "verification selects discarded_fallible by TESTING, not word-match: {}",
            verified.describe()
        );
        assert!(!run_plan(&verified, "rust", bad).is_empty(), "fires on the swallowed error");
        assert!(run_plan(&verified, "rust", good).is_empty(), "clean on the handled result");
    }

    /// The five primitives added in Step 4 each FIRE on their bad shape and stay clean on good
    /// code — through the same understand()/enforce() path, real corpus prose. Ignored (dictionary).
    #[test]
    #[ignore = "reads the local dictionary; Step-4 new-primitive firing check"]
    fn new_primitives_fire() {
        let meanings = understanding();
        let english = crate::lint_english::brain().expect("English brain");
        let b = Bridge::new(&meanings, &english);
        let cases: [(&str, &str, &str); 5] = [
            (
                "Never unwrap or expect the result of a fallible call.",
                "fn f() { let v: i32 = \"1\".parse().unwrap(); }",
                "fn f() -> Result<i32, ()> { \"1\".parse().map_err(|_| ()) }",
            ),
            (
                "Never bury an unexplained magic number literal in the code.",
                "fn f() -> i32 { let d = 86400; d }",
                "const DAY: i32 = 86400;\nfn f() -> i32 { DAY }",
            ),
            (
                "Never hardcode a secret in the source.",
                "fn f() { let api_key = \"sk-9f8a7b6c5d4e3f2a\"; }",
                "fn f() { let api_key = std::env::var(\"API_KEY\").unwrap_or_default(); }",
            ),
            (
                "Never interpolate untrusted input into a shell command string.",
                "fn f(u: &str) { std::process::Command::new(\"sh\").arg(format!(\"echo {u}\")); }",
                "fn f(u: &str) { std::process::Command::new(\"echo\").arg(u); }",
            ),
            (
                "Never write an enormous function that does too many things.",
                &format!("fn big() {{\n{}}}\n", "    let _ = 1;\n".repeat(30)),
                "fn small() { let a = 1; let b = 2; let _ = a + b; }",
            ),
        ];
        for (prose, bad, good) in cases {
            let bad_hits = b.enforce(prose, "rust", bad);
            let good_hits = b.enforce(prose, "rust", good);
            eprintln!("{:.40} -> bad {bad_hits:?} good {good_hits:?}", prose);
            assert!(!bad_hits.is_empty(), "must flag the bad shape: {prose}");
            assert!(good_hits.is_empty(), "must be clean on good: {prose} -> {good_hits:?}");
        }
    }

    /// USES_CONSTRUCT (owner directive 2026-07-09): a prohibition that NAMES a code construct but
    /// aligns to no CS primitive — "Never use the var keyword to declare a variable" — shapes
    /// `uses_construct(var)` and FIRES on javascript that uses `var`, staying clean on `let`/`const`
    /// code. `let`/`const` (the remedy in a later sentence) are NEVER chosen as the construct. The
    /// name is DATA read from the prose, never a coded token list. Ignored (reads the dictionary).
    #[test]
    #[ignore = "reads the local dictionary; the uses_construct firing check"]
    fn uses_construct_fires() {
        let meanings = understanding();
        let english = crate::lint_english::brain().expect("English brain");
        let bridge = Bridge::new(&meanings, &english);

        let prose = "Never use the var keyword to declare a variable. Use let or const instead.";
        let plan = bridge.understand(prose).expect("var prohibition understood");
        eprintln!("var plan: {}", plan.describe());
        assert!(
            matches!(&plan, Plan::UsesConstruct { construct } if construct == "var"),
            "names the var construct, not the let/const remedy: {}",
            plan.describe()
        );

        let bad = "function f() {\n    var x = 1;\n    return x;\n}\n";
        let good = "function f() {\n    let x = 1;\n    const y = 2;\n    return x + y;\n}\n";
        let hits_bad = bridge.enforce(prose, "javascript", bad);
        let hits_good = bridge.enforce(prose, "javascript", good);
        eprintln!("    bad flags {hits_bad:?}; good flags {hits_good:?}");
        assert!(hits_bad.contains(&2), "the `var` on line 2 flags: {hits_bad:?}");
        assert!(hits_good.is_empty(), "let/const code is clean: {hits_good:?}");
    }

    /// COVERAGE MAP (owner directive): read EVERY language-agnostic principle across the owner's
    /// CANON files EXACTLY as the LIVE lint reads them — [`crate::lint_train::canon_agnostic`] drops
    /// the language appendix, then [`crate::linter::Knowledge::read_document`] assembles each `##`
    /// section into a DocRule description (heading folded in, lines kept as `\n` sentence
    /// boundaries) — and report which the bridge enforces (with the plan it composed) and which
    /// ABSTAIN. Reading through the real assembler (not a synthetic space-join) is what makes this
    /// map HONEST: a terminal-less canon bullet and a `Never …` heading each surface as their own
    /// sentence, exactly as live understanding sees them. Ignored (reads the local dictionary + the
    /// repo corpus). Run: `cargo test --release --lib coverage_map -- --ignored --nocapture`
    #[test]
    #[ignore = "reads the local dictionary + repo corpus; the honest canon coverage map"]
    fn coverage_map() {
        let meanings = understanding();
        let english = crate::lint_english::brain().expect("English brain");
        let bridge = Bridge::new(&meanings, &english);
        let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("corpus");
        let (mut enforced, mut abstained) = (0u32, 0u32);
        for file in ["cs3500-rubric.md", "cs2420-principles.md"] {
            eprintln!("== {file} ==");
            let raw = std::fs::read_to_string(corpus.join(file)).expect("canon file");
            // The live path: strip the language appendix, then read the document into DocRules —
            // one per `##` principle — through the SAME assembler the lint uses.
            let doc = crate::lint_train::canon_agnostic(&raw);
            let rules = crate::linter::Knowledge::read_document("any", &doc, None).rules;
            for r in &rules {
                // Canon scope — the language-agnostic canon never mints a `uses_construct`; it
                // enforces through a structural primitive or abstains honestly.
                match bridge.understand_canon(&r.description) {
                    Some(plan) => {
                        enforced += 1;
                        // HONEST live note: the live corpus router only reaches understanding when
                        // the principle carries NO in-language example (`lint_match` mod.rs:
                        // `is_corpus_principle = source.contains("/corpus/") && bad.trim().is_empty()`).
                        // A principle whose canon prose includes a `// Bad`/`// Good` illustration
                        // (e.g. "6. Never Swallow Exceptions" ships a Java snippet) is DIVERTED live
                        // to the example-based path as a rule in the ILLUSTRATION's language — so the
                        // rule understanding shapes here does not fire live until corpus principles
                        // route understanding-FIRST. Flagged so the map never overstates live reach.
                        let live = if r.bad.trim().is_empty() {
                            "live"
                        } else {
                            "SHAPED-ONLY (live example-diverted; carries a bad example)"
                        };
                        eprintln!("  ENFORCE  {:44} -> {}  [{live}]", r.id, plan.describe());
                    }
                    None => {
                        abstained += 1;
                        eprintln!("  abstain  {:44} -> honest (no aligning primitive)", r.id);
                    }
                }
            }
        }
        eprintln!("COVERAGE: {enforced} enforced, {abstained} abstained");
    }
}


