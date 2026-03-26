# SRE SLO Engineering — SLI Selection, Error Budgets & Release Decisions

## Core Concepts

An SLI (Service Level Indicator) is a **measurable metric** of service behavior. An SLO (Service Level Objective) is a **target for that metric** over a time window. Error budgets are the inverse: the tolerated deviation from SLO, spent on risk-taking (deployments, experiments, maintenance).

```
SLI = measured value (e.g., "99.2% of requests succeeded")
SLO = target (e.g., "99% of requests must succeed")
Error Budget = 1 - SLO = allowed "budget" of failures (e.g., 1% of requests can fail)
```

SLOs align reliability investment with business needs. A system targeting 99% availability (8.5 hours of acceptable downtime per month) doesn't justify spending 10x to reach 99.99%. The SLO makes that trade-off explicit.

## Selecting SLIs

Choosing the right SLI requires understanding what users perceive and care about. Bad SLI choices hide real problems or over-optimize the wrong dimension.

### SLI Types & Examples

| SLI Type | Measures | Example | When to Use |
|----------|----------|---------|------------|
| **Availability** | Did the request succeed? | 99% of HTTP requests return 2xx/3xx | Basic requirement for all customer-facing services |
| **Latency** | How fast? | 99th percentile ≤ 500ms | User-facing systems where speed affects experience |
| **Error rate** | How many failures? | <0.1% of requests were user-triggered failures | Services expected to have transient failures |
| **Freshness/Staleness** | How current is the data? | Data updated within 1 hour | Batch systems, caches, dashboards |
| **Durability** | Can I retrieve what I stored? | 99.999% of written records retrievable 30 days later | Storage systems, databases, data pipelines |
| **Correctness** | Is the answer right? | 99.9% of search results have expected properties | Analytics, machine learning, content delivery |
| **Throughput** | Volume processed | 1000+ requests/sec sustained | Batch processors, API gateways |

### User-Centric SLI Design

The best SLI measures what users actually experience, not component health.

**Anti-pattern**: "API server CPU < 80%, memory < 75%, all pods healthy" → Operations metric, not user-visible reliability.

**Better**: "99% of user transactions complete within 3 seconds" → Directly tied to user experience.

**Depth**: A single SLI is often too coarse. Decompose:

- Availability by user tier (premium tier tolerates less downtime than free tier)
- Latency by operation (search should be <100ms; batch export can be < 30s)
- Error rate by error class (timeouts vs. auth failures vs. data inconsistencies)

Google's approach: Define 3–5 SLIs per service, weighted by business importance. Measure them separately, aggregate at decision points.

### Implementation: Server-Side vs. Client-Side

**Server-side measurement**: The service logs whether each request succeeded (2xx response, no error). Advantages: local data, no client cooperation. Disadvantage: misses issues "between" service and user (DNS, CDN, client network, browser crashes).

**Client-side measurement**: The user's browser or mobile app reports success/failure. Advantages: captures true user experience, including all layers. Disadvantage: requires client instrumentation, sampling bias (offline clients don't report), variance due to client diversity.

