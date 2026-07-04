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
//!
//! Cross-module theory, evidence hierarchy, and the failure ledger live in `LINTER.md` at the
//! repo root — the single authoritative doc; update it BEFORE changing semantics here.

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

/// True when `name` names a grammar bundled into the binary — a cheap, offline membership probe
/// for "is this word a language, not prose?" questions (e.g. reading a fence info string). Never
/// touches the network or the dynamic-grammar path.
pub fn bundled_language(name: &str) -> bool {
    BUNDLED.contains_key(name)
}

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
    // Offline runs never reach for npm, and a failed acquisition is remembered ON DISK —
    // without this, every run re-paid a network 404 per unknown extension (hundreds of ms
    // each), which alone dwarfed the entire inference pass.
    if std::env::var_os("HELPERS_LINT_OFFLINE").is_some() {
        return None;
    }
    let cache_dir = grammar_cache_dir();
    std::fs::create_dir_all(&cache_dir).ok()?;
    let absent_marker = cache_dir.join(format!("tree-sitter-{lang}.absent"));
    if absent_marker.exists() {
        return None;
    }

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
        let _ = std::fs::write(&absent_marker, b"npm install failed; delete this file to retry\n");
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
        let _ = std::fs::write(&absent_marker, b"grammar build failed; delete this file to retry\n");
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

/// A structural fingerprint of `code`: node-kind path trigrams under `lang`'s grammar (grandparent →
/// parent → node kinds) when a grammar is available, else token trigrams. This is the material for a
/// code example's associative-memory hypervector in [`crate::lint_read`] — it captures the SHAPE of
/// the code (which constructs nest in which), not its exact text, so structurally similar examples
/// bind near each other. Never re-derives a rule; the firing engine still parses code itself.
pub(crate) fn code_ngrams(lang: &str, code: &str) -> Vec<String> {
    if let Some(language) = language(lang) {
        let mut parser = Parser::new();
        if parser.set_language(&language).is_ok() {
            if let Some(tree) = parser.parse(code, None) {
                let mut out = Vec::new();
                collect_kind_paths(tree.root_node(), &mut Vec::new(), &mut out);
                if !out.is_empty() {
                    return out;
                }
            }
        }
    }
    let toks: Vec<&str> = code
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| !t.is_empty())
        .collect();
    toks.windows(3).map(|w| w.join(" ")).collect()
}

