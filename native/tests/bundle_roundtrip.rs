//! Integration test: export a project's index to a `.dxbundle` and install it
//! into another project, verifying the round-trip preserves the graph + docs.

use std::fs;

use gsh_native::index::bundle::{export_bundle, install_bundle, list_refs};

fn write(dir: &std::path::Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, body).unwrap();
}

#[test]
fn export_then_install_roundtrips() {
    let base = std::env::temp_dir().join(format!("gsh-bundle-{}", std::process::id()));
    let src = base.join("libproj");
    let dst = base.join("hostproj");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    write(&src, "src/api.rs", "pub fn entry() {}\n");
    write(&src, "src/use.rs", "fn caller() { entry(); }\n");

    // Export the source project's index.
    let bundle_path = base.join("libproj.dxbundle");
    let bundle = export_bundle(&src, &bundle_path).expect("export");
    assert_eq!(bundle.name, "libproj");
    assert!(bundle.file_count >= 2);
    assert!(bundle.docs.contains_key("map.dx"));
    assert!(bundle_path.exists());

    // Install it into the host project.
    let name = install_bundle(&dst, &bundle_path).expect("install");
    assert_eq!(name, "libproj");

    let ref_dir = dst.join(".gsh").join("index").join("refs").join("libproj");
    assert!(ref_dir.join("graph.json").exists());
    assert!(ref_dir.join("map.dx").exists());

    assert_eq!(list_refs(&dst), vec!["libproj".to_string()]);

    // The installed graph is parseable and carries the symbol.
    let installed: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(ref_dir.join("graph.json")).unwrap()).unwrap();
    assert!(installed["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["path"] == "src/api.rs"));

    let _ = fs::remove_dir_all(&base);
}
