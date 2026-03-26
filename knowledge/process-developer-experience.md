# Developer Experience — Inner Loop, Portals & Cognitive Load

## What is Developer Experience (DevEx)?

DevEx is the friction developers experience when doing their jobs. It spans:

- **Inner loop** (write → test → debug locally, 1-10 minute cycles)
- **Outer loop** (push → CI → staging → production, 1-hour+ cycles)
- **Cognitive overhead** (how many systems must I understand to make one change?)
- **Documentation quality** (can I understand a system from existing docs, or do I ask humans?)
- **Onboarding speed** (how long until a new engineer is independent?)

DevEx is often overshadowed by deployment, infrastructure, or monitoring improvements—but it compounds. A developer waiting 5 minutes per build, 8 times a day, across 100 developers, is **1000 developer-hours wasted per week**.

---

## The Inner Loop: Build Time and Test Feedback

### The Cost of Slow Builds

Build time has non-linear costs:

- **0-10 seconds**: Acceptable friction. Developer stays in flow state.
- **10-60 seconds**: Developer context-switches (checks Slack, email, browser)
- **1-5 minutes**: Developer leaves workstation (coffee, conversation)
- **5+ minutes**: Real work gets abandoned; developer checks out mentally

A team of 50 developers on a codebase with a **3-minute build time** loses:
```
50 developers × 10 builds/day × 3 min wastage × 200 work days/year
= 50,000 developer-hours/year wasted on waiting
```

At $150/hour loaded cost: **$7.5M annually in unproductive waiting**.

### Optimization Strategies

**1. Parallelize compilation**
- Split build into stages (compile unit → link → integration tests)
- Run these in parallel on developer machines
- Typical gain: 2-4x speedup with marginal investment

**2. Incremental builds**
- Only recompile changed files and their dependents
- Tools: Gradle Build Cache, Bazel, Turborepo, esbuild
- Gain: 50-300x speedup for small changes; 2-5x for larger changes

**3. Test sharding**
- Run unit tests in parallel; skip tests not affected by the change
- Example: Modified `auth-service`? Don't run e2e tests for `payments-service`
- Gain: 3-10x speedup depending on test independence

**4. Run only affected tests**
- Dependency graph your tests to the code they exercise
- Example: Monorepo with `services/auth`, `services/payments`, `libs/core`
  - Change in `libs/core` → run tests for `auth` and `payments`
  - Change in `services/auth` → run tests for `auth` only
- Tools: Nx, Turborepo, Buck2

**5. Local development modes**
- Offer a "dev mode" with skip rules: skip E2E, skip slow integration tests, no optimization
- Example: `npm run build:dev` (30s) vs `npm run build:prod` (5 min)
- Trade: Fast feedback loop locally, comprehensive checks in CI

### Test Feedback Structure

Developers need feedback at three layers:

```
Layer 1 (0-10s feedback): Fast unit tests, linting, type checking
  ├─ Run every file save (if <1s per change)
  ├─ Fail fast if possible (abort on first 3 failures)
  └─ Use IDE feedback, not just CLI

Layer 2 (30-60s feedback): Integration tests, API contract checks
  ├─ Run before local commit (git pre-commit hook)
  ├─ Must be faster than pushing to CI (developers lose faith quickly)
  └─ Can be filtered by change scope

Layer 3 (5-10 min): Full regression, E2E tests, security scans
  ├─ Run asynchronously in CI after push
  ├─ Don't block developer iteration (test in background)
  └─ Fail asynchronously; developer alerted via notification
```

Bad structure: **single 5-minute test suite that runs on every save**.
Good structure: **fast, scoped feedback loops at each stage**.

---

## Developer Portals: Backstage, Port, and Golden Paths

### The Problem: Cognitive Overload

A backend developer needs to understand:
- 20+ internal microservices
- 3 deployment targets (dev, staging, prod)
- 5 different CI/CD pipeline styles (Kubernetes, Lambda, Batch, Docker, bare metal)
- 2 monitoring stacks (Prometheus, Datadog)
- Custom secret management, custom logging setup, custom health checks, custom canary deployments

Each system has different CLI commands, configuration formats, and failure modes. **The cognitive load is unbearable,** especially for junior engineers.

### Developer Portals as Solutions

A developer portal (Backstage, Port, or custom) provides:

1. **Service catalog** — Discover all services: owner, deployment target, monitoring dashboard, runbook links, on-call rotation
2. **Golden paths** — Templates and automation for common tasks:
   - "Create a new microservice" (scaffolds, configures, points to docs)
   - "Add a database to my service" (spins up Postgres, applies migrations, grants permissions)
   - "Deploy to production" (one-click deployment, with safety guardrails)
