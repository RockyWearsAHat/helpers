# Backup and Disaster Recovery — Policy, Strategy, Testing, and Runbooks

## Overview

Backup and disaster recovery (DR) are distinct disciplines often conflated. **Backup** = copy of data at a point-in-time, enabling recovery from data loss or corruption. **Disaster Recovery** = plan to restore full service after catastrophic failure (hardware failure, data center loss, sabotage). Backup is prerequisite for DR; DR without backup is incomplete.

Outages stem from three sources: (1) data loss or corruption (backup solves), (2) service unavailability while data intact (replication + failover solves), (3) both (backup + DR together). Well-designed systems address all three.

## The 3-2-1 Backup Rule

**3-2-1 rule**: Keep three copies of data, on two different media types, with one copy offsite.

$$\text{copies} = 3, \quad \text{media types} = 2, \quad \text{offsite} = 1$$

**Example implementation:**
- Copy 1: Production database (live)
- Copy 2: Backup on local NAS (same data center; fast restore)
- Copy 3: Replicated to cloud storage in different region (slow but geographically safe)

**Rationale:**
- **Three copies**: Tolerates one failure. If primary fails, backup available. If backup fails, replica available.
- **Two media types**: Protects against media-specific failures (failed NAS doesn't threaten that data, still in cloud).
- **One offsite**: Protects against site disaster (fire, flood, regional outage).

**Cost formula:**
$$\text{cost} = \text{storage cost} \times 3 + \text{network cost (replication)} + \text{operational cost (verification)}$$

Smaller in practice; cloud storage is cheap. Trade-off: storage cost vs. recovery speed (local backup faster than cloud).

## Retention and Rotation: Grandfather-Father-Son (GFS)

**GFS** = retention schedule for rotating backup generations: daily (son), weekly (father), monthly (grandfather).

**Example schedule:**
```
Daily backups: Keep 7 (one week)
Weekly backups: Keep 4 (one month core + overlap)
Monthly backups: Keep 12 (one year)
```

Retention timeline:
```
Day 1: Daily (newest)
Day 2: Daily
...
Day 7: Daily → Promote to weekly at end of week. Age out daily before week 1.
Day 14: Weekly (day 7 backup)
...
Day 30: Monthly (day 1 backup of month)
```

**Storage math:**
$$\text{Total backups} = 7 \text{ daily} + 4 \text{ weekly} + 12 \text{ monthly} = 23$$

For 100GB database: $23 \times 100GB = 2.3TB$ storage (manageable).

**Deletion policy**: Delete daily after 7 days, weekly after 4 weeks, monthly after 12 months. Automated via retention tags (S3 lifecycle policies, Backup Vault retention rules).

**Failure mode**: Accidental deletion recoverable from daily (< 7 days ago). Older data requires weekly or monthly backup. Corruption in week 1 requires pre-corruption backup (outside 7-day window); need monthly backup.

## Immutable Backups (Append-Only)

**Immutable backup** = once written, cannot be deleted or modified for retention period. Protects against ransomware and accidental deletion.

**Implementation:**
- Cloud storage: S3 Object Lock (write-once-read-many, WORM), GCS retention locks
- Backup software: Commvault, NetBackup, Veeam support immutability settings

**Example S3 configuration:**
```json
{
  "Bucket": "backups",
  "Rules": [
    {
      "Status": "Enabled",
      "ObjectLockEnabled": "Enabled",
      "RetentionMode": "GOVERNANCE",
      "RetentionDays": 90
    }
  ]
}
```

GOVERNANCE mode: Admin can override retention with special permissions (recovery scenario). COMPLIANCE mode: No override (stronger but less flexible).

**Protection profile:**
- Ransomware deletes live data; backup still available (retention lock prevents deletion)
- Attacker gains cloud credentials; still can't delete backups (COMPLIANCE mode)
- Accidental deletion; app deletes backups; still protected (retention lock)

**Trade-off:** Higher cost (immutable storage more expensive); reduced operational flexibility (can't quickly adjust retention); strong guarantee.

When to use: Mission-critical data (financial records, production databases). Not every backup; cost prohibitive.

## Cloud Backup Strategies

### Cross-Region Replication

Backup stored in primary region + replicated to secondary region asynchronously.

**Architecture:**
```
Data → Backup (Region 1) → Replicate → Backup (Region 2) [eventual consistency]
```

Replication latency: seconds to minutes (depends on data size, network, replication settings).

**Failure scenario:** Region 1 destroyed (earthquake, data center fire). Backup still available in Region 2. Recovery: Point to Region 2 backup, restore to Region 3 or new Region 1 instance.

**Cost:** Storage cost × 2 + egress cost (replication). Cross-region replication typically 1-5% data transfer cost.

**Verification:** Periodically test restore from secondary region to ensure backup integrity and restore procedure works.

### Cross-Account Backup

Backups stored in separate AWS/Azure/GCP account (different organization/team). Prevents compromised application account from deleting backups.

**Architecture:**
```
Primary Account: Production Data
Primary Account: Backup Agent (creates backups)
Backup Account: Central Backup Storage (separate credentials, RBAC)
```

**Access control:**
- Application team: Full access to production account, zero access to backup account
- Backup team: Read-only access to backups; restore via support request
- Security team: Audit access to backup account

**Threat model:** Attacker compromises production account; can't delete backups (stored in uncompromised account). Even if attacker creates delete policies in backup account, separate account credentials required.

**Operational overhead:** More accounts to manage; cross-account IAM roles; testing restore requires cross-account permissions.

Common in heavily regulated environments (financial services, healthcare).

### Backup Encryption

Backups encrypted at-rest (in storage) and in-transit (during replication).

**At-rest:**
- Cloud storage encryption (S3 server-side encryption, GCS default)
- Key management: Customer-managed keys (CMK) in KMS, or provider-managed keys

**In-transit:**
- HTTPS/TLS for replication
- Backup software option: Encrypt before sending (end-to-end encryption)

**Key storage:**
- Provider-managed keys: Simpler; less control
- Customer-managed keys (CMK): More control; must manage key lifecycle (rotation, revocation)

**Shared responsibility:** Cloud provider manages infrastructure security; customer manages key access.

## RTO and RPO Planning

**RTO** (Recovery Time Objective): How long to restore service after failure.

$$\text{RTO} = (\text{Time to detect failure}) + (\text{Time to decide action}) + (\text{Time to restore})$$

**RPO** (Recovery Point Objective): Maximum acceptable data loss.

$$\text{RPO} = (\text{Backup frequency}) + (\text{Time since last backup})$$

For hourly backups, worst case: 1 hour of data loss (if failure occurs just before backup).

### Examples

**Scenario 1: Web Server Failure**
- Detect: 5 min (health check alerts)
- Decide: 1 min (auto-failover triggered)
- Restore: 30 sec (switch to replica)
- RTO = 6.5 min

- Backup frequency: N/A (stateless; no data loss)
- RPO = 0 (no data lost)

**Scenario 2: Database Corruption**
- Detect: 2 hours (alert on data quality check)
- Decide: 30 min (investigate, confirm data corruption)
- Restore: 1 hour (restore from backup, replay from WAL if available)
- RTO = 3.5 hours

- Backup frequency: 1 hour
- RPO = 1 hour (maybe 30 min if WAL available since backup)

**Scenario 3: Data Center Loss**
- Detect: 5 min (all monitoring in DC fails)
- Decide: 10 min (activate failover)
- Restore: 30 min (DNS/switch traffic, restore from backup in secondary region)
- RTO = 45 min

- Backup frequency: Continuous replication
- RPO = 5-10 sec (replication lag or last transaction group)

### Optimization

To **decrease RTO**:
- Reduce detect time: Aggressive health checks (probe every 10s instead of 60s)
- Reduce decide time: Automated failover (no human decision)
- Reduce restore time: Warm standby (replica ready to go) vs cold standby (backup storage only)

To **decrease RPO**:
- Increase backup frequency: Hourly → every 15 min → continuous
- Use write-ahead logging (WAL): Capture all writes; can replay from last backup + WAL to near zero RPO
- Use replication: Real-time replica has near-zero RPO (sync replication = zero RPO; async = lag)

**Trade-off table:**

| Strategy | RTO | RPO | Cost | Operational Effort |
|----------|-----|-----|------|-------------------|
| Backups only | Hours | Hours | Low | Low |
| Warm standby | Minutes | Minutes | Med | Med |
| Hot standby + replication | Seconds | Seconds | High | High |
| Active-Active | Seconds | Near-zero | Very High | Very High |

Most applications choose warm standby (RTO 5-30 min, RPO 15-60 min) as optimal balance.

## High Availability vs. Disaster Recovery

**HA** (High Availability): Multiple instances of same service; automatic failover. Same data center or region. Solves: single instance failure, temporary unavailability.

**DR** (Disaster Recovery): Separate instance in different region; manual or automated failover. Solves: data center loss, catastrophic failure.

**Typical architecture:**
- **Primary Region**: Active-Active (multiple instances, load balanced)
- **Secondary Region**: Warm standby (ready to activate; lagging slightly behind primary)
- **Backup**: Cross-region copy, immutable

**RTO/RPO trade-off:**
- HA: RTO 30 seconds, RPO ~5 seconds
- DR: RTO 5-30 minutes, RPO 1 hour (depends on replication)

## Active-Active vs. Active-Passive Deployments

### Active-Passive (Warm Standby)

Primary handles all traffic. Passive replica synced; ready but not serving requests.

```
Primary (Active)
  ├─ Serving all traffic
  ├─ Replicating to Passive
Passive (Standby)
  ├─ Receiving replication
  ├─ Ready to activate
```

**Failover:** Detect primary failure → Activate passive (switch DNS/load balancer) → Passive becomes primary → Create new passive.

**RTO:** Minutes (requires failover decision + DNS propagation).

**RPO:** Depends on replication latency (typically seconds to minutes).

**Cost:** Single primary; replica is low-cost standby (not serving requests).

**Orchestration:** Manually trigger failover or use automatic failover coordinator (e.g., Patroni for PostgreSQL, etcd-driven failover).

### Active-Active (Bidirectional)

Both instances serve traffic simultaneously. Both receive writes; replicate to each other (multi-primary replication).

```
Primary A (Active)
  ├─ Serving traffic region 1
  ├─ Replicating writes to B
Primary B (Active)
  ├─ Serving traffic region 2
  ├─ Replicating writes to A
```

**Failover:** Detect primary failure → Surviving instance continues serving (no activation needed).

**RTO:** Seconds (no orchestration; already serving traffic).

**RPO:** Near-zero if sync replication; latency-bound if async.

**Cost:** Both instances active; both expensive.

**Complexity:** Write conflicts (both instances modified same row). Resolution: conflict-free data types (CRDTs), application-level conflict detection, or conflict resolution rules.

**Best for:** Geographically distributed services (EU + US, each serving local region).

**Worst for:** Single-region deployments (no benefit; just double cost).

## Runbook Design and Testing

**Runbook** = documented procedure for recovery. Example:

```
# Database Recovery Runbook

## Symptoms
- Application unable to connect to database
- Dashboard shows no data
- Error logs: "Connection timeout after 30s"

## Detection
- Alert triggered: db_connection_failures > 10 per min
- Automated response: Health check confirms database down

## Initial Assessment (5 min)
1. SSH to database primary: `ssh db-prod-1`
2. Check disk space: `df -h` (expect > 50% free)
3. Check process: `systemctl status postgresql` (expect running)
4. Check logs: `tail -100 /var/log/postgresql/postgresql.log`

## If database process crashed:
1. Attempt restart: `systemctl restart postgresql`
2. Wait 30s
3. Verify: `systemctl status postgresql` (expect active)
4. Database automatically rejoins replication (already synced replica)
5. Resume traffic

## If database corruption detected:
1. Promote replica: `patroni ctl switchover`
2. Affected instance becomes standby
3. Restore from backup if needed
4. Recovery RTO: ~30 seconds

## Root cause analysis (After recovery):
1. Check for OOM (out of memory): `dmesg | grep -i "out of memory"`
2. Check for disk full: Previous `df -h` results
3. file bugs; schedule root cause postmortem
```

### Runbook Testing

Runbook must be tested regularly (at least quarterly) to ensure:
1. Steps are accurate (instructions don't fail)
2. Recovery actually restores service (not just starts process)
3. RTO/RPO estimates are realistic
4. Team knows the procedure (familiarity reduces response time)

**Chaos engineering**: Deliberately inject failures; trigger runbook. Measure actual RTO vs. target.

Example test:
```
Day 1: Shutdown database primary
        Measure time to detection (Goal: < 5 min)
        Measure time to failover (Goal: < 15 min)
        Exercise runbook steps; log any errors or missing instructions
        Restore primary; rejoin replica

Result: RTO = 12 minutes (< target of 15 min) ✓
        RPO = 2 minutes (< target of 5 min) ✓
```

Quarterly or post-incident, repeat test. Track metrics over time.

## Compliance and Audit

Regulatory frameworks (SOC 2, ISO 27001, HIPAA, PCI-DSS) require:

1. **Documentation**: Backup and DR strategy documented and approved
2. **Testing**: Annual DR test; results logged
3. **Metrics**: RTO/RPO defined and achievable
4. **Change control**: Backup policy changes reviewed and approved
5. **Retention**: Backups retained per policy; deletion audit-logged

**Evidence for auditors:** Backup software logs, restored backup verification logs, runbook tests, incident response documentation.

## Common Pitfalls

**Untested backups**: "We have backups but never tested restore." Restoration fails when needed due to format change, corrupted data, missing metadata.

**No restore verification**: Backups stored; never checked for integrity or size anomalies. Discover backup is corrupted only when needed.

**RPO/RTO not documented**: Team doesn't know expected recovery time. Expectations misaligned; frustration during incident.

**Backup and DR conflated**: "We have backups so we're covered for DR." Backups are good for data loss; don't solve service unavailability due to infrastructure failure.

**Key rotation forgotten**: Backup encrypted with key that's now inaccessible (key deleted, KMS service down). Backup unrecoverable.

**Replication lag ignored**: Replica lagging 30 minutes behind primary. Failover loses 30 minutes of data (greater than anticipated RPO).

## See Also

[cloud-disaster-recovery.md](cloud-disaster-recovery.md), [database-backup-recovery.md](database-backup-recovery.md), [infrastructure-gitops-patterns.md](infrastructure-gitops-patterns.md), [database-replication-patterns.md](database-replication-patterns.md)