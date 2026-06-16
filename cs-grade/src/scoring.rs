//! The rubric: turn [`Signals`] into per-category scores (each 0..1 with
//! evidence and concrete fixes), combine them by course-specific weights, and
//! derive the percentage, letter grade, and prioritised "Path to A+" gaps.
//!
//! Each `category_*` function is a direct port of its JavaScript counterpart,
//! including the exact wording and `toFixed` formatting of evidence strings.

use crate::fmt::{js_round, to_fixed};
use crate::signals::Signals;

/// One category's result before weighting.
pub struct Category {
    pub score: f64, // 0..1
    pub evidence: String,
    pub fixes: Vec<String>,
}

/// A weighted, evaluated category.
pub struct Scored {
    pub name: &'static str,
    pub weight: i64,
    pub earned: f64,
    pub score: f64,
    pub evidence: String,
    pub fixes: Vec<String>,
}

/// A category that is below A+ level, with the points it could recover.
pub struct Gap {
    pub scored: Scored,
    pub recoverable: f64,
}

/// The full graded result.
pub struct Grade {
    pub course: String,
    pub pct: f64,
    pub grade: &'static str,
    pub total: f64,
    pub results: Vec<Scored>,
    pub gaps: Vec<Gap>,
}

/// Clamp to the unit interval. Inputs here are always finite, so this matches
/// the original `Math.max(0, Math.min(1, x))`.
fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// Push `fix` only when `cond` holds (mirrors `cond && "..."` + `.filter(Boolean)`).
fn push_if(fixes: &mut Vec<String>, cond: bool, fix: &str) {
    if cond {
        fixes.push(fix.to_string());
    }
}

fn category_tests(s: &Signals) -> Category {
    let assertion_target = (8.0_f64).max(s.src_files as f64 * 3.0);
    let score = clamp01(
        0.3 * (s.junit_usage as i32 as f64)
            + 0.4 * s.test_ratio
            + 0.3 * clamp01(s.assertion_count as f64 / assertion_target),
    );
    let mut fixes = Vec::new();
    push_if(
        &mut fixes,
        !s.junit_usage,
        "Add JUnit tests (`@Test`, assertions) — no test framework usage detected.",
    );
    push_if(
        &mut fixes,
        s.test_ratio < 0.5,
        "Raise test coverage: aim for a test class per non-trivial source class.",
    );
    push_if(
        &mut fixes,
        (s.assertion_count as f64) < s.src_files as f64 * 3.0,
        "Add more assertions per test, including edge cases and failure paths.",
    );
    Category {
        score,
        evidence: format!(
            "{} test file(s) for {} source file(s) (ratio {}), {} assertion(s), JUnit {}.",
            s.test_files,
            s.src_files,
            to_fixed(s.test_ratio, 2),
            s.assertion_count,
            if s.junit_usage {
                "detected"
            } else {
                "NOT detected"
            }
        ),
        fixes,
    }
}

fn category_docs(s: &Signals) -> Category {
    let readme_score = clamp01(s.readme_bytes as f64 / 1200.0);
    let score =
        clamp01(0.55 * s.javadoc_ratio + 0.3 * readme_score + 0.15 * clamp01(s.design_docs as f64));
    let mut fixes = Vec::new();
    push_if(
        &mut fixes,
        s.javadoc_ratio < 0.9,
        "Add Javadoc to every public class, interface, and method (purpose, @param, @return, @throws).",
    );
    push_if(
        &mut fixes,
        s.readme_bytes < 1200,
        "Expand the README: overview, how to build/run, and a design overview.",
    );
    push_if(
        &mut fixes,
        s.design_docs == 0,
        "Add a design/analysis document describing key decisions.",
    );
    Category {
        score,
        evidence: format!(
            "Javadoc coverage ~{}% ({} blocks / {} public decls), README {} bytes, {} design/analysis doc(s).",
            to_fixed(s.javadoc_ratio * 100.0, 0),
            s.javadoc_blocks,
            s.public_decls,
            s.readme_bytes,
            s.design_docs
        ),
        fixes,
    }
}

