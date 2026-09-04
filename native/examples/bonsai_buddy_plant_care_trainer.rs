//! Example: Bonsai-Buddy Plant Care Model Trainer
//!
//! This example demonstrates how an external project like bonsai-buddy can:
//! 1. Train a domain-specific concept model from plant-care rules
//! 2. Serialize the model for persistence (e.g., to a database or cache)
//! 3. Load and use the model to confirm plant-care findings
//!
//! This shows the complete workflow: rules → training → serialization →
//! deserialization → inference → decision.

use helpers_native::hv_model::{HvRule, HypervectorModel};

fn main() {
    println!("=== Bonsai-Buddy Plant Care Model Trainer ===\n");

    // Step 1: Define plant-care rules (concepts)
    let plant_care_rules = vec![
        HvRule {
            id: "overwatering".into(),
            description: "water too frequently causes root rot".into(),
            example: "daily watering yellowing leaves".into(),
        },
        HvRule {
            id: "underwatering".into(),
            description: "insufficient water causes dry leaves".into(),
            example: "no water brown crispy edges".into(),
        },
        HvRule {
            id: "sunburn".into(),
            description: "direct midday sun bleaches leaves".into(),
            example: "harsh direct sun white patches".into(),
        },
        HvRule {
            id: "insufficient_light".into(),
            description: "lack of light causes weak growth".into(),
            example: "no sun leggy pale leaves".into(),
        },
        HvRule {
            id: "nutrient_deficiency".into(),
            description: "missing nutrients causes stunted growth".into(),
            example: "no fertilizer slow growth yellowing".into(),
        },
    ];

    println!("Step 1: Training model from {} plant-care rules...", plant_care_rules.len());

    // Step 2: Train the model
    let model = HypervectorModel::train(&plant_care_rules);
    assert_eq!(model.rule_count(), plant_care_rules.len());
    println!("✓ Model trained with {} concepts\n", model.rule_count());

    // Step 3: Demonstrate single inference
    println!("Step 2: Single inference examples...");
    let test_findings = vec![
        ("overwatering", vec!["daily", "watering", "yellowing"]),
        ("sunburn", vec!["harsh", "direct", "sun"]),
        ("underwatering", vec!["no", "water", "brown"]),
    ];

    for (expected_rule, tokens) in &test_findings {
        let keeps = model.confirm(expected_rule, tokens);
        println!("  - Rule '{}' with tokens {:?}: {}", expected_rule, tokens, if keeps { "KEEP" } else { "REJECT" });
    }
    println!("✓ Individual inference working\n");

    // Step 4: Batch inference (GPU-friendly, if thresholds met)
    println!("Step 3: Batch inference (GPU-friendly)...");
    let batch_items: Vec<(&str, Vec<&str>)> = test_findings
        .iter()
        .map(|(rule_id, tokens)| (rule_id.as_ref(), tokens.clone()))
        .collect();

    let verdicts = model.confirm_batch(&batch_items);
    println!("  - Batch verdicts: {:?}", verdicts);
    assert_eq!(verdicts.len(), batch_items.len());
    assert!(verdicts.iter().all(|v| *v), "all test cases should confirm");
    println!("✓ Batch inference working ({} verdicts)\n", verdicts.len());

    // Step 5: Serialization (for persistence to database/cache)
    println!("Step 4: Model serialization for persistence...");
    let serialized = model.serialize().expect("serialize");
    println!("✓ Model serialized to {} bytes\n", serialized.len());

    // Step 6: Deserialization and round-trip verification
    println!("Step 5: Deserialization and round-trip verification...");
    let loaded_model = HypervectorModel::deserialize(&serialized).expect("deserialize");
    assert_eq!(loaded_model.rule_count(), model.rule_count());

    // Verify loaded model gives same results
    for (rule_id, tokens) in &test_findings {
        let original_verdict = model.confirm(rule_id, tokens);
        let loaded_verdict = loaded_model.confirm(rule_id, tokens);
        assert_eq!(original_verdict, loaded_verdict, "round-trip verdict mismatch");
    }
    println!("✓ Round-trip serialization verified\n");

    println!("=== Complete ===");
    println!("The hv_model API is ready for external projects like bonsai-buddy to:");
    println!("  1. Define domain-specific rules");
    println!("  2. Train concept fingerprints");
    println!("  3. Persist models to storage");
    println!("  4. Load and infer at runtime");
    println!("  5. Make keep/reject decisions based on concept matching");
}
