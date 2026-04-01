# Package Managers — Dependency Resolution, Lockfiles & Supply Chain

## Core Problem: The Dependency Graph

Modern applications depend on libraries. Libraries depend on other libraries. Naively including all dependencies multiplies package size and creates complex version conflicts:

```
App v1.0
├── LibA v2.0
│   ├── LibX v1.5
│   └── LibY v1.0
├── LibB v3.0
│   └── LibX v1.4  ← Version conflict! Both v1.5 and v1.4 needed?
└── LibC v1.0
    └── LibY v2.0  ← Another conflict! Both v1.0 and v2.0?
```

Package managers solve this by:

1. **Resolving versions**: Find a set of versions satisfying all constraints
2. **Deduplicating**: Reusing common dependencies where possible
3. **Recording lockfiles**: Freezing exact versions for reproducibility

## npm, pnpm, Yarn: JavaScript Ecosystems

### npm (Node Package Manager)

Default for Node.js. Central registry (`npmjs.com`). Dependency resolution: **depth-first**.

**package.json specifies ranges:**

```json
{
  "dependencies": {
    "lodash": "^4.17.0",
    "express": "~4.18.0"
  }
}
```

- `^4.17.0`: Major version locked (4.*), minor/patch flexible (4.17.0–4.99.99)
- `~4.18.0`: Minor version locked (4.18.*), patch flexible

**npm install** resolves and installs to `node_modules/`, creating an **npm-shrinkwrap.json** (or **package-lock.json**) lockfile.

### Phantom Dependency Problem

npm's hoisting (moving nested deps to top level) creates phantom dependencies:

```
node_modules/
├── lodash/
├── express/
│   └── node_modules/
│       └── body-parser/
```

Developer code can directly `require('body-parser')` even though package.json doesn't list it. This works because npm hoists `body-parser` to the top level if no conflicts. **Gotcha:** Remove express, install something else, and suddenly body-parser is missing. This breaks.

### pnpm (Performance npm)

Solves npm's issues using a content-addressable store:

1. **No hoisting**: Each package gets its exact deps in `.pnpm/` sub-directories
2. **Hardlinks**: Actual installed packages hardlink to central store (saves disk)
3. **Phantom deps prevented**: You cannot access undeclared dependencies

```
node_modules/
└── .pnpm/
    ├── lodash@4.17.21/
    ├── express@4.18.2/
    │   └── node_modules/ (only its actual deps, not hoisted)
        └── body-parser@1.20.0/
```

Disk savings: 50-70% less storage than npm. Trade-off: Less familiar to developers trained on npm's hoisting.

### Yarn (Facebook)

Classic alternative to npm. Introduced concept of **workspaces** (monorepo support).

**pnpm vs Yarn:**
- Yarn: Mature, community-driven, workspaces widely adopted
- pnpm: Stricter discipline (no phantom deps), faster, growing adoption

Most new projects choose pnpm or npm v7+.

### Workspaces: Monorepo Management

```json
{
  "workspaces": ["packages/*"]
}
```

Yarn and npm both support workspaces:

```
repo/
├── packages/
│   ├── api/
│   │   └── package.json
│   └── web/
│       └── package.json
├── package.json
└── yarn.lock (shared lockfile for all workspaces)
```

One `yarn install` installs deps for all packages. Local packages can reference each other by name:

```json
{
  "dependencies": {
    "@myorg/api": "workspace:*"
  }
}
```

## Poetry & uv: Python Packaging Evolution

### Poetry (Standardized Dependency Management)

Python's historical pain: `pip` + `requirements.txt` + `virtualenv` is fragmented. Poetry unifies this.

```bash
poetry new my-project
# Creates
# my_project/
# ├── pyproject.toml
# ├── poetry.lock
# └── __init__.py

poetry add requests    # Add dependency, update lock
poetry install         # Install from lock
poetry publish         # Build + upload to PyPI
```

**pyproject.toml** (PEP 517/518 standard):

```toml
[tool.poetry]
name = "my-app"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.9"
requests = "^2.28.0"

[build-system]
requires = ["poetry-core"]
build-backend = "poetry.core.masonry.api"
```

**poetry.lock** records exact versions, hashes, and dependency tree for reproducibility.

### uv: Rust-Based Speed

Recent entrant: `uv` is written in Rust and achieves 10-100x faster operations than pip + poetry.

```bash
uv pip install requests     # Instant vs slow pip
uv pip compile requirements.in -o requirements.txt
uv sync                    # Create venv + install from lock
```

**uv philosophy:**
- Drop-in replacement for `pip`
- Compatible with standard `pyproject.toml` and lockfiles
- No external daemon; pure CLI

Trade-off: Newer; ecosystem not as mature as Poetry.

### Virtual Environments

Both enforce isolation via virtualenvs:

```bash
poetry env use /usr/bin/python3.11  # Select Python version
poetry run python script.py          # Run in venv
poetry shell                         # Activate venv (bash subshell)
```

## Cargo: Rust's Integrated Approach

Rust's package manager is tightly coupled with the language and toolchain.

```bash
cargo new my-app           # Init project
cargo add tokio
cargo build                # Compile
cargo test
cargo publish              # Upload to crates.io
```

