# Dependency Resolution: Algorithms, Constraints, and Conflict Resolution

## Overview

Dependency resolution is a constraint satisfaction problem: given a set of packages and their version constraints, find a valid assignment of versions that satisfies all constraints. This is computationally hard (NP-complete for version constraints in general), which is why different package managers use different strategies. Understanding the algorithms helps predict behavior, debug conflicts, and avoid dependency hell.

## The Core Problem: Dependency Hell

### Diamond Dependency

The canonical conflict: package A depends on both B and C, which depend on different versions of D.

```
       A
      / \
     B   C
      \ /
       D (B needs v1, C needs v2)
```

**Question**: Can we use both B v1.0 and C v2.0 if D doesn't have a compatible version?

**Answer depends on the strategy**:
1. **Single installation** — npm strategy: Try to find ONE version of D satisfying both constraints
2. **Multiple installations** — pnpm strategy: Install D@v1 + D@v2 separately (side-by-side versioning)
3. **Conflict detection** — Fail explicitly if no single version works

### Why Version Constraints Are Hard

A version constraint is an expression:

```
A requires D ∈ [2.0, 3.0)
B requires D ∈ [2.5, 4.0)
C requires D ∈ [1.0, 2.8)
```

Finding satisfying assignment:
- A: OK with 2.0, 2.1, ..., 2.99
- B: OK with 2.5, 2.6, ..., 3.99
- C: OK with 1.0, 1.1, ..., 2.79

**Overlap**: 2.5–2.79 satisfies all. **But** if C requires D ∈ [1.0, 2.4), there's no overlap → **conflict**.

With thousands of transitive dependencies, finding a valid assignment (or proving no solution exists) is **NP-complete**. Package managers use heuristics, not exhaustive search.

## Dependency Resolution Strategies

### 1. Greedy-First: npm's Approach

npm resolves in a single pass, depth-first, choosing the highest available version for each package.

**Algorithm**:
1. Start with root dependencies
2. For each dependency, resolve its transitive deps recursively (depth-first)
3. When encountering a package already seen, check if its required version overlaps with the current version
4. If yes, reuse it; if no, install a second copy (side-by-side at a deeper nesting level)

**Example**:
```
A (requires B@^1.0, C@^1.0)
├── B@1.5.0 (requires D@^2.0)
│   └── D@2.1.0
└── C@1.2.0 (requires D@^2.0)
    └── (reuses D@2.1.0 via hoisting)
```

**Advantages**:
- Fast (single pass)
- Minimal backtracking

**Disadvantages**:
- Non-deterministic (order of dependency traversal matters)
- No lookahead; may paint itself into a corner

### 2. Constraint Solver: Go Modules' MVS (Minimal Version Selection)

Go's module system uses **Minimal Version Selection**: select the **minimum** version that satisfies all constraints (rather than maximum). This is deterministic and reproducible.

**Algorithm**:
1. Build the full require graph
2. For each package, use its **minimum required version**
3. Prune versions that don't satisfy any constraint

**Example**:
```
A requires B >= 1.2, C >= 1.0
B@1.2 requires D >= 2.0
C@1.0 requires D >= 1.5

Result: A uses B@1.2 (minimum ≥ 1.2) and D@2.0 (minimum ≥ 2.0, due to B)
```

**Advantages**:
- Reproducible (always the same solution)
- Avoids unnecessary upgrades
- Simple to reason about

**Disadvantages**:
- May install outdated versions (psychological friction)
- Requires explicit version bumps to get security fixes

### 3. Backtracking SAT Solver: Poetry, pip 20.3+

Modern package managers use **Boolean Satisfiability (SAT) solvers** or **Constraint Propagation** to find a valid assignment.

**Algorithm**:
1. Encode version constraints as logical clauses
2. Use a SAT solver to find a satisfying assignment
3. If no solution exists, backtrack and try alternative versions
4. Prefer highest versions (optimization heuristic)

**Example**:
```
A requires B ∈ [1.0, 2.0), C ∈ [1.0, 2.0)
B@1.5 requires D ∈ [2.0, 3.0)
C@1.1 requires D ∈ [1.5, 2.5) — conflict with B's need!

Resolution backtracks: tries C@1.0 (if it requires D ∈ [1.5, 2.0])
Finds B@1.5 + C@1.0 + D@1.8 satisfies all constraints
```

