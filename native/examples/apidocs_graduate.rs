//! THROWAWAY MEASUREMENT HARNESS (untracked): COMPLETION PASS 14 rung 3 — graduate python + rust modules
//! from the newly-acquired API-doc crawls. Runs the REAL `lint_module::graduate` over each language's
//! whole-site pages, reports the funnel (pages → attested → candidates → proven), and runs the kitchen-sink
//! acceptance (a file using genuinely-deprecated items flags with cites; an idiomatic clean file is zero).
//! Run: `cargo run --release --features crawl --example apidocs_graduate`
use helpers_native::lint_attest::Attestation;
use helpers_native::lint_char;
use helpers_native::lint_codec::{self, Dec};
use helpers_native::lint_construct;
use helpers_native::lint_english;
use helpers_native::lint_module::{self, Outcome};
use helpers_native::lint_trace::{run_plan, Plan};

fn decode_crawl(bytes: &[u8]) -> Option<Vec<(String, String)>> {
    let (_s, mut d) = Dec::open(bytes, lint_codec::kind::CRAWL)?;
    let _v = d.str()?;
    let _c = d.fixed_u64()?;
    let n = d.u()? as usize;
    let mut pages = Vec::with_capacity(n.min(65_536));
    for _ in 0..n {
        let url = d.str()?;
        let body = d.str()?;
        let _ = d.boolean()?;
        let _ = d.fixed_u64()?;
        let _ = d.fixed_u64()?;
        pages.push((url, body));
    }
    Some(pages)
}
fn load(name: &str) -> Vec<(String, String)> {
    let home = std::env::var("HOME").unwrap();
    let dir = format!("{home}/.cache/helpers/lint-index/crawls");
    std::fs::read(format!("{dir}/{name}.bin")).ok().and_then(|b| decode_crawl(&b)).unwrap_or_default()
}

/// Harvest `<code>` interiors as a reference corpus (measurement scaffolding, as in web_module_train).
fn code_interiors(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lb: Vec<u8> = body.bytes().map(|b| b.to_ascii_lowercase()).collect();
    let find = |from: usize, needle: &[u8]| -> Option<usize> {
        if from > lb.len() { return None; }
        lb[from..].windows(needle.len()).position(|w| w == needle).map(|p| from + p)
    };
    let mut i = 0;
    while let Some(open) = find(i, b"<code") {
        let Some(gt) = find(open, b">").map(|g| g + 1) else { break };
        let Some(close) = find(gt, b"</code>") else { break };
        if !body.is_char_boundary(gt) || !body.is_char_boundary(close) { i = open + 5; continue; }
        let raw = &body[gt..close];
        let mut txt = String::new();
        let mut intag = false;
        for ch in raw.chars() {
            match ch { '<' => intag = true, '>' => intag = false, _ if !intag => txt.push(ch), _ => {} }
        }
        let txt = txt.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&").replace("&quot;", "\"");
        if txt.trim().len() >= 2 { out.push(txt.trim().to_string()); }
        i = close + 7;
    }
    out
}
fn reconstruct_memory(pages: &[(String, String)]) -> helpers_native::lint_read::Memory {
    let mut mem = helpers_native::lint_read::Memory::default();
    let mut seen = std::collections::HashSet::new();
    for (_u, body) in pages {
        for block in code_interiors(body) {
            if seen.insert(block.clone()) { mem.reference.push(block); }
        }
    }
    mem
}

/// (bad file using genuinely-deprecated items, clean idiomatic file, constructs we hope to see proven).
/// The clean files deliberately use the deprecations' own RECOMMENDED REPLACEMENTS and idioms that share
/// member names with dead junk shapes (`s.split`, `path.name`, `collections.abc.Sequence`) — the junk
/// floor is proven exactly by those NOT flagging.
fn fixtures(lang: &str) -> (&'static str, &'static str, Vec<&'static str>) {
    match lang {
        "python" => (
            "import codecs\nimport typing\nf = codecs.open('x.txt')\nd: typing.Dict[str, int] = {}\nl: typing.List[int] = []\nmod = loader.load_module('m')\n",
            "import collections.abc\nfrom pathlib import Path\ns = 'a,b'.split(',')\np = Path('x')\nprint(p.name)\nq: collections.abc.Sequence[int] = []\nwith open('x.txt') as f:\n    print(f.read())\n",
            vec!["codecs.open", "typing.Dict", "typing.List"],
        ),
        "rust" => (
            "fn main() {\n    let s = \" hi \";\n    let a = s.trim_left();\n    let b = s.trim_right();\n    let x = (1.5f32).abs_sub(2.0);\n    let v = vec![\"a\", \"b\"];\n    let j = v.connect(\",\");\n}\n",
            "fn main() {\n    let s = \" hi \";\n    let a = s.trim_start();\n    let b = s.trim_end();\n    let v = vec![\"a\", \"b\"];\n    let j = v.join(\",\");\n    println!(\"{a}{b}{j}\");\n}\n",
            vec![".trim_left", ".trim_right", ".abs_sub", ".connect"],
        ),
        _ => ("", "", vec![]),
    }
}

