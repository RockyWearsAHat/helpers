//! `project_setup` (a.k.a. `gsh setup`) — a deterministic project build-out
//! engine. Replaces the old Copilot devops-audit: instead of installing an AI
//! agent system, it analyses the repository *without any AI* and produces a
//! concise, structured plan that helps an agent build the project out to
//! completion fast — while enforcing three rules:
//!
//!   1. Minimal context — the plan is a tight, ranked summary, never a dump.
//!   2. Understand goals first — purpose/goals are surfaced (or flagged as
//!      unknown) before any build-out steps are proposed.
//!   3. Clarify with the user — ambiguities become explicit questions to ask
//!      before acting.
//!
//! Output is written to `.gsh/SETUP.md` and returned as the tool result.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::git::workspace_root;
use crate::index::walk::walk_repo;
use crate::proto::{text, ToolResult};

// ─── data model ──────────────────────────────────────────────────────────────

/// A detected technology stack and its conventional commands.
struct Stack {
    name: &'static str,
    manifest: String,
    build: Option<String>,
    test: Option<String>,
    lint: Option<String>,
}

/// Presence signals that drive the gap checklist.
#[derive(Default)]
struct Signals {
    readme: bool,
    tests: bool,
    ci: bool,
    license: bool,
    gitignore: bool,
    lint_config: bool,
    docs: bool,
    editorconfig: bool,
}

struct SetupReport {
    name: String,
    purpose: Option<String>,
    languages: Vec<(String, usize)>,
    top_dirs: Vec<String>,
    stacks: Vec<Stack>,
    entrypoints: Vec<String>,
    gaps: Vec<String>,
    questions: Vec<String>,
}

// ─── public tool entry ───────────────────────────────────────────────────────

fn root_arg(args: &Value) -> PathBuf {
    match args.get("root").and_then(Value::as_str) {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => workspace_root(),
    }
}

pub fn run(args: &Value) -> ToolResult {
    let root = root_arg(args);
    if !root.exists() {
        return Err(format!("project_setup: path not found: {}", root.display()));
    }
    let report = analyze(&root);
    let md = render(&report);

    // Persist the plan unless the caller opts out, so the agent (and the user)
    // share one durable, minimal source of truth.
    if args.get("write").and_then(Value::as_bool) != Some(false) {
        let dir = root.join(".gsh");
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(dir.join("SETUP.md"), &md);
    }
    Ok(vec![text(md)])
}

// ─── analysis (deterministic) ────────────────────────────────────────────────

fn analyze(root: &Path) -> SetupReport {
    let files = walk_repo(root);
    let rels: Vec<&str> = files.iter().map(|f| f.rel.as_str()).collect();
    let has = |p: &str| rels.contains(&p);
    let has_under = |dir: &str| rels.iter().any(|r| r.starts_with(dir));
    let has_match = |pred: &dyn Fn(&str) -> bool| rels.iter().any(|r| pred(r));

    // Language counts from extensions.
    let languages = language_counts(&files);

    // Top-level directories (skip noise already handled by the walk).
    let mut top_dirs: Vec<String> = Vec::new();
    for r in &rels {
        if let Some((dir, _)) = r.split_once('/') {
            if !top_dirs.iter().any(|d| d == dir) {
                top_dirs.push(dir.to_string());
            }
        }
    }
    top_dirs.sort();

    let stacks = detect_stacks(root, &has);

    let signals = Signals {
        readme: has("README.md") || has("README") || has("readme.md"),
        tests: has_match(&is_test_path),
        ci: has_under(".github/workflows/") || has(".gitlab-ci.yml") || has(".circleci/config.yml"),
        license: has("LICENSE") || has("LICENSE.md") || has("LICENSE.txt"),
        gitignore: has(".gitignore"),
        lint_config: has_match(&is_lint_config),
        docs: has_under("docs/") || has_under("doc/"),
        editorconfig: has(".editorconfig"),
    };

    let purpose = detect_purpose(root, &stacks);
    let entrypoints = detect_entrypoints(&rels);
    let gaps = gap_checklist(&signals, &stacks);
    let questions = clarifying_questions(&purpose, &signals);

    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    SetupReport {
        name,
        purpose,
        languages,
        top_dirs,
        stacks,
        entrypoints,
        gaps,
        questions,
    }
}

