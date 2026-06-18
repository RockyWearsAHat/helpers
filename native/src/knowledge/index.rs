//! Local TF-IDF knowledge index + search — port of `lib/mcp-knowledge-index.js`.
//! Preserves the on-disk `_index.json` schema (a contract with the GitHub
//! community index).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::knowledge::community::fetch_community_index;
use crate::knowledge::KnowledgeConfig;
use crate::tfidf::{self, Vector};
use crate::util::{get_markdown_title, now_iso, summarize_text};

const STOPWORDS: &[&str] = &[
    "a", "about", "above", "after", "again", "all", "also", "am", "an", "and", "any", "are", "as",
    "at", "be", "because", "been", "before", "being", "between", "both", "but", "by", "can",
    "could", "did", "do", "does", "doing", "down", "during", "each", "few", "for", "from",
    "further", "get", "got", "had", "has", "have", "having", "he", "her", "here", "him", "his",
    "how", "if", "in", "into", "is", "it", "its", "itself", "just", "let", "me", "more", "most",
    "must", "my", "new", "no", "nor", "not", "now", "of", "off", "on", "once", "only", "or",
    "other", "our", "out", "over", "own", "same", "she", "should", "so", "some", "such", "than",
    "that", "the", "their", "them", "then", "there", "these", "they", "this", "those", "through",
    "to", "too", "under", "until", "up", "use", "used", "using", "very", "via", "was", "we",
    "were", "what", "when", "where", "which", "while", "who", "will", "with", "would", "you",
    "your",
];

fn stopwords() -> &'static HashSet<&'static str> {
    static SW: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SW.get_or_init(|| STOPWORDS.iter().copied().collect())
}

fn doc_cleaner() -> &'static [Regex; 5] {
    static RE: OnceLock<[Regex; 5]> = OnceLock::new();
    RE.get_or_init(|| {
        [
            Regex::new(r"(?s)```.*?```").unwrap(),
            Regex::new(r"`[^`\n]+`").unwrap(),
            Regex::new(r"https?://\S+").unwrap(),
            Regex::new(r"[#*_\[\]()>|~^=+]").unwrap(),
            Regex::new(r"\b\d+\b").unwrap(),
        ]
    })
}

/// Port of `tokenizeDocText`: strip code/urls/markdown/numbers, split on
/// non-letters, keep tokens of length >= 3 that aren't stopwords.
pub fn tokenize_doc_text(text: &str) -> Vec<String> {
    let cleaners = doc_cleaner();
    let mut cleaned = cleaners[0].replace_all(text, " ").into_owned();
    for re in &cleaners[1..] {
        cleaned = re.replace_all(&cleaned, " ").into_owned();
    }
    cleaned
        .to_lowercase()
        .split(|c: char| !c.is_ascii_lowercase())
        .filter(|t| t.len() >= 3 && !stopwords().contains(*t))
        .map(str::to_string)
        .collect()
}

/// Recursively collect `.md` files under `root`.
pub fn collect_markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|e| e.to_str()) == Some("md") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// Build a snippet around the first query term, or a summary if none match.
pub fn build_knowledge_snippet(text: &str, terms: &[String]) -> String {
    let compact: String = {
        static WS: OnceLock<Regex> = OnceLock::new();
        WS.get_or_init(|| Regex::new(r"\s+").unwrap())
            .replace_all(text, " ")
            .trim()
            .to_string()
    };
    if compact.is_empty() {
        return "No extractable text.".to_string();
    }
    let lower = compact.to_lowercase();
    let chars: Vec<char> = compact.chars().collect();
    let lower_chars: Vec<char> = lower.chars().collect();
    let mut first: Option<usize> = None;
    for term in terms {
        if let Some(i) = find_char_index(&lower_chars, term) {
            first = Some(first.map_or(i, |f| f.min(i)));
        }
    }
    match first {
        None => summarize_text(&compact, 220),
        Some(idx) => {
            let start = idx.saturating_sub(80);
            let end = (idx + 180).min(chars.len());
            let prefix = if start > 0 { "..." } else { "" };
            let suffix = if end < chars.len() { "..." } else { "" };
            let mid: String = chars[start..end].iter().collect();
            format!("{prefix}{}{suffix}", mid.trim())
        }
    }
}

