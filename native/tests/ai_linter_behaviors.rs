//! The AI linter's user-facing contract, tested through its REAL public interface: the built
//! binary, invoked exactly as a user or agent invokes it (`helpers-native call lint` with a JSON
//! request on stdin). Each test is one behavior the tool promises:
//!
//!   * a language it has never seen is lintable with nothing but a plain-English rule file;
//!   * an instruction with no language named is law across every language in the project;
//!   * a new real language is set up with a DATA edit (a docs URL), never a code change;
//!   * it never crashes on the junk real repos contain;
//!   * offline and cold it still enforces the project's law.
//!
//! Tests are hermetic: caches and the learned-source registry are redirected into the test's
//! temp dir (`HELPERS_LINT_MODELS`, `HOME`), and network learning is disabled
//! (`HELPERS_LINT_OFFLINE`) except where a test serves its own docs from localhost.

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// A throwaway project directory plus the isolated cache/home dirs a hermetic lint run needs.
struct TestProject {
    root: PathBuf,
}

impl TestProject {
    /// Create an empty git project under a unique temp dir.
    fn new(name: &str) -> TestProject {
        let root = std::env::temp_dir().join(format!("ai-lint-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp project dir");
        let ok = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .status()
            .expect("git available")
            .success();
        assert!(ok, "git init failed");
        TestProject { root }
    }

    /// Write a file (creating parents) relative to the project root.
    fn write(&self, rel: &str, content: &str) -> &Self {
        let path = self.root.join(rel);
        fs::create_dir_all(path.parent().expect("has parent")).expect("mkdir");
        fs::write(path, content).expect("write fixture file");
        self
    }

    /// Run `helpers-native call lint` on this project and return the rendered verdict text.
    /// `offline` disables network learning so the run is deterministic.
    fn lint(&self, offline: bool) -> String {
        self.call("lint", &format!(r#"{{"root":{:?}}}"#, self.root.to_string_lossy()), offline)
    }

    /// Run any tool of the binary against this project; returns the tool's rendered text.
    fn call(&self, tool: &str, request: &str, offline: bool) -> String {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_helpers-native"));
        cmd.args(["call", tool])
            .current_dir(&self.root)
            .env("HELPERS_LINT_MODELS", self.root.join(".test-models"))
            .env("HOME", self.root.join(".test-home"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if offline {
            cmd.env("HELPERS_LINT_OFFLINE", "1");
        }
        let mut child = cmd.spawn().expect("binary runs");
        child
            .stdin
            .as_mut()
            .expect("stdin piped")
            .write_all(request.as_bytes())
            .expect("request written");
        let out = child.wait_with_output().expect("binary exits");
        assert!(
            out.status.success(),
            "`call {tool}` must exit cleanly.\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        let raw = String::from_utf8_lossy(&out.stdout).into_owned();
        let json: serde_json::Value = serde_json::from_str(raw.trim())
            .unwrap_or_else(|e| panic!("tool output is protocol JSON ({e}): {raw}"));
        json["content"][0]["text"].as_str().expect("text content").to_string()
    }
}

/// True when `rule` is flagged within `file`'s section of the verdict.
fn flagged_in(verdict: &str, file: &str, rule: &str) -> bool {
    verdict
        .lines()
        .skip_while(|l| !l.contains(file))
        .take_while(|l| !l.trim().is_empty())
        .any(|l| l.contains(rule))
}

/// The line of `text` that contains `needle`, for focused assertions with full context on failure.
fn line_with<'a>(text: &'a str, needle: &str) -> Option<&'a str> {
    text.lines().find(|l| l.contains(needle))
}

// ── B1: a language the system has never seen ──────────────────────────────────

/// Files in a made-up language + one plain-English project rule file = enforcement. No grammar,
/// no docs, no toolchain, no code change, no config: the rule file IS the entire setup.
#[test]
fn unseen_language_is_lintable_with_a_plain_english_rule_alone() {
    let p = TestProject::new("unseen-lang");
    p.write(
        "pipeline.zlang",
        "flow main:\n    emit(\"start\")\n    panic(\"boom\")\n    emit(\"end\")\n",
    );
    p.write(
        "util.zlang",
        "flow helper:\n    emit(\"ok\")\n",
    );
    p.write(
        ".helpers/lint-rules/zlang.md",
        "## no_panic [high]\nNever use `panic` in zlang code; raise a typed error instead.\n\n```zlang:bad\npanic(\"boom\")\n```\n",
    );

    let verdict = p.lint(true);

    let hit = line_with(&verdict, "no_panic")
        .unwrap_or_else(|| panic!("the zlang rule must fire on pipeline.zlang:\n{verdict}"));
    assert!(hit.contains("[high]"), "severity from the rule file is honored: {hit}");
    assert!(
        verdict.contains("pipeline.zlang"),
        "the violating file is named in the verdict:\n{verdict}"
    );
    assert!(
        !line_with(&verdict, "util.zlang").is_some_and(|l| l.contains("no_panic")),
        "the clean file must not be flagged:\n{verdict}"
    );
}

// ── B1b: the FP/FN contract, executable ────────────────────────────────────────

/// A generated law×construct matrix is planted in one project: for every construct a violating
/// file AND a trap file (the construct inside every quote style, inside comments, quoted from
/// the law itself, and embedded in larger identifiers). One offline run must flag EXACTLY the
/// violation set — the verdict's own issue count is asserted, so any extra finding is a false
/// positive and any missing one a false negative. Coverage grows by adding a matrix row, never
/// another test (construct shapes: plain word, snake_case, dotted, bare numeric).
#[test]
fn planted_violations_are_flagged_exactly_no_more_no_less() {
    let p = TestProject::new("ground-truth");
    const MATRIX: &[(&str, &str, &str)] = &[
        // (rule id, construct as the law writes it, the violating line)
        ("no_zap", "zap", "zap(input)"),
        ("no_secret_token", "secret_token", "secret_token = load()"),
        ("no_console_log", "console.log", "console.log(x)"),
        ("no_port_9099", "9099", "listen(9099)"),
    ];
    let mut law = String::new();
    for (id, written, _) in MATRIX {
        law.push_str(&format!(
            "## {id} [high]\nNever use `{written}` anywhere in this project; use the approved helper instead.\n\n"
        ));
    }
    p.write(".helpers/lint-rules/vlang.md", &law);
    for (id, written, bad_line) in MATRIX {
        p.write(&format!("src/bad_{id}.vlang"), &format!("unit start:\n    {bad_line}\n"));
        p.write(
            &format!("src/clean_{id}.vlang"),
            &format!(
                "# Never use `{written}` anywhere in this project; use the approved helper instead.\n\
                 note = \"{written} is banned in this repo\"\n\
                 memo = '{written} again'\n\
                 shim_zapper = 1\n"
            ),
        );
    }

    let verdict = p.lint(true);

    for (id, _, _) in MATRIX {
        assert!(
            flagged_in(&verdict, &format!("bad_{id}.vlang"), id),
            "false negative: {id} must fire in bad_{id}.vlang:\n{verdict}"
        );
        assert!(
            !flagged_in(&verdict, &format!("clean_{id}.vlang"), id),
            "false positive: {id} fired on strings/comments in clean_{id}.vlang:\n{verdict}"
        );
    }
    let want = format!("Verdict: {} issue(s)", MATRIX.len());
    assert!(
        verdict.contains(&want),
        "exactly one finding per planted violation and nothing else ({want}):\n{verdict}"
    );
}

/// The planted matrix's alias + universe dimensions (native/architecture.dx ledger #16 + evidence
/// hierarchy), end to end: the law file is named by extension alias (`js.md`) and must govern
/// its canonical language (javascript), and each row plants its violation in a different text
/// universe — code, a comment, a string interior. Exact-count contract like the matrix above;
/// a new alias or universe bug becomes a row, never a test function.
#[test]
fn alias_named_law_files_and_comment_or_string_constructs_are_enforced() {
    // (rule id, the law, the violating line — one per universe)
    const MATRIX: &[(&str, &str, &str)] = &[
        ("no_eval", "Never use `eval` in JavaScript code.", "eval(\"x\");"),
        ("no_todo", "Never leave TODO markers in committed code.", "// TODO: refactor this"),
        ("no_port_8080", "Never hardcode port 8080 anywhere.", "const conn = connect(\":8080\");"),
    ];
    let p = TestProject::new("alias-and-universe");
    let law: String =
        MATRIX.iter().map(|(id, law, _)| format!("## {id} [high]\n{law}\n\n")).collect();
    p.write(".helpers/lint-rules/js.md", &law);
    let bad: String = MATRIX.iter().map(|(_, _, line)| format!("{line}\n")).collect();
    p.write("src/app.js", &bad);
    // The clean file avoids the laws' own nouns ("port"): a project-grounded word wins
    // selection by document order, so `cfg.port` here would legitimately pull the 8080 law
    // onto `port` (an author pins that case by backticking the construct).
    p.write("src/clean.js", "parse(\"x\");\nconst conn = connect(cfg.address);\n");

    let verdict = p.lint(true);

    for (id, _, _) in MATRIX {
        assert!(
            flagged_in(&verdict, "src/app.js", id),
            "false negative: {id} must fire in src/app.js (alias stem + grounding universe):\n{verdict}"
        );
    }
    assert!(
        !verdict.lines().any(|l| l.contains("clean.js")),
        "clean.js plants nothing and must stay clean:\n{verdict}"
    );
    let want = format!("Verdict: {} issue(s)", MATRIX.len());
    assert!(verdict.contains(&want), "exactly one finding per planted violation ({want}):\n{verdict}");
}

/// The whole language surface, one row per language: for EVERY language the walker can name
/// (canonical names through the learned extension claims, data languages by extension), a project
/// law naming an invented construct must fire on the file that uses it and stay silent on the
/// clean file — exact issue count, so a false positive or negative in ANY language's
/// parse→mask→fire path fails the run. A new language is a new row, never a new test.
#[test]
fn every_language_the_walker_names_enforces_project_law() {
    // (canonical language, file extension). The construct is `zapcall<lang>` — invented, so
    // nothing pre-trained can know it; the law file stem is the canonical language name.
    const LANGS: &[(&str, &str)] = &[
        ("rust", "rs"),
        ("python", "py"),
        ("javascript", "js"),
        ("typescript", "ts"),
        ("go", "go"),
        ("java", "java"),
        ("ruby", "rb"),
        ("c", "c"),
        ("cpp", "cpp"),
        ("bash", "sh"),
        ("kotlin", "kt"),
        ("swift", "swift"),
        // csharp has no registered docs, so ".cs" is honestly the language named "cs" —
        // the ask-for-docs seam, not a hardcoded alias (the old file_lang table's csharp
        // wiring was knowledge the system never actually had).
        ("cs", "cs"),
        ("php", "php"),
        ("zig", "zig"),
        ("json", "json"),
        ("css", "css"),
        ("html", "html"),
        ("svg", "svg"),
        ("toml", "toml"),
        ("xml", "xml"),
        ("yaml", "yml"),
        ("markdown", "md"),
    ];
    let p = TestProject::new("every-language");
    for (lang, ext) in LANGS {
        p.write(
            &format!(".helpers/lint-rules/{lang}.md"),
            &format!(
                "## zap_{lang} [high]\nNever use zapcall{lang} anywhere in this project; use the approved helper instead.\n"
            ),
        );
        p.write(&format!("src/bad_{lang}.{ext}"), &format!("zapcall{lang}(1)\n"));
        p.write(&format!("src/clean_{lang}.{ext}"), "helper(1)\n");
    }

    let verdict = p.lint(true);

    for (lang, _) in LANGS {
        assert!(
            flagged_in(&verdict, &format!("bad_{lang}."), &format!("zap_{lang}")),
            "false negative: {lang}'s law must fire in bad_{lang}:\n{verdict}"
        );
        assert!(
            !flagged_in(&verdict, &format!("clean_{lang}."), &format!("zap_{lang}")),
            "false positive: {lang}'s law fired on the clean file:\n{verdict}"
        );
    }
    let want = format!("Verdict: {} issue(s)", LANGS.len());
    assert!(
        verdict.contains(&want),
        "exactly one finding per language and nothing else ({want}):\n{verdict}"
    );
}

// ── B2: one instruction, every language ───────────────────────────────────────

/// An `any.md` rule that names no language is the project's law for EVERY language present —
/// grammar languages (python, javascript) and grammarless made-up ones alike. And a law whose
/// examples differ only in punctuation (no watchable word) is REPORTED as not yet enforceable
/// for a grammarless language — never silently skipped, never a junk detector (native/architecture.dx,
/// open problems: values and operators are semantics; the AST diff is the path that carries
/// them, and qlang has no grammar).
#[test]
fn an_instruction_with_no_language_governs_every_language_in_the_project() {
    let p = TestProject::new("any-lang");
    p.write("app.py", "def main():\n    scores = mutcell(90, 85, 77)\n    return scores\n");
    p.write("web.js", "const nums = mutcell(1, 2, 3);\nconsole.log(nums);\n");
    p.write("data.qlang", "let sizes = mutcell(4, 5, 6)\n");
    p.write(
        ".helpers/lint-rules/any.md",
        "## q_no_containers [high]\nDo not use mutcell containers anywhere in this project. Use keyed structures instead.\n\n```bad\nxs = mutcell(1, 2, 3)\n```\n\n```good\nxs = fixcell(1, 2, 3)\n```\n\n## q_no_arrays [high]\nxyzzy qwerty plugh zork.\n\n```bad\nxs = [1, 2, 3]\n```\n\n```good\nxs = {\"a\": 1, \"b\": 2, \"c\": 3}\n```\n",
    );

    let verdict = p.lint(true);

    for file in ["app.py", "web.js", "data.qlang"] {
        let section_hit = flagged_in(&verdict, file, "q_no_containers");
        assert!(
            section_hit,
            "any.md q_no_containers must fire on {file} — one instruction governs every language:\n{verdict}"
        );
    }
    assert!(
        verdict.contains("not yet enforceable") && verdict.contains("q_no_arrays"),
        "a punctuation-only law must be reported as not yet enforceable for the grammarless \
         language, never silently skipped:\n{verdict}"
    );
}

// ── B3: prose-only instruction ─────────────────────────────────────────────────

/// One English sentence naming a construct — no code example, no fences — is enough to enforce.
/// The instruction file is literally just a heading and a sentence.
#[test]
fn a_prose_only_instruction_with_no_code_example_is_enforced() {
    let p = TestProject::new("prose-only");
    p.write(
        "job.py",
        "def run(expr):\n    value = eval(expr)\n    return value\n",
    );
    p.write("safe.py", "def add(a, b):\n    return a + b\n");
    p.write(
        ".helpers/lint-rules/python.md",
        "## no_eval [high]\nNever call `eval` anywhere in this project; parse the input explicitly.\n",
    );

    let verdict = p.lint(true);

    let job_hit = flagged_in(&verdict, "job.py", "no_eval");
    assert!(job_hit, "the prose-only rule must fire on job.py's eval call:\n{verdict}");
    assert!(
        !flagged_in(&verdict, "safe.py", "no_eval"),
        "safe.py has no eval and must stay clean:\n{verdict}"
    );
}

// ── B4: a new language is a DATA edit ─────────────────────────────────────────

/// Serve a tiny documentation site from localhost: an index linking to one rule-ish page whose
/// prose deprecates `goto` next to a bad example, plus an endorsed alternative.
fn serve_flowlang_docs() -> String {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind localhost");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 2048];
            let n = stream.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).into_owned();
            let body = if req.starts_with("GET /statements") {
                "<html><body><h1>Statements</h1>\
                 <p>Never use the goto statement anywhere; it is deprecated and will be removed.</p>\
                 <pre>goto cleanup</pre>\
                 <p>Use a structured loop instead; this is the correct form:</p>\
                 <pre>loop { step() }</pre>\
                 </body></html>"
            } else {
                "<html><body><h1>flowlang reference</h1>\
                 <p>The flowlang language reference.</p>\
                 <a href=\"/statements.html\">Statements</a>\
                 </body></html>"
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
        }
    });
    format!("http://127.0.0.1:{port}/")
}

/// A localhost WEBSITE documenting TWO invented languages, each under its own path segment —
/// the "A site is a source" fixture: nothing in the codebase can know either language, and the
/// only registration is the SITE.
fn serve_two_language_site() -> String {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind localhost");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 2048];
            let n = stream.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).into_owned();
            let body = if req.starts_with("GET /docs/flowlang/") {
                "<html><body><h1>flowlang statements</h1>\
                 <p>Never use the goto statement anywhere; it is deprecated and will be removed.</p>\
                 <pre>goto cleanup</pre>\
                 <p>Use a structured loop instead; this is the correct form:</p>\
                 <pre>loop { step() }</pre>\
                 </body></html>"
            } else if req.starts_with("GET /docs/glowlang/") {
                "<html><body><h1>glowlang elements</h1>\
                 <p>Never use the blink element anywhere; it is deprecated and will be removed.</p>\
                 <pre>blink fast</pre>\
                 <p>Use a steady element instead; this is the correct form:</p>\
                 <pre>steady on</pre>\
                 </body></html>"
            } else {
                "<html><body><h1>the languages of this site</h1>\
                 <p>Documentation for two little languages.</p>\
                 <a href=\"/docs/flowlang/statements.html\">flowlang</a>\
                 <a href=\"/docs/glowlang/elements.html\">glowlang</a>\
                 </body></html>"
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
        }
    });
    format!("http://127.0.0.1:{port}/")
}

