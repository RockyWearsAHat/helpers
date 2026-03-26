# Monorepo Strategies

## Why Monorepo

A monorepo houses multiple projects, packages, or services in a single repository.

### Benefits

| Benefit                    | Description                                                 |
| -------------------------- | ----------------------------------------------------------- |
| **Atomic commits**         | Cross-package changes in one commit, one PR, one review     |
| **Code sharing**           | Shared libraries used directly, no publish/consume cycle    |
| **Unified CI**             | One CI pipeline, consistent tooling and standards           |
| **Easier refactoring**     | Rename a shared function, all consumers update together     |
| **Dependency consistency** | One version of each external dependency across all packages |
| **Discoverability**        | All code searchable in one place                            |

### Challenges

| Challenge                | Description                                                 |
| ------------------------ | ----------------------------------------------------------- |
| **Build times**          | More code = longer builds without smart tooling             |
| **Code ownership**       | Who owns what? Boundaries blur without discipline           |
| **Repository size**      | Git operations slow down with large histories               |
| **CI complexity**        | Running all tests for every change doesn't scale            |
| **Dependency hell**      | Upgrading a shared lib affects all consumers simultaneously |
| **Tooling requirements** | Need specialized build tools to manage efficiently          |

## Monorepo Tools

### Nx

Full-featured monorepo toolkit for any language/framework.

```
# Create workspace
npx create-nx-workspace@latest myorg --preset=ts

# Project structure
apps/
  web/              # Application
  api/              # Application
libs/
  shared-ui/        # Library
  data-access/      # Library
  utils/            # Library
```

Key features:

| Feature                 | What It Does                                               |
| ----------------------- | ---------------------------------------------------------- |
| **Project graph**       | Understands dependencies between packages                  |
| **Affected commands**   | Only build/test/lint what changed and its dependents       |
| **Computation caching** | Cache task results locally and remotely (Nx Cloud)         |
| **Generators**          | Scaffold new packages consistently                         |
| **Module boundaries**   | Enforce architectural rules (e.g., libs can't import apps) |

```bash
# Only test affected projects
nx affected --target=test --base=main

# Visualize dependency graph
nx graph

# Run with remote caching
nx run-many --target=build --all  # Cache hits skip execution

# Enforce module boundaries (in .eslintrc)
# "@nx/enforce-module-boundaries" rule
```

**Module boundary rules**:

```json
{
  "@nx/enforce-module-boundaries": [
    "error",
    {
      "depConstraints": [
        {
          "sourceTag": "scope:app",
          "onlyDependOnLibsWithTags": ["scope:shared", "scope:feature"]
        },
        {
          "sourceTag": "scope:feature",
          "onlyDependOnLibsWithTags": ["scope:shared"]
        },
        {
          "sourceTag": "scope:shared",
          "onlyDependOnLibsWithTags": ["scope:shared"]
        }
      ]
    }
  ]
}
```

### Turborepo

Focused on speed. Less opinionated than Nx.

```json
// turbo.json
{
  "pipeline": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": ["dist/**", ".next/**"]
    },
    "test": {
      "dependsOn": ["build"]
    },
    "lint": {},
    "dev": {
      "cache": false,
      "persistent": true
    }
  }
}
```

| Feature               | Details                                           |
| --------------------- | ------------------------------------------------- |
| Pipeline definition   | Declare task dependencies and cacheable outputs   |
| Content-aware hashing | Cache key includes file contents, not timestamps  |
| Remote caching        | Vercel Remote Cache (or self-hosted)              |
| Parallel execution    | Runs independent tasks simultaneously             |
| Pruned subsets        | `turbo prune --scope=web` creates minimal install |

```bash
# Run build for everything, with caching
turbo run build

# Run only for specific package and dependencies
turbo run build --filter=web...

# Dry run to see what would execute
turbo run build --dry-run
```

### Bazel

Google's build system. Hermetic, reproducible, language-agnostic.

| Feature                  | Details                                                       |
| ------------------------ | ------------------------------------------------------------- |
| **Hermetic builds**      | Build output depends only on declared inputs, not environment |
| **Remote execution**     | Distribute build/test across a cluster                        |
| **Remote caching**       | Shared cache across all developers and CI                     |
| **Fine-grained targets** | Build individual libraries, not whole projects                |
| **Multi-language**       | Java, Go, Python, C++, TypeScript, and more                   |

```python
# BUILD file
load("@rules_java//java:defs.bzl", "java_library", "java_test")

java_library(
    name = "user-service",
    srcs = glob(["src/main/**/*.java"]),
    deps = [
        "//libs/common:utils",
        "@maven//:com_google_guava_guava",
    ],
    visibility = ["//apps:__subpackages__"],
)

java_test(
    name = "user-service-test",
    srcs = glob(["src/test/**/*.java"]),
    deps = [":user-service", "@maven//:junit_junit"],
)
```

**When Bazel**: Large codebases (thousands of packages), multi-language, need hermetic reproducibility. High setup cost — worth it at scale, overkill for small teams.

### Other Tools

| Tool      | Focus                                           | Ecosystem                     |
| --------- | ----------------------------------------------- | ----------------------------- |
| **Pants** | Python/Go/Java, Bazel-like but simpler setup    | Python-heavy monorepos        |
| **Rush**  | pnpm-based, enterprise JS/TS monorepos          | Microsoft ecosystem           |
| **Lerna** | Original JS monorepo tool, now maintained by Nx | Legacy JS monorepos           |
| **Moon**  | Rust-based, fast, language-agnostic             | Newer alternative to Nx/Turbo |

### Tool Comparison

| Aspect            | Nx             | Turborepo      | Bazel              | Pants          |
| ----------------- | -------------- | -------------- | ------------------ | -------------- |
| Setup             | Medium         | Low            | High               | Medium         |
| Languages         | Any (TS-first) | Any (JS-first) | Any                | Python/Go/Java |
| Affected analysis | Yes            | Via git diff   | Yes (fine-grained) | Yes            |
| Remote caching    | Nx Cloud       | Vercel         | Built-in           | Built-in       |
| Remote execution  | Nx Agents      | No             | Yes                | Yes            |
| Learning curve    | Medium         | Low            | High               | Medium         |

## Workspace Protocols

### Package Manager Workspaces

```json
// package.json (root) — npm/yarn/pnpm workspaces
{
  "workspaces": ["packages/*", "apps/*"]
}
```

```yaml
# pnpm-workspace.yaml
packages:
  - "packages/*"
  - "apps/*"
```

### Internal Package References

```json
// packages/utils/package.json
{
  "name": "@myorg/utils",
  "version": "0.0.0",
  "main": "./src/index.ts"
}

// apps/web/package.json
{
  "dependencies": {
    "@myorg/utils": "workspace:*"
  }
}
```

pnpm's `workspace:*` protocol resolves to the local package. On publish, it's replaced with the actual version.

### TypeScript Project References

```json
// tsconfig.base.json (root)
{
  "compilerOptions": {
    "paths": {
      "@myorg/utils": ["packages/utils/src"],
      "@myorg/ui": ["packages/ui/src"]
    }
  }
}

// packages/ui/tsconfig.json
{
  "extends": "../../tsconfig.base.json",
  "references": [{ "path": "../utils" }],
  "compilerOptions": {
    "composite": true,
    "outDir": "dist"
  }
}
```

## Dependency Management

### Internal Packages

| Strategy              | How                              | Best For                        |
| --------------------- | -------------------------------- | ------------------------------- |
| Direct source imports | TypeScript paths, no build step  | Small/medium monorepos          |
| Pre-built packages    | Build libs before consuming apps | Large monorepos, complex builds |
| Publishable packages  | Build + publish to npm registry  | Open source, external consumers |

### Version Policies

| Policy          | Description                                                      |
| --------------- | ---------------------------------------------------------------- |
| **Fixed**       | All packages share one version, release together (Angular-style) |
| **Independent** | Each package versioned independently (Babel-style)               |
| **Mixed**       | Core packages fixed versioning, utilities independent            |

### External Dependency Management

```bash
# Ensure consistent versions across all packages
# pnpm: strict by default (hoisted with links)
# npm/yarn: dedupe to reduce duplicates
npm dedupe

# Renovate/Dependabot: one PR to update everywhere
# Single lockfile at the root covers all packages
```

## CI Optimization

### Affected Testing

Only test packages that changed or depend on changed packages:

```bash
# Nx
nx affected --target=test --base=origin/main

# Turborepo (filter by changed)
turbo run test --filter='...[origin/main]'

# Manual (git diff → package mapping)
changed_files=$(git diff --name-only origin/main)
# Map files to packages, run tests for those + dependents
```

### Incremental Builds

```yaml
# GitHub Actions with Nx
- uses: actions/cache@v4
  with:
    path: node_modules/.cache/nx
    key: nx-${{ hashFiles('**/package-lock.json') }}-${{ github.sha }}
    restore-keys: nx-${{ hashFiles('**/package-lock.json') }}-

- run: npx nx affected --target=build --base=origin/main
```

### Parallel CI

```yaml
# Matrix strategy — parallelize across packages
jobs:
  test:
    strategy:
      matrix:
        package: [web, api, shared-ui, utils]
    steps:
      - run: npx nx test ${{ matrix.package }}
```

## Code Ownership

### CODEOWNERS

```
# .github/CODEOWNERS
/apps/web/           @frontend-team
/apps/api/           @backend-team
/packages/shared-ui/ @design-system-team
/packages/auth/      @security-team
/packages/utils/     @platform-team  # Shared ownership
```

### Visibility Rules

Nx module boundaries or Bazel visibility restrict who can depend on what:

```python
# Bazel: only apps/ can depend on this
java_library(
    name = "internal-api",
    visibility = ["//apps:__subpackages__"],  # Not visible to other libs
)
```

## Migration from Polyrepo

### Strategies

| Strategy        | Approach                                  | Risk                           |
| --------------- | ----------------------------------------- | ------------------------------ |
| **Big bang**    | Merge all repos at once                   | High disruption, one-time pain |
| **Incremental** | Move one repo at a time, maintain bridges | Lower risk, longer migration   |
| **Codemods**    | Automated import path rewriting           | Reduces manual work            |

### Steps

1. Choose a monorepo tool and set up the skeleton
2. Move the lowest-dependency package first (shared utils)
3. Update import paths and CI configuration
4. Verify builds and tests pass
5. Update CODEOWNERS and access controls
6. Repeat for the next package, working up the dependency graph
7. Remove old repositories (or archive them)

### Preserving Git History

```bash
# Merge repo into monorepo subdirectory with history
cd monorepo
git remote add old-repo <url>
git fetch old-repo
git merge old-repo/main --allow-unrelated-histories --no-commit
git mv * packages/old-repo-name/  # Move to subdirectory
git commit -m "Import old-repo into monorepo"
git remote remove old-repo
```

For cleaner history, use `git filter-repo` to rewrite paths before merging.

## Anti-Patterns

| Anti-Pattern               | Problem                           | Fix                                     |
| -------------------------- | --------------------------------- | --------------------------------------- |
| No build tooling           | Everything rebuilds always        | Adopt Nx/Turbo/Bazel                    |
| Global CODEOWNERS          | No clear ownership                | Per-package owners                      |
| Circular dependencies      | Can't build or test independently | Refactor shared code into leaf packages |
| Monorepo as monolith       | Tight coupling between packages   | Enforce module boundaries               |
| Skipping affected analysis | CI runs everything, slow PRs      | Use affected commands                   |
