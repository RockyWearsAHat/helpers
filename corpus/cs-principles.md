# CS2420 / CS3500 Principles

The hard-defined, always-good practices the reasoner holds for *every* project, regardless of
language module. This is a plain document — edit it, add your own rules, and the reasoner learns
them with no code change. Narrative sections (no code block) are context for humans; a section
with a `bad`/`good` code pair becomes a pattern the reasoner can detect and advise on.

## Single responsibility (behavioral)

A function should do one thing. When a unit juggles many distinct calls, branches, and loops it is
doing too much — split it. This is judged distributionally against how the project itself writes
code (the behavioral norm), not by a fixed number.

## Complexity (behavioral)

Prefer shallow, simple control flow. Deep nesting and many decision points make code hard to read
and test. Judged against the project's own norm.

## Error handling (behavioral)

Never force or silently discard a fallible result. Handle the error — propagate it, log it, or
recover — rather than unwrapping or ignoring it.

## Off by one indexing [high]

Indexing a collection with an inclusive range up to its length reads one element past the end — a
classic off-by-one that panics or corrupts memory. Use an exclusive range, or iterate directly.

```rust:bad
fn sum(xs: &[i32]) -> i32 {
    let mut total = 0;
    for i in 0..=xs.len() {
        total += xs[i];
    }
    total
}
```

```rust:good
fn sum(xs: &[i32]) -> i32 {
    let mut total = 0;
    for x in xs {
        total += x;
    }
    total
}
```

## Idiomatic emptiness check [low]

Test emptiness with `is_empty()`, not by comparing the length to zero. It states intent and is
correct for collections whose length is expensive to compute.

```rust:bad
fn describe(items: &[i32]) -> &str {
    if items.len() == 0 {
        "empty"
    } else {
        "has items"
    }
}
```

```rust:good
fn describe(items: &[i32]) -> &str {
    if items.is_empty() {
        "empty"
    } else {
        "has items"
    }
}
```
