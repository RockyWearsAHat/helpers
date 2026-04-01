# CI/CD: Pipeline Design

## Overview

A CI/CD pipeline moves code through a sequence of stages—lint, test, build, deploy—gating progress such that broken code is caught early and deployable artifacts are tested before release. Effective pipeline design balances feedback speed (developers wait for results) with correctness (catching all classes of bugs) and cost (resource consumption).

Key design axes: stage ordering, parallelization, caching, artifact flow, deployment gates, and feedback mechanisms.

## Pipeline Stages and Ordering

A typical pipeline flows: `commit → lint → test → build → artifact → deploy → monitor`.

Each stage is a gate: if a stage fails, subsequent stages are skipped. This prevents broken builds from deploying or broken deployments from reaching production.

**Lint stage**: Fast static analysis (syntax, style, type checking). Should run first and complete in seconds. Provides instant feedback without running expensive tests. Exit on first error to unblock developers quickly.

**Test stage**: Unit and integration tests. Comprehensive but slower than lint. Parallelized across test suites. Should complete in 5–10 minutes for rapid feedback.

**Build stage**: Compile, bundle, or package the application. Produces an artifact (binary, Docker image, JAR, etc.) that is deployed. Build once; reuse the artifact across environments to ensure consistency.

**Deploy stage**: Provision infrastructure, update services, smoke test. Segregated by environment (staging, production). Staging is fast and low-risk; production is gated by approval rules.

**Monitor stage**: Continuous validation post-deployment. Alerts on errors, latency, or anomalies. Feedback loop for detecting deployment issues.

## Parallelization

Sequential stages must execute in order, but independent work within a stage should parallelize.

**Within a stage**:
- Test suite split across runners: run tests in parallel on multiple machines.
- Build matrix: compile for multiple platforms (Linux, macOS, Windows) in parallel.

**Across stages**:
- Lint and build can run in parallel (independent).
- Tests must run after build (consume build artifacts).
- Deploy depends on tests passing.

**Cost vs. speed trade-off**: More parallelism = faster feedback but higher resource utilization and cost. Optimal balance depends on team size and infrastructure budget.

```yaml
# Example: lint and build in parallel; deploy after tests
jobs:
  lint:
    runs-on: ubuntu-latest
    steps: [...]
  build:
    runs-on: ubuntu-latest
    steps: [...]
  test:
    needs: build
    strategy:
      matrix:
        test-suite: [unit, integration, e2e]
    runs-on: ubuntu-latest
    steps: [...]
  deploy:
    needs: test
    runs-on: ubuntu-latest
    steps: [...]
```

## Caching Strategies

Caching reduces pipeline time by reusing dependencies and intermediate build outputs.

**Dependency caching**: Store npm, pip, Maven dependencies. Key by lock file hash. Fastest wins: cache misses are rare if dependencies don't change.

```yaml
- uses: actions/cache@v4
  with:
    path: node_modules
    key: ${{ runner.os }}-npm-${{ hashFiles('package-lock.json') }}
    restore-keys: ${{ runner.os }}-npm-
```

**Build artifact caching**: Share compiled artifacts (e.g., built JavaScript, Docker images) between jobs.

```yaml
- name: Build
  id: build
  run: npm run build
  
- uses: actions/upload-artifact@v4
  with:
    name: dist
    path: dist/
    retention-days: 1  # Short retention for temporary artifacts
```

Later jobs download:

```yaml
- uses: actions/download-artifact@v4
  with:
    name: dist
```

**Cache invalidation**: Cache keys based on lock file hashes (dependencies) or source SHA (cache-busting). Stale caches cause silently broken builds; invalidation strategies must be explicit.

**Docker layer caching**: Multi-stage Dockerfile with layer caching:

```dockerfile
FROM node:18 AS builder
COPY package*.json ./
RUN npm ci  # Layers cached if lock file unchanged

FROM node:18
COPY --from=builder /app/node_modules ./node_modules
COPY . .
RUN npm run build
```

## Artifact Management

Build artifacts (application binaries, Docker images, compiled code) are the "currency" of the pipeline. Artifacts should be versioned and traceable.

**Versioning**: Tag artifacts per build. Use commit SHA for immutability:

```bash
docker build -t myapp:${{ github.sha }} .
docker push registry/myapp:${{ github.sha }}
```

Or semantic versioning for releases:

```bash
docker build -t myapp:1.2.3 .
docker push registry/myapp:1.2.3
```

**Artifact storage**: Store in a registry (Docker Hub, ECR, GCR). Central registry enables all environments to pull the same artifact, ensuring consistency.

**Cleanup**: Old artifacts accumulate. Define retention policies. For staging/CI artifacts, short retention (days); for release artifacts, long retention (years).

Example GitHub Actions retention:

```yaml
- uses: actions/upload-artifact@v4
  with:
    name: build
    path: build/
    retention-days: 7  # Staging artifacts only needed for a week
```

