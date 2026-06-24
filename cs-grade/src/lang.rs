//! Language detection and per-language signal profiles.
//!
//! The rubric grades language-agnostic CS principles — design/structure, data
//! structures, tests, documentation, style, build, abstraction — but the *raw
//! signals* (what a "public declaration", "doc comment", "interface", or "test
//! framework" looks like) are syntactic and therefore language-specific. A
//! [`LangProfile`] captures those syntactic patterns plus a [`Vocab`] of the
//! wording the report uses, so the same scorer works across languages.
//!
//! [`detect`] picks the profile whose source extensions cover the most files in
//! the project (Java/Rust/Python/TypeScript-JavaScript/Go/C-C++), falling back to
//! a permissive [`generic`] profile when no known language dominates. The Java
//! profile reproduces the historical patterns and wording byte-for-byte, so a
//! Java project grades exactly as it did before multi-language support.

use crate::project::FileEntry;

/// User-facing wording the report substitutes per language. Every field is an
/// exact substring/sentence so a Java project renders identically to the
/// original report; other languages get language-appropriate phrasing.
#[derive(Clone, Copy)]
pub struct Vocab {
    /// Name of the doc-comment system, used as "`{doc_name}` coverage".
    pub doc_name: &'static str,
    /// Name of the test framework/runner shown in the tests evidence line.
    pub test_framework_name: &'static str,
    /// Plural label for the abstraction primitive, e.g. "interface(s)".
    pub interfaces_label: &'static str,
    /// Plural label for the concrete type primitive, e.g. "class(es)".
    pub types_label: &'static str,
    /// Plural label for abstract types, e.g. "abstract class(es)".
    pub abstract_label: &'static str,
    /// Plural label for module/namespace declarations, e.g. "package declaration(s)".
    pub module_label: &'static str,
    /// "Path to A+" fix for missing tests.
    pub add_tests_fix: &'static str,
    /// "Path to A+" fix for missing doc comments.
    pub add_docs_fix: &'static str,
    /// "Path to A+" fix for a missing build file.
    pub add_build_fix: &'static str,
    /// "Path to A+" fix for missing module organization.
    pub add_module_fix: &'static str,
    /// "Path to A+" fix for low abstraction.
    pub add_abstraction_fix: &'static str,
    /// "Path to A+" fix urging programming to interfaces/traits/protocols.
    pub program_to_interfaces_fix: &'static str,
    /// "Path to A+" fix for using appropriate standard data structures.
    pub good_structures_fix: &'static str,
}

/// The syntactic patterns and wording for one language. Regex fields are passed
/// verbatim to the `regex` crate (no lookaround/backreferences), matching the
/// constraints the original Java patterns already obeyed.
pub struct LangProfile {
    /// Human-readable language name (shown in the report header).
    pub name: &'static str,
    /// File extensions (with leading dot) that count as this language's sources.
    pub source_exts: &'static [&'static str],

    // ── test-file partition ──────────────────────────────────────────────────
    /// Case-insensitive match on the root-relative path marking a test directory.
    pub test_dir: &'static str,
    /// Match on the absolute path marking a test file by suffix (or empty).
    pub test_suffix: &'static str,
    /// Match on the file basename marking a test file by name (or empty).
    pub test_basename: &'static str,

    // ── signal patterns (run over the source corpus unless noted) ────────────
    /// A public/exported declaration (the denominator for doc coverage).
    pub public_decl: &'static str,
    /// A documentation-comment block (the numerator for doc coverage).
    pub doc_block: &'static str,
    /// An interface/trait/protocol declaration.
    pub interface_decl: &'static str,
    /// A concrete type (class/struct/…) declaration.
    pub type_decl: &'static str,
    /// An abstract type / polymorphism marker.
    pub abstract_decl: &'static str,
    /// A module/package/namespace declaration.
    pub module_decl: &'static str,
    /// Build-manifest filenames (matched against root-relative paths).
    pub build_files: &'static str,
    /// Standard source-layout marker (matched against root-relative paths).
    pub src_layout: &'static str,
    /// Test-framework usage (run over tests + source corpora).
    pub test_framework: &'static str,
    /// A test assertion call (run over the test corpus).
    pub assertion: &'static str,
    /// Idiomatic standard data structures for the language.
    pub good_structures: &'static str,
    /// Debug/console print or stack-dump calls (a style smell).
    pub debug_print: &'static str,
    /// Commented-out code (a leading line comment followed by code keywords).
    pub commented_code: &'static str,
    /// Very long brace-delimited bodies, or `None` for brace-free languages.
    pub long_method: Option<&'static str>,

    /// Report wording for this language.
    pub vocab: Vocab,
}

