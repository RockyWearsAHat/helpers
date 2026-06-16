//! `git-cs-grade` — an objective structural rubric for CS2420 / CS3500 projects.
//!
//! The pipeline mirrors the original Node implementation exactly:
//!
//! 1. [`project::Project::scan`] walks the tree and builds the source/test corpora.
//! 2. [`signals::Signals::compute`] extracts ~25 structural metrics via regex.
//! 3. [`scoring::grade`] scores each rubric category and totals the result.
//! 4. [`report`] renders GRADE.md or the `--json` payload.
//!
//! Keeping this logic in a library (rather than the binary) lets the test suite
//! exercise it directly and lets callers embed the grader.

pub mod fmt;
pub mod paths;
pub mod project;
pub mod report;
pub mod scoring;
pub mod signals;

use project::Project;
use scoring::Grade;
use std::path::Path;

/// Scan `root` and grade it for `course` ("auto", "cs2420", or "cs3500").
/// Returns the grade plus the source/test file counts used in the report header.
pub fn evaluate(root: &Path, course: &str) -> (Grade, usize, usize) {
    let project = Project::scan(root);
    let signals = signals::Signals::compute(&project);
    let resolved = scoring::detect_course(course, &signals);
    let graded = scoring::grade(&resolved, &signals);
    (graded, project.src_files.len(), project.test_files.len())
}

/// The project label shown in the report header: the root's base name.
pub fn project_label(root: &Path) -> String {
    project::relativize(root, root)
}
