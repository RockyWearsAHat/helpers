//! Knowledge MCP tools — handlers, schemas, and formatters wiring the
//! `knowledge` modules to the MCP surface (replaces the JS knowledge tools).

use serde_json::{json, Value};

use crate::knowledge::index::{
    build_knowledge_index, search_knowledge_index, BuildResult, IndexSearch,
};
use crate::knowledge::notes::{
    append_to_knowledge_note, read_knowledge_file_content, read_knowledge_note,
    search_knowledge_cache, submit_research, update_knowledge_note, write_knowledge_note,
    CacheSearch, IndexStatus, NoteResult, WriteResult,
};
use crate::knowledge::KnowledgeConfig;
use crate::proto::{text, ToolResult};
use crate::util::to_positive_int;

fn max_results(args: &Value, default: i64) -> usize {
    to_positive_int(
        args.get("max_results").unwrap_or(&Value::Null),
        default,
        1,
        20,
    ) as usize
}

// ─── handlers ───────────────────────────────────────────────────────────────

/// Handle `build_knowledge_index`: rebuild the local workspace TF-IDF index.
pub fn run_build_index(_args: &Value) -> ToolResult {
    let cfg = KnowledgeConfig::resolve();
    let r = build_knowledge_index(&cfg)?;
    Ok(vec![text(format_build(&r))])
}

/// Handle `search_knowledge_index`: TF-IDF search, falling back to keyword cache
/// search when no index is available.
pub fn run_search_index(args: &Value) -> ToolResult {
    let query = str_arg(args, "query");
    if query.is_empty() {
        return Err("search_knowledge_index requires a non-empty query.".into());
    }
    let cfg = KnowledgeConfig::resolve();
    let max = max_results(args, 5);
    let read = |filename: &str| read_knowledge_file_content(&cfg, filename);
    match search_knowledge_index(&cfg, &query, max, &read) {
        Some(result) => Ok(vec![text(format_index_search(&result))]),
        // No index available — fall back to the keyword cache search.
        None => {
            let cache = search_knowledge_cache(&cfg, &query, max)?;
            Ok(vec![text(format_cache_search(&cache))])
        }
    }
}

/// Handle `search_knowledge_cache`: keyword search over the note cache.
pub fn run_search_cache(args: &Value) -> ToolResult {
    let query = str_arg(args, "query");
    if query.is_empty() {
        return Err("search_knowledge_cache requires a non-empty query.".into());
    }
    let cfg = KnowledgeConfig::resolve();
    let cache = search_knowledge_cache(&cfg, &query, max_results(args, 5))?;
    Ok(vec![text(format_cache_search(&cache))])
}

/// Handle `read_knowledge_note`: return one note's content (optionally truncated).
pub fn run_read_note(args: &Value) -> ToolResult {
    let path = str_arg(args, "path");
    if path.is_empty() {
        return Err("read_knowledge_note requires a non-empty path.".into());
    }
    let cfg = KnowledgeConfig::resolve();
    let max_chars = match args.get("max_chars") {
        Some(v) if !v.is_null() => to_positive_int(v, 0, 500, 100_000) as usize,
        _ => 0,
    };
    let note = read_knowledge_note(&cfg, &path, max_chars)?;
    Ok(vec![text(format_note(&note))])
}

/// Handle `write_knowledge_note`: create or overwrite a note.
pub fn run_write_note(args: &Value) -> ToolResult {
    let r = write_knowledge_note(&KnowledgeConfig::resolve(), args)?;
    Ok(vec![text(format_write(&r))])
}

/// Handle `update_knowledge_note`: replace a section identified by heading.
pub fn run_update_note(args: &Value) -> ToolResult {
    let r = update_knowledge_note(&KnowledgeConfig::resolve(), args)?;
    Ok(vec![text(format_write(&r))])
}

/// Handle `append_to_knowledge_note`: append content to an existing note.
pub fn run_append_note(args: &Value) -> ToolResult {
    let r = append_to_knowledge_note(&KnowledgeConfig::resolve(), args)?;
    Ok(vec![text(format_write(&r))])
}

/// Handle `submit_community_research`: publish a note to the shared base.
pub fn run_submit(args: &Value) -> ToolResult {
    let (path, output) = submit_research(&KnowledgeConfig::resolve(), args)?;
    let mut lines = vec!["Action: submitted".to_string(), format!("Path: {path}")];
    if !output.is_empty() {
        lines.push(format!("Output: {output}"));
    }
    Ok(vec![text(lines.join("\n"))])
}

