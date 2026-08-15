//! The lossless AST-pattern path of [`super`]: compiling a rule's documented `bad`/`good`
//! examples into a generalized sub-tree pattern via `bad ∧ ¬good` tree-diff, and matching it by
//! exact sub-tree containment with variable binding. Operations/keywords/operators stay exact;
//! variables become bound wildcards; literals become typed wildcards. Deterministic — no
//! statistics, no floats. Theory and failure ledger: `native/architecture.dx`.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser};

use super::grammar::language;
use super::MAX_EXAMPLE_BYTES;

/// A pattern node: a required AST shape.
///
/// * `text = Some(s)` — this node's source text must equal `s` exactly (an operation name, a
///   keyword, an operator: the part of the rule that is the rule).
/// * `text = None`, `bind = Some(id)` — a wildcard for an operand whose identity matters: it matches
///   any node of `kind`, but every wildcard sharing `id` must bind to the SAME source text
///   (co-reference — the same variable used twice).
/// * `text = None`, `bind = None` — a typed wildcard (any literal/operand of `kind`).
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Pat {
    kind: String,
    text: Option<String>,
    bind: Option<u32>,
    children: Vec<Pat>,
}

/// A compiled rule: the essential generalized pattern, plus the language it parses. Serializable, so
/// a packed module carries the exact pattern and reuses it anywhere with no recompilation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RulePattern {
    lang: String,
    pat: Pat,
}

/// An unnamed token that is a real OPERATOR (`..=`, `==`, `+`, `.`) rather than mere bracketing
/// punctuation (`(){}[],;`). Its `kind()` IS its text, so it both distinguishes `..=` from `..` and
/// needs no source to read. These carry meaning a rule turns on, so they are part of the structure.
fn is_operator_token(node: Node) -> bool {
    if node.is_named() {
        return false;
    }
    let k = node.kind();
    !k.is_empty() && k.len() <= 3 && k.chars().all(|c| c.is_ascii_punctuation()) && !"(){}[],;".contains(k)
}

/// The children that carry meaning: named nodes plus operator tokens, in source order. Bracketing
/// punctuation is dropped (noise). Shared by hashing, compiling, and matching so all three agree on
/// "what the tree IS".
fn meaningful_children<'t>(node: Node<'t>) -> Vec<Node<'t>> {
    let mut cur = node.walk();
    node.children(&mut cur).filter(|c| c.is_named() || is_operator_token(*c)).collect()
}

/// Canonical hashes of every subtree's SHAPE — node kinds plus operators, in order —
/// text-independent for names/literals but KEEPING operators (so `0..=n` and `0..n` hash
/// differently), memoized by node id in ONE bottom-up pass. The former per-node recursive string
/// build was O(n²) and burned hours of CPU when a scraped "example" was a whole manual page.
struct Shapes(HashMap<usize, u64>);

impl Shapes {
    /// One post-order pass: a node's shape hash folds its kind over its children's hashes.
    fn of(root: Node) -> Shapes {
        let mut map = HashMap::new();
        fn fill(node: Node, map: &mut HashMap<usize, u64>) -> u64 {
            let mut h = crate::lint_ai::token_seed(node.kind());
            for c in meaningful_children(node) {
                h = h.rotate_left(7) ^ fill(c, map).wrapping_mul(0x9E3779B97F4A7C15);
            }
            map.insert(node.id(), h);
            h
        }
        fill(root, &mut map);
        Shapes(map)
    }

    /// The memoized shape hash of `node` (must belong to the tree this was built from).
    fn hash(&self, node: Node) -> u64 {
        self.0[&node.id()]
    }
}

/// Collect the shape hash of every subtree under `node`.
fn collect_shapes(node: Node, shapes: &Shapes, out: &mut HashSet<u64>) {
    out.insert(shapes.hash(node));
    for c in meaningful_children(node) {
        collect_shapes(c, shapes, out);
    }
}

