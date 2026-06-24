//! A tiny in-memory key/value store used as a Rust grading fixture.
//!
//! The crate exposes a [`Store`] trait (the abstraction) and a [`HashStore`]
//! implementation backed by a `HashMap` for O(1) average-case `get`/`put`.

pub mod store;

pub use store::{HashStore, Store};
