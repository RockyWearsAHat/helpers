# Web Monorepo Tooling — Nx, Turborepo, Task Orchestration & CI Optimization

## Monorepo Context

A monorepo houses multiple projects (applications, libraries, packages) in a single Git repository. Contrast: polyrepo (separate repos per project).

**Monorepo benefits:** shared code reuse, atomic commits across packages, coordinated releases, simplified dependency management, easier refactoring.

**Monorepo challenges:** build complexity increases, CI performance critical, tooling must orchestrate many tasks efficiently, dependency graphs become intricate.

Modern monorepo tools address these challenges through **intelligent task orchestration** and **computation caching**.

## Nx — Computation Caching & Task Graph

Nx orchestrates tasks across monorepo packages by building a **computation task graph**: which tasks depend on which, what can run in parallel, what outputs can be cached.

### Core Concepts

**Workspace structure:**
```
workspace/
├── apps/
│   ├── web/
│   ├── mobile/
│   └── admin/
├── libs/
│   ├── ui/
│   ├── utils/
│   └── api-client/
├── nx.json
└── workspace.json
```

**Task graph generation:**
Nx analyzes `package.json` scripts and build configuration to understand:
- Which packages depend on which
- Which tasks can run in parallel
- Which tasks depend on other tasks completing first

Example: `build` task for app might depend on `build` tasks for all dependencies first.

### Computation Caching

Nx caches task outputs:
- Hash inputs: source code, configuration, environment variables
- Store outputs: built artifacts, test results, generated files
- On re-run: if hash matches cache, restore outputs instead of re-running

**Caching saves time**: if you change only `apps/web`, Nx doesn't rebuild `libs/api-client`. It uses cached output.

**Local caching:** Stores cache on developer's machine.  
**Remote caching:** Stores cache on shared server (Nx Cloud, S3, custom). Team members and CI systems share cache.

### Affected Task Filtering

Instead of building everything:
```bash
nx run-many --target=build --all               # build everything
nx affected:build --base=main                  # build only what changed from main
```

`affected` reads git diff, determines which packages changed, runs tasks only for those packages and their dependents.

Example: PR changes `libs/ui`. Affected filtering rebuilds `libs/ui` + all apps that depend on it, skips untouched packages.

### Plugins & Executors

Nx provides plugins for framework-specific tasks:
- `@nx/react`: React app setup, build config
- `@nx/next`: Next.js app scaffolding, build
- `@nx/nest`: NestJS backend setup
- `@nx/node`: Node.js library/app
- `@nx/vite`, `@nx/webpack`: Bundler integration

Plugins define **executors** (how to run a task) and **generators** (scaffolding code).

### Distributed Task Execution

On CI, Nx agents parallelize task execution across machines:
- Machine 1: tests `apps/web`
- Machine 2: tests `apps/admin` 
- Machine 3: builds `libs/ui`

Tasks run in parallel; results aggregated. Total time = longest task, not sum.

## Turborepo — Lightweight, Incremental Builds

Turborepo focuses on **incremental builds** with minimal configuration.

### Core Concepts

**turbo.json configuration:**
```json
{
  "tasks": {
    "build": {
      "dependsOn": ["^build"],
      "inputs": ["src/**"],
      "outputs": ["dist/**"]
    },
    "test": {
      "dependsOn": ["^build"],
      "inputs": ["src/**", "test/**"],
      "outputs": [".coverage/**"]
    }
  }
}
```

