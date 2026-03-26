# Database Backup and Recovery — Strategies, PITR, WAL Archiving, and RTO/RPO

## Overview

**Backup** is a copy of database state at a point in time. **Recovery** is restoring from that backup after data loss or corruption. Recovery strategy directly determines **RTO** (Recovery Time Objective—how fast you need to be operational) and **RPO** (Recovery Point Objective—acceptable data loss window).

Backup strategy depends on:
- Data volume (affects backup time)
- Acceptable downtime during recovery
- Acceptable data loss (RTO/RPO)
- Storage bandwidth and cost
- Regulatory requirements (audit trails, retention)

---

## Backup Types

### Logical Backup

**Mechanism:** Export data as SQL statements or text (e.g., `SELECT ... INTO OUTFILE`, `mysqldump`, `pg_dump`).

**Characteristics:**
- **Storage format:** SQL statements or delimited text. Human-readable, portable across database versions.
- **Size:** Often larger than physical backup (schema info, text representation).
- **Speed:** Slower. Must scan tables, serialize to text.
- **Point-in-time restore:** Only if combined with binary logs/WAL (see WAL archiving).
- **Incremental backup:** Difficult. Full export each time.

**When used:**
- Migration between database versions (SQL is portable).
- Small databases where speed isn't critical.
- Automated regular backups for auditing (schema + sample data).

**Tools:**
- **PostgreSQL:** `pg_dump`, `pg_dumpall`
- **MySQL:** `mysqldump`, `Percona XtraBackup` (logical mode)
- **Limitations:** Cannot restore to a point-in-time without WAL replay.

### Physical Backup

**Mechanism:** Copy database files directly (pages, data files, indices). Binary representation of disk state.

**Characteristics:**
- **Storage format:** Raw database files (tablespaces, indices, WAL). Not portable across versions.
- **Size:** Compact. Only modified pages.
- **Speed:** Fast. File I/O at disk speed.
- **Point-in-time restore:** Native. Replay WAL from backup time to any commit point.
- **Incremental backup:** Possible. Copy only changed blocks since last backup.

**When used:**
- Large databases requiring fast recovery.
- PITR is mandatory (regulatory, audit trail).
- Continuous archiving for disaster recovery.

**Tools:**
- **PostgreSQL:** `pg_basebackup`, `pgBackRest`, `WAL-G`
- **MySQL:** `Percona XtraBackup` (physical mode), `MySQL Enterprise Backup`
- **Snapshots:** Cloud providers (EBS, GCP Persistent Disks) can snapshot storage directly.

---

## Continuous Archiving and WAL

### Write-Ahead Logging (WAL)

All SQL changes are written to a **WAL (Write-Ahead Log)** before applying to data files. WAL segments (e.g., 16MB in PostgreSQL) form a sequential chain.

**Why:** WAL is durable and sequential; faster to write than random page updates. Recovery replays WAL to reconstruct committed state.

### WAL Archiving

**Purpose:** Preserve WAL segments beyond their normal retention on-disk, enabling recovery to any point in time.

**Mechanism:**
1. Database generates WAL segments as transactions execute.
2. When a segment is full, database sends it to an **archive location** (S3, NFS, local path).
3. Recovery replays archived WAL segments to reconstruct data from a base backup.

**Configuration (PostgreSQL):**
```sql
-- postgresql.conf
wal_level = replica           -- Enable WAL archiving
archive_mode = on             -- Enable archiving
archive_command = 'test ! -f /archive/%f && cp %p /archive/%f'
archive_timeout = 300         -- Archive segment every 5 min if not full
```

### PITR (Point-in-Time Recovery)

**Mechanism:** Restore from base backup, then replay archived WAL segments up to a specific timestamp or transaction ID.

**Steps:**
1. Identify recovery target (timestamp, transaction ID, or named savepoint).
2. Restore base backup (full physical copy).
3. Replay WAL segments from backup time to recovery target.
4. Stop replay at target; database is at desired point-in-time state.

**Granularity:** WAL segments in PostgreSQL are ~16MB. Recovery accuracy ±1 transaction (typically milliseconds).

