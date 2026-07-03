//! The AI linter's user-facing contract, tested through its REAL public interface: the built
//! binary, invoked exactly as a user or agent invokes it (`helpers-native call lint` with a JSON
//! request on stdin). Each test is one behavior the tool promises:
//!
//!   * a language it has never seen is lintable with nothing but a plain-English rule file;
//!   * an instruction with no language named is law across every language in the project;
//!   * teaching it a new real language is a DATA edit (a docs URL), never a code change;
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

// ── B2: one instruction, every language ───────────────────────────────────────

/// An `any.md` rule that names no language is the project's law for EVERY language present —
/// grammar languages (python, javascript) and grammarless made-up ones alike.
#[test]
fn an_instruction_with_no_language_governs_every_language_in_the_project() {
    let p = TestProject::new("any-lang");
    p.write("app.py", "def main():\n    scores = [90, 85, 77]\n    return scores\n");
    p.write("web.js", "const nums = [1, 2, 3];\nconsole.log(nums);\n");
    p.write("data.qlang", "let sizes = [4, 5, 6]\n");
    p.write(
        ".helpers/lint-rules/any.md",
        "## no_arrays [high]\nDo not use array or list literals anywhere in this project. Use keyed structures instead.\n\n```bad\nxs = [1, 2, 3]\n```\n\n```good\nxs = {\"a\": 1, \"b\": 2, \"c\": 3}\n```\n",
    );

    let verdict = p.lint(true);

    for file in ["app.py", "web.js", "data.qlang"] {
        let section_hit = flagged_in(&verdict, file, "no_arrays");
        assert!(
            section_hit,
            "any.md no_arrays must fire on {file} — one instruction governs every language:\n{verdict}"
        );
    }
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

/// Registering a docs URL for a brand-new language — one JSON entry, pure data — makes the
/// linter learn that language from its documentation and enforce what the docs deprecate.
/// No code change, no rule files, no config beyond the URL.
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

    let verdict = p.lint(false); // online: it must actually read the served docs

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

// ── B5: wrong findings are trained away by flagging, not by config editing ────

/// Flag the same rule as a false positive at two sites → the next run suppresses it and says so.
/// The user never edits config; the tool learns from the feedback.
#[test]
fn flagging_a_false_positive_twice_suppresses_the_rule_on_the_next_run() {
    let p = TestProject::new("feedback");
    p.write("app.qx", "widget = make([1, 2, 3])\nlabel = make([4, 5])\n");
    p.write(
        ".helpers/lint-rules/qx.md",
        "## no_brackets [high]\nDo not use bracket literals in qx.\n\n```qx:bad\nxs = [1, 2, 3]\n```\n\n```qx:good\nxs = tuple(1, 2, 3)\n```\n",
    );

    let before = p.lint(true);
    assert!(before.contains("no_brackets"), "rule fires before any feedback:\n{before}");

    for line in [1, 2] {
        let ack = p.call(
            "lint_flag",
            &format!(
                r#"{{"root":{:?},"action":"false_positive","rule":"no_brackets","file":"app.qx","line":{line},"reason":"make() takes a spec list; this is idiomatic qx"}}"#,
                p.root.to_string_lossy()
            ),
            true,
        );
        assert!(!ack.is_empty(), "flag acknowledged");
    }

    let after = p.lint(true);
    assert!(
        !flagged_in(&after, "app.qx", "no_brackets"),
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
        "## no_arrays [high]\nDo not use list literals.\n\n```python:bad\nxs = [1, 2, 3]\n```\n\n```python:good\nxs = (1, 2, 3)\n```\n",
    );

    let verdict = p.lint(true); // call() asserts clean exit + protocol JSON
    assert!(
        verdict.contains("no_arrays"),
        "real work still happens amid the junk:\n{verdict}"
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
        "## no_arrays [high]\nDo not use list literals anywhere in this project.\n\n```python:bad\nxs = [1, 2, 3]\n```\n\n```python:good\nxs = (1, 2, 3)\n```\n",
    );

    let verdict = p.lint(true); // offline; .test-models/.test-home are empty = cold

    let hit = flagged_in(&verdict, "svc.py", "no_arrays");
    assert!(hit, "project law is enforced offline with cold caches:\n{verdict}");
}
