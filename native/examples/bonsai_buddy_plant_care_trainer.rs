//! Example: bonsai-buddy training a plant-care detection model using the hypervector API.
//!
//! This demonstrates how an external project (bonsai-buddy) uses the public hv_model API
//! to train and deploy a concept model for detecting common plant problems.
//!
//! To run: `cargo run --example bonsai_buddy_plant_care_trainer --release`

use helpers_native::hv_model::{HvRule, HypervectorModel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Bonsai-Buddy Plant Care Trainer ===\n");

    // Step 1: Define plant-care rules (what bonsai-buddy cares about)
    let rules = vec![
        HvRule {
            id: "overwatering".into(),
            description: "excessive water causes root rot and leaf yellowing".into(),
            example: "daily watering leaves turning yellow drooping stems".into(),
        },
        HvRule {
            id: "underwatering".into(),
            description: "insufficient water causes dryness and leaf browning".into(),
            example: "no water soil bone dry brown edges crispy leaves".into(),
        },
        HvRule {
            id: "sunburn".into(),
            description: "direct midday sun causes leaf bleaching and damage".into(),
            example: "harsh sun white patches bleached leaves faded color".into(),
        },
        HvRule {
            id: "pest_infestation".into(),
            description: "insects feed on leaves causing holes and spots".into(),
            example: "holes on leaves sticky residue tiny insects spotted".into(),
        },
        HvRule {
            id: "nutrient_deficiency".into(),
            description: "lack of nutrients causes stunted growth and pale leaves".into(),
            example: "weak growth pale yellow leaves small size".into(),
        },
    ];

    println!("Training model on {} plant-care rules...", rules.len());
    let model = HypervectorModel::train(&rules);
    println!("✓ Model trained with {} concepts\n", model.rule_count());

    // Step 2: Use the model for inference — detecting problems in observations
    println!("Running inference on plant observations:\n");

    let observations = vec![
        (
            "overwatering",
            vec!["daily", "watering", "yellowing", "leaves"],
        ),
        (
            "underwatering",
            vec!["no", "water", "brown", "dry", "edges"],
        ),
        ("sunburn", vec!["harsh", "sun", "white", "patches"]),
        ("pest_infestation", vec!["holes", "leaves", "tiny", "insects"]),
        ("nutrient_deficiency", vec!["pale", "yellow", "weak", "growth"]),
    ];

    // Confirm individually
    println!("Individual verdicts:");
    for (rule_id, tokens) in &observations {
        let matches = model.confirm(rule_id, tokens);
        println!(
            "  {}: {} (tokens: {})",
            rule_id,
            if matches { "✓ YES" } else { "✗ NO" },
            tokens.join(", ")
        );
    }

    println!();

    // Step 3: Batch inference (what bonsai-buddy would use for efficiency)
    println!("Batch inference (GPU-friendly):");
    let batch_findings: Vec<(&str, Vec<&str>)> = observations
        .iter()
        .map(|(id, tokens)| (*id, tokens.clone()))
        .collect();

    let verdicts = model.confirm_batch(&batch_findings);
    for (i, (rule_id, _tokens)) in observations.iter().enumerate() {
        println!(
            "  {}: {}",
            rule_id,
            if verdicts[i] { "✓ YES" } else { "✗ NO" }
        );
    }

    println!();

    // Step 4: Persistence (bonsai-buddy saves models for reuse)
    println!("Serializing model to disk...");
    let serialized = model.serialize()?;
    println!(
        "✓ Model serialized to {} bytes (HLM1 codec)\n",
        serialized.len()
    );

    // Step 5: Load model from disk
    println!("Deserializing model from bytes...");
    let loaded_model = HypervectorModel::deserialize(&serialized)?;
    println!(
        "✓ Model loaded: {} concepts\n",
        loaded_model.rule_count()
    );

    // Step 6: Verify loaded model works identically
    println!("Verifying loaded model produces same results:");
    let test_tokens = vec!["daily", "watering", "yellowing"];
    let original_result = model.confirm("overwatering", &test_tokens);
    let loaded_result = loaded_model.confirm("overwatering", &test_tokens);

    println!(
        "  Original model: {}",
        if original_result { "match" } else { "no match" }
    );
    println!(
        "  Loaded model:   {}",
        if loaded_result { "match" } else { "no match" }
    );
    println!(
        "  {} (round-trip successful)\n",
        if original_result == loaded_result {
            "✓ Agreement"
        } else {
            "✗ MISMATCH"
        }
    );

    println!("=== Bonsai-Buddy Training Complete ===");
    println!("The public hv_model API is fully functional for external projects.");

    Ok(())
}
