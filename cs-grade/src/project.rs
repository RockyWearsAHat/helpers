//! Scanning a project directory into the inputs the rubric needs: the full file
//! list (with root-relative paths), the source/test partition for the detected
//! language, and the concatenated text corpora the signal extractors run over.
//!
//! The walk/relativize/read logic mirrors the original `git-cs-grade.js`
//! exactly; the source/test partition is now driven by a [`LangProfile`] (the
//! Java profile reproduces the original `.java` + `isTestFile` behaviour) so the
//! same pipeline grades any supported language.

use crate::lang::LangProfile;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

/// Directory/entry names skipped during the walk (build output, VCS, IDE, etc.).
const IGNORE: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "build",
    "out",
    "bin",
    "dist",
    ".idea",
    ".vscode",
    ".gradle",
    ".settings",
    "__pycache__",
];

/// A single discovered file: absolute path plus its root-relative, forward-slash
/// path (the form every rubric regex matches against).
pub struct FileEntry {
    pub abs: PathBuf,
    pub rel: String,
}

/// A scanned project: the inputs every category scorer reads from.
pub struct Project {
    /// Display name for the project root: its base name (matches `rel(root)`).
    pub root_name: String,
    pub files: Vec<FileEntry>,
    pub src_files: Vec<usize>,  // indices into `files`
    pub test_files: Vec<usize>, // indices into `files`
    /// Indices of every file in the detected language (source and test).
    pub lang_files: Vec<usize>,
    /// Source files joined with "\n" (the primary analysis corpus).
    pub joined: String,
    /// Test files joined with "\n".
    pub test_corpus: String,
}

impl Project {
    /// Walk `root` and build the analysis inputs, partitioning source/test files
    /// for `profile`'s language.
    pub fn scan(root: &Path, profile: &LangProfile) -> Project {
        Project::from_files(root, walk_files(root), profile)
    }

    /// Partition a pre-walked file list for `profile`'s language and build the
    /// corpora. Split from [`scan`] so the caller can detect the language from
    /// the same `files` it then partitions (one walk, not two).
    pub fn from_files(root: &Path, files: Vec<FileEntry>, profile: &LangProfile) -> Project {
        let is_test = TestMatcher::for_profile(profile);
        let mut src_files = Vec::new();
        let mut test_files = Vec::new();
        let mut lang_files = Vec::new();
        for (i, f) in files.iter().enumerate() {
            if !profile.owns(&f.rel) {
                continue;
            }
            lang_files.push(i);
            if is_test.matches(&f.abs.to_string_lossy(), &f.rel) {
                test_files.push(i);
            } else {
                src_files.push(i);
            }
        }

        let joined = join_text(&files, &src_files);
        let test_corpus = join_text(&files, &test_files);
        let root_name = base_name(root);

        Project {
            root_name,
            files,
            src_files,
            test_files,
            lang_files,
            joined,
            test_corpus,
        }
    }

    /// Indices of every file in the detected language (source and test).
    pub fn lang_files(&self) -> impl Iterator<Item = usize> + '_ {
        self.lang_files.iter().copied()
    }

    /// Read a discovered file as UTF-8 (lossy, like Node's `readFileSync`),
    /// returning "" on any error.
    pub fn read(&self, index: usize) -> String {
        read_text(&self.files[index].abs)
    }
}

/// Walk `root` and return every discovered file in a stable, platform-independent
/// order (sorted by root-relative path) so corpora — and therefore scores — are
/// deterministic. Used to detect the language before partitioning source/tests.
pub fn walk_files(root: &Path) -> Vec<FileEntry> {
    let mut files = Vec::new();
    walk(root, root, &mut files);
    files.sort_by(|a, b| a.rel.cmp(&b.rel));
    files
}

/// Read a file as lossy UTF-8, "" on error (mirrors `readText`).
pub fn read_text(path: &Path) -> String {
    fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}

/// Root-relative, forward-slash path of `target` under `root`; the base name
/// when `target == root` (mirrors `rel(f) = relative(root, f) || basename(f)`).
pub fn relativize(root: &Path, target: &Path) -> String {
    match target.strip_prefix(root) {
        Ok(rest) if rest.as_os_str().is_empty() => base_name(target),
        Ok(rest) => normalize_slashes(&rest.to_string_lossy()),
        Err(_) => base_name(target),
    }
}

fn base_name(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| normalize_slashes(&p.to_string_lossy()))
}

fn normalize_slashes(s: &str) -> String {
    if std::path::MAIN_SEPARATOR == '/' {
        s.to_string()
    } else {
        s.replace(std::path::MAIN_SEPARATOR, "/")
    }
}

fn join_text(files: &[FileEntry], indices: &[usize]) -> String {
    let parts: Vec<String> = indices.iter().map(|&i| read_text(&files[i].abs)).collect();
    parts.join("\n")
}

/// Recursive directory walk, skipping any entry whose name is in `IGNORE`.
fn walk(root: &Path, dir: &Path, acc: &mut Vec<FileEntry>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if IGNORE.contains(&name.as_ref()) {
            continue;
        }
        let full = entry.path();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            walk(root, &full, acc);
        } else {
            let rel = relativize(root, &full);
            acc.push(FileEntry { abs: full, rel });
        }
    }
}

/// The independent conditions that mark a source file as a test, one regex per
/// [`LangProfile`] partition rule. Each is optional (an empty pattern means
/// "this rule doesn't apply"); a file is a test if any present rule matches. For
/// the Java profile these reproduce the original `isTestFile` exactly, including
/// the deliberate case-sensitivity of the suffix/basename rules.
struct TestMatcher {
    rel_dir: Option<Regex>,       // matched against the root-relative path
    abs_suffix: Option<Regex>,    // matched against the absolute path
    basename_word: Option<Regex>, // matched against the file basename
}

impl TestMatcher {
    fn for_profile(profile: &LangProfile) -> TestMatcher {
        let compile = |pat: &str| {
            if pat.is_empty() {
                None
            } else {
                Some(Regex::new(pat).expect("valid test-partition regex"))
            }
        };
        TestMatcher {
            rel_dir: compile(profile.test_dir),
            abs_suffix: compile(profile.test_suffix),
            basename_word: compile(profile.test_basename),
        }
    }

    fn matches(&self, abs: &str, rel: &str) -> bool {
        if self.rel_dir.as_ref().is_some_and(|r| r.is_match(rel))
            || self.abs_suffix.as_ref().is_some_and(|r| r.is_match(abs))
        {
            return true;
        }
        let base = abs.rsplit(['/', '\\']).next().unwrap_or(abs);
        self.basename_word.as_ref().is_some_and(|r| r.is_match(base))
    }
}
