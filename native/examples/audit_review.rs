//! An HONEST scored audit of the reviewer: plant code with a known answer key
//! (real bugs the deterministic floor should catch, CS2420/CS3500 violations the
//! semantic layer should advise on, and clean code that must NOT be flagged), run
//! the real pipeline, and score caught / missed / false-positive against the key.
//!
//!   cargo run --release --example audit_review
//!
//! Nothing here is rigged: the floor is `reviewer::review_grounded` (the same exact
//! checkers shipped), and the CS advice is `lint_semantic::Norms` learned from THIS
//! project's own source — the same corpus `grade_semantic` uses. The point is to
//! report what it genuinely does, including what it (by design) abstains on.

use std::fs;
use std::path::{Path, PathBuf};

use helpers_native::lint_semantic::{functions, Norms, Principle};
use helpers_native::reviewer::review_grounded;

/// One planted defect and where the answer key expects it surfaced.
struct Expect {
    line: usize,
    what: &'static str,
    by: Layer,
}

#[derive(PartialEq)]
enum Layer {
    Floor,    // a deterministic checker must flag this exact line
    Semantic, // the learned CS layer must advise on the function at/after this line
    NoTool,   // honest control: no static linter can catch this without running the code
}

fn ext_lang(p: &Path) -> Option<&'static str> {
    match p.extension().and_then(|e| e.to_str())? {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "js" | "mjs" | "cjs" => Some("javascript"),
        "ts" => Some("typescript"),
        "go" => Some("go"),
        _ => None,
    }
}

fn source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(n, "target" | ".git" | "node_modules" | ".helpers" | "dist" | "build") {
                continue;
            }
            source_files(&p, out);
        } else if ext_lang(&p).is_some() {
            out.push(p);
        }
    }
}

/// Learn the project's behavioral norms — same corpus `grade_semantic` uses, so the
/// CS judgments are calibrated to real code, not a toy distribution.
fn learn_project_norms() -> Norms {
    let mut files = Vec::new();
    source_files(Path::new(".."), &mut files);
    let sources: Vec<(String, String)> = files
        .iter()
        .filter_map(|f| Some((ext_lang(f)?.to_string(), fs::read_to_string(f).ok()?)))
        .collect();
    let refs: Vec<(&str, &str)> = sources.iter().map(|(l, c)| (l.as_str(), c.as_str())).collect();
    Norms::learn(&refs)
}

