//! Grammar resolution for [`super`] — resolving a language name to a tree-sitter grammar, plus
//! the structural n-gram fingerprint built on top of a parse. Pure mechanism: no rule content.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

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
//   4. Text fallback — if all of the above fail, token-sequence matching covers any language.

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
    if !crate::lint_train::network_allowed() {
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

/// The grammatical ROLE the language's OWN tree-sitter grammar gives `token`, read from the
/// grammar itself (never an enumerated keyword list): `"keyword"` for an anonymous literal token
/// (`use`, `as`, `fn`), `"primitive_type"` for a built-in type the grammar lexes as such
/// (`usize`, `u32`), or `None` for a nameable identifier a rule could legitimately point at
/// (`goto`, `panic`, a user construct). Used by the mint gate to reject a single-token detector
/// that is really part of the language's syntax and fires on nearly every file.
/// Whether ANY bundled grammar lexes `token` as a keyword or primitive type — the grammar-driven
/// test for "this word is a language CONSTRUCT, not prose" (`var`, `unsafe`, `goto`), read from the
/// grammars themselves, never an enumerated keyword list. Language-agnostic: a construct-naming
/// prohibition ("never use the var keyword") does not state which language, so any bundled grammar
/// recognising the token as syntax is evidence it names a construct.
pub(crate) fn is_construct_keyword(token: &str) -> bool {
    BUNDLED.keys().any(|l| token_role(l, token).is_some())
}

pub(crate) fn token_role(lang: &str, token: &str) -> Option<&'static str> {
    let ts = language(lang)?;
    // A keyword/operator is an ANONYMOUS (literal) token in the grammar — the grammar knows its
    // exact spelling. `id_for_node_kind(named=false)` resolves it iff it exists as such.
    let kw_id = ts.id_for_node_kind(token, false);
    if kw_id != 0 && (kw_id as usize) < ts.node_kind_count() {
        return Some("keyword");
    }
    // A built-in/primitive type is lexed by the grammar (not a literal token), so parse the token
    // in a TYPE position and see whether the node covering it is a primitive type rather than a
    // user type name. Several neutral templates cover the common grammar families.
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&ts).is_err() {
        return None;
    }
    for template in ["type __probe = TOK;", "let __probe: TOK = x;", "TOK __probe;"] {
        let src = template.replace("TOK", token);
        let Some(tree) = parser.parse(&src, None) else { continue };
        if let Some(node) = deepest_named_covering(tree.root_node(), &src, token) {
            let kind = node.kind();
            if kind.contains("primitive") || kind == "primitive_type" {
                return Some("primitive_type");
            }
        }
    }
    None
}

/// The deepest NAMED node whose text is exactly `token`, or `None` — the grammar's classification
/// of the token at that position.
fn deepest_named_covering<'a>(
    root: tree_sitter::Node<'a>,
    src: &str,
    token: &str,
) -> Option<tree_sitter::Node<'a>> {
    let mut found: Option<tree_sitter::Node> = None;
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if n.is_named() && n.utf8_text(src.as_bytes()).map(|t| t == token).unwrap_or(false) {
            // Prefer the DEEPEST such node (a primitive_type leaf over a wrapping type node).
            if found.map(|f| n.start_byte() >= f.start_byte()).unwrap_or(true) {
                found = Some(n);
            }
        }
        let mut cursor = n.walk();
        for child in n.children(&mut cursor) {
            stack.push(child);
        }
    }
    found
}

