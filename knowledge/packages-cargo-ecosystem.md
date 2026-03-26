# Cargo: Rust's Package Manager and Build System

## Overview

Cargo is considered the **gold standard** for package managers. It combines dependency resolution, version management, build orchestration, and publishing into a unified, well-designed tool. Unlike npm's fragmented tooling or Python's competing standards, Rust developers have one clear choice. Cargo's success stems from three core strengths: sensible defaults, explicit workspace support, and integration with Rust's type system for compile-time safety.

## Cargo.toml: Package Manifest

### Basic Structure

```toml
[package]
name = "my-library"
version = "0.1.0"
edition = "2021"
description = "A useful library"
authors = ["Alice <alice@example.com>"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/alice/my-library"
homepage = "https://example.com"
documentation = "https://docs.rs/my-library"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }

[dev-dependencies]
criterion = "0.5"

[build-dependencies]
cc = "1.0"

[[bin]]
name = "my-app"
path = "src/bin/main.rs"

[[example]]
name = "basic_example"
path = "examples/basic.rs"
```

### Edition System

Rust uses **editions** to publish breaking changes in a backwards-compatible way. All Cargo projects declare an edition.

| Edition | Year | Major Changes |
|---------|------|---------------|
| 2015    | 2015 | Original Rust |
| 2018    | 2018 | Module system overhaul, `async`/`await` |
| 2021    | 2021 | Const generics, `panic` changes |
| 2024    | 2024 | Yet to be released |

**Key property**: Different editions can coexist in the same workspace. A library can be 2021 while dependencies are 2018; Cargo handles translation at compile time.

**Example**:
```toml
edition = "2021"  # Enables latest syntax
```

## Dependency Declaration and Features

### Dependency Types

```toml
[dependencies]              # Runtime dependencies
serde = "1.0"

[dev-dependencies]          # Test-only; not included in published crate
criterion = "0.5"           # For benchmarks

[build-dependencies]        # For build.rs; not included in runtime
cc = "1.0"

[target.'cfg(windows)'.dependencies]  # Platform-specific
winapi = "0.3"

[target.'cfg(test)'.dependencies]     # Test-specific (alternative to dev-dep)
```

### Version Specifications

```toml
serde = "1.0.0"           # Exact version
serde = "1.0"             # Same as above
serde = "^1.0.0"          # Caret: bump minor/patch (>= 1.0, < 2.0)
serde = "~1.0.0"          # Tilde: bump patch only (>= 1.0.0, < 1.1)
serde = "1.*"             # Any 1.x
serde = ">= 1.0"          # Comparison operators
serde = ">= 1.0, < 2.0"   # Range
serde = "0.1.*"           # Pre-1.0 (strictly patch only, by semver philosophy)
```

Default: Caret (`^`). Versions starting with 0 are pre-release; `^0.1.0` means `>= 0.1.0, < 0.2.0` (not `< 1.0`).

### Features: Conditional Compilation

Features are **compile-time flags** enabling/disabling code paths and optional dependencies.

```toml
[package]
name = "my-lib"

[dependencies]
tokio = { version = "1.0", optional = true }

[features]
full = ["tokio"]
tokio-runtime = ["tokio"]
default = []                # No features by default
```

**Usage**:
```rust
#[cfg(feature = "tokio-runtime")]
async fn async_main() { }

#[cfg(not(feature = "tokio-runtime"))]
fn sync_main() { }
```

**Activation**:
```bash
cargo build --features tokio-runtime
cargo build --all-features
cargo build --no-default-features --features full
```

**In dependencies**:
```toml
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }
```

### Dependency Version Resolution

Cargo resolves versions using a **backtracking solver** similar to modern Python (Poetry). When conflicts arise, it tries alternative versions.

```
my-app
├── lib-auth@2.0 (requires crypto@^1.5)
└── lib-api@1.0 (requires crypto@^1.0)

Result: crypto@1.x (highest compatible)
```

If `lib-api` required crypto@2.0 (incompatible with lib-auth), Cargo would either:
1. Find a compatible version of lib-api/lib-auth, or
2. Fail with a clear error message

## Cargo.lock: Lockfile for Reproducibility

Similar to `package-lock.json` or `poetry.lock`, but with a key difference:

- **Libraries** (`[lib]` in Cargo.toml) **do NOT commit Cargo.lock** (their dependencies shouldn't force exact versions downstream)
- **Binaries** (applications) **DO commit Cargo.lock** (ensures reproducible builds)

### Structure

```toml
[[package]]
name = "my-app"
version = "0.1.0"
dependencies = [
 "serde 1.0.197",
 "tokio 1.35.0",
]

[[package]]
name = "serde"
version = "1.0.197"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3fb1c873e1b9b056a7a302d853ab92e5ca9434239d083d03d5d6db3a7907246e"

[[package]]
name = "tokio"
version = "1.35.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "fbc924face52014e1d08bcc83b3c37da47efd979d2e46ab5ef2f61a0e86e364d"
dependencies = [
 "pin-project-lite",
 "tracing",
]
```

**Key insight**: The `checksum` field ensures supply-chain security (hash verification).

## Build Scripts: build.rs

Cargo supports custom build steps via `build.rs`:

```rust
// build.rs (runs before compilation)
fn main() {
    // Generate code, compile C bindings, etc.
    println!("cargo:rustc-env=BUILD_TIME={}", chrono::Local::now());
    println!("cargo:rerun-if-changed=build.rs");
}
```

### Common Use Cases

1. **C/C++ Bindings**:
```rust
fn main() {
    cc::Build::new()
        .file("src/wrapper.c")
        .compile("wrapper");
    println!("cargo:rustc-link-lib=wrapper");
}
```

2. **Code Generation**:
```rust
use std::env;
use std::fs;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    fs::write(
        format!("{}/generated.rs", out_dir),
        "pub const VERSION: &str = \"0.1.0\";",
    ).unwrap();
}
```

3. **Conditional Compilation**:
```rust
fn main() {
    #[cfg(target_os = "windows")]
    println!("cargo:rustc-link-lib=user32");
}
```

### Conditional Compilation Attributes

```rust
#[cfg(target_os = "windows")]
fn platform_specific() { }

#[cfg(target_pointer_width = "64")]
fn is_64bit() { }

#[cfg(debug_assertions)]      // Debug builds only
#[cfg(not(debug_assertions))]  // Release builds only

#[cfg(feature = "tokio-runtime")]  // Feature-gated
fn async_main() { }

#[cfg(all(feature = "tokio", target_os = "linux"))]  // Multiple conditions
fn special_case() { }
```

## Workspaces

Monorepo support via workspaces:

```toml
# Root Cargo.toml
[workspace]
members = [
    "crates/core",
    "crates/cli",
    "crates/server",
]

[workspace.package]
version = "0.1.0"
authors = ["Alice <alice@example.com>"]
```

**Each member**:
```toml
# crates/core/Cargo.toml
[package]
name = "my-core"
version.workspace = true  # Inherit from workspace
authors.workspace = true
```

**Workspace benefits**:
- Single `Cargo.lock` for all members (shared dependencies)
- Unified versions and authors
- `cargo build -p my-core` — Build specific crate
- `cargo test --workspace` — Test all crates
- Cross-crate dependency resolution

## Publishing to crates.io

### Metadata Requirements

```toml
[package]
name = "my-lib"                        # Must be unique
version = "0.1.0"                      # Semver
description = "Brief description"      # Required
license = "MIT OR Apache-2.0"          # Must be SPDX expression
readme = "README.md"                   # Recommended
repository = "https://github.com/..."  # Recommended
documentation = "https://docs.rs/..."  # Auto-generated
```

### Publishing Workflow

```bash
# 1. Create account on crates.io
# 2. Generate API token
# 3. Configure credentials
cargo login YOUR_TOKEN

# 4. Publish
cargo publish

# 5. Verify
cargo search my-lib

# 6. Yank (remove) version if needed (doesn't delete but warns)
cargo yank --vers 0.1.0
```

### Yanking

Yanked versions are kept in history but marked as unavailable for new installs. Use if a critical bug is discovered post-release.

## crates.io Ecosystem

### Discovery and Reputation

- **Download counts** — Indicates popularity
- **Docs.rs integration** — Automatically generates docs from README + rustdoc
- **Maintenance status** — Explicitly marked if actively developed
- **Yanked versions** — Visible in version history

### Common Crates (de facto standards)

| Category       | Crate             | Notes |
|----------------|-------------------|-------|
| Async runtime  | `tokio`           | Industry standard |
| Serialization  | `serde`           | Near-mandatory |
| Web framework  | `axum`, `actix`   | Competing frameworks |
| Testing        | `criterion`       | Benchmarking |
| Logging        | `tracing`, `log`  | Structured logging |
| Error handling | `anyhow`, `thiserror` | Error wrapping |

## cargo install vs. cargo build

### cargo build

Compiles a crate into a library or binary in `target/` for local use.

```bash
cargo build --release  # Optimized build
cargo build --target wasm32-unknown-unknown  # Cross-compile to WASM
```

### cargo install

Installs a **binary crate** globally (into `~/.cargo/bin/`).

```bash
cargo install ripgrep
cargo install --path . --force  # Install from local crate
ripgrep --version  # Now available in PATH
```

Only binary crates (with `[[bin]]` targets) can be installed.

## Why Cargo is Considered the Gold Standard

1. **Single tool, full integration** — No fragmentation (unlike npm/Python)
2. **Excellent defaults** — Works well with zero configuration
3. **Type-system integration** — Features, conditional compilation are first-class
4. **Deterministic resolution** — Backtracking solver (like Poetry) ensures reproducibility
5. **Edition system** — Elegantly handles breaking changes
6. **Workspace support** — Built-in, not bolted-on
7. **Clear error messages** — Conflicts reported with context and suggestions
8. **Security by default** — Hashes, signatures, and yanking mechanisms

## Key Takeaways

1. **Cargo.toml** is the single source of truth for package metadata
2. **Features** enable compile-time configuration without runtime overhead
3. **Dependency types** (dependencies, dev-dependencies, build-dependencies) are explicit
4. **Cargo.lock** is committed for binaries, NOT for libraries
5. **Editions** allow breaking changes across the ecosystem; a binary can use 2021 with 2018-edition dependencies
6. **build.rs** enables custom build steps (C bindings, code generation, conditional compilation)
7. **Workspaces** provide monorepo support with unified versioning
8. **Publishing** to crates.io requires metadata; yanking removes versions retroactively
9. **Conditional compilation** (`#[cfg]`) is type-safe and compiler-enforced
10. **Cargo's design** prioritizes correctness over flexibility

## See Also

- [language-rust.md](language-rust.md) — Rust idioms and best practices
- [tools-package-managers.md](tools-package-managers.md) — Broader comparison across languages
- [build-systems-concepts.md](build-systems-concepts.md) — Build orchestration theory