**Recovery time:** Depends on data volume and WAL replay speed. Can take hours for very large databases.

**Example (PostgreSQL):**
```
recovery_target_timeline = 'latest'
recovery_target_name = 'before_bad_delete'  -- Named savepoint
```

---

## Backup Infrastructure

### pgBackRest (PostgreSQL)

**Comprehensive backup solution for PostgreSQL.**

**Features:**
- **Parallel backup/restore:** Multi-threaded for speed.
- **Incremental backup:** Copy only modified blocks since last full backup.
- **Differential backup:** Full backup is baseline; subsequent backups capture differences.
- **Compression:** Built-in compression reduces storage.
- **Remote storage:** S3, Azure Blob, Google Cloud Storage.
- **Archival of WAL:** Automatic WAL archiving to remote storage.
- **PITR:** Restore to any point via archived WAL.

**Workflow:**
```bash
pgbackrest backup                    # Full backup
pgbackrest backup --type=incr        # Incremental
pgbackrest restore --target-name=savepoint_1  # PITR
```

**Benefits:**
- Reliable, tested in production (used by major PostgreSQL deployments).
- Handles versioning challenges (backup in version N, restore in version M).

### Barman (PostgreSQL)

**Backup Archiving Recovery Manager.**

**Features:**
- **Centralized backup server:** Runs on separate machine, manages backups of multiple PostgreSQL instances.
- **WAL archiving:** Configurable destination (local disk, remote NFS, S3).
- **PITR:** Restore to any point-in-time.
- **Incremental backup:** If supported by backup tool.
- **Monitoring:** Health checks, backup verification.

**Architecture:** Barman server archiving WAL from multiple production databases.

**When used:** Large deployments with many PostgreSQL instances. Centralized backup administration.

### MySQL Enterprise Backup and Percona XtraBackup

**Backup tools for MySQL/MariaDB.**

**Percona XtraBackup:**
- **Physical backup:** Copies `.ibd` files directly while maintaining consistency.
- **Incremental:** Only changed blocks in subsequent backups.
- **No locking:** Online backup without global locks (InnoDB).
- **Compression:** Optional.

**Workflow:**
```bash
xtrabackup --backup --target-dir=/backups/full
xtrabackup --prepare --target-dir=/backups/full
```

---

## Snapshots and Cloud-Native Backups

### Snapshot-Based Backups

**Mechanism:** Cloud storage provider (AWS EBS, GCP Persistent Disks) creates point-in-time snapshots of storage volumes.

**Characteristics:**
- **Speed:** Instant (incremental from previous snapshot).
- **Storage:** Only changed blocks stored incrementally.
- **Recovery:** Quick (minutes to re-attach volume).
- **Consistency:** Snapshot is crash-consistent (data as if database crashed and recovered). Application must handle recovery.

**Gotchas:**
- **Crash-consistent, not application-consistent:** Uncommitted transactions may be in the snapshot. Database must recover on restore.
- **Encryption:** Snapshots may not be encrypted if volume isn't.
- **Cross-region:** Snapshots are region-specific; cross-region copy may be slow.

**When used:** Cloud-first deployments, large data volumes, fast disaster recovery.

### Automated Managed Backups (RDS, Cloud SQL)

Cloud providers offer integrated backup:
- **Automatic daily backups:** Full backup + incremental retention (typically 7–35 days).
- **PITR:** Restore to any point within retention window.
- **Multi-region:** Backup copied to secondary region for cross-region failover.
- **Encryption:** Automatic at-rest encryption.

**Tradeoff:** Convenience vs. cost (storage charges for backup retention).

---

## RTO and RPO Trade-Offs

### Recovery Point Objective (RPO)

**Definition:** Maximum acceptable data loss. How recent does recovered data need to be?

**Examples:**
- **RPO = 0:** Zero data loss. Require synchronous replication or PITR on every commit (very costly).
- **RPO = 1 hour:** Acceptable to lose up to 1 hour of transactions.
- **RPO = 24 hours:** Daily backups sufficient; acceptable if failure occurs early after backup.

