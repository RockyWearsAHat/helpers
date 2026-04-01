# Release Engineering — Semantic Versioning, Automation & Artifact Management

## Overview

Release engineering is the discipline of packaging, versioning, and deploying artifacts (code, documentation, artifacts) to users in a controlled, repeatable manner. It spans from commit authoring conventions through automated version bumping, changelog generation, artifact signing, and provenance tracking.

The goal: minimize manual toil while maintaining traceability. A good release process is **deterministic** (same inputs always produce same outputs), **auditable** (who made what change), and **reversible** (rollback is possible).

## Semantic Versioning (Semver)

Semantic versioning is a versioning scheme that encodes breaking changes into version numbers, enabling consumers to make intelligent upgrade decisions.

### Format: MAJOR.MINOR.PATCH

```
v1.3.2
 │ │ └─ PATCH: Bug fixes, backward-compatible changes
 │ └─── MINOR: New features, backward-compatible
 └───── MAJOR: Breaking changes
```

**PATCH (1.3.0 → 1.3.1):** Only bug fixes. Consumers should upgrade automatically; zero risk.

**MINOR (1.3.0 → 1.4.0):** New features, but all old code still works. Consumers can safely upgrade; new optional features available.

**MAJOR (1.3.0 → 2.0.0):** Breaking change. Old code will not work without modification. Consumers must test and validate before upgrading.

### Rules

1. Once a version is released, never modify it. Use a new version number.
2. After 1.0.0 release, only increment according to breaking/feature/bugfix rules.
3. Increment MAJOR only for breaking changes (removing API, changing function signature, etc.).
4. Increment MINOR even if MAJOR hasn't changed (e.g., 1.0.0 → 1.1.0 is valid).
5. Zero versions (0.y.z) can change API freely; any increment can be breaking.

### Pre-release and Build Metadata

```
v2.0.0-beta.1
v2.0.0-rc.1
v2.0.0-alpha.1.2
v2.0.0+build.123
```

**Pre-release** (hyphen): Indicates not-yet-stable; semantically precedes release (2.0.0-beta < 2.0.0).

**Build metadata** (plus): Additional info (commit hash, build timestamp); ignored for precedence (2.0.0+build.1 == 2.0.0 precedence-wise).

### Implications for Dependency Managers

Tools like Renovate and Dependabot use semver to automate dependency updates:

- Patch bumps: Auto-merge (safe)
- Minor bumps: Auto-merge (safe)
- Major bumps: Flag for review (breaking)

A library that violates semver (e.g., calls a "patch" release that breaks API) will silently break dependents' builds.

## Conventional Commits

Conventional Commits is a specification for commit message format that enables **automated changelog generation and semantic version bumping**.

### Format

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Types:**

- `feat:` — A new feature; triggers MINOR bump
- `fix:` — A bug fix; triggers PATCH bump
- `BREAKING CHANGE:` — In body or footer; triggers MAJOR bump
- `docs:`, `style:`, `refactor:`, `perf:`, `test:`, `chore:` — Not in changelog; no version bump by default

### Examples

```
feat(auth): add two-factor authentication

Add support for TOTP-based 2FA alongside password authentication.
Partially resolves #123.
```

→ MINOR version bump; entry in changelog: "Add two-factor authentication"

```
fix(api): return correct error code for invalid token

Previously returned 500; now returns 401 Unauthorized.

Fixes #456
```

→ PATCH version bump; changelog: "Return correct error code for invalid token"

```
feat(database): migrate to PostgreSQL

BREAKING CHANGE: Requires PostgreSQL 12+; MySQLconnection strings no longer supported.
See MIGRATION.md for upgrade guide.
```

→ MAJOR version bump; changelog marks as breaking.

### Integration with Release Automation

Tools like **semantic-release** and **release-please** parse commit history, determine version bump, generate changelog, and create GitHub release automatically:

```
Commits since last version:
  feat(auth): add 2FA
  fix(api): error code
  chore: update deps

→ Version determination: MINOR (feat is the highest)
→ New version: 1.2.0 (was 1.1.0)
→ Changelog entry:
   ### Features
   - Add two-factor authentication
   
   ### Bug Fixes
   - Return correct error code for invalid token
```

## Release Automation Tools

### semantic-release

Full automation: commits → determine version → generate changelog → create GitHub release → publish to npm, PyPI, etc.

**Setup:**

```bash
# Install
npm install -D semantic-release

# Configure in package.json or .releaserc.js
```

```json
{
  "release": {
    "branches": ["main"],
    "plugins": [
      "@semantic-release/commit-analyzer",
      "@semantic-release/release-notes-generator",
      "@semantic-release/github",
      "@semantic-release/npm"
    ]
  }
}
```

**Workflow:**

1. Developer commits with conventional message (`feat: add feature`)
2. PR is merged to main
3. CI/CD runs semantic-release
4. Commits analyzed; version bumped
5. Changelog generated
6. GitHub release created
7. Package published to npm
8. Version tag pushed to git