/// For every node in the fix, its kind paired with the sorted multiset of its meaningful children's
/// shape hashes. This lets the localizer recognize a construct the fix kept intact but added
/// siblings INTO (a None-guard, an early return, a `try`/`except` wrap): such a node's own contents
/// are unchanged, so it is incidental context, not the violation — the change is in a sibling.
fn collect_child_shapes(node: Node, shapes: &Shapes, out: &mut Vec<(String, Vec<u64>)>) {
    let kids = meaningful_children(node);
    let mut hashes: Vec<u64> = kids.iter().map(|k| shapes.hash(*k)).collect();
    hashes.sort_unstable();
    out.push((node.kind().to_string(), hashes));
    for c in kids {
        collect_child_shapes(c, shapes, out);
    }
}

/// Whether the sorted multiset `sub` is contained in the sorted multiset `sup`.
fn is_submultiset(sub: &[u64], sup: &[u64]) -> bool {
    let mut counts: HashMap<u64, i32> = HashMap::new();
    for s in sup {
        *counts.entry(*s).or_default() += 1;
    }
    sub.iter().all(|s| match counts.get_mut(s) {
        Some(c) if *c > 0 => {
            *c -= 1;
            true
        }
        _ => false,
    })
}

/// True when the fix kept this exact construct and only INSERTED siblings inside it: some fix node of
/// the same kind has a child-shape multiset that STRICTLY contains this node's. The construct's own
/// children are all preserved by the fix, so the violation is not here — it is in a sibling subtree
/// (the `target=[]` default that sits beside the body the fix only wrapped in a guard). Without this,
/// a fix that adds a guard makes the body itself look novel, and the localizer over-captures the
/// whole unit into a pattern so literal that a stray docstring or log line defeats the match.
fn fix_only_inserted(node: Node, shapes: &Shapes, good_children: &[(String, Vec<u64>)]) -> bool {
    let kids = meaningful_children(node);
    if kids.is_empty() {
        return false;
    }
    let mut want: Vec<u64> = kids.iter().map(|k| shapes.hash(*k)).collect();
    want.sort_unstable();
    good_children
        .iter()
        .any(|(kind, have)| kind == node.kind() && have.len() > want.len() && is_submultiset(&want, have))
}

/// The SMALLEST subtree of `node` carrying the distinction from the fix: the deepest named node that
/// is novel (its shape is absent from `good_shapes`) yet sits over children the fix DOES share — so
/// the difference is localized right here. This is what isolates `0..=W.len()` from a whole function
/// (the operator diff would otherwise bubble all the way up), while still keeping the function scope
/// for a `break` (because the break-block shape IS shared with the loop fix, descent stops above it).
fn novel_root<'t>(
    node: Node<'t>,
    shapes: &Shapes,
    good_shapes: &HashSet<u64>,
    good_kinds: &HashSet<String>,
    good_children: &[(String, Vec<u64>)],
) -> Option<Node<'t>> {
    if good_shapes.contains(&shapes.hash(node)) {
        return None; // shape shared with the fix → incidental context, not the violation
    }
    if fix_only_inserted(node, shapes, good_children) {
        return None; // the fix only added siblings here → this construct is not the violation
    }
    let mut cur = node.walk();
    let novel: Vec<Node> = node
        .named_children(&mut cur)
        .filter(|c| novel_root(*c, shapes, good_shapes, good_kinds, good_children).is_some())
        .collect();
    // Descend into the single differing child ONLY when this node's KIND survives in the fix — i.e.
    // the construct is preserved and only its content changed (a `range_expression` `..=`→`..`). If
    // the fix REPLACED this kind (a `lambda` assignment became a `def`, so `assignment` is absent
    // from the fix), the construct itself is the violation — keep it, don't strip its context. Zero
    // or several novel children ⇒ the change is at/across this node ⇒ stop here.
    // A call is atomic — its callee IS the rule's identity (`range`, `re.sub`); never strip it by
    // descending into its arguments. So stop at a call even if the change is in an argument.
    let atomic = matches!(node.kind(), "call" | "call_expression" | "macro_invocation");
    // A CHILDLESS novel child never becomes the root (native/architecture.dx, "Compile"): a bare leaf
    // is a degenerate pattern by definition, and stripping its context is what turned
    // `items=[]`-as-default-parameter into "any empty list literal" — this node is the
    // smallest construct that still says WHERE the leaf lives.
    let leaf_child = novel.len() == 1 && meaningful_children(novel[0]).is_empty();
    if novel.len() == 1 && good_kinds.contains(node.kind()) && !atomic && !leaf_child {
        novel_root(novel[0], shapes, good_shapes, good_kinds, good_children)
    } else {
        Some(node)
    }
}