- `dependsOn`: task dependencies (`^` means dependency's task must run first)
- `inputs`: files that affect this task's output
- `outputs`: cache keys (what to store)

Turborepo hashes inputs, caches outputs. Incremental: only rebuild if inputs change.

### Speed

Turborepo is designed to be **fast to learn and fast to run**:
- Simpler than Nx for smaller monorepos
- Faster startup
- Less overhead

Trade-off: less introspection than Nx. Nx provides deeper visibility into the task graph.

### Remote Caching (Vercel)

Turborepo integrates with **Vercel Remote Cache** (hosted by Vercel):
```bash
turbo login                  # authenticate with Vercel
turbo build --remote-only    # use only remote cache
```

Developers and CI share cache. A built artifact on one developer's machine is immediately available on another's, saving rebuild time.

### Monorepo Scope

Turborepo is opinionated toward monorepos but lighter-weight than Nx. Suited for 5-50 packages.

## Lerna — Package Publishing Orchestration

Lerna focuses on **publishing multiple packages** from a monorepo.

### Versioning Modes

**Fixed:** All packages share one version number. Version bump applies to all.
```
lerna version minor  # bumps v1.2.0 → v1.3.0 for all packages
```

**Independent:** Each package has its own version. Bump per-package as needed.

### Publishing Workflow

```bash
lerna version               # read commits, propose version bumps
lerna publish               # publish to npm registry
git push origin --tags      # push version tags
```

Lerna reads conventional commits (`feat:`, `fix:`, `BREAKING CHANGE:`) to auto-determine versions (semver).

### Use Cases

- **Component library**: Multiple exports as separate packages (buttons, forms, icons). Each package versioned independently.
- **Framework plugins**: Core + plugins. Core bumps less frequently; plugins bump independently.

Lerna doesn't replace Turborepo/Nx; they're complementary. Turborepo/Nx handle builds; Lerna handles publishing.

## pnpm Workspaces — Dependency Management

**pnpm** is a package manager optimized for monorepos through **workspaces**.

### Workspace Structure
```yaml
# pnpm-workspace.yaml
packages:
  - 'apps/*'
  - 'libs/*'
```

All packages in workspace share one `node_modules`, reducing disk space (~3x smaller than npm/yarn).

### Depedency Hoisting and Resolution

pnpm uses **strict dependency resolution**: packages can only use dependencies they explicitly declare. Prevents accidental transitive dependency leaks.

Example: `apps/web` depends on `lodash`. Old npm might hoist `lodash` so `libs/utils` could use it without declaring it. pnpm prevents this; `libs/utils` must explicitly declare `lodash`.

**Benefit:** Cleaner dependency graph, fewer surprises when upgrading.

### Workspace Protocol

Link local packages:
```json
{
  "dependencies": {
    "@monorepo/ui": "workspace:*"
  }
}
```

`workspace:*` means use the local package version, not npm registry. Auto-resolves to correct version.

## Module Federation — Sharing Code Across Micro-Frontends

For large monorepos with independent micro-frontends (separate deployments), **module federation** (webpack, Vite) allows runtime code sharing:

- `apps/admin` deploys independently, exposes `admin-ui` module
- `apps/dashboard` imports `admin-ui` at runtime
- Shared modules (React, utilities) loaded once, not bundled twice

Webpack module federation config:
```javascript
// apps/admin/webpack.config.js
new ModuleFederationPlugin({
  name: 'admin',
  exposes: {
    './Dashboard': './src/Dashboard.tsx',
  },
})

// apps/dashboard/webpack.config.js
new ModuleFederationPlugin({
  remotes: {
    admin: 'admin@http://localhost:3001/remoteEntry.js',
  },
})
```

**Trade-offs:** Version mismatches at runtime, debugging complexity, federation overhead.

## Dependency Management in Monorepos

### Challenges

1. **Version consistency**: Multiple packages, shared dependencies. If `libs/ui` and `libs/api-client` both use `react`, must be same version to avoid duplicates.

2. **Circular dependencies**: Hard to detect. `libs/a` imports from `libs/b` which imports from `libs/a`. Task graph fails.

3. **External dependency duplication**: If `apps/web` and `apps/admin` both bring in `lodash` with different versions, bundle size bloats.

### Solutions

- **Single version policy**: All packages use exact same versions. Lock file enforced.
- **Dependency graph visualization**: Tools like `pnpm ls` or Nx graph reveal circular dependencies.
- **Shared dependencies**: Extract common dependencies to `libs/shared`, all packages import from there.
- **Workspaces protocol**: pnpm's `workspace:*` ensures internal packages use local versions.

## CI Optimization for Monorepos

### Test & Build Filtering

Instead of running all tests on every commit:

**Nx:**
```bash
nx affected:test --base=main          # test only changed packages
```

**Turborepo:**
```bash
turbo run test --filter='...[HEAD~1]' # test changed + dependents
```

Saves CI time 10-100x for large monorepos.

### Parallel Job Distribution

CI systems (GitHub Actions, GitLab CI, CircleCI) spawn multiple machines:
```yaml
# GitHub Actions
jobs:
  dist1:
    runs-on: ubuntu-latest
    steps:
      - run: npx nx affected:test --workspaceRoot --ci --nxCloud
  dist2:
    runs-on: ubuntu-latest
    steps:
      - run: npx nx affected:test --workspaceRoot --ci --nxCloud
```

Both agents run tasks; Nx coordinates via cloud. Total time = longest job, ~12 min instead of 40 min.

### Caching Layer

Remote cache (Nx Cloud, Turborepo Vercel):
- Local builds on developers' machines cache results
- CI system reuses cache
- Identical task on same commit: zero time (cache hit)

Example: PR builds successfully locally. Pushed to CI. Same build artifacts restored from cache in seconds.

### Shallow Clones

Reduce clone time:
```bash
git clone --depth 1 https://repo  # shallow clone, fast
```

Git history not needed for build. Only matters for `git diff --base`.

## Tool Comparison

| Aspect | Nx | Turborepo | Lerna | pnpm |
|--------|----|-----------|----- |------|
| **Task orchestration** | Powerful, introspective | Simple, fast | Limited | N/A (Package mgr) |
| **Caching** | Local + cloud (Nx Cloud) | Local + cloud (Vercel) | None | Workspace hoisting |
| **Affected runs** | Yes (`affected:*`) | Yes (`--filter`) | No | N/A |
| **Plugin ecosystem** | Large (React, Next, Node) | Minimal | Minimal | Integrates with all |
| **Learning curve** | Steep | Gentle | Gentle | Moderate |
| **Monorepo size** | 10-1000+ packages | 5-50 packages | Any size | Any size |
| **Publishing** | Via pipelines | Via scripts | Native support | Via lerna overlay |
| **Best for** | Enterprise, complex | Startups, simple | Multi-package releases | Any Node monorepo |

## Common Patterns

### Shared Configuration
Extract `tsconfig.json`, `.eslintrc`, `jest.config.js` to root or shared package, referenced by all packages.

### Shared Dependencies
Create `libs/shared` for common utilities, constants, types. All packages import from here.

### CI Gates
- Lint, test, build, type-check must pass before merge
- Remote cache speeds up CI
- Affected filtering ensures only relevant tasks run

### Release Workflow

1. Developer commits with conventional commits
2. PR passes CI (tests, lint, affected builds)
3. Merge to main
4. CI automatically:
   - Detects version bumps (from commits)
   - Publishes updated packages to npm
   - Tags git release
   - Updates CHANGELOG

## See Also

- architecture-monorepo.md — monorepo fundamentals
- tools-package-managers.md — npm, pnpm, yarn
- tools-ci-cd-pipeline-design.md — CI/CD in monorepos
- build-systems-concepts.md — build graph concepts