**Cargo.toml** includes dependencies with features and versions:

```toml
[package]
name = "my-app"
version = "0.1.0"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
criterion = "0.5"
```

### Features & Conditional Compilation

Rust packages can be compiled with different feature flags:

```bash
cargo build --no-default-features --features async
```

This enables:
- Lightweight core + opt-in functionality
- No bloated default builds
- Compile-time configuration

### Cargo.lock

`Cargo.lock` is committed to version control (opposite of npm/pip practice):

- **Libraries** (published to crates.io): Don't commit Cargo.lock; users get their own
- **Binaries** (distributed executables): Commit Cargo.lock; ensures reproducible builds

## Go Modules: Minimal Versioning Semantics

Go's `go.mod` and `go.sum`:

```bash
go get -u github.com/sirupsen/logrus@v1.9.0
go build ./...   # Builds using versions in go.mod
```

**go.mod:**

```
module example.com/my-app

go 1.21

require (
    github.com/sirupsen/logrus v1.9.0
    github.com/goreleaser/goreleaser v1.0.0
)
```

**Minimal Version Selection (MVS)** algorithm:

Unlike npm (maximal matching), Go uses MVS: "Given all constraints, use the minimum version that satisfies them."

Rationale: Backward compatibility. If `@v1.5` has a bug fixed in `@v1.8`, but your code works with `@v1.5`, Go uses `@v1.5` because later versions might introduce breaking changes (even with semantic versioning).

## Dependency Resolution Algorithms

### NPM-Style: Depth-First, Maximal

Install the highest compatible version of each dependency.

```
resolve(lodash ^4.0)
  = 4.17.21 (highest 4.x)
```

**Pros:** Upgrades automatically included; fewer major versions active.  
**Cons:** Can cascade breaking changes; binary search for compatibility harder.

### Go-Style: Minimal Version Selection

Use the lowest version satisfying constraints.

```
resolve(logrus v1.0-v1.9)
  = v1.4 (minimal satisfying both requirements)
```

**Pros:** Conservative; limits surprise upgrades.  
**Cons:** May miss bug fixes in later minimal versions.

### SAT Solver Approach (Advanced)

Constraint satisfaction. Framed as: **Find a valid assignment of versions satisfying all constraints.**

```
constraints:
  App requires A ∈ [1.0, 2.0]
  A requires B ∈ [1.0, 1.5]
  B requires A ∈ [1.5, 1.9]

Result: Could be infeasible, or A=1.5, B=1.5 (example simplification)
```

Poetry and pip use SAT solvers (more sophisticated than npm's greedy approach) to detect conflicts earlier.

## Lockfiles: Reproducibility & Security

### Purpose

Lock files record the exact resolved dependency graph so deployments are reproducible:

```json
{
  "packages": {
    "lodash": "4.17.21",
    "express": "4.18.2"
  },
  "integrity": "sha512-..."
}
```

**Workflow:**

1. Developer updates `package.json`
2. `npm install` resolves, generates `package-lock.json`
3. CI/production: `npm ci` (clean install) uses lockfile exactly, no resolution
4. Semantic versioning won't upgrade you unexpectedly

### Hash Verification

Lockfiles include hashes (SHA-512, etc.) for each package:

```
lodash@4.17.21: sha512-1234abcd...
```

During install, npm verifies the downloaded package matches the hash. Detects:
- Package tampering
- Man-in-the-middle attacks
- Registry compromises

## Security & Supply Chain

### Dependency Scanning

Tools audit the dependency tree for known vulnerabilities:

```bash
npm audit
# + npm ERR! 5 vulnerabilities found
#   Package: lodash < 4.17.21 (prototype pollution)
#   npm audit fix          # Auto-upgrade to 4.17.21
```

Popular tools:
- npm's built-in `npm audit`
- Snyk (comprehensive, SCA: Software Composition Analysis)
- GitHub Dependabot (GitHub-integrated, auto-PRs)
- OWASP Dependency-Check

### Minimizing Attack Surface

**Practice:**
- Audit regularly (`npm audit`)
- Lock production versions (`npm ci`, not `npm install`)
- Review dependency list quarterly; remove unused packages
- Use minimal scopes: `devDependencies` for build-time tools, not runtime

**The `typosquatting` attack:** Register package with name similar to popular package (`lodash` → `lodassh`). Users mistype and install malicious package. Mitigation: Double-check package names before first install.

## Version Constraints: Semantic Versioning

Most registries follow semantic versioning (semver):

```
MAJOR.MINOR.PATCH
 v2.  5.     3

MAJOR: Breaking changes
MINOR: New feature, backward-compatible
PATCH: Bug fixes
```

Constraint syntax:

| Syntax      | Meaning                      |
| ----------- | ---------------------------- |
| `2.5.3`     | Exact                        |
| `^2.5.3`    | Major locked (2.*)           |
| `~2.5.3`    | Major.minor locked (2.5.*)   |
| `>=2.5.3`   | At least this version        |
| `2.5.x`     | Any patch in 2.5             |
| `*`         | Any version (risky)          |

**Pre-release:** `2.0.0-beta.1` (lower precedence than `2.0.0`)