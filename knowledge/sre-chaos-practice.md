# SRE — Chaos Engineering in Practice: Experiments, Tools & Organizational Adoption

## Core Principles

Chaos engineering deliberately injects failures into production or staging environments to validate that systems remain resilient. Rather than waiting for failures to surprise you, you provoke them under controlled conditions.

**Key premise**: If a system cannot tolerate a specific failure mode, it will fail under that condition whether you test it first or discover it in 3am production incident.

## Steady State Hypothesis

Every chaos experiment begins with a **steady state hypothesis**: "When system is operating normally, [metric] stays within [bounds]."

Examples:

- "99th percentile latency < 100ms when requests are evenly distributed"
- "All requests eventually complete or timeout after 30s"
- "Traffic loss during AZ failover < 0.1%"
- "CPU stays below 80% during normal load"

During the experiment, you violate the assumed conditions (inject latency, kill instances, partition network), then verify whether steady state breaks. If it does, you've discovered a vulnerability. If it holds, you've validated resilience.

### Distinguishing chaos from other testing

- **Unit tests** validate single components in isolation
- **Integration tests** exercise multiple components with mocked failures
- **Load tests** measure behavior under high traffic (but assume network/infrastructure work)
- **Chaos experiments** inject *real* failures (latency, packet loss, service unavailability, data corruption) to find emergent system vulnerabilities

## Chaos Engineering in Practice: Tools & Patterns

### Chaos Monkey & Netflix Legacy

**Chaos Monkey** (Netflix, 2010s) pioneered chaos by randomly killing production instances. The premise: if Netflix's systems break because one instance dies, they're too fragile. By regularly inducing failures, engineering teams either built resilience or experienced the failure on their schedule.

Lessons from Chaos Monkey:

- **Large blast radius is acceptable** if you still maintain availability; the goal is to find where architecture breaks
- **Randomness prevents adaptation**: If failures are predictable, teams overfit; randomness forces robust solutions
- **Production testing is necessary**: Staging often lacks the complexity, scale, and real hardware behavior of production

### Modern Chaos Tools

| Tool | Focus | Use Case |
|------|-------|----------|
| **Chaos Monkey** (Netflix) | Random instance termination | Fault tolerance, auto-scaling validation |
| **Litmus** (CNCF, Kubernetes-native) | Declarative chaos experiments | Kubernetes-specific resilience (pod eviction, node failure) |
| **Gremlin** | Commercial; network, compute, storage faults | Enterprise organizations with guidance + safety |
| **ChaosBlade** | Fault injection framework; supports Kubernetes, JVM, containers, Linux | Large-scale, diverse infrastructure |
| **Toxiproxy** | Proxy-based application-level faults | Latency, packet loss, connection resets between services |

### Blast Radius Control

Chaos without boundaries causes real outages. Mitigate risk:

1. **Start in staging**: Prove hypothesis on non-production before promoting to prod
2. **Limit scope**: Inject failure into single service, region, or instance pool first
3. **Canary approach**: Run chaos against 1% of traffic; increase gradually
4. **Time windows**: Schedule during business hours with team present + adjacent team monitoring
5. **Abort criteria**: Pre-define conditions that halt the experiment (error rate > threshold, user complaints, specific metric anomaly)
6. **Runbook nearby**: On-call team available; if chaos breaks something unexpected, they can respond

### Common Chaos Experiments

**Network-level failures**:

- **Latency injection**: Add 500ms) to network calls; verify timeouts + retries work
- **Packet loss**: Drop 10% of traffic; confirm circuit breakers kick in after threshold
- **Network partition**: Simulate AZ connectivity loss; verify failover behavior
- **Connection reset**: Abruptly close connections; check if clients reconnect correctly

**Compute failures**:

- **CPU stress**: Consume 80% CPU; confirm system degrades gracefully, not crashes
- **Memory pressure**: Fill memory to trigger OOM; does graceful shutdown execute? Do others restart?
- **Disk space exhaustion**: Reduce disk; verify error handling (logs, temp storage)
- **Process termination**: Kill critical process; does orchestrator restart it? How quickly?