impl LangProfile {
    /// Whether `rel`'s extension belongs to this language.
    pub fn owns(&self, rel: &str) -> bool {
        self.source_exts.iter().any(|ext| rel.ends_with(ext))
    }
}

/// Pick the profile whose source extensions cover the most discovered files.
/// Ties resolve by the fixed priority order below; a project with no recognized
/// source files falls back to [`generic`].
pub fn detect(files: &[FileEntry]) -> LangProfile {
    // Priority order also breaks ties (earlier wins on equal counts).
    let candidates = [
        java(),
        rust(),
        python(),
        typescript(),
        go(),
        cpp(),
    ];
    let mut best: Option<(usize, usize)> = None; // (index, count)
    for (i, p) in candidates.iter().enumerate() {
        let count = files.iter().filter(|f| p.owns(&f.rel)).count();
        if count == 0 {
            continue;
        }
        match best {
            Some((_, bc)) if count <= bc => {}
            _ => best = Some((i, count)),
        }
    }
    match best {
        Some((i, _)) => match i {
            0 => java(),
            1 => rust(),
            2 => python(),
            3 => typescript(),
            4 => go(),
            _ => cpp(),
        },
        None => generic(),
    }
}

/// Java — reproduces the original grader's patterns and wording exactly.
pub fn java() -> LangProfile {
    LangProfile {
        name: "Java",
        source_exts: &[".java"],
        test_dir: r"(?i)(^|/)(test|tests)/",
        test_suffix: r"Test[s]?\.java$",
        test_basename: r"Tests?\b",
        public_decl: r"\bpublic\s+(?:static\s+)?(?:final\s+)?(?:abstract\s+)?(?:class|interface|enum|[\w<>\[\]]+\s+\w+\s*\()",
        doc_block: r"/\*\*[\s\S]*?\*/",
        interface_decl: r"\binterface\s+\w+",
        type_decl: r"\bclass\s+\w+",
        abstract_decl: r"\babstract\s+class\s+\w+",
        module_decl: r"(?m)^\s*package\s+[\w.]+;",
        build_files: r"(^|/)(pom\.xml|build\.gradle(\.kts)?|build\.xml|Makefile)$",
        src_layout: r"(^|/)src/",
        test_framework: r"org\.junit|@Test",
        assertion: r"\bassert\w*\s*\(",
        good_structures: r"\b(HashMap|HashSet|TreeMap|TreeSet|PriorityQueue|ArrayDeque|LinkedList|ArrayList)\b",
        debug_print: r"System\.out\.print|printStackTrace\(",
        commented_code: r"(?m)^\s*//\s*(if|for|while|return|System\.|int |String |public |private )",
        long_method: Some(r"\{[^{}]{1600,}\}"),
        vocab: Vocab {
            doc_name: "Javadoc",
            test_framework_name: "JUnit",
            interfaces_label: "interface(s)",
            types_label: "class(es)",
            abstract_label: "abstract class(es)",
            module_label: "package declaration(s)",
            add_tests_fix: "Add JUnit tests (`@Test`, assertions) — no test framework usage detected.",
            add_docs_fix: "Add Javadoc to every public class, interface, and method (purpose, @param, @return, @throws).",
            add_build_fix: "Add a build file (pom.xml / build.gradle / Makefile) so the project builds reproducibly.",
            add_module_fix: "Organize classes into Java packages rather than the default package.",
            add_abstraction_fix: "Increase abstraction: define interfaces for major roles; avoid depending on concrete types.",
            program_to_interfaces_fix: "Program to interfaces: expose behavior through interfaces, keep concrete classes behind them.",
            good_structures_fix: "Use appropriate data structures (HashMap/TreeMap/PriorityQueue/…) for each access pattern.",
        },
    }
}

