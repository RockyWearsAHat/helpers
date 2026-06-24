# Design & Complexity

## Abstraction

The `Store` trait is the single seam in the design. By programming to the trait,
callers are decoupled from the concrete `HashStore`; an alternative
implementation (a `BTreeMap`-backed ordered store, say) could be dropped in
without changing any caller.

## Data structure choice

`HashStore` uses a `HashMap<String, String>`. This is the right structure for an
unordered key/value store where the dominant access pattern is point lookup by
key:

- `get` — O(1) average, O(n) worst case (hash collisions).
- `put` — O(1) average (amortized over occasional rehashing).
- `len` / `is_empty` — O(1).

If ordered iteration over keys were required, a `BTreeMap` (O(log n) operations)
would be the better fit. The trait makes that substitution local.

## Testing

`tests/store_test.rs` exercises insertion, replacement (verifying the previous
value is returned), and the empty-store edge cases, asserting on both the return
values and the resulting length.
