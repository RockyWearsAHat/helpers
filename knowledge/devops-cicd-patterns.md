# CI/CD Patterns & Release Engineering

## Branching Strategies

### Trunk-Based Development

Single integration branch (`main`/`trunk`). All developers commit directly or via short-lived feature branches (< 24h). Requires strong CI, feature flags, and automated testing.

```
main ──●──●──●──●──●──●──●──  (always deployable)
        \─●─/   \─●─/          (short-lived branches, < 1 day)
```

**Key practices:**

- Feature flags decouple deployment from release
- Branch by abstraction for large refactors
- No long-lived branches — everything merges to trunk daily
- Release from trunk (tag or cut release branch at ship time)

### Feature Branch Workflow

Longer-lived branches with PR review before merge. Common in open-source and teams with formal review gates.

| Aspect               | Trunk-Based         | Feature Branch     |
| -------------------- | ------------------- | ------------------ |
| Branch lifetime      | Hours               | Days to weeks      |
| Merge frequency      | Multiple/day        | Once on completion |
| Merge conflicts      | Rare, small         | Frequent, complex  |
| CI feedback loop     | Immediate           | Delayed            |
| Feature flags needed | Yes                 | Optional           |
| Code review          | Post-commit or pair | Pre-merge PR       |

### Merge Queues

Serialize merges to prevent broken main. GitHub merge queues, Bors, Mergify:

```yaml
# GitHub merge queue settings
merge_queue:
  merge_method: squash
  min_entries_to_merge: 1
  max_entries_to_merge: 5
  grouping_strategy: ALLGREEN
  entry_checks:
    - ci/build
    - ci/test
```

**How it works:** PR approved → enters queue → rebased on latest main + other queued PRs → CI runs on combined state → merges if green, ejects if red. Prevents "merge skew" where two individually-passing PRs break together.

## Commit Conventions

### Conventional Commits

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

| Type              | Purpose                    | Semver bump |
| ----------------- | -------------------------- | ----------- |
| `feat`            | New feature                | MINOR       |
| `fix`             | Bug fix                    | PATCH       |
| `docs`            | Documentation only         | none        |
| `style`           | Formatting, no code change | none        |
| `refactor`        | Neither fix nor feat       | none        |
| `perf`            | Performance improvement    | PATCH       |
| `test`            | Adding/fixing tests        | none        |
| `chore`           | Build, CI, tooling         | none        |
| `BREAKING CHANGE` | Footer or `!` after type   | MAJOR       |

```bash
feat(auth): add OAuth2 PKCE flow
fix!: remove deprecated API endpoint    # ! = breaking change
chore(deps): bump express to 4.19.2
```

### Semantic Versioning

`MAJOR.MINOR.PATCH` — MAJOR for breaking changes, MINOR for features, PATCH for fixes.

Pre-release: `1.0.0-alpha.1`, `1.0.0-beta.3`, `1.0.0-rc.1`
Build metadata: `1.0.0+build.123`

**Version precedence:** `1.0.0-alpha < 1.0.0-alpha.1 < 1.0.0-beta < 1.0.0-rc.1 < 1.0.0`

## Release Automation

### release-please (Google)

Maintains a release PR that accumulates conventional commits. Merging the PR creates a GitHub Release with changelog.

```yaml
# .github/workflows/release.yml
on:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: googleapis/release-please-action@v4
        with:
          release-type: node # or python, rust, simple, etc.
          token: ${{ secrets.GITHUB_TOKEN }}
```

Generates `CHANGELOG.md`, bumps version files, creates GitHub Release with notes. Supports monorepo via manifest config.

### semantic-release

Fully automated — no release PR. Analyzes commits since last release, determines version bump, publishes.

```json
// .releaserc.json
{
  "branches": ["main", { "name": "beta", "prerelease": true }],
  "plugins": [
    "@semantic-release/commit-analyzer",
    "@semantic-release/release-notes-generator",
    "@semantic-release/changelog",
    "@semantic-release/npm",
    "@semantic-release/github",
    "@semantic-release/git"
  ]
}
```

| Feature       | release-please     | semantic-release |
| ------------- | ------------------ | ---------------- |
| Release PR    | Yes (human merges) | No (fully auto)  |
| Changelog     | Auto-generated     | Auto-generated   |
| Monorepo      | Native manifest    | Via plugins      |
| npm publish   | Manual/separate    | Built-in         |
| Customization | Release types      | Plugin system    |

## Deployment Strategies

### Rolling Deployment

Replace instances incrementally. At any point, some run old version, some new. Default for Kubernetes Deployments.

```yaml
spec:
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 25% # max extra pods during update
      maxUnavailable: 25% # max pods down during update
```

**Pros:** Zero downtime, resource-efficient. **Cons:** Mixed versions during rollout, harder to debug.

### Blue-Green Deployment

Two identical environments. Route traffic from blue (current) to green (new) atomically.

```
               ┌──────────┐
  Traffic ────►│  Router   │
               └─────┬─────┘
                ┌────┴────┐
           ┌────▼──┐  ┌──▼────┐
           │ Blue  │  │ Green │
           │(v1.0) │  │(v1.1) │
           └───────┘  └───────┘
```

**Pros:** Instant rollback (switch back to blue), full environment testing. **Cons:** 2x infrastructure cost, database schema must be forward/backward compatible.

### Canary Deployment

Route a small percentage of traffic to the new version. Gradually increase if metrics are healthy.

```
  Traffic ──► 95% ──► v1.0 (stable)
          └── 5%  ──► v1.1 (canary)
```

