# Infrastructure — Chaos Engineering Tools: Experiments, Blast Radius & CI/CD Integration

## Overview

Chaos engineering tools automate failure injection at scale: inject latency, kill instances, partition networks, corrupt data. Rather than manual `kill -9` or network cable unplugging, tools codify experiments, schedule recurring runs, measure steady-state drift, and integrate with CI/CD pipelines. This note covers tool categories (chaos orchestration platforms, network-layer tools, cloud-native chaos), experiment design patterns, blast radius management, and CI/CD integration.

## Chaos Tool Categories

### Orchestration Platforms (Cloud-Native)

Managed platforms for defining and running chaos experiments on Kubernetes or cloud infrastructure.

#### Chaos Mesh

CNCF project; runs as Kubernetes operator. Injects faults at pod/network/I/O layer via eBPF.

**Capabilities**:
- Pod crash, CPU throttle, memory pressure, I/O latency/loss
- Network partition, latency injection, bandwidth limit
- Kernel panic, file descriptor exhaust
- DNS chaos (resolution delays, NXDOMAIN)
- Scheduled recurring experiments
- Web dashboard for experiment visualization

**Experiment DSL** (YAML-based CRD):
```yaml
apiVersion: chaos-mesh.org/v1alpha1
kind: NetworkChaos
metadata:
  name: partition-svc-a
spec:
  action: partition
  duration: 5m
  selector:
    namespaces:
      - production
    labelSelectors:
      app: service-a
  direction: both
  target:
    namespaces:
      - production
    labelSelectors:
      app: service-b
```

**Ecosystem**: Integrates with Prometheus for metrics scraping, supports webhook notifications on failure.

#### LitmusChaos

CNCF project; Kubernetes-native; focuses on application and infrastructure chaos.

**Capabilities**:
- Pod deletion, network delay/packet loss
- Node drain, resource exhaustion
- Application-level chaos (custom test scripts)
- Scenario workflows (chain multiple experiments)
- GitOps-driven experiment management via ArgoCD

**Differentiation from Chaos Mesh**: Stronger emphasis on application probes (custom health checks), chaos workflows (multi-step experiments), and community experiment hub.

#### Gremlin

Commercial SaaS platform; supports Kubernetes, servers, Lambda, databases.

**Capabilities**:
- Compute chaos: CPU, memory, disk, process kill
- Network: latency, bandwidth limit, packet loss, partition
- Application-level: HTTP error injection, state manipulation
- Guided mode: Interactive troubleshooting; automatic hypothesis generation
- Audit trail and compliance reporting

**Unique features**: Blast radius controls (% of targets, duration guardrails), integration with PagerDuty/Slack for on-call alerting.

#### ChaosBlade

Open-source; developed by Alibaba; multi-platform (Linux, Docker, Kubernetes, cloud VMs).

**Capabilities**:
- Host-level chaos: CPU, memory, network, disk I/O
- Container and Kubernetes chaos
- Application-level chaos: Java method invocation delay, return value override
- Command-line and daemon modes for easy scripting

**Differentiator**: Lightweight (~20MB binary); runs standalone without Kubernetes operator; good for legacy infrastructure migration scenarios.

### Network-Layer Tools (Lower-Level)

#### tc (Traffic Control) & iptables

Linux kernel networking tools; fine-grained control over packet behavior.

**tc (qdisc, tc filter)**:
```bash
tc qdisc add dev eth0 root netem delay 50ms loss 1%
# Adds 50ms latency and 1% packet loss to all eth0 traffic
```

**Capabilities**:
- Latency, jitter, packet loss, reordering
- Bandwidth rate limiting
- Queue depth manipulation (tail drop, RED)

**Use case**: Testing on bare metal or VMs; no container/Kubernetes overhead.

**Tradeoff**: Affects entire host or specific interface; not scoped to single process or container by default.

**iptables**:
```bash
iptables -A OUTPUT -d 10.0.0.5 -j DROP  # Block all outbound to 10.0.0.5
iptables -I OUTPUT -p tcp --dport 3306 -j REJECT --reject-with tcp-reset  # Reject MySQL
```

**Capabilities**: Packet filtering, connection reset, route manipulation.

**Modern alternative**: `nftables` (newer kernel interface, more flexible).

#### Toxiproxy

Standalone proxy (Go); intercepts TCP/UDP traffic, injects latency/corruption/drop.

**Model**: Client → Toxiproxy → downstream service. Toxiproxy rules inject faults per connection or rule.

**Use case**: Local testing, integration tests, staging; not production-grade at scale.

```yaml
# Toxiproxy config: inject 100ms latency to Redis
toxics:
  - name: latency
    type: latency
    attributes:
      latency: 100
      jitter: 10
```

#### Pumba

Docker chaos tool; kills containers, pauses, throttles CPU/network.

```bash
pumba kill --interval 10s 'service-a'  # Kill service-a container every 10 seconds
pumba pause --duration 30s 'service-b'  # Pause service-b for 30 seconds
```

