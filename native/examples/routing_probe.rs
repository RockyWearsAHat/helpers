//! Throwaway probe (untracked): dump the understanding trace + firing for candidate JS constructs,
//! to diagnose the eval→shell_injection misroute and validate the routing fix.
use helpers_native::lint_char;
use helpers_native::lint_english;
use helpers_native::lint_trace::{explain, run_plan, understand, Bridge, Plan};

fn dump(prose: &str, lang: &str, bad: &str, good: &str) {
    let (br, en) = (lint_char::brain().unwrap(), lint_english::brain().unwrap());
    let bridge = Bridge::new(br.meanings(), en);
    println!("\n=== {prose}");
    if let Some(ex) = explain(prose, false) {
        println!("  sentence: {}", ex.sentence);
        println!("  prohibition: {}", ex.prohibition);
        println!("  operators: {:?}  inner_neg: {:?}", ex.operators, ex.inner_negations);
        for c in &ex.concepts {
            println!(
                "    concept {:12} nearest={:18} dist={:5} runner={:5} ratio={:.3} central={:5} aligned={:?}",
                c.word, c.nearest, c.distance, c.runner_up, c.ratio, c.centrality, c.aligned
            );
        }
        match &ex.plan {
            Some(p) => println!("  PLAN: {}", p.describe()),
            None => println!("  ABSTAIN: {:?}", ex.abstain),
        }
    }
    if let Some(p) = understand(prose) {
        let fb = run_plan(&p, lang, bad);
        let fg = run_plan(&p, lang, good);
        println!("  understand()={} fires bad={:?} good={:?}", p.describe(), fb, fg);
    } else {
        println!("  understand()=None");
    }
    match bridge.understand_verified(prose, lang, bad, good) {
        Some(p) => println!("  understand_verified()={}", p.describe()),
        None => println!("  understand_verified()=None"),
    }
}

fn main() {
    dump(
        "Never use the `eval` function to execute a string of code.",
        "javascript",
        "eval(userInput);",
        "JSON.parse(userInput);",
    );
    dump(
        "Never use the eval function to execute a string of code.",
        "javascript",
        "eval(userInput);",
        "JSON.parse(userInput);",
    );
    dump(
        "Never use the var keyword to declare a variable. Use let or const instead.",
        "javascript",
        "var x = 1;",
        "let x = 1;",
    );
    dump(
        "Never use the with statement, it is deprecated and confusing.",
        "javascript",
        "with (obj) { x = 1; }",
        "const x = obj.x;",
    );
    dump(
        "Never use the loose equality operator `==` to compare values; use `===` instead.",
        "javascript",
        "if (a == b) { doThing(); }",
        "if (a === b) { doThing(); }",
    );
    dump(
        "Never use `document.write` to insert content into the page.",
        "javascript",
        "document.write('<p>hi</p>');",
        "el.append(node);",
    );
    dump(
        "Never use the `with` statement, it is deprecated and confusing.",
        "javascript",
        "with (obj) { x = 1; }",
        "const x = obj.x;",
    );
    // ── Genuine CS-defect rules (Rust) — must KEEP their CS-primitive plan after the routing fix ──
    dump(
        "Never unwrap or expect the result of a fallible call.",
        "rust",
        "fn f() { let v: i32 = \"1\".parse().unwrap(); }",
        "fn f() -> Result<i32, ()> { \"1\".parse().map_err(|_| ()) }",
    );
    dump(
        "Never bury an unexplained magic number literal in the code.",
        "rust",
        "fn f() -> i32 { let d = 86400; d }",
        "const DAY: i32 = 86400;\nfn f() -> i32 { DAY }",
    );
    dump(
        "Never hardcode a secret in the source.",
        "rust",
        "fn f() { let api_key = \"sk-9f8a7b6c5d4e3f2a\"; }",
        "fn f() { let api_key = std::env::var(\"API_KEY\").unwrap_or_default(); }",
    );
    dump(
        "Never interpolate untrusted input into a shell command string.",
        "rust",
        "fn f(u: &str) { std::process::Command::new(\"sh\").arg(format!(\"echo {u}\")); }",
        "fn f(u: &str) { std::process::Command::new(\"echo\").arg(u); }",
    );
    dump(
        "Never ignore, discard, or swallow an error.",
        "rust",
        "fn f() { let d = std::fs::read_to_string(\"x\"); let _ = d; }",
        "fn f() -> std::io::Result<()> { std::fs::read_to_string(\"x\")?; Ok(()) }",
    );
}