fn find_char_index(haystack: &[char], needle: &str) -> Option<usize> {
    let n: Vec<char> = needle.chars().collect();
    if n.is_empty() || n.len() > haystack.len() {
        return None;
    }
    (0..=haystack.len() - n.len()).find(|&i| haystack[i..i + n.len()] == n[..])
}

/// Keyword match score for the cache search: title×10 + path×6 + body×2.
pub fn score_knowledge_match(rel_path: &str, title: &str, body: &str, terms: &[String]) -> i64 {
    let (p, t, b) = (
        rel_path.to_lowercase(),
        title.to_lowercase(),
        body.to_lowercase(),
    );
    let mut score = 0i64;
    for term in terms {
        score += t.matches(term.as_str()).count() as i64 * 10;
        score += p.matches(term.as_str()).count() as i64 * 6;
        score += b.matches(term.as_str()).count() as i64 * 2;
    }
    score
}

// ─── On-disk index schema ───────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct FileInfo {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub top_terms: Vec<String>,
    // The JS builder occasionally emits null weights; drop them on read.
    #[serde(default, deserialize_with = "de_f64_map")]
    pub norm_vec: HashMap<String, f64>,
    #[serde(default)]
    pub related: Vec<String>,
}

/// Deserialize a `{term: number}` map, silently dropping null / non-numeric
/// values so a JS-produced index with stray nulls still parses.
fn de_f64_map<'de, D>(d: D) -> Result<HashMap<String, f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: HashMap<String, Option<f64>> = HashMap::deserialize(d)?;
    Ok(raw
        .into_iter()
        .filter_map(|(k, v)| v.map(|x| (k, x)))
        .collect())
}

#[derive(Serialize, Deserialize, Default)]
pub struct KnowledgeIndex {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub built_at: String,
    #[serde(default)]
    pub file_count: usize,
    #[serde(default, deserialize_with = "de_f64_map")]
    pub idf: HashMap<String, f64>,
    #[serde(default)]
    pub files: HashMap<String, FileInfo>,
    #[serde(default)]
    pub posting: HashMap<String, Vec<String>>,
}

pub struct BuildResult {
    pub path: String,
    pub file_count: usize,
    pub term_count: usize,
}

/// Build the local workspace TF-IDF index and write `_index.json`.
pub fn build_knowledge_index(cfg: &KnowledgeConfig) -> Result<BuildResult, String> {
    let entries = std::fs::read_dir(&cfg.knowledge_root).map_err(|e| {
        format!(
            "Cannot read workspace knowledge directory ({}): {e}",
            cfg.knowledge_root.display()
        )
    })?;
    let mut md_files: Vec<String> = entries
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .filter(|n| n.ends_with(".md") && !n.starts_with('_'))
        .collect();
    md_files.sort();
    if md_files.is_empty() {
        return Err("No markdown files found in workspace knowledge directory.".into());
    }

    struct Doc {
        filename: String,
        title: String,
        tf: Vector,
    }
    let mut docs = Vec::new();
    for filename in &md_files {
        let text = std::fs::read_to_string(cfg.knowledge_root.join(filename)).unwrap_or_default();
        let title = get_markdown_title(&text, filename.trim_end_matches(".md"));
        let tf = tfidf::compute_tf(&tokenize_doc_text(&text));
        docs.push(Doc {
            filename: filename.clone(),
            title,
            tf,
        });
    }

    let idf = tfidf::compute_idf(
        &docs.iter().map(|d| d.tf.clone()).collect::<Vec<_>>(),
        docs.len(),
    );

    let mut files: HashMap<String, FileInfo> = HashMap::new();
    let mut posting: HashMap<String, Vec<String>> = HashMap::new();
    for doc in &docs {
        let weighted = tfidf::tfidf(&doc.tf, &idf);
        let sorted = tfidf::top_n(&weighted, weighted.len());
        let top_terms: Vec<String> = sorted.iter().take(15).map(|(t, _)| t.clone()).collect();
        let sparse: Vector = sorted.into_iter().take(120).collect();
        let norm_vec = tfidf::l2_normalize(&sparse);
        for term in norm_vec.keys() {
            posting
                .entry(term.clone())
                .or_default()
                .push(doc.filename.clone());
        }
        files.insert(
            doc.filename.clone(),
            FileInfo {
                title: doc.title.clone(),
                top_terms,
                norm_vec,
                related: Vec::new(),
            },
        );
    }

    // Precompute related files via pairwise cosine (top 5, sim > 0.03).
    let names: Vec<String> = {
        let mut v: Vec<String> = files.keys().cloned().collect();
        v.sort();
        v
    };
    for a in &names {
        let vec_a = files[a].norm_vec.clone();
        let mut sims: Vec<(String, f64)> = Vec::new();
        for b in &names {
            if a == b {
                continue;
            }
            let s = tfidf::cosine_sim(&vec_a, &files[b].norm_vec);
            if s > 0.03 {
                sims.push((b.clone(), s));
            }
        }
        sims.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal));
        files.get_mut(a).unwrap().related = sims.into_iter().take(5).map(|(n, _)| n).collect();
    }

    let term_count = idf.len();
    let index = KnowledgeIndex {
        version: 1,
        built_at: now_iso(),
        file_count: docs.len(),
        idf,
        files,
        posting,
    };
    let json = serde_json::to_string_pretty(&index).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&cfg.knowledge_root).map_err(|e| e.to_string())?;
    std::fs::write(&cfg.local_index_path, json).map_err(|e| e.to_string())?;

    Ok(BuildResult {
        path: rel(&cfg.workspace_root, &cfg.local_index_path),
        file_count: docs.len(),
        term_count,
    })
}

