//! Prototype the DIRECTED-CROSS-REFERENCE + NEGATION-POLARITY assertional comparator.
//! Throwaway probe (untracked). Measures every required proof pair end to end so the final
//! module encodes only what the frozen substrate actually supports.
use helpers_native::lint_ai::token_seed;
use helpers_native::lint_char::{self, MeaningNetwork};
use helpers_native::lint_english;
use std::collections::{HashSet, VecDeque};

/// Bounded bidirectional definition-reference path length. 0 = same word, 1 = direct edge.
/// `None` = unreachable within `horizon`.
fn ref_hops(m: &MeaningNetwork, a: &str, b: &str, horizon: usize) -> Option<usize> {
    fn dir(m: &MeaningNetwork, from: &str, to: &str, horizon: usize) -> Option<usize> {
        let (from, to) = (from.to_lowercase(), to.to_lowercase());
        if from == to {
            return Some(0);
        }
        let mut seen = HashSet::new();
        let mut q = VecDeque::from([(from.clone(), 0usize)]);
        seen.insert(from);
        while let Some((cur, d)) = q.pop_front() {
            if d >= horizon {
                continue;
            }
            let Some(ws) = m.definition_words(&cur) else { continue };
            for w in ws {
                let wl = w.to_lowercase();
                if wl == to {
                    return Some(d + 1);
                }
                if seen.insert(wl.clone()) {
                    q.push_back((wl, d + 1));
                }
            }
        }
        None
    }
    match (dir(m, a, b, horizon), dir(m, b, a, horizon)) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    }
}

fn content(m: &MeaningNetwork, s: &str) -> Vec<String> {
    let known: Vec<String> = s
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

/// Statement polarity: negative when a negation OPERATOR (English::is_negation) appears.
fn negative(en: &lint_english::English, s: &str) -> bool {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .any(|t| en.is_negation(token_seed(&t.to_lowercase())))
}

const HORIZON: usize = 3;

/// Directed-reference distance between two concepts: hops if reachable, else a sentinel one
/// step past the horizon (comparative, derived from the horizon — not a hand-set threshold).
fn concept_dist(m: &MeaningNetwork, a: &str, b: &str) -> f64 {
    ref_hops(m, a, b, HORIZON).map_or((HORIZON + 1) as f64, |h| h as f64)
}

fn dir_align(m: &MeaningNetwork, from: &[String], to: &[String]) -> Option<f64> {
    if from.is_empty() || to.is_empty() {
        return None;
    }
    let (mut num, mut den) = (0.0, 0.0);
    for a in from {
        let nearest = to.iter().map(|b| concept_dist(m, a, b)).fold(f64::MAX, f64::min);
        let w = f64::from(m.centrality(a)).max(1.0);
        num += w * nearest;
        den += w;
    }
    Some(num / den)
}

/// Returns (polarity_mismatch, reference_distance). Lexicographic: polarity dominates.
fn consistency(
    m: &MeaningNetwork,
    en: &lint_english::English,
    a: &str,
    b: &str,
) -> Option<(bool, f64)> {
    let (ca, cb) = (content(m, a), content(m, b));
    let ab = dir_align(m, &ca, &cb)?;
    let ba = dir_align(m, &cb, &ca)?;
    let mismatch = negative(en, a) != negative(en, b);
    Some((mismatch, (ab + ba) / 2.0))
}

fn main() {
    let brain = lint_char::brain().expect("char brain");
    let m = brain.meanings();
    let en = lint_english::brain().expect("english brain");

    println!("== is_negation (polarity operators) ==");
    for w in ["avoid", "not", "do", "use", "dark", "bright", "never", "delete", "remove", "no"] {
        println!("  is_negation({w:>8}) = {}", en.is_negation(token_seed(w)));
    }

    println!("\n== content concepts ==");
    for s in ["a dog is a canine", "a dog is a bird", "avoid eval", "do not use eval", "use eval", "the sun is bright", "the sun is dark"] {
        println!("  {s:>22} -> {:?}  neg={}", content(m, s), negative(en, s));
    }

    println!("\n== comparator verdicts (mismatch, ref_dist); lower ref_dist = more consistent ==");
    let cases: &[(&str, &str, &str)] = &[
        ("IS-A T vs F", "a dog is a canine", "a dog is a bird"),
        ("co-hyponym", "a mammal is an animal", "a mammal is a fish"),
        ("synonym", "water is a liquid", "water is a fluid"),
        ("synonym2", "delete the file", "remove the file"),
        ("antonym", "the sun is bright", "the sun is dark"),
        ("negation OK", "avoid eval", "do not use eval"),
        ("negation BAD", "avoid eval", "use eval"),
    ];
    for (label, a, b) in cases {
        println!("  {label:>14}: d({a:?}, {b:?}) = {:?}", consistency(m, en, a, b));
    }

    println!("\n== corroborates margins: is `good` nearer to anchor than `foil`? ==");
    let trials: &[(&str, &str, &str, &str)] = &[
        ("is-a", "a dog is a canine", "a dog is a canine animal", "a dog is a bird"),
        ("co-hyp", "the cat is a mammal", "the cat is an animal", "the cat is a fish"),
        ("co-hyp2", "a dog is a mammal", "a dog is an animal", "a dog is a fish"),
        ("synonym", "water is a liquid", "water is a fluid", "water is a gas"),
        ("neg-flip", "do not use eval", "never use eval", "use eval"),
        ("antonym", "the sun is bright", "the sun is luminous", "the sun is dark"),
    ];
    for (label, anchor, good, foil) in trials {
        let dg = consistency(m, en, anchor, good);
        let df = consistency(m, en, anchor, foil);
        let ok = match (dg, df) {
            (Some(g), Some(f)) => Some(g < f),
            _ => None,
        };
        println!("  {label:>10}: good={dg:?} foil={df:?} -> corroborates={ok:?}");
    }

    println!("\n== graduation engine: classify each dog-is-canine witness (foil='a dog is a bird') ==");
    use helpers_native::lint_ism::{classify, graduate, Candidate};
    let candidate = Candidate::new("a dog is a canine", "a dog is a bird");
    let base = consistency(m, en, &candidate.truth, &candidate.foil).unwrap().1;
    println!("  baseline d(truth, foil) = {base:.3}");
    let witnesses = [
        "a dog is a canine",
        "the dog belongs to the canine family",
        "every dog is a canine animal",
        "a hound is a canine creature",
        "the domestic dog is classified as a canine",
        "a puppy grows into a canine",
        "a dog is a member of the canine group",
        "the dog is a kind of canine",
        "a dog counts as a canine mammal",
        "our pet dog is a canine",
        "the dog species is canine",
        "a dog is fundamentally a canine",
        "a dog remains a canine",
        "a beagle is a canine",
        "a dog qualifies as a canine",
        "the hound is a canine breed",
        "a terrier is a canine",
        "a stray dog is still a canine",
        "the shepherd dog is a canine",
        "a dog is a domesticated canine",
    ];
    for w in witnesses {
        let d = consistency(m, en, &candidate.truth, w).map(|c| c.1);
        println!("  {:?} d={:?} -> {:?}", w, d, classify(m, en, &candidate, w));
    }
    println!("  graduate -> {:?}", graduate(m, en, &candidate, witnesses));
}