/// Map file extensions to human language names and count them, most common
/// first.
fn language_counts(files: &[crate::index::walk::WalkedFile]) -> Vec<(String, usize)> {
    use std::collections::HashMap;
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    for f in files {
        if let Some(lang) = lang_for_ext(&f.ext) {
            *counts.entry(lang).or_insert(0) += 1;
        }
    }
    let mut v: Vec<(String, usize)> = counts
        .into_iter()
        .map(|(k, n)| (k.to_string(), n))
        .collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

fn lang_for_ext(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "rs" => "Rust",
        "js" | "mjs" | "cjs" => "JavaScript",
        "ts" | "tsx" => "TypeScript",
        "jsx" => "JavaScript (JSX)",
        "py" => "Python",
        "go" => "Go",
        "java" => "Java",
        "rb" => "Ruby",
        "php" => "PHP",
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" => "C++",
        "cs" => "C#",
        "swift" => "Swift",
        "kt" | "kts" => "Kotlin",
        "sh" | "bash" => "Shell",
        "md" => "Markdown",
        _ => return None,
    })
}

/// Detect technology stacks from conventional manifest files.
fn detect_stacks(root: &Path, has: &dyn Fn(&str) -> bool) -> Vec<Stack> {
    let mut stacks: Vec<Stack> = Vec::new();

    if has("package.json") {
        let (build, test, lint) = node_scripts(root);
        stacks.push(Stack {
            name: "Node.js",
            manifest: "package.json".into(),
            build,
            test,
            lint,
        });
    }
    if has("Cargo.toml") {
        stacks.push(Stack {
            name: "Rust",
            manifest: "Cargo.toml".into(),
            build: Some("cargo build --release".into()),
            test: Some("cargo test".into()),
            lint: Some("cargo clippy && cargo fmt --check".into()),
        });
    }
    if has("go.mod") {
        stacks.push(Stack {
            name: "Go",
            manifest: "go.mod".into(),
            build: Some("go build ./...".into()),
            test: Some("go test ./...".into()),
            lint: Some("go vet ./...".into()),
        });
    }
    if has("pyproject.toml") || has("requirements.txt") || has("setup.py") {
        let manifest = if has("pyproject.toml") {
            "pyproject.toml"
        } else if has("setup.py") {
            "setup.py"
        } else {
            "requirements.txt"
        };
        stacks.push(Stack {
            name: "Python",
            manifest: manifest.into(),
            build: None,
            test: Some("pytest".into()),
            lint: Some("ruff check . || flake8".into()),
        });
    }
    if has("pom.xml") {
        stacks.push(Stack {
            name: "Java (Maven)",
            manifest: "pom.xml".into(),
            build: Some("mvn -q compile".into()),
            test: Some("mvn -q test".into()),
            lint: None,
        });
    }
    if has("build.gradle") || has("build.gradle.kts") {
        stacks.push(Stack {
            name: "Java/Kotlin (Gradle)",
            manifest: "build.gradle".into(),
            build: Some("./gradlew build".into()),
            test: Some("./gradlew test".into()),
            lint: None,
        });
    }
    if has("Makefile") && stacks.is_empty() {
        stacks.push(Stack {
            name: "Make",
            manifest: "Makefile".into(),
            build: Some("make".into()),
            test: Some("make test".into()),
            lint: None,
        });
    }
    stacks
}

/// Pull `build`/`test`/`lint` (and sensible fallbacks) from package.json scripts.
fn node_scripts(root: &Path) -> (Option<String>, Option<String>, Option<String>) {
    let raw = fs::read_to_string(root.join("package.json")).unwrap_or_default();
    let pkg: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    let script = |key: &str| -> Option<String> {
        pkg.get("scripts")
            .and_then(|s| s.get(key))
            .and_then(Value::as_str)
            .map(|_| format!("npm run {key}"))
    };
    let build = script("build");
    let test = script("test").or_else(|| {
        pkg.get("scripts")
            .and_then(|s| s.get("test"))
            .map(|_| "npm test".to_string())
    });
    let lint = script("lint");
    (build, test, lint)
}

/// Best-effort project purpose from package.json description, then the first
/// real prose line of the README.
fn detect_purpose(root: &Path, stacks: &[Stack]) -> Option<String> {
    if stacks.iter().any(|s| s.name == "Node.js") {
        let raw = fs::read_to_string(root.join("package.json")).unwrap_or_default();
        if let Ok(pkg) = serde_json::from_str::<Value>(&raw) {
            if let Some(d) = pkg.get("description").and_then(Value::as_str) {
                if !d.trim().is_empty() {
                    return Some(d.trim().to_string());
                }
            }
        }
    }
    for name in ["README.md", "README", "readme.md"] {
        let p = root.join(name);
        if p.is_file() {
            if let Ok(content) = fs::read_to_string(&p) {
                if let Some(line) = first_prose_line(&content) {
                    return Some(line);
                }
            }
        }
    }
    None
}

