//! White-box + black-box integration test for the knowledge subsystem: build
//! the local index, search it, keyword-search the cache, and exercise note
//! CRUD — all against a temp workspace with a pre-seeded (offline) community
//! cache so the test is deterministic and never touches the network.

use std::fs;
use std::path::Path;

use gsh_native::knowledge::index::{build_knowledge_index, search_knowledge_index};
use gsh_native::knowledge::notes::{
    read_knowledge_file_content, read_knowledge_note, search_knowledge_cache,
};
use gsh_native::knowledge::KnowledgeConfig;
use gsh_native::util::now_millis;
use serde_json::json;

fn mkcfg(tmp: &Path) -> KnowledgeConfig {
    let ws = tmp.join("ws");
    let kr = ws.join("knowledge");
    let cache = tmp.join("cache");
    fs::create_dir_all(&kr).unwrap();
    fs::create_dir_all(&cache).unwrap();
    // Seed a fresh, empty community index so fetch_community_index uses the cache
    // (no network call) and returns an index with no files.
    fs::write(
        cache.join("_cache_meta.json"),
        format!(
            r#"{{"etags":{{}},"fetched_at":{{"knowledge/_index.json":{}}}}}"#,
            now_millis()
        ),
    )
    .unwrap();
    fs::write(
        cache.join("knowledge__index.json"),
        r#"{"version":1,"built_at":"x","file_count":0,"idf":{},"files":{},"posting":{}}"#,
    )
    .unwrap();
    KnowledgeConfig {
        workspace_root: ws.clone(),
        repo_root: tmp.join("repo"),
        knowledge_root: kr.clone(),
        repo_knowledge_root: tmp.join("repo").join("knowledge"),
        local_index_path: kr.join("_index.json"),
        github_cache_dir: cache.clone(),
        cache_meta_path: cache.join("_cache_meta.json"),
    }
}

#[test]
fn build_search_and_note_crud() {
    let tmp = std::env::temp_dir().join(format!("gsh-kn-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    let cfg = mkcfg(&tmp);

    fs::write(
        cfg.knowledge_root.join("networking-dns.md"),
        "# DNS Resolution\nThe domain name system resolves hostnames to addresses using recursive resolvers and caching nameservers.\n",
    )
    .unwrap();
    fs::write(
        cfg.knowledge_root.join("algorithms-graph.md"),
        "# Graph Algorithms\nBreadth first search and depth first search traverse graph vertices and edges efficiently.\n",
    )
    .unwrap();

    // ── build (white box: counts reflect the corpus) ────────────────────────
    let built = build_knowledge_index(&cfg).expect("build index");
    assert_eq!(built.file_count, 2);
    assert!(built.term_count > 0);
    assert!(cfg.local_index_path.exists());

    // ── TF-IDF search (black box: relevant note surfaces) ───────────────────
    let read = |fname: &str| read_knowledge_file_content(&cfg, fname);
    let search =
        search_knowledge_index(&cfg, "dns resolver nameserver", 5, &read).expect("index search");
    assert!(search.local);
    assert!(
        search
            .results
            .iter()
            .any(|h| h.path.contains("networking-dns")),
        "expected networking-dns in results"
    );

    // ── keyword cache search ────────────────────────────────────────────────
    let cache = search_knowledge_cache(&cfg, "graph traversal", 5).expect("cache search");
    assert!(cache
        .results
        .iter()
        .any(|h| h.path.contains("algorithms-graph")));

    // ── note CRUD (black box behaviors) ─────────────────────────────────────
    use gsh_native::knowledge::notes::{
        append_to_knowledge_note, update_knowledge_note, write_knowledge_note,
    };

    let w = write_knowledge_note(
        &cfg,
        &json!({ "path": "new-note.md", "content": "# New Note\nbody one\n" }),
    )
    .expect("write");
    assert_eq!(w.action, "created");
    assert!(cfg.knowledge_root.join("new-note.md").exists());

    // Overwrite guard.
    match write_knowledge_note(&cfg, &json!({ "path": "new-note.md", "content": "x" })) {
        Err(e) => assert!(e.contains("already exists"), "got: {e}"),
        Ok(_) => panic!("expected overwrite to be blocked"),
    }

    let a = append_to_knowledge_note(
        &cfg,
        &json!({ "path": "new-note.md", "content": "appended line" }),
    )
    .expect("append");
    assert_eq!(a.action, "appended");

    let u = update_knowledge_note(
        &cfg,
        &json!({ "path": "new-note.md", "heading": "New Note", "content": "replaced body" }),
    )
    .expect("update");
    assert_eq!(u.action, "updated");

    // update replaces the whole section under "New Note", so the appended line
    // (which lived under that heading) is correctly gone.
    let note = read_knowledge_note(&cfg, "new-note.md", 0).expect("read");
    assert!(note.text.contains("replaced body"), "got: {}", note.text);
    assert!(
        !note.text.contains("appended line"),
        "section should be replaced"
    );
    assert_eq!(note.source, "workspace");

    // Path containment guard.
    let escape = read_knowledge_note(&cfg, "../../etc/passwd", 0);
    assert!(escape.is_err());

    let _ = fs::remove_dir_all(&tmp);
}
