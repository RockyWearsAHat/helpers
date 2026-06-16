//! Copilot model resolution — port of `lib/mcp-model-utils.js`. Reads
//! `~/.copilot/available-models.json` and resolves a user-supplied name/id to a
//! canonical model id, or picks the cheapest available model.

use serde::Deserialize;

use crate::git::home;

/// Cheapest models suitable for commit-message generation, in preference order.
const CHEAP_MODEL_PREFERENCE: &[&str] = &[
    "gpt-5.4-mini",
    "gpt-5-mini",
    "claude-haiku-4.5",
    "gpt-4o-mini",
    "copilot-fast",
    "gpt-4.1",
    "qwen3:4b",
];

#[derive(Deserialize, Default)]
pub struct Model {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "qualifiedName", default)]
    pub qualified_name: Option<String>,
}

#[derive(Deserialize, Default)]
struct ModelsFile {
    #[serde(default)]
    models: Vec<Model>,
}

/// Read `~/.copilot/available-models.json`; empty vec if absent/malformed.
pub fn load_available_models() -> Vec<Model> {
    let path = home().join(".copilot").join("available-models.json");
    let raw = match std::fs::read_to_string(path) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str::<ModelsFile>(&raw)
        .map(|f| f.models)
        .unwrap_or_default()
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| {
            !matches!(
                c,
                ' ' | '\t' | '-' | '_' | '.' | ':' | '(' | ')' | '[' | ']'
            )
        })
        .collect()
}

/// Resolve `input` to a canonical model id, mirroring the JS matching order
/// (exact id/name/qualifiedName, then normalized fuzzy, then prefix substring).
pub fn resolve_model_id(input: &str, models: &[Model]) -> Option<String> {
    let inp = input.trim();
    if inp.is_empty() || models.is_empty() {
        return None;
    }
    let inp_lower = inp.to_lowercase();
    let inp_norm = normalize(inp);

    if let Some(m) = models.iter().find(|m| m.id == inp) {
        return Some(m.id.clone());
    }
    if let Some(m) = models
        .iter()
        .find(|m| m.name.as_deref().map(|n| n.to_lowercase()) == Some(inp_lower.clone()))
    {
        return Some(m.id.clone());
    }
    if let Some(m) = models
        .iter()
        .find(|m| m.qualified_name.as_deref().map(|n| n.to_lowercase()) == Some(inp_lower.clone()))
    {
        return Some(m.id.clone());
    }
    if let Some(m) = models.iter().find(|m| {
        normalize(&m.id) == inp_norm
            || m.name.as_deref().map(normalize) == Some(inp_norm.clone())
            || m.qualified_name.as_deref().map(normalize) == Some(inp_norm.clone())
    }) {
        return Some(m.id.clone());
    }
    if let Some(m) = models.iter().find(|m| {
        normalize(&m.id).contains(&inp_norm)
            || m.name
                .as_deref()
                .map(|n| normalize(n).contains(&inp_norm))
                .unwrap_or(false)
    }) {
        return Some(m.id.clone());
    }
    None
}

/// The cheapest available preferred model id, defaulting to `gpt-4o-mini`.
pub fn detect_cheap_model(models: &[Model]) -> String {
    for preferred in CHEAP_MODEL_PREFERENCE {
        if models.iter().any(|m| m.id == *preferred) {
            return (*preferred).to_string();
        }
    }
    "gpt-4o-mini".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(id: &str, name: &str) -> Model {
        Model {
            id: id.into(),
            name: Some(name.into()),
            qualified_name: None,
        }
    }

    #[test]
    fn resolves_by_id_name_and_fuzzy() {
        let models = vec![
            m("claude-haiku-4.5", "Claude Haiku 4.5"),
            m("gpt-4o-mini", "GPT-4o mini"),
        ];
        assert_eq!(
            resolve_model_id("gpt-4o-mini", &models).as_deref(),
            Some("gpt-4o-mini")
        );
        assert_eq!(
            resolve_model_id("Claude Haiku 4.5", &models).as_deref(),
            Some("claude-haiku-4.5")
        );
        assert_eq!(
            resolve_model_id("haiku", &models).as_deref(),
            Some("claude-haiku-4.5")
        );
        assert_eq!(resolve_model_id("nope-xyz", &models), None);
    }

    #[test]
    fn cheap_model_prefers_list_then_defaults() {
        let models = vec![m("gpt-4o-mini", "GPT-4o mini")];
        assert_eq!(detect_cheap_model(&models), "gpt-4o-mini");
        assert_eq!(detect_cheap_model(&[]), "gpt-4o-mini");
    }
}
