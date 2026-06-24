//! Key/value store abstraction and a hash-map-backed implementation.

use std::collections::HashMap;

/// A string-keyed store of string values. Programming against this trait keeps
/// callers independent of the concrete storage strategy.
pub trait Store {
    /// Insert or replace `key`'s value, returning the previous value if any.
    fn put(&mut self, key: &str, value: &str) -> Option<String>;

    /// Return the value stored under `key`, or `None` if absent.
    fn get(&self, key: &str) -> Option<&str>;

    /// Number of distinct keys currently stored.
    fn len(&self) -> usize;

    /// Whether the store holds no entries.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A [`Store`] backed by a `HashMap`, giving O(1) average-case lookup/insert.
#[derive(Default)]
pub struct HashStore {
    entries: HashMap<String, String>,
}

impl HashStore {
    /// Create an empty store.
    pub fn new() -> Self {
        HashStore {
            entries: HashMap::new(),
        }
    }
}

impl Store for HashStore {
    fn put(&mut self, key: &str, value: &str) -> Option<String> {
        self.entries.insert(key.to_string(), value.to_string())
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}
