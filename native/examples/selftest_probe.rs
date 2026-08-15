//! Throwaway probe (untracked): the self-generated test loop on the CONCRETE `var` rule, driving the
//! real `lint_selftest` module end to end against the frozen brains + real linter. Reports the genuine
//! rule graduating, the faked-understanding control being rejected, and the eval expectation phase-out.
use helpers_native::lint_char::{self};
use helpers_native::lint_english::{self};
use helpers_native::lint_selftest::{prove, KnownRule, LearnedRule, Verdict};
use helpers_native::lint_trace::{run_plan, understand, Plan};

fn main() {
    let (Some(br), Some(en)) = (lint_char::brain(), lint_english::brain()) else {
        eprintln!("no frozen brains on disk");
        return;
    };
    let m = br.meanings();

    // LEARNED rule: prohibition prose → firing Plan (the trace bridge shapes uses_construct(var)).
    let learned_prose = "Never use the var keyword to declare a variable. Use let or const instead.";
    let var_plan = understand(learned_prose).expect("understand should shape a plan for the var prohibition");
    println!("learned '{learned_prose}'\n  -> {}", var_plan.describe());

    // The linter's KNOWN rules and the English advice each reports when it fires (found-violation English).
    let book = vec![
        KnownRule::new(var_plan.clone(), "using var declares a variable whose scope leaks out of its enclosing block"),
        // a sibling security rule whose advice is a genuine competing meaning
        KnownRule::new(
            Plan::UsesConstruct { construct: "eval".into() },
            "calling eval executes an arbitrary string of code as a security risk",
        ),
    ];

    // Self-generated VARIED violations (block/loop/method/arrow/try/switch/class; later ones tangential).
    let samples = [
        "var x = 1;",
        "function f() { var y = 2; return y; }",
        "for (var i = 0; i < 3; i++) {}",
        "var a = 1, b = 2;",
        "if (t) { var z = 9; }",
        "var name = getName();",
        "window.onload = function () { var q = 0; };",
        "var obj = { k: 1 };",
        "try { var r = risky(); } catch (e) {}",
        "switch (n) { case 1: var s = 'x'; break; }",
        "var fn = () => { var inner = 1; return inner; };",
        "class C { m() { var local = this.x; return local; } }",
    ];
    for (i, code) in samples.iter().enumerate() {
        let hits = run_plan(&var_plan, "javascript", code);
        println!("  rep {:>2} flagged={:5} lines={:?}  {}", i + 1, !hits.is_empty(), hits, code);
    }
    for clean in ["let x = 1;", "const y = 2;", "// var is discouraged", "const s = 'var x = 1';"] {
        let hits = run_plan(&var_plan, "javascript", clean);
        println!("  clean  flagged={:5} lines={:?}  {}", !hits.is_empty(), hits, clean);
    }

    // GENUINE understanding of the var rule → should GRADUATE.
    let genuine = LearnedRule::new(
        "using var declares a variable whose scope leaks out of its enclosing block",
        "calling eval executes an arbitrary string of code as a security risk",
        "javascript",
    );
    println!("\nGENUINE  -> {:?}", prove(m, en, &genuine, &book, &samples));

    // FAKED understanding (var is about indentation, not scope) → should be REJECTED.
    let faked = LearnedRule::new(
        "var should be indented with two spaces for readable code formatting",
        "using var declares a variable whose scope leaks out of its enclosing block",
        "javascript",
    );
    let faked_verdict = prove(m, en, &faked, &book, &samples);
    println!("CONTROL  -> {faked_verdict:?}");
    assert!(matches!(faked_verdict, Verdict::Unproven(_)), "the faked understanding must NOT graduate");

    // HONEST NOTE: the eval prohibition understands to shell_injection (a Rust format! primitive),
    // which does NOT fire on JS eval(...) — a measured expectation phase-out (see native/architecture.dx).
    let eval_prose = "Never use the eval function to execute a string of code.";
    if let Some(p) = understand(eval_prose) {
        let fires = !run_plan(&p, "javascript", "eval(userInput);").is_empty();
        println!("\nNOTE eval: '{eval_prose}' -> {} ; fires on eval(userInput)? {fires}", p.describe());
    }
}