**Advantage**: Simple HTTP API and CLI; pairs with Kubernetes for node-level chaos via node SSH.

### Cloud-Specific Chaos Injection

#### AWS Fault Injection Simulator (FIS)

Managed service; injects faults on EC2, RDS, Lambda, ECS/EKS, networks.

```json
{
  "targets": {
    "Instances": {
      "resourceType": "ec2:instance",
      "resourceTags": {"Environment": "production"},
      "selectionMode": "COUNT",
      "selectionValue": "2"
    }
  },
  "actions": {
    "CPUStress": {
      "actionId": "aws:ec2:cpu-stress",
      "parameters": {"cpuUtilization": "90"},
      "targets": {"Instances": 1}
    }
  },
  "stopConditions": [
    {
      "source": "CloudWatch",
      "value": "arn:aws:cloudwatch:...:alarm/HighErrorRate"
    }
  ]
}
```

**Advantage**: Integrates with AWS-native monitoring; automatic rollback on alarm.

## Experiment Design Patterns

### Steady State Hypothesis

Define metrics characterizing normal operation:
- Latency (p50, p99)
- Error rate (4xx, 5xx)
- Throughput (requests/sec)
- Resource utilization (CPU, memory)
- Business metrics (conversion, revenue per minute)

**Hypothesis**: "When load balancer A fails, latency stays < 500ms and error rate < 1%."

### Blast Radius Constraints

Limit experiment scope to avoid full outage:

- **Canary approach**: Kill 1 instance in 50-instance cluster (2% blast radius)
- **Traffic percentage**: Inject latency to 5% of requests only
- **Duration**: Run 1-10 minutes; auto-rollback on violation
- **Monitoring**: Have on-call watching real-time metrics

**Anti-pattern**: Running unmonitored experiments in production; no rollback criteria.

### Progressive Hypothesis Testing

Start small; increase blast radius as confidence grows:

1. Day 1: Kill 1 pod in 10-pod deployment (10% blast radius)
2. Day 3: Kill 3 pods (30% blast radius)
3. Day 7: Kill 5 pods + introduce 200ms latency to one service
4. Day 14: Run full dependency chaos (multiple services failing)

### Game Day / Runbook Validation

Structured chaos experiment with incident commander:

- Hypothesis: "When database primary fails, auto-failover completes in < 30s with zero data loss"
- Experiment: Terminate primary instance during business hours
- Expected outcome: Databases failover, alerts fire, incident response activates
- Validation: Verify incident response runbook executed correctly

## Chaos in CI/CD Pipelines

### Pre-Deployment Chaos Testing

Run chaos experiment on staging cluster before production deployment:

```yaml
# CI/CD stage
deploy-staging:
  stage: test-staging
  script:
    - deploy-app.sh staging
    - run-chaos staging --duration 5m --blast-radius 30%
    - if $? != 0; then abort deployment; fi
    - promote-to-production.sh
```

**Benefit**: Catches regressions (new deployment less resilient than old) before they hit production.

### Continuous Resilience Validation

Recurring chaos jobs outside CI/CD; scheduled runs (e.g., 2am, 4am) during low-traffic windows.

```yaml
# Kubernetes CronJob: run chaos experiments daily
kind: CronJob
metadata:
  name: daily-chaos
spec:
  schedule: "0 2 * * *"  # 2am daily
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: chaos
            image: chaos-mesh:v1.0
            command: ["chaos-mesh", "--scenario", "dependency-failure"]
```

### Metrics Aggregation & Alerting

Chaos platforms export experiment results (pass/fail, metric variance) to observability systems:

- Prometheus: Scrape `chaos_experiment_passed` gauge
- CloudWatch: FIS logs to EventBridge; trigger Lambda for analysis
- Datadog/New Relic: Custom events for experiment runs

## Anti-Patterns & Pitfalls

### Over-Aggressive Blast Radius

Running experiments that take down production (e.g., killing all instances, partitioning entire network). Chaos should stress but not catastrophe.

**Mitigation**: Start at 5% and increase after observing success; require runbooks for blast radius > 30%.

### No Rollback Criteria

Experiment runs forever; takes down production unexpectedly. 

**Mitigation**: Set explicit steady-state thresholds; abort automatically if violated.

### Experiments That Don't Change Behavior

Team runs chaos, everything passes, but system hasn't been tested. Common with synthetic workloads (low traffic).

**Mitigation**: Use production traffic; validate experiment actually impacted system (check latency distribution changed).

### Experiment Fatigue

Teams stop reading chaos results after 20 experiments; become immune to alerts.

**Mitigation**: Rotate experiment design; focus on business outcomes (conversion drop, revenue impact); make findings actionable.

## See Also

- [Chaos Engineering Fundamentals](devops-chaos-engineering.md) — Principles, steady state hypothesis, resilience testing
- [SRE Chaos in Practice](sre-chaos-practice.md) — Organizational adoption, incident response, cultural challenges
- [Infrastructure Observability](devops-observability-patterns.md) — Metrics collection during chaos