fn main() {
    // ---- The planted file with a known answer key. Line numbers are 1-based here. ----
    let code = r#"// 1
fn get_config() {                       // 3: getter that returns nothing -> NamingMismatch
    let _ = load();                     //    (discards a fallible result too)
}

fn parse_port(s: &str) -> u16 {         // 7
    s.parse().unwrap()                  // 8: unwrap_used (floor) + ErrorHandling (semantic)
}

fn check(a: i32, b: i32) -> bool {      // 11
    if a == a { return true; }          // 12: eq_op (floor) + bool-ish
    a == b
}

fn flag_on(enabled: bool) -> bool {     // 16
    if enabled == true {                // 17: bool_comparison (floor)
        panic!("nope");                 // 18: panic (floor)
    }
    let name = "";
    if name == "" { }                   // 21: comparison_to_empty (floor)
    return enabled;                     // 22: needless_return (floor)
}

fn god(xs: &[i32]) -> i32 {             // 25: SingleResponsibility + Complexity (semantic)
    let mut t = 0;
    for x in xs {
        if *x > 0 {
            if *x > 10 { t += a1(*x); } else { t += a2(*x); }
        } else if *x < 0 {
            while t > 0 { t = b1(t); if t == 5 { break; } }
        } else {
            t += c1(*x); t += c2(*x); t += c3(*x);
        }
    }
    println!("{}", t);                  // 37: print_stdout (floor)
    t
}

fn average(xs: &[i32]) -> i32 {         // 41
    let mut sum = 0;
    for i in 0..=xs.len() { sum += xs[i]; } // 43: OFF-BY-ONE (<=) -> NoTool control
    sum / xs.len() as i32
}

// Clean code below — MUST NOT be flagged (false-positive check):
fn add(a: i32, b: i32) -> i32 { a + b } // 48: clean
fn is_even(n: i32) -> bool { n % 2 == 0 } // 49: clean getter-ish, returns a value
"#;

    // Line numbers below are the TRUE 1-based positions in `code` (its first line is `// 1`).
    let key = [
        Expect { line: 2, what: "get_config: getter returns nothing", by: Layer::Semantic },
        Expect { line: 7, what: "unwrap_used", by: Layer::Floor },
        Expect { line: 11, what: "eq_op (a == a)", by: Layer::Floor },
        Expect { line: 16, what: "bool_comparison (== true)", by: Layer::Floor },
        Expect { line: 17, what: "panic!", by: Layer::Floor },
        Expect { line: 20, what: "comparison_to_empty", by: Layer::Floor },
        Expect { line: 21, what: "needless_return", by: Layer::Floor },
        Expect { line: 24, what: "god: single-responsibility / complexity", by: Layer::Semantic },
        Expect { line: 35, what: "print_stdout", by: Layer::Floor },
        Expect { line: 41, what: "off-by-one (<= len)", by: Layer::NoTool },
    ];

    let lines: Vec<&str> = code.lines().collect();

    // ---- Run the real pipeline ----
    let floor = review_grounded("rust", &lines); // exact checkers, 0 FP by contract
    let norms = learn_project_norms();
    let mut semantic: Vec<(usize, Vec<Principle>, String)> = Vec::new();
    for m in functions("rust", code) {
        let v = norms.judge(&m);
        if !v.is_empty() {
            semantic.push((m.line, v, m.name));
        }
    }

    let floor_lines: std::collections::HashSet<usize> = floor.iter().map(|f| f.line).collect();
    let clean_lines = [47usize, 48];

    println!("=== FLOOR (deterministic, contract: 0 false positives) ===");
    let mut floor_hits: Vec<_> = floor.iter().collect();
    floor_hits.sort_by_key(|f| f.line);
    for f in &floor_hits {
        println!("  L{:<3} {:<22} [{}]", f.line, f.rule_id, f.severity);
    }
    println!("\n=== SEMANTIC (CS2420/CS3500, learned from this project) ===");
    println!(
        "  norms: single-responsibility > {} concerns, complexity > {} (from {} fns)",
        norms.responsibility_p90, norms.complexity_p90, norms.sampled
    );
    semantic.sort_by_key(|s| s.0);
    for (line, v, name) in &semantic {
        let tags: Vec<&str> = v
            .iter()
            .map(|p| match p {
                Principle::SingleResponsibility => "single-responsibility",
                Principle::Complexity => "complexity",
                Principle::ErrorHandling => "error-handling",
                Principle::NamingMismatch => "naming-vs-behavior",
            })
            .collect();
        println!("  L{:<3} fn {:<12} -> {}", line, name, tags.join(", "));
    }

    // ---- Score against the answer key ----
    let sem_lines: std::collections::HashSet<usize> = semantic.iter().map(|s| s.0).collect();
    println!("\n=== SCORECARD vs answer key ===");
    let (mut caught, mut missed, mut abstained) = (0, 0, 0);
    for e in &key {
        let hit = match e.by {
            Layer::Floor => floor_lines.contains(&e.line),
            // semantic flags by function-start line; the planted bug's line is the fn header
            Layer::Semantic => sem_lines.contains(&e.line),
            Layer::NoTool => false,
        };
        let mark = match (&e.by, hit) {
            (Layer::NoTool, _) => {
                abstained += 1;
                "ABSTAIN (no static tool can; honest control)"
            }
            (_, true) => {
                caught += 1;
                "CAUGHT"
            }
            (_, false) => {
                missed += 1;
                "MISSED"
            }
        };
        println!("  L{:<3} {:<40} {}", e.line, e.what, mark);
    }

    // ---- False positives: anything flagged on a known-clean line ----
    let fp: Vec<usize> = floor_lines
        .iter()
        .chain(sem_lines.iter())
        .filter(|l| clean_lines.contains(l))
        .copied()
        .collect();

    let real = key.iter().filter(|e| e.by != Layer::NoTool).count();
    println!(
        "\nReal defects: {caught}/{real} caught, {missed} missed.  Off-by-one control: {abstained} abstained (expected).",
    );
    println!("False positives on clean code: {} {}", fp.len(), if fp.is_empty() { "(none ✓)" } else { "(!!)" });
    if !fp.is_empty() {
        println!("  flagged clean lines: {fp:?}");
    }
}
