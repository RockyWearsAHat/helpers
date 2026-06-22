//! Fast linter: instant exemplar memory + the causation gate (no slow cloze training).
//!
//! Each rule's bad-example windows are stored as labeled hypervectors (hash+bundle —
//! instant), so recall on a rule's own example is 100% by construction. A code window is
//! flagged only when (a) its nearest exemplar is a rule within a clean-calibrated
//! distance cap and (b) the causation gate fires: drop the window's most distinctive
//! token and the nearest label CHANGES, i.e. that token caused the verdict. Otherwise
//! abstain. Measures recall, attribution (right rule), and held-out false flags.
//!
//!   cargo run --release --example measure_fast [clean_cap] [heldout_loc] [rule_stride] [dist_cap]

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use helpers_native::lint_ai::{bind, tokenize, Hv};

const WIN: usize = 4;
const CLEAN: u32 = u32::MAX; // sentinel label id for "not a violation"

fn rust_sources(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            if let Ok(t) = fs::read_to_string(&p) {
                out.push(t);
            }
        }
    }
}

fn windows(code: &str) -> Vec<Vec<String>> {
    let toks: Vec<String> = tokenize(code).into_iter().map(|t| t.text).collect();
    toks.windows(WIN).map(|w| w.to_vec()).collect()
}

/// One stored example: the window's code and its label (a rule id index, or CLEAN).
struct Ex {
    hv: Hv,
    label: u32,
}

/// The fast linter: exemplar memory + token frequencies (for the distinctive subject).
struct Fast {
    ex: Vec<Ex>,
    freq: HashMap<String, u32>,
    cap: u32,
}

impl Fast {
    fn teach(&mut self, win: &[String], label: u32) {
        let refs: Vec<&str> = win.iter().map(String::as_str).collect();
        self.ex.push(Ex { hv: bind(&refs), label });
        for t in win {
            *self.freq.entry(t.clone()).or_insert(0) += 1;
        }
    }

    /// Nearest exemplar's label and distance for a window (binds the given tokens).
    fn nearest(&self, toks: &[&str]) -> (u32, u32) {
        let q = bind(toks);
        let mut best = CLEAN;
        let mut bd = u32::MAX;
        for e in &self.ex {
            let d = e.hv.distance(&q);
            if d < bd {
                bd = d;
                best = e.label;
            }
        }
        (best, bd)
    }

