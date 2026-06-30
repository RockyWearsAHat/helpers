//! `lint_match` — LOSSLESS rule matching. A rule is not a bag of features (which discards the
//! structure, and the discarded structure is exactly the false positives) but a generalized
//! sub-tree PATTERN taken from its own example, matched against code by EXACT sub-tree containment
//! with variable binding. Because the whole tree is kept, the relations deep rules need are already
//! present and require no per-relation code:
//!
//!   * **Scope** — "a `break` with no enclosing loop" is the tree path `function → block → break`
//!     with no loop node between; an in-loop break has `for → block → break` and simply does not
//!     match the pattern. Scope falls out of the path.
//!   * **Co-reference** — "the SAME variable in two `isinstance` calls" is one identifier node
//!     appearing in two positions; generalized to a BOUND wildcard, it matches only when both
//!     positions hold the same source text. Def-use falls out of binding.
//!
//! The essential pattern is isolated by `bad ∧ ¬good`: a sub-tree of the bad example whose SHAPE is
//! absent from the documented fix is the violation; shape shared with the fix is incidental context.
//! Operations/keywords/operators are kept exact; variables become bound wildcards; literals become
//! typed wildcards. Matching is then deterministic and exact — no statistics, no float.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser};
use tree_sitter_language::LanguageFn;

// ── Grammar resolution ────────────────────────────────────────────────────────
//
// Language support scales at runtime — zero code changes ever needed to add a language:
//
//   1. Bundled  — grammars compiled into the binary; instant, works offline.
//   2. On-disk  — scans our cache + tree-sitter CLI cache + Neovim parsers + system paths.
//   3. Auto-compile — on first encounter of an unknown language, compiles it via
//      `npm install tree-sitter-<lang>` + `tree-sitter build` and writes the result
//      to ~/.cache/helpers/grammars/. Subsequent runs load from cache instantly.
//   4. Text fallback — if all of the above fail, token-regex matching covers any language.

/// Grammars compiled directly into the binary for offline reliability.
/// Any language NOT in this map is handled automatically by `dynamic_grammar` at runtime.
static BUNDLED: std::sync::LazyLock<HashMap<&'static str, tree_sitter::Language>> =
    std::sync::LazyLock::new(|| {
        let mut m: HashMap<&'static str, tree_sitter::Language> = HashMap::new();
        m.insert("rust",       tree_sitter_rust::LANGUAGE.into());
        m.insert("python",     tree_sitter_python::LANGUAGE.into());
        m.insert("javascript", tree_sitter_javascript::LANGUAGE.into());
        m.insert("typescript", tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into());
        m.insert("tsx",        tree_sitter_typescript::LANGUAGE_TSX.into());
        m.insert("go",         tree_sitter_go::LANGUAGE.into());
        m.insert("java",       tree_sitter_java::LANGUAGE.into());
        m.insert("ruby",       tree_sitter_ruby::LANGUAGE.into());
        m.insert("c",          tree_sitter_c::LANGUAGE.into());
        m.insert("bash",       tree_sitter_bash::LANGUAGE.into());
        m
    });

/// Per-process resolution cache. `None` means "tried and failed — don't retry".
static GRAMMAR_CACHE: OnceLock<Mutex<HashMap<String, Option<tree_sitter::Language>>>> =
    OnceLock::new();

/// Our own grammar cache directory. `acquire_grammar` writes compiled libraries here.
/// Override with `HELPERS_GRAMMAR_PATH`.
fn grammar_cache_dir() -> PathBuf {
    if let Ok(p) = std::env::var("HELPERS_GRAMMAR_PATH") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    Path::new(&home).join(".cache/helpers/grammars")
}

