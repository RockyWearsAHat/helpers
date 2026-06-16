//! Structural signals extracted from the project corpora.
//!
//! Every field corresponds 1:1 to a `const` in `git-cs-grade.js`, and each
//! regex is a faithful port of the original (same flags, same case-sensitivity).
//! Counts use non-overlapping left-to-right matching, exactly like JavaScript's
//! `String.prototype.match` with a global flag.

use crate::project::{read_text, Project};
use regex::Regex;

/// Design-pattern names probed for by `\b\w*<name>\b`, in the original order.
const PATTERNS: &[&str] = &[
    "Strategy",
    "Command",
    "Factory",
    "Builder",
    "Observer",
    "Adapter",
    "Decorator",
    "Visitor",
    "Composite",
    "Iterator",
    "Singleton",
    "Facade",
];

/// All derived metrics the rubric scores against.
pub struct Signals {
    pub src_files: usize,
    pub test_files: usize,

    pub public_decls: usize,
    pub javadoc_blocks: usize,
    pub javadoc_ratio: f64,

    pub interface_count: usize,
    pub class_count: usize, // floored at 1, matching `count(...) || 1`
    pub abstract_count: usize,

    pub pattern_hits: Vec<String>,

    pub has_model: bool,
    pub has_view: bool,
    pub has_controller: bool,
    pub mvc_score: usize,

    pub junit_usage: bool,
    pub test_ratio: f64,
    pub assertion_count: usize,

    pub build_files: Vec<String>, // root-relative paths
    pub uses_packages: usize,
    pub uses_src_layout: bool,

    pub readme_bytes: u64,
    pub design_docs: usize,

    pub big_o_mentions: usize,
    pub uses_good_structures: bool,

    pub god_classes: Vec<String>, // root-relative paths, >400 lines
    pub long_method_hits: usize,
    pub debug_prints: usize,
    pub todo_markers: usize,
    pub commented_code: usize,
}

/// Count non-overlapping matches of `pattern` in `hay`.
fn count(pattern: &str, hay: &str) -> usize {
    Regex::new(pattern).unwrap().find_iter(hay).count()
}

/// Whether `pattern` matches anywhere in `hay`.
fn has(pattern: &str, hay: &str) -> bool {
    Regex::new(pattern).unwrap().is_match(hay)
}

/// JS `String.split("\n").length`: one more than the number of newlines.
fn line_count(text: &str) -> usize {
    text.matches('\n').count() + 1
}

impl Signals {
    pub fn compute(project: &Project) -> Signals {
        let joined = &project.joined;
        let test_corpus = &project.test_corpus;
        let src_files = project.src_files.len();
        let test_files = project.test_files.len();

        let public_decls = count(
            r"\bpublic\s+(?:static\s+)?(?:final\s+)?(?:abstract\s+)?(?:class|interface|enum|[\w<>\[\]]+\s+\w+\s*\()",
            joined,
        );
        let javadoc_blocks = count(r"/\*\*[\s\S]*?\*/", joined);
        let javadoc_ratio = if public_decls > 0 {
            (javadoc_blocks as f64 / public_decls as f64).min(1.0)
        } else {
            0.0
        };

        let interface_count = count(r"\binterface\s+\w+", joined);
        let class_count = count(r"\bclass\s+\w+", joined).max(1);
        let abstract_count = count(r"\babstract\s+class\s+\w+", joined);

        let pattern_hits: Vec<String> = PATTERNS
            .iter()
            .filter(|p| has(&format!(r"\b\w*{p}\b"), joined))
            .map(|p| p.to_string())
            .collect();

        // MVC probe runs over the source corpus concatenated with every Java
        // file's relative path (tests included), with no separator between.
        let java_rel_joined = project
            .java_files()
            .map(|i| project.files[i].rel.clone())
            .collect::<Vec<_>>()
            .join("\n");
        let mvc_hay = format!("{joined}{java_rel_joined}");
        let has_model = has(
            r"(?i)(^|/)model(s)?(/|\.|$)|class\s+\w*Model\b|interface\s+\w*Model\b",
            &mvc_hay,
        );
        let has_view = has(
            r"(?i)(^|/)view(s)?(/|\.|$)|class\s+\w*View\b|interface\s+\w*View\b",
            &mvc_hay,
        );
        let has_controller = has(
            r"(?i)(^|/)controller(s)?(/|\.|$)|class\s+\w*Controller\b|interface\s+\w*Controller\b",
            &mvc_hay,
        );
        let mvc_score = has_model as usize + has_view as usize + has_controller as usize;

        let junit_usage = has(r"org\.junit|@Test", &format!("{test_corpus}{joined}"));
        let test_ratio = if src_files > 0 {
            (test_files as f64 / src_files as f64).min(1.0)
        } else {
            0.0
        };
        let assertion_count = count(r"\bassert\w*\s*\(", test_corpus);

        let build_re =
            Regex::new(r"(^|/)(pom\.xml|build\.gradle(\.kts)?|build\.xml|Makefile)$").unwrap();
        let build_files: Vec<String> = project
            .files
            .iter()
            .filter(|f| build_re.is_match(&f.rel))
            .map(|f| f.rel.clone())
            .collect();
        let uses_packages = count(r"(?m)^\s*package\s+[\w.]+;", joined);
        let src_layout_re = Regex::new(r"(^|/)src/").unwrap();
        let uses_src_layout = project
            .java_files()
            .any(|i| src_layout_re.is_match(&project.files[i].rel));

        let readme_re = Regex::new(r"(?i)readme(\.md|\.txt)?$").unwrap();
        let readmes: Vec<usize> = project
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| readme_re.is_match(base_name(&f.rel)))
            .map(|(i, _)| i)
            .collect();
        let readme_bytes: u64 = readmes
            .iter()
            .map(|&i| {
                std::fs::metadata(&project.files[i].abs)
                    .map(|m| m.len())
                    .unwrap_or(0)
            })
            .sum();