3. **Self-service infrastructure** — Provisioning without ops tickets:
   - "I need a Redis cluster" → portal handles provisioning, scaling, backups
   - "I need a new environment" → portal automates EC2, networking, IAM
4. **Single pane of glass** — Developers see their services, dependencies, incidents, and runbooks in one place

### Examples

**Backstage (Spotify-backed, open source):**
- Component registry with owner metadata
- Template system for scaffolding services
- TechDocs integration for runbooks
- Custom plugins for internal tools
- Workflow orchestration for deployment

**Port (commercial):**
- UI-first service catalog
- Automation via UI, no coding
- Integration with existing tools (GitHub, Slack, PagerDuty, Datadog)
- Lightweight relative to Backstage

**Custom portals:**
- Smaller teams sometimes build a tailored UI for deployment, service discovery, and runbook links
- Risk: maintenance burden; benefit: tight alignment with team workflows

### Golden Paths

A "golden path" is the **recommended way to do a common task**, with automation:

```
Developer goal: "Deploy my service to production"

Without golden path:
  1. Understand our CI triggers
  2. Push to branch, wait for tests
  3. Create PR, wait for review
  4. Merge PR, wait for builds
  5. SSH into prod box (how? memory password?)
  6. Git pull, systemctl restart
  7. Check logs (hope monitoring alerts if broken)

With golden path:
  1. Run: `portal deploy --service=my-service --target=prod`
  2. Portal checks: tests pass? monitoring webhook ready? DB backups current?
  3. Portal executes deployment with canary (5% traffic), monitors p99 latency
  4. If p99 spikes, auto-rollback; notify team
  5. After 2 hours stable, roll out 100%
  6. Slack notification: "Deployed successfully"
```

Without golden paths, each developer invents their own process, or copies an outdated process, or guesses incorrectly.

---

## Cognitive Load Reduction

### The Irreducible Complexity

Some complexity is inherent (Kubernetes is complex; event-driven systems are complex). But **accidental complexity** — confusion from poor tool choices, unclear documentation, or fragmented information — is fixable.

### Patterns for Reduction

1. **Conventions over configuration** — All services follow the same deployment pattern (Helm chart template, CloudFormation, Terraform module). New service automatically inherits logging, monitoring, alerting. Developers make exceptions only for good reason.

2. **Local dev environments that mirror production** — Docker Compose or `tilt up` spins up every dependency locally. Developers debug issues locally, not on prod. Gain: faster iteration, safer experiments.

3. **Explicit boundaries** — Clear ownership of each system. Service deployed by team A, monitored by team A, on-called by team A. Engineers not on team A know they go to team A for questions.

4. **Runbooks linked at the point of pain** — When a Datadog alert fires, it links to a runbook. When an error appears in logs, it links to a GitHub issue. Documentation isn't "somewhere in Notion"; it's one click from the system causing confusion.

5. **Async-first documentation** — Answers must be written down (not in Slack). Junior engineers searching "how to deploy" should find docs, not outdated Slack threads. Tools: Markdown in repo, wiki, internal blog.

---

## Metrics: DORA Metrics and Onboarding Speed

### DORA Metrics (Deployment Frequency, Lead Time, MTTR, Change Failure Rate)

**Deployment Frequency:** How often you ship  
- Good: Daily to weekly
- Bad: Quarterly or manual

**Lead Time for Changes:** Idea to production  
- Good: <1 hour
- Bad: >1 month

**Mean Time to Recovery (MTTR):** Time from incident detection to resolution  
- Good: <15 minutes
- Bad: >2 hours

**Change Failure Rate:** Fraction of changes causing issues  
- Good: <15%
- Bad: >50%

These aren't measures of DevEx directly, but they **correlate** with it. Poor DevEx (slow builds, unclear deployments, high cognitive load) leads to worse DORA metrics.

### Onboarding Speed Metrics

- **Time to first PR:** How long before new hire makes a meaningful code change? (Should be <1 week)
- **Time to first production deploy:** When do they touch production? (Should be <2 weeks)
- **Survival rate:** Do they still work here after 6 months? (Should be >90%; lower indicates poor onboarding)
- **Ramp-up productivity:** Velocity at month 1 vs. month 3 vs. month 6 (should double every 3 months)

---

## See Also

- [Platform Engineering — Internal Developer Platforms & Self-Service Infrastructure](platform-engineering.md) — Platform teams building for DevEx
- [Developer Onboarding & Knowledge Transfer](process-developer-onboarding.md) — Onboarding as DevEx
- [Process: Code Ownership](process-code-ownership.md) — Clarity and responsibility reduce cognitive load
- [SRE: On-Call](sre-on-call.md) — Developer well-being and on-call experience