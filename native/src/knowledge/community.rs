//! GitHub-hosted community knowledge: ETag-cached fetch (via `curl`) of the
//! pre-built index and individual notes, plus the publish/submit path (which
//! shells out to the existing `community-research-submit.sh`).

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::knowledge::{KnowledgeConfig, GITHUB_RAW_BASE, INDEX_MAX_AGE_MS};
use crate::util::now_millis;

#[derive(Serialize, Deserialize, Default)]
struct CacheMeta {
    #[serde(default)]
    etags: HashMap<String, String>,
    #[serde(default)]
    fetched_at: HashMap<String, i64>,
}

fn load_cache_meta(cfg: &KnowledgeConfig) -> CacheMeta {
    std::fs::read_to_string(&cfg.cache_meta_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_cache_meta(cfg: &KnowledgeConfig, meta: &CacheMeta) {
    if let Ok(json) = serde_json::to_string(meta) {
        let _ = std::fs::create_dir_all(&cfg.github_cache_dir);
        let _ = std::fs::write(&cfg.cache_meta_path, json);
    }
}

fn cache_file(cfg: &KnowledgeConfig, repo_path: &str) -> std::path::PathBuf {
    cfg.github_cache_dir.join(repo_path.replace('/', "__"))
}

fn is_cache_fresh(cfg: &KnowledgeConfig, key: &str) -> bool {
    load_cache_meta(cfg)
        .fetched_at
        .get(key)
        .map(|ts| now_millis() - ts < INDEX_MAX_AGE_MS)
        .unwrap_or(false)
}

/// Fetch a file from the community repo's raw content with ETag caching.
/// Returns the text (from network or the local cache on 304/failure).
pub fn fetch_github_file(cfg: &KnowledgeConfig, repo_path: &str) -> Result<String, String> {
    let mut meta = load_cache_meta(cfg);
    let cache = cache_file(cfg, repo_path);
    let url = format!("{GITHUB_RAW_BASE}/{repo_path}");
    let _ = std::fs::create_dir_all(&cfg.github_cache_dir);

    let body_tmp = cfg
        .github_cache_dir
        .join(format!(".dl-{}", std::process::id()));
    let headers_tmp = cfg
        .github_cache_dir
        .join(format!(".hd-{}", std::process::id()));

    let mut args: Vec<String> = vec![
        "-sS".into(),
        "--max-time".into(),
        "15".into(),
        "-A".into(),
        "gsh-mcp/1.0".into(),
        "-D".into(),
        headers_tmp.to_string_lossy().to_string(),
        "-o".into(),
        body_tmp.to_string_lossy().to_string(),
        "-w".into(),
        "%{http_code}".into(),
    ];
    if let Some(etag) = meta.etags.get(repo_path) {
        args.push("-H".into());
        args.push(format!("If-None-Match: {etag}"));
    }
    args.push(url);

    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = Command::new("curl").args(&arg_refs).output();

    let result = (|| -> Result<String, String> {
        let output = output.map_err(|e| format!("curl failed to start: {e}"))?;
        let code = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if code == "304" {
            meta.fetched_at.insert(repo_path.to_string(), now_millis());
            save_cache_meta(cfg, &meta);
            return std::fs::read_to_string(&cache).map_err(|e| e.to_string());
        }
        if !code.starts_with('2') {
            return Err(format!("HTTP {code} fetching {repo_path}"));
        }
        let text = std::fs::read_to_string(&body_tmp).map_err(|e| e.to_string())?;
        if let Some(etag) = parse_etag(&headers_tmp) {
            meta.etags.insert(repo_path.to_string(), etag);
        }
        meta.fetched_at.insert(repo_path.to_string(), now_millis());
        save_cache_meta(cfg, &meta);
        std::fs::write(&cache, &text).map_err(|e| e.to_string())?;
        Ok(text)
    })();

    let _ = std::fs::remove_file(&body_tmp);
    let _ = std::fs::remove_file(&headers_tmp);

    // Network failure → fall back to any cached copy.
    result.or_else(|e| std::fs::read_to_string(&cache).map_err(|_| e))
}

fn parse_etag(headers_file: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(headers_file).ok()?;
    for line in raw.lines() {
        if let Some((key, value)) = line.split_once(':') {
            if key.trim().eq_ignore_ascii_case("etag") {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// Fetch and parse the community `_index.json`, honoring the freshness window.
pub fn fetch_community_index(cfg: &KnowledgeConfig) -> Result<Value, String> {
    let key = "knowledge/_index.json";
    if is_cache_fresh(cfg, key) {
        if let Ok(raw) = std::fs::read_to_string(cache_file(cfg, key)) {
            if let Ok(v) = serde_json::from_str(&raw) {
                return Ok(v);
            }
        }
    }
    let text = fetch_github_file(cfg, key)?;
    serde_json::from_str(&text).map_err(|e| format!("invalid community index: {e}"))
}

/// Submit a note to the shared knowledge base via the existing shell script.
pub fn submit_community_research(
    cfg: &KnowledgeConfig,
    resolved_path: &Path,
) -> Result<String, String> {
    let script = cfg
        .repo_root
        .join("scripts")
        .join("community-research-submit.sh");
    let output = Command::new("bash")
        .arg(&script)
        .arg(resolved_path)
        .current_dir(&cfg.workspace_root)
        .output()
        .map_err(|e| format!("Community research submit failed: {e}"))?;
    if !output.status.success() {
        let msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Community research submit failed: {}", msg.trim()));
    }
    let out = String::from_utf8_lossy(&output.stdout);
    let err = String::from_utf8_lossy(&output.stderr);
    Ok(if !err.trim().is_empty() {
        err.trim().to_string()
    } else {
        out.trim().to_string()
    })
}
