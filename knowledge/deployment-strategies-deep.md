# Deployment Strategies — Comparison, Trade-Offs & Implementation

## Overview

A deployment strategy is the mechanism by which a new version of software is made live. The choice affects downtime, risk exposure, infrastructure cost, rollback complexity, and observability requirements. Each strategy represents a different point on a spectrum: instant cutover (risky, simple) vs. gradual rollout (safer, complex).

No strategy is universally best. The choice depends on system risk tolerance, infrastructure capacity, and observability maturity.

## Blue-Green Deployment

### How It Works

Two identical production environments run in parallel: **Blue** (live, serving traffic) and **Green** (standby, new version). At deployment:

1. New version is deployed to Green
2. Green is tested (health checks, smoke tests, or manual validation)
3. Load balancer/router switches traffic from Blue to Green (instant cutover)
4. Blue becomes the standby for next deployment

### Advantages

- **True zero-downtime:** Traffic switches instantaneously
- **Simple rollback:** Route traffic back to Blue immediately
- **Full environment test:** The entire new version runs in production-like conditions before traffic arrives
- **No gradual transition complexity:** Either all traffic is on Blue or all on Green

### Disadvantages

- **Infrastructure cost:** Must provision 2x capacity (two full environments running simultaneously)
- **Database migration complexity:** If schema changes, must coordinate migration timing with traffic switch
- **State management:** Session stores, caches must be accessible from either environment or replicated
- **Slow successive deployments:** Must wait for Green become Blue and a new Green to be provisioned

### When to Use

- **Cost is not a constraint:** Large orgs with capital budgets
- **Deployments are infrequent:** Once per day or less; the 2x capacity idle time is acceptable
- **Simple applications:** Stateless services or with replicated session stores
- **Need instant rollback guarantees:** Financial transactions, critical systems

### Database Considerations

Blue-Green with schema changes requires **database migration management**:

**Approach 1: Expand-Contract (see deployment-database-migrations.md)**
- Blue and Green share the same database
- Migration uses expand-contract: deploy schema additions, traffic switch, then clean up old schema
- No database downtime, but requires migration discipline

**Approach 2: Database replication**
- Replicate the Blue database to Green beforehand
- After traffic switch, Green is primary (async replication lag possible)
- Requires careful handling of primary key conflicts, sequences

**Approach 3: Separate databases**
- Blue and Green have separate databases
- Requires application-level data synchronization (ETL, dual-write) pre-deployment
- Complex, rarely justified

### Example: Kubernetes Blue-Green

```yaml
# Green (new version)
apiVersion: apps/v1
kind: Deployment
metadata:
  name: app-green
spec:
  replicas: 3
  selector:
    matchLabels:
      app: app
      version: green
  template:
    metadata:
      labels:
        app: app
        version: green
    spec:
      containers:
      - name: app
        image: app:v2.0
        # ...

---
# Service points to green
apiVersion: v1
kind: Service
metadata:
  name: app
spec:
  selector:
    app: app
    version: green  # Switch here to blue after rollout
  ports:
  - port: 80
```

Switch traffic by changing the selector and re-applying the Service.

## Canary Deployment

### How It Works

New version is deployed to a **small subset of infrastructure** (10% of machines, 5% of users). Traffic is gradually shifted based on success metrics:

1. Deploy new version to 1-2 instances (canary)
2. Monitor error rate, latency, business metrics on canary vs. stable version
3. If healthy, gradually increase traffic (10% → 25% → 50% → 100%)
4. If metrics degrade, halt and rollback to previous version

### Advantages

- **Risk is contained:** Only a small percentage of users are affected by bugs
- **Observability-driven:** Metrics directly inform rollout decisions
- **Automatic gradual rollout:** Traffic shifting can be automated based on metric gates
- **No 2x capacity needed:** Only a small number of canary instances; infrastructure is reused

### Disadvantages

- **Requires sophisticated observability:** Must collect and compare metrics between canary and stable version
- **Complex routing:** Balancer must support traffic splitting (can be simpler with service mesh)
- **Longer deployment time:** Gradual rollout takes minutes to hours
- **State consistency:** Users might flip between canary and stable version (caches, sessions must be replicated)

### Metric Gates

Canary deployment is not purely gradual; it's **metric-gated**. Common gates:

- **Error rate:** Canary error rate < baseline + threshold
- **Latency:** p99 latency < baseline + threshold
- **Business metrics:** Conversion rate, checkout success, API hits (not degraded)
- **Saturation:** CPU/memory usage reasonable (not spiking)

Example gate logic:

```
if (canary_error_rate < baseline + 2%) and (canary_latency_p99 < baseline + 50ms) {
  proceed to next traffic shift
} else {
  halt rollout, alert team, consider rollback
}
```

Tools that implement this: **Flagger** (Kubernetes GitOps), **Harness**, **LaunchDarkly** (with Envoy proxy), **Istio + Kiali**.

