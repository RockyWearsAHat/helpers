# Lint Index — packed, version-matched, official-doc-sourced rule catalog

The lint index is the **packed, committed** rule catalog Helpers ships and keeps
current. It is the fast path: a project's toolchain versions are checksum-matched
against the packed index and used directly — no fetch, no crawl, no model. The
shipped crawler/indexer is a **last resort**, run only to fill a gap, and its
result is submitted back so future users get it for free.

## Tiered resolution (the runtime contract)

For a project, for each detected tool/language:

1. **Detect** the toolchain + version (e.g. `rustc --version` → `1.95.0`).
2. **Fast path:** find `lint-index/<tool>.json`, verify its `checksum`, and check
   `toolchainVersion`/`docsVersion` cover the detected version. Match → use it.
   This is O(read + hash) — the common case, instant.
3. **Poll/pull:** periodically (and on miss) pull index updates from the live
   Helpers repo (reuse the `community-cache` machinery). A newer packed index may
   already cover the version.
4. **Crawl-on-miss (last resort):** only if still uncovered (version mismatch, or
   a rule set never crawled) does the shipped crawler/indexer fetch the official
   docs for that version, **expand the index**, and **submit it back** (PR) so it
   becomes the packed fast path for everyone next time.

Crawling is strictly slower than a checksum; never crawl when the packed index
already covers the toolchain.

## File: `lint-index/<tool>.json`

One file per tool (`clippy`, `ruff`, `eslint`, `go-vet`, …). Version coverage is
recorded inside; a tool may keep multiple version snapshots if needed.

```json
{
  "tool": "clippy",
  "language": "rust",
  "toolchainVersion": "1.95.0",
  "docsVersion": "1.82.0",
  "source": "rust-clippy",
  "docsBase": "https://rust-lang.github.io/rust-clippy/rust-1.82.0",
  "fetchedAt": "2026-06-20T...Z",
  "checksum": "sha256:<hex of the canonical-serialized rules array>",
  "ruleCount": 294,
  "rules": [
    {
      "id": "unwrap_used",
      "category": "correctness",
      "severity": "high",
      "description": "Checks for `.unwrap()` calls on Result/Option…",
      "exampleBad": "x.unwrap()",
      "source": "https://rust-lang.github.io/rust-clippy/master/index.html#unwrap_used"
    }
  ]
}
```

### Field contract

- `tool` — stable tool id; the filename stem.
- `language` — lowercase language id the tool lints.
- `toolchainVersion` — the toolchain version this snapshot targets.
- `docsVersion` — the official-docs version the rules were sourced from (may lag
  the toolchain; the resolver picks the newest published ≤ toolchain).
- `source`, `docsBase`, `fetchedAt` — provenance.
- `checksum` — `sha256:` of the canonical JSON of the `rules` array (sorted by
  `id`, no whitespace). The fast-path integrity/version check compares this; it
  is how Helpers decides "packed index is current" without refetching.
- `ruleCount` — `rules.length`.
- `rules[]` — `id`, `category`, `severity` (`high|medium|low`), `description`,
  optional `exampleBad`/`exampleGood`, and a direct `source` URL. Every rule is
  sourced **directly from the official documentation** — no hand-authoring.

## Determinism

Given the same toolchain version and the same official docs, the packed index is
byte-stable (canonical serialization + checksum), so committing it and pulling it
is reproducible across machines.
