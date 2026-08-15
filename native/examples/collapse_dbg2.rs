//! throwaway: name the proven rules the module lost.
fn main() {
    let web = helpers_native::lint_web::load("javascript");
    let derived = helpers_native::lint_web::derive_rules("javascript", &web);
    for (r, _) in &derived {
        println!("DERIVED {}", r.id);
    }
}