/// Rust — `pub` items, `///`/`//!`/`/** */` docs, traits, `cargo test`.
pub fn rust() -> LangProfile {
    LangProfile {
        name: "Rust",
        source_exts: &[".rs"],
        test_dir: r"(?i)(^|/)tests?/",
        test_suffix: r"(_tests?|tests?)\.rs$",
        test_basename: r"tests?\b",
        public_decl: r"\bpub(?:\s*\([^)]*\))?\s+(?:async\s+)?(?:unsafe\s+)?(?:fn|struct|enum|trait|type|mod|const|static)\b",
        doc_block: r"(?m)(?:^[ \t]*//[/!].*\n)+|/\*\*[\s\S]*?\*/",
        interface_decl: r"\btrait\s+\w+",
        type_decl: r"\b(?:struct|enum)\s+\w+",
        abstract_decl: r"\bimpl\s+\w+\s+for\s+\w+",
        module_decl: r"(?m)^\s*(?:pub\s+)?mod\s+\w+",
        build_files: r"(^|/)(Cargo\.toml|Makefile)$",
        src_layout: r"(^|/)src/",
        test_framework: r"#\[\s*(?:tokio::)?test\s*\]|#\[\s*cfg\s*\(\s*test\s*\)\s*\]|\bassert(?:_eq|_ne)?!",
        assertion: r"\bassert\w*!\s*\(",
        good_structures: r"\b(HashMap|BTreeMap|HashSet|BTreeSet|BinaryHeap|VecDeque|LinkedList|Vec)\b",
        debug_print: r"\b(?:e?println|dbg)!\s*\(",
        commented_code: r"(?m)^\s*//\s*(if|for|while|return|let |fn |pub |match |println)",
        long_method: Some(r"\{[^{}]{1600,}\}"),
        vocab: Vocab {
            doc_name: "doc-comment",
            test_framework_name: "cargo test",
            interfaces_label: "trait(s)",
            types_label: "type(s)",
            abstract_label: "trait impl(s)",
            module_label: "module declaration(s)",
            add_tests_fix: "Add `#[test]` unit tests (and integration tests under tests/) — no test usage detected.",
            add_docs_fix: "Add `///` doc comments to every public item (modules, types, traits, functions): purpose, parameters, returns/errors.",
            add_build_fix: "Add a Cargo.toml so the crate builds reproducibly with `cargo build`.",
            add_module_fix: "Organize code into `mod`s (and a clear crate/module tree) rather than one flat file.",
            add_abstraction_fix: "Increase abstraction: define traits for major roles; depend on traits, not concrete types.",
            program_to_interfaces_fix: "Program to traits: expose behavior through traits and keep concrete types behind them.",
            good_structures_fix: "Use appropriate data structures (HashMap/BTreeMap/BinaryHeap/VecDeque/…) for each access pattern.",
        },
    }
}

/// Python — `def`/`class`, `"""docstrings"""`, ABCs/Protocols, pytest/unittest.
pub fn python() -> LangProfile {
    LangProfile {
        name: "Python",
        source_exts: &[".py"],
        test_dir: r"(?i)(^|/)tests?/",
        test_suffix: r"(_test|test_\w+)\.py$",
        test_basename: r"(?i)(^test_|_test\.py$|^test\.py$)",
        public_decl: r"(?m)^\s*(?:async\s+)?(?:def|class)\s+[A-Za-z]\w*",
        doc_block: r#""""[\s\S]*?"""|'''[\s\S]*?'''"#,
        interface_decl: r"class\s+\w+\s*\([^)]*(?:ABC|Protocol)[^)]*\)",
        type_decl: r"(?m)^\s*class\s+\w+",
        abstract_decl: r"@abstractmethod|\(\s*ABC\s*\)|\bProtocol\b",
        module_decl: r"(?m)^\s*(?:from\s+[\w.]+\s+import|import\s+\w+)",
        build_files: r"(^|/)(pyproject\.toml|setup\.py|setup\.cfg|requirements\.txt|Makefile)$",
        src_layout: r"(^|/)(src|tests?)/",
        test_framework: r"\bimport\s+pytest|\bimport\s+unittest|@pytest|(?m)^\s*def\s+test_\w+",
        assertion: r"(?m)^\s*assert\b|self\.assert\w+\s*\(",
        good_structures: r"\b(dict|set|frozenset|deque|defaultdict|OrderedDict|Counter|heapq|namedtuple)\b",
        debug_print: r"(?m)^\s*print\s*\(|pprint\s*\(|traceback\.print",
        commented_code: r"(?m)^\s*#\s*(if|for|while|return|def |class |print\()",
        long_method: None,
        vocab: Vocab {
            doc_name: "docstring",
            test_framework_name: "pytest/unittest",
            interfaces_label: "protocol/ABC(s)",
            types_label: "class(es)",
            abstract_label: "abstract method/ABC(s)",
            module_label: "import/module statement(s)",
            add_tests_fix: "Add pytest/unittest tests (`def test_…`, assertions) — no test usage detected.",
            add_docs_fix: "Add docstrings to every public module, class, and function describing purpose, args, and returns/raises.",
            add_build_fix: "Add a build/dependency manifest (pyproject.toml / setup.py / requirements.txt).",
            add_module_fix: "Organize code into packages/modules (with __init__.py) rather than one flat script.",
            add_abstraction_fix: "Increase abstraction: define Protocols/ABCs for major roles; depend on those, not concrete classes.",
            program_to_interfaces_fix: "Program to Protocols/ABCs: expose behavior through abstract base types and keep concrete classes behind them.",
            good_structures_fix: "Use appropriate data structures (dict/set/deque/heapq/Counter/…) for each access pattern.",
        },
    }
}