/// With no fix to diff against, a documented bad example often shows the SAME anti-pattern more than
/// once (clippy lists several instances: `if x == true {}` / `if y == false {}`). Keeping the whole
/// multi-statement root then builds a pattern that demands every instance at once — so brittle that
/// not even the example's own reuse matches it. When every meaningful child of `node` shares one
/// shape, they ARE the one violation repeated, so descend to a single representative instance. A node
/// whose children differ in shape is left intact (we cannot localize a real difference without a fix).
fn collapse_repeated<'t>(node: Node<'t>, shapes: &Shapes) -> Node<'t> {
    let kids = meaningful_children(node);
    if kids.len() < 2 {
        return node;
    }
    let first = shapes.hash(kids[0]);
    if kids.iter().all(|k| shapes.hash(*k) == first) {
        return collapse_repeated(kids[0], shapes);
    }
    node
}

/// Render `node`'s source with every literal leaf masked by its kind. Two same-shape doc instances
/// that are identical under this masking differ only in literal VALUES.
fn literal_masked_text(node: Node, src: &[u8]) -> String {
    let kind = node.kind();
    if is_literal_kind(kind) {
        return kind.to_string();
    }
    let kids = meaningful_children(node);
    if kids.is_empty() {
        return node.utf8_text(src).unwrap_or("").trim().to_string();
    }
    kids.iter().map(|k| literal_masked_text(*k, src)).collect::<Vec<_>>().join(" ")
}

/// Whether a fix-less bad example shows several same-shape instances that differ ONLY in literal
/// values (`"{".format(foo)` vs `" {} ".format(foo)`) — the docs are saying the VALUE is the rule
/// (an invalid format string, a specific constant). Structure cannot represent value semantics, so
/// the AST path must abstain rather than compile a pattern that matches every instance of the
/// shape, valid or not. Descends the same way [`collapse_repeated`] does.
fn value_dependent(node: Node, shapes: &Shapes, src: &[u8]) -> bool {
    let kids = meaningful_children(node);
    if kids.len() < 2 {
        return kids.first().map(|k| value_dependent(*k, shapes, src)).unwrap_or(false);
    }
    let first_shape = shapes.hash(kids[0]);
    if !kids.iter().all(|k| shapes.hash(*k) == first_shape) {
        return false;
    }
    let m0 = literal_masked_text(kids[0], src);
    let t0 = kids[0].utf8_text(src).unwrap_or("");
    let masked_same = kids.iter().all(|k| literal_masked_text(*k, src) == m0);
    let text_differs = kids.iter().any(|k| k.utf8_text(src).unwrap_or("") != t0);
    if masked_same && text_differs {
        return true;
    }
    value_dependent(kids[0], shapes, src)
}

/// Collect every node kind under `node`.
fn collect_kinds(node: Node, out: &mut HashSet<String>) {
    out.insert(node.kind().to_string());
    for c in meaningful_children(node) {
        collect_kinds(c, out);
    }
}

/// True when an identifier node names an OPERATION (kept exact), not an operand (generalized): the
/// `function` of a call, an attribute/field/method name, a macro name. Everything else that is an
/// identifier is a variable/operand and becomes a bound wildcard. This is the one general rule that
/// decides "what is the rule" vs "what is incidental", with no per-rule knowledge.
fn is_operation_name(node: Node) -> bool {
    let Some(parent) = node.parent() else { return false };
    let is_field = |names: &[&str]| {
        names
            .iter()
            .find_map(|n| parent.child_by_field_name(n))
            .map(|f| f.id())
            == Some(node.id())
    };
    match parent.kind() {
        // The accessed member is the operation; the receiver (`xs` in `xs.len()`) is an operand.
        "attribute" | "field_expression" | "member_expression" => is_field(&["attribute", "field", "property"]),
        // `f(...)` — the callee is the operation, the arguments are operands.
        "call" | "call_expression" => is_field(&["function"]),
        "scoped_identifier" => is_field(&["name"]),
        "macro_invocation" => true,
        _ => false,
    }
}

