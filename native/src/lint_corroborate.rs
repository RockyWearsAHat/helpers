//! The English-equality corroboration judge — the referee the corroboration loop stands on.
//!
//! Contract: `LINTER.md` → north-star section → "The English-equality corroboration judge". Given
//! two English statements (the expected outcome and the actual outcome, both derived back into
//! English by the corroboration loop), decide whether they assert the **same / consistent** thing.
//! The decision is reduced ENTIRELY to the frozen dictionary meaning graph ([`MeaningNetwork`]):
//! meaning-set overlap via [`MeaningNetwork::related`], never spelling, never a word list.
//!
//! Two properties are load-bearing and enforced by construction here:
//!   * **Comparative, never a magic threshold.** Nothing decides "consistent iff score < K". The
//!     public verdicts are a ranking ([`more_consistent`]) and a margin against a caller-supplied
//!     foil ([`corroborates`]). The corroboration engine always has such a foil — the alternative
//!     expectation it also derived — so equality is judged as a margin, not against a constant.
//!   * **Concepts orthogonal until provably linked.** No concept's meaning is weighted into
//!     another's; the judge only ever READS `related`/`centrality`/`has` off the frozen graph.
//!
//! Measured competence (2026-07-10, see the contract): a reliable relatedness FLOOR (a true
//! restatement scores far nearer an anchor than an off-topic foil) but a WEAK assertional referee
//! (labeled AUC ≈ 0.80 — it cannot separate a restatement from a same-topic contradiction, because
//! the graph measures topical relatedness, not assertional equality). Callers must keep foils
//! off-topic or gate same-topic judgments as unproven.

use std::cmp::Ordering;

use crate::lint_char::MeaningNetwork;

/// The content concepts of an English statement: its tokens the bedrock KNOWS ([`MeaningNetwork::has`]),
/// keeping those whose [`centrality`](MeaningNetwork::centrality) is at or above the statement's own
/// MEDIAN. The median cut is comparative — it drops the shared filler ("the", "is", "a") that every
/// statement carries without any stop list, exactly as the meaning bundles already suppress it — while
/// keeping the distinctive words the statement is actually about. Words unknown to English (jargon like
/// `eval`) are dropped here rather than fed to `related`, which would score them as maximally unrelated;
/// an honest English judge simply cannot speak to a word English does not define.
pub fn content_concepts(m: &MeaningNetwork, statement: &str) -> Vec<String> {
    let known: Vec<String> = statement
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .filter(|t| m.has(t))
        .collect();
    if known.is_empty() {
        return known;
    }
    let mut cents: Vec<u32> = known.iter().map(|w| m.centrality(w)).collect();
    cents.sort_unstable();
    let median = cents[cents.len() / 2];
    known.into_iter().filter(|w| m.centrality(w) >= median).collect()
}

/// Directed centrality-weighted chamfer distance from `from` to `to`: for each concept in `from`, its
/// NEAREST meaning distance to any concept in `to` ([`MeaningNetwork::related`]), weighted by the
/// source concept's [`centrality`](MeaningNetwork::centrality) so distinctive concepts dominate the
/// score. `None` when either side has no content concept (nothing English to compare).
fn directed_alignment(m: &MeaningNetwork, from: &[String], to: &[String]) -> Option<f64> {
    if from.is_empty() || to.is_empty() {
        return None;
    }
    let mut num = 0.0;
    let mut den = 0.0;
    for a in from {
        let nearest = to.iter().map(|b| m.related(a, b)).min().unwrap_or(u32::MAX);
        let w = f64::from(m.centrality(a));
        num += w * f64::from(nearest);
        den += w;
    }
    (den > 0.0).then(|| num / den)
}

/// The CONSISTENCY DISTANCE of two statements: the symmetric centrality-weighted chamfer over their
/// content concepts (lower = the two assert more nearly the same thing). This is meaning-set overlap
/// reduced to the frozen graph — pure and stable, the same statistic the meaning bundles weigh by.
/// `None` when either statement has no English content concept (the judge has nothing to speak to).
pub fn consistency_distance(m: &MeaningNetwork, a: &str, b: &str) -> Option<f64> {
    let ca = content_concepts(m, a);
    let cb = content_concepts(m, b);
    let ab = directed_alignment(m, &ca, &cb)?;
    let ba = directed_alignment(m, &cb, &ca)?;
    Some((ab + ba) / 2.0)
}

/// The comparative referee primitive: which of `x`, `y` asserts something NEARER the `anchor`?
/// `Ordering::Less` means `x` is more consistent with `anchor` than `y` is (a smaller consistency
/// distance). No absolute threshold — the whole decision is a comparison of two distances. `None`
/// when a distance is undecidable (a statement with no English content concept).
pub fn more_consistent(m: &MeaningNetwork, anchor: &str, x: &str, y: &str) -> Option<Ordering> {
    let dx = consistency_distance(m, anchor, x)?;
    let dy = consistency_distance(m, anchor, y)?;
    dx.partial_cmp(&dy)
}

