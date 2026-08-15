//! Throwaway probe (untracked): graduate the JS construct rules through the REAL self-generated test
//! loop (`lint_selftest::prove`) against the frozen brains + real linter. Each rule's firing Plan is
//! shaped by the trace bridge from its backticked prohibition prose (`understand`), so the plan is
//! DATA read from the prose, not a coded token. Reports Proven / Unproven per rule, with the
//! reconciliation distances behind each decision.
use helpers_native::lint_char;
use helpers_native::lint_corroborate::corroborates;
use helpers_native::lint_english;
use helpers_native::lint_selftest::{prove, KnownRule, LearnedRule, Verdict};
use helpers_native::lint_trace::{run_plan, understand, Plan};

/// One JS rule under test: its prohibition prose (→ firing plan), the found-violation advice the
/// linter reports, the AI's understanding, a genuine sibling foil, and varied violating samples.
struct Case {
    name: &'static str,
    prose: &'static str,
    advice: &'static str,
    understanding: &'static str,
    foil: &'static str,
    samples: Vec<&'static str>,
    clean: Vec<&'static str>,
}

fn main() {
    let (Some(br), Some(en)) = (lint_char::brain(), lint_english::brain()) else {
        eprintln!("no frozen brains on disk");
        return;
    };
    let m = br.meanings();

    let cases = vec![
        Case {
            name: "eval",
            prose: "Never use the `eval` function to execute a string of code.",
            advice: "calling eval executes an arbitrary string of code as a security risk",
            understanding: "using eval runs a string of code and is a security risk",
            foil: "using var declares a variable whose scope leaks out of its block",
            samples: vec![
                "eval(userInput);",
                "function run(s) { return eval(s); }",
                "const r = eval('1 + 1');",
                "if (ok) { eval(payload); }",
                "for (const s of list) eval(s);",
                "window.onload = () => eval(code);",
                "try { eval(expr); } catch (e) {}",
                "const f = () => eval(src);",
                "obj.run = function () { eval(this.code); };",
                "switch (k) { case 1: eval(a); break; }",
                "class C { go() { return eval(this.s); } }",
                "[1].forEach(() => eval(dangerous));",
            ],
            clean: vec!["JSON.parse(input);", "const r = 1 + 1;", "// eval is dangerous", "const s = 'eval(x)';"],
        },
        Case {
            name: "var",
            prose: "Never use the `var` keyword to declare a variable. Use let or const instead.",
            advice: "using var declares a variable whose scope leaks out of its enclosing block",
            understanding: "the var keyword declares a variable that leaks its block scope",
            foil: "calling eval executes an arbitrary string of code as a security risk",
            samples: vec![
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
            ],
            clean: vec!["let x = 1;", "const y = 2;", "// var is discouraged", "const s = 'var x = 1';"],
        },
        Case {
            name: "with",
            prose: "Never use the `with` statement, it makes variable scope ambiguous and is deprecated.",
            advice: "the with statement makes variable scope ambiguous and is deprecated",
            understanding: "using with creates an ambiguous variable scope and is deprecated",
            foil: "the var keyword declares a variable that leaks its block scope",
            samples: vec![
                "with (obj) { x = 1; }",
                "function f(o) { with (o) { return a; } }",
                "with (Math) { y = cos(0); }",
                "if (t) { with (data) { z = v; } }",
                "with (a.b) { c = 1; }",
                "with (window) { alert(msg); }",
                "for (;;) { with (o) { break; } }",
                "with (config) { setup(); }",
                "const g = () => { with (o) { p(); } };",
                "with (state) { render(); }",
                "try { with (o) { risky(); } } catch (e) {}",
                "class C { m(o) { with (o) { return q; } } }",
            ],
            clean: vec!["const x = obj.x;", "const y = Math.cos(0);", "// with is deprecated", "const s = 'with (o) {}';"],
        },
        Case {
            name: "==",
            prose: "Never use the loose equality operator `==` to compare values; use `===` instead.",
            advice: "the equality operator compares values with type coercion and is error prone",
            understanding: "the equality operator coerces types when comparing values and is error prone",
            foil: "using with creates an ambiguous variable scope and is deprecated",
            samples: vec![
                "if (a == b) { doThing(); }",
                "return x == null;",
                "while (i == 0) { i++; }",
                "const same = p == q;",
                "if (t == 'yes') go();",
                "for (; n == m; ) {}",
                "flag = a == b ? 1 : 2;",
                "switch (true) { case a == b: break; }",
                "const f = () => a == b;",
                "if (x == 1 && y == 2) {}",
                "obj.eq = function () { return this.v == other; };",
                "arr.filter((e) => e == target);",
            ],
            clean: vec!["if (a === b) {}", "return x === null;", "// == is loose", "const s = 'a == b';"],
        },
        Case {
            name: "document.write",
            prose: "Never use `document.write` to insert content into the page.",
            advice: "calling document.write injects markup into the document and risks script injection",
            understanding: "using document.write writes markup into the page and risks injection",
            foil: "using == compares values with type coercion which is error prone",
            samples: vec![
                "document.write('<p>hi</p>');",
                "function show(x) { document.write(x); }",
                "if (ok) document.write(html);",
                "for (const h of list) document.write(h);",
                "document.write('<b>' + name + '</b>');",
                "window.onload = () => document.write(msg);",
                "try { document.write(data); } catch (e) {}",
                "const f = () => document.write(s);",
                "obj.render = function () { document.write(this.html); };",
                "switch (k) { case 1: document.write(a); break; }",
                "class C { go() { document.write(this.s); } }",
                "[1].forEach(() => document.write(row));",
            ],
            clean: vec!["el.append(node);", "container.innerHTML = safe;", "// document.write is bad", "const s = 'document.write(x)';"],
        },
    ];

    // Build the linter's book of KNOWN rules: every case's firing plan (shaped by the bridge from its
    // backticked prose) + the found-violation advice it reports. All rules are in the book so firing is
    // realistic (any rule may fire on any sample).
    let mut book: Vec<KnownRule> = Vec::new();
    for c in &cases {
        match understand(c.prose) {
            Some(plan @ Plan::UsesConstruct { .. }) => book.push(KnownRule::new(plan, c.advice)),
            other => {
                println!("[{}] SKIP — prose did not shape uses_construct: {:?}", c.name, other.map(|p| p.describe()));
            }
        }
    }

    println!("== FIRING CHECK (real linter over self-generated samples) ==");
    for c in &cases {
        let Some(plan) = understand(c.prose) else { continue };
        let fired: usize = c.samples.iter().filter(|s| !run_plan(&plan, "javascript", s).is_empty()).count();
        let clean_flagged: usize = c.clean.iter().filter(|s| !run_plan(&plan, "javascript", s).is_empty()).count();
        println!(
            "  {:15} plan={:28} samples flagged {}/{}  clean wrongly-flagged {}/{}",
            c.name, plan.describe(), fired, c.samples.len(), clean_flagged, c.clean.len()
        );
    }

    println!("\n== RECONCILIATION (understanding vs found-advice, over sibling foil) ==");
    for c in &cases {
        let d_true = corroborates(m, en, c.understanding, c.advice, c.foil);
        println!("  {:15} corroborates(understanding, advice | foil) = {:?}", c.name, d_true);
    }

    println!("\n== GRADUATION (the full self-generated test loop) ==");
    for c in &cases {
        let rule = LearnedRule::new(c.understanding, c.foil, "javascript");
        let verdict = prove(m, en, &rule, &book, &c.samples);
        let mark = if matches!(verdict, Verdict::Proven) { "PROVEN " } else { "unproven" };
        println!("  {:15} {} -> {:?}", c.name, mark, verdict);
    }
}
