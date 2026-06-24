# keyvalue

A tiny in-memory key/value store, used as a multi-language grading fixture for
`git-cs-grade`. It demonstrates the language-agnostic CS principles the rubric
grades — clear abstraction, appropriate data structures, documentation, and
tested behaviour — expressed in idiomatic Rust rather than Java.

## Design

The crate is organized around a single abstraction, the `Store` trait, which
defines the operations a key/value store must support: `put`, `get`, `len`, and
`is_empty`. Callers depend on the trait, not on any concrete type, so the
storage strategy can change without touching call sites.

`HashStore` is the default implementation. It is backed by the standard
library's `HashMap`, which gives O(1) average-case complexity for both lookups
and insertions. Because keys and values are owned `String`s, the store takes
copies on insert and hands out borrowed slices on read.

## Building and running

This is a library crate. Build it with:

```
cargo build
```

Run the test suite with:

```
cargo test
```

## Layout

- `src/lib.rs` — crate root; re-exports the public API.
- `src/store.rs` — the `Store` trait and the `HashStore` implementation.
- `tests/store_test.rs` — integration tests covering insert, replace, and the
  empty-store edge cases.
- `docs/design.md` — a short design and complexity writeup.

## Complexity

All core operations (`get`, `put`) are O(1) on average thanks to hashing. The
`len` and `is_empty` queries are O(1) as well, since the underlying map tracks
its own size.