/// Identifier-like node kinds whose text names a variable/operand (candidate for a bound wildcard).
fn is_identifier_kind(kind: &str) -> bool {
    matches!(kind, "identifier" | "type_identifier" | "field_identifier" | "shorthand_property_identifier")
}

/// Literal node kinds whose VALUE is incidental — generalized to a typed wildcard (any literal of
/// that kind), so a rule about `"…".join(...)` is not pinned to the example's exact string.
fn is_literal_kind(kind: &str) -> bool {
    kind.contains("string") || kind.contains("integer") || kind.contains("float") || kind.contains("number")
}

/// True when `text` is named by the rule's `desc` (lowercased) — the rule is explicitly ABOUT this
/// value (`"0.0.0.0"`, the `xml.sax` module), so it is essential and kept exact, not generalized.
/// This is the docs themselves disambiguating "the value IS the rule" from "the value is incidental".
fn named_in_desc(text: &str, desc: &str) -> bool {
    let t = text.trim_matches(|c| c == '"' || c == '\'' || c == '`').to_lowercase();
    t.len() >= 2 && desc.contains(&t)
}

/// Compile a code node into a generalized pattern. Operands (variables) become bound wildcards;
/// operations/keywords/operators stay exact; literals are typed wildcards UNLESS the rule's `desc`
/// names their value (then they are essential and kept exact). `binds` co-references repeated vars.
fn compile(node: Node, src: &[u8], desc: &str, binds: &mut HashMap<String, u32>) -> Pat {
    let kind = node.kind().to_string();
    // An operator token's kind IS its text, so a kind match alone pins it exactly — a typed
    // wildcard of that kind matches only that operator.
    if is_operator_token(node) {
        return Pat { kind, text: None, bind: None, children: Vec::new() };
    }
    let own_text = node.utf8_text(src).unwrap_or("");

    // A literal is one leaf (a string in Python's grammar has start/content/end children — descend
    // and they would be generalized away). Its VALUE matters only when the rule NAMES it (`0.0.0.0`),
    // else any literal of the kind matches.
    if is_literal_kind(&kind) {
        let text = named_in_desc(own_text, desc).then(|| own_text.to_string());
        return Pat { kind, text, bind: None, children: Vec::new() };
    }
    // A bare operand identifier → bound wildcard (co-reference by name) UNLESS the rule names it
    // (e.g. an imported module `xml.sax`), in which case it is the rule's subject and kept exact.
    if is_identifier_kind(&kind) && !is_operation_name(node) {
        if named_in_desc(own_text, desc) {
            return Pat { kind, text: Some(own_text.to_string()), bind: None, children: Vec::new() };
        }
        let next = binds.len() as u32;
        let id = *binds.entry(own_text.to_string()).or_insert(next);
        return Pat { kind, text: None, bind: Some(id), children: Vec::new() };
    }
    let kids = meaningful_children(node);
    // Leaf with meaning (operation name, keyword): keep its exact text.
    let text = if kids.is_empty() {
        Some(own_text.trim().to_string()).filter(|t| !t.is_empty())
    } else {
        None
    };
    let children = kids.iter().map(|c| compile(*c, src, desc, binds)).collect();
    Pat { kind, text, bind: None, children }
}

/// A collection-literal node kind (`[]`, `{}`, `(a, b)`) across the grammars we parse. Unlike a bare
/// identifier or operator, a collection literal is a CONCRETE construct a rule can turn on (a mutable
/// default argument, a list where a generator belongs), so it counts as an anchoring identity even as
/// a typed wildcard — the rule is "a value of this kind in this slot".
fn is_container_kind(kind: &str) -> bool {
    matches!(kind, "list" | "dictionary" | "set" | "tuple" | "array" | "object" | "array_expression")
}

