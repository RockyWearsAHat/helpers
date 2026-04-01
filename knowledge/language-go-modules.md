# Go Modules: Package Management, Versioning & Dependency Resolution

## Introduction

Go modules (introduced in Go 1.11, made default in 1.13) are the standard dependency management system. A module is a collection of packages versioned together, identified by a module path (import path). The `go.mod` file defines a module's identity and dependencies; `go.sum` records secure hashes.

Modules use **semantic versioning** and **minimum version selection (MVS)** to determine which versions of dependencies to use. Unlike dependency management systems that pick "latest compatible," Go is intentionally conservative: it uses the minimum version of each dependency that works.

## go.mod and go.sum

`go.mod` is the module manifest:

```
module github.com/user/project

go 1.21

require (
    github.com/some/package v1.2.3
    github.com/other/lib v0.5.0
)

exclude (
    github.com/problematic/dep <= v2.1.0
)

replace (
    github.com/fork/lib => ./local-fork
    old.import.path => github.com/new/path v1.0.0
)

retract (
    v1.0.0
    v1.1.0
)
```

**Fields:**
- `module` — The module's own import path. Used to resolve relative imports within the module.
- `go` — The minimum Go version this module expects. Go versions prior to 1.21 interpret this as a hard requirement; 1.21+ use it only for breaking change detection.
- `require` — Direct dependencies with versions (as constraints). Versions use semantic versioning: `v1.2.3` (MAJOR.MINOR.PATCH).
- `exclude` — Versions that should never be selected (blacklist). Rarely used; often indicates an upstream bug.
- `replace` — Local development aliases or fork redirection. Used to test local packages or redirect imports. Can point to a local path or another remote module at a specific version.
- `retract` — Versions that were released but should be avoided (e.g., security fixes). Go will warn users of retracted versions.

`go.sum` contains cryptographic hashes of each dependency's version to detect tampering or corruption:

```
github.com/some/package v1.2.3 h1:abc123...
github.com/some/package v1.2.3/go.mod h1:def456...
```

Both go.mod and go.sum should be committed to version control. `go.sum` ensures reproducible builds across machines.

## Semantic Import Versioning

Go uses semantic versioning with an additional rule: **import paths include the major version**:

```go
import "github.com/user/lib/v2"  // Importing major version 2
```

When a package reaches a breaking change, the maintainer increments the major version and changes the import path. Existing code importing `github.com/user/lib` (implicitly v1) continues to work; new code can opt into `v2` by importing the new path.

**Why this matters:** Go's module system allows multiple major versions of the same package in a single build. `v1` and `v2` are treated as entirely separate modules:

```go
import (
    v1 "github.com/user/lib"      // v1.x.x
    v2 "github.com/user/lib/v2"   // v2.x.x
)

lib1 := v1.New()
lib2 := v2.New()  // Different types, all APIs separate
```

Pre-release and local versions use suffixes: `v1.0.0-alpha`, `v1.0.0+build123`. These are useful for testing but are not preferred for releases.

## Minimum Version Selection (MVS)

Go's dependency resolution uses **MVS**: it finds the minimum version of each package that satisfies all constraints in the dependency graph.

Example: If projects A and B both depend on library L, and A requires `L >= v1.0.0` and B requires `L >= v1.1.0`, MVS selects `v1.1.0` (the minimum that satisfies both). This is conservative—it minimizes unnecessary upgrades and reduces the surface area of changes.

**Why MVS?**
- **Predictable:** You get the oldest compatible version, not the "latest."
- **Upgrade safety:** Upgrdes only happen when necessary (when you explicitly `go get -u` or when requirements change).
- **Reproducibility:** `go.mod` fully specifies the dependency tree; `go mod download` re-fetches verified hashes.

In contrast, many npm/pip workflows upgrade to the latest compatible version, which can introduce unexpected behavioral changes.

## go get and Upgrades

`go get` is the command for adding, upgrading, or removing dependencies:

