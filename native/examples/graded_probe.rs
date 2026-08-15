//! THROWAWAY PROBE (untracked): PASS 26 RUNG B sizing — enumerate the ATTESTED-but-NOT-graduated web
//! nodes (a revoked doc-role — deprecated/removal/prohibition — but `proven==false`, no rule) per language
//! web on disk. These are the constructs that today FIRE NOTHING (abstention) and would gain the LOW tier.
//! Run: `cargo run --release --example graded_probe`
use helpers_native::lint_web;

// A construct safe to fire a LOW graded finding on: a qualified `receiver.member` where BOTH sides are
// real identifiers (≥2 chars, alnum/_), the member is distinctive, and it is NOT a doc URL basename.
// Excludes bare leading-dot members (`.read`), URL basenames (`struct.Vec.html`), and bare short generics.
fn qualified_safe(c: &str) -> bool {
    if c.starts_with('.') { return false; }
    let doc_ext = [".html", ".htm", ".php"];
    if doc_ext.iter().any(|e| c.ends_with(e)) { return false; }
    // rustdoc anchor forms (method.x / struct.X / primitive.x) are NOT code — drop them
    let anchor_pref = ["method.", "struct.", "primitive.", "trait.", "enum.", "fn.", "macro.", "constant.", "type.", "mod.", "keyword.", "associatedtype.", "union.", "associatedconstant."];
    if anchor_pref.iter().any(|p| c.to_lowercase().starts_with(p)) { return false; }
    let parts: Vec<&str> = c.split('.').collect();
    if parts.len() < 2 { return false; } // require a qualifier (receiver.member)
    parts.iter().all(|p| p.len() >= 2 && p.chars().next().is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
                    && p.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_'))
}

fn main() {
    let langs = lint_web::languages_with_web();
    println!("languages with a web sidecar: {langs:?}\n");
    let revoked = ["deprecated", "removal", "prohibition"];
    for lang in &langs {
        let web = lint_web::load(lang);
        if web.is_empty() { continue; }
        let proven = web.iter().filter(|n| n.proven).count();
        let read = web.iter().filter(|n| !n.proven).count();
        // unproven nodes carrying a revoked role
        let graded: Vec<&_> = web.iter()
            .filter(|n| !n.proven && n.roles.iter().any(|r| revoked.contains(&r.as_str())))
            .collect();
        // proven nodes carrying a revoked role (already enforced — not graded)
        let proven_revoked = web.iter()
            .filter(|n| n.proven && n.roles.iter().any(|r| revoked.contains(&r.as_str())))
            .count();
        // SAFE gate: qualified receiver.member (both sides real idents), OR distinctive standalone —
        // never a bare leading-dot member, a URL basename, or a short generic single identifier.
        let safe: Vec<&_> = graded.iter().filter(|n| qualified_safe(&n.construct)).copied().collect();
        println!("== {lang}: {} nodes ({proven} proven, {read} read) | revoked proven={proven_revoked}: {} GRADED-all, {} GRADED-SAFE ==",
                 web.len(), graded.len(), safe.len());
        for n in &safe {
            println!("    SAFE [{}] {}", n.roles.join(","), n.construct);
        }
    }
}