**Dependency failures**:

- **Service unavailability**: Mock external API returning 503; does system fallback, queue requests, or cascade?
- **Slow dependency**: Simulate external service responding in 30s; are timeouts respected?
- **Data corruption**: Return corrupted responses; does system detect and recover?
- **Authentication failure**: Mock auth service offline; can system still validate cached tokens?

## Chaos Maturity Model

Adoption progresses through stages:

### Level 1: Awareness

Team understands the value of chaos; no formal practice yet. Ad-hoc testing ("let's see what breaks if we kill this service").

### Level 2: Defined Experiments

Team selects specific scenarios, documents steady state hypotheses, and runs them regularly (monthly, quarterly). Often scripted but not yet continuous.

Example: "First Tuesday of each month, we run AZ failover chaos. Do we lose < 0.1% of requests?"

### Level 3: Automated Continuous Chaos

Chaos runs automatically on schedule or triggered by deployments. Results are tracked; regressions alert the team.

Example: Gremlin runs low-intensity experiments nightly; if steady state breaks, pipeline blocks deployment.

### Level 4: Resilience as Requirement

Chaos results inform SLO definitions, on-call rotations, and deployment policies. Services must pass defined chaos scenarios (e.g., "survive 50ms latency injection") before reaching production.

### Level 5: Organizationally Integrated

Chaos is part of hiring, onboarding, and architecture review. "Does this design survive [blast radius X]?" is a standard question. Findings flow to strategic decisions: infrastructure choices, service design, team structure.

## Game Days & War Gaming

**Game days** simulate realistic incidents in a controlled setting:

1. Injectors (chaos team) introduce failures (network partition, load spike, data corruption)
2. Responders (on-call team) detect and respond using normal procedures
3. Observers record timeline, decisions, and handoff effectiveness
4. Post-game, team reviews what worked, what broke, and changes to defense

Example: "Today we simulate losing an entire database replica. Did we lose data? How long until traffic shifted? Did on-call detect and escalate correctly?"

Benefits:

- Discovers gaps in monitoring (you can't respond to what you don't see)
- Validates runbook accuracy (chaos often exposes outdated documentation)
- Trains on-call in low-pressure environment (mistakes don't cause real incidents)
- Builds team muscle memory for incident response

## Organizational Adoption Patterns

### Stages of Integration

**Phase 1: Enthusiasts**

Single team or platform org (infrastructure, SRE) runs chaos. Findings are shared but not mandatory.

Obstacles: Expense, time, skepticism from application teams.

**Phase 2: Chasing Incidents**

After a production incident, org runs chaos to test if that scenario would have been caught. Painful lessons accelerate adoption.

**Phase 3: Systematic Coverage**

Chaos becomes part of deployment pipelines. Services must pass defined scenarios before canary→production.

**Phase 4: Strategic Decisions**

Architecture decisions ("Should we use multi-region or single-region?") are informed by chaos results.

### Governance & Safety

- **Communication**: Announce chaos window; notify stakeholders
- **Progress tracking**: Public dashboard of chaos results (which services, which scenarios, pass/fail)
- **Failures as learning**: Treat chaos-discovered vulnerabilities as low-severity findings, not incidents
- **Investment tie-in**: Budget resilience engineering work from chaos insights (don't just run experiments; fix findings)

## Pitfalls

1. **Chaos without action**: Running experiments but not fixing findings wastes effort
2. **Over-broad blast radius**: Killing 50% of infrastructure is a real outage, not a learning opportunity
3. **Metrics-blind**: Experiments with no observability; can't tell if steady state broke or held
4. **Isolated to infrastructure**: App-level chaos (dependency fault, slow API) often reveals more vulnerabilities than infrastructure
5. **Decorative documentation**: Runbooks exist but weren't tested by chaos; they don't reflect reality

## See Also

- [Testing — Chaos Testing for Applications](testing-chaos.md) — Application-level fault injection
- [Architecture Resilience](architecture-resilience.md) — Patterns for building resilient systems
- [Devops — Chaos Engineering](devops-chaos-engineering.md) — Broader chaos engineering context