### When to Use

- **Mature observability:** Logs, metrics, and dashboards are reliable
- **Real-time decision making:** Team can monitor and intervene quickly
- **High-risk deployments:** Major features, critical services, or schema changes
- **User base is large:** Statistically significant metrics even with 5% canary traffic

### Example: Kubernetes Canary with Flagger

```yaml
# Flagger Canary resource
apiVersion: flagger.app/v1beta1
kind: Canary
metadata:
  name: app
spec:
  targetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: app
  progressDeadlineSeconds: 300
  service:
    port: 80
  analysis:
    interval: 1m
    threshold: 5  # 5 consecutive successful metrics checks to proceed
    maxWeight: 50  # Max 50% traffic to canary
    stepWeight: 10  # Shift 10% every interval
    metrics:
    - name: request-error-rate
      thresholdRange:
        max: 1  # Max 1% error rate
      interval: 1m
    - name: request-duration
      thresholdRange:
        max: 500  # Max p99 latency 500ms
      interval: 1m
  skipAnalysis: false
```

Flagger automatically shifts traffic 10% → 20% → ... → 50% while evaluating metrics every minute.

## Rolling Deployment

### How It Works

Old and new versions run simultaneously; instances are **gradually replaced in-place**:

1. Stop instance N+1 (remove from load balancer)
2. Deploy new version to N+1
3. Add N+1 back to load balancer
4. Repeat for each remaining instance

### Advantages

- **No extra infrastructure cost:** Only need capacity for one version at a time
- **Built into orchestrators:** Kubernetes rolling update, Docker service update, health checks built-in
- **Simple mental model:** Just upgrade the app, rest is automatic
- **Works with any application:** No special session store requirements

### Disadvantages

- **Downtime risk:** If pod startup is slow, users see errors during the brief moment pod is down (requires properly configured health checks)
- **Slower than blue-green:** Cannot switch all traffic instantly; takes as long as max(rolling_update_duration, instance_startup_time)
- **Hard to rollback cleanly:** All instances are partially mixed; rolling back requires rolling out old version again
- **State handoff issues:** In-flight requests might be lost if connection pooling isn't graceful

### Zero-Downtime Rolling Updates

True zero-downtime requires:

1. **Readiness/liveness probes configured:** Orchestrator waits for new pod to be ready before considering it live
2. **Graceful shutdown:** Old process drains in-flight requests (e.g., 30s grace period before SIGKILL)
3. **Proper maxUnavailable/maxSurge:** Kubernetes parameters controlling how many instances can be down/overprovisioned during rolling update

Kubernetes example:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: app
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 1      # At most 1 pod down
      maxSurge: 1            # At most 4 pods total (3 + 1)
  template:
    spec:
      containers:
      - name: app
        image: app:v2
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 15
          periodSeconds: 10
      terminationGracePeriodSeconds: 30
```

- `readinessProbe` tells orchestrator when pod is ready to serve traffic
- `terminationGracePeriodSeconds: 30` gives the process 30s to drain connections before being forcibly killed
- `maxUnavailable: 1` ensures at most 1 pod is down (no more than 33% unavailability)

### When to Use

- **Cost-conscious deployments:** No capacity overhead
- **High-frequency deployments:** Multiple deploys per day (rolling out one instance at a time is fast enough)
- **Simple stateless applications:** Microservices, APIs, workers
- **Acceptable brief downtime:** If health checks miss a bad deployment, a few requests fail; app recovers

## A/B Testing Deployment

### How It Works

New version and old version both receive traffic, but routing is **feature-based** (usually a header, cookie, or user ID):

```
if (request.header('X-Feature-Version') === 'new') {
  route to new version
} else {
  route to old version
}
```

Users are often segmented: e.g., 50% of logged-in users get the new version; all guests get the old version.

### Advantages

- **Measure business impact:** Compare conversion, engagement, revenue between versions directly
- **Decouple deployment from rollout:** Deploy new code, but don't enable it for all users yet (controlled by feature flags)
- **Extended validation:** Keep both versions live for days/weeks to collect data
- **Instant fallback:** Switch routing without redeploying

### Disadvantages

- **Infrastructure cost:** Both versions run simultaneously (similar to blue-green)
- **Code complexity:** Must support multiple versions simultaneously; deprecated APIs must be maintained
- **Data consistency:** Shared database sees updates from both versions; schema must be compatible or coordinated
- **User experience:** Some users see new UI, some see old; inconsistency can be confusing

### Common Pattern: Feature Flags + A/B Test

A/B testing is usually implemented via **feature flags** (not a deployment strategy per se, but often paired with deployment):

1. Deploy new code to production (wrapped in feature flag)
2. Feature flag is OFF for all users initially
3. Gradually or selectively enable flag for segments (5% of users, then 25%, then 100%)
4. Monitor metrics for each segment
5. Once confident, disable old code path and remove feature flag

This decouples deployment (code is live) from rollout (feature is enabled).

### When to Use

- **Measuring user behavior:** Need causal evidence that a feature improves metrics
- **E-commerce, SaaS:** High-value business decisions depend on measurement
- **Long evaluation periods:** Need days of data to reach statistical significance
- **Code is backward-compatible:** New version can share the database with old version

## Shadow / Dark Launch

### How It Works

New version is deployed to production, but receives **only copies of production traffic** (requests are duplicated, responses from new version are discarded):

```
User Request
    │
    ├─→ Old Version (live, response sent to user)
    └─→ New Version (shadow, response discarded, metrics recorded)