**Pros:**
- Fully automated; zero manual steps
- Deterministic (same commits always produce same version)
- Supports multiple repositories and languages

**Cons:**
- Opinionated (enforces conventional commits, GitHub, npm)
- Can be fragile if commit messages are malformed

### release-please (Google)

Release management automation; creates PRs for version bumps and changelog generation. Released by humans, but PR is auto-generated.

**Workflow:**

1. Merge feature commits to main
2. CI runs release-please
3. Release-please opens a PR: "Bump version to 1.2.0, update CHANGELOG"
4. Human reviews PR (changelog looks good, version makes sense)
5. Merge PR
6. CI detects merged release PR; publishes release and package

**Difference from semantic-release:**
- Separation of concerns: automation creates PR, human merges
- More control (can edit changelog before merge)
- Less magical; easier to debug

**Setup:**

```yaml
# .github/workflows/release-please.yml
on:
  push:
    branches: [main]

jobs:
  release-please:
    runs-on: ubuntu-latest
    steps:
      - uses: googleapis/release-please-action@v4
        with:
          release-type: node
```

### Changesets (Yarn/npm Ecosystem)

Stores version bump intent separately from commits. Developers author changesets detailing what changed; release process uses changesets to version.

**Workflow:**

1. Developer adds changeset: `yarn changeset` → creates `.changeset/*.md` file
2. Changeset describes: type (major/minor/patch), description, affected packages
3. PR review includes changeset review (version intent is explicit)
4. After merge, run `yarn changeset version` → updates package.json, CHANGELOG.md
5. Run `yarn changeset publish` → publishes packages

**Example changeset file:**

```markdown
---
"@acme/ui": minor
"@acme/cli": major
---

Add support for dark mode theming across all components.
Renamed `theme` prop to `colorScheme` in UI library (breaking).
```

**Advantages:**
- Explicit version intent (no guessing from commits)
- Great for monorepos (multiple packages, each has its version)
- Human-friendly changelog (handwritten descriptions)

**Disadvantages:**
- More manual than semantic-release (human writes changeset)
- Less enforcement (easy to forget changeset)

## Changelog Management

### Keep a Changelog Format

Standardized change log format (keepachangelog.com):

```markdown
# Changelog

## [Unreleased]
### Added
- New dark mode support

### Fixed
- Bug in user registration

## [1.0.0] - 2025-03-26
### Added
- Initial public release

### Changed
- Refactored authentication module

### Fixed
- Memory leak in connection pooling
```

**Conventions:**

- Releases are links to git tags: `[1.0.0]`
- Sections: Added, Changed, Deprecated, Removed, Fixed, Security
- Write in past tense, user-facing language (not "implemented caching", but "cache queries for faster response")

### Automated Changelog Generation

Tools like semantic-release and release-please generate changelog from commits:

```javascript
// semantic-release configuration
{
  "plugins": [
    ["@semantic-release/commit-analyzer"],
    ["@semantic-release/release-notes-generator", {
      "preset": "conventionalcommits"
    }],
    ["@semantic-release/github"]
  ]
}
```

Generates changelog automatically from conventional commits. Trade-off: less human polish, but deterministic.

## Release Trains and Scheduled Releases

Some teams use release trains: predetermined release windows (e.g., every 2 weeks on Thursday at 2pm). Changes are accumulated; one release contains many features.

**Advantages:**
- Predictable schedule (users know when new versions arrive)
- Batch testing (multiple features tested together)
- Fewer release processes to maintain

**Disadvantages:**
- Hotfixes are delayed until next train (or require special process)
- Risk accumulates (more changes in one release = more potential bugs)

**Calendar-based example:**

```
Release Train Schedule:
  Sprint 1: weeks 1-2, release on Friday
  Sprint 2: weeks 3-4, release on Friday
  ...
  Main branch receives commits continuously
  On release Friday: tag main as vX.Y.Z, deploy
```

## Release Candidates and Staging

A release candidate (RC) is a pre-release version expected to become stable. Used for final validation:

```
v1.0.0-rc.1 (candidate 1)
v1.0.0-rc.2 (bug fixes in RC; candidate 2)
v1.0.0 (release; rc is now stable)
```

**Workflow:**

1. Freeze main branch (only fixes, no new features)
2. Create release branch or tag `v1.0.0-rc.1`
3. Deploy RC to staging environment
4. Run full test suite, manual testing, performance profiling
5. If bugs found, fix on release branch; cherry-pick to main
6. Tag as `v1.0.0-rc.2`; repeat
7. When confident, tag as `v1.0.0` (remove -rc)

## Artifact Signing and Provenance

As software supply chain attacks increase, proof of provenance and integrity becomes critical.

### Code Signing

Git commits and tags can be signed with GPG/SSH to prove authorship:

```bash
# Sign a tag
git tag -s v1.0.0 -m "Release version 1.0.0"

# Verify signature
git tag -v v1.0.0
```

GitHub and other platforms verify signatures, marking verified commits with a green badge.

**Use cases:**
- Prove that a maintainer (not an attacker) authored the release
- Prevent tag spoofing

### Artifact Signing and Binary Provenance

Package artifacts (JAR, wheel, tarball, container image) should be signed:

```bash
# Sign a tarball with GPG
gpg --armor --detach-sign app-1.0.0.tar.gz
# Creates app-1.0.0.tar.gz.asc

# User verifies
gpg --verify app-1.0.0.tar.gz.asc app-1.0.0.tar.gz
# Returns OK or FAILED
```

### SLSA Framework (Supply chain Levels for Software Artifacts)

SLSA is a framework for verifying software hasn't been tampered with. Provides:

- **Provenance attestation:** Machine-readable proof of what produced the artifact (which source code, build parameters, environment)
- **Signed certificates:** Cryptographically signed proof that cert matches artifact

Example SLSA provenance:

```json
{
  "builder": "https://github.com/org/builder",
  "sourceUri": "git@github.com:org/repo@commit:abc123",
  "buildArgs": {"CFLAGS": "-O2"},
  "builtAt": "2025-03-26T01:02:03Z",
  "signature": "..."
}
```

Infrastructure: GitHub Actions provides SLSA provenance natively via `actions/attest-build-provenance`.

## Hotfixes and Patch Releases

Urgent bugs in production require hotfixes. Typical workflow:

```
1. Branch from last release tag (e.g., v1.0.0)
2. Fix bug on hotfix branch
3. Tag as v1.0.1
4. Cherry-pick fix back to main (so main also has the fix)
```

**Git Flow example:**

```bash
# Create hotfix branch from release
git checkout -b hotfix/critical-security-bug v1.0.0

# Make fix
# Commit, test

# Tag as patch version
git tag v1.0.1

# Merge back to main
git checkout main
git merge hotfix/critical-security-bug
git push origin main v1.0.1
```

## Multi-Version Support

Some projects maintain multiple release series (e.g., 1.x, 2.x) receiving patches independently:

```
v1.5.x (patch support)
  v1.5.0, v1.5.1, v1.5.2 (all bugfixes, no new features)
v2.0.x (current stable)
  v2.0.0, v2.0.1, v2.0.2, v2.1.0 (features + bugfixes)
v3.0.0-alpha (future)
```

**Requires discipline:**
- Backporting fixes (cherry-pick fix to older release branches)
- Version documentation (which versions are supported)
- Deprecation policy (which versions are EOL)

## Dependency Management and Version Constraints

Downstream projects declare version constraints on your library:

- `^1.2.3` (npm): Bump minor/patch, not major (1.2.3 < 1.3.0 < 2.0.0 would break)
- `~1.2.3` (npm): Bump patch only (1.2.3 < 1.2.4, but not 1.3.0)
- `>=1.0, <2.0` (Maven): Any version in range; bumping 2.0 breaks constraint

Semantic versioning enables automatic dependency updates: Dependabot auto-merges patch/minor, flags major.

## Release Checklist

```
- [ ] Version bumped in package.json / pyproject.toml / etc.
- [ ] CHANGELOG updated with human-readable descriptions
- [ ] All tests pass (unit, integration, end-to-end)
- [ ] No breaking changes without MAJOR version bump
- [ ] Breaking changes documented in MIGRATION.md
- [ ] Commit(s) follow conventional commit format (if using semantic-release)
- [ ] Tag is signed (git tag -s)
- [ ] Release notes on GitHub are complete
- [ ] Artifacts are built and signed
- [ ] Security audit of dependencies (no critical CVEs)
- [ ] Documentation updated for new API
- [ ] Performance benchmarks run (especially for libraries)
```

## Common Mistakes

**Mistake: "Forgot to bump version, released with old version number."**
- Fix: Use automated versioning (semantic-release, release-please)

**Mistake: "Breaking change released as patch (1.0.0 → 1.0.1)."**
- Fix: Use conventional commits + semantic versioning; enforce MAJOR for breaking changes in review

**Mistake: "No changelog; users don't know what changed."**
- Fix: Automate changelog from commits or require changeset for every PR

**Mistake: "We released a security fix, but old users are still running vulnerable version."**
- Fix: Public security advisories; communicate EOL dates for old versions

**Mistake: "Can't figure out when a bug was fixed or which version contains a feature."**
- Fix: Use individual commit SHAs in changelog; provide version-to-commit mapping

## See Also

- **process-release-management.md** — Release trains, freeze periods, staged rollouts
- **version-control-workflows.md** — Git workflows (trunk-based, Git Flow)
- **devops-cicd-patterns.md** — CI/CD automation around releases
- **security-secrets-management.md** — Signing keys and credential management for release pipelines