/// TypeScript / JavaScript — `export` decls, JSDoc, interfaces, Jest/Vitest.
pub fn typescript() -> LangProfile {
    LangProfile {
        name: "TypeScript/JavaScript",
        source_exts: &[".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"],
        test_dir: r"(?i)(^|/)(tests?|__tests__)/",
        test_suffix: r"\.(test|spec)\.[cm]?[jt]sx?$",
        test_basename: r"(?i)(test|spec)",
        public_decl: r"\bexport\s+(?:default\s+)?(?:async\s+)?(?:function|class|interface|const|let|type|enum)\b",
        doc_block: r"/\*\*[\s\S]*?\*/",
        interface_decl: r"\binterface\s+\w+|\btype\s+\w+\s*=",
        type_decl: r"\bclass\s+\w+",
        abstract_decl: r"\babstract\s+class\s+\w+",
        module_decl: r"(?m)^\s*(?:import|export)\s",
        build_files: r"(^|/)(package\.json|tsconfig\.json|Makefile)$",
        src_layout: r"(^|/)src/",
        test_framework: r"\b(?:describe|it|test|expect)\s*\(|@jest|vitest|mocha",
        assertion: r"\bexpect\s*\(|\bassert\w*\s*\(",
        good_structures: r"\b(Map|Set|WeakMap|WeakSet)\b",
        debug_print: r"console\.(?:log|debug|error|warn)\s*\(",
        commented_code: r"(?m)^\s*//\s*(if|for|while|return|const |let |function |class )",
        long_method: Some(r"\{[^{}]{1600,}\}"),
        vocab: Vocab {
            doc_name: "JSDoc",
            test_framework_name: "Jest/Vitest",
            interfaces_label: "interface/type(s)",
            types_label: "class(es)",
            abstract_label: "abstract class(es)",
            module_label: "import/export statement(s)",
            add_tests_fix: "Add Jest/Vitest tests (`describe`/`it`, `expect`) — no test usage detected.",
            add_docs_fix: "Add JSDoc to every exported function, class, and type (purpose, @param, @returns).",
            add_build_fix: "Add a package.json (and tsconfig.json for TypeScript) so the project builds reproducibly.",
            add_module_fix: "Split code into ES modules with explicit imports/exports rather than one large file.",
            add_abstraction_fix: "Increase abstraction: define interfaces/types for major roles; depend on those, not concretions.",
            program_to_interfaces_fix: "Program to interfaces: expose behavior through interfaces/types and keep concrete classes behind them.",
            good_structures_fix: "Use appropriate data structures (Map/Set/typed arrays/…) for each access pattern.",
        },
    }
}