        let design_re =
            Regex::new(r"(?i)(design|architecture|analysis|writeup|report)\.(md|txt|pdf)$")
                .unwrap();
        let design_docs: Vec<usize> = project
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| design_re.is_match(base_name(&f.rel)))
            .map(|(i, _)| i)
            .collect();

        // Big-O probe spans the source corpus plus README and design-doc text.
        let readme_text = readmes
            .iter()
            .map(|&i| read_text(&project.files[i].abs))
            .collect::<Vec<_>>()
            .join("\n");
        let design_text = design_docs
            .iter()
            .map(|&i| read_text(&project.files[i].abs))
            .collect::<Vec<_>>()
            .join("\n");
        let big_o_hay = format!("{joined}{readme_text}{design_text}");
        let big_o_mentions = count(
            r"(?i)\bO\([^)]+\)|big-?o|asymptotic|time complexity",
            &big_o_hay,
        );
        let uses_good_structures = has(
            r"\b(HashMap|HashSet|TreeMap|TreeSet|PriorityQueue|ArrayDeque|LinkedList|ArrayList)\b",
            joined,
        );

        let god_classes: Vec<String> = project
            .src_files
            .iter()
            .filter(|&&i| line_count(&project.read(i)) > 400)
            .map(|&i| project.files[i].rel.clone())
            .collect();
        let long_method_hits = count(r"\{[^{}]{1600,}\}", joined);
        let debug_prints = count(r"System\.out\.print|printStackTrace\(", joined);
        let todo_markers = count(r"\b(TODO|FIXME|XXX|HACK)\b", joined);
        let commented_code = count(
            r"(?m)^\s*//\s*(if|for|while|return|System\.|int |String |public |private )",
            joined,
        );

        Signals {
            src_files,
            test_files,
            public_decls,
            javadoc_blocks,
            javadoc_ratio,
            interface_count,
            class_count,
            abstract_count,
            pattern_hits,
            has_model,
            has_view,
            has_controller,
            mvc_score,
            junit_usage,
            test_ratio,
            assertion_count,
            build_files,
            uses_packages,
            uses_src_layout,
            readme_bytes,
            design_docs: design_docs.len(),
            big_o_mentions,
            uses_good_structures,
            god_classes,
            long_method_hits,
            debug_prints,
            todo_markers,
            commented_code,
        }
    }
}

/// Final path segment of a forward-slash relative path (its basename).
fn base_name(rel: &str) -> &str {
    rel.rsplit('/').next().unwrap_or(rel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_count_matches_js_split() {
        assert_eq!(line_count("a\nb"), 2);
        assert_eq!(line_count("a\nb\n"), 3);
        assert_eq!(line_count(""), 1);
    }

    #[test]
    fn count_is_non_overlapping() {
        assert_eq!(count(r"aa", "aaaa"), 2);
        assert_eq!(count(r"\bTODO\b", "TODO TODO FIXME"), 2);
    }
}
