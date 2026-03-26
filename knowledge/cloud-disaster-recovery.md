# Cloud Disaster Recovery — Patterns, RPO/RTO, Replication & Runbooks

## Overview

**Disaster recovery** (DR) is the set of practices and technologies enabling recovery from catastrophic failures: data center outages, region-wide disasters (earthquakes, hurricanes), cyber attacks, or whole-provider outages. DR planning defines **RTO** (Recovery Time Objective; how long can you be down?) and **RPO** (Recovery Point Objective; how much data loss is acceptable?), then implements patterns to meet those targets. DR is not backup alone — backup is one component of a comprehensive recovery strategy.

## RPO & RTO Definitions

### Recovery Time Objective (RTO)
**RTO** is the maximum acceptable downtime after a disaster. Measured in minutes, hours, or days.

- **RTO = 15 minutes**: System must be restored and serving traffic within 15 minutes of failure detection. Examples: ecommerce checkouts, payment processing (business loss >$1M/hour downtime).
- **RTO = 4 hours**: Acceptable for internal tools, non-critical SaaS. No business immediately affected; operations slowed.
- **RTO = 24 hours**: Acceptable for archival, infrequently accessed data. Can restart from backup next day.

**Hidden costs of low RTO**:
- Infrastructure cost: 4-hour RTO requires 2 active regions; 15-minute RTO requires hot standby + automated failover (3-10x cost).
- Operational overhead: Automated failover is brittle; orchestration failures cause cascading outages. Manual failover is slow and error-prone.

### Recovery Point Objective (RPO)
**RPO** is the maximum acceptable data loss after recovery. Measured in time or transaction count.

- **RPO = 5 minutes**: Losing 5 minutes of data is acceptable. Must have backups/replicas updated every 5 minutes.
- **RPO = zero (synchronous replication)**: No data loss; every write must be replicated to standby before acknowledging to client. High latency cost.
- **RPO = 24 hours**: Acceptable data loss; daily backups sufficient.

**Tradeoff**: Lower RPO costs more (synchronous replication adds latency; more frequent backups consume storage). Balance recovery urgency against operational complexity.

## DR Patterns & Architectures

### Backup & Restore

**Pattern**: Regular backups (daily, hourly, per-transaction) stored separately from production. On disaster, restore from latest backup.

**Implementation**:
```
Production system writes data
    ↓
Backup agent (AWS Backup, Velero)
    ↓
Backup storage (S3, Azure Blob, Glacier)
    ↓
[Disaster strikes]
    ↓
Manual restore from backup
    ↓
Spin up new infrastructure
    ↓
Restore data from backup
    ↓
Service online (RTO: 2-4 hours)
```

**RTO**: 2-4 hours (time to detect disaster + restore from backup + spin up infra + DNS update). Assumes manual OR partially automated restore process.

**RPO**: RPO matches backup frequency. Hourly backups = max 1 hour data loss. Per-transaction backups (transaction logs) = near-zero data loss.

**Cost**: Low. Backup storage is cheap ($0.01-0.03/GB/month for cold storage).

**Suitable for**: Most applications. Non-critical systems where 2-4 hour downtime is tolerable.

**Failure modes**:
- Backup corruption: Ransomware or silent corruption infects backup. Test restore regularly (yearly full restore test).
- Restore latency: Restoring terabytes of data from slow storage (Glacier) takes 12+ hours. Use faster (more expensive) tiers.
- Forget to backup: Unmanaged databases or hand-provisioned servers often lack backups. Enforce with cloud policies (AWS Backup Plans via Config rules).

### Pilot Light

**Pattern**: Keep a minimal standby system running in secondary region. On disaster, scale it up.

**Implementation**:
```
Production (Region A):
  - Full infrastructure (LBs, auto-scaling, databases)
  - Heavy traffic

Standby (Region B):
  - Minimal infrastructure (single server, database replica on minimal compute)
  - Receives read-only copies of data; no traffic
  - Cost: 5-10% of production

[Disaster in Region A]
  ↓
Detect failure
  ↓
Point DNS to Region B
  ↓
Scale up Region B (20 min to spin instances, warm up caches)
  ↓
Service online (RTO: 20-40 min)
```

**RTO**: 15-40 minutes. Faster than backup & restore (infrastructure exists); slower than active-active (scale-up latency).

**RPO**: Depends on replication frequency. Pilot light typically uses asynchronous replication (RPO: 1-10 minutes).

**Cost**: 10-20% extra for standby region.

**Suitable for**: Applications where 20-minute downtime is acceptable; cost-conscious teams.