Typical progression: 1% → 5% → 25% → 50% → 100%, with automated metric checks at each step.

### Progressive Delivery

Combines canary with automated analysis. Argo Rollouts, Flagger, or LaunchDarkly evaluate metrics (error rate, latency, saturation) to auto-promote or rollback.

```yaml
# Argo Rollouts canary strategy
spec:
  strategy:
    canary:
      steps:
        - setWeight: 5
        - pause: { duration: 5m }
        - analysis:
            templates: [- templateName: success-rate]
        - setWeight: 25
        - pause: { duration: 10m }
        - setWeight: 75
        - pause: { duration: 10m }
```

## Rollback Patterns

| Method                      | Speed         | Risk                      |
| --------------------------- | ------------- | ------------------------- |
| Revert commit + redeploy    | Minutes       | Low — uses same pipeline  |
| Re-deploy previous artifact | Seconds       | Low — known-good image    |
| Feature flag toggle         | Instant       | None — no deployment      |
| Blue-green switch           | Seconds       | Low — previous env intact |
| Database rollback           | Minutes-hours | High — data loss risk     |

**Principle:** Deploying forward is usually preferable. `git revert` creates a new commit that undoes the change without rewriting history. Reserve `git reset --hard` + force push for emergencies only.

## Database Migrations in CI

### Forward-Only Migrations

Down migrations in production carry significant risk. If a migration is wrong, a new forward migration to fix it is generally safer.

```
migrations/
  001_create_users.sql
  002_add_email_index.sql
  003_add_phone_column.sql    # if wrong, write 004 to fix it
```

### Expand-Contract Pattern

Safe schema changes in two phases:

1. **Expand:** Add new column/table, backfill data, deploy code that writes to both old and new
2. **Contract:** After all consumers use the new schema, drop old column/table

```sql
-- Phase 1: Expand (backward compatible)
ALTER TABLE users ADD COLUMN email_v2 VARCHAR(255);
UPDATE users SET email_v2 = LOWER(email);

-- Phase 2: Contract (after all code uses email_v2)
ALTER TABLE users DROP COLUMN email;
ALTER TABLE users RENAME COLUMN email_v2 TO email;
```

### Migration Tools

| Tool           | Language | Approach                 |
| -------------- | -------- | ------------------------ |
| Flyway         | Java/JVM | Versioned SQL files      |
| Liquibase      | Java/JVM | XML/YAML/SQL changesets  |
| golang-migrate | Go       | SQL files, CLI           |
| Alembic        | Python   | Python migration scripts |
| Prisma Migrate | Node.js  | Schema-driven SQL        |
| sqitch         | Perl     | Plan-based, revertable   |

## Secret Management in CI

### Hierarchy of Approaches

1. **Native CI secrets** — GitHub Actions secrets, GitLab CI variables. Encrypted at rest, masked in logs.
2. **External vault** — HashiCorp Vault, AWS Secrets Manager, GCP Secret Manager. CI fetches at runtime.
3. **OIDC federation** — CI identity token exchanged for cloud credentials. No long-lived secrets stored.

```yaml
# GitHub Actions OIDC for AWS (no stored credentials)
permissions:
  id-token: write
  contents: read
steps:
  - uses: aws-actions/configure-aws-credentials@v4
    with:
      role-to-arn: arn:aws:iam::123456789:role/ci-role
      aws-region: us-east-1
```

**Anti-patterns:** Secrets in environment variables in Dockerfiles (`ENV`), secrets in build args visible in image layers, echoing secrets in debug mode, secrets in commit history.

## Monorepo CI

### Change Detection

Only build/test what changed. Tools: `nx affected`, `turborepo`, `bazel`, path-based filtering.

```yaml
# GitHub Actions path filtering
on:
  push:
    paths:
      - "packages/api/**"
      - "packages/shared/**" # shared dep triggers api build too
jobs:
  build-api:
    if: contains(github.event.head_commit.modified, 'packages/api')
```

### Dependency Graph Awareness

```
packages/
  shared/      ← used by api + web
  api/         ← depends on shared
  web/         ← depends on shared
  mobile/      ← independent
```

Change to `shared/` must trigger CI for `api/` and `web/` but not `mobile/`. Bazel and Nx model this via build graph; simpler setups use explicit path lists.

## Security Scanning in Pipeline

### Shift-Left Security

| Stage      | Tool Category          | Examples                   |
| ---------- | ---------------------- | -------------------------- |
| Pre-commit | Secret scanning        | gitleaks, detect-secrets   |
| PR / Build | SAST (static analysis) | Semgrep, CodeQL, SonarQube |
| PR / Build | Dependency scanning    | Dependabot, Snyk, Trivy    |
| Build      | Container scanning     | Trivy, Grype, Anchore      |
| Build      | License compliance     | FOSSA, license-checker     |
| Deploy     | DAST (dynamic)         | OWASP ZAP, Nuclei          |
| Runtime    | Runtime protection     | Falco, Sysdig              |

```yaml
# Trivy container scan in GitHub Actions
- name: Scan image
  uses: aquasecurity/trivy-action@master
  with:
    image-ref: myapp:${{ github.sha }}
    severity: CRITICAL,HIGH
    exit-code: 1 # fail the build
```

### Supply Chain Security

- **SLSA framework** — provenance attestation for build artifacts
- **Sigstore cosign** — keyless signing of container images
- **SBOM generation** — syft, cyclonedx-cli produce software bill of materials
- **Dependency pinning** — pin exact versions + lock files, verify checksums
