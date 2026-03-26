# Site Reliability Engineering — Principles, SLOs & Error Budgets

## The SRE Philosophy

Site Reliability Engineering emerged from the observation that operations problems are fundamentally software engineering problems. Rather than treating reliability as a separate discipline from development, SRE applies software engineering approaches — automation, measurement, iterative improvement — to operational concerns.

The central tension in SRE is **reliability vs. velocity**. Every system exists on a spectrum: maximizing reliability means slowing change (fewer deployments, more testing, conservative rollouts), while maximizing velocity means accepting more risk. SRE provides a framework for making this trade-off explicit and data-driven rather than political.

| Approach                    | Reliability Bias                   | Velocity Bias            |
| --------------------------- | ---------------------------------- | ------------------------ |
| Change freeze               | High stability, zero new features  | N/A                      |
| Weekly releases with canary | Moderate risk, moderate throughput | Balanced                 |
| Continuous deployment       | Risk managed by automation         | High throughput          |
| No gates at all             | Uncontrolled risk                  | Maximum short-term speed |

The philosophical foundation: **100% reliability is the wrong target for almost every system.** Users cannot perceive the difference between 99.999% and 100% availability for most services, yet the engineering cost between those two numbers is enormous. SRE makes this cost curve visible and navigable.

## SLIs, SLOs, and SLAs

Three related but distinct concepts form the measurement backbone of reliability engineering:

### Service Level Indicators (SLIs)

An SLI is a **quantitative measure of some aspect of service behavior**. SLIs answer "how is the service performing right now?" Common SLI categories:

| SLI Category | What It Measures                    | Example Metric                                   |
| ------------ | ----------------------------------- | ------------------------------------------------ |
| Availability | Whether the service responds at all | Proportion of successful requests                |
| Latency      | How fast responses arrive           | 50th, 95th, 99th percentile response time        |
| Throughput   | Volume the system handles           | Requests per second processed                    |
| Error rate   | Proportion of incorrect responses   | 5xx responses / total responses                  |
| Freshness    | How current the data is             | Age of the most recent data point                |
| Durability   | Whether stored data persists        | Proportion of data retrievable after write       |
| Correctness  | Whether responses are right         | Proportion of responses matching expected output |

Choosing the right SLIs requires understanding what users actually experience. A service might have 100% server-side availability while being unreachable due to DNS issues — the SLI must capture the user-visible behavior, not just the component health.

**SLI specification vs. implementation**: The specification defines _what_ to measure (e.g., "proportion of requests served in under 200ms"), while the implementation defines _how_ to measure it (e.g., server-side logs, client-side instrumentation, synthetic probes). Different implementations of the same specification can yield different numbers.

### Service Level Objectives (SLOs)

An SLO is a **target value or range for an SLI**, measured over a time window. SLOs answer "how reliable should this service be?"

```
SLO = SLI + Target + Time Window

Example: 99.9% of requests complete successfully (SLI: availability)
         measured over a rolling 30-day window (time window)
```

SLOs serve multiple purposes:

- **Engineering prioritization**: When the SLO is at risk, reliability work takes precedence
- **Expectation setting**: Teams and stakeholders share a common understanding of acceptable behavior
- **Alerting thresholds**: Alerts fire based on SLO burn rate, not raw metric thresholds
- **Architecture decisions**: SLO targets influence redundancy, caching, and failover design

Setting SLOs too tight wastes engineering effort on diminishing returns. Setting them too loose erodes user trust. The calibration process typically involves examining historical performance, understanding user expectations, and considering the cost of improvement.

### Service Level Agreements (SLAs)

An SLA is a **contractual commitment**, typically with financial consequences for violation. SLAs are business instruments; SLOs are engineering instruments.

A common pattern: set SLOs tighter than SLAs. If the SLA promises 99.9% availability, the internal SLO might target 99.95% — providing a buffer between "we need to act" and "we owe customers money."

## The Error Budget Concept

The error budget is the inverse of the SLO, expressed as the **permitted amount of unreliability** over a time window.

```
Error Budget = 1 - SLO Target

If SLO = 99.9% availability over 30 days:
  Error budget = 0.1% of requests can fail
  In time: ~43.2 minutes of downtime per 30-day period
```

Error budgets transform the reliability-vs-velocity conversation from subjective ("we should be more careful") to objective ("we have 28 minutes of budget remaining this month").

### Error Budget Policies

Organizations define policies around error budget consumption:

| Budget Status                      | Typical Response                                                |
| ---------------------------------- | --------------------------------------------------------------- |
| Budget healthy (>50% remaining)    | Normal development velocity, feature work proceeds              |
| Budget strained (25-50% remaining) | Increased caution, more thorough rollout procedures             |
| Budget low (<25% remaining)        | Shift focus to reliability, reduce risky changes                |
| Budget exhausted (0% remaining)    | Feature freeze until budget recovers, all effort on reliability |