```bash
go get github.com/user/lib              # Add/upgrade to latest (respecting go.mod constraints)
go get github.com/user/lib@v1.5.0       # Pin to specific version
go get github.com/user/lib@latest       # Upgrade to absolute latest
go get github.com/user/lib@none         # Remove the dependency
go get ./...                             # Upgrade all modules in the current module
go get -u                                # Upgrade all to newer minor/patch versions
go get -u=patch                          # Upgrade only patch versions
```

## replace and retract Directives

`replace` is powerful but can be misused:

```
replace github.com/orig/lib => github.com/fork/lib v1.2.0
replace github.com/bug/dep => ./local-bug-fix
```

Common uses:
- **Local development:** Point to a local directory during development.
- **Forks:** Use a maintained fork of an abandoned package.
- **Build-time flexibility:** CI systems sometimes use replace to inject versioned dependencies.

**Warning:** replace is not transitive. If you replace a dependency, your dependents cannot inherit that replacement unless they also declare it. This is intentional—it forces replaces to be explicit.

`retract` signals that certain versions should be avoided:

```
retract (
    v1.0.0          // Withdrawn entirely
    v1.1.0-alpha    // Pre-release, testing only
    [v1.2.0, v1.3.0]   // Range of versions
)
```

Retractions appear in the module's go.mod on the version server and will cause warnings in `go get` for users of retracted versions.

## Workspace Mode

Go 1.18+ supports **workspaces** for multi-module development:

```
// go.work
go 1.21

use (
    ./api
    ./lib
    ./cli
)
```

In workspace mode, Go treats multiple modules as a unit. Changes in one module are immediately visible to others. Workspaces are useful for monorepos or during large refactorings spanning multiple modules.

## Private Modules and GOPROXY

Go automatically fetches public modules from `proxy.golang.org`. For private modules, configure:

**Via environment (recommended):**
```bash
export GOPROXY="https://yourproxy.com,direct"
export GOPRIVATE="github.com/company/*"
export GOINSECURE="github.com/dev/*"  # Skip certificate verification for dev
```

**Via git config:**
```bash
git config --global url."git@github.com:company/".insteadOf "https://github.com/company/"
```

`GOPRIVATE` tells Go which import paths are private (don't proxy them). `GOPROXY` lists proxies to try in order; `direct` means fetch directly from version control.

## Vanity Import Paths

Vanity import paths use domain-based names independently of hosting:

```go
import "example.com/mylib"
```

This works by serving a special HTML response from `example.com`:

```html
<meta name="go-import" content="example.com/mylib git https://github.com/user/mylib">
```

Go fetches this metadata, discovers the real repository, and clones accordingly. Vanity paths allow portability—if you move the repository, only the metadata changes.

## Module Caching and go.sum

Go caches downloaded modules in `$GOPATH/pkg/mod` (usually `~/go/pkg/mod`). The cache is managed automatically:

```bash
go clean -modcache  # Remove all cached modules
```

`go.sum` records the cryptographic hash of each module version's `.mod` file and source content. When you run code, Go verifies against `go.sum`. If a module has been tampered with, Go rejects it. This provides defense against supply-chain attacks.

## Vendoring

While Go prefers explicit version management in go.mod, vendoring (copying dependencies into the repo) is still supported:

```bash
go mod vendor  # Create vendor/ directory with all dependencies
go mod vendor -o ./alt-vendor  # Vendor to custom directory
go build -mod=vendor  # Build using vendored copies
```

Vendoring was more relevant before reliable module proxies. Today it's used when:
- Offline builds are required
- Supply chain security demands local verification
- Corporate policies mandate dependency bundling

## Guidelines

- Keep go.mod minimal: list only direct dependencies, not transitive ones.
- Commit both go.mod and go.sum to version control.
- Use `go mod tidy` regularly to remove unused dependencies and deduplicate.
- For packages you maintain, support multiple major versions cleanly via import path versioning.
- Understand MVS—Go will use the minimum compatible version, not the latest.
- Use `replace` sparingly and document why it's needed.
- For monorepos, consider workspace mode (Go 1.18+).

See also: [language-go.md](language-go.md), [api-design.md](api-design.md)