**Advantages**:
- Exhaustive search (finds solution if it exists)
- Clear conflict reporting (explains why certain versions don't work)
- Globally optimal solution

**Disadvantages**:
- Slower for large dependency trees (NP-hard)
- Can take seconds—or timeout—for pathological graphs

## Semantic Versioning and Version Constraints

### Semantic Versioning (semver)

Versions follow `MAJOR.MINOR.PATCH`:
- **MAJOR** — Breaking changes
- **MINOR** — New features, backward-compatible
- **PATCH** — Bug fixes, backward-compatible

### Caret vs. Tilde (npm/Node.js convention)

- `^X.Y.Z` — "Don't bump major, lock when major=0" 
  - `^1.2.3` matches `[1.2.3, 2.0.0)` — patches and minors OK
  - `^0.2.3` matches `[0.2.3, 0.3.0)` — patches only (0.x is pre-release)
- `~X.Y.Z` — "Don't bump minor"
  - `~1.2.3` matches `[1.2.3, 1.3.0)` — patches only

**Philosophy difference**:
- **npm (caret default)** — Optimistic: Trust authors to respect semver; auto-upgrade minors
- **Go, Rust (exact versions default)** — Conservative: Pin exact versions until explicitly bumped

## Common Conflict Patterns

### Pattern 1: Transitive Dependency Hell

```
MyApp
├── lib-auth@2.0 (uses crypto-utils@3.0)
└── lib-api@1.0 (uses crypto-utils@2.0)

Result: npm installs both crypto-utils@2.0 and @3.0
Risk: Logic discrepancies if they're not backward-compatible
```

**Solution**: Authors should relax constraints (`crypto-utils@^2.0` instead of `@2.0`) or coordinate major versions.

### Pattern 2: Pre-release Versions

```
A requires B@^1.0.0
B@2.0.0-beta.1 is released but won't be selected (pre-release, not in range 1.x)
B@2.0.0 is released, now selected automatically

Problem: Developers may not realize a major version bump changed behavior
```

**Mitigation**: Use `npm audit` to review changes; lock-file ensures reproducibility.

### Pattern 3: Version Pinning Anti-Pattern

```json
// ❌ Too strict
{
  "dependencies": {
    "lodash": "4.17.21",
    "express": "4.18.2"
  }
}
```

This prevents **any** updates, including security patches. Paradoxically, libraries should use caret (`^`); applications can be stricter.

## Lockfiles and Reproducibility

### The Role of Lockfiles

A lockfile is the **ground truth** for a resolved dependency tree. It must be committed to version control.

```json
// package-lock.json (npm v7+)
{
  "packages": {
    "": {
      "dependencies": {"express": "^4.18.2"}
    },
    "node_modules/express": {
      "version": "4.18.2",  // ← Exact version
      "resolved": "https://registry.npmjs.org/express/-/...",
      "integrity": "sha512-..."  // ← Hash for verification
    }
  }
}
```

Why it matters:
1. **CI consistency** — `npm clean-install` uses lockfile, not `package.json`
2. **Author intent** — Captures the exact versions that worked when locked
3. **Security** — Prevents accidental upgrades to malicious versions

### When Lockfiles Diverge

```bash
npm install lodash@^4.0.0  # Updates package.json
npm install                 # Updates package-lock.json

git diff package*.json      # Shows divergence
npm ci                      # Uses lockfile, ignores package.json versions
```

**Best practice**: Use `npm ci` (clean install) in CI/CD; use `npm install` locally for development.

## Package Manager Comparison: Resolution Strategies

| Manager | Strategy        | Deterministic | Speed | Conflicts |
|---------|-----------------|---------------|-------|-----------|
| npm     | Greedy depth-first | ❌ Sorting-dependent | Fast | Hoists or nests duplicates |
| pnpm    | Greedy depth-first | ❌ Same as npm | Very Fast | Multiple versions OK |
| Yarn    | Constraint prop. | ✅ Yes | Slow | Detailed error messages |
| Poetry  | Backtracking SAT | ✅ Yes | Slow | Clear conflict explanation |
| Go      | Minimal version sel. | ✅ Yes | Fast | Explicit version bumps required |

## Resolving Conflicts: Practical Strategies

### 1. Relax Constraints

If `lib-a@2.0` and `lib-b@1.0` conflict on `util@3.0` vs `@2.0`:

```json
// Before
"lib-a": "2.0",
"lib-b": "1.0" 

// After (if lib-b allows it)
"lib-b": "1.1"  // Uses util@3.0
```

### 2. Peer Dependencies

If you're a library author, declare peer dependencies to let **consumers** resolve:

```json
{
  "peerDependencies": {
    "react": "^18.0.0"
  }
}
```

This shifts responsibility upward (better than hiding it).

### 3. npm Force / Legacy Flags

```bash
npm install --legacy-peer-deps  # Use npm v6 behavior; ignore unmet peer deps
npm install --force             # Install even if conflicts exist
```

**Warning**: These are safety-off switches; use only as last resort.

### 4. Constraint Relaxation in package.json

```json
// ❌ Exact version (inflexible)
"lodash": "4.17.21"

// ✅ Relaxed range (allows patches)
"lodash": "^4.17.0"
```

The more relaxed your constraints, the better dependency resolution works. But you're trusting authors to maintain compatibility.

## Version Conflict: The Hard Case

When no solution exists:

```
A requires B ∈ [1.0, 2.0)
B requires C ∈ [2.0, 3.0)
C requires A ∈ [3.0, 4.0)  ← Circular with incompatible version
```

**Resolution options**:
1. **Backtrack** (SAT solvers) — Try alternative versions of A, B, C
2. **Fail loudly** (npm) — Report conflict; let developer decide
3. **Install both** (pnpm can do this for B+C) — Less helpful for circular deps

## Key Takeaways

1. **Dependency resolution is NP-complete** — Package managers use heuristics; no perfect solution
2. **Greedy algorithms (npm)** are fast but non-deterministic; lockfiles enforce reproducibility
3. **Constraint solvers (Poetry, Yarn)** are slower but deterministic and report conflicts clearly
4. **Minimal version selection (Go)** avoids unnecessary upgrades but requires explicit bumps
5. **Phantom dependencies** (npm/Yarn hoisting) masquerade as resolved; switch to pnpm to eliminate
6. **Version constraint relaxation** enables resolution; excessive pinning creates hell
7. **Lockfiles are mandatory** for CI/CD; always commit them

## See Also

- [packages-npm-ecosystem.md](packages-npm-ecosystem.md) — npm's hoisting and phantom dependencies
- [packages-python-ecosystem.md](packages-python-ecosystem.md) — Poetry's SAT-based resolution
- [language-go-modules.md](language-go-modules.md) — Go's minimal version selection strategy