/// "A site is a source": ONE site registration — no per-language URLs anywhere — and the
/// system maps the site, attributes each page to the language it documents (path-segment
/// typography here), mints a module PER language, and the offline lint that follows enforces
/// each language's own rules on its own files with no cross-fire.
#[test]
fn one_site_registration_yields_a_module_per_language_it_teaches() {
    let url = serve_two_language_site();
    let p = TestProject::new("site-source");
    p.write("main.flowlang", "start:\n    goto cleanup\n    emit(\"done\")\n");
    p.write("main.glowlang", "scene:\n    blink fast\n    hold(\"done\")\n");
    p.write(
        "lint-index/sources.json",
        &format!(r#"{{"version": 3, "sources": [{{"tool": "twolang-site", "language": "site", "kind": "site", "seed": "{url}"}}]}}"#),
    );

    // TRAIN (network allowed, localhost only): the site is crawled once, its languages
    // discovered and trained.
    let ack = p.call(
        "lint_config",
        &format!(r#"{{"root":{:?},"action":"train"}}"#, p.root.to_string_lossy()),
        false,
    );
    assert!(
        ack.contains("teaches: flowlang, glowlang"),
        "the site's languages are discovered and reported:\n{ack}"
    );
    assert!(
        line_with(&ack, "flowlang:").is_some_and(|l| l.contains("rule(s)"))
            && line_with(&ack, "glowlang:").is_some_and(|l| l.contains("rule(s)")),
        "both discovered languages train to modules:\n{ack}"
    );

    // OFFLINE lint: each language's own docs-derived law fires on its own file — and never
    // on the other's.
    let verdict = p.lint(true);
    assert!(
        flagged_in(&verdict, "main.flowlang", "goto"),
        "flowlang's own rule fires on flowlang code:\n{verdict}"
    );
    assert!(
        flagged_in(&verdict, "main.glowlang", "blink"),
        "glowlang's own rule fires on glowlang code:\n{verdict}"
    );
    assert!(
        !flagged_in(&verdict, "main.glowlang", "goto") && !flagged_in(&verdict, "main.flowlang", "blink"),
        "no cross-language fire between the site's languages:\n{verdict}"
    );
}

/// Registering a docs URL for a brand-new language — one JSON entry, pure data — makes one
/// TRAINING run learn that language from its documentation, and the OFFLINE lint that follows
/// enforces what the docs deprecate. No code change, no rule files, no config beyond the URL.
#[test]
fn a_new_language_is_taught_with_one_docs_url_data_entry() {
    let url = serve_flowlang_docs();
    let p = TestProject::new("data-taught-lang");
    p.write("main.flowlang", "start:\n    goto cleanup\n    emit(\"done\")\n");
    p.write(
        "lint-index/sources.json",
        &format!(
            r#"{{"version": 3, "sources": [{{"tool": "flowdocs", "language": "flowlang", "kind": "crawl", "seed": "{url}"}}]}}"#
        ),
    );

    let trained = p.call(
        "lint_config",
        &format!(r#"{{"root":{:?},"action":"train"}}"#, p.root.to_string_lossy()),
        false, // setup is the online step: it must actually read the served docs
    );
    assert!(
        trained.contains("flowlang"),
        "training must report the data-registered language:\n{trained}"
    );

    let verdict = p.lint(true); // linting is offline: it replays the trained module

    assert!(
        verdict.to_lowercase().contains("flowlang"),
        "the new language is analyzed:\n{verdict}"
    );
    let goto_flagged = verdict
        .lines()
        .skip_while(|l| !l.contains("main.flowlang"))
        .take_while(|l| !l.trim().is_empty())
        .any(|l| l.to_lowercase().contains("goto") || l.contains("L2"));
    assert!(
        goto_flagged,
        "what the docs deprecate (goto) must be flagged from the data entry alone:\n{verdict}"
    );
}

/// Serve a documentation site that is ALL PROSE — not one code block anywhere (json.org's
/// shape: the grammar is diagrams). Reading it is still a successful read.
fn serve_prose_only_docs() -> String {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind localhost");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 2048];
            let n = stream.read(&mut buf).unwrap_or(0);
            let _ = String::from_utf8_lossy(&buf[..n]);
            let body = "<html><body><h1>proselang</h1>\
                 <p>proselang is a lightweight data-interchange format. It is easy for humans \
                 to read and write, and easy for machines to parse and generate. A value can \
                 be an object, an array, a number, or a string. Whitespace between tokens is \
                 insignificant.</p>\
                 </body></html>";
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
        }
    });
    format!("http://127.0.0.1:{port}/")
}

