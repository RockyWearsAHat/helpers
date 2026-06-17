//! Knowledge note read/write/cache-search — port of the note half of
//! `lib/mcp-knowledge-rw.js`.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use regex::Regex;
use serde_json::Value;

use crate::knowledge::community::{fetch_github_file, submit_community_research};
use crate::knowledge::index::{
    build_knowledge_index, build_knowledge_snippet, collect_markdown_files, score_knowledge_match,
    BuildResult,
};
use crate::knowledge::KnowledgeConfig;
use crate::util::{get_markdown_title, summarize_text, tokenize_query};

fn rel(base: &Path, p: &Path) -> String {
    pathdiff(p, base)
}

/// Lexical relative path of `p` from `base` (no filesystem access), good enough
/// for display and the containment checks below.
fn pathdiff(p: &Path, base: &Path) -> String {
    let p = normalize(p);
    let base = normalize(base);
    p.strip_prefix(&base)
        .map(|r| r.to_string_lossy().to_string())
        .unwrap_or_else(|_| p.to_string_lossy().to_string())
}

/// Lexically normalize a path (resolve `.`/`..` without touching the FS).
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn is_under(child: &Path, parent: &Path) -> bool {
    normalize(child).starts_with(normalize(parent))
}

// ─── file content resolution ────────────────────────────────────────────────

/// Read a knowledge file: local workspace → repo bundle → GitHub community.
pub fn read_knowledge_file_content(cfg: &KnowledgeConfig, filename: &str) -> Option<String> {
    for root in [&cfg.knowledge_root, &cfg.repo_knowledge_root] {
        if let Ok(s) = std::fs::read_to_string(root.join(filename)) {
            return Some(s);
        }
    }
    fetch_github_file(cfg, &format!("knowledge/{filename}")).ok()
}

// ─── search_knowledge_cache ─────────────────────────────────────────────────

pub struct CacheHit {
    pub rank: usize,
    pub path: String,
    pub title: String,
    pub source: &'static str,
    pub snippet: String,
}

pub struct CacheSearch {
    pub query: String,
    pub total: usize,
    pub results: Vec<CacheHit>,
}