/// Emit a `grandparent>parent>node` kind trigram for every node, threading the ancestor kind stack.
fn collect_kind_paths(node: Node, stack: &mut Vec<String>, out: &mut Vec<String>) {
    stack.push(node.kind().to_string());
    let n = stack.len();
    if n >= 3 {
        out.push(format!("{}>{}>{}", stack[n - 3], stack[n - 2], stack[n - 1]));
    }
    let mut cur = node.walk();
    for c in node.named_children(&mut cur) {
        collect_kind_paths(c, stack, out);
    }
    stack.pop();
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
    if novel.len() == 1 && good_kinds.contains(node.kind()) && !atomic {
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

/// Largest documented example that can still BE a rule. A pointable anti-pattern is at most a
/// screenful; anything bigger is a scraped sample program or a whole manual page, which no single
/// rule describes — and which turns compilation (tree diffing, token-pair regex search) into
/// minutes of work for zero yield. Same spirit as [`MAX_PATTERN_DEPTH`]: the cap encodes what a
/// rule IS, not what any language looks like.
const MAX_EXAMPLE_BYTES: usize = 8192;

/// Smallest reference corpus (in lines) the REFERENCE-FIRE gate may judge from. The gate is
/// statistical — "this detector trips on the language's own normal code" — and a handful of
/// grounding examples or a discovery probe cannot testify to that; below this scale the gate
/// stays out of the way.
const REFERENCE_FIRE_MIN_LINES: usize = 500;

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
    /// patterns return false and are treated as imprecise by [`RuleSet::flag`].
    pub(crate) fn text_anchored(&self) -> bool {
        has_text_anchor(&self.pat)
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
    fn matches_in(&self, root: Node, src: &[u8]) -> Vec<usize> {
        let mut hits = Vec::new();
        find(root, &self.pat, src, &mut hits);
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

/// Grounded evidence [`RuleSet::build`] reads English descriptions through: the real-code
/// reference corpus the language's own documentation served, and the learned polarity classifier
/// (which carries the reader and its word-frequency knowledge of the language's prose). Both are
/// LEARNED artifacts — construct selection consults them instead of any authored word list or
/// dictionary, so a miss is fixed by more reading, never by another extraction pass.
#[derive(Default)]
pub struct Grounding {
    /// Real code examples from the language's documentation — the "what's normal" corpus. This
    /// is the ONLY corpus that can ground a LEARNED rule's construct: documented code is code.
    pub reference: Vec<String>,
    /// The project's own sources. They ground and rank the PROJECT'S law (a law names constructs
    /// that live in the code it governs) but never a learned rule — project files carry English
    /// comments and data strings, and teaching vocabulary must not pass as code through them.
    pub project: Vec<String>,
    /// The learned prohibition/endorsement classifier; its reader knows which words are common
    /// connective prose in this language's documentation.
    pub polarity: Option<crate::lint_read::Polarity>,
    /// Ids of rules the PROJECT itself authored (`.helpers/lint-rules/`, root `lintPref`). Their
    /// rule file is the label: everything a user writes there is law by location, so these are
    /// exempt from the prohibition gate exactly as the live path exempts them from the Hv gate.
    pub trusted: std::collections::HashSet<String>,
}

/// [`Grounding`] precomputed for one `RuleSet::build` run: the reference corpus flattened to an
/// identifier set, and the reader borrowed out of the classifier.
struct GroundView<'a> {
    /// Tokens of the documentation's real, comment-stripped code — what grounds a LEARNED rule.
    code_tokens: std::collections::HashSet<String>,
    /// Tokens of the project's own comment-stripped sources — extra ranking evidence for the
    /// project's law only.
    project_tokens: std::collections::HashSet<String>,
    /// The reader whose learned frequencies say which words are common prose.
    reader: Option<&'a crate::lint_read::Reader>,
    /// The full learned classifier — decides whether a description STATES a violation at all.
    polarity: Option<&'a crate::lint_read::Polarity>,
}

impl<'a> GroundView<'a> {
    /// Flatten `g`'s corpora into token sets and borrow its reader and classifier. Comment lines
    /// are dropped first: grounding means "occurs in CODE", and a comment is English inside a
    /// code file — exactly the text that must not launder teaching vocabulary into constructs.
    fn of(g: &'a Grounding) -> GroundView<'a> {
        let tokens_of = |corpus: &[String]| -> std::collections::HashSet<String> {
            let mut out = std::collections::HashSet::new();
            for code in corpus {
                for line in code.lines() {
                    let t = line.trim_start();
                    if t.starts_with("//") || t.starts_with('#') || t.starts_with('*') || t.starts_with("/*") || t.starts_with("--") {
                        continue;
                    }
                    for tok in line.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
                        if tok.len() >= 3 {
                            out.insert(tok.to_lowercase());
                        }
                    }
                }
            }
            out
        };
        GroundView {
            code_tokens: tokens_of(&g.reference),
            project_tokens: tokens_of(&g.project),
            reader: g.polarity.as_ref().map(|p| p.reader()),
            polarity: g.polarity.as_ref(),
        }
    }

    /// The polarity CONTEXT of each whitespace word of `desc`, read along the text: a word's
    /// context is the lean of the nearest word (scanning outward through the reading sequence)
    /// that renders a decisive per-token verdict — its own lean first. No punctuation is
    /// consulted and nothing is chopped: "Do not leave TODO comments; file an issue instead"
    /// places TODO two words from "not" (prohibition context) and "issue" beside
    /// "file…instead" (remedy context) purely by learned leans and adjacency. `None` per word
    /// when no decisive word is in reach; all-`None` when no classifier is ready.
    fn word_contexts(&self, desc: &str) -> Vec<Option<bool>> {
        let words: Vec<&str> = desc.split_whitespace().collect();
        let Some(p) = self.polarity.filter(|p| p.is_ready()) else {
            return vec![None; words.len()];
        };
        // Each word's OWN lean: the first decisive lean among its inner tokens. COMMON words
        // (corpus-scaled cutoff) never project a lean: a register verb like "use" reads
        // decisively endorsing in many languages' docs and would poison its neighbors'
        // contexts ("Never use unsafe…" must not mark `unsafe` as remedy vocabulary) — only
        // informative words carry context, the same reverse-frequency principle the vote
        // weights follow. The cutoff scales with the reading, so a two-page corpus (where
        // nothing is truly common) keeps every lean.
        let common = |w: &&str| -> bool {
            let toks = crate::lint_read::tokens(w);
            !toks.is_empty() && toks.iter().all(|t| p.reader().is_common_word(t))
        };
        let own: Vec<Option<bool>> = words
            .iter()
            .map(|w| {
                if common(w) {
                    return None;
                }
                crate::lint_read::tokens(w).iter().find_map(|t| p.token_lean(t))
            })
            .collect();
        (0..words.len())
            .map(|i| {
                (0..words.len().max(1))
                    .flat_map(|d| [i.checked_sub(d), i.checked_add(d).filter(|j| *j < words.len())])
                    .flatten()
                    .find_map(|j| own[j])
            })
            .collect()
    }
}

/// Derive a discriminating regex by READING the rule's English *description* — the prose the
/// documentation actually wrote. This is how a prose-only rule ("Never call `eval` anywhere")
/// becomes a detector with no code example.
///
/// No shape expectations: no backtick, dotted-path, call-syntax, or morphology rules — those are
/// conventions that cannot be guaranteed forever, and each one the engine expects is a way for a
/// valid law to become invisible. Instead there is ONE tokenization (the sentence's
/// whitespace-delimited words, edge punctuation trimmed, so `console.log`, `8080`, and a
/// backticked word each survive exactly as written) and selection is LEARNED evidence only:
///
///   * words whose inner tokens the reader has absorbed as common English are connective prose
///     and drop out — what the reading cannot account for is the construct;
///   * among the salient words, one grounded in the language's real reference code outranks the
///     rest; then the rarer word (fewest reads) wins; then reading order.
///
/// More reading sharpens the selection — the fix for a wrong pick is never a new shape rule.
/// When `bad` is non-empty every candidate must appear in it — the description says what is
/// wrong and the example must exhibit it. With no reader and no example there is no evidence at
/// all, and the engine ABSTAINS rather than guessing.
///
/// `only_grounded` restricts candidates to words that occur in REAL code (the docs' reference
/// corpus or the project's own sources). Learned rules require it: a rare English word in a
/// principle's imperative clause ("don't over-engineer") is not a code construct, and a detector
/// built from one fires on every comment that discusses the principle. The project's own law
/// passes `false` — the rule file is evidence enough that the named thing is worth watching for.
/// `contexts` is each whitespace word's polarity context ([`GroundView::word_contexts`]): a
/// candidate in a remedy context ("…; use the logging module instead") is the alternative the
/// rule endorses, never its construct, so prohibition-context candidates outrank everything and
/// endorsement-context candidates are dropped outright.
fn description_discriminator(
    desc: &str,
    bad: &str,
    ground: &GroundView,
    contexts: &[Option<bool>],
    only_grounded: bool,
) -> Option<String> {
    let reader = ground.reader?;
    // (surface word, reading position, forbidden-context?, grounded?, rarity = fewest reads
    // among inner tokens). No stop-list and no frequency CUTOFF anywhere: connective prose
    // simply ranks last by its read counts, which stays true at every corpus size — a threshold
    // that felt right at one scale silently dies at another.
    let mut candidates: Vec<(String, usize, bool, bool, bool, u32)> = Vec::new();
    for (position, raw) in desc.split_whitespace().enumerate() {
        let surface = raw.trim_matches(|c: char| !c.is_alphanumeric());
        if surface.chars().count() < 2 {
            continue;
        }
        let context = contexts.get(position).copied().flatten();
        let inner = crate::lint_read::tokens(surface);
        if inner.is_empty() {
            continue;
        }
        // Documented code grounds anyone; the project's own code additionally grounds (and
        // ranks) only when the rule is not held to `only_grounded` — i.e. the project's law.
        let in_docs = inner.iter().any(|t| ground.code_tokens.contains(t));
        if only_grounded && !in_docs {
            continue;
        }
        let in_project = inner.iter().any(|t| ground.project_tokens.contains(t));
        let grounded = in_docs || in_project;
        // Remedy-context vocabulary is endorsed, not forbidden — ineligible. EXCEPT for a
        // project-law word that exists in the project's own code: the author named a word that
        // literally lives in the code they govern, and that existence outweighs the docs
        // register that painted it ("unsafe" reads endorsed all over the rust reference, yet
        // "Never use unsafe blocks" plainly names it). Document order still ranks the earlier
        // violation word above a later grounded remedy word.
        if context == Some(false) && (only_grounded || !grounded) {
            continue;
        }
        let rarity = inner.iter().map(|t| reader.read_count(t)).min().unwrap_or(0);
        candidates.push((surface.to_string(), position, context == Some(true), in_project, grounded, rarity));
    }
    // Ordering: EXISTENCE first (grounding — a construct that never occurs in real code can
    // never fire, and register words like "Never" read as decisively forbidding without being
    // anyone's construct), then understanding (forbidding context), then — for words the
    // reading can account for as connective prose (the corpus head) — last place always. Below
    // that the two rule kinds differ: the PROJECT'S LAW reads like an instruction — the author
    // names the violation before the remedy, so among grounded content words document order
    // decides ("Do not use print…; use logging instead" names `print`, and no rarity score may
    // outbid that), while an ungrounded law construct falls back to rarity (the word the
    // reading can least account for). LEARNED doc prose carries no order promise, so rarity
    // decides there throughout (its candidates are all grounded already, so leading with
    // grounding changes nothing for learned rules).
    let connective = |surface: &str| {
        crate::lint_read::tokens(surface).iter().all(|t| reader.is_head_word(t))
    };
    if only_grounded {
        candidates.sort_by_key(|(surface, position, forbidden, _in_project, grounded, rarity)| {
            (!*grounded, !*forbidden, connective(surface), *rarity, *position)
        });
    } else {
        // The law governs THIS project's code, so a word that exists in the project's own
        // sources is the strongest possible construct evidence — docs-grounded register words
        // ("never" appears in doc example identifiers) must not outrank it by document order.
        candidates.sort_by_key(|(surface, position, forbidden, in_project, grounded, rarity)| {
            let order = if *grounded { *position as u64 } else { *rarity as u64 };
            (!*in_project, !*grounded, !*forbidden, connective(surface), order, *position)
        });
    }

    // Validate: when bad is known the candidate must appear in it; when absent, trust the
    // winner — SELF-FIRE and query-time silence guard a wrong pick. The pattern is
    // case-insensitive on the lowercased surface: prose capitalizes sentence-initial words
    // ("Unsafe blocks are banned…") but the construct in code is whatever case the code uses,
    // and grounding already matched case-normalized tokens.
    for (surface, ..) in &candidates {
        let pat = format!(r"(?i)\b{}\b", regex::escape(&surface.to_lowercase()));
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
    // A pointable anti-pattern is at most a screenful ([`MAX_EXAMPLE_BYTES`]); the pair search
    // below compiles a regex per candidate window, which on a scraped manual page is hours.
    if bad.len() > MAX_EXAMPLE_BYTES || good.len() > MAX_EXAMPLE_BYTES {
        return None;
    }
    // Strip doc-page comments before tokenising — they pollute the discriminator.
    let bad = strip_code_comments(bad);
    let good = strip_code_comments(good);
    let (bad, good) = (bad.as_str(), good.as_str());

    // Broad tokeniser: handles identifiers (must start with letter/underscore so bare numeric
    // literals are ignored — pure-value differences like 0 vs 1 are semantic, not syntactic),
    // Ruby ?/! methods, shell flags (--verbose, -v), sigiled vars ($var, @var), and operators.
    let tok_re = regex::Regex::new(
        r"--?[A-Za-z][\w-]*|[$@]\w+|[A-Za-z_]\w*[!?]?|==|!=|<=|>=|->|=>|\.\.|::|[\[\]{}]",
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
    // Two passes: first demand BOTH tokens be absent from the fix (pure bad ∧ ¬good — these
    // generalize, e.g. `[.*?]` from a no-arrays rule), then relax to "not both present" so a
    // pair anchored on one shared name can still discriminate when nothing purer exists.
    for strict in [true, false] {
        for win in bad_toks.windows(2) {
            if !is_useful(win[0]) && !is_useful(win[1]) {
                continue; // both are single-char — no discriminating power
            }
            let in_good = (good_set.contains(win[0]), good_set.contains(win[1]));
            if if strict { in_good.0 || in_good.1 } else { in_good.0 && in_good.1 } {
                continue;
            }
            let pat = format!("{}.*?{}", wpat(win[0]), wpat(win[1]));
            if let Ok(re) = regex::Regex::new(&pat) {
                if re.is_match(bad) && !re.is_match(good) {
                    return Some(pat);
                }
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
    /// Stored as a string (regex::Regex is not Serialize); compiled ONCE on first use and
    /// cached — recompiling per file made a whole-project pass pay ~rules×files compiles.
    Text {
        pattern: String,
        #[serde(skip)]
        compiled: std::sync::OnceLock<Option<regex::Regex>>,
    },
}

impl MatchKind {
    /// Lines in `code` where this rule fires. 1-based.
    fn matches(&self, code: &str) -> Vec<usize> {
        match self {
            MatchKind::Ast(pat) => pat.matches(code),
            MatchKind::Text { pattern, compiled } => {
                let Some(re) = compiled.get_or_init(|| regex::Regex::new(pattern).ok()) else {
                    return vec![];
                };
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
    /// The rule's English advice — carried WITH the compiled detector so rendering a finding
    /// never re-reads the multi-megabyte learned catalogs it came from.
    #[serde(default)]
    description: String,
    /// Where the rule came from (doc URL or rule-file path) — the finding's citation.
    #[serde(default)]
    source: String,
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
    /// True when the match is a lossless AST pattern with an exact-text anchor (reported directly);
    /// false for token-regex fallbacks and container-only AST patterns, which the live path
    /// confirms through the Hv concept gate first.
    pub precise: bool,
}

impl RuleSet {
    /// Compile a language's documented `(id, severity, bad, good, description)` rules.
    ///
    /// For languages with a tree-sitter grammar: lossless AST patterns via `bad ∧ ¬good`.
    /// For any other language: discriminating token-regex patterns, derived the same way.
    /// Both paths apply the same quality gate: self-fire (must flag its own `bad`) and
    /// over-fire (must not flag any `good` in the corpus). Only rules that pass both survive.
    /// `ground` is the learned evidence prose-only rules are read through ([`Grounding`]);
    /// pass `Grounding::default()` when no docs have been read for the language yet.
    pub fn build(lang: &str, rules: &[(String, String, String, String, String, String)], ground: &Grounding) -> RuleSet {
        let trusted = &ground.trusted;
        let reference_corpus = &ground.reference;
        let ground = GroundView::of(ground);
        let mut compiled = Vec::new();
        let mut seen = HashSet::new();
        let has_grammar = language(lang).is_some();
        for (id, severity, bad, good, desc, source) in rules {
            if id.is_empty() || !seen.insert(id.clone()) {
                continue;
            }
            // bad may be empty when the documentation only provides prose (description-only
            // rules). description_discriminator will read the English doc to derive a pattern;
            // the SELF-FIRE gate below will then validate or drop it.
            if desc.trim().is_empty() && bad.trim().is_empty() {
                continue; // nothing to learn from
            }
            // Read the description's polarity ALONG the text — each word's context is the
            // nearest decisive lean, no chopping, no punctuation ([`GroundView::word_contexts`]).
            let contexts = ground.word_contexts(desc);
            // The entry ticket for every LEARNED rule, example-backed or not: some word of its
            // description must sit in a forbidding context. A teaching section's fenced
            // illustrations read as "bad examples" only by document-order fallback — enforcing
            // them turns reading material into law. Project rules are law by location and skip
            // the reading; with no ready classifier the question is unanswerable and the
            // author's material is trusted as before.
            let classifier_ready = ground.polarity.is_some_and(|p| p.is_ready());
            if !trusted.contains(id) && classifier_ready && !contexts.iter().any(|c| *c == Some(true)) {
                continue;
            }
            // A description-derived detector exists only for prose that STATES a violation:
            // project law states one by LOCATION (the user wrote it in a rule file — that is
            // the label), learned prose by the classifier's reading. Within the description,
            // prohibition-context words outrank all others and remedy-context words are never
            // eligible ("…; use the logging module instead" must never compile `logging`) —
            // that is what keeps English understanding from being confused for a lintable
            // code language. Learned rules additionally require the construct to exist in
            // real documented code (`only_grounded`); the project's own law does not.
            let desc_detector = |view: &GroundView| -> Option<String> {
                description_discriminator(desc, bad, view, &contexts, !trusted.contains(id))
            };
            let kind = if has_grammar {
                if let Some(pat) = RulePattern::compile(lang, bad, good, desc) {
                    // AST pattern — lossless and most precise; no regex needed.
                    MatchKind::Ast(pat)
                } else if let Some(re) = desc_detector(&ground) {
                    // English prose is the primary documentation; read it first.
                    // The description names the construct to flag: "avoid `e.printStackTrace()`".
                    MatchKind::Text { pattern: re, compiled: std::sync::OnceLock::new() }
                } else if let Some(re) = text_discriminator(bad, good) {
                    // Code-diff fallback: description had no extractable term but the bad/good
                    // examples (themselves part of the official documentation) still distinguish.
                    MatchKind::Text { pattern: re, compiled: std::sync::OnceLock::new() }
                } else {
                    continue;
                }
            } else {
                // No grammar — text matching only. Documentation prose is the primary signal;
                // code examples (which appear in the same docs) refine when prose is thin.
                if let Some(re) = desc_detector(&ground) {
                    MatchKind::Text { pattern: re, compiled: std::sync::OnceLock::new() }
                } else if let Some(re) = text_discriminator(bad, good) {
                    MatchKind::Text { pattern: re, compiled: std::sync::OnceLock::new() }
                } else {
                    continue;
                }
            };
            compiled.push(CompiledRule {
                id: id.clone(),
                severity: severity.clone(),
                description: desc.clone(),
                source: source.clone(),
                kind,
            });
        }
        // SELF-FIRE: when a bad example is known, the compiled rule must flag it.
        // Description-only rules (bad is empty) skip this gate — they are validated at
        // query time: if the extracted pattern fires on real violations found in the project,
        // it was correct; if nothing matches, it stays silent (never a false flag).
        // Both gates run BEFORE pattern dedup so an invalid rule can never claim a pattern
        // signature and knock out the valid rule that shares it (`seen` above keeps the maps
        // first-wins for duplicate ids, matching which rule actually compiled).
        let mut bad_map: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        let mut good_map: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        for (id, _, bad, good, _, _) in rules {
            bad_map.entry(id.as_str()).or_insert(bad.as_str());
            good_map.entry(id.as_str()).or_insert(good.trim());
        }
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
        // REFERENCE-FIRE: a violation detector must stay quiet on the language's own
        // documented-NORMAL code. A rule whose real meaning is semantic (borrow usage, operand
        // nullness) tree-diffs down to a ubiquitous construct — "any `&mut` parameter", the bare
        // `null` literal — and would flag idiomatic code everywhere; running every compiled
        // detector over the reference corpus once at compile time drops exactly those. The bar
        // is two-tier by how much the detector's own shape vouches for it: a structured pattern
        // (depth ≥ 2 with an exact anchor) gets quarantine's 1% bar; a degenerate one (leaf
        // pattern, all-wildcard shape, or any single-token text regex) has only the corpus as
        // witness and gets 0.1% — a genuinely banned construct (`goto`) is near-absent from
        // normal examples and passes, a pervasive one (`null`) cannot mark violations and dies.
        // Statistical, so it needs scale ([`REFERENCE_FIRE_MIN_LINES`]); project law is exempt
        // by location. Runs before dedup for the same reason the other gates do: an
        // over-general rule must not claim a pattern signature it cannot keep.
        let ref_lines: usize = reference_corpus.iter().map(|e| e.lines().count()).sum();
        if ref_lines >= REFERENCE_FIRE_MIN_LINES {
            let probe = RuleSet { lang: lang.to_string(), rules: compiled };
            let mut fired: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for example in reference_corpus {
                for f in probe.flag(example) {
                    *fired.entry(f.rule).or_default() += 1;
                }
            }
            compiled = probe.rules;
            compiled.retain(|r| {
                let bar = match &r.kind {
                    MatchKind::Ast(p) if pat_depth(&p.pat) >= 2 && pat_has_anchor(&p.pat) => ref_lines / 100,
                    _ => ref_lines / 1000,
                };
                trusted.contains(&r.id) || fired.get(&r.id).copied().unwrap_or(0) <= bar
            });
        }
        // Dedup identical compiled patterns: noisy docs pages often yield several rule entries
        // that compile to the same pattern (the same wiki page scraped under multiple slugs).
        // One pattern = one rule; keep the first id — the caller orders rules by trust
        // (project > corpus folder > crawled docs), so the most trusted rule wins its pattern.
        let mut seen_patterns = HashSet::new();
        compiled.retain(|r| {
            seen_patterns.insert(serde_json::to_string(&r.kind).unwrap_or_default())
        });
        RuleSet { lang: lang.to_string(), rules: compiled }
    }

    /// Number of compiled rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// The ids of the rules that actually compiled a detector — the honest answer to "which of
    /// the laws I wrote can you enforce?". A caller compares this against what it asked for and
    /// REPORTS the difference; law must never vanish silently.
    pub fn rule_ids(&self) -> impl Iterator<Item = &str> {
        self.rules.iter().map(|r| r.id.as_str())
    }

    /// A compiled rule's reporting facts: `(severity, description, source)`. The model is the
    /// single source of truth for what it enforces — no catalog re-read at render time.
    pub fn info_of(&self, id: &str) -> Option<(&str, &str, &str)> {
        self.rules
            .iter()
            .find(|r| r.id == id)
            .map(|r| (r.severity.as_str(), r.description.as_str(), r.source.as_str()))
    }

    /// What a rule's detector actually watches for — the honest answer to "what did you
    /// understand my law as?". A text rule shows its literal pattern; an AST rule is a
    /// structural match compiled from the rule's own examples. `None` when the rule did not
    /// compile. Surfacing this lets the author correct a mis-read law by rephrasing it,
    /// instead of discovering the misunderstanding through missing findings.
    pub fn detector_of(&self, id: &str) -> Option<String> {
        self.rules.iter().find(|r| r.id == id).map(|r| match &r.kind {
            MatchKind::Ast(_) => "structural pattern from your example".to_string(),
            MatchKind::Text { pattern, .. } => format!("`{}`", pattern.replace(r"\b", "")),
        })
    }

    /// Flag `code`: every line where a rule fires (AST match or text match), deduped per rule.
    /// Each finding carries `precise` so the caller can confirm the imprecise ones. Imprecise:
    /// token-regex fallbacks, and AST patterns whose only identity is a container kind — several
    /// distinct rules can compile to the same bare container, so the concept gate must arbitrate.
    pub fn flag(&self, code: &str) -> Vec<Finding> {
        // Parse ONCE and run every AST rule over the same tree — a rule set is many rules but
        // one grammar, and re-parsing per rule multiplied the whole-project pass by rule count.
        let tree = language(&self.lang).and_then(|language| {
            let mut parser = Parser::new();
            parser.set_language(&language).ok()?;
            parser.parse(code, None)
        });
        let mut out = Vec::new();
        for r in &self.rules {
            let (precise, mut lines) = match &r.kind {
                MatchKind::Ast(p) => (
                    p.text_anchored(),
                    tree.as_ref()
                        .map(|t| p.matches_in(t.root_node(), code.as_bytes()))
                        .unwrap_or_default(),
                ),
                MatchKind::Text { .. } => (false, r.kind.matches(code)),
            };
            lines.sort_unstable();
            lines.dedup();
            for line in lines {
                out.push(Finding { rule: r.id.clone(), severity: r.severity.clone(), line, precise });
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A `(id, severity, bad, good, desc)` tuple in the shape `RuleSet::build` expects.
    fn rule(id: &str, bad: &str, good: &str, desc: &str) -> (String, String, String, String, String, String) {
        (id.into(), "high".into(), bad.into(), good.into(), desc.into(), "test://rule".into())
    }

    fn lines_for<'a>(fs: &'a [Finding], id: &str) -> Vec<usize> {
        fs.iter().filter(|f| f.rule == id).map(|f| f.line).collect()
    }

    /// An empty grounding: no docs read, no reference code — only the author's own signals count.
    fn unground() -> GroundView<'static> {
        GroundView {
            code_tokens: std::collections::HashSet::new(),
            project_tokens: std::collections::HashSet::new(),
            reader: None,
            polarity: None,
        }
    }

    /// A reader that has READ ordinary instruction English — the salience baseline the
    /// discriminator selects against. No shapes, no markup: just reading.
    fn read_reader() -> crate::lint_read::Reader {
        let mut r = crate::lint_read::Reader::new();
        for _ in 0..50 {
            r.learn_span(
                "do not use this anywhere in the project and never call it; \
                 read the value from the port configuration instead and parse the input explicitly; \
                 do not ship the calls to the structured logger for the committed code; \
                 file an issue for the comments left behind and hardcode nothing",
            );
        }
        r
    }

    /// Wrap a reader into the polarity carrier the grounded path hands the discriminator.
    fn ground_with_reader() -> crate::lint_read::Polarity {
        let mut b = crate::lint_read::PolarityBuilder::new(read_reader());
        b.accumulate("never do this", true);
        b.accumulate("this is the recommended form", false);
        b.build()
    }

    #[test]
    fn the_unread_word_is_the_construct_whatever_its_shape() {
        // No digit rule, no backtick rule: "8080" is simply the one word the reader has never
        // read. The same selection finds an identifier, a number, or notation not yet invented.
        let polarity = ground_with_reader();
        let ground = Grounding { reference: Vec::new(), polarity: Some(polarity), ..Default::default() };
        let view = GroundView::of(&ground);
        let re = description_discriminator("Do not hardcode port 8080 anywhere; read the port from configuration.", "", &view, &view.word_contexts("Do not hardcode port 8080 anywhere; read the port from configuration."), false);
        assert_eq!(re.as_deref(), Some(r"(?i)\b8080\b"));
    }

    #[test]
    fn a_dotted_construct_stays_one_word_as_the_author_wrote_it() {
        // "console.log" is one whitespace-delimited word; no dotted-path regex needed.
        let polarity = ground_with_reader();
        let ground = Grounding { reference: Vec::new(), polarity: Some(polarity), ..Default::default() };
        let view = GroundView::of(&ground);
        let re = description_discriminator("Do not ship console.log calls; use the structured logger.", "", &view, &view.word_contexts("Do not ship console.log calls; use the structured logger."), false);
        assert_eq!(re.as_deref(), Some(r"(?i)\bconsole\.log\b"));
    }

    #[test]
    fn no_reading_and_no_example_means_abstain_not_guess() {
        // With no reader and no bad example there is no evidence to select by — the engine
        // abstains rather than guessing a word.
        let re = description_discriminator("Do not hardcode port 8080 anywhere.", "", &unground(), &unground().word_contexts("Do not hardcode port 8080 anywhere."), false);
        assert_eq!(re, None);
    }

    #[test]
    fn rule_ids_expose_what_actually_compiled() {
        let rules = [
            rule("no_eval", "eval(x)", "parse(x)", "Never call eval."),
            rule("hopeless", "", "", ""),
        ];
        let set = RuleSet::build("python", &rules, &Grounding::default());
        let ids: Vec<&str> = set.rule_ids().collect();
        assert!(ids.contains(&"no_eval"));
        assert!(!ids.contains(&"hopeless"), "an uncompiled rule is not among the ids");
    }

    #[test]
    fn an_unread_english_word_is_selected_without_any_markup() {
        // "panic" is an ordinary English word the test reader has never read — that alone names
        // it, backticks or not. The backticks are edge punctuation and vanish in tokenization.
        let polarity = ground_with_reader();
        let ground = Grounding { reference: Vec::new(), polarity: Some(polarity), ..Default::default() };
        let view = GroundView::of(&ground);
        let re = description_discriminator("Never call `panic` in library code; return an error value instead.", "", &view, &view.word_contexts("Never call `panic` in library code; return an error value instead."), false);
        assert_eq!(re.as_deref(), Some(r"(?i)\bpanic\b"));
        let bare = description_discriminator("Never call panic in library code; return an error value instead.", "", &view, &view.word_contexts("Never call panic in library code; return an error value instead."), false);
        assert_eq!(bare.as_deref(), Some(r"(?i)\bpanic\b"), "markup is optional, not a gate");
    }

    #[test]
    fn grounded_reading_qualifies_a_plain_word_the_docs_taught() {
        // "Never call panic in library code" — nothing is marked. The learned evidence decides:
        // the reader read the docs (so connective words are common) and the reference corpus
        // contains real code where `panic` occurs. That combination — learned, not authored —
        // names the construct.
        let mut reader = crate::lint_read::Reader::new();
        for _ in 0..40 {
            reader.learn_span("never call this in library code; return an error value instead of that");
        }
        let polarity = {
            let mut b = crate::lint_read::PolarityBuilder::new(reader);
            b.accumulate("never do this", true);
            b.accumulate("this is the recommended form", false);
            b.build()
        };
        let ground = Grounding {
            reference: vec!["func f() { panic(\"x\") }".into(), "return fmt.Errorf(\"y\")".into()],
            polarity: Some(polarity),
            ..Default::default()
        };
        let view = GroundView::of(&ground);
        let re = description_discriminator("Never call panic in library code; return an error value instead.", "", &view, &view.word_contexts("Never call panic in library code; return an error value instead."), false);
        assert_eq!(re.as_deref(), Some(r"(?i)\bpanic\b"));
    }

    /// A trained polarity classifier in the shape the live path carries one — built from labeled
    /// prose exactly like the grounded path builds it.
    fn polarity() -> crate::lint_read::Polarity {
        crate::lint_read::Polarity::from_labeled(&[
            ("never call this it is dangerous forbidden and unsafe", true),
            ("do not use this deprecated broken pattern anywhere", true),
            ("avoid this fragile obsolete style it leaks badly", true),
            ("this brittle call is discouraged and must not ship", true),
            ("prefer the recommended safe supported form instead", false),
            ("always use the idiomatic clean correct approach here", false),
            ("this canonical fix is right and well tested", false),
            ("use this when random access is needed it scales well", false),
        ])
    }

    #[test]
    fn neutral_guidance_prose_compiles_no_detector_but_prohibition_does() {
        // English understanding separates READING material from LAW: a concept/guidance span
        // ("use X when …") teaches the model but states no violation, so it must not become a
        // firing pattern; a prohibition names a violation and must.
        let ground = Grounding { reference: Vec::new(), polarity: Some(polarity()), ..Default::default() };
        let guidance = [rule(
            "use_arraylist",
            "",
            "",
            "Use `ArrayList` when random access is needed; it scales well and is the supported approach.",
        )];
        let set = RuleSet::build("python", &guidance, &ground);
        assert_eq!(set.rule_count(), 0, "guidance prose is comprehension, not a detector");

        let prohibition = [rule(
            "no_eval",
            "",
            "",
            "Never call `eval` anywhere; it is dangerous and forbidden.",
        )];
        // A learned rule's construct must also exist in real code — the docs that taught the
        // rule showed it, so its reference corpus carries it. Without that grounding a learned
        // prohibition abstains (only the project's own law may name the unseen).
        let ungrounded = RuleSet::build("python", &prohibition, &ground);
        assert_eq!(ungrounded.rule_count(), 0, "no code evidence → no learned detector");
        let grounded = Grounding {
            reference: vec!["value = eval(source)".into()],
            polarity: Some(polarity()),
            ..Default::default()
        };
        let set = RuleSet::build("python", &prohibition, &grounded);
        assert_eq!(set.rule_count(), 1, "a grounded prohibition IS a statement of a violation");
        assert_eq!(lines_for(&set.flag("x = eval(s)"), "no_eval"), vec![1]);
    }

    #[test]
    fn project_law_compiles_even_when_its_register_reads_ambiguous() {
        // "Never call eval anywhere" is law by LOCATION (the user's rule file), yet its words
        // ("never", "call") are ordinary reference-doc vocabulary — Rust and TypeScript both have
        // a `never` type — so the classifier abstains. The rule file is the label: trusted ids
        // bypass the register reading entirely.
        let mut ground = Grounding { reference: Vec::new(), polarity: Some(polarity()), ..Default::default() };
        let rules = [rule("no_eval", "", "", "Never call `eval` anywhere in this project; parse the input explicitly.")];
        let gated = RuleSet::build("python", &rules, &ground);
        ground.trusted.insert("no_eval".to_string());
        let trusted = RuleSet::build("python", &rules, &ground);
        assert_eq!(trusted.rule_count(), 1, "project law always compiles");
        assert_eq!(lines_for(&trusted.flag("x = eval(s)"), "no_eval"), vec![1]);
        // Whatever the classifier said for the untrusted twin, the trusted one must not depend on it.
        assert!(gated.rule_count() <= trusted.rule_count());
    }

    #[test]
    fn example_backed_rules_also_need_a_forbidding_sentence_unless_trusted() {
        // A teaching section's fenced illustration reads as a "bad example" only by document
        // order — without a forbidding sentence the whole rule is reading material, not law.
        let mut ground = Grounding { reference: Vec::new(), polarity: Some(polarity()), ..Default::default() };
        let rules = [rule(
            "q_rule",
            "xs = [1, 2, 3]",
            "xs = (1, 2, 3)",
            "xyzzy qwerty plugh zork.",
        )];
        let set = RuleSet::build("qlang", &rules, &ground); // grammarless → example diff path
        assert_eq!(set.rule_count(), 0, "neutral prose + examples = comprehension, not a detector");
        // The project's own law is exempt by location …
        ground.trusted.insert("q_rule".to_string());
        assert_eq!(RuleSet::build("qlang", &rules, &ground).rule_count(), 1);
        // … and with no classifier at all the author's examples are trusted as before.
        let unground = Grounding::default();
        assert_eq!(RuleSet::build("qlang", &rules, &unground).rule_count(), 1);
    }

    #[test]
    fn value_dependent_rule_does_not_compile_an_overfiring_ast_pattern() {
        // F521-class docs: two same-shape bad instances differing only in the string VALUE
        // (an invalid vs odd format string), no good example. Structure cannot represent value
        // semantics — the AST path must abstain so a valid `.format()` call is never flagged
        // as a precise match.
        let rules = [rule(
            "F521",
            "\"{\" . format ( foo )\n\" {} \" . format ( foo )",
            "",
            ".format call has invalid format string: {message}",
        )];
        let set = RuleSet::build("python", &rules, &Grounding::default());
        let hits = set.flag("greeting = \"hello {}\".format(name)");
        assert!(
            hits.iter().all(|f| !(f.rule == "F521" && f.precise)),
            "a value-dependent rule must never produce a precise match on a valid call: {:?}",
            hits.iter().map(|f| (&f.rule, f.line, f.precise)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn container_rule_generalizes_past_element_types() {
        // The rule's bad example uses an integer list, but the rule is about the CONTAINER —
        // the verbatim bad line, a string list (any element type, any length) must fire too.
        let rules = [rule(
            "q_containers",
            "scores = [90, 85, 77]",
            "scores = {\"first\": 90, \"second\": 85}",
            "Containers of this kind are banned in this project. Use explicit keyed structures (dict, dataclass) so every element has a name.",
        )];
        let set = RuleSet::build("python", &rules, &Grounding::default());
        let hits = set.flag("scores = [90, 85, 77]\nlabels = [\"a\", \"b\", \"c\"]\nmixed = [1, \"two\"]\nempty = []");
        assert_eq!(lines_for(&hits, "q_containers"), vec![1, 2, 3, 4], "container rule must fire on the verbatim line and every element type");
    }

    #[test]
    fn container_good_example_does_not_neutralize_the_rule() {
        // A good example that is ITSELF a container (tuple) must not neutralize the rule:
        // the bad container kind (list) is still novel relative to the good tree.
        let rules = [rule(
            "q_containers",
            "xs = [1, 2, 3]",
            "xs = (1, 2, 3)",
            "Do not use list literals anywhere in this project. Use tuples instead.",
        )];
        let set = RuleSet::build("python", &rules, &Grounding::default());
        let hits = set.flag("scores = [90, 85, 77]\nlabels = [\"math\", \"sci\", \"art\"]\nok = (1, 2)");
        assert_eq!(lines_for(&hits, "q_containers"), vec![1, 2], "list rule with tuple good example must fire on lists and spare tuples");
    }

    #[test]
    fn js_container_rule_fires_on_var_items() {
        let rules = [rule(
            "q_containers_js",
            "var items = [1, 2, 3]",
            "var items = {a: 1, b: 2, c: 3}",
            "These containers are banned; use keyed objects so every element has a name.",
        )];
        let set = RuleSet::build("javascript", &rules, &Grounding::default());
        let hits = set.flag("var items = [1, 2, 3]");
        assert_eq!(lines_for(&hits, "q_containers_js"), vec![1], "JS container rule must fire on `var items = [1, 2, 3]`");
    }

    #[test]
    fn format_call_does_not_fire_extra_named_argument_class_rules() {
        // F522/F525/PLE0605-class rules are about specific misuse; a plain positional
        // `"hello {}".format(name)` must NOT trip them. A UP032-style f-string upgrade MAY fire.
        let rules = [
            rule(
                "F522",
                "\"{foo}\".format(bar=1)",
                "\"{foo}\".format(foo=1)",
                "format called with extra named arguments that are never used",
            ),
            rule(
                "PLE0605",
                "__all__ = \"foo\"",
                "__all__ = [\"foo\"]",
                "invalid format for __all__, must be a tuple or list",
            ),
            rule(
                "UP032",
                "\"{}\".format(x)",
                "f\"{x}\"",
                "use an f-string instead of str.format",
            ),
        ];
        let set = RuleSet::build("python", &rules, &Grounding::default());
        let hits = set.flag("greeting = \"hello {}\".format(name)");
        assert!(lines_for(&hits, "F522").is_empty(), "F522 must not fire on a plain positional .format()");
        assert!(lines_for(&hits, "PLE0605").is_empty(), "PLE0605 must not fire on a .format() call");
        // The legitimate f-string upgrade is allowed to fire — its shape does occur here.
        assert!(!lines_for(&hits, "UP032").is_empty(), "UP032-style rule may still fire");
    }

    #[test]
    fn pprint_rule_attributes_only_lines_containing_pprint() {
        let rules = [rule(
            "no_pprint",
            "pprint(data)",
            "",
            "avoid pprint in production code; use logging",
        )];
        let set = RuleSet::build("python", &rules, &Grounding::default());
        let src = "import pprint\n\ndef f(x):\n    return x\n\npprint(data)\n";
        let hits = set.flag(src);
        assert_eq!(lines_for(&hits, "no_pprint"), vec![6], "pprint must be attributed only to the call line (6)");
    }

    #[test]
    fn pathological_deep_example_never_breaks_the_cache_round_trip() {
        // A docs page whose "bad example" is a monster (deeply nested expression) must not compile
        // into a pattern that serializes but cannot be deserialized (serde_json's 128-level
        // recursion limit) — the exact failure that silently dropped a language's model on warm
        // runs. Either the pattern is abstained or the round-trip must succeed.
        let deep = format!("x = {}1{}", "foo(".repeat(200), ")".repeat(200));
        let rules = [
            rule("monster", &deep, "", "avoid this deeply nested thing"),
            rule("healthy_rule", "xs = [1, 2, 3]", "xs = (1, 2, 3)", "Do not use list literals."),
        ];
        let set = RuleSet::build("python", &rules, &Grounding::default());
        let loaded = RuleSet::from_json(&set.to_json()).expect("cached model must always load back");
        assert_eq!(loaded.rule_count(), set.rule_count(), "round trip loses nothing");
        assert!(!loaded.flag("scores = [90, 85]").is_empty(), "the healthy rule still fires after the round trip");
    }

    #[test]
    fn clean_idiomatic_file_produces_no_findings() {
        let rules = [rule(
            "q_containers",
            "scores = [90, 85, 77]",
            "scores = {\"first\": 90}",
            "These containers are banned in this project. Use explicit keyed structures.",
        )];
        let set = RuleSet::build("python", &rules, &Grounding::default());
        let src = "def greet(name):\n    return f\"hello {name}\"\n\nconfig = {\"first\": 90, \"second\": 85}\n";
        assert!(set.flag(src).is_empty(), "clean idiomatic code must yield zero findings");
    }
}