/// Reading IS the module (native/architecture.dx): a docs site with prose but ZERO code examples still
/// sets the language up — the trained module may hold no prohibition rules, but "not yet set
/// up" means COULD NOT READ ANYTHING, never "read it and found nothing to ban". Measured
/// before this held: json.org (its grammar is diagrams, no code blocks) reported
/// "docs not learned" off a clean 1-page read.
#[test]
fn a_prose_only_docs_site_still_sets_the_language_up() {
    let url = serve_prose_only_docs();
    let p = TestProject::new("prose-only-docs");
    p.write("data.proselang", "record start\n    field a = 1\nend\n");
    p.write(
        "lint-index/sources.json",
        &format!(
            r#"{{"version": 3, "sources": [{{"tool": "prosedocs", "language": "proselang", "kind": "crawl", "seed": "{url}"}}]}}"#
        ),
    );

    let trained = p.call(
        "lint_config",
        &format!(r#"{{"root":{:?},"action":"train"}}"#, p.root.to_string_lossy()),
        false, // setup is the online step: it must actually read the served prose
    );
    let line = trained
        .lines()
        .find(|l| l.contains("proselang"))
        .unwrap_or_else(|| panic!("training must report the language:\n{trained}"));
    assert!(
        line.contains("rule(s)"),
        "a prose-only read mints the module (zero rules is fine): {line}\n{trained}"
    );
    assert!(
        !line.contains("not learned"),
        "a language that WAS read must never report 'docs not learned': {line}\n{trained}"
    );

    let verdict = p.lint(true); // offline replay of whatever setup ensured
    let asked = line_with(&verdict, "Not yet set up").unwrap_or("");
    assert!(
        !asked.contains("proselang"),
        "a set-up language must not be asked for again at lint time: {asked}\n{verdict}"
    );
}

/// Warm runs REPLAY per-file verdicts (native/architecture.dx, "The live path"): the second run of an
/// unchanged project reports identical findings out of the verdict cache, and editing a file
/// re-lints exactly that file — the moved violation is found at its new line.
#[test]
fn warm_runs_replay_verdicts_and_edits_relint() {
    let p = TestProject::new("verdict-replay");
    p.write("lintPref.md", "Never use `zorkle` anywhere in this project.\n");
    p.write("main.qfile", "begin\n    zorkle cleanup\n    emit(\"done\")\n");
    let first = p.lint(true);
    assert!(
        flagged_in(&first, "main.qfile", "L2"),
        "the planted violation must be flagged on the first (cold) run:\n{first}"
    );
    let cache_dir = p.root.join(".test-models/lint-verdicts");
    assert!(
        std::fs::read_dir(&cache_dir).is_ok_and(|mut d| d.next().is_some()),
        "the verdict cache must exist after a run"
    );
    let second = p.lint(true);
    assert!(
        flagged_in(&second, "main.qfile", "L2"),
        "an unchanged project must replay the same verdict:\n{second}"
    );
    // Edit: the violation moves down a line — the replay must not serve the stale line.
    p.write("main.qfile", "begin\n    emit(\"start\")\n    zorkle cleanup\n    emit(\"done\")\n");
    let third = p.lint(true);
    assert!(
        flagged_in(&third, "main.qfile", "L3") && !flagged_in(&third, "main.qfile", "L2"),
        "an edited file must re-lint and find the violation at its NEW line:\n{third}"
    );
}

/// A legacy JSON module cache MIGRATES on first touch (native/architecture.dx, "Save"): the offline run
/// loads it, saves the `HLM1` container beside it, deletes the JSON — and the loaded engine
/// behaves identically: the planted violation still fires from the migrated module alone.
#[test]
fn a_legacy_json_module_cache_migrates_to_the_container_and_still_fires() {
    let tv = helpers_native::lint_train::train_version();
    let fp = format!("{:016x}", helpers_native::lint_ai::token_seed(""));
    let rules = helpers_native::lint_match::RuleSet::build(
        "zetalang",
        &[(
            "no_zorkle".to_string(),
            "high".to_string(),
            "zorkle cleanup".to_string(),
            "loop {{ step() }}".replace("{{", "{").replace("}}", "}"),
            "Never use the zorkle statement anywhere; it is deprecated and will be removed.".to_string(),
            "https://registry.example/zetalang/statements".to_string(),
            None,
        )],
        &helpers_native::lint_match::Grounding {
            reference: vec!["loop { step() }".to_string(), "emit(\"done\")".to_string()],
            ..Default::default()
        },
    );
    assert!(rules.rule_count() > 0, "the fixture module must actually compile a detector");
    let concept = helpers_native::lint_ai::ConceptModel::compile(
        &[("no_zorkle".to_string(), "Never use the zorkle statement anywhere".to_string(), "zorkle cleanup".to_string())],
        "zetalang",
    );
    let module_json = format!(
        r#"{{"version":"","train_version":"{tv}","sources_fp":"{fp}","trained_at":1,"learned_from":"legacy-test","rules":{},"concept":{}}}"#,
        rules.to_json(),
        serde_json::to_string(&concept).expect("concept serializes"),
    );

    let p = TestProject::new("module-migration");
    p.write("main.zetalang", "begin:\n    zorkle cleanup\n    emit(\"done\")\n");
    p.write(".test-models/zetalang.module.json", &module_json);

    let verdict = p.lint(true); // offline: the legacy cache is the only possible source
    let zorkle_flagged = verdict
        .lines()
        .skip_while(|l| !l.contains("main.zetalang"))
        .take_while(|l| !l.trim().is_empty())
        .any(|l| l.to_lowercase().contains("zorkle") || l.contains("L2"));
    assert!(
        zorkle_flagged,
        "the migrated module must keep flagging what the JSON module flagged:\n{verdict}"
    );
    assert!(
        p.root.join(".test-models/zetalang.module.bin").exists(),
        "migration must save the HLM1 container"
    );
    assert!(
        !p.root.join(".test-models/zetalang.module.json").exists(),
        "migration must delete the legacy JSON so the machine keeps exactly one copy"
    );
}

/// A language whose AI MODULE is PUBLISHED in the model registry is downloaded and loaded
/// AS-IS instead of re-trained — the acquisition order's "pull from GitHub first" step,
/// against a localhost registry serving a fictional language nothing in the codebase can
/// know. The published artifact is the compiled runnable module (pattern engine + concept
/// gate + provenance) — never documentation.
#[test]
fn a_published_model_is_downloaded_from_the_registry_instead_of_recrawled() {
    let tv = helpers_native::lint_train::train_version();
    // The project registers no sources for zetalang, so the local source-set fingerprint is
    // the hash of the empty set — the published entry must carry the same one to match.
    let fp = format!("{:016x}", helpers_native::lint_ai::token_seed(""));
    // Compile the module exactly as training would: the same public engines.
    let rules = helpers_native::lint_match::RuleSet::build(
        "zetalang",
        &[(
            "no_zorkle".to_string(),
            "high".to_string(),
            "zorkle cleanup".to_string(),
            "loop {{ step() }}".replace("{{", "{").replace("}}", "}"),
            "Never use the zorkle statement anywhere; it is deprecated and will be removed.".to_string(),
            "https://registry.example/zetalang/statements".to_string(),
            None,
        )],
        &helpers_native::lint_match::Grounding {
            reference: vec!["loop { step() }".to_string(), "emit(\"done\")".to_string()],
            ..Default::default()
        },
    );
    assert!(rules.rule_count() > 0, "the fixture module must actually compile a detector");
    let concept = helpers_native::lint_ai::ConceptModel::compile(
        &[("no_zorkle".to_string(), "Never use the zorkle statement anywhere".to_string(), "zorkle cleanup".to_string())],
        "zetalang",
    );
    let module = format!(
        r#"{{"version":"","train_version":"{tv}","sources_fp":"{fp}","trained_at":1,"learned_from":"registry-test","rules":{},"concept":{}}}"#,
        rules.to_json(),
        serde_json::to_string(&concept).expect("concept serializes"),
    );
    let sha = helpers_native::lint_sign::sha256_hex(module.as_bytes());
    let index = format!(
        r#"[{{"language":"zetalang","toolchain":"","train_version":"{tv}","sources":"{fp}","sha256":"{sha}","module":"zetalang@any.module.json"}}]"#
    );
    // The consumed registry is only real when a TRUSTED key signed it: mint a registry
    // identity, sign the exact index bytes, and trust the public half in the project's data.
    let (registry_priv, registry_pub) = helpers_native::lint_sign::generate_keypair();
    let signature = helpers_native::lint_sign::sign_with(index.as_bytes(), &registry_priv)
        .expect("registry key signs");
    let url = serve_json_registry(index, signature, module);

    let p = TestProject::new("registry-pull");
    p.write("main.zetalang", "begin:\n    zorkle cleanup\n    emit(\"done\")\n");
    p.write(
        "lint-index/sources.json",
        &format!(r#"{{"version": 3, "registry": "{url}", "sources": []}}"#),
    );
    p.write(
        "lint-index/trusted-keys.json",
        &format!(r#"{{"registry": ["{registry_pub}"]}}"#),
    );

    let trained = p.call(
        "lint_config",
        &format!(r#"{{"root":{:?},"action":"train"}}"#, p.root.to_string_lossy()),
        false, // setup is the online step: it must actually download from the registry
    );
    assert!(
        trained.contains("Downloaded from the model registry"),
        "training must say the module came from the registry:\n{trained}"
    );

    let verdict = p.lint(true); // linting is offline: it loads the pulled module as-is
    let zorkle_flagged = verdict
        .lines()
        .skip_while(|l| !l.contains("main.zetalang"))
        .take_while(|l| !l.trim().is_empty())
        .any(|l| l.to_lowercase().contains("zorkle") || l.contains("L2"));
    assert!(
        zorkle_flagged,
        "what the published module forbids (zorkle) must be flagged from the download alone:\n{verdict}"
    );
}

/// An attacker who can write to the registry branch — or sit on the wire — still cannot make
/// a consumer LOAD anything: an index whose signature no trusted key made simply does not
/// exist, and the machine falls through to reading the docs itself (here: zetalang has no
/// docs, so it honestly reports "not set up" instead of loading the planted module).
#[test]
fn an_unsigned_or_tampered_registry_is_never_loaded() {
    let tv = helpers_native::lint_train::train_version();
    let fp = format!("{:016x}", helpers_native::lint_ai::token_seed(""));
    let module = format!(
        r#"{{"version":"","train_version":"{tv}","sources_fp":"{fp}","trained_at":1,"learned_from":"attacker","rules":{{"lang":"zetalang","rules":[]}},"concept":{{"rules":[]}}}}"#
    );
    let sha = helpers_native::lint_sign::sha256_hex(module.as_bytes());
    let index = format!(
        r#"[{{"language":"zetalang","toolchain":"","train_version":"{tv}","sources":"{fp}","sha256":"{sha}","module":"zetalang@any.module.json"}}]"#
    );
    // Signed by the ATTACKER's key — which no consumer trusts.
    let (attacker_priv, _attacker_pub) = helpers_native::lint_sign::generate_keypair();
    let signature = helpers_native::lint_sign::sign_with(index.as_bytes(), &attacker_priv)
        .expect("attacker signs happily");
    let (_, victim_trusted_pub) = helpers_native::lint_sign::generate_keypair();
    let url = serve_json_registry(index, signature, module);

    let p = TestProject::new("registry-tamper");
    p.write("main.zetalang", "begin:\n    zorkle cleanup\n    emit(\"done\")\n");
    p.write(
        "lint-index/sources.json",
        &format!(r#"{{"version": 3, "registry": "{url}", "sources": []}}"#),
    );
    p.write(
        "lint-index/trusted-keys.json",
        &format!(r#"{{"registry": ["{victim_trusted_pub}"]}}"#),
    );

    let trained = p.call(
        "lint_config",
        &format!(r#"{{"root":{:?},"action":"train"}}"#, p.root.to_string_lossy()),
        false,
    );
    assert!(
        !trained.contains("Downloaded from the model registry"),
        "a registry signed by an untrusted key must not exist for this machine:\n{trained}"
    );
    assert!(
        trained.contains("zetalang"),
        "the language is still reported honestly (needs docs), never silently loaded:\n{trained}"
    );
}

/// A tiny HTTP server for the registry contract: `/index.json`, `/index.sig`, and exactly one
/// module file.
fn serve_json_registry(index: String, signature: String, module: String) -> String {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind localhost");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 2048];
            let n = stream.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).into_owned();
            let body = if req.starts_with("GET /index.json") {
                &index
            } else if req.starts_with("GET /index.sig") {
                &signature
            } else {
                &module
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
        }
    });
    format!("http://127.0.0.1:{port}")
}

/// Batch training runs the pipeline for every registered language and reports each outcome
/// honestly — with the network down (hermetic switch), every language is named as not learned
/// and the reply asks to reconnect, never pretends, never fails.
#[test]
fn batch_training_reports_every_registered_language_honestly() {
    let p = TestProject::new("batch-train");
    p.write(
        "lint-index/sources.json",
        r#"{"version": 3, "sources": [{"tool": "q9docs", "language": "qlang9", "kind": "crawl", "seed": "https://qlang9.example/docs/"}]}"#,
    );
    // A file so the DEFAULT (project-scoped) train has something in scope.
    p.write("main.py", "x = 1\n");

    // Default scope is MODULAR: the project has no qlang9 files, so qlang9's module is not
    // paid for — and the report says it was left untouched.
    let scoped = p.call(
        "lint_config",
        &format!(r#"{{"root":{:?},"action":"train"}}"#, p.root.to_string_lossy()),
        true, // hermetic: the network is down
    );
    assert!(
        !scoped.lines().any(|l| l.trim_start().starts_with("qlang9:")),
        "a registered language the project does not use must not train by default:\n{scoped}"
    );
    assert!(
        scoped.to_lowercase().contains("scoped to this project"),
        "the report must say unused registered languages were left untouched:\n{scoped}"
    );

    // The machine-wide batch (`all=true`) still reports every registered language honestly.
    let ack = p.call(
        "lint_config",
        &format!(r#"{{"root":{:?},"action":"train","all":true}}"#, p.root.to_string_lossy()),
        true, // hermetic: the network is down
    );
    assert!(
        ack.contains("qlang9")
            && (ack.to_lowercase().contains("not learned")
                || ack.to_lowercase().contains("not answering")),
        "every registered language's outcome is reported in the batch:\n{ack}"
    );
    assert!(
        ack.to_lowercase().contains("reconnect"),
        "a dead network must ask to reconnect, not pretend or fail:\n{ack}"
    );
}

/// The self-assembly loop end to end, with exactly two setup verbs: an unknown language makes
/// the run ASK for the addition at runtime; `add_source` registers the docs (offline-safe,
/// trains nothing); `train` (online) learns the module; the next OFFLINE run enforces what the
/// docs deprecate — online to set up, offline to run. The docs are a localhost-served
/// fictional language — nothing in the codebase can know it.
#[test]
fn an_unknown_language_is_set_up_by_adding_its_docs_source_then_training() {
    let url = serve_flowlang_docs();
    let p = TestProject::new("add-source-on-ask");
    p.write("main.flowlang", "start:\n    goto cleanup\n    emit(\"done\")\n");
    // Pin an EMPTY sources registry: `action=train` must train exactly the language this test
    // adds — never fall back to the embedded registry and crawl the real internet.
    p.write("lint-index/sources.json", r#"{"version": 3, "sources": []}"#);

    let before = p.lint(true); // linting never sets up: the run must ask for the addition
    assert!(
        before.contains("flowlang") && before.contains("action=train"),
        "the report must name the unknown language and point at the setup verb:\n{before}"
    );

    let registered = p.call(
        "lint_config",
        &format!(
            r#"{{"root":{:?},"action":"add_source","lang":"flowlang","url":"{url}"}}"#,
            p.root.to_string_lossy()
        ),
        true, // registering a source is a data write — offline-safe by design
    );
    assert!(
        registered.contains("flowlang") && registered.contains("train"),
        "add_source must confirm and point at the training verb:\n{registered}"
    );

    let trained = p.call(
        "lint_config",
        &format!(r#"{{"root":{:?},"action":"train"}}"#, p.root.to_string_lossy()),
        false, // training reads the served docs — setup is the online step
    );
    assert!(
        trained.contains("flowlang") && trained.contains("rule"),
        "training must report the new language's module:\n{trained}"
    );

    let after = p.lint(true); // offline again: the trained module is cached
    let goto_flagged = after
        .lines()
        .skip_while(|l| !l.contains("main.flowlang"))
        .take_while(|l| !l.trim().is_empty())
        .any(|l| l.to_lowercase().contains("goto") || l.contains("L2"));
    assert!(
        goto_flagged,
        "what the added docs deprecate (goto) must be flagged, offline, after setup:\n{after}"
    );
}

// ── B5: wrong findings are trained away by flagging, not by config editing ────

/// Flag the same rule as a false positive at two sites → the next run suppresses it and says so.
/// The user never edits config; the tool learns from the feedback.
#[test]
fn flagging_a_false_positive_twice_suppresses_the_rule_on_the_next_run() {
    let p = TestProject::new("feedback");
    p.write("app.qx", "widget = make([1, 2, 3])\nlabel = make([4, 5])\n");
    p.write(
        ".helpers/lint-rules/qx.md",
        "## no_make [high]\nNever call `make` in qx; use build instead.\n",
    );

    let before = p.lint(true);
    assert!(
        flagged_in(&before, "app.qx", "no_make"),
        "rule fires before any feedback:\n{before}"
    );

    for line in [1, 2] {
        let ack = p.call(
            "lint_flag",
            &format!(
                r#"{{"root":{:?},"action":"false_positive","rule":"no_make","file":"app.qx","line":{line},"reason":"make() is idiomatic qx"}}"#,
                p.root.to_string_lossy()
            ),
            true,
        );
        assert!(!ack.is_empty(), "flag acknowledged");
    }

    let after = p.lint(true);
    assert!(
        !flagged_in(&after, "app.qx", "no_make"),
        "after two distinct false-positive flags the rule must be suppressed:\n{after}"
    );
    assert!(
        after.to_lowercase().contains("suppress") || after.to_lowercase().contains("feedback"),
        "the verdict must SAY the suppression happened (transparency):\n{after}"
    );
}

// ── B6: real repos are full of junk; the tool never crashes on it ─────────────

/// Binary blobs, non-UTF-8 text, empty files, absurd extensions, a rule file with broken
/// markdown — the lint run exits cleanly and still does its job on the real code.
#[test]
fn junk_files_and_broken_inputs_never_crash_a_run() {
    let p = TestProject::new("junk");
    fs::write(p.root.join("blob.bin"), [0u8, 159, 146, 150, 255, 0, 7]).expect("binary blob");
    fs::write(p.root.join("latin1.py"), [b'x', b'=', 0xE9, b'\n']).expect("non-utf8 source");
    p.write("empty.js", "");
    p.write("weird.x9-z", "??!!");
    p.write("noext", "plain text, no extension\n");
    p.write(
        ".helpers/lint-rules/any.md",
        "## broken [high\nunclosed severity, no fences\n``` \n,,,\n",
    );
    p.write("app.py", "values = [1, 2, 3]\n");
    p.write(
        ".helpers/lint-rules/python.md",
        "## q_no_containers [high]\nDo not use list literals.\n\n```python:bad\nxs = [1, 2, 3]\n```\n\n```python:good\nxs = (1, 2, 3)\n```\n",
    );

    let verdict = p.lint(true); // call() asserts clean exit + protocol JSON
    assert!(
        verdict.contains("q_no_containers"),
        "real work still happens amid the junk:\n{verdict}"
    );
}

// ── B7a: a root lintPref file of bare English sentences is the project's law ──

/// No `.helpers/` directory, no headings, no fences, no tags — a `lintPref.md` dropped in the
/// project root containing nothing but plain English instructions is read and enforced. This is
/// the "hand the linter your conventions like you'd hand them to a person" contract.
#[test]
fn a_root_lintpref_of_plain_english_sentences_is_enforced() {
    let p = TestProject::new("lintpref-root");
    p.write("job.py", "def run(expr):\n    value = eval(expr)\n    return value\n");
    p.write("safe.py", "def add(a, b):\n    return a + b\n");
    p.write(
        "lintPref.md",
        "Never call `eval` anywhere in this project; parse the input explicitly.\n\n\
         Do not use `printStackTrace` for error reporting; use the project logger instead.\n",
    );

    let verdict = p.lint(true);

    let hit = flagged_in(&verdict, "job.py", "eval");
    assert!(hit, "a bare English sentence in root lintPref.md must be enforced:\n{verdict}");
    assert!(
        !flagged_in(&verdict, "safe.py", "eval"),
        "the clean file must stay clean:\n{verdict}"
    );
    // The advice shown is the user's own English, readable by a human or another agent.
    let line = line_with(&verdict, "eval").expect("finding line exists");
    assert!(
        line.contains("parse the input explicitly"),
        "the finding must carry the user's own advice verbatim: {line}"
    );
}

/// The same contract holds for a `.txt` file — the format of the file is as unformatted as it
/// gets, and the rules still apply across every language in the project.
#[test]
fn a_root_lintpref_txt_governs_every_language() {
    let p = TestProject::new("lintpref-txt");
    p.write("app.py", "import pickle\n\ndef load(b):\n    return pickle.loads(b)\n");
    p.write("web.js", "function show(x) { console.log(x); }\n");
    p.write(
        "lintPref.txt",
        "Never use pickle.loads on untrusted data anywhere in this repository.\n\n\
         Do not ship console.log calls; use the structured logger.\n",
    );

    let verdict = p.lint(true);

    assert!(
        flagged_in(&verdict, "app.py", "pickle"),
        "python violation of a txt instruction must be flagged:\n{verdict}"
    );
    assert!(
        flagged_in(&verdict, "web.js", "console"),
        "javascript violation of a txt instruction must be flagged:\n{verdict}"
    );
}

// ── B7: offline + cold caches, the project's law still holds ──────────────────

/// No network, no caches, brand-new machine shape: the tool still enforces the project's own
/// rules — graceful degradation, never a refusal to work.
#[test]
fn offline_and_cold_the_projects_law_is_still_enforced() {
    let p = TestProject::new("offline-cold");
    p.write("svc.py", "def handler(evt):\n    ids = [evt.a, evt.b]\n    return ids\n");
    p.write(
        ".helpers/lint-rules/python.md",
        "## q_no_containers [high]\nDo not use list literals anywhere in this project.\n\n```python:bad\nxs = [1, 2, 3]\n```\n\n```python:good\nxs = (1, 2, 3)\n```\n",
    );

    let verdict = p.lint(true); // offline; .test-models/.test-home are empty = cold

    let hit = flagged_in(&verdict, "svc.py", "q_no_containers");
    assert!(hit, "project law is enforced offline with cold caches:\n{verdict}");
}

// ── B-diverse: every reading stage, one real-world page SHAPE each ─────────────

/// Serve a documentation site for the invented "vexlang" where每 page is a SHAPE the crawler
/// meets in the wild. Each page exists to exercise one BIT of the reading process; the
/// end-to-end assertions in [`a_diverse_real_shaped_site_exercises_every_reading_stage`]
/// name the bit they pin.
fn serve_diverse_vexlang_site() -> String {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind localhost");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).into_owned();
            let body: &str = if req.starts_with("GET /zapcall") {
                // MDN-reference SHAPE: site chrome that is NOT inside <nav>/<footer> tags
                // (plain divs with ids, css-ish text), then an anchored prohibition section
                // with a bad/good example pair.
                "<html><body>\
                 <div id=\"menu_click_menu--vex\">VEX reference .button{align-items:center;border:1px solid}</div>\
                 <div id=\"breadcrumb_click--2\">Reference</div>\
                 <h1>zapcall()</h1>\
                 <p>The zapcall function sends a payload to the zap bus and returns its ticket. \
                 It is part of the classic bus interface and appears throughout older programs. \
                 The sections below describe its behavior and the supported alternatives in detail.</p>\
                 <h2 id=\"never_use_zapcall\">Never use zapcall!</h2>\
                 <p>Never use the zapcall statement anywhere; it is deprecated and will be removed.</p>\
                 <pre>zapcall(payload)</pre>\
                 <p>Use a structured call instead; this is the correct form:</p>\
                 <pre>safecall(payload)</pre>\
                 <div id=\"footer--link\">Site map</div>\
                 </body></html>"
            } else if req.starts_with("GET /docs/rules/all") {
                // Single-page multi-rule SHAPE (clippy's lint list): each rule in its own
                // anchored article; a rule's fix must pair within ITS anchor, never steal
                // the next rule's.
                "<html><body><h1>vexlang lints</h1>\
                 <article id=\"lint-no_flob\"><h2>no_flob</h2>\
                 <p>Never use the flob statement anywhere; it is deprecated and will be removed.</p>\
                 <pre>flob left</pre>\
                 <p>Use a structured form instead; this is the correct form:</p>\
                 <pre>flib left</pre></article>\
                 <article id=\"lint-no_wibble\"><h2>no_wibble</h2>\
                 <p>Never use the wibble statement anywhere; it is deprecated and will be removed.</p>\
                 <pre>wibble both</pre>\
                 <p>Use a structured form instead; this is the correct form:</p>\
                 <pre>wend both</pre></article>\
                 </body></html>"
            } else if req.starts_with("GET /errors/bad-literal") {
                // Error-page SHAPE: prohibition register whose bad/good examples differ ONLY
                // in punctuation — the compile must abstain (values and operators are
                // semantics), never mint a token detector that fires on normal use.
                "<html><body><h1>SyntaxError: vex literal</h1>\
                 <p>Never write a trailing separator in a vex literal; the string cannot be parsed and the program is rejected.</p>\
                 <pre>parsevex(\"[1,2,]\")</pre>\
                 <p>Use the plain form instead; this is the correct form:</p>\
                 <pre>parsevex(\"[1,2]\")</pre>\
                 </body></html>"
            } else if req.starts_with("GET /wobble") {
                // Deprecated-marker SHAPE.
                "<html><body><h1>The wobble form</h1>\
                 <p>Deprecated: never use the wobble form in new code; it is deprecated and will be removed.</p>\
                 <pre>wobble mode</pre>\
                 <p>Use the bobble form instead; this is the correct form:</p>\
                 <pre>bobble mode</pre>\
                 </body></html>"
            } else if req.starts_with("GET /flare") {
                // BLOCKLESS prohibition SHAPE (the MDN-eval warning): heading + prose,
                // zero <pre>. Learns AND fires through the PASS-36 triple-conjunction gate
                // (native/architecture.dx failure-ledger re-land addendum, 2026-07-18).
                "<html><body><h1>flare()</h1>\
                 <h2 id=\"never_use_flare\">Never use flare!</h2>\
                 <p>Never use the flare statement anywhere; it is dangerous and will be removed.</p>\
                 </body></html>"
            } else if req.starts_with("GET /getting-started") {
                // Tutorial SHAPE: neutral prose around a whole sample program — a sample
                // program is not a rule (ledger #13) and must mint nothing that fires.
                "<html><body><h1>Getting started</h1>\
                 <p>A first vexlang program prints a greeting and finishes cleanly.</p>\
                 <pre>begin\n  greet(\"world\")\n  finish\nend</pre>\
                 <p>Run it with the vex runner and read the output.</p>\
                 </body></html>"
            } else if req.starts_with("GET /intl") {
                // Multibyte SHAPE: CJK and emoji chrome around a neutral example — crawl
                // and unit forming must stay char-boundary safe.
                "<html><body><h1>国際化 🌍</h1>\
                 <p>言語の設定は端末に従います。The steady element keeps its state.</p>\
                 <pre>steady on</pre>\
                 </body></html>"
            } else {
                "<html><body><h1>vexlang documentation</h1>\
                 <div id=\"menu_click_menu--all\">All pages</div>\
                 <p>The reference for the vex language.</p>\
                 <a href=\"/zapcall.html\">zapcall</a> \
                 <a href=\"/docs/rules/all.html\">lints</a> \
                 <a href=\"/errors/bad-literal.html\">errors</a> \
                 <a href=\"/wobble.html\">wobble</a> \
                 <a href=\"/flare.html\">flare</a> \
                 <a href=\"/getting-started.html\">start</a> \
                 <a href=\"/intl.html\">intl</a>\
                 </body></html>"
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
        }
    });
    format!("http://127.0.0.1:{port}/")
}

/// The DIVERSITY contract: one invented language whose documentation exhibits, page by page,
/// every real-world shape the reading pipeline must handle — and each assertion names the
/// process BIT it pins. When this passes together with the planted-violation matrix, the
/// pipeline's stages are each proven on a diverse input, not on one lucky rule.
#[test]
fn a_diverse_real_shaped_site_exercises_every_reading_stage() {
    let url = serve_diverse_vexlang_site();
    let p = TestProject::new("diverse-shapes");
    p.write(
        "bad.vex",
        "zapcall(payload)\nflob left\nwibble both\nwobble mode\nflare now\nparsevex(\"[1,2,]\")\n",
    );
    p.write(
        "clean.vex",
        "safecall(payload)\nflib left\nwend both\nbobble mode\nparsevex(\"[1,2]\")\ngreet(\"world\")\nsteady on\n",
    );
    p.write(
        "lint-index/sources.json",
        &format!(
            r#"{{"version": 3, "sources": [{{"tool": "vexdocs", "language": "vex", "kind": "crawl", "seed": "{url}"}}]}}"#
        ),
    );

    // Train online against the localhost site (the one network step)…
    let ack = p.call("lint_config", r#"{"action":"train"}"#, false);
    assert!(
        line_with(&ack, "vex:").is_some_and(|l| l.contains("rule(s)")),
        "STAGE crawl+read+mint: the site trains a vex module:\n{ack}"
    );
    // …then everything below is the offline live path.
    let verdict = p.lint(true);

    // STAGE reference-shape extraction (chrome + anchored section): the prohibition minted
    // and fires on the exact line, citing the docs' own words.
    assert!(
        flagged_in(&verdict, "bad.vex", "zapcall") && verdict.contains("Never use the zapcall statement"),
        "MDN-reference shape must mint the zapcall rule and fire it on bad.vex:\n{verdict}"
    );
    // STAGE anchor slugs (multi-rule page): BOTH rules mint under their own anchors and fire.
    assert!(
        flagged_in(&verdict, "bad.vex", "flob"),
        "clippy shape: no_flob fires on its line:\n{verdict}"
    );
    assert!(
        flagged_in(&verdict, "bad.vex", "wibble"),
        "clippy shape: no_wibble fires on its line (its fix must not be stolen by no_flob):\n{verdict}"
    );
    // STAGE deprecated-marker shape.
    assert!(
        flagged_in(&verdict, "bad.vex", "wobble"),
        "deprecated-marker shape: the wobble rule fires:\n{verdict}"
    );
    // STAGE punctuation-only diff (error page): the compile ABSTAINS — parsevex must be
    // flagged in NEITHER file (a token detector here would fire on every normal call).
    assert!(
        !flagged_in(&verdict, "bad.vex", "parsevex") && !flagged_in(&verdict, "clean.vex", "parsevex"),
        "error-page shape: a punctuation-only diff compiles no detector:\n{verdict}"
    );
    // STAGE blockless prohibition (the MDN-eval shape): LEARNS AND FIRES (PASS 36 re-land,
    // native/architecture.dx failure-ledger addendum 2026-07-18) — minted through the triple-conjunction
    // gate (frozen-English forbidding sentence + a construct common English cannot account
    // for + the section's own anchor slug naming it), fired at the LOW tier citing the docs'
    // own words. The clean file must stay silent (asserted by the zero-FP stage below).
    assert!(
        flagged_in(&verdict, "bad.vex", "flare") && verdict.contains("use the flare statement"),
        "blockless shape must mint through the triple-conjunction gate and fire citing the docs' words:\n{verdict}"
    );
    // STAGE zero false positives across every shape at once: the clean file uses every
    // GOOD form, the error-page feature legitimately, the tutorial's own construct, and
    // the multibyte page's element — none of it may be flagged.
    assert!(
        !verdict
            .lines()
            .skip_while(|l| !l.contains("clean.vex"))
            .take_while(|l| !l.trim().is_empty())
            .any(|l| l.trim_start().starts_with('[')),
        "clean.vex must be CLEAN across all shapes:\n{verdict}"
    );
    // STAGE chrome hygiene: nothing minted from menus/footers/css-ish text ever fires, and
    // no fired advice carries markup junk.
    assert!(
        !verdict.contains("menu_click") && !verdict.contains("footer--") && !verdict.contains("align-items"),
        "site chrome must never surface in findings:\n{verdict}"
    );
}