/// Whether `pat` keeps at least one anchoring IDENTITY — a leaf whose retained text carries a word (a
/// method/operation name, a keyword, or a doc-named literal like `0.0.0.0`), or a collection literal
/// ([`is_container_kind`]). Operators and punctuation are exact-by-kind but not an identity (too
/// common), so they do not count. A pattern with no anchor is pure structure-plus-wildcards and would
/// match a generic shape; it has no rule to match and abstains. (A pattern that does anchor but is
/// still too broad is caught downstream by the self-test against the docs' own good examples.)
fn has_named_anchor(pat: &Pat) -> bool {
    has_text_anchor(pat) || is_container_kind(&pat.kind) || pat.children.iter().any(has_named_anchor)
}

/// Deepest pattern nesting a compiled rule may keep. A real anti-pattern is a construct a reader can
/// point at — a handful of levels; a pattern this deep only comes from a docs page whose "example"
/// is a whole sample program, which is not a rule (and whose JSON form would exceed serde_json's
/// 128-level recursion limit — each `Pat` level nests two JSON levels — making the cached model
/// unloadable). Compile abstains on such monsters.
const MAX_PATTERN_DEPTH: usize = 48;

/// Whether any node of the pattern kept an EXACT token from its example (operations, keywords,
/// operators stay exact; variables and literals generalize to wildcards). A pattern with no
/// anchor anywhere matches purely by shape — "any method call" — and cannot discriminate.
fn pat_has_anchor(pat: &Pat) -> bool {
    pat.text.is_some() || pat.children.iter().any(pat_has_anchor)
}

/// The nesting depth of a pattern (a leaf is 1).
fn pat_depth(pat: &Pat) -> usize {
    1 + pat.children.iter().map(pat_depth).max().unwrap_or(0)
}

/// Whether `pat` keeps an EXACT-TEXT anchor anywhere — a retained operation name, keyword, or
/// doc-named literal. A pattern without one is anchored only by container kinds; several distinct
/// rules can share that identity (a no-arrays ban and a membership-test rule both reduce to a bare
/// `list`), so such matches are imprecise and the live path arbitrates them through the Hv concept
/// gate instead of reporting them directly.
fn has_text_anchor(pat: &Pat) -> bool {
    pat.text.as_deref().is_some_and(|t| t.chars().any(|c| c.is_ascii_alphanumeric()))
        || pat.children.iter().any(has_text_anchor)
}