fn main() {
    let (Some(br), Some(en)) = (lint_char::brain(), lint_english::brain()) else {
        eprintln!("no frozen brains on disk"); return;
    };
    let m = br.meanings();
    // PARTITION ∅ CROSS-CHECK: graduate each language over the UNION of both corpora — the grammar
    // partition must keep every rule on its own language's pages (py∩rust = ∅).
    let union: Vec<(String, String)> = { let mut u = load("python-library"); u.extend(load("rust-std")); u };
    for lang in ["python", "rust"] {
        let mem = reconstruct_memory(&union);
        let constructions = lint_construct::load(lang);
        let all_urls: std::collections::HashSet<String> = union.iter().map(|(u, _)| u.clone()).collect();
        let (outcomes, _, _, _, _) = lint_module::graduate(lang, union.clone(), &mem, m, en, &constructions, &all_urls);
        let cross: Vec<String> = outcomes.iter()
            .filter_map(|o| o.rule.as_ref().map(|(_, url)| url.clone()))
            .filter(|u| if lang == "python" { u.contains("rust-lang") } else { u.contains("python.org") })
            .collect();
        println!("UNION {lang}: {} proven, cross-language sources: {} {:?}",
                 outcomes.iter().filter(|o| o.rule.is_some()).count(), cross.len(), cross.iter().take(4).collect::<Vec<_>>());
    }
    for (lang, crawl) in [("python", "python-library"), ("rust", "rust-std")] {
        let pages = load(crawl);
        let att = Attestation::discover(&pages);
        let attested = pages.iter().filter(|(_, b)| att.attests(b)).count();
        let memory = reconstruct_memory(&pages);
        println!("==== {lang}: {} pages, {} attested-deprecated, {} ref blocks ====",
                 pages.len(), attested, memory.reference.len());
        // DEBUG: inspect the first attested page's structural reading + example corpus.
        let probe_url = if lang == "python" { "ssl.html" } else { "trait.Error.html" };
        if let Some((u, b)) = pages.iter().find(|(uu, bb)| uu.contains(probe_url) && att.attests(bb)) {
            let aset: std::collections::HashSet<String> =
                pages.iter().filter(|(_, bb)| att.attests(bb)).map(|(uu, _)| uu.clone()).collect();
            let bridge = helpers_native::lint_trace::Bridge::new(m, en);
            let construction_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
            let dp = helpers_native::lint_lang_layer::read_doc_page(u, b, en, &bridge, &aset, &construction_map);
            let corpus: Vec<String> = code_interiors(b).into_iter().filter(|s| s.trim().len() >= 3).take(40).collect();
            println!("   DEBUG page {u}");
            println!("     has class=deprecated: {}  has dotted-id: {}",
                     b.contains("class=\"deprecated\"") || b.contains("stab deprecated"),
                     b.contains("id=\"ssl.") || b.contains("id=\"method."));
            println!("     constructs read ({}): {:?}", dp.constructs.len(), &dp.constructs.iter().take(20).collect::<Vec<_>>());
            // Which read constructs FIRE cleanly on the UNCAPPED code corpus (all <code> interiors)?
            let full: Vec<String> = code_interiors(b).into_iter().filter(|s| s.trim().len() >= 3).collect();
            println!("     UNCAPPED corpus blocks: {}", full.len());
            let mut fired = 0;
            for c in &dp.constructs {
                let fires = full.iter().any(|blk| helpers_native::lint_trace::parses_cleanly(lang, blk)
                    && !run_plan(&Plan::UsesConstruct { construct: c.clone() }, lang, blk).is_empty());
                if fires { fired += 1; if fired <= 12 { println!("        FIRES {c:?}"); } }
            }
            println!("     total firing constructs (uncapped): {fired}");
        }
        // Replicate the EXACT partition gate (page_proves_in_lang: FIRST block containing the construct
        // must parse cleanly AND fire) across every attested page — count how many pages pass.
        let cin = |code: &str, c: &str| -> bool {
            if c.starts_with('.') { return code.contains(c); }
            let is_sym = c.chars().all(|ch| !ch.is_ascii_alphanumeric() && ch != '.');
            if is_sym { return code.split_whitespace().any(|t| t.trim_matches(|x: char| "();{},".contains(x)) == c); }
            let b = code.as_bytes(); let mut from = 0;
            while let Some(rel) = code[from..].find(c) {
                let s = from + rel; let e = s + c.len();
                let bd = |x: u8| !(x.is_ascii_alphanumeric() || x == b'.');
                if (s == 0 || bd(b[s-1])) && (e >= b.len() || bd(b[e])) { return true; }
                from = s + 1;
            }
            false
        };
        let aset: std::collections::HashSet<String> =
            pages.iter().filter(|(_, bb)| att.attests(bb)).map(|(uu, _)| uu.clone()).collect();
        let bridge = helpers_native::lint_trace::Bridge::new(m, en);
        let construction_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        let mut passing = 0; let mut sample: Vec<String> = Vec::new();
        for (u, b) in pages.iter().filter(|(_, bb)| att.attests(bb)) {
            let dp = helpers_native::lint_lang_layer::read_doc_page(u, b, en, &bridge, &aset, &construction_map);
            if !dp.prohibited || dp.constructs.is_empty() { continue; }
            let own: Vec<String> = code_interiors(b).into_iter().map(|s| s.trim().to_string()).filter(|s| s.len() >= 3).collect();
            let pass = dp.constructs.iter().any(|c| {
                own.iter().find(|blk| cin(blk, c)).map(|blk|
                    helpers_native::lint_trace::parses_cleanly(lang, blk) && !run_plan(&Plan::UsesConstruct { construct: c.clone() }, lang, blk).is_empty()
                ).unwrap_or(false)
            });
            if pass { passing += 1; if sample.len() < 6 { sample.push(u.rsplit('/').next().unwrap_or(u).to_string()); } }
        }
        println!("   PARTITION GATE: {passing} of {} attested pages pass (fire on own example); sample {:?}", aset.len(), sample);
        // PROPOSE-path debug: partition-passing pages' marked shapes + the chosen firing shape.
        let probes: &[&str] = if lang == "python" {
            &["re.html", "collections.abc.html", "gzip.html", "select.html", "asyncio-task.html", "urllib.parse.html"]
        } else {
            &["struct.Vec.html"]
        };
        for probe2 in probes {
            let Some((u, b)) = pages.iter().find(|(uu, bb)| uu.contains(probe2) && att.attests(bb)) else { continue };
            let dp = helpers_native::lint_lang_layer::read_doc_page(u, b, en, &bridge, &aset, &construction_map);
            let ex = &dp.example_code;
            let chosen = dp.constructs.iter().find(|c| ex.iter().any(|blk| cin(blk, c) && !run_plan(&Plan::UsesConstruct { construct: (*c).clone() }, lang, blk).is_empty()));
            println!("   PROPOSE-DEBUG {probe2}: marked ({}): {:?} chosen={chosen:?}",
                     dp.marked_deprecated.len(), dp.marked_deprecated.iter().take(12).collect::<Vec<_>>());
        }
        let cands = lint_module::proposed(lang, &pages, &memory, m, en);
        println!("   proposed() candidates: {} ; sample {:?}", cands.len(),
                 cands.iter().take(8).map(|c| c.construct.clone()).collect::<Vec<_>>());
        let t = std::time::Instant::now();
        let constructions = lint_construct::load(lang);
        let (outcomes, _, _, _, _) = lint_module::graduate(lang, pages.clone(), &memory, m, en, &constructions, &aset);
        let proven: Vec<&Outcome> = outcomes.iter().filter(|o| o.rule.is_some()).collect();
        println!("   {} candidates, {} PROVEN, {:.2}s", outcomes.len(), proven.len(), t.elapsed().as_secs_f64());
        for o in proven.iter().take(30) {
            if let Some((r, url)) = &o.rule {
                println!("   PROVEN {:24} fire={:4} id={:28} src={}", o.candidate.construct, o.violating, r.id, url);
            }
        }
        let mut near: Vec<&Outcome> = outcomes.iter().filter(|o| o.rule.is_none() && o.violating >= 3).collect();
        near.sort_by_key(|o| std::cmp::Reverse(o.violating));
        for o in near.iter().take(15) {
            println!("   unproven {:22} fire={:4} {:?}", o.candidate.construct, o.violating, o.verdict);
        }
        // Acceptance
        let (bad, good, wanted) = fixtures(lang);
        let mut bad_flags = 0;
        for o in &proven {
            let plan = Plan::UsesConstruct { construct: o.candidate.construct.clone() };
            let b = run_plan(&plan, lang, bad);
            if !b.is_empty() { bad_flags += 1; println!("   BAD flag {:20} lines {:?}", o.candidate.construct, b); }
        }
        let good_flags: Vec<String> = proven.iter()
            .filter(|o| !run_plan(&Plan::UsesConstruct { construct: o.candidate.construct.clone() }, lang, good).is_empty())
            .map(|o| o.candidate.construct.clone()).collect();
        println!("   acceptance: bad flagged by {bad_flags} rules; good wrongly flagged by {good_flags:?}");
        for w in &wanted {
            println!("   want {w:20} -> {}", if proven.iter().any(|o| &o.candidate.construct == w) { "PROVEN" } else { "missing" });
        }
        println!();
    }
}