/// Go — exported (capitalized) decls, `//` doc comments, interfaces, `go test`.
pub fn go() -> LangProfile {
    LangProfile {
        name: "Go",
        source_exts: &[".go"],
        test_dir: r"(?i)(^|/)tests?/",
        test_suffix: r"_test\.go$",
        test_basename: r"_test\b",
        public_decl: r"\bfunc\s+(?:\([^)]*\)\s*)?[A-Z]\w*|\btype\s+[A-Z]\w*",
        doc_block: r"(?m)(?:^[ \t]*//.*\n)+",
        interface_decl: r"\btype\s+\w+\s+interface\b",
        type_decl: r"\btype\s+\w+\s+struct\b",
        abstract_decl: r"\binterface\s*\{",
        module_decl: r"(?m)^\s*package\s+\w+",
        build_files: r"(^|/)(go\.mod|Makefile)$",
        src_layout: r"(^|/)(cmd|internal|pkg)/",
        test_framework: r"\btesting\.[TB]\b|(?m)^\s*func\s+Test\w+",
        assertion: r"t\.(?:Error|Errorf|Fatal|Fatalf)\s*\(|\bif\s+\w+\s*!=\s*nil",
        good_structures: r"\b(map\[|sync\.Map|container/heap|container/list|container/ring)\b",
        debug_print: r"\bfmt\.Print|\blog\.Print|\bprintln\s*\(",
        commented_code: r"(?m)^\s*//\s*(if|for|return|func |var |fmt\.)",
        long_method: Some(r"\{[^{}]{1600,}\}"),
        vocab: Vocab {
            doc_name: "doc-comment",
            test_framework_name: "go test",
            interfaces_label: "interface(s)",
            types_label: "struct(s)",
            abstract_label: "interface type(s)",
            module_label: "package declaration(s)",
            add_tests_fix: "Add `_test.go` tests (`func TestXxx(t *testing.T)`) — no test usage detected.",
            add_docs_fix: "Add doc comments above every exported identifier (starting with its name), per Go convention.",
            add_build_fix: "Add a go.mod so the module builds reproducibly with `go build`.",
            add_module_fix: "Organize code into packages (cmd/internal/pkg) rather than one flat package.",
            add_abstraction_fix: "Increase abstraction: define interfaces for major roles; accept interfaces, return structs.",
            program_to_interfaces_fix: "Program to interfaces: accept interface parameters and keep concrete structs behind them.",
            good_structures_fix: "Use appropriate data structures (map, container/heap, sync.Map, …) for each access pattern.",
        },
    }
}

/// C / C++ — classes/structs, Doxygen comments, GoogleTest/Catch2, CMake/Make.
pub fn cpp() -> LangProfile {
    LangProfile {
        name: "C/C++",
        source_exts: &[".c", ".h", ".cpp", ".hpp", ".cc", ".cxx", ".hh"],
        test_dir: r"(?i)(^|/)tests?/",
        test_suffix: r"(_test|test_\w+)\.(c|cc|cpp|cxx)$",
        test_basename: r"(?i)(^test_|_test\.)",
        public_decl: r"\b(?:class|struct)\s+\w+|^\s*[\w:<>*&]+\s+\w+\s*\([^;]*\)\s*\{",
        doc_block: r"/\*\*[\s\S]*?\*/|(?m)(?:^[ \t]*///.*\n)+",
        interface_decl: r"\bclass\s+\w+[^{};]*\bvirtual\b|=\s*0\s*;",
        type_decl: r"\b(?:class|struct)\s+\w+",
        abstract_decl: r"\bvirtual\b",
        module_decl: r"(?m)^\s*namespace\s+\w+",
        build_files: r"(^|/)(CMakeLists\.txt|Makefile|configure\.ac|meson\.build)$",
        src_layout: r"(^|/)(src|include)/",
        test_framework: r"\b(?:TEST|TEST_F|TEST_CASE|REQUIRE|EXPECT_\w+|ASSERT_\w+)\b|gtest|catch2",
        assertion: r"\b(?:assert|REQUIRE|CHECK|EXPECT_\w+|ASSERT_\w+)\s*\(",
        good_structures: r"\bstd::(?:unordered_map|map|set|unordered_set|priority_queue|deque|list|vector)\b",
        debug_print: r"\b(?:printf|fprintf)\s*\(|\bstd::cout\b",
        commented_code: r"(?m)^\s*//\s*(if|for|while|return|int |std::)",
        long_method: Some(r"\{[^{}]{1600,}\}"),
        vocab: Vocab {
            doc_name: "doc-comment",
            test_framework_name: "GoogleTest/Catch2",
            interfaces_label: "abstract class(es)",
            types_label: "class/struct(s)",
            abstract_label: "virtual/abstract type(s)",
            module_label: "namespace declaration(s)",
            add_tests_fix: "Add GoogleTest/Catch2 tests (TEST/REQUIRE, assertions) — no test usage detected.",
            add_docs_fix: "Add Doxygen comments to every public class and function (purpose, @param, @return).",
            add_build_fix: "Add a build file (CMakeLists.txt / Makefile) so the project builds reproducibly.",
            add_module_fix: "Organize code into namespaces and a src/include layout rather than the global namespace.",
            add_abstraction_fix: "Increase abstraction: define abstract base classes (pure virtual) for major roles.",
            program_to_interfaces_fix: "Program to abstract interfaces: depend on abstract base classes, keep concretions behind them.",
            good_structures_fix: "Use appropriate data structures (std::unordered_map/map/priority_queue/…) for each access pattern.",
        },
    }
}

