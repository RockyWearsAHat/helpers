# Process: Release Management — Release Trains, Versioning, and Staged Rollouts

## The Release as a Formal Process

A release is the act of making a version of software available to users. Release management is the discipline of coordinating changes, validating readiness, and controlling when and how software reaches production. Unlike continuous deployment (every commit to main is production-ready), releases impose a release cycle: a window during which changes are frozen, tested, and released together.

## Semantic Versioning: Public Contracts

Semantic versioning (semver) is a scheme for communicating breaking changes to consumers:

```
MAJOR.MINOR.PATCH
  v1.3.2
```

- **MAJOR** increments on backward-incompatible changes. Consumers expect breaking changes; they must update and test before adopting.
- **MINOR** increments on new features with backward compatibility. Consumers can safely upgrade.
- **PATCH** increments on bug fixes. Pure safety upgrades.

**Implications:**

- A library that doesn't honor semver creates invisible risk: a "patch" bump that silently breaks consumers.
- Tools like Renovate and Dependabot parse semver to make intelligent update decisions: they'll auto-merge patch and minor updates (low risk) but flag majors (breaking).
- Public APIs should semver; internal implementation details need not. A function that's `@internal` can change freely.

**Pre-release versions** (e.g., `1.0.0-beta.1`, `v2.0.0-rc.2`) indicate published but not-yet-stable code. Consumers know risks exist.

## Release Trains and Cadences

A **release train** is a time-boxed release window: commits made before a cut-off reach this release; commits after miss it and go to the next train.

### Fixed-Schedule Model

Releases ship on a predictable calendar: the third Tuesday of each month, every two weeks, or on a sprint boundary.

**Pros:**
- Predictable for communications, testing, and support planning
- Creates natural batching of features (prevents constant trickle)
- Aligns with organizational ceremonies (sprint end)
- Teams know their release window weeks/months in advance

**Cons:**
- Hotfixes miss the train; they require emergency processes
- If a critical bug is found the day before release, waiting feels wrong
- Teams feel pressured to ship incomplete features to meet the train

**Common cadences:**
- Enterprise software: quarterly major releases; monthly security patches
- SaaS: weekly (e.g., GitHub), biweekly (e.g., Slack), continuous-but-batched
- Mobile apps: monthly or quarterly (app store review latency is a factor)

### Ad-Hoc / On-Demand Model

Releases happen whenever a feature is ready, or after a fixed set of features accumulate.

**Pros:**
- No artificial delays; features ship when done
- Each release is smaller, easier to reason about
- Fewer incentives to ship incomplete work

**Cons:**
- Hard to predict; impossible to plan testing or communication windows
- Release fatigue: teams burned out from constant releases
- Support struggles with "which version has this bug fix?" across seven releases in two weeks

**Hybrid approach:** Most teams run fixed trains (weekly/biweekly) but allow hotfix trains for critical issues outside the normal cycle.

## Feature Freeze and Release Candidates

### Feature Freeze

A release branch is cut; new features are forbidden. Only bug fixes, performance tuning, and documentation land. The goal: stabilize the release for testing and production use.

**When to freeze:** By convention, often 1-2 weeks before the scheduled release date. Features nearly done but not quite are held back to the next release.

**Branches during freeze:**
- `main`: continues accepting features for the next release
- `release/1.3.x`: bugfix-only; fixes from this branch are backported to main
- Developers planning features must complete and land them before freeze, or they miss the release

### Release Candidates

A release candidate (RC) is a pre-release: it's feature-complete and **intended to be the final release, but testing may reveal blockers**.

```
v1.3.0-rc.1  → Found critical bug  → v1.3.0-rc.2  → Approved  → v1.3.0 (final)
```

RCs allow:
- QA to perform final round testing on a known version
- Users to opt-in to testing the release before it ships
- A window to discover breaking changes or regressions without the pressure of already-announced release dates

Some teams ship RCs to production with feature flagging disabled, treating them as a production-like test environment.

## Changelog Automation