**Failure modes**:
- Replica lags during high traffic: Standby's replica can't keep up; when failover happens, recent transactions are lost.
- Scale-up not tested: Assuming standby can auto-scale is dangerous; test failover monthly.
- DNS propagation: 15-40 minute RTO assumes aggressive DNS TTL (60 sec); conservative TTLs (3600 sec) increase RTO 10x.

### Warm Standby

**Pattern**: Active standby infrastructure in secondary region, receiving live traffic (e.g., read-only requests) or 10-50% of write traffic.

**Implementation**:
```
Region A (Primary):
  - Receives 100% of updates
  - Active-active reads or async replication

Region B (Standby):
  - Receives 50% of reads, 10% of updates (or asynchronous replicas)
  - Can handle traffic increase if A fails
  - Costs 50-80% of main region

[Region A fails]
  ↓
Route all traffic to Region B (already running, caches warm)
  ↓
Service online (RTO: 1-5 min) — faster than pilot light
```

**RTO**: 1-5 minutes (DNS failover only; no scale-up needed).

**RPO**: 1-5 minutes (async replication to warm standby).

**Cost**: 50-80% extra (maintain 2 regions at significant capacity).

**Suitable for**: Critical systems where <5 min downtime is needed; higher budget.

**Failure modes**:
- Split-brain: Both regions accept writes; conflict resolution is complex (timestamps, vector clocks, application logic).
- Cascading failures: Region B already at 50% capacity; if traffic spike during failover, Region B is overloaded.

### Multi-Site Active-Active

**Pattern**: Multiple regions actively serve production traffic simultaneously. All regions are equivalent; any region failure is tolerable.

**Implementation**:
```
Region A: Serves traffic
         Writes to global database (Dynamo, CockroachDB, PostgreSQL with multi-master)
         
Region B: Serves traffic (same as A)
         All writes replicate bi-directionally
         
Region C: (optional) Same setup, more redundancy

[Region A fails]
  ↓
Traffic rerouted to B, C
  ↓
Service unaffected (RTO: near-zero)
```

**RTO**: Near-zero (milliseconds to detect and reroute traffic).

**RPO**: Near-zero if using synchronous replication; milliseconds if async.

**Cost**: 2-3x (run 2-3 full regions in parallel).

**Suitable for**: Mission-critical systems (exchanges, emergency services, gambling platforms) where even 1-minute downtime costs millions.

**Failure modes**:
- Replication conflicts: If Region A and B both serve writes, they can diverge (user creates account in A, another user creates same account in B). Resolution requires:
  - Optimistic replication: Accept conflicts; resolve via application logic or manual intervention.
  - Pessimistic replication: Use consensus (Raft, Paxos) to ensure all regions agree before committing. Adds latency.
  - Commutative operations: Design writes so order doesn't matter (e.g., append-only logs, CRDTs).
- Network partition: If A↔B link fails, both regions think the other is dead; both continue accepting writes, conflict explodes.

## Cross-Region Replication Patterns

### Asynchronous Replication
Primary writes locally; replication agent ships data to standby after commit.

```
User sends update
  ↓
Primary database commits locally (acknowledged to user)
  ↓
Replication background job copies change to standby (100 ms later)
  ↓
Standby applies change
```

**Latency to user**: No added latency (write is local).

**RPO**: Time between primary write and replication batch. Typically 1-60 seconds (configurable). If primary crashes before replication, recent writes are lost.

**Cost**: Low (replication is background I/O, not blocking user request).

**Drawback**: Data loss possible. Use when data loss of seconds is tolerable.

### Synchronous Replication
Primary waits for standby to confirm receipt before committing.

```
User sends update
  ↓
Primary routes to standby (network round-trip, 50-200 ms)
  ↓
Standby applies change, confirms
  ↓
Primary commits, acknowledges to user
```

**Latency to user**: +100-300 ms (round-trip time to standby).

**RPO**: Zero (every write is confirmed on both systems).

**Cost**: Higher (network latency added to every write; reduced throughput).

**Suitable for**: Critical data (financial records, medical records, legal documents).

**Drawback**: Network partition makes replication impossible; must choose between availability (ignore standby, lose synchronous guarantee) or consistency (wait forever for standby, system hangs). Many systems degrade to async on network failures.

### Cross-Region Streams & Event Logs
Instead of row-by-row replication, capture all events (writes, updates, deletes) in logs; stream logs to remote region.

```
Event log (S3, Kinesis)
  ↓
Stream replication (Lambda, managed replication service)
  ↓
Remote region applies events (exactly-once guarantee)
```