/// Keyword-rank markdown notes across the workspace and repo knowledge roots,
/// returning the top `max_results` scored hits with snippets. Errors if `query`
/// has no searchable terms.
pub fn search_knowledge_cache(
    cfg: &KnowledgeConfig,
    query: &str,
    max_results: usize,
) -> Result<CacheSearch, String> {
    let terms = tokenize_query(query);
    if terms.is_empty() {
        return Err("search_knowledge_cache query must include searchable terms.".into());
    }

    let mut roots: Vec<(&PathBuf, &'static str)> = vec![(&cfg.knowledge_root, "workspace")];
    if normalize(&cfg.repo_knowledge_root) != normalize(&cfg.knowledge_root) {
        roots.push((&cfg.repo_knowledge_root, "repo"));
    }

    let mut seen = HashSet::new();
    let mut scored: Vec<(String, String, f64, &'static str, String)> = Vec::new();
    for (root, source) in roots {
        for fp in collect_markdown_files(root) {
            let canon = normalize(&fp);
            if !seen.insert(canon) {
                continue;
            }
            let body = std::fs::read_to_string(&fp).unwrap_or_default();
            let relative_path = if source == "workspace" {
                rel(&cfg.workspace_root, &fp)
            } else {
                rel(&cfg.repo_root, &fp)
            };
            let stem = fp.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let title = get_markdown_title(&body, stem);
            let score = score_knowledge_match(&relative_path, &title, &body, &terms);
            if score <= 0 {
                continue;
            }
            let snippet = build_knowledge_snippet(&body, &terms);
            scored.push((relative_path, title, score as f64, source, snippet));
        }
    }

    scored.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    let total = scored.len();
    let results = scored
        .into_iter()
        .take(max_results)
        .enumerate()
        .map(|(i, (path, title, _, source, snippet))| CacheHit {
            rank: i + 1,
            path,
            title,
            source,
            snippet,
        })
        .collect();

    Ok(CacheSearch {
        query: query.to_string(),
        total,
        results,
    })
}

// ─── read_knowledge_note ────────────────────────────────────────────────────

pub struct NoteResult {
    pub path: String,
    pub source: &'static str,
    pub title: String,
    pub text: String,
}

/// Read one knowledge note by filename or workspace-relative path, resolving
/// workspace → repo bundle → GitHub community. Rejects paths outside the
/// knowledge directories; truncates to `max_chars` when `max_chars > 0`.
pub fn read_knowledge_note(
    cfg: &KnowledgeConfig,
    note_path: &str,
    max_chars: usize,
) -> Result<NoteResult, String> {
    let resolved = if !note_path.contains('/') {
        cfg.knowledge_root.join(note_path)
    } else {
        normalize(&cfg.workspace_root.join(note_path))
    };
    let in_workspace = is_under(&resolved, &cfg.knowledge_root);
    let in_repo = is_under(&resolved, &cfg.repo_knowledge_root);
    if !in_workspace && !in_repo {
        return Err("read_knowledge_note only allows files under the knowledge directory.".into());
    }
    let filename = if in_workspace {
        rel(&cfg.knowledge_root, &resolved)
    } else {
        rel(&cfg.repo_knowledge_root, &resolved)
    };

    let mut text = None;
    let mut source = "workspace";
    if let Ok(s) = std::fs::read_to_string(cfg.knowledge_root.join(&filename)) {
        text = Some(s);
    }
    if text.is_none() {
        if let Ok(s) = std::fs::read_to_string(cfg.repo_knowledge_root.join(&filename)) {
            text = Some(s);
            source = "repo";
        }
    }
    if text.is_none() {
        if let Ok(s) = fetch_github_file(cfg, &format!("knowledge/{filename}")) {
            text = Some(s);
            source = "community";
        }
    }
    let text = text
        .ok_or_else(|| format!("Knowledge note not found locally or in community: {filename}"))?;
    let trimmed = text.trim();
    let stem = Path::new(&filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&filename);

    Ok(NoteResult {
        path: format!("knowledge/{filename}"),
        source,
        title: get_markdown_title(trimmed, stem),
        text: if max_chars > 0 {
            summarize_text(trimmed, max_chars)
        } else {
            trimmed.to_string()
        },
    })
}

// ─── note writers ───────────────────────────────────────────────────────────

/// Resolve a writable note path under the knowledge root, rejecting paths that
/// escape it or are not `.md` files.
pub fn resolve_knowledge_path(cfg: &KnowledgeConfig, note_path: &str) -> Result<PathBuf, String> {
    let resolved = if !note_path.contains('/') {
        cfg.knowledge_root.join(note_path)
    } else {
        normalize(&cfg.workspace_root.join(note_path))
    };
    if !is_under(&resolved, &cfg.knowledge_root) {
        return Err(format!(
            "Knowledge note path must be under the knowledge directory ({}/).",
            rel(&cfg.workspace_root, &cfg.knowledge_root)
        ));
    }
    if resolved.extension().and_then(|e| e.to_str()) != Some("md") {
        return Err("Knowledge notes must be .md files.".into());
    }
    Ok(resolved)
}

pub struct WriteResult {
    pub action: &'static str,
    pub path: String,
    pub heading: Option<String>,
    pub index: IndexStatus,
    pub publish: PublishStatus,
}

pub enum IndexStatus {
    Rebuilt {
        file_count: usize,
        term_count: usize,
        path: String,
    },
    Failed(String),
}

pub struct PublishStatus {
    pub status: String,
    pub message: Option<String>,
    pub output: Option<String>,
}

/// Create (or, with `overwrite=true`, replace) a knowledge note from the call
/// args, then rebuild the index and optionally publish.
pub fn write_knowledge_note(cfg: &KnowledgeConfig, args: &Value) -> Result<WriteResult, String> {
    let note_path = str_arg(args, "path");
    if note_path.is_empty() {
        return Err("write_knowledge_note requires a non-empty path.".into());
    }
    let content = str_arg(args, "content");
    if content.is_empty() {
        return Err("write_knowledge_note requires non-empty content.".into());
    }
    let resolved = resolve_knowledge_path(cfg, &note_path)?;
    let exists = resolved.exists();
    if exists && args.get("overwrite").and_then(Value::as_bool) != Some(true) {
        return Err(format!(
            "File already exists: {}. Set overwrite=true to replace it.",
            rel(&cfg.workspace_root, &resolved)
        ));
    }
    if let Some(parent) = resolved.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&resolved, &content).map_err(|e| e.to_string())?;
    Ok(finalize(
        cfg,
        if exists { "overwritten" } else { "created" },
        &resolved,
        None,
        args,
    ))
}

/// Replace the body under a given heading in an existing note (other sections
/// preserved), then rebuild the index and optionally publish.
pub fn update_knowledge_note(cfg: &KnowledgeConfig, args: &Value) -> Result<WriteResult, String> {
    let note_path = str_arg(args, "path");
    let heading = str_arg(args, "heading");
    let content = str_arg(args, "content");
    if note_path.is_empty() {
        return Err("update_knowledge_note requires a non-empty path.".into());
    }
    if heading.is_empty() {
        return Err("update_knowledge_note requires a heading to locate the section.".into());
    }
    if content.is_empty() {
        return Err("update_knowledge_note requires non-empty content.".into());
    }
    let resolved = resolve_knowledge_path(cfg, &note_path)?;
    let text = std::fs::read_to_string(&resolved).map_err(|e| e.to_string())?;

    let heading_re = Regex::new(&format!(
        r"(?m)^(#{{1,6}})\s+{}\s*$",
        regex::escape(&heading)
    ))
    .map_err(|e| e.to_string())?;
    let m = heading_re.captures(&text).ok_or_else(|| {
        format!(
            "Heading \"{heading}\" not found in {}.",
            rel(&cfg.workspace_root, &resolved)
        )
    })?;
    let whole = m.get(0).unwrap();
    let level = m.get(1).unwrap().as_str().len();
    let section_start = whole.end();

    let next_re = Regex::new(&format!(r"(?m)^#{{1,{level}}}\s+")).unwrap();
    let rest = &text[section_start..];
    let section_end = next_re
        .find(rest)
        .map(|mm| section_start + mm.start())
        .unwrap_or(text.len());

    let updated = format!(
        "{}\n{}\n\n{}",
        &text[..section_start],
        content,
        &text[section_end..]
    );
    std::fs::write(&resolved, updated).map_err(|e| e.to_string())?;
    Ok(finalize(cfg, "updated", &resolved, Some(heading), args))
}

/// Append content to the end of an existing note, then rebuild the index and
/// optionally publish.
pub fn append_to_knowledge_note(
    cfg: &KnowledgeConfig,
    args: &Value,
) -> Result<WriteResult, String> {
    let note_path = str_arg(args, "path");
    let content = str_arg(args, "content");
    if note_path.is_empty() {
        return Err("append_to_knowledge_note requires a non-empty path.".into());
    }
    if content.is_empty() {
        return Err("append_to_knowledge_note requires non-empty content.".into());
    }
    let resolved = resolve_knowledge_path(cfg, &note_path)?;
    let existing = std::fs::read_to_string(&resolved).map_err(|e| e.to_string())?;
    let separator = if existing.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    std::fs::write(&resolved, format!("{existing}{separator}{content}\n"))
        .map_err(|e| e.to_string())?;
    Ok(finalize(cfg, "appended", &resolved, None, args))
}

/// Submit an existing knowledge note to the shared community base, returning the
/// note's relative path and the submission output.
pub fn submit_research(cfg: &KnowledgeConfig, args: &Value) -> Result<(String, String), String> {
    let note_path = str_arg(args, "path");
    if note_path.is_empty() {
        return Err("submit_community_research requires a non-empty path.".into());
    }
    let resolved = resolve_knowledge_path(cfg, &note_path)?;
    if !resolved.exists() {
        return Err(format!(
            "Knowledge note not found: {}",
            rel(&cfg.workspace_root, &resolved)
        ));
    }
    let output = submit_community_research(cfg, &resolved)?;
    Ok((rel(&cfg.workspace_root, &resolved), output))
}

// ─── finalize (rebuild index + maybe publish) ───────────────────────────────

fn finalize(
    cfg: &KnowledgeConfig,
    action: &'static str,
    resolved: &Path,
    heading: Option<String>,
    args: &Value,
) -> WriteResult {
    let index = match build_knowledge_index(cfg) {
        Ok(BuildResult {
            file_count,
            term_count,
            path,
        }) => IndexStatus::Rebuilt {
            file_count,
            term_count,
            path,
        },
        Err(e) => IndexStatus::Failed(e),
    };
    let publish = maybe_publish(cfg, args, resolved, &index);
    WriteResult {
        action,
        path: rel(&cfg.workspace_root, resolved),
        heading,
        index,
        publish,
    }
}

fn maybe_publish(
    cfg: &KnowledgeConfig,
    args: &Value,
    resolved: &Path,
    index: &IndexStatus,
) -> PublishStatus {
    let requested = args.get("publish").is_some();
    if !requested {
        return PublishStatus {
            status: "local-only".into(),
            message: Some(
                "Note kept local. Set publish=true to submit it to the shared knowledge base."
                    .into(),
            ),
            output: None,
        };
    }
    if args.get("publish").and_then(Value::as_bool) != Some(true) {
        return PublishStatus {
            status: "local-only".into(),
            message: Some("Note kept local by request.".into()),
            output: None,
        };
    }
    if !matches!(index, IndexStatus::Rebuilt { .. }) {
        return PublishStatus {
            status: "blocked".into(),
            message: Some(
                "Local knowledge index rebuild failed, so the note was not published.".into(),
            ),
            output: None,
        };
    }
    if !sharing_enabled(cfg) {
        return PublishStatus {
            status: "blocked".into(),
            message: Some("Knowledge sharing is not enabled. Set shareKnowledge: true (or legacy shareResearch: true) in community settings.".into()),
            output: None,
        };
    }
    match submit_community_research(cfg, resolved) {
        Ok(output) => PublishStatus {
            status: "submitted".into(),
            message: None,
            output: Some(output),
        },
        Err(e) => PublishStatus {
            status: "failed".into(),
            message: Some(e),
            output: None,
        },
    }
}

fn sharing_enabled(cfg: &KnowledgeConfig) -> bool {
    let read = |p: PathBuf| -> Value {
        std::fs::read_to_string(p)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Value::Null)
    };
    let global = read(
        crate::git::home()
            .join(".copilot")
            .join("devops-audit-community-settings.json"),
    );
    let workspace = read(
        cfg.workspace_root
            .join(".github")
            .join("devops-audit-community-settings.json"),
    );
    let pick = |key: &str| {
        workspace
            .get(key)
            .or_else(|| global.get(key))
            .and_then(Value::as_bool)
    };
    pick("shareKnowledge")
        .or_else(|| pick("shareResearch"))
        .unwrap_or(false)
}

fn str_arg(args: &Value, key: &str) -> String {
    args.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}