## Deployment Approvals and Gates

Production deployments should require explicit human approval to enforce caution.

**GitHub Environments** example:

```yaml
deploy-prod:
  environment: production
  runs-on: ubuntu-latest
  steps:
    - run: deploy.sh
```

Configure the environment in repository settings to require approval. The job pauses; reviewers approve from the Actions tab.

**Deployment gates**: Before deploying, verify:

- All tests passed.
- Code is reviewed and merged.
- Deployment window is open (no weekend/holiday deployments).
- Feature flags are staged (allow gradual rollout).

## Environment Promotion

Promote builds through environments (dev → staging → production) to increase confidence.

```yaml
deploy-staging:
  needs: test
  environment: staging
  runs-on: ubuntu-latest
steps: [...]

deploy-prod:
  needs: deploy-staging
  environment: production
  runs-on: ubuntu-latest
  steps: [...]
```

Test in staging fully before promoting to production. Staging should mirror production (infrastructure, data scale, config). Bugs found in staging are free; bugs found in production are expensive.

## Database Migrations in CI

Migrations introduce risk: a botched migration can corrupt data or cause downtime. CI must handle them carefully.

**Separate migration from code deployment**: Run migrations before or after code is deployed, depending on backwards compatibility.

```yaml
- name: Run migrations
  run: |
    npm run migrate:up

- name: Deploy code
  run: |
    ./scripts/deploy.sh
```

**Backwards-compatible migrations**: Migrations must work with both old and new code. Example: add a new optional column without changing the schema version until code can handle it.

**Automated migration verification**: Test migrations on a copy of production data (sanitized) to catch issues before production.

```bash
# Dump production DB, sanitize, restore to staging
pg_dump --host=prod-db > prod-dump.sql
./scripts/sanitize-dump.sh prod-dump.sql > sanitized.sql
psql --host=staging-db < sanitized.sql

# Run migrations
npm run migrate:up --database staging
```

**Rollback capability**: Migrations must be reversible. Test rollbacks.

```bash
npm run migrate:up
npm run migrate:down  # Should return to prior state
```

## Feature Branch Deployments

Enable deployments from feature branches for testing, review, or demo purposes without risking main.

```yaml
deploy-staging:
  needs: test
  if: always()  # Deploy even if tests fail (demo purposes)
  environment:
    name: staging-${{ github.ref_name }}
  runs-on: ubuntu-latest
  steps:
    - run: deploy.sh --environment=staging-${{ github.ref_name }}
```

This creates per-branch staging environments. Useful for:

- Demonstrating features to stakeholders.
- Performance testing under realistic load.
- Collaborative testing across teams.

Per-branch environments incur infrastructure cost. Clean up old branch deployments.

## Flaky Test Management

Tests that fail intermittently are cancer; they erode confidence. Identify and fix flakes.

**Detection**: Run tests multiple times in CI. Flakes are caught by variance (test passes/fails on reruns).

```yaml
- name: Run tests with retries
  run: npm test -- --bail=false --retries=2
```

Tests passed twice should almost never be flaky. Persistent flakes indicate real bugs (race conditions, timing issues, missing waits).

**Quarantine**: Disable flaky tests temporarily and file a bug. Re-enable once fixed. Prevents flaky tests from blocking merges.

```javascript
it.skip('flaky test - TODO fix timing issue #1234', () => { ... });
```

**Root cause**: Flakes usually stem from:
- Timing assumptions (test assumes action completes in X ms).
- Non-deterministic test data.
- Shared state between tests.
- Mock/stub edge cases.

## Performance and Feedback Speed

Fast feedback = faster iteration. Slow pipelines are demoralizing and waste developer time.

**Target**: Pipelines should complete in < 10 minutes. Lint + test in < 5 min. Build in < 2 min.

**Optimizations**:

- Parallelize ruthlessly (test matrix, multiple agents).
- Cache aggressively (dependencies, build artifacts).
- Skip unnecessary steps (e.g., don't deploy if only docs changed).
- Fail fast (exit on first error, don't wait for independent jobs).

```yaml
# Skip deploy if only docs changed
if: |
  github.event_name == 'push' && 
  !contains(github.event.head_commit.modified, 'docs/**')
```

- Use fast languages for lint (Go, Rust faster than Python/Ruby for large codebases).
- Profile the pipeline itself. The slowest stage is the bottleneck.

## See Also

- [tools-ci-cd-pipeline-design.md](tools-ci-cd-pipeline-design.md) — Pipeline architecture and tooling survey
- [devops-cicd.md](devops-cicd.md) — CI/CD principles and continuous integration philosophy
- [devops-cicd-patterns.md](devops-cicd-patterns.md) — Deployment patterns (trunk-based, GitOps, blue-green)
- [devops-database-migrations.md](devops-database-migrations.md) — Database migration strategies in depth
- [ci-github-actions-patterns.md](ci-github-actions-patterns.md) — GitHub Actions implementation specifics