//! Language detection + tree-sitter tag extraction.
//!
//! Tags are tree-sitter's symbol model: `is_definition` tags are declarations
//! (functions, classes, structs, …) and the rest are references (calls, type
//! uses). This is the same signal aider's repo-map uses to rank and connect
//! files. Configurations are compiled once per language and reused across files.

use std::collections::HashMap;

use tree_sitter_tags::{TagsConfiguration, TagsContext};

/// A single extracted tag (definition or reference).
pub struct RawTag {
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub is_def: bool,
}

/// Map a file extension to a language id, or `None` if unsupported by tags.
pub fn lang_for_ext(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "rs" => "rust",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "py" | "pyi" => "python",
        "go" => "go",
        _ => return None,
    })
}

fn build_config(lang: &str) -> Option<TagsConfiguration> {
    let (language, query): (tree_sitter::Language, &str) = match lang {
        "rust" => (
            tree_sitter_rust::LANGUAGE.into(),
            tree_sitter_rust::TAGS_QUERY,
        ),
        "javascript" => (
            tree_sitter_javascript::LANGUAGE.into(),
            tree_sitter_javascript::TAGS_QUERY,
        ),
        "typescript" => (
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            tree_sitter_typescript::TAGS_QUERY,
        ),
        "tsx" => (
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            tree_sitter_typescript::TAGS_QUERY,
        ),
        "python" => (
            tree_sitter_python::LANGUAGE.into(),
            tree_sitter_python::TAGS_QUERY,
        ),
        "go" => (tree_sitter_go::LANGUAGE.into(), tree_sitter_go::TAGS_QUERY),
        _ => return None,
    };
    TagsConfiguration::new(language, query, "").ok()
}

/// Caches one compiled `TagsConfiguration` per language for reuse across files.
#[derive(Default)]
pub struct TagExtractor {
    ctx: TagsContext,
    configs: HashMap<String, Option<TagsConfiguration>>,
}

impl TagExtractor {
    pub fn new() -> Self {
        Self {
            ctx: TagsContext::new(),
            configs: HashMap::new(),
        }
    }

    /// Extract tags from `source` for the given language id. Returns an empty
    /// vec for unsupported languages or parse failures (best-effort indexing).
    pub fn extract(&mut self, lang: &str, source: &[u8]) -> Vec<RawTag> {
        let config = self
            .configs
            .entry(lang.to_string())
            .or_insert_with(|| build_config(lang));
        let config = match config {
            Some(c) => c,
            None => return Vec::new(),
        };
        let (tags, _) = match self.ctx.generate_tags(config, source, None) {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };
        let mut out = Vec::new();
        for tag in tags.flatten() {
            let name = String::from_utf8_lossy(&source[tag.name_range.clone()]).to_string();
            if name.is_empty() {
                continue;
            }
            let kind = config.syntax_type_name(tag.syntax_type_id).to_string();
            out.push(RawTag {
                name,
                kind,
                line: tag.span.start.row + 1,
                is_def: tag.is_definition,
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_rust_definitions() {
        let src = b"pub fn alpha() {}\nstruct Beta;\nfn beta_user() { alpha(); }\n";
        let mut ex = TagExtractor::new();
        let tags = ex.extract("rust", src);
        let defs: Vec<_> = tags
            .iter()
            .filter(|t| t.is_def)
            .map(|t| t.name.as_str())
            .collect();
        assert!(defs.contains(&"alpha"), "definitions: {defs:?}");
        assert!(defs.contains(&"Beta"), "definitions: {defs:?}");
        let refs: Vec<_> = tags
            .iter()
            .filter(|t| !t.is_def)
            .map(|t| t.name.as_str())
            .collect();
        assert!(refs.contains(&"alpha"), "references: {refs:?}");
    }

    #[test]
    fn extracts_javascript_definitions() {
        let src = b"export function alpha() {}\nclass Beta {}\nalpha();\n";
        let mut ex = TagExtractor::new();
        let tags = ex.extract("javascript", src);
        let defs: Vec<_> = tags
            .iter()
            .filter(|t| t.is_def)
            .map(|t| t.name.as_str())
            .collect();
        assert!(defs.contains(&"alpha"), "definitions: {defs:?}");
        assert!(defs.contains(&"Beta"), "definitions: {defs:?}");
    }
}
