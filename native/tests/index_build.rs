//! Integration test: build a project index over a small temp repo and verify
//! symbol extraction, the reference graph, and ranking.

use std::fs;

use gsh_native::index::build::build_index;

fn write(dir: &std::path::Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, body).unwrap();
}

#[test]
fn builds_graph_and_ranks_referenced_files_higher() {
    let dir = std::env::temp_dir().join(format!("gsh-idx-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    // core.rs defines `compute`; two callers reference it.
    write(
        &dir,
        "src/core.rs",
        "pub fn compute(x: i32) -> i32 { x + 1 }\n",
    );
    write(
        &dir,
        "src/a.rs",
        "use crate::core::compute;\nfn run_a() { let _ = compute(1); }\n",
    );
    write(
        &dir,
        "src/b.rs",
        "use crate::core::compute;\nfn run_b() { let _ = compute(2); }\n",
    );
    write(&dir, "tool.sh", "#!/bin/bash\ndeploy() {\n  echo hi\n}\n");
    write(&dir, "README.md", "# Title\n## Section\nbody\n");

    let index = build_index(&dir);

    // Files indexed (5 source files).
    assert_eq!(index.file_count, 5, "expected 5 files");

    // core.rs defines `compute`.
    let core = index
        .files
        .iter()
        .find(|f| f.path == "src/core.rs")
        .expect("core.rs indexed");
    assert!(core.defs.iter().any(|d| d.name == "compute"));

    // Shell function extracted via fallback.
    let sh = index.files.iter().find(|f| f.path == "tool.sh").unwrap();
    assert!(sh.defs.iter().any(|d| d.name == "deploy"), "shell fn");
    assert_eq!(sh.lang, "shell");

    // Markdown headings extracted.
    let readme = index.files.iter().find(|f| f.path == "README.md").unwrap();
    assert!(readme.headings.iter().any(|h| h == "Title"));
    assert!(readme.headings.iter().any(|h| h == "Section"));

    // Edges connect a.rs and b.rs -> core.rs via `compute`.
    let core_idx = index
        .files
        .iter()
        .position(|f| f.path == "src/core.rs")
        .unwrap();
    let edges_to_core = index.edges.iter().filter(|e| e.to == core_idx).count();
    assert_eq!(edges_to_core, 2, "two callers should reference core.rs");
    assert!(index
        .edges
        .iter()
        .any(|e| e.to == core_idx && e.via.iter().any(|v| v == "compute")));

    // core.rs (referenced twice) should outrank its callers.
    let ranked = index.ranked();
    assert_eq!(
        ranked[0].path, "src/core.rs",
        "most-referenced file ranks first"
    );

    let _ = fs::remove_dir_all(&dir);
}
