# Tools: CI/CD Pipeline Design

## Pipeline Architecture

A CI/CD pipeline moves code through stages: commit → build → test → artifacts → deploy → monitor. Each stage is a gate; failures block progression.

### Stages & Gates

```
Commit
  ↓
├─→ Lint & Format Check (fail fast)
├─→ Build (compile, transpile)
├─→ Unit Tests (fast, isolated)
├─→ Integration Tests (database, cache, queues)
├─→ Security Scan (SAST, dependency check)
├─→ Build Artifact (Docker image, JAR, binary)
├─→ Deploy to Staging
├─→ Smoke Tests (sanity checks)
├─→ Performance Tests (optional)
├─→ Manual Approval (optional)
└─→ Deploy to Production
     ↓
    Monitor & Rollback (if needed)
```

**Fast feedback loop** (10 min target for main stages):
- Lint/format checks: < 1 min
- Build: 2-3 min
- Unit tests: 3-5 min
- Integration tests: 2-5 min (parallelized)
- Security scan: 1-2 min

**Rationale for ordering**:
1. **Lint first**: Catches style/syntax errors before compilation (fast, cheap)
2. **Build next**: Confirms code compiles; fail early
3. **Unit tests**: Validate logic in isolation (fast, deterministic)
4. **Integration tests**: Verify component interactions (can be flaky)
5. **Security before artifact**: Scan for vulnerabilities before packaging
6. **Artifact after gates**: Only package if all checks pass
7. **Staging first**: Smoke tests before production

### Failing Builds

**Rules**:
- Broken build blocks the mainline (all pull requests)
- Developer who broke it owns responsibility to fix (not rotate to next shift)
- Fix should be submitted within 15 minutes (or revert)
- Root cause analysis: Why did the test/check miss it?

**Anti-pattern**: "We'll fix it in staging/production." Defeats the purpose of CI.

## Build Optimization

### Caching Strategies

**Dependency caching** (most impactful):
- Cache package managers (npm, pip, Maven, Go modules)
- Example: NPM cache can reduce `npm ci` from 60s → 5s
- Invalidate only when lockfile changes, not on every commit

**Build output caching**:
- Cache compiled artifacts (Java classes, Go binaries, TypeScript builds)
- Example: `.next/` cache for Next.js (90% rebuild time reduction)
- Tradeoff: Storage vs time; typically worth it

**Docker layer caching**:
- BuildKit + push remote cache: Builds on CI reuse layers from previous builds
- `--cache-from=type=registry` + `--cache-to=type=registry`
- Can reduce Docker build time by 30-50% on unchanged code

**git clone depth**:
- `git clone --depth=1` instead of full history
- Reduces initial checkout time from 30s → 5s on large repos

**Considerations**:
- Cache invalidation is hard; use content-addressed keys (lockfiles, source hash)
- Too much caching = stale state and debugging nightmares
- Profile before/after; measure actual time savings

### Parallelization

**Matrix builds** (test across versions):
```yaml
strategy:
  matrix:
    node-version: [18, 20, 22]
    os: [ubuntu-latest, macos-latest]
```
Runs N parallel jobs; overall time = max(job times), not sum.

**Test sharding** (split test suite):
```yaml
- run: npm test -- --shard=1/4  # runs 25% of tests
- run: npm test -- --shard=2/4
- run: npm test -- --shard=3/4
- run: npm test -- --shard=4/4
```
Reduces unit test phase from 10 min → 3 min (4 shards), with CI overhead.

**Independent stages**:
```
Build → [Unit Tests, Lint, Security Scan]  (all parallel)
        → Artifact
        → Integration Tests
        → Deploy
```
Three independent checks happen simultaneously.

**Tradeoff**: Resource consumption vs wall-clock time. Most CI/CD vendors charge by concurrent jobs.

## Test Strategies in CI

### Test Pyramid

```
           △
          /│\
         / │ \  E2E Tests (few, slow)
        /  │  \
       / ──┼── \
      /   │    \  Integration Tests
     / ───┼─── \
    /────│────\ Unit Tests (many, fast)
   /─────┼─────\
 Base
```

**Unit tests** (70-80% coverage):
- No external dependencies
- Mock database, queue, API calls
- Run in < 100ms each
- Goal: Test business logic in isolation

**Integration tests** (10-20%):
- Spin real database containers (Postgres, Redis)
- Test component interactions (API → DB → Cache)
- Run in < 1 second each
- Catch "works in unit tests, broke in prod" bugs