Changelogs communicate what changed to users, support, and operators. **Manual changelogs are unreliable:** entries are forgotten, versioning is inconsistent, and the changelog drifts from actual changes.

**Automated changelogs** parse commit messages or generate them from merged PRs:

```
# Tools: conventional-changelog, Release-Please (Google), Semantic Release

# From conventional commits:
git log v1.2.0..main | grep "^feat:" | convert to changelog

# Generate:
## [1.3.0] - 2025-03-25

### Added
 - New `--verbose` flag for API queries (#438)
 - Support for regional endpoints (#445)

### Fixed
 - Corrected timezone handling in date parsing (#442)

### Deprecated
 - `oldFunction()` will be removed in 2.0.0 (#441)
```

**Best practice:**

- Use [Conventional Commits](http://conventionalcommits.org): `feat:`, `fix:`, `breaking:` prefixes in commit messages
- Tool parses commits → generates semver bump + changelog
- Automate via CI: on merge to main, release tool bumps version, generates changelog, commits, and tags
- If version is already bumped manually (e.g., v1.3.0.txt), release tool detects it and uses that

## Release Branches and Hotfixes

### Long-Lived Release Branches

```
main ───●──●──●──●──●─────●──●──●───  (current development)
         ↓
release/1.3 ───●──●──●──●─────  (v1.3.0, v1.3.1, v1.3.2)
             ↓
        v1.3.1 (hotfix applied)
```

Each major or minor release gets a **release branch** (`release/1.3.x`, `release/1.2.x`). When a critical bug is found in production:

1. Fix is applied to the release branch
2. Version is bumped (1.3.0 → 1.3.1 patch)
3. Tag is created; build triggered
4. **Fix is backported to main** to prevent regression

This approach keeps old versions receiving bug fixes for an extended period (6 months to 2+ years, depending on support windows).

### Hotfix Workflow

Some teams forgo release branches for simpler hotfix handling:

1. Bug discovered in v1.3.0 (live in production)
2. Create branch `hotfix/1.3.1` from the v1.3.0 tag
3. Apply fix; test
4. Merge to prod branch; tag v1.3.1
5. Backport fix to main

## Staged Rollouts and Risk Mitigation

Not all production deployments happen at once. Staged rollouts reduce blast radius:

### Canary Deployment

Deploy to a small fraction of production (2–5% of traffic). Monitor for errors, latency, or anomalies. If healthy, gradually increase to 10%, 25%, 100%. If issues detected, roll back the canary.

**Tools:** Istio, Flagger, AWS CloudFormation, Kubernetes canary strategies.

### Blue-Green Deployment

Two **identical target environments**: Blue (current production) and Green (new version). Traffic switches at the load balancer. Rollback is a single switch back to Blue.

```
Traffic → Load Balancer → Blue (1.2.0) [production]
                        → Green (1.3.0) [staged, tested]

After validation: Traffic → Green (1.3.0)  [production]
                        → Blue (1.2.0)   [now for next release]
```

### Feature Flags

Ship code to production that's **inactive by default**. Gradually enable for users, regions, or experiments:

```javascript
if (featureFlag.isEnabled('new-checkout-flow')) {
  return newCheckout(req);
} else {
  return classicCheckout(req); // fallback
}
```

Decouples deployment (code shipped) from release (feature turned on). Allows instant rollback without redeployment.

## Release Retrospectives

After every release, spend 15–30 minutes discussing:

- What went smoothly?
- What was painful (communication delays, test failures, deploy timing)?
- What do we change for the next release?

Document action items. Most teams find recurring pain points: "We always scramble for changelog updates" or "We need an earlier feature freeze date." Adjust the process incrementally.

---

## See Also

- [CI/CD Patterns](devops-cicd-patterns.md) — branching strategies and automation
- [API Versioning](api-versioning.md) — evolution strategies across API boundaries
- [Progressive Delivery](progressive-delivery.md) — techniques for safe production rollouts
- [SRE Incident Management](sre-incident-management.md) — handling incidents during/after releases