```

### Advantages

- **True production load testing:** New version handles realistic traffic patterns, concurrency, data volume
- **No user impact from bugs:** New version failures don't affect live users (responses discarded)
- **Discover subtle bugs:** Issues only visible under production tail latencies, edge cases, or high concurrency appear here

### Disadvantages

- **Significant infrastructure cost:** Every request is processed twice (2x compute, network egress)
- **Cannot measure user behavior:** Shadow responses are discarded; you see technical metrics (latency, errors) but not business metrics
- **State mutation issues:** If new version makes database writes, shadow must use a separate database or defer writes
- **Long validation time:** Must run for hours/days to build confidence in different scenarios

### Implementation

AWS VPC Traffic Mirroring, Istio VirtualService with duplicate traffic, or application-level request duplication:

```python
# Application-level shadow traffic
def process_request(req):
    # Real request
    real_response = real_handler(req)
    
    # Shadow request (async, fire-and-forget)
    asyncio.create_task(shadow_handler(req))
    
    return real_response
```

### When to Use

- **Complex, stateful systems:** Subtle concurrency bugs, state corruption
- **High-risk deployments:** Major architectural changes, database optimizations
- **Infrastructure capacity available:** Can afford 2x traffic without cost
- **Need absolute confidence before rollout:** Financial systems, payment processing

## Comparing Strategies

| Strategy | Downtime | Infrastructure Cost | Risk Exposure | Rollback Speed | Complexity |
|----------|----------|---------------------|---------------|--------------------|------------|
| Blue-Green | None | 2x | Users on Blue until switch; switch instant | Instant | Medium |
| Canary | Possible from bugs | 1.05x | Gradual (5-50%) | Manual; proportional | High |
| Rolling | Possible if misconfigured | 1.0x | 100% at end of rollout | Slow (re-deploy old) | Low |
| A/B Test | None | 2x | Varies (50% both, 100% new) | Instant (flag toggle) | High |
| Shadow | None | 2x | Zero (shadow only) | N/A (no user traffic) | Medium |

## Database Migration Compatibility

All deployment strategies require database migrations to be **compatible with both old and new code**:

- **Blue-Green:** Both versions may run simultaneously during traffic switch; schema must be backward-compatible
- **Canary:** Old and new code access the same database for extended period; schema must support both
- **Rolling:** Old and new instances running together; same requirement
- **A/B Test:** Explicitly designed for both versions simultaneous; strictest database requirement

This requires the **expand-contract pattern** (see deployment-database-migrations.md):
- **Expand:** Add new schema (new column, table) without removing old schema
- **Migrate:** Move data from old schema to new (can be gradual)
- **Contract:** Remove old schema after code is fully deployed

## Rollback Procedures

### Blue-Green Rollback

Switch traffic back to Blue. Fastest rollback possible. No re-deployment needed.

### Canary Rollback

Traffic is automatically shifted back to stable version if metrics degrade. Manual rollback: change traffic split to 100% stable.

### Rolling Rollback

Requires redeploying old version. Equivalent to another rolling update, so takes as long as the original deployment.

Faster alternative: Keep old version's image in registry; scale new version to zero and old version back up (Kubernetes: scale down new Deployment, scale up old).

### A/B Test Rollback

Disable feature flag for new version. Instant (if using feature flags) or route all traffic to old version.

## Choosing a Strategy

1. **Can you afford 2x infrastructure?** Blue-green, A/B test, or shadow; otherwise rolling or canary.
2. **Need metrics to decide?** Canary or A/B test (requires observability).
3. **How often do you deploy?** High frequency → rolling. Low frequency (once a week) → blue-green. Risky changes → canary.
4. **Database schema changes?** All strategies work if migration is expand-contract. Shadow/A/B require separate DB or deferred writes.
5. **Risk tolerance?** Blue-green/A/B are lower-risk (easy rollback). Rolling is riskier but simpler.

## See Also

- **progressive-delivery.md** — Canary, blue-green, feature flags overview
- **deployment-database-migrations.md** — Zero-downtime database migrations
- **devops-cicd-patterns.md** — CI/CD workflows and automation
- **sre-slo-engineering.md** — Using error budgets to decide rollout risk