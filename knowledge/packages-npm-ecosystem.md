# npm Ecosystem: Architecture, Hoisting, and Dependency Management

## Overview

npm (Node Package Manager) is the default package manager for Node.js, managing dependencies for JavaScript and TypeScript projects. With over 2 million packages, npm's ecosystem is the largest by package count. Understanding npm's internal architecture—particularly its dependency resolution strategy, hoisting mechanism, and lockfile formats—is critical for avoiding common pitfalls like phantom dependencies and build reproducibility issues.

## npm Install Process and node_modules Layout

### The Three-Phase Install

When you run `npm install`, npm executes three phases:

1. **Resolution** — Navigates the dependency tree, resolves version constraints to specific versions, and detects conflicts
2. **Fetching** — Downloads resolved packages and their metadata from the registry
3. **Linking** — Installs packages into `node_modules/` and updates lockfiles

### Hoisting and the Flat node_modules Structure

npm v3 (2015) introduced **hoisting**: moving nested dependencies to the root of `node_modules/` to reduce duplication and disk usage. This differs from npm v1-v2, which used a pure nested structure.

```
// npm v3+: Flat hoisted structure
node_modules/
├── lodash@4.17.21/
├── express@4.18.2/
├── body-parser@1.20.0/  ← Hoisted from express's dep
└── .bin/ (executables)
```

The intention: A single dependency can be referenced by multiple dependents without duplication. The side effect: **Phantom dependencies**.

### The Phantom Dependency Problem

A phantom dependency occurs when code accesses an undeclared dependency because it was hoisted:

```javascript
// app.js: package.json does NOT list body-parser
require('body-parser')  // ✓ Works if express uses body-parser
                        // ✗ Breaks if you uninstall express
```

This happens because:
1. `express` depends on `body-parser`
2. npm hoists `body-parser` to `node_modules/` root
3. Your code can access it without declaring it
4. Remove `express`, and `body-parser` disappears

**Result**: Invisible, undeclared dependencies that break unpredictably.

### Version Mediation

npm uses **depth-first resolution**: the first occurrence of a package in the tree wins. If two dependencies require conflicting versions:

```
app
├── lodash@4.17.0
├── lib-a
│   └── lodash@4.15.0
```

npm keeps both versions: `lodash@4.17.0` at root and `lodash@4.15.0` nested under `lib-a`. This prevents version conflicts but increases disk usage and can cause duplicate code execution.

## npm's Dependency Declaration

### package.json Format

```json
{
  "dependencies": {
    "express": "^4.18.2",
    "lodash": "4.17.*"
  },
  "devDependencies": {
    "jest": "^29.0.0"
  },
  "peerDependencies": {
    "react": "^18.0.0"
  },
  "optionalDependencies": {
    "bufferutil": "^4.0.0"
  }
}
```

### Version Constraint Syntax (semver ranges)

- `^4.18.2` — Caret: "Allows changes that do not modify the left-most non-zero digit." = `≥4.18.2 <5.0.0`
- `~4.18.2` — Tilde: "Approximately equivalent to version." = `≥4.18.2 <4.19.0`
- `4.18.2` — Exact version
- `4.18.*` or `4.18` — Minor-level lock
- `*` or unspecified — Any version (not recommended)

### Peer Dependencies

Peer dependencies declare a **capability** your package provides or requires:

```json
{
  "peerDependencies": {
    "react": "^18.0.0",
    "react-dom": "^18.0.0"
  }
}
```

Consumers of your package must install the peer dependency themselves. This is commonly used in plugins (e.g., a React component library for a host React app). **Key point**: npm v7+ emits a warning if peer dependencies are unmet; npm v6 just warned; npm v3-v5 silently ignored them.

### Optional Dependencies

Marked as `optionalDependencies`: npm ignores failures if these don't install (e.g., native addons that compile only on certain platforms).

```
npm install  # Succeeds even if bufferutil fails to build
```

## npm Lockfiles: package-lock.json

The lockfile locks the entire resolved tree to a specific set of versions, ensuring reproducible installs across developers and CI/CD systems.

### Structure

```json
{
  "lockfileVersion": 3,
  "name": "my-app",
  "version": "1.0.0",
  "requires": true,
  "packages": {
    "": {
      "name": "my-app",
      "version": "1.0.0",
      "dependencies": {
        "express": "^4.18.2"
      }
    },
    "node_modules/express": {
      "version": "4.18.2",
      "resolved": "https://registry.npmjs.org/express/-/express-4.18.2.tgz",
      "integrity": "sha512-...",
      "dependencies": {
        "body-parser": "1.20.0"
      }
    }
  }
}
```

**Key fields**:
- `lockfileVersion` — Format version (3 = npm v7+; includes nested `packages` object)
- `resolved` — Exact URL to the tarball
- `integrity` — SHA-512 hash for verification
- `requires` — Boolean: whether nested packages are listed

### Why Lockfiles Matter

1. **Reproducibility** — Different machines install identical versions
2. **Security** — CI/CD always installs known-good versions, not latest
3. **Rollback** — Revert to known-working state without changing `package.json`

**Gotcha**: Lockfile can diverge from `package.json` if dependencies are manually edited. Run `npm install` to resync.

## npm Scripts and Lifecycle Hooks

### Lifecycle Scripts

npm recognizes built-in lifecycle phases and allows pre/post hooks:

```json
{
  "scripts": {
    "preinstall": "echo Installing...",
    "install": "npm run build",
    "postinstall": "npm run test",
    "pretest": "npm run lint",
    "test": "jest",
    "posttest": "npm run coverage",
    "prebuild": "npm run clean",
    "build": "tsc",
    "postbuild": "npm run minify"
  }
}
```

**Lifecycle events** (automatically called):
- `install`, `postinstall` — After `npm install`
- `uninstall` — During `npm uninstall`
- `prestart`, `start`, `poststart` — With `npm start`

**Custom scripts** (invoked with `npm run`):
- `test` — `npm test`
- `build` — `npm run build`
- Any string you define

### Pre/Post Hook Pattern

For any script `name`, npm automatically runs `prename` (before) and `postname` (after). Use cases:

```json
{
  "scripts": {
    "prebuild": "npm run clean",
    "build": "tsc --declaration",
    "postbuild": "npm run minify && npm run copy-files"
  }
}
```

**Security concern**: Lifecycle scripts run with elevated permissions. Malicious packages can abuse `postinstall` hooks to exfiltrate data or inject backdoors.

### Environment Variables

npm injects `npm_` prefixed environment variables into script execution:

```bash
# Inside a script, these are available:
npm_package_name
npm_package_version
npm_package_description
npm_config_registry  # The npm registry URL
NODE_ENV=production npm run build
```

## .npmrc Configuration

### File Locations and Precedence

npm searches for configuration in this order (first match wins):

1. per-project: `./npmrc` (in the working directory)
2. per-user: `~/.npmrc` (home directory)
3. global: `/usr/local/etc/npmrc`
4. built-in defaults

### Common .npmrc Settings

```ini
# Registry configuration
registry=https://registry.npmjs.org/
@babel:registry=https://registry.npmjs.org/

# Authentication
//registry.npmjs.org/:_authToken=YOUR_TOKEN
//private.registry.com/:username=myuser
//private.registry.com/:_password=PASSWORD_BASE64

# Scoped packages (private registry override)
@mycompany:registry=https://private.company.com/npm/

# Security
audit=true
audit-level=moderate

# Performance
depth=0  # Shallow dependency tree
fetch-timeout=60000

# Legacy settings
legacy-peer-deps=true  # Use npm v6 peer dependency behavior
force=true  # Force npm to ignore conflicts
```

### Scoped Registries

Use scopes (e.g., `@babel`, `@mycompany`) to route packages to different registries:

```ini
@babel:registry=https://registry.npmjs.org/
@mycompany:registry=https://private.registry.com/npm/
```

This allows hosting internal packages separately from public ones.

### Authentication Methods

- **Token authentication**: `_authToken` — Single-use or long-lived token
- **Basic auth**: `username` + `_password` (Base64, not plaintext)
- **OAuth**: Some registries (GitHub, npm) support OAuth tokens

**Best practice**: Use `npm login` to automatically create and store `_authToken`; never commit `.npmrc` with credentials to version control.

## npm vs. pnpm vs. Yarn: Key Differences

### pnpm: Strict Isolation

pnpm eliminates hoisting and phantom dependencies via a **content-addressable file store**:

```
node_modules/
└── .pnpm/
    ├── lodash@4.17.21/
    │   └── node_modules/ (only its explicit deps)
    └── express@4.18.2/
        └── node_modules/
            └── body-parser@1.20.0/
```

**Advantages**:
- No phantom dependencies (strict enforcement)
- 50-70% less disk space (hardlinks to central store)
- Faster installs (parallel download + deduplicated storage)
- Better for monorepos

**Trade-off**: Less familiar hoisting behavior; some legacy tools expect hoisted node_modules.

### Yarn: Community Alternative

Yarn (Facebook) introduced **workspaces** for monorepo support and features:
- Deterministic lockfile (yarn.lock)
- Parallel downloads
- Offline mode caching
- Better error messages

Modern projects are trending toward **pnpm** (stricter, faster) or **npm v7+** (now with workspaces support). Yarn remains popular but development has slowed.

## Monorepo Support: npm Workspaces

npm v7+ and Yarn support **workspaces**, allowing multiple packages to coexist in a single repo:

```json
{
  "name": "monorepo",
  "workspaces": [
    "packages/web",
    "packages/cli",
    "packages/shared"
  ]
}
```

**package.json** (packages/shared):
```json
{
  "name": "@monorepo/shared",
  "version": "1.0.0"
}
```

**Benefits**:
- Single lockfile for all packages
- Shared dependencies deduplicated
- Cross-package dependency resolution (local packages take precedence)
- Unified CI/CD

**Limitation**: npm workspaces don't support nested workspaces; pnpm does.

## Key Takeaways

1. **Phantom dependencies** are npm's Achilles heel—always declare all imports in `package.json`
2. **Lockfiles are mandatory** for reproducible, secure builds; commit `package-lock.json` to version control
3. **Hoisting reduces disk usage** but increases complexity; pnpm avoids this trade-off
4. **npm scripts** are powerful but security-sensitive; never run untrusted `postinstall` scripts
5. **Scoped registries** (.npmrc) enable private package hosting without duplicating credentials
6. **Workspaces** simplify monorepo management; npm v7+, Yarn, and pnpm all support them

## See Also

- [tools-package-managers.md](tools-package-managers.md) — Broader comparison of package managers across languages
- [process-dependency-management.md](process-dependency-management.md) — Dependency updates and vulnerability scanning
- [api-authentication.md](api-authentication.md) — Registry authentication patterns