**Advantage**: Decoupled; remote region can replay events even if logs existed offline.

**Drawback**: Eventual consistency; remote region lags during high write volume.

**Example**: AWS S3 cross-region replication (CRR) replicates objects via event-driven Lambda. Latency: 1-60 seconds.

## Chaos Testing for DR

### Failure Injection
Regularly test DR procedures **in production** (or production-like staging) to catch brittleness:

1. **Kill primary region compute**: Take down a single server → verify failover works. Then scale: kill a rack, then a zone, then an entire region.
2. **Simulate network partition**: Drop traffic between regions (using iptables, AWS VPC mirroring). Measure failover latency.
3. **Corrupt database replica**: Inject silent corruption; test backup restoration.
4. **Run in degraded state**: Reduce resources (half the servers) and measure if SLA is still met.

**Frequency**: Quarterly full chaos tests; monthly small-scale tests.

**Metrics to track**:
- Time to detect failure (MTTD)
- Time to failover (MTTR if failover is manual; MTTR if automated)
- Data loss measured in transactions or time span
- User impact (% of requests affected during failover)

**Success**: Teams become confident in DR. Failures become routine exercises, not panic.

## Runbook Automation

### Manual Runbooks vs. Automation
A runbook is a step-by-step disaster recovery procedure: "If primary region is down, do X, Y, Z."

**Manual runbook**:
1. Detect outage (PagerDuty alert)
2. SSH to backup region
3. Update DNS records
4. Scale up auto-scaling groups
5. Verify health checks
6. Notify users

**Time to execute**: 10-30 minutes (error-prone, slow).

### Automated Runbooks
Encode procedures as code (Lambda functions, Kubernetes operators, Terraform modules). Trigger automatically on detection or with one-click execution.

```python
def failover_to_secondary_region():
  if primary_region_down():
    # Update DNS
    update_dns('example.com', secondary_region_ip)
    # Scale up
    scale_auto_group('secondary-asg', min=100, desired=200)
    # Wait for health
    wait_for_healthy(secondary_region, timeout=3min)
    # Notify
    notify_slack('#incidents', 'Failover complete')
```

**Time to execute**: 1-5 minutes (automated, reliable).

**Caveats**:
- Automation can fail silently (DNS update fails; system thinks it's done, but failover incomplete).
- Automation is brittle (hardcoded region names, fragile health checks).
- Runaway automation: Auto-failover triggered by false positive; both regions down from cascading failures.

**Best practice**: Semi-automated runbooks. Automation detects failure + alerts; human clicks "Execute failover" button; automation runs. Gives human override while maintaining speed.

## Compliance & Regulatory Requirements

### Financial Services
- **RTO < 4 hours** for critical systems (regulatory requirement, not customer SLA).
- **RPO < 1 day** (daily backups + transaction logs acceptable).
- **Annual DR test**: Simulate failure; document recovery time.
- **Audit trail**: Every transaction must be logged and recoverable for 7 years.

### Healthcare (HIPAA)
- **Backup every 24 hours** (minimum); more frequently for critical patient data.
- **Encryption during replication** (in-transit and at-rest).
- **Testing**: Document all DR tests; store tests in patient records.
- **RTO**: No explicit requirement; depends on patient impact (e.g., active surgery requires <1 hr RTO; administrative systems can be 8+ hrs).

### Data Residency (GDPR, CCPA)
- Backups and replicas must respect geographic constraints.
- GDPR: EU data cannot be replicated to US without privacy shield (post-Schrems II, complex).
- China: Data must not leave China; no single-cloud strategy with US provider will work.

## Comparison of Patterns

| Pattern | RTO | RPO | Cost | Complexity | Suitable For |
| --- | --- | --- | --- | --- | --- |
| Backup & Restore | 2-4 hours | 1-60 min | Very Low (backup storage) | Low | Non-critical systems |
| Pilot Light | 15-40 min | 1-10 min | Low (minimal standby) | Medium | Growing systems |
| Warm Standby | 1-5 min | 1-5 min | Medium (50% extra) | High | Critical systems |
| Active-Active | Near-zero | Near-zero | High (2-3x cost) | Very High | Mission-critical |

## See Also

- **architecture-resilience.md** — General resilience patterns beyond DR
- **distributed-replication.md** — Replication consistency models (CAP, eventual consistency)
- **cloud-multi-cloud.md** — DR across multiple cloud providers
- **cloud-aws-databases.md** — AWS multi-AZ, RDS cross-region replication
- **devops-cicd-patterns.md** — Automated deployment as runbook infrastructure