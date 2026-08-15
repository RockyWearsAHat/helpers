//! THROWAWAY PROBE (untracked): COMPLETION PASS 12 — measure the two halves of the polarity rung.
//!
//! Half 2 (morphological): for each candidate negation PREFIX, measure the systematic-flip rate —
//! among headwords H = prefix+stem where the STEM is itself a headword, what fraction of H's own
//! definition NEGATES the stem (definition carries a discovered base negator AND references the stem
//! or a one-hop meaning-neighbor of it). A prefix whose rate rides far above the background
//! "definition carries a negator" base rate earns negation-operator status covenant-clean
//! (typography, no word list). Then test whether the register words cross under a prefix-augmented
//! `is_negation` through the definition-compounding path already in the substrate.
//!
//! Run: `cargo run --release --example polrung`
use helpers_native::lint_ai::token_seed;
use helpers_native::lint_english;
use std::collections::{HashMap, HashSet};

fn main() {
    let eng = lint_english::brain().expect("english brain");
    let defs = lint_english::dictionary_definitions(None, 12).expect("dictionary parses");
    println!("dictionary: {} headword definitions, {} discovered base negators",
        defs.len(), eng.negators.len());

    let heads: HashSet<String> =
        defs.iter().map(|(h, _)| h.clone()).filter(|h| !h.contains(' ')).collect();
    let mut def_of: HashMap<String, Vec<String>> = HashMap::new();
    for (h, ws) in &defs {
        if !h.contains(' ') {
            def_of.entry(h.clone()).or_insert_with(|| ws.clone());
        }
    }

    // A word is a negator in round r if: it is a base discovered negator, OR it is a
    // qualified-prefix + headword-stem (PURE TYPOGRAPHY — no def gate; the whole point of the
    // learned prefix rule), OR (compounding) its def carries a round-negator AND another
    // round-negator-defined word. Prefixes qualify iteratively: round 1 sees only base negators.
    let prefixes = ["un", "in", "im", "il", "ir", "dis", "non", "mis", "de", "a", "ab", "anti", "counter"];

    // Returns (qualified prefixes, is_negator closure) at a fixpoint over ROUNDS rounds.
    let base_negator = |w: &str| eng.negators.binary_search(&token_seed(w)).is_ok();
    let mut qualified: Vec<String> = Vec::new();
    for round in 1..=4 {
        // is_negator under the CURRENT qualified set (pure typography for prefixes).
        let qset = qualified.clone();
        let is_prefix_neg = |w: &str| qset.iter().any(|p| {
            w.strip_prefix(p.as_str()).is_some_and(|s| s.chars().count() >= 3 && heads.contains(s))
        });
        let is_neg = |w: &str| base_negator(w) || is_prefix_neg(w);
        let def_has_neg = |ws: &[String]| ws.iter().any(|w| is_neg(w));
        let base_hits = def_of.values().filter(|ws| def_has_neg(ws)).count();
        let base_rate = base_hits as f64 / def_of.len().max(1) as f64;
        println!("--- ROUND {round}: {} negators so far {:?}; base def-carries-negator rate {:.4} ---",
            eng.negators.len() + qset.iter().map(|p| def_of.keys().filter(|h| h.starts_with(p.as_str())).count()).sum::<usize>(),
            qset, base_rate);
        println!("{:<9} {:>5} {:>8} {:>6} {:>5}", "prefix", "sup", "neg+stem", "rate%", "lift");
        let mut newly: Vec<String> = Vec::new();
        for p in prefixes {
            if qualified.iter().any(|q| q == p) { continue; }
            let (mut support, mut neg_stem) = (0usize, 0usize);
            for (h, ws) in &def_of {
                let Some(stem) = h.strip_prefix(p) else { continue };
                if stem.chars().count() < 3 || !heads.contains(stem) { continue; }
                support += 1;
                let has_neg = def_has_neg(ws);
                let stem_ref = ws.iter().any(|w| {
                    w == stem || def_of.get(w).is_some_and(|d| d.iter().any(|x| x == stem))
                });
                if has_neg && stem_ref { neg_stem += 1; }
            }
            if support < 20 { continue; }
            let rate = neg_stem as f64 / support as f64;
            let lift = rate / base_rate.max(1e-9);
            let q = rate >= 3.0 * base_rate && neg_stem >= 20;
            if q { newly.push(p.to_string()); }
            if lift >= 1.5 || q {
                println!("{p:<9} {support:>5} {neg_stem:>8} {:>5.1}% {lift:>4.1}x{}",
                    rate * 100.0, if q { "  <== QUALIFIES" } else { "" });
            }
        }
        if newly.is_empty() { println!("(fixpoint — no new prefix qualifies)\n"); break; }
        qualified.extend(newly);
        println!();
    }
    println!("QUALIFIED negation prefixes at fixpoint: {:?}\n", qualified);

    // Final aug_negation: base OR pure-typography prefix OR def-compounding (is_negation shape).
    let qfin = qualified.clone();
    let hset = heads.clone();
    let is_prefix_neg = move |w: &str| qfin.iter().any(|p| {
        w.strip_prefix(p.as_str()).is_some_and(|s| s.chars().count() >= 3 && hset.contains(s))
    });
    let aug_negation = |w: &str| -> bool {
        if eng.is_negation(token_seed(w)) || is_prefix_neg(w) { return true; }
        // compounding: def has a negator AND another negator-defined word (is_negation shape)
        let Some(def) = def_of.get(w) else { return false };
        let neg = |x: &str| eng.is_negation(token_seed(x)) || is_prefix_neg(x);
        let neg_defined = |x: &str| def_of.get(x).is_some_and(|d| d.iter().any(|y| neg(y)));
        def.iter().any(|x| neg(x)) && def.iter().filter(|x| !neg(x)).any(|x| neg_defined(x))
    };

    println!("== register words: does each read NEGATIVE under fixpoint prefixes? ==");
    let lemma = |w: &str| -> String {
        for suf in ["d", "ed", "ing", "s", "es"] {
            if let Some(stem) = w.strip_suffix(suf) {
                if stem.chars().count() >= 3 && heads.contains(stem) { return stem.to_string(); }
            }
        }
        w.to_string()
    };
    for w in ["deprecated", "deprecate", "discouraged", "discourage", "obsolete", "avoid",
              "disapproval", "disapprove", "approval", "approve", "recommended", "forbidden", "removed"] {
        let frozen = eng.is_negation(token_seed(w));
        let lm = lemma(w);
        let neg = aug_negation(w) || aug_negation(&lm);
        let def = def_of.get(w).or_else(|| def_of.get(&lm));
        let defwords = def.map(|d| d.join(" ")).unwrap_or_else(|| "<no def>".into());
        println!("  {w:12} frozen={frozen:5} NEG={neg:5}  [{lm}] def=[{defwords}]");
    }

    // ── Which token is the single discovered base negator? ──
    println!("\n== base negator identity ==");
    for w in ["not", "no", "never", "none", "nor", "without", "lack"] {
        println!("  {w:9} base_discovered={}", eng.negators.binary_search(&token_seed(w)).is_ok());
    }

    // ── HALF 1: sentence-level foil test on the REAL MDN texts ──
    // is_negation position-free (any word) vs frozen states_prohibition (imperative-lead).
    println!("\n== HALF 1: sentence polarity on real banner texts ==");
    let sentences: &[(&str, bool)] = &[
        // (text, SHOULD-read-negative?)  the 117 deprecation register = true; banners/section = false
        ("Deprecated: This feature is no longer recommended.", true),
        ("This feature is no longer recommended.", true),
        ("Warning: This is an obsolete API and is no longer guaranteed to work.", true),
        ("Be aware that this feature may cease to work at any time.", true),
        ("This feature is well established and works across many devices and browser versions.", false),
        ("This feature is not Baseline because it does not work in some of the most widely-used browsers.", false),
        ("Not all browsers may have implemented every part of this specification yet.", false),
        ("Return value", false),
        ("Computed value", false),
    ];
    let pos_free_isneg = |s: &str| s.split_whitespace().any(|w| {
        let t = w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
        !t.is_empty() && eng.is_negation(token_seed(&t))
    });
    let pos_free_aug = |s: &str| s.split_whitespace().any(|w| {
        let t = w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
        !t.is_empty() && aug_negation(&t)
    });
    println!("{:>8} {:>9} {:>9} {:>10}  text", "want", "impered", "posfree", "posf+aug");
    for (s, want) in sentences {
        println!("{:>8} {:>9} {:>9} {:>10}  {:?}",
            if *want { "NEG" } else { "-" },
            eng.sentence_states_prohibition(s),
            pos_free_isneg(s),
            pos_free_aug(s),
            &s[..s.len().min(60)]);
    }

    // ── FP audit: promoting un-/non- into is_negation — how many words flip, sample them ──
    println!("\n== FP audit: words that read NEG only via un-/non- prefix (sample) ==");
    let mut flipped = 0usize;
    let mut sample: Vec<String> = Vec::new();
    for h in &heads {
        if !eng.is_negation(token_seed(h)) && aug_negation(h) {
            flipped += 1;
            if sample.len() < 25 { sample.push(h.clone()); }
        }
    }
    println!("  {flipped} headwords newly read NEG; sample: {sample:?}");
}