**E2E tests** (5-10%):
- Full application stack
- Verify user workflows (login → checkout → confirmation)
- Slow (10-30s per test); flaky if not careful
- Use sparingly; focus on critical paths

**Testing in CI strategy**:
- Run all unit tests on every commit (must be fast)
- Run integration tests on PR merge (acceptable: 5-10 min)
- Run E2E tests nightly or on tag (acceptable: 30+ min)
- Quarantine flaky E2E tests; don't block shipping

### Flaky Test Handling

**Prevention**:
- Avoid implicit waits (use polling with timeout)
- Don't rely on timing (`sleep(1000); assert()`)
- Use immutable test data or factories
- Run tests multiple times locally before commit

**Detection**:
- Rerun failed tests 3x; if 2/3 pass, it's flaky, not a real failure
- Track flaky tests separately; alert team
- Don't merge code that causes flaky tests

**Options**:
1. **Quarantine**: Skip flaky test until fixed; track in issue
2. **Retry**: CI re-runs flaky tests automatically; merge if eventual pass
3. **Fix**: Stabilize the test (investigate root cause)

**Production impact**: Flaky tests erode trust in CI; developers ignore alerts.

## Artifact Management

### Build Once, Deploy Everywhere

Build a single artifact (Docker image, JAR, binary) once; deploy that exact artifact to dev, staging, prod.

**Benefit**: Eliminates "works in staging, broken in prod" — same code, same dependencies.

**Implementation**:
```yaml
Build:
  - npm ci
  - npm run build
  - docker build -t myapp:$COMMIT_SHA .
  - docker push registry.example.com/myapp:$COMMIT_SHA

Deploy to Staging:
  - kubectl set image deployment/app-staging \
      app=registry.example.com/myapp:$COMMIT_SHA

Deploy to Prod:
  - kubectl set image deployment/app-prod \
      app=registry.example.com/myapp:$COMMIT_SHA
```

Tag with git commit SHA (immutable, traceable). Also tag mutable aliases (`latest`, `stable`) pointing to the same image.

### Artifact Storage

**Registry options**:
- **Docker Hub**: Free public, paid private; easy authentication
- **ECR (AWS)**: Integrated with AWS infrastructure; auto-cleanup policies
- **GCR (Google)**: Integrated with GKE; strong IAM
- **Self-hosted (Harbor, Artifactory)**: Full control; storage/maintenance burden
- **Quay.io**: Strong RBAC; strong security scanning

**Retention policies**:
- Clean old images: Keep last 10 per branch; delete after 30 days
- Reduce storage costs; speeds up image pulls
- Exception: tag `stable` or `production` (never auto-delete)

**Signing & Verification**:
- Sign images (cosign, DCT) to verify authenticity
- Enforce image signatures in Kubernetes admission controller
- Prevents supply chain attacks (compromised registry, malicious third-party)

## Deployment Strategies

### Rolling Deployment (most common)

Replace instances gradually. Old and new versions coexist temporarily.

```
Initial:  [v1] [v1] [v1]
Step 1:   [v2] [v1] [v1]
Step 2:   [v2] [v2] [v1]
Step 3:   [v2] [v2] [v2]
```

