//! THROWAWAY (untracked): rebuild the char brain after a BRAIN_REV bump (rung 1 markdown stage).
//! Run: `cargo run --release --features crawl --example rebuild_brain`
use helpers_native::lint_char;
use std::path::PathBuf;

fn main() {
    let _ = helpers_native::lint_english::ensure_built();
    // The repo root holds `extraDocs/` — the same discriminator `tools::lint::data_root` uses;
    // registered doc sources resolve from here, the crawl cache is read from `~/.cache/helpers`.
    let data = PathBuf::from("/Users/alexwaldmann/bin");
    match lint_char::ensure_brain(&data) {
        Some(msg) => println!("{msg}"),
        None => eprintln!("ensure_brain returned None (no dictionary/web?)"),
    }
    if let Some(br) = lint_char::brain() {
        println!("roles learned: {}", br.structure().len());
        // Did the markdown markers earn learned roles? (fence -> code +1, heading -> boundary -1)
        let fence = helpers_native::lint_ai::token_seed("```");
        let head = helpers_native::lint_ai::token_seed("#");
        println!("fence role: {:?}  heading role: {:?}", br.structure_role(fence), br.structure_role(head));
        for name in ["pre", "code", "h1", "h2", "h3", "table"] {
            let s = helpers_native::lint_ai::token_seed(name);
            println!("  <{name}> role: {:?}", br.structure_role(s));
        }
        println!("title ceiling: {}", br.structure().title_ceiling());

        // Segment a REAL markdown file with the live rebuilt brain.
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join(".cache/helpers/mattpocock-skills");
        let mut probed = 0usize;
        let mut with_units = 0usize;
        let mut example: Option<(String, usize)> = None;
        let mut stack = vec![dir];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "md") {
                    if let Ok(text) = std::fs::read_to_string(&p) {
                        probed += 1;
                        let units = helpers_native::lint_graph::read_markdown(&text, br);
                        if !units.is_empty() {
                            with_units += 1;
                            if example.is_none() && units.iter().any(|u| u.code.contains('\n')) {
                                example = Some((p.display().to_string(), units.len()));
                            }
                        }
                    }
                }
            }
        }
        println!("markdown files probed: {probed}, with >=1 code-fence unit: {with_units}");
        if let Some((f, n)) = example {
            println!("example: {f} -> {n} units");
        }
    }
}
