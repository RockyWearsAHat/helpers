//! Scanning a project directory into the inputs the rubric needs: the full file
//! list (with root-relative paths), the Java source/test partition, and the
//! concatenated text corpora the signal extractors run over.
//!
//! Behaviour mirrors the original `walk`, `rel`, `readText`, and `isTestFile`
//! logic from `git-cs-grade.js` exactly, including which directories are
//! skipped and how relative paths are normalised to forward slashes.

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
    /// Source files joined with "\n" (the primary analysis corpus).
    pub joined: String,
    /// Test files joined with "\n".
    pub test_corpus: String,
}

impl Project {
    /// Walk `root` and build the analysis inputs.
    pub fn scan(root: &Path) -> Project {
        let mut files = Vec::new();
        walk(root, root, &mut files);
        // Stable order independent of filesystem enumeration so corpora — and
        // therefore scores — are deterministic across platforms.
        files.sort_by(|a, b| a.rel.cmp(&b.rel));

        let is_test = test_matcher();
        let mut src_files = Vec::new();
        let mut test_files = Vec::new();
        for (i, f) in files.iter().enumerate() {
            if !f.abs.to_string_lossy().ends_with(".java") {
                continue;
            }
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
            joined,
            test_corpus,
        }
    }

    /// Indices of every `.java` file (source and test).
    pub fn java_files(&self) -> impl Iterator<Item = usize> + '_ {
        self.files
            .iter()
            .enumerate()
            .filter(|(_, f)| f.abs.to_string_lossy().ends_with(".java"))
            .map(|(i, _)| i)
    }

    /// Read a discovered file as UTF-8 (lossy, like Node's `readFileSync`),
    /// returning "" on any error.
    pub fn read(&self, index: usize) -> String {
        read_text(&self.files[index].abs)
    }
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

/// The three independent conditions that mark a Java file as a test, matching
/// `isTestFile` (note the deliberate case-sensitivity of the last two).
struct TestMatcher {
    rel_dir: Regex,       // (^|/)(test|tests)/   case-insensitive
    abs_suffix: Regex,    // Test[s]?\.java$      case-sensitive
    basename_word: Regex, // Tests?\b             case-sensitive
}

fn test_matcher() -> TestMatcher {
    TestMatcher {
        rel_dir: Regex::new(r"(?i)(^|/)(test|tests)/").unwrap(),
        abs_suffix: Regex::new(r"Test[s]?\.java$").unwrap(),
        basename_word: Regex::new(r"Tests?\b").unwrap(),
    }
}

impl TestMatcher {
    fn matches(&self, abs: &str, rel: &str) -> bool {
        if self.rel_dir.is_match(rel) || self.abs_suffix.is_match(abs) {
            return true;
        }
        let base = abs.rsplit(['/', '\\']).next().unwrap_or(abs);
        self.basename_word.is_match(base)
    }
}
