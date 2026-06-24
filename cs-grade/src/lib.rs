//! `git-cs-grade` — an objective structural rubric for CS2420 / CS3500 projects.
//!
//! The pipeline is language-aware:
//!
//! 1. [`project::walk_files`] walks the tree; [`lang::detect`] picks the
//!    dominant language's [`lang::LangProfile`].
//! 2. [`project::Project::from_files`] partitions source/test files for that
//!    language and builds the corpora.
//! 3. [`signals::Signals::compute`] extracts ~25 structural metrics via the
//!    profile's regexes.
//! 4. [`scoring::grade`] scores each (language-agnostic) rubric category.
//! 5. [`report`] renders GRADE.md or the `--json` payload.
//!
//! Keeping this logic in a library (rather than the binary) lets the test suite
//! exercise it directly and lets callers embed the grader.

pub mod fmt;
pub mod lang;
pub mod paths;
pub mod project;
pub mod report;
pub mod scoring;
pub mod signals;

use project::Project;
use scoring::Grade;
use std::path::Path;

/// Scan `root`, detect its language, and grade it for `course` ("auto",
/// "cs2420", "cs3500", or "full"). Returns the grade plus the source/test file
/// counts used in the report header.
pub fn evaluate(root: &Path, course: &str) -> (Grade, usize, usize) {
    let files = project::walk_files(root);
    let profile = lang::detect(&files);
    let project = Project::from_files(root, files, &profile);
    let signals = signals::Signals::compute(&project, &profile);
    let resolved = scoring::detect_course(course, &signals);
    let graded = scoring::grade(&resolved, &signals);
    (graded, project.src_files.len(), project.test_files.len())
}

/// The project label shown in the report header: the root's base name.
pub fn project_label(root: &Path) -> String {
    project::relativize(root, root)
}