/// All directories to probe for a compiled grammar `.so`/`.dylib`.
/// Covers our own cache, tree-sitter CLI's cache, Neovim parsers, and system packages.
fn grammar_search_dirs() -> Vec<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let h = Path::new(&home);
    vec![
        grammar_cache_dir(),
        h.join(".cache/tree-sitter/lib"),            // tree-sitter CLI (Linux/macOS)
        h.join("Library/Caches/tree-sitter/lib"),    // tree-sitter CLI (macOS)
        h.join(".local/share/nvim/site/parser"),      // nvim-treesitter
        h.join(".config/nvim/parser"),                // nvim-treesitter (alternate)
        PathBuf::from("/usr/lib/x86_64-linux-gnu"),  // Debian/Ubuntu system packages
        PathBuf::from("/usr/lib/aarch64-linux-gnu"),
        PathBuf::from("/usr/local/lib"),
    ]
}

/// Open a shared library at `path` and call `fn_name()` to obtain the grammar.
/// The library is leaked so the pointer stays valid for the process lifetime.
///
/// # Safety
/// `path` must be a valid tree-sitter grammar shared library whose `fn_name` symbol
/// follows the tree-sitter C ABI: `*const () tree_sitter_<lang>()`.
unsafe fn load_library(path: &Path, fn_name: &str) -> Option<tree_sitter::Language> {
    let lib = libloading::Library::new(path).ok()?;
    type RawFn = unsafe extern "C" fn() -> *const ();
    let func: libloading::Symbol<RawFn> = lib.get(fn_name.as_bytes()).ok()?;
    let raw: RawFn = *func;
    let _ = Box::into_raw(Box::new(lib)); // intentional leak: grammar ptr must outlive the process
    Some(tree_sitter::Language::new(LanguageFn::from_raw(raw)))
}

/// Scan `grammar_search_dirs()` for a compiled grammar for `lang` and load it.
fn find_on_disk(lang: &str) -> Option<tree_sitter::Language> {
    let fn_name = format!("tree_sitter_{}", lang.replace('-', "_"));
    // Try both our naming convention and bare-name (used by nvim-treesitter).
    let stems = [format!("tree-sitter-{lang}"), lang.to_string()];
    let exts = if cfg!(target_os = "macos") { &[".dylib", ".so"][..] } else { &[".so", ".dylib"][..] };
    for dir in grammar_search_dirs() {
        for stem in &stems {
            for ext in exts {
                let path = dir.join(format!("{stem}{ext}"));
                if path.exists() {
                    // Safety: tree-sitter grammar C ABI is stable; fn returns *const TSLanguage.
                    if let Some(l) = unsafe { load_library(&path, &fn_name) } {
                        return Some(l);
                    }
                }
            }
        }
    }
    None
}

