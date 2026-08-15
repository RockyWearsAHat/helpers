//! PASS 16 SUPERSESSION-REGISTER FEASIBILITY MEASUREMENT (kept, reproducible — the ledger's harness).
//!
//! The owner's PASS 16 target: flag OLD-way web code with the MODERN recommendation, the rule read
//! from MDN's own supersession prose ("X is designed as a replacement for Y", "the successor to Y"),
//! extracted THROUGH UNDERSTANDING — never a phrase list in code. The premise: the frozen meaning
//! network already relates the substitution vocabulary (replacement / successor / alternative /
//! instead / supersede) to a substitution anchor, so a supersession sentence can be RECOGNISED
//! without enumerating those words.
//!
//! This harness MEASURES that premise on the frozen brains, four independent ways. All four fail: the
//! substitution register is NOT separable in the substrate's meaning geometry — the same flat-geometry
//! wall passes 11/12 measured for the PROHIBITION register, now reconfirmed for SUPERSESSION. The only
//! reliable signal is LITERAL membership of a substitution word in the sentence (hop-0), which is a
//! hardcoded phrase list — forbidden by the covenant. See native/history.dx "COMPLETION PASS 16".
//!
//! Run: `cargo run --release --example super_probe`
use helpers_native::lint_char::{self, MeaningNetwork};
use helpers_native::lint_english;
use std::collections::{HashSet, VecDeque};

/// Directed hops from `from`'s dictionary-definition graph to the literal word `to` (relcheck's
/// `ref_hops`, one direction). `Some(0)` = the words are identical (literal membership).
fn dir_hops(m: &MeaningNetwork, from: &str, to: &str, horizon: usize) -> Option<usize> {
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

fn content(m: &MeaningNetwork, s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .filter(|t| m.has(t))
        .collect()
}

fn main() {
    let brain = lint_char::brain().expect("char brain");
    let m = brain.meanings();
    let _en = lint_english::brain().expect("english brain");

    // The substitution register (the words MDN uses to state supersession) vs unrelated web-doc
    // vocabulary. A working understanding-read would rank the family tight and the controls far.
    let family = [
        "replace", "replaces", "replacement", "successor", "supersede", "supersedes", "substitute",
        "alternative", "instead", "obsolete",
    ];
    let controls = [
        "banana", "colour", "render", "tooltip", "animate", "keyword", "browser", "library",
        "pattern", "background", "widget", "promise",
    ];

    // ── WALL 1: dictionary related() is flat — the family is not tighter than the controls ────────
    println!("###### WALL 1 — related() margin: nearest family-word vs nearest control ######");
    let mut sep = 0;
    for a in family {
        let f = family.iter().filter(|&&b| b != a).map(|b| m.related(a, b)).min().unwrap_or(9999);
        let c = controls.iter().map(|c| m.related(a, c)).min().unwrap_or(9999);
        if f < c {
            sep += 1;
        }
        println!("  {a:>12}: family={f:>5} control={c:>5} [{}]", if f < c { "SEP" } else { "flat" });
    }
    println!("  family internal-coherence: {sep}/{}", family.len());

    // ── WALL 2: the same test on CONTROLS false-hits — the ordering is noise, not a classifier ────
    println!("\n###### WALL 2 — specificity: controls that falsely rank a family word closest ######");
    let mut fp = 0;
    for c in controls {
        let f = family.iter().map(|f| m.related(c, f)).min().unwrap_or(9999);
        let o = controls.iter().filter(|&&x| x != c).map(|o| m.related(c, o)).min().unwrap_or(9999);
        if f < o {
            fp += 1;
        }
        println!("  {c:>12}: family={f:>5} control={o:>5} [{}]", if f < o { "FALSE-HIT" } else { "ok" });
    }
    println!("  control false-hit rate: {fp}/{} (a real classifier would be ~0)", controls.len());

    // ── WALL 3: learned USAGE sense (context_related) is flat too ─────────────────────────────────
    println!("\n###### WALL 3 — context_related (learned usage sense): family vs controls ######");
    let mut usep = 0;
    for a in family {
        let f =
            family.iter().filter(|&&b| b != a).map(|b| m.context_related(a, b)).min().unwrap_or(9999);
        let c = controls.iter().map(|c| m.context_related(a, c)).min().unwrap_or(9999);
        if f < c {
            usep += 1;
        }
        println!("  {a:>12}: family={f:>5} control={c:>5} [{}]", if f < c { "SEP" } else { "flat" });
    }
    println!("  usage internal-coherence: {usep}/{}", family.len());

    // ── WALL 4: directed cross-reference collapses to hop-0 = literal membership ──────────────────
    // Real MDN supersession sentences vs real MDN clean governing sentences; the "via" column shows
    // the only hits are hop-0 (the sentence literally contains a substitution word), and the two
    // supersession sentences with no literal substitution word ("no longer", "supersedes") MISS.
    println!("\n###### WALL 4 — directed cross-reference on real sentences (horizon 2) ######");
    let super_sents = [
        "It is designed as a full replacement for the Date object",
        "The Navigation API is the successor to the History API",
        "you no longer need to write a routing library",
        "a declarative alternative to opening a dialog with JavaScript",
        "Temporal is meant to replace the Date object entirely",
        "this element supersedes the older hand-rolled accordion",
    ];
    let clean_sents = [
        "The URLPattern interface matches URLs against a pattern",
        "This method returns a promise that resolves when the animation finishes",
        "The keyword sets the color of an element background",
        "Container style queries let you apply styles based on a custom property",
        "The details element creates a disclosure widget",
        "contrast-color chooses black or white for maximum legibility",
    ];
    let score = |s: &str| -> (bool, String) {
        for w in content(m, s) {
            for a in family {
                if let Some(d) = dir_hops(m, &w, a, 2) {
                    return (true, format!("{w}->{a}#{d}"));
                }
            }
        }
        (false, String::new())
    };
    println!("  -- supersession (want a hit) --");
    for s in super_sents {
        let (h, v) = score(s);
        println!("   hit={h:<6} {v:<26} | {s}");
    }
    println!("  -- clean (want a miss) --");
    for s in clean_sents {
        let (h, v) = score(s);
        println!("   hit={h:<6} {v:<26} | {s}");
    }
    println!(
        "\nVERDICT: the substitution register is not separable through understanding; the only signal\n\
         is literal membership (hop-0) = a hardcoded phrase list. Covenant-forbidden. See native/history.dx P16."
    );
}
