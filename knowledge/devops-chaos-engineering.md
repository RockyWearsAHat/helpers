# Chaos Engineering — Proactive Resilience Testing

Chaos engineering is deliberately injecting failures into systems to discover weaknesses *before* they cause incidents. Rather than hoping failures never happen, chaos teams simulate failures in controlled conditions: network outages, service crashes, latency spikes, resource exhaustion. The hypothesis: if the system survives controlled chaos, it's more likely to survive real failures.

## Core Principles

The Principles of Chaos Engineering (principlesofchaos.org) define the discipline:

### 1. Steady State Hypothesis

Define what "healthy" looks like for the system. Metrics that characterize normal operation:

- Average request latency, p99 latency
- Error rate (% of 5xx responses)
- Throughput (requests per second)
- Database connection pool utilization
- Cache hit rate

**Hypothesis formulated as:** *"In healthy state, p99 latency < 200ms and error rate < 0.1%"*

The experiment will vary real-world events and check if the system maintains this steady state. If steady state degrades significantly, the system has a weakness.

### 2. Vary Real-World Events

Don't inject arbitrary failures. Simulate failures that actually occur:

- Service instances crash (deployments, OOM, segfaults)
- Entire services become unavailable (datacenter outage, DNS failures)
- Latency increases (network congestion, GC pauses)
- Packet loss and corruption (WAN, WiFi networks)
- Resource exhaustion (CPU, memory, disk, file descriptors)
- External dependencies fail (database down, third-party API timeout)

Chaos tools provide libraries of realistic failure modes.

### 3. Run Experiments in Production

Real-world configuration, load, dependencies exist only in production. Staging clusters often miss:

- Customer load patterns and peak hours
- Interactions between services at scale
- Real network conditions (latency, jitter)
- Actual failure modes (not all hypothetical)

**Requirement:** Experiments must be small in scope (limited blast radius) to run safely.

### 4. Automate Experiments to Run Continuously

Manual "jump in and break things" is one-time learning. Systematic chaos requires:

- Experiments defined as code (reproducible)
- Scheduled runs (weekly, daily, or constantly)
- Automated rollback if steady state violates
- Metrics collection and reporting
- Integration with CI/CD pipeline

Automation ensures insights don't degrade over time as code changes.

### 5. Minimize Blast Radius

Limit the scope and impact of experiments so failures don't take down production:

- **Scope:** Start with one service, one datacenter, one percentage of traffic
- **Duration:** Run experiments for limited time (1-10 minutes)
- **Rollback criteria:** Kill experiment immediately if SLOs violated
- **Monitoring:** Have on-call engineer watching during first runs

*Example:* Inject 500ms latency into 5% of requests to Service A for 2 minutes. Monitor p99 latency and error rate. If either exceeds threshold, abort immediately.

**Progression:** Start small. After success, expand scope (more services, more traffic, longer duration). Never go directly to high-impact chaos on production.

## Chaos Experiments in Practice

### Experiment Structure

Each chaos experiment follows a pattern:

1. **Hypothesis:** "If database latency increases by 2s, request latency grows by < 100ms (due to timeouts and fallbacks)"
2. **Target:** Select scope (service, region, percentage of traffic, instance count)
3. **Fault:** Type and parameters (latency increase, error rate, connection drops)
4. **Duration:** How long to inject fault (2-10 minutes typical)
5. **Rollback:** How to stop or recover (kill pod, restore DNS, etc.)
6. **Steady state criteria:** What metrics to watch, thresholds for abort
7. **Analysis:** Did steady state hold? Why or why not?

### Real-World Example

**Hypothesis:** Service A has auto-retry logic. If downstream Service B is slow, A should timeout and retry. If retry succeeds 80% of the time, overall availability should remain > 99%.

**Experiment:**
```
Target: Service B (100% of requests)
Fault: Add 5s latency to all requests
Duration: 5 minutes
Abort criteria: If Service A error rate > 2% or availability < 99%
Metrics: Monitor Service A error rate, latency, retry rate; Service B latency
```

