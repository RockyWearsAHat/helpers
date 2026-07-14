//! THROWAWAY PROBE (untracked): call the LIVE graduated path exactly as `lint_train::doc_rules` does —
//! `lint_module::graduated_rules(lang, memory)` over `lint_docs::raw_pages(lang)` — for the web stack, and
//! report how many rules graduate on THIS machine's caches. This is the item-5 reproduction check: the
//! harness (reading the crawl bins directly) graduates css 31 / html 8, but the LIVE seam depends on
//! `raw_pages` returning the notecard/reference pages; if it comes back thin (offline/version drift), the
//! flip falls to the miner and box-orient never fires live. Prints the live page count and rule count so
//! the gap is measured, not guessed.
//! Run: `cargo run --release --features crawl --example live_grad_probe`
use helpers_native::lint_module;
use helpers_native::lint_train;

fn main() {
    for lang in ["css", "html", "javascript", "svg"] {
        let mem = lint_train::cached_memory(lang).unwrap_or_default();
        let rules = lint_module::graduated_rules(lang, &mem).rules;
        println!(
            "{:12} cached_memory: {} bindings / {} ref-blocks | graduated_rules -> {} rule(s){}",
            lang,
            mem.bindings.len(),
            mem.reference.len(),
            rules.len(),
            if rules.len() >= 3 { "  [>=FLOOR: would FLIP to graduated]" } else { "  [<FLOOR: stays on MINER]" }
        );
        for (r, url) in rules.iter().take(6) {
            println!("      {:24} construct={:?}  {}", r.id, r.construct, url);
        }
    }
}