fn category_style(s: &Signals) -> Category {
    let cleanliness = 1.0
        - clamp01(s.god_classes.len() as f64 * 0.25)
        - clamp01(s.long_method_hits as f64 * 0.1)
        - clamp01(s.debug_prints as f64 * 0.05)
        - clamp01(s.todo_markers as f64 * 0.05)
        - clamp01(s.commented_code as f64 * 0.05);
    let score = clamp01(cleanliness);

    let god_suffix = if s.god_classes.is_empty() {
        String::new()
    } else {
        format!(" ({})", join_first(&s.god_classes, 3))
    };
    let mut fixes = Vec::new();
    if !s.god_classes.is_empty() {
        fixes.push(format!(
            "Split god classes (>400 lines): {}.",
            join_first(&s.god_classes, 5)
        ));
    }
    push_if(
        &mut fixes,
        s.long_method_hits > 0,
        "Extract long methods into small, single-responsibility helpers.",
    );
    push_if(
        &mut fixes,
        s.debug_prints > 0,
        "Remove debug prints / printStackTrace; use proper error handling or logging.",
    );
    push_if(
        &mut fixes,
        s.todo_markers > 0,
        "Resolve or remove TODO/FIXME/HACK markers before submission.",
    );
    push_if(
        &mut fixes,
        s.commented_code > 0,
        "Delete commented-out code — version control is the history.",
    );
    Category {
        score,
        evidence: format!(
            "{} file(s) >400 lines{}, {} very-long method body(ies), {} debug print(s), {} TODO/FIXME, {} commented-out code line(s).",
            s.god_classes.len(),
            god_suffix,
            s.long_method_hits,
            s.debug_prints,
            s.todo_markers,
            s.commented_code
        ),
        fixes,
    }
}

fn category_build(s: &Signals) -> Category {
    let build_term = if !s.build_files.is_empty() {
        1.0
    } else if s.uses_src_layout {
        0.6
    } else {
        0.3
    };
    let package_term = if s.uses_packages > 0 { 1.0 } else { 0.4 };
    let score = clamp01(0.5 * build_term + 0.5 * package_term);
    let mut fixes = Vec::new();
    push_if(
        &mut fixes,
        s.build_files.is_empty(),
        "Add a build file (pom.xml / build.gradle / Makefile) so the project builds reproducibly.",
    );
    push_if(
        &mut fixes,
        s.uses_packages == 0,
        "Organize classes into Java packages rather than the default package.",
    );
    push_if(
        &mut fixes,
        !s.uses_src_layout,
        "Adopt a standard src/ (and src/test) source layout.",
    );
    Category {
        score,
        evidence: format!(
            "Build file: {}; {} package declaration(s); src/ layout {}.",
            if s.build_files.is_empty() {
                "none".to_string()
            } else {
                s.build_files.join(", ")
            },
            s.uses_packages,
            if s.uses_src_layout { "yes" } else { "no" }
        ),
        fixes,
    }
}

fn category_design_ood(s: &Signals) -> Category {
    let interface_use =
        clamp01(s.interface_count as f64 / (2.0_f64).max(s.class_count as f64 * 0.3));
    let score = clamp01(
        0.4 * (s.mvc_score as f64 / 3.0)
            + 0.35 * interface_use
            + 0.25 * clamp01(s.pattern_hits.len() as f64 / 2.0),
    );
    let mut fixes = Vec::new();
    if s.mvc_score < 3 {
        let mut missing = Vec::new();
        if !s.has_model {
            missing.push("model");
        }
        if !s.has_view {
            missing.push("view");
        }
        if !s.has_controller {
            missing.push("controller");
        }
        fixes.push(format!(
            "Establish clear MVC separation (missing: {}).",
            missing.join(", ")
        ));
    }
    push_if(
        &mut fixes,
        interface_use < 0.8,
        "Program to interfaces: expose behavior through interfaces, keep concrete classes behind them.",
    );
    push_if(
        &mut fixes,
        s.pattern_hits.len() < 2,
        "Apply appropriate design patterns (Strategy/Command/Factory/Builder/Observer) where they reduce coupling.",
    );
    Category {
        score,
        evidence: format!(
            "MVC: model={}, view={}, controller={}; {} interface(s) / {} class(es); patterns seen: {}.",
            s.has_model,
            s.has_view,
            s.has_controller,
            s.interface_count,
            s.class_count,
            if s.pattern_hits.is_empty() {
                "none".to_string()
            } else {
                s.pattern_hits.join(", ")
            }
        ),
        fixes,
    }
}

fn category_abstraction(s: &Signals) -> Category {
    let interface_term =
        clamp01(s.interface_count as f64 / (1.0_f64).max(s.class_count as f64 * 0.25));
    let score = clamp01(
        0.5 * interface_term
            + 0.25 * if s.abstract_count > 0 { 1.0 } else { 0.5 }
            + 0.25 * if s.debug_prints == 0 { 1.0 } else { 0.5 },
    );
    let mut fixes = Vec::new();
    let ceil_quarter = (s.class_count as f64 * 0.25).ceil();
    push_if(
        &mut fixes,
        (s.interface_count as f64) < ceil_quarter,
        "Increase abstraction: define interfaces for major roles; avoid depending on concrete types.",
    );
    fixes.push(
        "Ensure no field is public; expose state through methods and keep representation private."
            .to_string(),
    );
    Category {
        score,
        evidence: format!(
            "{} interface(s), {} abstract class(es); implementation details {}.",
            s.interface_count,
            s.abstract_count,
            if s.debug_prints > 0 {
                "leak via prints"
            } else {
                "appear encapsulated"
            }
        ),
        fixes,
    }
}