**Possible outcomes:**

- **Steady state holds:** Retry logic works. Hypothesis confirmed.
- **Steady state breaks:** Service A crashes under load (e.g., retry loop exhausts connection pool). Weakness found. Team fixes code.
- **Partial degradation:** Error rate rises to 0.5%, acceptable tradeoff. Knowledge gained: acceptable limits of retry strategy.

## Chaos Engineering Tools

### Netflix's Chaos Monkey (Pioneering)

First widely-known chaos tool. Randomly kills instances in production. Simple but powerful principle: if service survives random instance kill, it's resilient.

**Mechanism:** Daemon runs on instances, randomly kills process. Service auto-restarts (or replacement instance starts).

**Limitation:** Only tests failure of individual instances, not coordinated failures or network issues.

### Gremlin (Commercial SaaS)

Enables many failure modes:

- **Infrastructure failures:** Kill processes, crash entire VMs
- **Network chaos:** Latency, packet loss, bandwidth limits, blackhole routing
- **Resource attacks:** Consume CPU, memory, disk; open file descriptors
- **State failures:** Corrupt disk, fill filesystem, close TCP connections

**Strengths:** Exhaustive failure taxonomy, easy-to-use UI, good for teams without low-level infrastructure access.

**Model:** Cloud SaaS; agent runs on each instance/container.

### Litmus (Open-Source, Kubernetes-Native)

CNCF project for chaos engineering on Kubernetes. Chaos faults defined as CRDs (Kubernetes custom resources):

```yaml
apiVersion: litmuschaos.io/v1alpha1
kind: ChaosExperiment
metadata:
  name: pod-cpu-hog
spec:
  definition:
    image: litmuschaos/go-runner
    args:
      - -c
      - "stress -c 1 -t 300" # spike CPU for 5 minutes
```

**Strengths:** Kubernetes-native, many pod/node/cluster-level faults, community extensions.

**Community:** Active CNCF project, vendor-agnostic.

### Chaos Mesh (Cloud-Native)

Another Kubernetes chaos engineering tool. Similar scope to Litmus; differences in workflow, experiment types, and UI.

### AWS Fault Injection Simulator (FIS)

AWS-native tool for chaos in AWS infrastructure. Inject failures on EC2, ECS, RDS, etc. Tightly integrated with AWS services.

**Trade-off:** AWS-only; less flexible than Gremlin or open-source tools for non-AWS infrastructure.

### Tool Landscape

**Open-source:** Litmus (most mature), Chaos Mesh, ChaosBlade, PowerfulSeal

**Commercial:** Gremlin (most feature-complete), LaunchDarkly (feature flags + chaos), Harness (CI/CD + chaos)

**Cloud-native:** AWS FIS, Azure Chaos Studio, Google Cloud Failure Injection Testing

Choice depends on:
- Infrastructure (Kubernetes, VMs, AWS, hybrid?)
- Feature requirements (latency, CPU, network, disk?)
- Team size and expertise (docs and support important for beginners)
- Budget (open-source vs. commercial)

## Experiment Taxonomy: What to Break

### Network Faults

- **Latency:** Add delay to packets. Tests timeout handling, retry logic, slow dependencies
- **Packet loss:** Drop percentage of packets. Tests resilience to flaky networks
- **Blackhole:** Route traffic to /dev/null (connection fails immediately). Tests fallback logic
- **Bandwidth limit:** Throttle throughput. Tests queueing, backpressure handling
- **DNS failure:** Resolve to wrong address or fail. Tests fallback servers, connection pooling recovery
- **Connection reset:** Abruptly close TCP connections. Tests reconnection logic

### Resource Faults

- **CPU spike:** Consume N CPU cores. Tests CPU-intensive workload capacity, autoscaling trigger
- **Memory hog:** Allocate and hold memory. Tests OOM killer behavior, garbage collection under pressure
- **Disk full:** Exhaust disk space. Tests logging, caching systems, graceful degradation
- **File descriptor exhaustion:** Open many files/sockets. Tests connection pooling limits

