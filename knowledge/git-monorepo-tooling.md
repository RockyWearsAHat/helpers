# Git Monorepo Tooling — Patterns, Tools, and Tradeoffs

## Monorepo Defined

A **monorepo** is a single Git repository containing multiple projects, packages, or services. Contrasts with **polyrepo** (separate repositories per project).

**Example monorepo structure:**

```
repo/
  packages/
    auth-service/
    api-gateway/
    frontend/
  libs/
    shared-utils/
    database-client/
  apps/
    web/
    mobile/
  infra/
    terraform/
    docker/
```

All projects share `.git`, CI/CD pipeline, and dependency management. Teams must coordinate shared concerns (versions, releases, deployments).

## Monorepo vs Polyrepo Tradeoffs

### Monorepo Advantages

- **Atomic transactions:** A single commit can update multiple projects (changes to shared libs + service that uses them in one transaction)
- **Refactoring across projects:** Rename files, reorganize modules, update APIs without cross-repo coordination
- **Code reuse:** Shared libraries are easy to extract, reference, and maintain
- **Unified CI/CD:** One pipeline for all projects (consistency)
- **Dependency management:** Single package manager configuration, uniform tooling
- **Ease of onboarding:** Clone once, have the entire codebase

### Monorepo Disadvantages

- **Scaling challenges:** Large monorepos become slow (clone, checkout, operations) without tooling
- **Security isolation:** Hard to grant selective access (all or nothing)
- **Release complexity:** Coordinating releases across many projects requires discipline
- **Build coupling:** Changes to shared libs trigger full rebuilds of dependents (mitigation: incremental builds, caching)
- **Tool overhead:** Requires monorepo-aware tooling (Nx, Turborepo, Lerna) to manage scale

### Polyrepo Advantages

- **Ownership and isolation:** Each team owns their repo, controls releases
- **Performance:** Smaller repos are faster to clone and operate on
- **Security:** Fine-grained access control per repository
- **Independence:** Projects can upgrade dependencies, change tooling at own pace

### Polyrepo Disadvantages

- **Cross-repo changes:** Updating a shared library requires coordinating changes across many repos
- **Dependency hell:** Versioning and compatibility between projects becomes a management burden
- **Code duplication:** Shared code isn't easily extracted (copy-paste instead of import)
- **Fragmentation:** Many CI/CD pipelines, inconsistent tooling
- **Onboarding:** New developers must clone and understand many repos

## Monorepo Tools

### Nx (TypeScript/JavaScript, Language-Agnostic)

Nx is a build system and monorepo framework. Core features:

- **Computation caching:** Tasks cached by hash of inputs (source code, dependencies); expensive builds skipped if nothing changed
- **Affected commands:** `nx affected --target=test` runs tests only for packages changed in PR (not entire codebase)
- **Task orchestration:** Define dependencies between tasks; Nx runs in optimal order and parallelizes where possible
- **Plugin system:** Language-specific plugins (React, Angular, Node, Python, etc.)
- **Workspace visualization:** `nx graph` shows dependencies between packages

**Example:**

```json
{
  "projects": {
    "packages/api": {
      "targets": {
        "build": { "executor": "@nx/node:build" },
        "test": { "executor": "@nx/jest:jest", "dependsOn": ["build"] }
      }
    },
    "packages/ui": {
      "targets": {
        "build": { "executor": "@nx/react:build" },
        "test": { "dependsOn": ["build"] }
      }
    }
  }
}
```

**Workflow:**

```bash
nx build packages/api        # Build only api
nx affected --target=test    # Test only packages changed in PR
nx graph                      # Visualize dependency graph
```

**Strengths:** Powerful caching, language-agnostic, mature ecosystem.
**Weaknesses:** Learning curve, significant configuration overhead.

### Turborepo (TypeScript/JavaScript, Pure Task Runner)

Turborepo focuses on task orchestration with a simpler mental model than Nx.

**Key features:**

- **Task pipeline definition:** `turbo.json` defines task dependencies and outputs
- **Caching:** Caches task outputs by input hash
- **Remote caching:** Store cache artifacts remotely (Vercel, self-hosted) for CI/CD
- **Parallel execution:** Runs independent tasks in parallel

**Example turbo.json:**

```json
{
  "pipeline": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": ["dist/**", ".next/**"]
    },
    "test": {
      "dependsOn": ["build"],
      "outputs": ["coverage/**"]
    }
  }
}
```

**Workflow:**

```bash
turbo run build              # Build all packages
turbo run build --filter=@myapp/api    # Build only api
turbo run test --parallel    # Test in parallel (respecting deps)
```

**Strengths:** Simpler than Nx, smaller learning curve, faster for task management.
**Weaknesses:** Less powerful introspection, newer ecosystem.

### Lerna (Package Publishing Focus)

Lerna manages versioning and publishing of multiple npm packages.

**Workflow:**

```bash
lerna init                   # Initialize monorepo
lerna add lodash             # Add dependency to all packages
lerna publish                # Publish new versions
lerna changed                # List packages changed since last release
```

**Version management:**

- Fixed versioning: All packages have the same version
- Independent versioning: Each package versioned separately

**Publishing:**