**Pros**: Simple, backwards-compatible, no spare capacity needed
**Cons**: Difficult to rollback (versions interleaved); database migration risk (old v1 can't handle new schema)

**Best for**: Compatible changes, stateless services

### Blue-Green Deployment

Two identical environments (blue and green). Cut traffic from blue → green.

```
Initial:  Blue [v1 v1 v1] ← traffic
          Green [idle]

Deploy:   Blue [v1 v1 v1]
          Green [v2 v2 v2] ← warming up

Switch:   Blue [idle]
          Green [v2 v2 v2] ← traffic

Rollback: Blue [v1 v1 v1] ← traffic (instant)
          Green [v2 v2 v2]
```

**Pros**: Instant rollback (switch traffic back); no version interleaving
**Cons**: Requires 2x infrastructure; data synchronization between environments; DNS/LB lag

**Best for**: Risk-averse teams, zero-downtime deployments

### Canary Deployment (recommended for risky changes)

Route small % of traffic to new version; monitor metrics; gradually increase if stable.

```
Initial:    [v1=95%] [v2=5%]
Monitor:    Error rate, latency, custom metrics
If OK:      [v1=90%] [v2=10%]
If OK:      [v1=70%] [v2=30%]
Final:      [v1=0%]  [v2=100%]

If error rate ↑→ Rollback: [v1=100%] [v2=0%]
```

**Pros**: Catches bugs early with minimal blast radius; gradual rollout
**Cons**: Complex to implement; requires real-time metrics; traffic duplication for logging

**Best for**: Critical changes, microservices, data schema changes

**Implementation**:
- Kubernetes: Flagger + Prometheus (automated canary with metrics feedback)
- Load balancer: Manual weight adjustment (manual canary)
- Application: Feature flags (safer, application-level control)

### Feature Flags

Deploy code behind a flag; control rollout at runtime (no redeployment).

```javascript
if (featureFlags.isEnabled('new-checkout')) {
  return newCheckoutFlow(user);
} else {
  return legacyCheckoutFlow(user);
}
```

**Pros**: Instant rollback (flip flag), A/B testing, gradual rollout, no infrastructure duplication
**Cons**: Code complexity (legacy paths), temporary feature debt

**Best for**: Experimental features, A/B tests, gradual rollouts

## Rollback Mechanisms

### Database Schema Changes

**Forward-compatible migrations**:
1. Add new column (non-required, nullable)
2. Backfill old data (if needed)
3. Update code to write to new column
4. Old code still works (reads old column or defaults)
5. Later: Remove old column

**Rationale**: New code can coexist with old code; rollback is safe.

**Anti-pattern**: Breaking migrations (drop column, rename, change type) require simultaneous code + schema update. If code update breaks, you can't rollback the schema.

### Artifact Rollback

Fastest rollback: Keep previous image tag, redeploy.

```bash
# Prod running myapp:abc123
# New deployment myapp:def456 fails
kubectl set image deployment/app app=myapp:abc123
# Instant traffic back to previous version
```

Requires:
- Immutable image tags (by commit SHA, not `latest`)
- Keep previous image in registry (retention policy)
- Monitor metrics to detect failure (automated rollback trigger)

### Monitoring for Rollback

Automated rollback triggers:
- Error rate > 5% (vs baseline)
- P99 latency > 2x baseline
- Application-level health checks fail
- SLO breach

**Implementation** (Flagger):
```yaml
apiVersion: flagger.app/v1beta1
kind: Canary
metadata:
  name: app
spec:
  targetRef:
    kind: Deployment
    name: app
  service:
    port: 80
  analysis:
    interval: 1m
    threshold: 5
    maxWeight: 50
    stepWeight: 10
    metrics:
    - name: request-success-rate
      thresholdRange:
        min: 99
      interval: 1m
    - name: request-duration
      thresholdRange:
        max: 500
      interval: 1m
```
Flagger monitors metrics; if SLI breached, rolls back automatically.

## Pipeline as Code

Version control the pipeline definition alongside application code.

**Benefits**:
- Code review for pipeline changes
- History of when/why pipeline changed
- Different pipelines per branch (experiment safely)
- Disaster recovery (rebuild pipeline from repo)

**Tools**:
- **GitHub Actions**: `.github/workflows/` (YAML)
- **GitLab CI**: `.gitlab-ci.yml` (YAML)
- **Jenkins**: `Jenkinsfile` (Groovy DSL)
- **CircleCI**: `.circleci/config.yml` (YAML)

**Anti-pattern**: UI-only pipeline configuration (no version history, no code review).

## Monorepo CI

Monorepos (one repo, multiple services/packages) need smart pipeline design.

### Change Detection

Only rebuild/test affected packages.

```bash
# Which packages changed?
git diff main...HEAD --name-only -- packages/*/src

# Rebuild only affected
npm run build --filter=...affected
npm run test --filter=...affected
```

Tools: Nx, Lerna, Turborepo handle change detection + parallelization automatically.

### Build Cache

Turborepo/Nx cache build outputs across the monorepo.

```
packages/utils:
  ./dist (cached from previous build on another developer's machine)
  Reuse instead of rebuild → 5 min saved

packages/ui:
  Depends on packages/utils → uses cached output
```

**Impact**: 50-70% pipeline time reduction for stable dependencies.

### Dependency Graph

Understand package dependencies; build in correct order + parallelize.

```
            utils
           /     \
        ui        api
           \     /
          app
```

Build order: utils → [ui, api] (parallel) → app

Turborepo/Nx do this automatically.

## See Also

devops-cicd, devops-cicd-patterns, progressive-delivery, version-control-workflows