/// Generic fallback when no known language dominates: permissive C-like and
/// hash-comment heuristics over common source extensions. Scores the
/// language-agnostic categories without over-claiming language specifics.
pub fn generic() -> LangProfile {
    LangProfile {
        name: "source",
        source_exts: &[
            ".java", ".rs", ".py", ".ts", ".tsx", ".js", ".jsx", ".go", ".rb", ".kt",
            ".swift", ".scala", ".cs", ".c", ".h", ".cpp", ".hpp", ".cc",
        ],
        test_dir: r"(?i)(^|/)(tests?|__tests__|spec)/",
        test_suffix: r"(?i)(_test|test_\w+|\.test|\.spec)\.[a-z]+$",
        test_basename: r"(?i)(test|spec)",
        public_decl: r"\b(?:pub|public|export)\b|(?m)^\s*(?:def|func|fn|function)\s+\w+",
        doc_block: r#"/\*\*[\s\S]*?\*/|(?m)(?:^[ \t]*(?:///|//!|##).*\n)+|"""[\s\S]*?""""#,
        interface_decl: r"\b(?:interface|trait|protocol)\s+\w+",
        type_decl: r"\b(?:class|struct|enum|type)\s+\w+",
        abstract_decl: r"\b(?:abstract|virtual|trait|interface)\b",
        module_decl: r"(?m)^\s*(?:package|module|namespace|mod)\b|^\s*(?:import|from)\s",
        build_files: r"(^|/)(Makefile|CMakeLists\.txt|pom\.xml|build\.gradle|Cargo\.toml|go\.mod|package\.json|pyproject\.toml|setup\.py)$",
        src_layout: r"(^|/)(src|lib|cmd|internal|pkg)/",
        test_framework: r"@Test|#\[\s*test|\bdef\s+test_|func\s+Test|\b(?:describe|it|expect)\s*\(|TEST\b|REQUIRE\b",
        assertion: r"\bassert\w*[!]?\s*\(|\bexpect\s*\(|\bREQUIRE\s*\(",
        good_structures: r"\b(HashMap|TreeMap|BTreeMap|PriorityQueue|BinaryHeap|VecDeque|Deque|unordered_map|priority_queue|heapq|defaultdict)\b",
        debug_print: r"\b(?:println|printf|print|console\.log|fmt\.Print|System\.out\.print)\b",
        commented_code: r"(?m)^\s*(?://|#)\s*(if|for|while|return|def |fn |func |class |print)",
        long_method: Some(r"\{[^{}]{1600,}\}"),
        vocab: Vocab {
            doc_name: "doc-comment",
            test_framework_name: "test framework",
            interfaces_label: "interface(s)",
            types_label: "type(s)",
            abstract_label: "abstract type(s)",
            module_label: "module declaration(s)",
            add_tests_fix: "Add automated tests with assertions — no test framework usage detected.",
            add_docs_fix: "Add doc comments to every public type and function (purpose, parameters, returns/errors).",
            add_build_fix: "Add a build/dependency manifest so the project builds reproducibly.",
            add_module_fix: "Organize code into modules/packages with a clear directory structure.",
            add_abstraction_fix: "Increase abstraction: define interfaces for major roles; depend on abstractions, not concretions.",
            program_to_interfaces_fix: "Program to interfaces: expose behavior through abstractions and keep concrete types behind them.",
            good_structures_fix: "Use appropriate data structures (maps/sets/heaps/queues/…) for each access pattern.",
        },
    }
}