### Failure Modes

- **Process kill:** Crash the service. Tests service restart, replica availability, connection draining
- **Pod restart:** Kill Kubernetes pod (replica immediately restarts). Tests graceful shutdown, connection cleanup
- **Node failure:** Entire VM/node goes down. Tests multi-node resilience, data replication

### State Corruption

- **Database unavailable:** Simulate dependency failure. Tests bulkhead isolation, circuit breakers
- **Cache cleared:** Empty cache or invalidate entries. Tests cold-cache performance, stampede conditions
- **Disk errors:** Inject I/O errors. Tests error handling in persistence layer
- **Clock skew:** System clock changes. Tests time-sensitive code (distributed consensus, token expiration)

## Game Days: Large-Scale Chaos Simulations

A **game day** is a coordinated, large-scale simulation involving multiple teams:

- **Scenario:** Define realistic, complex failure (region outage, cascading service failures, data corruption)
- **Teams:** On-call engineers, SREs, platform team, sometimes management
- **Duration:** 2-4 hours
- **Rules:** Teams respond as if incident were real (page people, follow runbooks, coordinate)
- **Injected chaos:** Facilitators inject failures as scenario unfolds (not predetermined)
- **Observation:** Facilitators watch for gaps in playbooks, unclear responsibilities, tooling issues
- **Debrief:** Post-game analysis, action items, process improvements

**Benefits:** Discovers coordination gaps, tests runbooks, builds team muscle memory, reveals gaps in tooling.

**Risks:** Significant operational lift, requires strong organizational buy-in, can backfire if perceived as "just for fun."

## Measuring Resilience

After chaos experiment, how do you know if resilience improved?

### Metrics

- **MTTR (Mean Time To Recovery):** How long does it take to return to steady state after fault injection?
- **Severity of degradation:** How much did metrics deviate during chaos? (100% failure vs. 5% error rate?)
- **Blast radius:** How many users/requests affected?
- **Automated vs. manual recovery:** Did system self-heal or require human intervention?

### Trends Over Time

- Run same experiment every week
- Track if MTTR decreases (system getting better at recovering)
- Track if degradation severity decreases (systems becoming more resilient)
- If trends flatten, resilience not improving; deeper issues need fixing

## Organizational Adoption Challenges

### Fear of Production Impact

*"What if chaos breaks production?"*

**Mitigation:** Start tiny (0.1% of traffic, 30 seconds duration, monitored closely). Build confidence gradually.

### Blame Culture

*"SREs should chaos test, when incident happens"*

Chaos testing is only valuable if failures trigger learning, not blame. Team must be psychologically safe to find weaknesses.

### Integration with Development

Developers often resist chaos (feels like attack on their code). Better framing: "Let's find weaknesses before customers do."

### Automation Overhead

Chaos at scale requires robust automation (experiment scheduling, metrics collection, rollback). Can't be manual hobby.

## When Chaos Engineering Works

- Organizations comfortable with production testing and calculated risk
- Systems with enough redundancy that small-scope chaos is tolerable
- Team capability to interpret chaos results and implement fixes
- Culture of proactive resilience (not just reactive incident response)

## When NOT to Start

- Systems with no redundancy (single database, single instance)
- Highly regulated environments (healthcare, finance) with strict change control
- Teams with low incident response capability (results won't drive improvement)
- Immature observability (can't tell if chaos is working without metrics)

## See Also

- Resilience patterns: [architecture-resilience.md](architecture-resilience.md)
- Advanced testing strategies: [testing-advanced-patterns.md](testing-advanced-patterns.md)
- Incident response and SRE practices: [sre-incident-management.md](sre-incident-management.md)
- Observability and metrics: [devops-observability-patterns.md](devops-observability-patterns.md)
- Kubernetes operational patterns: [devops-kubernetes.md](devops-kubernetes.md)