    /// The verdict for a window: a rule label it is KNOWN to violate, or None.
    fn known(&self, win: &[String]) -> Option<u32> {
        if win.len() < 2 {
            return None;
        }
        let refs: Vec<&str> = win.iter().map(String::as_str).collect();
        let (best, dist) = self.nearest(&refs);
        if best == CLEAN || dist > self.cap {
            return None; // clean, or too far from any known violation
        }
        // distinctive subject = rarest observed token in the window
        let subj = win
            .iter()
            .filter(|t| self.freq.contains_key(*t))
            .min_by_key(|t| self.freq.get(*t).copied().unwrap_or(u32::MAX))?;
        let without: Vec<&str> = refs.iter().copied().filter(|t| t != subj).collect();
        if without.len() < 2 {
            return None;
        }
        let (generic, _) = self.nearest(&without);
        if generic == best {
            return None; // verdict didn't depend on the distinctive token → abstain
        }
        Some(best)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let clean_cap: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1500);
    let heldout_cap: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1500);
    let rule_stride: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let dist_cap: u32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1400);

    let raw = fs::read_to_string("../lint-index/clippy.json").expect("clippy.json");
    let idx: serde_json::Value = serde_json::from_str(&raw).expect("parse");
    let mut pairs: Vec<(String, String, String)> = Vec::new();
    for r in idx["rules"].as_array().expect("rules") {
        let bad = r["exampleBad"].as_str().unwrap_or("");
        let good = r["exampleGood"].as_str().unwrap_or("");
        if !bad.is_empty() && !good.is_empty() {
            pairs.push((
                r["id"].as_str().unwrap_or("").to_string(),
                bad.to_string(),
                good.to_string(),
            ));
        }
    }

    let mut clean = Vec::new();
    rust_sources(Path::new("src"), &mut clean);
    clean.sort();
    let split = clean.len() * 4 / 5;
    let (calib, held_out) = clean.split_at(split);

    let filter: u32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(600);

    let mut f = Fast { ex: Vec::new(), freq: HashMap::new(), cap: dist_cap };
    // 1) Teach all known-good code first: repo clean (capped) + every rule's good example.
    let mut cw: Vec<Vec<String>> = calib.iter().flat_map(|s| windows(s)).collect();
    let stride = (cw.len() / clean_cap).max(1);
    cw = cw.into_iter().step_by(stride).collect();
    for w in &cw {
        f.teach(w, CLEAN);
    }
    for (_, _, good) in &pairs {
        for w in windows(good) {
            f.teach(&w, CLEAN);
        }
    }
    // Distinctiveness reference = ALL calib clean windows (not the stored sample) + every
    // rule's good example. The bigger this is, the better it recognizes generic
    // scaffolding (fn/params/braces) so those windows can't become a rule's fingerprint.
    let mut clean_hvs: Vec<Hv> =
        calib.iter().flat_map(|s| windows(s)).map(|w| {
            let r: Vec<&str> = w.iter().map(String::as_str).collect();
            bind(&r)
        }).collect();
    for (_, _, good) in &pairs {
        for w in windows(good) {
            let r: Vec<&str> = w.iter().map(String::as_str).collect();
            clean_hvs.push(bind(&r));
        }
    }
    let min_clean = |hv: &Hv| clean_hvs.iter().map(|c| c.distance(hv)).min().unwrap_or(u32::MAX);

    // 2) Teach each rule's bad windows, but ONLY the DISTINCTIVE ones — a window that
    //    appears in clean code (near a clean exemplar) is scaffolding, not the violation,
    //    so it must never become the rule's fingerprint.
    let mut kept = 0;
    let mut dropped = 0;
    for (ri, (_, bad, _)) in pairs.iter().enumerate() {
        // score each window by how distinctive it is from clean code
        let scored: Vec<(Vec<String>, u32)> = windows(bad)
            .into_iter()
            .map(|w| {
                let refs: Vec<&str> = w.iter().map(String::as_str).collect();
                let d = min_clean(&bind(&refs));
                (w, d)
            })
            .collect();
        // keep every window above the filter; if none qualify, keep just the single most
        // distinctive one so the rule still has a fingerprint (recall without FP blowup).
        let any_kept = scored.iter().any(|(_, d)| *d > filter);
        let best = scored.iter().enumerate().max_by_key(|(_, (_, d))| *d).map(|(i, _)| i);
        for (i, (w, d)) in scored.iter().enumerate() {
            let keep = if any_kept { *d > filter } else { Some(i) == best };
            if keep {
                f.teach(w, ri as u32);
                kept += 1;
            } else {
                dropped += 1;
            }
        }
    }
    eprintln!(
        "exemplars: {}  tokens: {}  distinctive bad kept: {kept} dropped: {dropped}",
        f.ex.len(),
        f.freq.len()
    );

    // recall + attribution on a strided sample of rules.
    let mut flagged_any = 0;
    let mut flagged_right = 0;
    let mut tested = 0;
    for (ri, (_, bad, _)) in pairs.iter().enumerate().step_by(rule_stride) {
        tested += 1;
        let hits: Vec<u32> = windows(bad).iter().filter_map(|w| f.known(w)).collect();
        if !hits.is_empty() {
            flagged_any += 1;
        }
        if hits.contains(&(ri as u32)) {
            flagged_right += 1;
        }
    }
    let tested = tested.max(1);

    // held-out false flags (+ a sample of WHAT is flagged, to judge real vs spurious).
    let mut loc = 0usize;
    let mut ff = 0usize;
    let mut shown = 0;
    for s in held_out {
        if loc >= heldout_cap {
            break;
        }
        loc += s.lines().count();
        for w in windows(s) {
            if let Some(ri) = f.known(&w) {
                ff += 1;
                if shown < 25 {
                    eprintln!("  FLAG [{}]  {}", pairs[ri as usize].0, w.join(" "));
                    shown += 1;
                }
            }
        }
    }

    println!("rules: {} (candidates), tested {tested}", pairs.len());
    println!(
        "recall (flags own bad):          {flagged_any}/{tested} ({:.0}%)",
        flagged_any as f64 / tested as f64 * 100.0
    );
    println!(
        "attribution (flags RIGHT rule):  {flagged_right}/{tested} ({:.0}%)",
        flagged_right as f64 / tested as f64 * 100.0
    );
    println!(
        "held-out clean: {loc} LOC, {ff} false flags = {:.2} per 100 lines",
        ff as f64 / loc.max(1) as f64 * 100.0
    );
}