Lerna handles publishing to npm, managing version bumps, changelogs, and tags.

**Limitations:** Primarily for npm publishing; doesn't handle task orchestration or caching. Modern alternatives (Nx, Turborepo) are more feature-complete.

### Rush (Enterprise-Grade)

Rush (from Microsoft) is designed for large, multi-team monorepos.

**Features:**

- **Workspace management:** Unified `rush.json` configuration for all projects
- **Dependency graph resolution:** Manages complex interdependencies
- **Lock files:** Shared `common/config/rush/pnpm-lock.yaml` ensures consistent dependency installation
- **Change logs:** Automatic changelog generation per package
- **Incremental builds:** Skip unchanged packages

**Workflow:**

```bash
rush install                 # Install all dependencies
rush build                   # Build all packages in order
rush rebuild --changed       # Rebuild only changed packages
```

**Strengths:** Enterprise features, mature, designed for scale.
**Weaknesses:** Steep learning curve, opinionated defaults, smaller ecosystem than Npm/Yarn.

## Monorepo Patterns

### Loose Coupling

Packages should depend on stable APIs, versioned appropriately:

```json
{
  "dependencies": {
    "@myapp/shared": "^1.0.0"
  }
}
```

Semantic versioning ensures compatibility. Internal packages can use different versions simultaneously.

### Shared Configuration

Central `shared/` or `config/` directory for common configs:

```
repo/
  shared/
    babel.config.js
    typescript.config.json
    eslint.config.js
  packages/
    api/
      babel.config reference (extends shared)
    web/
      babel.config reference (extends shared)
```

Reduces duplication; enable teams to opt-in to breaking changes incrementally.

### Dependency Graph Visualization

Tree or graph representation helps identify circular dependencies and coupling:

```bash
nx graph              # Interactive visualization
pnpm ls --depth=2    # List dependency tree
```

A healthy monorepo has clear layers:

```
apps/          (applications, topmost layer)
  ├─ frontend (depends on services + shared)
  └─ backend  (depends on services)
services/      (business logic and API)
  ├─ auth
  └─ payment   (depends on shared)
shared/        (utilities, no internal dependencies)
  └─ utils
```

Circular dependencies slow builds and confuse developers.

### Incremental Builds and Caching

Most modern tools support:

1. **Input-based caching:** Hash source files and configs; if unchanged, skip task
2. **Remote caching:** Cache artifacts in S3, Turborepo cloud, or self-hosted server
3. **Affected analysis:** Determine which packages changed, run tasks only for those

Example with Nx:

```bash
nx affected:build --base=main  # Build only packages changed since main
```

In CI:

```bash
nx run-many --target=test --parallel --maxWorkers=4
```

## Implementation Strategies

### Greenfield Monorepo

Start with a tooling choice:

- **JavaScript/TypeScript:** Turborepo (simpler) or Nx (more powerful)
- **Python:** Poetry or PEP 517 workspaces; no mature equivalents to above
- **Multi-language:** Self-orchestrated (Makefile, shell scripts) or cloud-native (Bazel, Buck)

Example structure (TypeScript with Turborepo):

```
turbo.json                  # Task definitions
package.json               # Root package, workspace config
packages/
  api/
    package.json
    src/
  web/
    package.json
    src/
pnpm-workspace.yaml        # Workspace definition
```

### Migrating from Polyrepo

1. **Export history:** Preserve commit history (optional but recommended):
   ```bash
   git subtree add --prefix=packages/api https://github.com/user/api.git main
   ```
2. **Consolidate packages:** Move packages into monorepo structure
3. **Unify configs:** Centralize ESLint, TypeScript, Babel configs
4. **Adopt tooling:** Set up Nx/Turborepo with task definitions
5. **Coordinate releases:** Establish versioning and publishing workflow

### Multi-Language Monorepos

For monorepos spanning Python, Go, JavaScript, etc.:

- **Bazel** or **Buck** (Facebook): Language-agnostic build orchestration
- **Makefile** + custom scripts: Manual but transparent
- **Meson** (C/C++ projects): Works for many languages
- **Cloud native:** Containers + Kubernetes for isolation

Most enterprises with multi-language monorepos build custom orchestration.

## Sparse-Checkout for Monorepo Scale

For very large monorepos, developers can use sparse-checkout to work on subsets:

```bash
git clone --filter=blob:none --sparse https://github.com/user/monorepo.git
cd monorepo
git sparse-checkout set packages/api packages/shared
```

Result: Only relevant packages are checked out; others skipped. Combined with partial clone, reduces bandwidth and disk usage significantly.

## Common Pitfalls

- **Circular dependencies:** Tools should detect and warn. Manual review during planning helps.
- **Forgotten affects analysis:** Running full test suite on every commit defeats monorepo benefits.
- **Unversioned changes:** Publishing without version bumps creates confusion. Use semantic versioning rigorously.
- **Inconsistent tooling:** Different packages using different build tools complicates orchestration.
- **Large lock files:** Many dependencies create "vendor lock" and merge conflicts. Use workspace lock files wisely.

---

_Sources: monorepo.tools (comparison), Nx documentation, Turborepo documentation, Rush (Microsoft), Lerna documentation, Bazel/Buck documentation_