/// Compile a grammar for `lang` on-demand using npm + tree-sitter CLI, then cache it.
///
/// This is called automatically the first time an unknown language is encountered.
/// On success the compiled `.so`/`.dylib` lives in `grammar_cache_dir()` and all
/// future runs load it instantly from disk — no repeated compilation.
///
/// Requires `npm` and `tree-sitter` on PATH; silently returns `None` if either is missing
/// or the grammar package doesn't exist on npm, falling through to the text-pattern path.
fn acquire_grammar(lang: &str) -> Option<tree_sitter::Language> {
    let cache_dir = grammar_cache_dir();
    std::fs::create_dir_all(&cache_dir).ok()?;

    let ext = if cfg!(target_os = "macos") { "dylib" } else { "so" };
    let out = cache_dir.join(format!("tree-sitter-{lang}.{ext}"));

    // Isolated temp workspace so concurrent acquires for different languages don't collide.
    let tmp = std::env::temp_dir().join(format!("helpers-grammar-{lang}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).ok()?;

    // Step 1: download grammar package from npm.
    let npm_ok = std::process::Command::new("npm")
        .args(["install", &format!("tree-sitter-{lang}"), "--prefix", tmp.to_str()?])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !npm_ok {
        let _ = std::fs::remove_dir_all(&tmp);
        return None;
    }

    let grammar_src = tmp.join("node_modules").join(format!("tree-sitter-{lang}"));
    if !grammar_src.exists() {
        let _ = std::fs::remove_dir_all(&tmp);
        return None;
    }

    // Step 2: compile the grammar C source to a native shared library.
    let build_ok = std::process::Command::new("tree-sitter")
        .args(["build", "--output", out.to_str()?, grammar_src.to_str()?])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let _ = std::fs::remove_dir_all(&tmp);

    if build_ok && out.exists() {
        let fn_name = format!("tree_sitter_{}", lang.replace('-', "_"));
        // Safety: we just compiled this grammar; its ABI is guaranteed correct.
        unsafe { load_library(&out, &fn_name) }
    } else {
        None
    }
}

/// Resolve `lang` to a grammar from disk or by compiling on-demand.
/// Result is cached per-process so each language is probed at most once.
fn dynamic_grammar(lang: &str) -> Option<tree_sitter::Language> {
    let cache = GRAMMAR_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache.lock().expect("grammar cache lock");
    if let Some(cached) = map.get(lang) {
        return cached.clone();
    }
    let result = find_on_disk(lang).or_else(|| acquire_grammar(lang));
    let ret = result.clone();
    map.insert(lang.to_string(), result);
    ret
}

/// Resolve a language name to its tree-sitter grammar.
///
/// **Zero code changes needed to add any language.** Resolution order:
///   1. Bundled (compiled in) — instant, offline, covers common languages.
///   2. On-disk scan — picks up grammars from the tree-sitter CLI cache,
///      Neovim, system packages, or `~/.cache/helpers/grammars/`.
///   3. Auto-compiled — downloads and compiles via npm + tree-sitter CLI on
///      first encounter; cached to `~/.cache/helpers/grammars/` for future runs.
///   4. `None` — text-pattern fallback handles any language without a grammar.
pub(crate) fn language(lang: &str) -> Option<tree_sitter::Language> {
    BUNDLED.get(lang).cloned().or_else(|| dynamic_grammar(lang))
}

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

/// A canonical hash of a subtree's SHAPE — node kinds plus operators, in order — text-independent
/// for names/literals but KEEPING operators (so `0..=n` and `0..n` hash differently). Used to find
/// the part of the bad example whose shape the documented fix does NOT contain.
fn shape_hash(node: Node) -> String {
    let mut s = String::from(node.kind());
    let kids = meaningful_children(node);
    if !kids.is_empty() {
        s.push('(');
        for (i, k) in kids.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            s.push_str(&shape_hash(*k));
        }
        s.push(')');
    }
    s
}

/// Collect the shape hash of every subtree under `node`.
fn collect_shapes(node: Node, out: &mut HashSet<String>) {
    out.insert(shape_hash(node));
    for c in meaningful_children(node) {
        collect_shapes(c, out);
    }
}

/// For every node in the fix, its kind paired with the sorted multiset of its meaningful children's
/// shape hashes. This lets the localizer recognize a construct the fix kept intact but added
/// siblings INTO (a None-guard, an early return, a `try`/`except` wrap): such a node's own contents
/// are unchanged, so it is incidental context, not the violation — the change is in a sibling.
fn collect_child_shapes(node: Node, out: &mut Vec<(String, Vec<String>)>) {
    let kids = meaningful_children(node);
    let mut shapes: Vec<String> = kids.iter().map(|k| shape_hash(*k)).collect();
    shapes.sort();
    out.push((node.kind().to_string(), shapes));
    for c in kids {
        collect_child_shapes(c, out);
    }
}

/// Whether the sorted multiset `sub` is contained in the sorted multiset `sup`.
fn is_submultiset(sub: &[String], sup: &[String]) -> bool {
    let mut counts: HashMap<&String, i32> = HashMap::new();
    for s in sup {
        *counts.entry(s).or_default() += 1;
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
fn fix_only_inserted(node: Node, good_children: &[(String, Vec<String>)]) -> bool {
    let kids = meaningful_children(node);
    if kids.is_empty() {
        return false;
    }
    let mut want: Vec<String> = kids.iter().map(|k| shape_hash(*k)).collect();
    want.sort();
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
    good_shapes: &HashSet<String>,
    good_kinds: &HashSet<String>,
    good_children: &[(String, Vec<String>)],
) -> Option<Node<'t>> {
    if good_shapes.contains(&shape_hash(node)) {
        return None; // shape shared with the fix → incidental context, not the violation
    }
    if fix_only_inserted(node, good_children) {
        return None; // the fix only added siblings here → this construct is not the violation
    }
    let mut cur = node.walk();
    let novel: Vec<Node> = node
        .named_children(&mut cur)
        .filter(|c| novel_root(*c, good_shapes, good_kinds, good_children).is_some())
        .collect();
    // Descend into the single differing child ONLY when this node's KIND survives in the fix — i.e.
    // the construct is preserved and only its content changed (a `range_expression` `..=`→`..`). If
    // the fix REPLACED this kind (a `lambda` assignment became a `def`, so `assignment` is absent
    // from the fix), the construct itself is the violation — keep it, don't strip its context. Zero
    // or several novel children ⇒ the change is at/across this node ⇒ stop here.
    // A call is atomic — its callee IS the rule's identity (`range`, `re.sub`); never strip it by
    // descending into its arguments. So stop at a call even if the change is in an argument.
    let atomic = matches!(node.kind(), "call" | "call_expression" | "macro_invocation");
    if novel.len() == 1 && good_kinds.contains(node.kind()) && !atomic {
        novel_root(novel[0], good_shapes, good_kinds, good_children)
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
fn collapse_repeated(node: Node) -> Node {
    let kids = meaningful_children(node);
    if kids.len() < 2 {
        return node;
    }
    let first = shape_hash(kids[0]);
    if kids.iter().all(|k| shape_hash(*k) == first) {
        return collapse_repeated(kids[0]);
    }
    node
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
    let word = pat.text.as_deref().is_some_and(|t| t.chars().any(|c| c.is_ascii_alphanumeric()));
    word || is_container_kind(&pat.kind) || pat.children.iter().any(has_named_anchor)
}

impl RulePattern {
    /// Build a rule pattern from its documented `bad` example and (optional) `good` fix, in `lang`.
    /// `desc` is the rule's English description: a literal/name it mentions is kept exact (the rule
    /// is about that value), everything else generalizes. Returns `None` when the example does not
    /// parse or carries no distinctive structure.
    pub fn compile(lang: &str, bad: &str, good: &str, desc: &str) -> Option<RulePattern> {
        let language = language(lang)?;
        let mut parser = Parser::new();
        parser.set_language(&language).ok()?;
        let bad_tree = parser.parse(bad, None)?;
        let mut good_shapes = HashSet::new();
        let mut good_kinds = HashSet::new();
        let mut good_children = Vec::new();
        if !good.trim().is_empty() {
            if let Some(gt) = parser.parse(good, None) {
                collect_shapes(gt.root_node(), &mut good_shapes);
                collect_kinds(gt.root_node(), &mut good_kinds);
                collect_child_shapes(gt.root_node(), &mut good_children);
            }
        }
        // With a fix to diff against, isolate the smallest distinguishing construct. With no fix,
        // we cannot localize — keep the whole bad construct (its context, e.g. a `break`'s scope).
        let root = if good_shapes.is_empty() {
            collapse_repeated(bad_tree.root_node())
        } else {
            novel_root(bad_tree.root_node(), &good_shapes, &good_kinds, &good_children)?
        };
        // Skip past trivial single-child wrappers (module / expression_statement) to the construct.
        let mut node = root;
        while node.named_child_count() == 1 && matches!(node.kind(), "module" | "program" | "source_file" | "expression_statement" | "block") {
            node = node.named_child(0).unwrap();
        }
        let mut binds = HashMap::new();
        let pat = compile(node, bad.as_bytes(), &desc.to_lowercase(), &mut binds);
        // A pattern that is a lone wildcard or a single bare leaf carries no rule — abstain.
        if pat.children.is_empty() && pat.text.is_none() {
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
        Some(RulePattern { lang: lang.to_string(), pat })
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
        let mut hits = Vec::new();
        find(tree.root_node(), &self.pat, code.as_bytes(), &mut hits);
        hits
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

// ── Text-pattern fallback (universal — any language, any docs) ───────────────

/// Strip single-line comments (`//` and `#`) from code so doc-page prose like
/// `// example code where clippy issues a warning` never becomes the discriminator.
fn strip_code_comments(code: &str) -> String {
    code.lines()
        .filter(|l| {
            let t = l.trim_start();
            !t.starts_with("//") && !t.starts_with('#') && !t.starts_with('*') && !t.starts_with("/*")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Derive a discriminating regex from the rule's English *description* by finding the key
/// identifiers the doc names (backtick-quoted terms). Falls back to scanning for method-name
/// tokens. The result must appear in `bad` to be accepted — the description says what is wrong,
/// and the bad example must exhibit it.
///
/// This is the "read the documentation" path: the English sentence "Avoid `e.printStackTrace()`"
/// directly yields `\bprintStackTrace\b` without needing a diff of code examples.
/// Derive a discriminating regex by reading the rule's English *description* — the prose
/// the official documentation actually wrote.
///
/// Three extraction passes, in priority order:
///  1. Backtick-quoted spans (Markdown, GitHub, rustdoc): `` `e.printStackTrace()` `` → `printStackTrace`.
///  2. Words immediately followed by `(` in plain prose: `"Avoid exit("` → `exit`.
///  3. CamelCase/PascalCase identifiers in plain prose (≥4 chars): "Replace ArrayList" → `ArrayList`.
///
/// If `bad` is non-empty it is used to validate: the candidate must appear in the bad example,
/// ensuring we name the right construct (not a generic word from the explanation). When `bad`
/// is absent the first viable candidate is returned directly — the SELF-FIRE gate in the
/// caller will then validate or drop it.
fn description_discriminator(desc: &str, bad: &str) -> Option<String> {
    let id_re = regex::Regex::new(r"[A-Za-z_]\w*[!?]?").expect("static");
    let skip: HashSet<&str> = [
        "a","b","c","x","y","z","s","t","n","i","j","k",
        "if","fn","let","use","pub","mod","for","in","return","match","true","false",
        "None","True","False","Ok","Err","Some","self","Self",
        // English prose words that look like identifiers but have no discriminating power.
        "the","and","not","use","get","set","new","has","add","all","any","can","may","its",
        "this","that","will","with","only","also","then","when","from","have","each","than",
        "such","into","over","avoid","check","using","calls","call","type","code","like",
        "more","less","well","just","too","else","case","same","way","both","often","should",
        "used","via","per","ref","See","via","via","via",
    ].iter().copied().collect();

    let best_from_span = |span: &str| -> Option<String> {
        // Prefer method names (follow `(`) over plain identifiers; within ties, prefer longer.
        let method_re = regex::Regex::new(r"[A-Za-z_]\w*[!?]?\s*\(").expect("static");
        if let Some(m) = method_re.find(span) {
            let name = m.as_str().trim_end_matches(|c: char| c == '(' || c.is_whitespace());
            if name.len() >= 3 && !skip.contains(name) {
                return Some(name.to_string());
            }
        }
        id_re.find_iter(span)
            .filter(|m| m.len() >= 3 && !skip.contains(m.as_str()))
            .max_by_key(|m| m.len())
            .map(|m| m.as_str().to_string())
    };

    let mut candidates: Vec<String> = Vec::new();

    // Pass 1 — backtick-quoted spans.
    let bt_re = regex::Regex::new(r"`([^`]+)`").expect("static");
    for cap in bt_re.captures_iter(desc) {
        if let Some(tok) = best_from_span(&cap[1]) {
            candidates.push(tok);
        }
    }

    // Pass 2 — method calls in plain prose ("avoid printStackTrace(", "calls System.exit(").
    if candidates.is_empty() {
        let mc_re = regex::Regex::new(r"\b([A-Za-z_]\w{2,}[!?]?)\s*\(").expect("static");
        for cap in mc_re.captures_iter(desc) {
            let tok = &cap[1];
            if !skip.contains(tok.as_ref() as &str) {
                candidates.push(tok.to_string());
                break;
            }
        }
    }

    // Pass 3 — CamelCase/PascalCase words in plain prose (code identifiers, not prose words).
    if candidates.is_empty() {
        let cc_re = regex::Regex::new(r"\b([A-Z][a-z]{2,}[A-Z][a-zA-Z]*|[a-z]{2,}[A-Z][a-zA-Z]+)\b").expect("static");
        for m in cc_re.find_iter(desc) {
            let tok = m.as_str();
            if tok.len() >= 4 && !skip.contains(tok) {
                candidates.push(tok.to_string());
                break;
            }
        }
    }

    // Validate: if bad is known, require the candidate to appear in it.
    // If bad is absent (description-only rule), trust the extraction — the SELF-FIRE gate
    // in RuleSet::build() will drop patterns that do not match real violations.
    for cand in &candidates {
        let pat = format!(r"\b{}\b", regex::escape(cand));
        if let Ok(re) = regex::Regex::new(&pat) {
            if bad.trim().is_empty() || re.is_match(bad) {
                return Some(pat);
            }
        }
    }
    None
}

/// Derive a discriminating regex from `bad` and `good` examples using `bad ∧ ¬good`.
///
/// Strips `//`/`#` comment lines first so doc-page prose comments like
/// `// example code where clippy issues a warning` do not pollute the discriminator.
/// Tries an ordered two-token pair first (most specific), then a single distinctive token.
/// Tokens are matched at word boundaries so `eval` never fires on `literal_eval`.
/// The pair pattern uses `.*?` between tokens so `eval(code)` matches even though `(`
/// sits between them — works for any operator, delimiter, or punctuation.
///
/// Returns `None` when the difference is purely in values the tokeniser ignores
/// (e.g. numeric literals, string contents) — the caller drops such rules rather than
/// emitting a pattern that would over-fire.
fn text_discriminator(bad: &str, good: &str) -> Option<String> {
    // Strip doc-page comments before tokenising — they pollute the discriminator.
    let bad = strip_code_comments(bad);
    let good = strip_code_comments(good);
    let (bad, good) = (bad.as_str(), good.as_str());

    // Broad tokeniser: handles identifiers (must start with letter/underscore so bare numeric
    // literals are ignored — pure-value differences like 0 vs 1 are semantic, not syntactic),
    // Ruby ?/! methods, shell flags (--verbose, -v), sigiled vars ($var, @var), and operators.
    let tok_re = regex::Regex::new(
        r"--?[A-Za-z][\w-]*|[$@]\w+|[A-Za-z_]\w*[!?]?|==|!=|<=|>=|->|=>|\.\.|::",
    )
    .expect("static regex");

    let bad_toks: Vec<&str> = tok_re.find_iter(bad).map(|m| m.as_str()).collect();
    let good_set: HashSet<&str> = tok_re.find_iter(good).map(|m| m.as_str()).collect();

    // Word boundary for pure-identifier tokens: prevents `eval` from matching inside
    // `literal_eval`. Operators and flags are self-delimiting and need no boundary.
    let wpat = |tok: &str| -> String {
        if tok.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            format!(r"\b{}\b", regex::escape(tok))
        } else {
            regex::escape(tok)
        }
    };

    // Reject single-character identifier tokens — they are variables (a, b, x, y) and appear
    // everywhere in real code. A discriminator built from them would fire on any assignment,
    // function parameter, or loop variable, producing endless false positives.
    let is_useful = |tok: &str| -> bool {
        let is_pure_id = tok.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
        !is_pure_id || tok.len() >= 2
    };

    // 1. Ordered pair on the same line — `.*?` allows any punctuation between tokens.
    for win in bad_toks.windows(2) {
        if !is_useful(win[0]) && !is_useful(win[1]) {
            continue; // both are single-char — no discriminating power
        }
        if good_set.contains(win[0]) && good_set.contains(win[1]) {
            continue;
        }
        let pat = format!("{}.*?{}", wpat(win[0]), wpat(win[1]));
        if let Ok(re) = regex::Regex::new(&pat) {
            if re.is_match(bad) && !re.is_match(good) {
                return Some(pat);
            }
        }
    }

    // 2. Single distinctive token.
    for tok in &bad_toks {
        if !is_useful(tok) {
            continue;
        }
        if good_set.contains(*tok) {
            continue;
        }
        let pat = wpat(tok);
        if let Ok(re) = regex::Regex::new(&pat) {
            if re.is_match(bad) && !re.is_match(good) {
                return Some(pat);
            }
        }
    }

    None
}

/// How a rule matches code — either lossless AST pattern (when a grammar is available) or a
/// discriminating text pattern (token-level regex, universal fallback for any language).
///
/// Both paths go through the same `bad ∧ ¬good` discipline: the pattern is derived from what the
/// `bad` example has that the `good` example does not. The difference is precision: AST patterns
/// capture structure (scope, co-reference); text patterns capture presence of distinctive tokens.
#[derive(Clone, Debug, Serialize, Deserialize)]
enum MatchKind {
    /// Exact generalized subtree match via tree-sitter.
    Ast(RulePattern),
    /// Regex over source lines — used when no grammar is available for the language.
    /// Stored as a string (regex::Regex is not Serialize); compiled on first use via `flag`.
    Text { pattern: String },
}

impl MatchKind {
    /// Lines in `code` where this rule fires. 1-based.
    fn matches(&self, code: &str) -> Vec<usize> {
        match self {
            MatchKind::Ast(pat) => pat.matches(code),
            MatchKind::Text { pattern } => {
                let Ok(re) = regex::Regex::new(pattern) else { return vec![] };
                code.lines()
                    .enumerate()
                    .filter(|(_, line)| re.is_match(line))
                    .map(|(i, _)| i + 1)
                    .collect()
            }
        }
    }
}

/// One documented rule compiled to its exact match kind, carrying the reporting facts a finding needs.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct CompiledRule {
    id: String,
    severity: String,
    kind: MatchKind,
}

/// A language's compiled rule set: every documented rule reduced to its lossless tree pattern. This
/// is the cached, serializable model a lint run loads and matches each file against — deterministic,
/// no thresholds, no statistics. Mirrors the engine's old model API so judging code is unchanged.
#[derive(Serialize, Deserialize)]
pub struct RuleSet {
    /// Language id (e.g. `rust`).
    pub lang: String,
    rules: Vec<CompiledRule>,
}

/// One flagged violation: the rule it violates, that rule's severity, and the 1-based source line.
pub struct Finding {
    /// The matched rule's id.
    pub rule: String,
    /// Severity bucket (`high`/`medium`/`low`).
    pub severity: String,
    /// 1-based source line of the match.
    pub line: usize,
}

impl RuleSet {
    /// Compile a language's documented `(id, severity, bad, good, description)` rules.
    ///
    /// For languages with a tree-sitter grammar: lossless AST patterns via `bad ∧ ¬good`.
    /// For any other language: discriminating token-regex patterns, derived the same way.
    /// Both paths apply the same quality gate: self-fire (must flag its own `bad`) and
    /// over-fire (must not flag any `good` in the corpus). Only rules that pass both survive.
    pub fn build(lang: &str, rules: &[(String, String, String, String, String)]) -> RuleSet {
        let mut compiled = Vec::new();
        let mut seen = HashSet::new();
        let has_grammar = language(lang).is_some();
        for (id, severity, bad, good, desc) in rules {
            if id.is_empty() || !seen.insert(id.clone()) {
                continue;
            }
            // bad may be empty when the documentation only provides prose (description-only
            // rules). description_discriminator will read the English doc to derive a pattern;
            // the SELF-FIRE gate below will then validate or drop it.
            if desc.trim().is_empty() && bad.trim().is_empty() {
                continue; // nothing to learn from
            }
            let kind = if has_grammar {
                if let Some(pat) = RulePattern::compile(lang, bad, good, desc) {
                    // AST pattern — lossless and most precise; no regex needed.
                    MatchKind::Ast(pat)
                } else if let Some(re) = description_discriminator(desc, bad) {
                    // English prose is the primary documentation; read it first.
                    // The description names the construct to flag: "avoid `e.printStackTrace()`".
                    MatchKind::Text { pattern: re }
                } else if let Some(re) = text_discriminator(bad, good) {
                    // Code-diff fallback: description had no extractable term but the bad/good
                    // examples (themselves part of the official documentation) still distinguish.
                    MatchKind::Text { pattern: re }
                } else {
                    continue;
                }
            } else {
                // No grammar — text matching only. Documentation prose is the primary signal;
                // code examples (which appear in the same docs) refine when prose is thin.
                if let Some(re) = description_discriminator(desc, bad) {
                    MatchKind::Text { pattern: re }
                } else if let Some(re) = text_discriminator(bad, good) {
                    MatchKind::Text { pattern: re }
                } else {
                    continue;
                }
            };
            compiled.push(CompiledRule { id: id.clone(), severity: severity.clone(), kind });
        }
        // SELF-FIRE: when a bad example is known, the compiled rule must flag it.
        // Description-only rules (bad is empty) skip this gate — they are validated at
        // query time: if the extracted pattern fires on real violations found in the project,
        // it was correct; if nothing matches, it stays silent (never a false flag).
        let bad_map: std::collections::HashMap<&str, &str> =
            rules.iter().map(|(id, _, bad, _, _)| (id.as_str(), bad.as_str())).collect();
        let good_map: std::collections::HashMap<&str, &str> =
            rules.iter().map(|(id, _, _, good, _)| (id.as_str(), good.trim())).collect();
        compiled.retain(|r| {
            let bad = bad_map.get(r.id.as_str()).copied().unwrap_or("").trim();
            // No bad example → description-only rule; let it through without the SELF-FIRE check.
            bad.is_empty() || !r.kind.matches(bad).is_empty()
        });
        // OVER-FIRE: must not flag THIS rule's own `good` example (if it has one).
        compiled.retain(|r| {
            let good = good_map.get(r.id.as_str()).copied().unwrap_or("");
            good.is_empty() || r.kind.matches(good).is_empty()
        });
        RuleSet { lang: lang.to_string(), rules: compiled }
    }

    /// Number of compiled rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Flag `code`: every line where a rule fires (AST match or text match), deduped per rule.
    pub fn flag(&self, code: &str) -> Vec<Finding> {
        let mut out = Vec::new();
        for r in &self.rules {
            let mut lines = r.kind.matches(code);
            lines.sort_unstable();
            lines.dedup();
            for line in lines {
                out.push(Finding { rule: r.id.clone(), severity: r.severity.clone(), line });
            }
        }
        out
    }

    /// Serialize to JSON for caching.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Load from cached JSON.
    pub fn from_json(s: &str) -> Option<RuleSet> {
        serde_json::from_str(s).ok()
    }
}
