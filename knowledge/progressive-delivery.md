# Progressive Delivery — Canary, Blue-Green, Feature Flags & Controlled Rollout

Progressive delivery extends continuous delivery by introducing mechanisms to release
changes to production incrementally — controlling who sees what, when, and rolling back
quickly when signals indicate problems. Where continuous delivery ensures code is always
deployable, progressive delivery ensures deployments are always controllable.

## The Core Idea: Separating Deployment from Release

Traditional deployment models treat "code is deployed" and "users experience the change"
as the same event. Progressive delivery decouples these:

| Concept    | Meaning                                           |
| ---------- | ------------------------------------------------- |
| Deployment | Code is placed into the production environment    |
| Release    | The change becomes visible to users               |
| Rollout    | The gradual process of expanding release exposure |

This separation enables deploying code that is invisible to end users (dark launches),
releasing to a subset of users first (canary), or toggling features on and off without
redeployment (feature flags). The deployment becomes a non-event; the release becomes the
controlled, observable process.

## Feature Flags

Feature flags (also called feature toggles, feature switches, or feature gates) are
conditional logic that controls whether a code path executes for a given request.

### Flag Types

| Type            | Purpose                           | Typical lifetime | Example                           |
| --------------- | --------------------------------- | ---------------- | --------------------------------- |
| Release flag    | Gate incomplete or risky features | Days to weeks    | New checkout flow                 |
| Experiment flag | A/B testing and data collection   | Weeks to months  | Recommendation algorithm variant  |
| Ops flag        | Runtime operational control       | Indefinite       | Circuit breaker, maintenance mode |
| Permission flag | Entitlement or access control     | Indefinite       | Premium feature access            |

### Flag Lifecycle

The lifecycle of a feature flag is often where operational discipline breaks down:

1. **Creation** — flag is defined with a default state (typically off), targeting rules,
   and metadata (owner, purpose, expiration date)
2. **Development** — code paths are wrapped in flag checks; both paths must be tested
3. **Staged rollout** — flag is enabled for internal users, then beta, then percentage
   rollout, then full release
4. **Stabilization** — once the feature is fully released and stable, the flag becomes
   a candidate for removal
5. **Cleanup** — the flag conditional is removed from code, the old code path is deleted,
   and the flag definition is retired

### The Technical Debt Problem

Feature flags that are never cleaned up accumulate as technical debt:

- **Code complexity** — every flag doubles the possible code paths; N flags create up to
  2^N theoretical combinations
- **Testing burden** — flag combinations may need to be tested together, and stale flags
  make the matrix unwieldy
- **Cognitive load** — developers must understand which flags are active, which are stale,
  and what each flag's current state means
- **Dead code** — the "off" path of a fully-rolled-out flag is dead code that still gets
  compiled, reviewed, and maintained

Approaches to managing flag debt:

- Mandatory expiration dates on release flags — alerts fire when flags outlive their
  intended lifespan
- Automated detection of flags that have been 100% enabled for extended periods
- Regular "flag cleanup sprints" as part of engineering hygiene
- Ownership tracking — every flag has an accountable team

### Flag Evaluation: Where and How

| Evaluation model | How it works                                             | Trade-offs                                                |
| ---------------- | -------------------------------------------------------- | --------------------------------------------------------- |
| Server-side      | Flag evaluated on each request by application server     | Full control, easy targeting, adds latency per evaluation |
| Client-side      | Flag state fetched at initialization, evaluated locally  | Fast evaluation, requires SDK, may serve stale state      |
| Edge evaluation  | Flag evaluated at CDN/proxy layer before reaching app    | Lowest latency, limited targeting complexity              |
| Hybrid           | Initial state from edge/cache, complex rules server-side | Balanced; architectural complexity                        |

**Targeting dimensions** commonly supported:

- User identity (specific users, user segments)
- Geographic location
- Device type, OS, browser
- Percentage-based random sampling
- Organization or account membership
- Custom attributes from user context

### Configuration Management