The power of error budgets lies in giving both product and operations teams a shared incentive structure. Product teams want to ship features (consuming budget); operations teams want stability (preserving budget). The budget makes the trade-off visible and negotiable.

### Complications with Error Budgets

Error budgets assume that unreliability is evenly distributed, but in practice:

- A single 30-minute outage has a different user impact than 30 one-minute blips
- Some failures affect all users; others affect a small subset
- Budget consumption from planned maintenance differs from production incidents
- External dependencies can consume budget outside the team's control

## Nines of Availability

The "nines" shorthand quantifies availability targets and their practical implications:

| Availability         | Downtime/Year | Downtime/Month | Downtime/Week |
| -------------------- | ------------- | -------------- | ------------- |
| 99% (two nines)      | 3.65 days     | 7.31 hours     | 1.68 hours    |
| 99.9% (three nines)  | 8.77 hours    | 43.83 minutes  | 10.08 minutes |
| 99.95%               | 4.38 hours    | 21.92 minutes  | 5.04 minutes  |
| 99.99% (four nines)  | 52.60 minutes | 4.38 minutes   | 1.01 minutes  |
| 99.999% (five nines) | 5.26 minutes  | 26.30 seconds  | 6.05 seconds  |

Each additional nine roughly requires an order of magnitude more engineering investment. The jump from three nines to four nines often requires fundamentally different architectures — active-active redundancy, automated failover, multi-region deployment — not just incremental improvement.

**Composite availability**: When services depend on other services, the combined availability is the product of individual availabilities. Two services each at 99.9% yield a combined 99.8% for requests traversing both. This cascading math explains why microservice architectures require higher per-service availability targets than monoliths.

## Reliability vs. Durability

These terms are frequently conflated but measure different properties:

- **Reliability/Availability**: The system responds when requested. Measured as uptime or successful request proportion.
- **Durability**: Data survives over time. Measured as the probability of data loss over a period.

A storage system can be highly durable (data never lost) but have poor availability (frequent periods where reads fail). Conversely, a cache can be highly available but not durable (data evicted under pressure). The engineering strategies for each differ substantially.

## Toil

Toil is **manual, repetitive, automatable, tactical, devoid of enduring value, and scales linearly with service growth**. It is distinct from overhead (meetings, planning) and distinct from necessary engineering work.

Characteristics that identify toil:

- **Manual**: Requires a human to perform
- **Repetitive**: Done more than once or twice
- **Automatable**: Could be handled by software
- **Reactive**: Triggered by external events rather than proactive improvement
- **No enduring value**: The system is in the same state after the work as before
- **Scales with growth**: More users/services means more toil

Examples across the toil spectrum:

| Activity                                    | Likely Toil?  | Reasoning                                          |
| ------------------------------------------- | ------------- | -------------------------------------------------- |
| Manually restarting a crashed service       | Yes           | Repetitive, automatable, reactive                  |
| Writing a postmortem                        | No            | Produces enduring value (learning)                 |
| Manually scaling capacity for a known event | Depends       | If predictable, automatable; if novel, engineering |
| Rotating credentials quarterly              | Yes if manual | Repetitive, automatable                            |
| Investigating a novel alert                 | No            | Requires judgment, produces understanding          |

The SRE guideline — keeping toil below roughly 50% of a team's time — provides a forcing function for automation investment. When toil exceeds the threshold, it crowds out the engineering work that would reduce future toil, creating a vicious cycle.

## The Fallacies of Distributed Computing Applied to Reliability

The eight fallacies, first articulated in the 1990s, remain relevant to reliability planning:

1. **The network is reliable** → Design for network partitions, timeouts, retries with backoff
2. **Latency is zero** → Budget for latency variance; tail latency (p99, p999) matters more than median
3. **Bandwidth is infinite** → Capacity plan for peak, not average; consider payload sizes
4. **The network is secure** → Defense in depth; assume breach in reliability modeling
5. **Topology doesn't change** → Service discovery, health checking, dynamic routing
6. **There is one administrator** → Coordination costs scale with organizational complexity
7. **Transport cost is zero** → Cross-region replication has real latency and financial cost
8. **The network is homogeneous** → Different paths have different failure modes

Each fallacy maps to a reliability risk. Systems designed with these fallacies in mind tend to degrade gracefully; those that assume them tend to fail catastrophically.

## Capacity Planning

Capacity planning balances the cost of over-provisioning against the risk of under-provisioning:

**Demand forecasting approaches**:

- **Organic growth**: Extrapolating historical usage trends
- **Inorganic growth**: Anticipated spikes from launches, marketing campaigns, seasonal patterns
- **Adoption curves**: New features that shift usage patterns
- **Headroom**: Buffer above predicted peak for unexpected demand