**Best practice**: Measure both. Server-side SLI for alerting and internal decisions; client-side SLI for business decisions and annual reviews (it's the ground truth).

## Setting SLO Targets

SLO selection is a business decision, masquerading as a technical one.

### Common Target Levels

| Nines | Availability Per Year | Per Month | Per Week | Per Day |
|-------|----------------------|-----------|---------|---------|
| 99% | 87.6 hours | 7.2 hours | 1.7 hours | 14 min |
| 99.5% | 43.8 hours | 3.6 hours | 0.84 hours | 7 min |
| 99.9% | 8.76 hours | 43 min | 10 min | 1.4 min |
| 99.99% | 52 min | 4.3 min | 1 min | 8.6 sec |
| 99.999% | 5.2 min | 26 sec | 6 sec | 0.86 sec |

**The cost of additional 9s grows nonlinearly**: Going from 99% to 99.9% often requires better monitoring (detect faster) and redundancy (reduce blast radius). Going from 99.9% to 99.99% often requires geographic distribution, failover automation, and specialized hardware.

### How to Choose

**Start with business constraints**:

- **Revenue impact**: Downtime costs $X per minute? Work backward: "We can tolerate 1 hour per month" → 99.93%.
- **Industry norms**: Credit card processing expects 99.99%. Email tolerates 99.9%. Internal tooling might be 99%.
- **Competitive positioning**: If competitors target 99.9%, targeting 99.5% makes you uncompetitive.
- **Team capacity**: 99.99% requires on-call depth (multiple responders), sophisticated monitoring, and rehearsed failovers. A startup might not have this capability.

**Conservative start**: Pick the minimum credible target (e.g., 99.5–99.9%), measure vs. actual, and adjust. Targets are not commitments; they're statements of intent with reality checks.

**Per-customer SLOs**: Different customers have different needs. Premium customers might require 99.99%; free tier might be 99%. Make this explicit.

### Seasonality & Planning Periods

SLOs are usually measured over:

- **Monthly or quarterly** (easy to reason about; aligns with business planning)
- **Rolling 30 days** (smoother; a bad day doesn't doom the month)
- **Calendar month** (aligns with billing; creates cliff risks at month-end)

Choose based on operational reality. If you have predictable maintenance windows (e.g., third Sunday of month), calendar months make it visible. If incidents cluster (e.g., end-of-quarter deployments), rolling windows are fairer.

## Error Budgets

An error budget is the allowed deviation from SLO, spent on change and risk-taking.

### Calculating Error Budget

```
Error Budget = (1 - SLO) × Time Period

Example: 99.9% SLO over 30 days
Error Budget = (1 - 0.999) × (30 days × 24 hours × 60 min)
            = 0.001 × 43,200 min
            = 43.2 minutes allowed downtime
```

In 30 days, you can incur 43.2 minutes of unavailability and still meet the SLO. Anything beyond that violates the SLO.

### Budget Spending

Error budgets are spent on:

- **New deployments** (always introduce risk)
- **Experiments** (A/B tests, new features)
- **Maintenance** (infrastructure upgrades, data migrations)
- **Incident mitigation** (rapid fixes that might introduce new bugs)
- **Incidents** (actual failures)

**Mental shift**: Instead of "we don't deploy on Fridays" (arbitrary, overly conservative), ask "do we have error budget remaining this month?" If yes, deploy. If no, defer high-risk changes.

### Budget Burn Rate

**Burn rate** is the speed at which error budget is consumed.

```
Burn Rate = (1 - current_SLI_value) / (1 - SLO_target)

If SLI is 95% and SLO is 99%, burn rate = 0.05 / 0.01 = 5x
Meaning: the service is failing 5× faster than our SLO allows.
```

**Burn rate interpretation**:

- 1x burn: Service is exactly at SLO target; budget will exactly last the period.
- 1–2x burn: Service is degraded but sustainable; budget will be exhausted in half the period.
- 5x+ burn: Critical; error budget will be exhausted in ~1/5 of the period if this continues.

### Multi-Window Alerting

Simple threshold alerts ("page if error rate > 1%") are brittle. Better: alert based on burn rate over multiple windows.

**Example strategy** (Google SRE):

- **2% 1-hour error budget burn**: Alert after 50 hours of monitoring (when you realize burn is sustained)
- **10% 5-minute burn**: Alert immediately (this is catastrophic failures happening now)
- **50% 30-minute burn**: Critical escalation (service is down)

This avoids false positives (brief blip during deployment) while catching sustained degradation.

## SLO-Based Release Decisions

### The Release Decision Framework

Before deploying, ask:
1. Do we have error budget remaining this period?
2. If this deployment has a 0.1% chance of introducing a 10-minute outage, what's the expected impact on our budget?
3. Is the expected impact acceptable, or should we defer?

This makes risk explicit and quantifies trade-offs.

### Change Velocity vs. Reliability

**The core SRE trade-off**:

```
Low velocity (rare deployments)    → High reliability, innovation stalled
High velocity (daily deployments)  → Fast feature delivery, reliability at risk
SLO-based framework balances:     → Spend available error budget on features
```

**Policy example**:

- If error budget consumed < 25% this period: Deploy with standard process (canary, staged rollout).
- If 25–75% consumed: Deploy only critical fixes; require extra validation for features.
- If > 75% consumed: Freeze non-critical changes; focus on stability recovery.

This is more flexible than "no deploys on Fridays" and tied to actual reliability.

### User Journey SLOs

Aggregate SLOs across a user's multi-step journey, not just individual services.

**Example e-commerce flow**:

1. Browse products (GET /products)
2. Click product (GET /product/{id})
3. Add to cart (POST /cart)
4. Checkout (POST /checkout)

**Naive SLO**: Each service targets 99.9%.

**Actual user success rate**: 0.999^4 = 0.9964 ≈ 99.64% (worse than any individual service's SLO!)

**User journey SLO approach**:

- Define the journey as one logical unit
- Measure: % of journeys that complete end-to-end
- Set the SLO at the journey level (e.g., 99.5%)
- Budget reliability improvements across components where they're cheapest (maybe auth improves latency; checkout improves availability)

This prevents the "each component is reliable but the system isn't" trap.

## SLO Documentation & Communication

**SLO documents** should communicate:

1. **Service boundaries**: What is "in scope" for this SLO? (e.g., "From user requests hitting the LB to responses returned; excludes client-side issues, CDN failures, ISP downtime")
2. **SLI definition**: Exactly how the metric is measured. Include code or queries.
3. **SLO targets**: The percentage and time window. Include rationale if available ("selected to match competitor standards" or "budget of 40 min/month to deploy safely").
4. **Error budget policy**: How the budget is tracked and spent. Define "freeze" conditions.
5. **SLI history**: Past 12 months of actual SLI performance. Are we consistently meeting the SLO? Consistently overperforming? This informs whether target is realistic.
6. **Exceptions**: Planned downtime, excluded incident types, definitions of "user-visible" failures.

Make documents accessible to product, business, and engineering leadership. SLOs are agreements between engineering and the business about acceptable trade-offs.

## Organizational Adoption

### Common Pitfalls

**SLOs without budgets** lose credibility: "99.9% uptime is our goal" means nothing if you freeze all changes. Budgets make tradeoffs real.

**SLOs that are never updated**: If actual SLI outperforms target year-over-year, raise the target or redeploy the margin elsewhere. Stale targets drift from reality.

**SLOs that are pure marketing**: "We promise 99.99% uptime" when internal target is 99.5%. Eventually the gap gets exposed; it damages trust.

**SLOs that aren't instrumented**: You can't manage what you don't measure. No alerting on SLI? No dashboards? It's dead—it'll be ignored.

### Building SLO Culture

- **Start with 1–2 key services**: Get the disciplines right (measurement, alerting, decision-making) before scaling.
- **Public SLO status**: Everyone can see current SLI, burn rate, budget remaining. Transparency drives accountability.
- **Monthly review**: Team reviews SLO performance, burn rate trends, action items (if consistently overperforming, raise target; if underperforming, investigate why).
- **Link to release process**: "Can we deploy?" → check SLO budget. This makes the connection visible and enforced.
- **Celebrate overperformance**: If team consistently outperforms targets, celebrate and use margin for risky improvements (technical debt, experiments).

SLOs done well become the unifying language: engineering can talk to product about risk, business can understand reliability trade-offs, and neither feels surprised when incidents happen (they're expected; the question is whether they blow the budget).