Flags exist within a configuration system that must handle:

- **Environments** — a flag may be enabled in staging but disabled in production
- **Segments** — named groups of users (e.g., "beta testers", "enterprise customers")
- **Rules and precedence** — when multiple rules match, which takes priority
- **Audit logging** — who changed what flag, when, and why
- **Change propagation** — how quickly flag changes take effect across running instances

## Canary Deployments

A canary deployment routes a small percentage of production traffic to a new version
while the majority continues hitting the current version.

### The Process

1. Deploy the new version alongside the current version
2. Route a small fraction of traffic (often 1-5%) to the new version
3. Monitor key metrics: error rates, latency, resource consumption, business KPIs
4. If metrics are healthy, gradually increase the canary's traffic share
5. If metrics degrade, route all traffic back to the current version
6. Once the canary reaches 100%, decommission the old version

### Traffic Routing Mechanisms

- **Load balancer weighting** — configure upstream weights to split traffic
- **Service mesh routing rules** — fine-grained traffic policies at the mesh layer
- **DNS-based splitting** — weighted DNS records (coarser-grained, slower to change)
- **Application-level routing** — request middleware that routes based on headers or
  attributes

### Canary Analysis

The decision to advance or roll back a canary can be:

| Approach               | Description                                                 | Trade-offs                                 |
| ---------------------- | ----------------------------------------------------------- | ------------------------------------------ |
| Manual                 | Engineer watches dashboards and decides                     | Flexible; slow, error-prone, doesn't scale |
| Threshold-based        | Automated checks against predefined metric thresholds       | Fast; requires well-calibrated thresholds  |
| Statistical comparison | Canary metrics compared to baseline using statistical tests | Rigorous; more complex to implement        |
| ML-based               | Anomaly detection models flag deviations                    | Adaptive; opaque, requires training data   |

Key signals for canary evaluation:

- Request error rate (5xx responses, exceptions)
- Latency distribution (p50, p95, p99)
- Resource utilization (CPU, memory, connection pools)
- Business metrics (conversion rate, cart abandonment, API call patterns)
- Downstream dependency health (increased failures in called services)

## Blue-Green Deployments

Blue-green deployment maintains two identical production environments:

| Aspect | Blue (current)                | Green (new)              |
| ------ | ----------------------------- | ------------------------ |
| State  | Serving all traffic           | Deployed, tested, idle   |
| Action | Will become idle after switch | Will receive all traffic |

### The Process

1. The "blue" environment serves production traffic
2. Deploy the new version to the "green" environment
3. Run smoke tests and validation against green
4. Switch the router/load balancer to direct traffic to green
5. Green is now live; blue is idle but available for instant rollback
6. After confidence period, blue can be updated or decommissioned

### Trade-offs

**Advantages:**

- Instant rollback — switch back to the other environment
- Full environment testing before traffic hits it
- Zero-downtime deployment (if switching is fast enough)

**Challenges:**

- Double the infrastructure cost during deployment windows
- Database schema changes require careful handling — both environments may read/write
  the same database
- Long-running sessions or connections may be disrupted during the switch
- State synchronization between environments (caches, in-memory data)

## A/B Testing and Experimentation

A/B testing overlaps with progressive delivery but serves a different primary goal:
progressive delivery manages risk, while A/B testing measures impact.

| Dimension         | Progressive delivery            | A/B testing                            |
| ----------------- | ------------------------------- | -------------------------------------- |
| Primary goal      | Safe rollout, risk reduction    | Measure causal impact of a change      |
| Traffic split     | Temporary, trending toward 100% | Sustained for statistical significance |
| Decision criteria | Operational health metrics      | Business/product metrics               |
| Duration          | As short as possible            | As long as needed for significance     |
| Control group     | Eventually eliminated           | Maintained throughout experiment       |

In practice, the infrastructure is often shared — the same feature flag and traffic
routing systems support both progressive rollout and experimentation.

## Traffic Splitting and Weighted Routing