**Provisioning strategies**:

| Strategy                    | Trade-off                                        |
| --------------------------- | ------------------------------------------------ |
| Static provisioning at peak | High cost, high reliability, zero scaling delay  |
| Auto-scaling on demand      | Lower cost, scaling delay risk, complexity       |
| Scheduled scaling           | Moderate cost, works for predictable patterns    |
| Hybrid (base + burst)       | Base static capacity plus auto-scaling for peaks |

**Load testing as capacity validation**: Synthetic load tests in production-like environments reveal bottlenecks that capacity models miss — database connection limits, lock contention, garbage collection pauses under load, upstream rate limits.

## Load Shedding and Graceful Degradation

When demand exceeds capacity, systems face a choice: serve everyone poorly or serve some well and reject the rest.

**Load shedding** — deliberately dropping requests when the system is overloaded — preserves service quality for admitted traffic. Approaches include:

- Priority-based admission control (serve paid users, shed free-tier traffic)
- Random rejection above a threshold
- Queue-based rejection when wait times exceed usefulness
- Circuit breakers that stop calling failing dependencies

**Graceful degradation** — reducing functionality rather than failing entirely:

- Serve cached data when the backend is unavailable
- Disable non-critical features under load (recommendations, analytics)
- Reduce response fidelity (lower-resolution images, fewer search results)
- Switch from real-time to batch processing

The key insight: degradation modes must be designed and tested before they are needed. A system that has never practiced degradation will not degrade gracefully under novel stress.

## On-Call Practices

On-call is the operational backbone of reliability, but unsustainable on-call practices are a leading cause of SRE attrition.

**Rotation design considerations**:

- Rotation length (weekly is common; shorter rotations reduce fatigue but increase context-switching)
- Primary/secondary structure for escalation
- Follow-the-sun vs. single-timezone rotations
- Compensation philosophy (time off in lieu, additional pay, or included in role expectations)

**Cognitive load management**:

- Limit the number of services one responder covers
- Provide runbooks and diagnostic tools that reduce time-to-context
- Set expectations for response time by severity (page-level vs. ticket-level)
- Track interrupt frequency per rotation to detect unsustainable trends

**Escalation** should be frictionless. If the primary responder cannot resolve within a defined window, escalation paths — to secondary on-call, subject-matter experts, or management for customer communication — must be well-documented and regularly rehearsed.

## Risk Analysis and Risk Acceptance

Not all risks justify mitigation. Engineering involves explicitly accepting some risks:

**Risk assessment factors**:

- **Likelihood**: How probable is the failure scenario?
- **Impact**: What is the blast radius — users affected, data lost, revenue impacted?
- **Detection time**: How quickly would the failure be noticed?
- **Recovery time**: How long to restore service?
- **Mitigation cost**: What does it cost to reduce the risk?

**Risk register** as a living document:

| Risk                   | Likelihood | Impact   | Mitigation                        | Status              |
| ---------------------- | ---------- | -------- | --------------------------------- | ------------------- |
| Single-region outage   | Low        | High     | Multi-region failover             | Mitigated           |
| Database corruption    | Very Low   | Critical | Point-in-time recovery, checksums | Partially mitigated |
| Key person departure   | Medium     | Medium   | Documentation, cross-training     | Accepted risk       |
| Third-party API outage | Medium     | Medium   | Circuit breaker, cached fallback  | Mitigated           |

The practice of explicitly recording accepted risks — and the reasoning behind acceptance — prevents knowledge loss when team members rotate and provides audit trails for future review.

## SLO-Driven Operational Decisions

SLOs transform operational conversations from "is this bad?" to "is this consuming our error budget at an unsustainable rate?"

**Burn rate alerting**: Rather than alerting on instantaneous error spikes, burn-rate alerts fire when the error rate, if sustained, would exhaust the error budget before the window ends. This approach reduces alert noise from transient issues while catching sustained degradation early.

```
Burn rate = (observed error rate) / (SLO-permitted error rate)

Burn rate 1.0 = consuming budget at exactly the expected rate
Burn rate 10.0 = consuming budget 10x faster than sustainable
```

**Multi-window burn rates** combine fast-burn detection (short window, high threshold — catches acute incidents) with slow-burn detection (long window, lower threshold — catches gradual degradation).

The operational cadence typically follows:

1. SLO dashboards reviewed regularly (weekly or biweekly)
2. Error budget trends inform sprint planning — reliability tasks vs. feature tasks
3. Budget exhaustion triggers defined policy responses
4. Quarterly SLO review adjusts targets based on user feedback and cost analysis

This framework replaces subjective "the system feels slow" conversations with data-driven discussions about where to invest engineering effort for the greatest reliability return.
