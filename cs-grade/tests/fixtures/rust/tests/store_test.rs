//! Integration tests for the key/value store.

use keyvalue::{HashStore, Store};

#[test]
fn put_then_get_returns_value() {
    let mut store = HashStore::new();
    assert_eq!(store.put("a", "1"), None);
    assert_eq!(store.get("a"), Some("1"));
    assert_eq!(store.len(), 1);
}

#[test]
fn put_replaces_and_returns_previous() {
    let mut store = HashStore::new();
    store.put("a", "1");
    assert_eq!(store.put("a", "2"), Some("1".to_string()));
    assert_eq!(store.get("a"), Some("2"));
}

#[test]
fn empty_store_reports_empty() {
    let store = HashStore::new();
    assert!(store.is_empty());
    assert_eq!(store.get("missing"), None);
}