The mechanical layer that enables progressive delivery patterns:

### Splitting Strategies

- **Random percentage** — each request has an N% chance of hitting the new version;
  statistically representative but individual users may flip between versions
- **Sticky sessions** — once a user is assigned to a version, they stay there for the
  session or a defined period; consistent experience but complicates analysis
- **Hash-based assignment** — deterministic assignment based on a hash of user ID or
  session ID; reproducible, consistent, and analytically clean
- **Geographic** — route by region; useful for locale-specific changes but introduces
  geographic confounds
- **Header-based** — route based on request headers; useful for internal testing and
  partner-specific rollouts

### Gradual Ramp Schedules

Common rollout patterns:

```
Day 1:  1% (smoke test)
Day 2:  5% (early signal)
Day 3:  25% (meaningful sample)
Day 4:  50% (half traffic)
Day 5:  100% (full rollout)
```

The specific schedule depends on traffic volume (low-traffic services need longer at
each stage for statistical confidence), risk tolerance, and the nature of the change.

## Rollback Strategies

When a progressive rollout goes wrong, the response depends on the deployment model:

| Strategy             | Mechanism                         | Speed    | Considerations                              |
| -------------------- | --------------------------------- | -------- | ------------------------------------------- |
| Traffic shift        | Route traffic back to old version | Seconds  | Requires old version to still be running    |
| Feature flag disable | Toggle flag off                   | Seconds  | Only works for flag-gated changes           |
| Redeployment         | Deploy the previous version       | Minutes  | Requires CI/CD pipeline execution           |
| Database rollback    | Reverse schema or data changes    | Variable | Often the hardest part; may not be possible |

**Instant rollback** (traffic shift, flag toggle) is possible only when:

- The old version is still running and healthy
- No irreversible state changes have occurred (schema migrations, data transformations)
- The change is purely additive or behind a flag

**Gradual reverse rollout** — reducing the new version's traffic percentage over time
rather than instantly — can be appropriate when the issue is performance-related rather
than correctness-related, allowing observation of whether the reversal resolves the
degradation.

## The Observability Requirement

Progressive delivery is only as effective as the signals available to make
advance/rollback decisions:

**What must be observable:**

- Per-version metrics — error rates, latency, and resource usage split by deployment
  version, not aggregated
- Business metrics — conversion funnels, API success rates, user engagement, segmented
  by version
- Dependency health — whether the new version is causing downstream problems
- Anomaly detection — automated identification of metric deviations from baseline

**Without adequate observability:**

- Canary deployments become "deploy and hope" — the canary runs but nobody watches
- Rollback decisions are based on user complaints rather than metrics — reactive, not
  proactive
- Subtle regressions (p99 latency increase, minor error rate bump) go unnoticed
  until they compound

The observability investment typically precedes or accompanies progressive delivery
adoption; attempting progressive delivery without adequate signals yields limited benefit.

## Dark Launches

Deploying code that executes in production but whose results are invisible to users:

**Approaches:**

- **Shadow traffic** — new version receives a copy of production traffic; responses are
  compared but only the old version's responses are served to users
- **Silent execution** — new code path runs alongside the old one; results are logged
  but not returned
- **Write-path shadowing** — new version processes writes in parallel; results stored
  separately for comparison

**Use cases:**

- Validating performance characteristics under real production load
- Comparing output of a new algorithm against the existing one
- Load-testing a new service before it receives real traffic
- Building confidence in a major refactor before switching

**Caveats:**

- Side effects must be carefully managed — the dark code should not send emails, charge
  credit cards, or mutate production state
- Resource consumption increases — running two code paths consumes more compute
- Data consistency — if the dark path writes to a shadow store, that store must be
  managed and eventually cleaned up

## Ring-Based Rollout

A staged rollout model that progresses through concentric rings of increasing exposure:

| Ring   | Audience                    | Purpose                               |
| ------ | --------------------------- | ------------------------------------- |
| Ring 0 | Internal team / developers  | Dogfooding, catch obvious issues      |
| Ring 1 | Internal organization       | Broader internal validation           |
| Ring 2 | Beta / early adopter users  | Real-world signal from tolerant users |
| Ring 3 | Percentage of general users | Scaled validation with real traffic   |
| Ring 4 | All users                   | Full release                          |

Each ring transition requires explicit criteria being met — typically a combination of
metric thresholds, time-in-ring minimums, and absence of blocking issues.

Ring-based rollout is common in large-scale consumer software and enterprise platforms
where the user base is vast and diverse enough that internal testing alone cannot
surface all issues.

## Kill Switches

Emergency mechanisms for instantly disabling a feature or rolling back a deployment:

**Characteristics of effective kill switches:**

- Operable by on-call engineers without deep deployment knowledge
- Take effect within seconds, not minutes
- Do not require a deployment pipeline to execute
- Are tested regularly — an untested kill switch may not work when needed
- Have clear documentation on when and how to use them

**Implementation patterns:**

- Feature flag set to "off" with override priority above all targeting rules
- Load balancer configuration that shifts 100% traffic to the known-good version
- DNS failover to a static fallback or previous deployment
- Circuit breaker that short-circuits the problematic code path

Kill switches are distinct from rollback — a kill switch stops the bleeding immediately;
rollback restores the previous known-good state. Both may be needed in sequence.

## Organizational Discipline

Progressive delivery introduces operational practices that require sustained discipline:

### Flag Hygiene

- Every flag has an owner, a purpose, and an intended lifetime
- Stale flags are identified and removed proactively, not reactively
- Flag changes are code-reviewed and auditable
- Flag naming conventions prevent collisions and improve discoverability

### Testing Flag Combinations

- Critical paths are tested with various flag states, not just the happy path
- Default-off flags are tested in both states before rollout begins
- Integration test suites can be parameterized by flag configuration
- Flag-dependent behavior is documented so QA understands the test matrix

### Rollout Coordination

- Changes that span multiple services may require coordinated rollout sequences
- Backward compatibility is maintained during the rollout window — not all instances
  flip simultaneously
- Communication channels exist for announcing rollouts, flagging issues, and
  coordinating rollbacks

### Incident Response

- Runbooks include flag states and rollback procedures for progressive delivery
- On-call engineers know how to operate kill switches and traffic controls
- Post-incident reviews examine whether progressive delivery signals were adequate
  and whether rollback was fast enough

## Tensions and Trade-offs

Progressive delivery is not universally beneficial; it introduces its own complexity:

| Benefit                                 | Corresponding cost                            |
| --------------------------------------- | --------------------------------------------- |
| Reduced blast radius of bad deployments | Increased infrastructure complexity           |
| Data-driven release decisions           | Observability investment and maintenance      |
| Feature flag flexibility                | Flag technical debt and combinatorial testing |
| Instant rollback capability             | Running multiple versions simultaneously      |
| Experimentation culture                 | Statistical literacy requirements             |
| Controlled exposure                     | Potential for inconsistent user experiences   |

The appropriate level of progressive delivery sophistication depends on the risk
profile of the application, the volume of deployments, the size of the user base,
and the maturity of the engineering organization. A startup deploying to hundreds
of users may not benefit from canary analysis automation that serves an organization
deploying to millions.

## Relationship to Adjacent Practices

Progressive delivery intersects with several other engineering disciplines:

- **Continuous delivery** — progressive delivery builds on CD; without automated,
  reliable deployment pipelines, controlled rollout is impractical
- **Observability** — the feedback loop that makes progressive delivery decisions
  possible; without it, progressive delivery is just complex deployment
- **Chaos engineering** — both practices accept that failures will occur and focus on
  controlling their impact; they share observability infrastructure
- **Trunk-based development** — feature flags enable trunk-based development by
  decoupling merge from release; incomplete features merge behind flags
- **Platform engineering** — internal developer platforms often provide progressive
  delivery capabilities as a service to application teams
