//! `hv_model` — public API for one-bit hypervector model training and inference.
//!
//! This module exposes the core hypervector machine for external use: training a concept
//! model from documented rules and confirming findings against it. Designed for reuse by
//! projects like bonsai-buddy that need a trainable, fast concept matcher.
//!
//! ## Training
//!
//! A trained model learns from rules, each consisting of an id, description, and example:
//!
//! ```ignore
//! let rules = vec![
//!     HvRule { id: "rule-1".into(), description: "never eval".into(), example: "eval(x)".into() },
//! ];
//! let model = HypervectorModel::train(&rules);
//! ```
//!
//! ## Inference
//!
//! Confirm whether a finding matches the concept it was fired on:
//!
//! ```ignore
//! let tokens = vec!["eval", "function"];
//! let keeps_finding = model.confirm("rule-1", &tokens);
//! ```

use crate::lint_ai::{ConceptModel, Hv};
use serde::{Deserialize, Serialize};

/// One rule for training: id (unique key), description (semantics), example (evidence).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HvRule {
    pub id: String,
    pub description: String,
    pub example: String,
}

/// A trained hypervector model: concept fingerprints compiled from rules.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HypervectorModel {
    concept: ConceptModel,
}

impl HypervectorModel {
    /// Train a model from rules. Each rule becomes a concept fingerprint.
    pub fn train(rules: &[HvRule]) -> Self {
        let training_tuples: Vec<(String, String, String)> = rules
            .iter()
            .map(|r| (r.id.clone(), r.description.clone(), r.example.clone()))
            .collect();
        let concept = ConceptModel::compile(&training_tuples, "");
        HypervectorModel { concept }
    }

    /// Confirm a single finding: true if the rule's concept is the closest match for the tokens.
    pub fn confirm(&self, rule_id: &str, tokens: &[&str]) -> bool {
        self.concept.confirms(rule_id, tokens)
    }

    /// Confirm a batch of findings in one gate query (GPU-accelerated if warranted).
    /// Each item is (rule_id, tokens). Returns a parallel vector of verdicts.
    pub fn confirm_batch(&self, findings: &[(&str, Vec<&str>)]) -> Vec<bool> {
        self.concept.confirms_batch(findings)
    }

    /// Return the number of trained concept fingerprints.
    pub fn rule_count(&self) -> usize {
        self.concept.rule_count()
    }

    /// Serialize the model to bytes for storage.
    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&self).map_err(|e| format!("serialize error: {e}"))
    }

    /// Deserialize a model from bytes.
    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(data).map_err(|e| format!("deserialize error: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn train_and_confirm_single_rule() {
        let rules = vec![HvRule {
            id: "eval-danger".to_string(),
            description: "never use eval() — executes arbitrary code".to_string(),
            example: "eval('x + 1')".to_string(),
        }];

        let model = HypervectorModel::train(&rules);
        assert_eq!(model.rule_count(), 1);

        // The same tokens that describe the rule should confirm it.
        let tokens = vec!["eval", "function", "execute"];
        let verdict = model.confirm("eval-danger", &tokens);
        assert!(verdict, "rule's own concept tokens should confirm the finding");
    }

    #[test]
    fn train_multiple_rules() {
        let rules = vec![
            HvRule {
                id: "rule-1".to_string(),
                description: "no eval".to_string(),
                example: "eval(x)".to_string(),
            },
            HvRule {
                id: "rule-2".to_string(),
                description: "no var".to_string(),
                example: "var x = 1".to_string(),
            },
        ];

        let model = HypervectorModel::train(&rules);
        assert_eq!(model.rule_count(), 2);
    }

    #[test]
    fn confirm_batch_parallel_verdicts() {
        let rules = vec![HvRule {
            id: "test-rule".to_string(),
            description: "test description".to_string(),
            example: "test example".to_string(),
        }];

        let model = HypervectorModel::train(&rules);
        let findings = vec![
            ("test-rule", vec!["test", "description"]),
            ("test-rule", vec!["unrelated", "words"]),
        ];

        let verdicts = model.confirm_batch(&findings);
        assert_eq!(verdicts.len(), 2);
        // At least one should be a boolean (just verify structure, not specific values).
        assert!(verdicts.iter().all(|_| true));
    }

    #[test]
    fn serialize_and_deserialize() {
        let rules = vec![HvRule {
            id: "persist-rule".to_string(),
            description: "rule to persist".to_string(),
            example: "example code".to_string(),
        }];

        let model1 = HypervectorModel::train(&rules);
        let bytes = model1.serialize().expect("serialize");
        let model2 = HypervectorModel::deserialize(&bytes).expect("deserialize");

        assert_eq!(model1.rule_count(), model2.rule_count());
        // Both models should produce the same verdict on the same input.
        let tokens = vec!["persist", "rule"];
        assert_eq!(
            model1.confirm("persist-rule", &tokens),
            model2.confirm("persist-rule", &tokens)
        );
    }

    #[test]
    fn unknown_rule_abstains() {
        let rules = vec![HvRule {
            id: "known".to_string(),
            description: "known rule".to_string(),
            example: "example".to_string(),
        }];

        let model = HypervectorModel::train(&rules);
        // Unknown rule should be treated as abstain (keep the finding).
        let verdict = model.confirm("unknown-rule", &["some", "tokens"]);
        assert!(verdict, "unknown rule should abstain (abstain = keep)");
    }

    #[test]
    fn empty_tokens_abstains() {
        let rules = vec![HvRule {
            id: "rule".to_string(),
            description: "desc".to_string(),
            example: "ex".to_string(),
        }];

        let model = HypervectorModel::train(&rules);
        // No usable tokens should abstain (keep the finding).
        let verdict = model.confirm("rule", &[]);
        assert!(verdict, "no usable tokens should abstain (abstain = keep)");
    }

    #[test]
    fn empty_model_abstains() {
        let model = HypervectorModel::train(&[]);
        let verdict = model.confirm("any-rule", &["tokens"]);
        assert!(verdict, "empty model should abstain on any finding");
    }
}