**Achieving low RPO:**
- Frequent backups (hourly, continuous WAL archiving).
- Synchronous replication to a second site.
- Cost: Storage, network, and compute for archiving and replication.

### Recovery Time Objective (RTO)

**Definition:** Maximum acceptable downtime. How quickly must the database be operational after a failure?

**Examples:**
- **RTO = 5 min:** Automatic failover + promote replica.
- **RTO = 1 hour:** Manual failover + restore from backup.
- **RTO = 24 hours:** Cold restore, acceptable downtime.

**Achieving low RTO:**
- Replicas (hot standby) for instant failover.
- Automated failover orchestration (Patroni, etcd).
- Pre-configured recovery infrastructure.
- Cost: Multiple running replicas, orchestration complexity.

### Matrix of Strategies

| Strategy | RPO | RTO | Cost | Complexity |
|----------|-----|-----|------|------------|
| Daily full backup | 24h | 8–24h | Low | Low |
| Daily backup + continuous WAL archiving | 1–5m | 1–4h | Medium | Medium |
| Synchronous replication | ~0 | 5–10m | High | High |
| Async replication + PITR | 5–30m | 5–15m | High | High |
| RPO=0 sync + RTO<5m auto failover | 0 | 5min | Very High | Very High |

---

## Backup Validation and Testing

**Critical:** A backup is worthless if recovery fails. Regular testing is mandatory.

### Validation Techniques

1. **Restore a copy to spare hardware:** Periodically restore a backup to a test environment. Verify data and application.
2. **Checksum verification:** Backup tool computes checksum; verify at recovery time.
3. **Point-in-time recovery test:** Restore backup, replay WAL to a specific point, verify correctness.
4. **Synthetic restore:** Backup tool performs dry-run restore without actually writing to disk (some tools support this).

### Testing Frequency

- **Monthly (minimum):** Full restore test to spare hardware.
- **Weekly:** Checksum verification and recovery metadata checks.
- **On major changes:** Schema migration, replication topology change.

---

## Operational Patterns

### Scheduled vs. Continuous

**Scheduled backups:** Full backup daily at off-hours, incremental backups every few hours.
- **Pros:** Predictable, simple scheduling.
- **Cons:** Gap between backups is unrecoverable data.

**Continuous archiving:** WAL segments archived as produced. PITR available for all time.
- **Pros:** Minimal RPO (only up to last WAL segment).
- **Cons:** Continuous storage I/O, archive management overhead.

### Backup Retention Policy

Retain backups for:
- **Short-term (7–30 days):** Quick recovery, low storage cost.
- **Medium-term (1–3 months):** Compliance, historical audit.
- **Long-term (1–7 years):** Regulatory, archival (usually cheaper storage, e.g., Glacier).

Automate deletion of old backups to control costs.

### Disaster Recovery Runbook

1. **Detect failure:** Alert on connection loss to primary.
2. **Assess impact:** Which data lost? How much recovery needed?
3. **Choose recovery strategy:** PITR, replica failover, or full restore?
4. **Execute recovery:** Promote replica, restore from backup, or both.
5. **Verify integrity:** Run integrity checks, sample data queries.
6. **Apply pending changes:** If replicating during recovery, catch up replicas.
7. **Communicate:** Notify stakeholders, document incident.

---

## Anti-Patterns

1. **Backup without testing:** Never tested a restore = backup is likely corrupted.
2. **Storing backups next to production:** Single failure (ransomware, fire) destroys both backup and data.
3. **Relying on replication alone:** Replicas propagate corruption and deletes. Need immutable backups.
4. **RPO and RTO undefined:** Leads to under-provisioned recovery infrastructure; unexpected downtime.
5. **Backing up during peak load:** Slows production database. Use replicas for backup source.

---

## When NOT to Backup

- **Development databases:** Recreate from schema + test data.
- **Read-only cache layers:** Rebuilt from source on failure.
- **Highly transient data:** Logs that are naturally ephemeral.

See also: cloud-disaster-recovery, database-replication-patterns, database-internals-storage.