/// The corroboration verdict: does `actual` corroborate `expected`, judged against a `contrast` foil?
/// True iff `actual` aligns to `expected` STRICTLY nearer than the `contrast` baseline does — a margin
/// against a foil, never a magic constant. The corroboration loop supplies the foil (the alternative /
/// negated expectation it also derived). Per the measured competence boundary, the foil must be
/// genuinely OFF-TOPIC for the verdict to be trustworthy; a same-topic foil is inside the judge's
/// blind spot and its verdict there is unproven. `None` when a distance is undecidable.
pub fn corroborates(m: &MeaningNetwork, expected: &str, actual: &str, contrast: &str) -> Option<bool> {
    let d_actual = consistency_distance(m, expected, actual)?;
    let d_contrast = consistency_distance(m, expected, contrast)?;
    Some(d_actual < d_contrast)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint_char;

    /// The frozen brain, or `None` when the dictionary artifact is not on disk (the judge is defined
    /// only over the real bedrock, so tests observe against it or skip honestly — never fake a pass).
    fn brain() -> Option<&'static lint_char::CharReader> {
        lint_char::brain()
    }

    #[test]
    fn floor_ranks_restatement_over_offtopic_foil() {
        // The PROVEN competence: against an off-topic foil the ranker is reliable. A true restatement
        // of the anchor is nearer than a topically-unrelated statement. This is the floor the rest of
        // the substrate may lean on.
        let Some(br) = brain() else {
            eprintln!("skip: no frozen brain on disk");
            return;
        };
        let m = br.meanings();
        let anchor = "a dog is a canine";
        assert_eq!(
            more_consistent(m, anchor, "a dog is a canine animal", "a rock is a mineral"),
            Some(Ordering::Less),
            "true restatement must rank nearer than an off-topic foil"
        );
        // And the corroboration verdict holds with an off-topic contrast.
        assert_eq!(
            corroborates(m, anchor, "a dog is a canine animal", "a rock is a mineral"),
            Some(true)
        );
    }

    #[test]
    fn identical_statement_is_maximally_consistent() {
        let Some(br) = brain() else {
            eprintln!("skip: no frozen brain on disk");
            return;
        };
        let m = br.meanings();
        let d = consistency_distance(m, "water is a liquid", "water is a liquid");
        assert_eq!(d, Some(0.0), "a statement is exactly consistent with itself");
    }

    #[test]
    fn no_english_content_is_undecidable_not_a_false_match() {
        // Jargon unknown to English (`eval`) yields no content concept — the honest answer is "cannot
        // judge" (None), NOT a false zero-distance match. This is the unknown-word edge handled honestly.
        let Some(br) = brain() else {
            eprintln!("skip: no frozen brain on disk");
            return;
        };
        let m = br.meanings();
        assert_eq!(consistency_distance(m, "zzqx", "qxzz"), None);
    }

    #[test]
    fn measured_weak_assertional_signal_present_but_not_a_fine_referee() {
        // OBSERVES the competence boundary as data (not a shape assertion): a weak assertional signal
        // exists (consistent pairs mean-nearer than inconsistent), but it does NOT cleanly separate a
        // restatement from a same-topic contradiction — AUC well below a fine referee's ≈1.0. If this
        // ever rises to near-perfect, the substrate has genuinely strengthened and the verdict in
        // LINTER.md should be revisited.
        let Some(br) = brain() else {
            eprintln!("skip: no frozen brain on disk");
            return;
        };
        let m = br.meanings();
        let consistent = [
            ("a dog is a canine", "a dog is a canine animal"),
            ("the cat is a mammal", "the cat is an animal"),
            ("water is a liquid", "water is a fluid"),
            ("a car is a vehicle", "an automobile is a vehicle"),
            ("delete the file", "remove the file"),
            ("the sun is bright", "the sun is luminous"),
        ];
        let inconsistent = [
            ("a dog is a canine", "a dog is a bird"),
            ("the cat is a mammal", "the cat is a fish"),
            ("water is a liquid", "water is a gas"),
            ("a car is a vehicle", "a banana is a fruit"),
            ("delete the file", "create the file"),
            ("the sun is bright", "the sun is dark"),
        ];
        let cs: Vec<f64> = consistent.iter().filter_map(|(a, b)| consistency_distance(m, a, b)).collect();
        let is: Vec<f64> = inconsistent.iter().filter_map(|(a, b)| consistency_distance(m, a, b)).collect();
        let cmean = cs.iter().sum::<f64>() / cs.len() as f64;
        let imean = is.iter().sum::<f64>() / is.len() as f64;
        // Weak signal present: consistent pairs are on average nearer than inconsistent ones.
        assert!(cmean < imean, "expected a weak assertional signal: {cmean:.0} !< {imean:.0}");
        // But NOT a fine referee: some inconsistent pair scores nearer than some consistent one
        // (same-topic contradiction inside the restatement band). Documented, not hidden.
        let cmax = cs.iter().cloned().fold(f64::MIN, f64::max);
        let imin = is.iter().cloned().fold(f64::MAX, f64::min);
        assert!(
            imin < cmax,
            "if the graph ever cleanly separates these (imin {imin:.0} >= cmax {cmax:.0}), revisit the LINTER.md verdict"
        );
    }
}