fn rel(base: &Path, p: &Path) -> String {
    p.strip_prefix(base)
        .unwrap_or(p)
        .to_string_lossy()
        .to_string()
}

// ─── Search ─────────────────────────────────────────────────────────────────

pub struct IndexHit {
    pub path: String,
    pub title: String,
    pub score: f64,
    pub top_terms: Vec<String>,
    pub related: Vec<String>,
    pub source: &'static str,
    pub snippet: String,
}

pub struct IndexSearch {
    pub query: String,
    pub local: bool,
    pub community: bool,
    pub built_at: Option<String>,
    pub total_results: usize,
    pub results: Vec<IndexHit>,
}

/// Search local + community indexes. Returns `None` when neither index exists
/// (the caller then falls back to the keyword cache search).
pub fn search_knowledge_index(
    cfg: &KnowledgeConfig,
    query: &str,
    max_results: usize,
    read_file: &dyn Fn(&str) -> Option<String>,
) -> Option<IndexSearch> {
    let local: Option<KnowledgeIndex> = std::fs::read_to_string(&cfg.local_index_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());
    let community: Option<KnowledgeIndex> = fetch_community_index(cfg)
        .ok()
        .and_then(|v| serde_json::from_value(v).ok());

    if local.is_none() && community.is_none() {
        return None;
    }

    // Query terms: doc tokenizer ∪ light split (len >= 2).
    let mut query_terms: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    for t in tokenize_doc_text(query) {
        if seen.insert(t.clone()) {
            query_terms.push(t);
        }
    }
    for t in query
        .to_lowercase()
        .split(|c: char| !c.is_ascii_lowercase())
        .filter(|t| t.len() >= 2 && !stopwords().contains(*t))
    {
        if seen.insert(t.to_string()) {
            query_terms.push(t.to_string());
        }
    }
    if query_terms.is_empty() {
        return None;
    }

    // filename -> (source, &index). Local wins on overlap.
    let mut cand_source: HashMap<String, &'static str> = HashMap::new();
    let indexes: [(Option<&KnowledgeIndex>, &'static str); 2] =
        [(local.as_ref(), "local"), (community.as_ref(), "community")];

    let gather = |idx: Option<&KnowledgeIndex>,
                  source: &'static str,
                  cand: &mut HashMap<String, &'static str>,
                  broad: bool| {
        let idx = match idx {
            Some(i) => i,
            None => return,
        };
        for q in &query_terms {
            if let Some(files) = idx.posting.get(q) {
                for f in files {
                    cand.entry(f.clone())
                        .and_modify(|s| {
                            if source == "local" {
                                *s = source
                            }
                        })
                        .or_insert(source);
                }
            }
            for (term, files) in &idx.posting {
                let hit = if broad {
                    term.contains(q.as_str())
                } else {
                    q.len() >= 3
                        && term != q
                        && (term.starts_with(q.as_str())
                            || (q.len() >= 5 && term.contains(q.as_str())))
                };
                if hit {
                    for f in files {
                        cand.entry(f.clone())
                            .and_modify(|s| {
                                if source == "local" {
                                    *s = source
                                }
                            })
                            .or_insert(source);
                    }
                }
            }
        }
    };

    for (idx, src) in indexes {
        gather(idx, src, &mut cand_source, false);
    }
    if cand_source.is_empty() {
        for (idx, src) in indexes {
            gather(idx, src, &mut cand_source, true);
        }
    }

    let index_for = |src: &str| -> Option<&KnowledgeIndex> {
        if src == "local" {
            local.as_ref()
        } else {
            community.as_ref()
        }
    };

    let mut scored: Vec<(String, f64, &'static str)> = Vec::new();
    for (filename, source) in &cand_source {
        let idx = match index_for(source) {
            Some(i) => i,
            None => continue,
        };
        let info = match idx.files.get(filename) {
            Some(f) => f,
            None => continue,
        };
        let mut score = 0.0;
        let title_lower = info.title.to_lowercase();
        for term in &query_terms {
            if let Some(v) = info.norm_vec.get(term) {
                score += v;
            }
            if title_lower.contains(term.as_str()) {
                score += 0.4;
            }
        }
        if *source == "local" {
            score *= 1.15;
        }
        if score > 0.0 {
            scored.push((filename.clone(), score, source));
        }
    }
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| (a.2 == "local").cmp(&(b.2 == "local")).reverse())
            .then_with(|| a.0.cmp(&b.0))
    });

    let total = scored.len();
    let results = scored
        .iter()
        .take(max_results)
        .map(|(filename, score, source)| {
            let info = index_for(source)
                .and_then(|i| i.files.get(filename))
                .cloned()
                .unwrap_or_default();
            let snippet = read_file(filename)
                .map(|body| build_knowledge_snippet(&body, &query_terms))
                .unwrap_or_default();
            IndexHit {
                path: format!("knowledge/{filename}"),
                title: info.title,
                score: (score * 1000.0).round() / 1000.0,
                top_terms: info.top_terms.into_iter().take(8).collect(),
                related: info.related,
                source,
                snippet,
            }
        })
        .collect();

    let built_at = local
        .as_ref()
        .map(|i| i.built_at.clone())
        .or_else(|| community.as_ref().map(|i| i.built_at.clone()));

    Some(IndexSearch {
        query: query.to_string(),
        local: local.is_some(),
        community: community.is_some(),
        built_at,
        total_results: total,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_and_strips_markdown() {
        let toks = tokenize_doc_text(
            "# Title\n```\ncode here\n```\nThe `quick` brown http://x.com fox 123 jumps",
        );
        assert!(toks.contains(&"title".to_string()));
        assert!(toks.contains(&"brown".to_string()));
        assert!(toks.contains(&"jumps".to_string()));
        assert!(!toks.iter().any(|t| t == "code")); // inside fenced block
        assert!(!toks.iter().any(|t| t == "quick")); // inline code
        assert!(!toks.iter().any(|t| t == "the")); // stopword
    }

    #[test]
    fn snippet_centers_on_term() {
        let body = "alpha beta gamma delta epsilon zeta eta theta needle iota kappa lambda";
        let s = build_knowledge_snippet(body, &["needle".to_string()]);
        assert!(s.contains("needle"));
    }

    #[test]
    fn match_score_weights_title_over_body() {
        let s = score_knowledge_match(
            "p.md",
            "needle title",
            "needle body needle",
            &["needle".to_string()],
        );
        // title(10) + body(2*2) = 14
        assert_eq!(s, 14);
    }
}