impl RulePattern {
    /// Build a rule pattern from its documented `bad` example and (optional) `good` fix, in `lang`.
    /// `desc` is the rule's English description: a literal/name it mentions is kept exact (the rule
    /// is about that value), everything else generalizes. Returns `None` when the example does not
    /// parse or carries no distinctive structure.
    pub fn compile(lang: &str, bad: &str, good: &str, desc: &str) -> Option<RulePattern> {
        // A real anti-pattern is a construct a reader can point at; an "example" the size of a
        // program listing is a scraped page, not a rule — abstain before spending any parse.
        if bad.len() > MAX_EXAMPLE_BYTES {
            return None;
        }
        let language = language(lang)?;
        let mut parser = Parser::new();
        parser.set_language(&language).ok()?;
        let bad_tree = parser.parse(bad, None)?;
        let bad_shapes = Shapes::of(bad_tree.root_node());
        let mut good_shapes = HashSet::new();
        let mut good_kinds = HashSet::new();
        let mut good_children = Vec::new();
        if !good.trim().is_empty() && good.len() <= MAX_EXAMPLE_BYTES {
            if let Some(gt) = parser.parse(good, None) {
                let gs = Shapes::of(gt.root_node());
                collect_shapes(gt.root_node(), &gs, &mut good_shapes);
                collect_kinds(gt.root_node(), &mut good_kinds);
                collect_child_shapes(gt.root_node(), &gs, &mut good_children);
            }
        }
        // With a fix to diff against, isolate the smallest distinguishing construct. With no fix,
        // we cannot localize — keep the whole bad construct (its context, e.g. a `break`'s scope).
        let root = if good_shapes.is_empty() {
            // Same-shape instances differing only in literal values ⇒ the value is the rule;
            // structure cannot learn it — abstain (the gated description path takes over).
            if value_dependent(bad_tree.root_node(), &bad_shapes, bad.as_bytes()) {
                return None;
            }
            collapse_repeated(bad_tree.root_node(), &bad_shapes)
        } else {
            novel_root(bad_tree.root_node(), &bad_shapes, &good_shapes, &good_kinds, &good_children)?
        };
        // Skip past trivial single-child wrappers (module / expression_statement) to the construct.
        let mut node = root;
        while node.named_child_count() == 1 && matches!(node.kind(), "module" | "program" | "source_file" | "expression_statement" | "block") {
            node = node.named_child(0).unwrap();
        }
        // Still the translation unit after wrapper-skipping ⇒ several top-level constructs
        // remain: this "example" is a SAMPLE PROGRAM the docs shipped (a tutorial's hello
        // world), not a pointable anti-pattern — a rule is a construct a reader can point at,
        // never a whole file. Compiling it would flag every program of the same overall shape
        // (tutorial narration once minted `first_statement_in_a_go`, which fired on any
        // hello-world). Same spirit as MAX_PATTERN_DEPTH / MAX_EXAMPLE_BYTES; abstain.
        if matches!(node.kind(), "module" | "program" | "source_file" | "translation_unit") {
            return None;
        }
        let mut binds = HashMap::new();
        let mut pat = compile(node, bad.as_bytes(), &desc.to_lowercase(), &mut binds);
        // A container literal whose elements carry no anchor is the rule ITSELF — "a value of this
        // kind in this slot" — not a container of exactly those N element kinds. Keep just the
        // container, so `[90, 85, 77]` and `["a", "b"]` both match a no-arrays rule.
        if is_container_kind(&pat.kind) && !pat.children.iter().any(has_named_anchor) {
            pat.children.clear();
        }
        // A pattern that is a lone wildcard or a single bare leaf carries no rule — abstain.
        // A bare container literal is the exception: the container kind IS its identity.
        if pat.children.is_empty() && pat.text.is_none() && !is_container_kind(&pat.kind) {
            return None;
        }
        // The rule's IDENTITY is the named tokens it turns on — a method/operation name, a keyword,
        // or a literal the docs name (`len`, `true`, `break`, `re.sub`, `0.0.0.0`). A pattern that
        // generalized down to pure structure plus wildcards, with no such anchor, matches a generic
        // shape (`let x = a::b::c` for `absolute_paths`) rather than its own rule — it has no
        // identity to match, so it abstains. Operators/punctuation alone (`::`, `=`) are not an
        // identity: they are too common to be the rule. This is the docs deciding what is learnable
        // from a single example: a rule whose essence is types or dataflow leaves no syntactic
        // anchor and is correctly not learned here.
        if !has_named_anchor(&pat) {
            return None;
        }
        // A pattern deeper than any pointable construct is a docs sample program, not a rule —
        // and it would not survive the JSON cache round-trip. Abstain.
        if pat_depth(&pat) > MAX_PATTERN_DEPTH {
            return None;
        }
        Some(RulePattern { lang: lang.to_string(), pat })
    }

    /// Whether this pattern carries an exact-text anchor (see [`has_text_anchor`]). Container-only
    /// patterns return false and are treated as imprecise by [`super::RuleSet::flag`].
    pub(super) fn text_anchored(&self) -> bool {
        has_text_anchor(&self.pat)
    }

    /// Whether the pattern's own SHAPE can vouch for it at the reference-fire gate: nested at
    /// least two levels AND keeping at least one exact token from its example. Anything less is
    /// degenerate — a bare leaf or an all-wildcard shape — and the reference corpus is the only
    /// witness left, so [`super::RuleSet::build`] holds it to the stricter bar.
    pub(super) fn structured(&self) -> bool {
        pat_depth(&self.pat) >= 2 && pat_has_anchor(&self.pat)
    }