/// First non-empty, non-heading, non-badge line of markdown.
fn first_prose_line(md: &str) -> Option<String> {
    for line in md.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with('!') || t.starts_with("<") {
            continue;
        }
        let t = t.trim_start_matches('>').trim();
        if t.len() >= 12 {
            return Some(t.to_string());
        }
    }
    None
}

/// Conventional entry-point files, in priority order, capped for minimal context.
fn detect_entrypoints(rels: &[&str]) -> Vec<String> {
    const CANDIDATES: &[&str] = &[
        "src/main.rs",
        "src/lib.rs",
        "main.go",
        "cmd/main.go",
        "src/index.ts",
        "src/index.js",
        "index.ts",
        "index.js",
        "src/main.py",
        "main.py",
        "app.py",
        "src/App.tsx",
        "src/app.tsx",
    ];
    let mut out: Vec<String> = CANDIDATES
        .iter()
        .filter(|c| rels.contains(c))
        .map(|c| c.to_string())
        .collect();
    out.truncate(5);
    out
}

/// The prioritized build-out checklist: only genuinely-missing pieces.
fn gap_checklist(s: &Signals, stacks: &[Stack]) -> Vec<String> {
    let mut gaps = Vec::new();
    if !s.readme {
        gaps.push("Add a README.md stating the project's purpose, setup, and usage.".into());
    }
    if !s.tests {
        gaps.push("Add a test suite — no test files detected.".into());
    }
    if !s.ci {
        gaps.push("Add CI (e.g. .github/workflows) to build, test, and lint on push.".into());
    }
    if !s.lint_config && !stacks.is_empty() {
        gaps.push("Add linter/formatter config so style is enforced consistently.".into());
    }
    if !s.license {
        gaps.push("Add a LICENSE so the terms of use are explicit.".into());
    }
    if !s.gitignore {
        gaps.push("Add a .gitignore so build artifacts never pollute the repo.".into());
    }
    if !s.editorconfig {
        gaps.push("Consider an .editorconfig for consistent whitespace across editors.".into());
    }
    if !s.docs {
        gaps.push("Consider a docs/ directory once the public surface stabilizes.".into());
    }
    gaps
}

/// Questions to ask the user where goals/scope are ambiguous (rules 2 & 3).
fn clarifying_questions(purpose: &Option<String>, s: &Signals) -> Vec<String> {
    let mut q = Vec::new();
    if purpose.is_none() {
        q.push("What is the primary goal of this project, in one sentence?".into());
        q.push("Who is the intended user, and what is the single most important thing it must do well?".into());
    }
    q.push(
        "What does \"done\" look like — the concrete acceptance criteria for this project?".into(),
    );
    if !s.tests {
        q.push("What is the testing strategy and the expected level of coverage?".into());
    }
    q.push(
        "Are there constraints (deadline, platforms, dependencies, performance) I must respect?"
            .into(),
    );
    q
}

fn is_test_path(p: &str) -> bool {
    let pl = p.to_lowercase();
    pl.contains("/test")
        || pl.contains("/tests/")
        || pl.starts_with("test")
        || pl.starts_with("tests/")
        || pl.ends_with("_test.go")
        || pl.ends_with(".test.ts")
        || pl.ends_with(".test.js")
        || pl.ends_with(".spec.ts")
        || pl.ends_with(".spec.js")
        || pl.ends_with("_test.py")
        || pl.contains("test_")
        || pl.contains("__tests__/")
}

fn is_lint_config(p: &str) -> bool {
    let base = Path::new(p)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    base.starts_with(".eslintrc")
        || base.starts_with(".prettierrc")
        || base == "rustfmt.toml"
        || base == ".rustfmt.toml"
        || base == "clippy.toml"
        || base == ".flake8"
        || base == "ruff.toml"
        || base == ".ruff.toml"
        || base == ".golangci.yml"
        || base == ".golangci.yaml"
        || base == "biome.json"
}

// ─── rendering (minimal, structured) ─────────────────────────────────────────