fn category_data_structures(s: &Signals) -> Category {
    let score = clamp01(
        0.5 * if s.uses_good_structures { 1.0 } else { 0.3 }
            + 0.5 * clamp01(s.big_o_mentions as f64 / 4.0),
    );
    let mut fixes = Vec::new();
    push_if(
        &mut fixes,
        !s.uses_good_structures,
        "Use appropriate data structures (HashMap/TreeMap/PriorityQueue/…) for each access pattern.",
    );
    push_if(
        &mut fixes,
        s.big_o_mentions < 4,
        "Document the asymptotic complexity of key operations and include a timing/analysis writeup.",
    );
    Category {
        score,
        evidence: format!(
            "Standard collections {}; {} complexity/Big-O mention(s) in code+docs.",
            if s.uses_good_structures {
                "used"
            } else {
                "not detected"
            },
            s.big_o_mentions
        ),
        fixes,
    }
}

type CategoryFn = fn(&Signals) -> Category;

/// (display name, weight, scorer) tuples per course; weights sum to 100.
fn rubric_for(course: &str) -> Vec<(&'static str, i64, CategoryFn)> {
    match course {
        "cs3500" => vec![
            ("Object-oriented design", 25, category_design_ood),
            ("Tests & coverage", 20, category_tests),
            ("Documentation & Javadoc", 15, category_docs),
            ("Code style & cleanliness", 15, category_style),
            ("Correctness & build", 15, category_build),
            ("Abstraction & encapsulation", 10, category_abstraction),
        ],
        _ => vec![
            ("Correctness & build", 20, category_build),
            ("Tests & coverage", 20, category_tests),
            ("Data structures & complexity", 15, category_data_structures),
            ("Documentation & Javadoc", 15, category_docs),
            ("Code style & cleanliness", 15, category_style),
            ("Design & structure", 15, category_design_ood),
        ],
    }
}

/// Resolve "auto" to a concrete course using OOD vs DSA signal strength.
pub fn detect_course(requested: &str, s: &Signals) -> String {
    if requested != "auto" {
        return requested.to_string();
    }
    let ood = s.mvc_score * 2 + s.interface_count + s.pattern_hits.len();
    let dsa = if s.uses_good_structures { 2 } else { 0 } + s.big_o_mentions.min(4);
    if ood >= dsa {
        "cs3500".to_string()
    } else {
        "cs2420".to_string()
    }
}

/// JavaScript-style letter grade thresholds.
pub fn letter(p: f64) -> &'static str {
    match p {
        p if p >= 97.0 => "A+",
        p if p >= 93.0 => "A",
        p if p >= 90.0 => "A-",
        p if p >= 87.0 => "B+",
        p if p >= 83.0 => "B",
        p if p >= 80.0 => "B-",
        p if p >= 77.0 => "C+",
        p if p >= 73.0 => "C",
        p if p >= 70.0 => "C-",
        p if p >= 60.0 => "D",
        _ => "F",
    }
}

/// Evaluate the project: score every category, total it, and rank the gaps.
pub fn grade(course: &str, signals: &Signals) -> Grade {
    let results: Vec<Scored> = rubric_for(course)
        .into_iter()
        .map(|(name, weight, scorer)| {
            let c = scorer(signals);
            Scored {
                name,
                weight,
                earned: c.score * weight as f64,
                score: c.score,
                evidence: c.evidence,
                fixes: c.fixes,
            }
        })
        .collect();

    let total: f64 = results.iter().map(|r| r.earned).sum();
    let pct = js_round(total * 10.0) / 10.0;
    let grade = letter(pct);

    // A category is "at A+" once it earns ≥97% of its weight; everything else
    // becomes a gap, ranked by recoverable points (stable for ties).
    let mut gaps: Vec<Gap> = results
        .iter()
        .filter(|r| r.earned < r.weight as f64 * 0.97)
        .map(|r| Gap {
            scored: Scored {
                name: r.name,
                weight: r.weight,
                earned: r.earned,
                score: r.score,
                evidence: r.evidence.clone(),
                fixes: r.fixes.clone(),
            },
            recoverable: r.weight as f64 - r.earned,
        })
        .collect();
    gaps.sort_by(|a, b| b.recoverable.partial_cmp(&a.recoverable).unwrap());

    Grade {
        course: course.to_string(),
        pct,
        grade,
        total,
        results,
        gaps,
    }
}

/// `slice(0, n).join(", ")` for the first `n` entries.
fn join_first(items: &[String], n: usize) -> String {
    items.iter().take(n).cloned().collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letter_thresholds() {
        assert_eq!(letter(97.0), "A+");
        assert_eq!(letter(96.9), "A");
        assert_eq!(letter(59.9), "F");
        assert_eq!(letter(100.0), "A+");
    }
}
