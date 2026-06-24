//! Golden-file integration tests: grade the checked-in fixture projects through
//! the public library API and assert the rendered GRADE.md and `--json` payloads
//! match byte-for-byte. The golden files were captured from output verified
//! identical to the original Node implementation.

use cs_grade::{evaluate, project_label, report};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Grade `name` for `course` and return (markdown_file_body, json_stdout) in the
/// exact byte form the binary writes/prints (trailing newline included).
fn render(name: &str, course: &str) -> (String, String) {
    let root = fixture(name);
    let (grade, src, test) = evaluate(&root, course);
    let md = report::markdown(&grade, &project_label(&root), src, test);
    (format!("{md}\n"), format!("{}\n", report::json(&grade)))
}

#[test]
fn sample_project_matches_golden() {
    let (md, json) = render("sample", "auto");
    assert_eq!(md, include_str!("golden/sample.GRADE.md"));
    assert_eq!(json, include_str!("golden/sample.json"));
}

#[test]
fn aplus_project_matches_golden() {
    let (md, json) = render("aplus", "cs2420");
    assert_eq!(md, include_str!("golden/aplus.GRADE.md"));
    assert_eq!(json, include_str!("golden/aplus.json"));
    // The A+ fixture must actually reach A+ (exit-0 path).
    let (grade, ..) = evaluate(&fixture("aplus"), "cs2420");
    assert_eq!(grade.grade, "A+");
    assert!(grade.gaps.is_empty());
}

#[test]
fn auto_detects_cs3500_for_oo_project() {
    let (grade, ..) = evaluate(&fixture("sample"), "auto");
    assert_eq!(grade.course, "cs3500");
}

/// A non-Java project must be detected as its real language and graded on its
/// own merits — not handed an automatic F for containing zero `.java` files
/// (the bug this multi-language support fixes).
#[test]
fn rust_project_is_detected_and_graded_on_its_merits() {
    let (grade, src, test) = evaluate(&fixture("rust"), "full");
    assert_eq!(grade.lang, "Rust");
    assert!(src >= 2, "expected the Rust source files, found {src}");
    assert!(test >= 1, "expected the Rust test file, found {test}");
    // Well-documented, tested, Cargo-built crate should land a real grade, not F.
    assert_ne!(grade.grade, "F", "Rust project wrongly graded F ({})", grade.pct);
    assert!(grade.pct > 70.0, "expected a solid grade, got {}", grade.pct);
    // The report header and evidence must speak Rust, not Java.
    let md = report::markdown(&grade, &project_label(&fixture("rust")), src, test);
    assert!(md.contains("· Rust ·"), "header should name the language");
    assert!(md.contains("cargo test"), "tests evidence should name the runner");
    assert!(!md.contains("Javadoc"), "Java wording must not leak into a Rust report");
}