fn render(r: &SetupReport) -> String {
    let mut s = String::new();
    s.push_str(&format!("# Project setup plan — {}\n\n", r.name));
    s.push_str(
        "_Deterministic build-out plan from `gsh setup`. Work top-down: understand goals, \
         clarify with the user, then close the gaps — keeping context minimal._\n\n",
    );

    // 1. Purpose / goals (rule 2 first).
    s.push_str("## Purpose & goals\n\n");
    match &r.purpose {
        Some(p) => s.push_str(&format!("- Detected purpose: {p}\n")),
        None => s.push_str(
            "- **Purpose unknown** — do not start building until the user confirms it (see Questions).\n",
        ),
    }
    s.push('\n');

    // 2. Stack & commands.
    if !r.stacks.is_empty() {
        s.push_str("## Stack & commands\n\n");
        for st in &r.stacks {
            s.push_str(&format!("- **{}** ({})\n", st.name, st.manifest));
            if let Some(b) = &st.build {
                s.push_str(&format!("  - build: `{b}`\n"));
            }
            if let Some(t) = &st.test {
                s.push_str(&format!("  - test: `{t}`\n"));
            }
            if let Some(l) = &st.lint {
                s.push_str(&format!("  - lint: `{l}`\n"));
            }
        }
        s.push('\n');
    }

    // 3. Shape (languages, dirs, entry points) — kept terse.
    s.push_str("## Shape\n\n");
    if !r.languages.is_empty() {
        let langs: Vec<String> = r
            .languages
            .iter()
            .take(6)
            .map(|(l, n)| format!("{l} ({n})"))
            .collect();
        s.push_str(&format!("- Languages: {}\n", langs.join(", ")));
    }
    if !r.top_dirs.is_empty() {
        let dirs: Vec<String> = r.top_dirs.iter().take(12).cloned().collect();
        s.push_str(&format!("- Top-level dirs: {}\n", dirs.join(", ")));
    }
    if !r.entrypoints.is_empty() {
        s.push_str(&format!("- Entry points: {}\n", r.entrypoints.join(", ")));
    }
    s.push('\n');

    // 4. Gap checklist (the build-out path).
    s.push_str("## Build-out checklist\n\n");
    if r.gaps.is_empty() {
        s.push_str("- No structural gaps detected — the project scaffold is complete.\n");
    } else {
        for g in &r.gaps {
            s.push_str(&format!("- [ ] {g}\n"));
        }
    }
    s.push('\n');

    // 5. Questions for the user (rule 3) — always present.
    s.push_str("## Clarify before building (ask the user)\n\n");
    for q in &r.questions {
        s.push_str(&format!("- {q}\n"));
    }
    s.push('\n');

    s
}

// ─── schema ──────────────────────────────────────────────────────────────────

pub fn schema() -> Value {
    json!({
        "name": "project_setup",
        "description": "Analyze the repository deterministically (no AI) and return a concise, structured build-out plan: detected purpose/goals, technology stack with build/test/lint commands, project shape, a prioritized gap checklist, and clarifying questions to ask the user before building. Use this when starting on a project or to drive it toward a complete, well-structured state quickly. Enforces minimal context, understanding goals first, and clarifying ambiguities with the user. Writes .gsh/SETUP.md.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "root": { "type": "string", "description": "Optional path to the project root. Defaults to the current workspace." },
                "write": { "type": "boolean", "description": "Write the plan to .gsh/SETUP.md. Default true." }
            },
            "required": []
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("gsh-setup-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn detects_rust_stack_purpose_and_gaps() {
        let root = scratch("rust");
        fs::write(root.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(
            root.join("README.md"),
            "# X\n\nA small deterministic tool that does one job well.\n",
        )
        .unwrap();

        let r = analyze(&root);
        assert!(r.stacks.iter().any(|s| s.name == "Rust"));
        assert!(r.purpose.is_some());
        assert!(r.entrypoints.contains(&"src/main.rs".to_string()));
        // No tests/CI/license -> those gaps must be present.
        assert!(r.gaps.iter().any(|g| g.contains("test suite")));
        assert!(r.gaps.iter().any(|g| g.contains("CI")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn unknown_purpose_yields_clarifying_questions() {
        let root = scratch("bare");
        fs::write(root.join("notes.txt"), "scratch\n").unwrap();
        let r = analyze(&root);
        assert!(r.purpose.is_none());
        assert!(r.questions.iter().any(|q| q.contains("primary goal")));
        // Rendering always includes the Questions section.
        let md = render(&r);
        assert!(md.contains("Clarify before building"));
        let _ = fs::remove_dir_all(&root);
    }
}