    /// Every 1-based line in `code` where the rule's pattern occurs (exact sub-tree match with
    /// consistent variable binding). Empty when the rule does not apply — deterministically.
    pub fn matches(&self, code: &str) -> Vec<usize> {
        let Some(language) = language(&self.lang) else { return Vec::new() };
        let mut parser = Parser::new();
        if parser.set_language(&language).is_err() {
            return Vec::new();
        }
        let Some(tree) = parser.parse(code, None) else { return Vec::new() };
        self.matches_in(tree.root_node(), code.as_bytes())
    }

    /// [`RulePattern::matches`] against an ALREADY-PARSED tree — the whole-project pass parses
    /// each file once and runs every rule over the same tree, instead of paying rules×files
    /// parses (which was the dominant cost of a warm run).
    pub(super) fn matches_in(&self, root: Node, src: &[u8]) -> Vec<usize> {
        let mut hits = Vec::new();
        find(root, &self.pat, src, &mut hits);
        hits
    }

    /// The only node kind this pattern's root can match ([`match_at`] rejects every other kind
    /// first) — the index key that lets [`super::RuleSet::flag`] walk each file's tree ONCE and
    /// try at every node only the patterns rooted at that node's kind, instead of paying one
    /// full-tree walk per rule.
    pub(super) fn root_kind(&self) -> &str {
        &self.pat.kind
    }

    /// Whether the pattern matches rooted exactly at `node` — no descent; the caller owns the
    /// walk. Identical decision to [`Self::matches_in`] at a single position.
    pub(super) fn matches_at(&self, node: Node, src: &[u8]) -> bool {
        let mut binds: HashMap<u32, String> = HashMap::new();
        match_at(node, &self.pat, src, &mut binds)
    }
}

// ── HLM1 binary codec (native/architecture.dx, "Save") — the pattern tree is pure structure/text. ──

impl crate::lint_codec::Bin for Pat {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.str(&self.kind);
        self.text.enc(e);
        self.bind.enc(e);
        self.children.enc(e);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<Pat> {
        Some(Pat {
            kind: d.str()?,
            text: Option::dec(d)?,
            bind: Option::dec(d)?,
            children: Vec::dec(d)?,
        })
    }
}

impl crate::lint_codec::Bin for RulePattern {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.str(&self.lang);
        self.pat.enc(e);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<RulePattern> {
        Some(RulePattern { lang: d.str()?, pat: Pat::dec(d)? })
    }
}

/// Try the pattern at `node` and recurse into children, collecting match lines.
fn find(node: Node, pat: &Pat, src: &[u8], hits: &mut Vec<usize>) {
    let mut binds: HashMap<u32, String> = HashMap::new();
    if match_at(node, pat, src, &mut binds) {
        hits.push(node.start_position().row + 1);
    }
    let mut cur = node.walk();
    for c in node.children(&mut cur) {
        find(c, pat, src, hits);
    }
}

/// Exact match of one pattern node against one code node, threading variable bindings.
fn match_at(node: Node, pat: &Pat, src: &[u8], binds: &mut HashMap<u32, String>) -> bool {
    if node.kind() != pat.kind {
        return false;
    }
    if let Some(id) = pat.bind {
        // A bound wildcard: any node of this kind, but the same id must always be the same text.
        let text = node.utf8_text(src).unwrap_or("").to_string();
        return match binds.get(&id) {
            Some(prev) => prev == &text,
            None => {
                binds.insert(id, text);
                true
            }
        };
    }
    if let Some(t) = &pat.text {
        return node.utf8_text(src).map(|x| x.trim() == t).unwrap_or(false);
    }
    if pat.children.is_empty() {
        return true; // typed wildcard (any literal/operand/operator of this kind)
    }
    // Structural node: its meaningful children must match the pattern's children in order.
    let kids = meaningful_children(node);
    if kids.len() != pat.children.len() {
        return false;
    }
    kids.iter().zip(&pat.children).all(|(c, p)| match_at(*c, p, src, binds))
}