fn str_arg(args: &Value, key: &str) -> String {
    args.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

// ─── formatters ─────────────────────────────────────────────────────────────

fn format_build(r: &BuildResult) -> String {
    format!(
        "Action: built\nFiles indexed: {}\nUnique terms: {}\nIndex path: {}",
        r.file_count, r.term_count, r.path
    )
}

fn format_index_search(r: &IndexSearch) -> String {
    let src_label = [
        r.local.then_some("local"),
        r.community.then_some("community"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" + ");
    let mut lines = vec![
        format!("Query: {}", r.query),
        format!(
            "Index: {} · {} candidates · sources: {}",
            r.built_at.clone().unwrap_or_else(|| "no index".into()),
            r.total_results,
            if src_label.is_empty() {
                "none".to_string()
            } else {
                src_label
            }
        ),
        String::new(),
        "Results:".to_string(),
    ];
    for (i, h) in r.results.iter().enumerate() {
        lines.push(format!("{}. {}", i + 1, h.title));
        lines.push(format!("   Path: {}", h.path));
        lines.push(format!(
            "   Score: {}  |  Source: {}  |  Terms: {}",
            fmt_num(h.score),
            h.source,
            h.top_terms.join(", ")
        ));
        if !h.related.is_empty() {
            lines.push(format!("   Related: {}", h.related.join(", ")));
        }
        if !h.snippet.is_empty() {
            lines.push(format!("   Snippet: {}", h.snippet));
        }
    }
    if r.results.is_empty() {
        lines.push("No matching knowledge notes found.".to_string());
    }
    lines.join("\n")
}

fn format_cache_search(r: &CacheSearch) -> String {
    let mut lines = vec![
        format!("Query: {}", r.query),
        "Knowledge root: knowledge".to_string(),
        format!("Total results: {}", r.total),
        String::new(),
        "Results:".to_string(),
    ];
    for item in &r.results {
        lines.push(format!("{}. {}", item.rank, item.title));
        lines.push(format!("   Path: {}", item.path));
        lines.push(format!("   Source: {}", item.source));
        if !item.snippet.is_empty() {
            lines.push(format!("   Snippet: {}", item.snippet));
        }
    }
    if r.results.is_empty() {
        lines.push("No cached knowledge notes matched.".to_string());
    }
    lines.join("\n")
}

fn format_note(r: &NoteResult) -> String {
    format!(
        "Title: {}\nPath: {}\n\n{}",
        r.title,
        r.path,
        if r.text.is_empty() {
            "No text available."
        } else {
            &r.text
        }
    )
}

fn format_write(r: &WriteResult) -> String {
    let mut lines = vec![format!("Action: {}", r.action), format!("Path: {}", r.path)];
    if let Some(h) = &r.heading {
        lines.push(format!("Heading: {h}"));
    }
    match &r.index {
        IndexStatus::Rebuilt {
            file_count,
            term_count,
            path,
        } => {
            lines.push(format!(
                "Index: rebuilt ({file_count} files, {term_count} terms)"
            ));
            lines.push(format!("Index path: {path}"));
        }
        IndexStatus::Failed(msg) => {
            lines.push("Index: failed".to_string());
            lines.push(format!("Index detail: {msg}"));
        }
    }
    lines.push(format!("Publish: {}", r.publish.status));
    if let Some(m) = &r.publish.message {
        lines.push(format!("Publish detail: {m}"));
    }
    if let Some(o) = &r.publish.output {
        lines.push(format!("Publish detail: {o}"));
    }
    lines.join("\n")
}

/// Render a number like JS: integers without a decimal point.
fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

// ─── schemas ────────────────────────────────────────────────────────────────

/// MCP tool schema for `search_knowledge_cache`.
pub fn schema_search_cache() -> Value {
    json!({
        "name": "search_knowledge_cache",
        "description": "Search the durable knowledge cache and return matching note paths with snippets. Searches both the local workspace (.github/knowledge/) and the community knowledge base (knowledge/ in the repo or fetched from GitHub).",
        "inputSchema": { "type": "object", "properties": {
            "query": { "type": "string", "description": "Search query." },
            "max_results": { "type": "integer", "description": "Number of note matches to return (1-20)." }
        }, "required": ["query"] }
    })
}

/// MCP tool schema for `read_knowledge_note`.
pub fn schema_read_note() -> Value {
    json!({
        "name": "read_knowledge_note",
        "description": "Read a specific knowledge note. Pass a bare filename (e.g. networking-dns.md) or a workspace-relative path. Resolves from workspace knowledge root, then repo bundled, then GitHub community.",
        "inputSchema": { "type": "object", "properties": {
            "path": { "type": "string", "description": "Filename (e.g. networking-dns.md) or workspace-relative path. Bare filenames resolve to the detected knowledge root." },
            "max_chars": { "type": "integer", "description": "Optional. Maximum characters to return (500-100000). Default: no limit (full content)." }
        }, "required": ["path"] }
    })
}

/// MCP tool schema for `write_knowledge_note`.
pub fn schema_write_note() -> Value {
    json!({
        "name": "write_knowledge_note",
        "description": "Create or overwrite a knowledge note. Writes to the workspace's knowledge directory (auto-detected: knowledge/ in source repo, .github/knowledge/ elsewhere), rebuilds the local index before returning, and can optionally publish to the shared knowledge base when publish=true and sharing is enabled.",
        "inputSchema": { "type": "object", "properties": {
            "path": { "type": "string", "description": "Filename for the note, e.g. networking-dns.md. Can also be a workspace-relative path. The tool places bare filenames in the detected knowledge root automatically." },
            "content": { "type": "string", "description": "Full markdown content to write." },
            "overwrite": { "type": "boolean", "description": "Set to true to replace an existing file. Default false (fails if file exists)." },
            "publish": { "type": "boolean", "description": "When true, submit the note to the shared knowledge base after the local index rebuild succeeds. Requires shareKnowledge (or legacy shareResearch) to be enabled in community settings." }
        }, "required": ["path", "content"] }
    })
}

/// MCP tool schema for `update_knowledge_note`.
pub fn schema_update_note() -> Value {
    json!({
        "name": "update_knowledge_note",
        "description": "Replace a specific section (identified by heading) in an existing knowledge note. Preserves all other sections, rebuilds the local index before returning, and can optionally publish the updated note.",
        "inputSchema": { "type": "object", "properties": {
            "path": { "type": "string", "description": "Filename (e.g. networking-dns.md) or workspace-relative path to the knowledge note." },
            "heading": { "type": "string", "description": "Exact text of the heading to replace (without the # prefix)." },
            "content": { "type": "string", "description": "New content to place under the heading. The heading line is preserved; only the body below it is replaced." },
            "publish": { "type": "boolean", "description": "When true, submit the updated note to the shared knowledge base after the local index rebuild succeeds." }
        }, "required": ["path", "heading", "content"] }
    })
}

/// MCP tool schema for `append_to_knowledge_note`.
pub fn schema_append_note() -> Value {
    json!({
        "name": "append_to_knowledge_note",
        "description": "Append content to the end of an existing knowledge note. Rebuilds the local index before returning and can optionally publish the updated note when the appended content is shareable.",
        "inputSchema": { "type": "object", "properties": {
            "path": { "type": "string", "description": "Filename (e.g. networking-dns.md) or workspace-relative path to the knowledge note." },
            "content": { "type": "string", "description": "Markdown content to append at the end of the file." },
            "publish": { "type": "boolean", "description": "When true, submit the updated note to the shared knowledge base after the local index rebuild succeeds." }
        }, "required": ["path", "content"] }
    })
}

/// MCP tool schema for `submit_community_research`.
pub fn schema_submit() -> Value {
    json!({
        "name": "submit_community_research",
        "description": "Submit a knowledge note to the shared knowledge base as a pull request. Requires knowledge sharing to be enabled in settings. The note content is validated for privacy and the submission rebuilds knowledge/_index.json so the published cache stays searchable.",
        "inputSchema": { "type": "object", "properties": {
            "path": { "type": "string", "description": "Path to the knowledge note to submit." }
        }, "required": ["path"] }
    })
}

/// MCP tool schema for `build_knowledge_index`.
pub fn schema_build_index() -> Value {
    json!({
        "name": "build_knowledge_index",
        "description": "Build or rebuild the local workspace TF-IDF search index (_index.json) from knowledge files. The community knowledge index is pre-built on GitHub and fetched automatically — this tool only rebuilds the local workspace index. Run manually after bulk additions; also rebuilt automatically after write/update/append operations.",
        "inputSchema": { "type": "object", "properties": {}, "required": [] }
    })
}

/// MCP tool schema for `search_knowledge_index`.
pub fn schema_search_index() -> Value {
    json!({
        "name": "search_knowledge_index",
        "description": "Search the knowledge base using TF-IDF indexes. Merges results from the local workspace index (if built) and the community knowledge index (pre-built on GitHub, fetched with ETag caching). Returns ranked results with relevance scores, source tags (local/community), related files, and text snippets. Falls back to keyword search if no index is available.",
        "inputSchema": { "type": "object", "properties": {
            "query": { "type": "string", "description": "Search query." },
            "max_results": { "type": "integer", "description": "Number of results to return (1-20). Default: 5." }
        }, "required": ["query"] }
    })
}
