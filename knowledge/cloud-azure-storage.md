# Azure Storage Concepts — Blobs, Disks, Files & Tables

Azure's storage portfolio addresses the fundamental tension in distributed data management: balancing durability, performance, accessibility, and cost. Each storage service occupies a distinct niche — blob storage for unstructured objects, managed disks for VM-attached block storage, file shares for protocol-compatible access, and Data Lake for analytics-optimized hierarchical data. Understanding the economic model behind storage tiers is as important as understanding the technical characteristics.

## Storage Account Foundations

A storage account is the top-level namespace and management boundary for Azure storage services. It defines the redundancy level, access tier defaults, networking rules, and encryption configuration that apply to resources within it.

### Account Types

| Account Type         | Services Supported        | Performance Tiers             | Use Cases                                  |
| -------------------- | ------------------------- | ----------------------------- | ------------------------------------------ |
| General-purpose v2   | Blob, File, Queue, Table  | Standard (HDD), Premium (SSD) | Most workloads — the default choice        |
| BlockBlobStorage     | Block blobs, append blobs | Premium only                  | High-transaction blob workloads            |
| FileStorage          | Azure Files only          | Premium only                  | Enterprise file shares needing low latency |
| BlobStorage (legacy) | Blob only                 | Standard only                 | Legacy — GPv2 supersedes this              |

General-purpose v2 accounts support all features and tiering options. The specialized account types exist for workloads where premium performance is required for a specific service.

### Redundancy Options

Azure storage redundancy controls how data is replicated for durability and availability:

| Redundancy | Copies | Geographic Scope                    | Durability (annual)           | Use Case Context                     |
| ---------- | ------ | ----------------------------------- | ----------------------------- | ------------------------------------ |
| LRS        | 3      | Single datacenter                   | 99.999999999% (11 nines)      | Cost-sensitive, reconstructable data |
| ZRS        | 3      | Across availability zones           | 99.9999999999% (12 nines)     | Zone-level failure protection        |
| GRS        | 6      | Primary region + secondary region   | 99.99999999999999% (16 nines) | Regional disaster protection         |
| GZRS       | 6      | Zones in primary + secondary region | 99.99999999999999% (16 nines) | Highest protection level             |

**Read-access variants** (RA-GRS, RA-GZRS): Enable read access to the secondary region replica. Without the RA- prefix, secondary data exists for failover but cannot be read during normal operations. The secondary region data may lag behind the primary (eventual consistency with RPO typically under 15 minutes).

**Cost trade-offs**: Each step up in redundancy increases storage costs. LRS to ZRS adds roughly 25%. ZRS to GZRS can double the cost. The decision involves weighing data criticality against budget — transient or easily regenerated data may warrant LRS, while business-critical records may justify GZRS.

**Failover considerations**: Account failover to a secondary region involves potential data loss (writes since last sync) and converts the account to LRS in the new primary region. Re-establishing geo-redundancy requires reconfiguration after failover completes.

## Azure Blob Storage

Blob storage organizes data into a flat namespace (within containers) optimized for massive-scale unstructured data — images, documents, backups, logs, media, data lake files.

### Structure

```
Storage Account
  └── Container (logical grouping, access policy boundary)
       ├── Block blob (composed of blocks, optimized for upload/download)
       ├── Append blob (optimized for append operations — logs, audit trails)
       └── Page blob (random read/write, backing store for VM disks)
```

Container-level access policies define whether blobs are publicly accessible (anonymous read) or require authentication. Production configurations almost universally restrict public access and use authenticated access patterns.

### Block Blob Operations

Block blobs support staged uploads — uploading blocks independently and committing them as a blob in a single atomic operation. This enables:

- **Parallel upload**: Multiple blocks uploaded simultaneously for throughput.
- **Retry granularity**: Failed blocks can be retried without re-uploading the entire blob.
- **Large objects**: Blobs up to 190.7 TiB via block composition (up to 50,000 blocks of up to ~4000 MiB each).

For smaller objects, single-shot `Put Blob` operations are simpler and sufficient.

### Access Tiers

The tier model reflects the economic reality that storage cost and access cost have an inverse relationship:

| Tier    | Storage Cost   | Access Cost           | Retrieval Latency   | Minimum Retention | Context                                   |
| ------- | -------------- | --------------------- | ------------------- | ----------------- | ----------------------------------------- |
| Hot     | Highest        | Lowest                | Milliseconds        | None              | Frequently accessed data                  |
| Cool    | ~40-50% of Hot | Higher per-operation  | Milliseconds        | 30 days           | Infrequent access, immediate availability |
| Cold    | ~60-70% of Hot | Higher than Cool      | Milliseconds        | 90 days           | Rare access, still online                 |
| Archive | ~80-90% of Hot | Highest per-operation | Hours (rehydration) | 180 days          | Long-term retention, rare retrieval       |

**Early deletion charges**: Deleting or moving a blob before its minimum retention period incurs a charge equivalent to storing it for the full period. This penalizes tier selection errors — placing volatile data in Archive is expensive if it needs modification or deletion within 180 days.

**Rehydration from Archive**: Archive-tier blobs are offline and require rehydration (to Hot or Cool) before reading. Standard rehydration takes up to 15 hours; high-priority rehydration completes faster at higher cost. This delay makes Archive inappropriate for data that might need urgent access.

**Tier selection heuristics**: Access frequency thresholds depend on specific pricing, but roughly:

- Hot: Accessed more than once per month.
- Cool: Accessed a few times per quarter.
- Cold: Accessed once or twice per year.
- Archive: Accessed less than once per year, if ever.

### Lifecycle Management Policies

Automated rules transition blobs between tiers or delete them based on age, access time, or creation date:

```json
{
  "rules": [
    {
      "name": "age-based-tiering",
      "type": "Lifecycle",
      "definition": {
        "actions": {
          "baseBlob": {
            "tierToCool": { "daysAfterModificationGreaterThan": 30 },
            "tierToCold": { "daysAfterModificationGreaterThan": 90 },
            "tierToArchive": { "daysAfterModificationGreaterThan": 365 },
            "delete": { "daysAfterModificationGreaterThan": 2555 }
          },
          "snapshot": {
            "delete": { "daysAfterCreationGreaterThan": 90 }
          }
        },
        "filters": {
          "blobTypes": ["blockBlob"],
          "prefixMatch": ["logs/", "backups/"]
        }
      }
    }
  ]
}
```

Lifecycle policies embody data governance decisions. The distinction between `daysAfterModificationGreaterThan`, `daysAfterCreationGreaterThan`, and `daysAfterLastAccessTimeGreaterThan` matters — modification-based rules suit data that's written once and read occasionally; access-based rules suit data with unpredictable access patterns.

**Access time tracking**: Requires explicit enablement and adds a small per-operation overhead. Without it, policies can only use creation and modification timestamps.

### Blob Versioning and Soft Delete

- **Versioning**: Maintains previous versions of blobs automatically. Each overwrite or delete creates a new version. Useful for audit trails and accidental overwrite protection, but version accumulation increases storage costs.
- **Soft delete**: Retains deleted blobs (and containers) for a configurable retention period. Acts as a recycle bin — protection against accidental deletion at the cost of continued storage charges during retention.
- **Snapshots**: Point-in-time read-only copies of a blob. Billed for delta storage (blocks that differ from the base blob).

These features interact — enabling both versioning and soft delete provides layered protection but multiplies storage consumption if not managed with lifecycle policies.

## Azure Managed Disks

Managed Disks abstract the underlying storage account management for VM-attached block storage. The choice among disk types reflects the performance-cost spectrum:

| Disk Type      | IOPS (max) | Throughput (max) | Latency         | Pricing Model                            | Context                                   |
| -------------- | ---------- | ---------------- | --------------- | ---------------------------------------- | ----------------------------------------- |
| Ultra Disk     | 160,000    | 4,000 MB/s       | Sub-millisecond | Provisioned IOPS + throughput + capacity | Tier-1 databases, SAP HANA                |
| Premium SSD v2 | 80,000     | 1,200 MB/s       | Sub-millisecond | Independent IOPS + throughput + capacity | Flexible high-performance                 |
| Premium SSD    | 20,000     | 900 MB/s         | Single-digit ms | Size-based (IOPS scale with size)        | Production databases, transaction systems |
| Standard SSD   | 6,000      | 750 MB/s         | Single-digit ms | Size-based                               | Web servers, dev/test, light databases    |
| Standard HDD   | 2,000      | 500 MB/s         | Tens of ms      | Size-based                               | Backups, infrequent access, archival      |

### Performance Characteristics

**Premium SSD and Standard SSD** tie IOPS and throughput to disk size — larger disks get more performance. This creates situations where teams provision larger disks than needed solely for IOPS, paying for unused capacity.

**Premium SSD v2** decouples these dimensions — IOPS, throughput, and capacity are provisioned independently. This granularity enables right-sizing without the coupled-scaling constraint, though the pricing model is more complex.

**Ultra Disks** offer the highest performance with independently configurable IOPS and throughput, plus the ability to adjust performance dynamically without detaching the disk. The premium pricing limits their practicality to workloads that genuinely require extreme I/O.

### Disk Bursting

Standard SSD and smaller Premium SSD disks support bursting — temporarily exceeding baseline IOPS and throughput using accumulated credits. Similar to B-series VM CPU credits:

- Credits accumulate during periods below baseline performance.
- Credits are spent during burst periods exceeding baseline.
- Credit depletion drops performance to baseline until credits regenerate.

Disk bursting matters for workloads with intermittent I/O spikes — boot sequences, periodic batch processing, deployment operations. Sustained high I/O requires a disk tier that provides the needed baseline performance.

### Disk Encryption

- **Server-side encryption (SSE)**: Enabled by default for all managed disks. Platform-managed keys (default) or customer-managed keys via Azure Key Vault.
- **Azure Disk Encryption (ADE)**: OS-level encryption using BitLocker (Windows) or dm-crypt (Linux). Encrypts the disk from the VM's perspective.
- **Encryption at host**: Encrypts data on the VM host before it reaches storage, covering temp disks and OS/data disk caches.

These layers address different threat models — SSE protects data at rest in storage infrastructure; ADE protects against scenarios where disk images are accessed outside the VM context; encryption at host covers the cache and temp disk gap.

### Shared Disks

Managed disks can be attached to multiple VMs simultaneously for clustered workloads (Windows Failover Clustering, Linux Pacemaker). This requires application-level coordination — the disk provides shared block storage, not a distributed filesystem. Without proper cluster software managing concurrent access, data corruption results.

## Azure Files

Azure Files provides fully managed file shares accessible via SMB (Server Message Block) and NFS (Network File System) protocols.

### Protocol Selection

| Protocol | OS Compatibility      | Authentication                  | Use Case Context                                |
| -------- | --------------------- | ------------------------------- | ----------------------------------------------- |
| SMB 3.x  | Windows, Linux, macOS | AD DS, Azure AD DS, storage key | Lift-and-shift Windows workloads, shared config |
| NFS 4.1  | Linux                 | Network-based (no user auth)    | Linux workloads, HPC, container shared storage  |

SMB shares support identity-based authentication through Active Directory, enabling permission management consistent with on-premises file server patterns. NFS shares rely on network-level access control (VNet, private endpoints) rather than user-level authentication.

### Performance Tiers

| Tier                  | Media | IOPS          | Throughput      | Latency         | Context                                       |
| --------------------- | ----- | ------------- | --------------- | --------------- | --------------------------------------------- |
| Premium               | SSD   | Up to 100,000 | Up to 10 GiB/s  | Single-digit ms | Databases, latency-sensitive workloads        |
| Transaction Optimized | HDD   | Up to 10,000  | Up to 300 MiB/s | Variable        | Transaction-heavy, moderate latency tolerance |
| Hot                   | HDD   | Up to 10,000  | Up to 300 MiB/s | Variable        | General-purpose team shares                   |
| Cool                  | HDD   | Up to 10,000  | Up to 300 MiB/s | Variable        | Archival, infrequently accessed shares        |

Premium file shares are provisioned (pay for capacity allocated), while standard tiers are pay-as-you-go (pay for capacity used). Provisioned model suits predictable workloads; pay-as-you-go suits variable or growing datasets.

### Azure File Sync

Azure File Sync extends Azure Files to on-premises Windows servers, enabling:

- **Cloud tiering**: Infrequently accessed files replaced with pointers on the local server; transparently fetched from Azure on access. Reduces local storage needs while maintaining a full namespace view.
- **Multi-site sync**: Multiple servers sync with the same Azure file share, providing distributed caching with centralized storage.
- **Backup integration**: Cloud snapshots back up the centralized data without per-server backup infrastructure.

Cloud tiering decisions balance local cache hit rates against storage savings. Aggressive tiering saves disk space but increases access latency for cache misses.

## Azure Data Lake Storage Gen2

Data Lake Storage Gen2 combines blob storage economics with a hierarchical filesystem namespace, Purpose-built for analytics workloads.

### Hierarchical Namespace

Standard blob storage uses a flat namespace — the "/" in `container/path/to/file` is part of the blob name, not a directory structure. Hierarchical namespace makes directories real objects:

- **Rename operations**: Renaming a directory is atomic and O(1), not O(n) where n is the number of blobs with matching prefixes.
- **POSIX-like permissions**: ACLs on directories and files, supporting fine-grained access control for multi-tenant analytics.
- **Atomic directory operations**: Delete, move, and permission changes on directories are atomic.

These properties matter significantly for analytics frameworks that perform frequent directory-level operations — partitioned data layouts, ETL output staging, and job-level atomic commits.

### Dual Access Patterns

Data Lake Gen2 supports both Blob API and ABFS (Azure Blob File System) driver access:

- **Blob API**: Standard REST interface, compatible with existing blob tooling.
- **ABFS driver**: Hadoop-compatible filesystem driver optimized for analytics workloads — used by Spark, Databricks, HDInsight, and Synapse.

This dual nature allows the same data to be accessed by traditional applications (via blob interface) and analytics engines (via ABFS) without duplication.

### Access Control Model

Data Lake Gen2 layers POSIX ACLs on top of Azure RBAC:

- **RBAC roles**: Coarse-grained access at the storage account or container level.
- **ACLs**: Fine-grained POSIX-style permissions (read, write, execute) on individual files and directories.
- **Default ACLs**: Inherited by newly created child items, reducing per-object permission management.

The interaction between RBAC and ACLs can be subtle — RBAC superuser roles bypass ACL checks entirely, which may violate least-privilege expectations if not carefully managed. Organizations with strict data governance requirements typically use ACLs for data-level access and RBAC for management operations.

## Shared Access Signatures (SAS)

SAS tokens provide delegated, time-limited, scoped access to storage resources without sharing account keys.

### SAS Types

| Type                | Scope                                     | Signed By            | Revocation                                        |
| ------------------- | ----------------------------------------- | -------------------- | ------------------------------------------------- |
| Account SAS         | Account-level operations                  | Account key          | Rotate account key or delete stored access policy |
| Service SAS         | Single service (blob, file, queue, table) | Account key          | Rotate key or delete stored access policy         |
| User Delegation SAS | Blob/Data Lake                            | Azure AD credentials | Revoke user's Azure AD token                      |

**User delegation SAS** represents the most secure option — signed by Azure AD credentials rather than account keys, eliminating the risk of key exposure. However, it's limited to blob and Data Lake storage operations.

### Stored Access Policies

Stored access policies define reusable constraint sets (permissions, time bounds) that SAS tokens reference. The key benefit: policy modification or deletion immediately affects all SAS tokens referencing it, providing a revocation mechanism without rotating account keys.

```
Stored access policy: "readonly-30days"
  Permissions: Read, List
  Start: (set at token creation)
  Expiry: +30 days from start

SAS token references this policy → revoking the policy invalidates all tokens
```

Without stored access policies, revoking a SAS token requires rotating the account key, which invalidates ALL tokens and connections using that key.

## Immutable Storage

Immutable blob storage enforces write-once-read-many (WORM) policies for compliance and data integrity:

- **Time-based retention**: Blobs cannot be modified or deleted for a specified duration (up to the enterprise retention maximum). The policy can be locked, after which even administrators cannot shorten the retention period.
- **Legal hold**: Blobs protected indefinitely until the hold is explicitly cleared. Used for litigation preservation and regulatory holds.

Immutability applies at the container level (in current implementations, also at version level). Design decisions:

- Separate containers for immutable and mutable data.
- Retention periods should reflect actual regulatory requirements — excessive retention increases storage costs.
- Locked policies are irreversible — testing with unlocked policies before committing avoids expensive mistakes.

## CDN Integration

Azure CDN caches blob storage content at edge locations globally, reducing latency for geographically distributed consumers.

Key configuration decisions:

- **Caching rules**: Time-to-live (TTL) per path pattern. Static assets may cache aggressively (days/weeks); dynamic content requires shorter TTL or cache-busting strategies.
- **Origin configuration**: CDN origin points to the storage account's blob endpoint. Private origins use managed identity for authentication.
- **Purge strategy**: Invalidating cached content when the origin changes — immediate purge for critical updates vs TTL-based expiration for routine changes.
- **Custom domains and HTTPS**: CDN endpoints support custom domains with managed TLS certificates.

CDN economics work when read-to-write ratios are high and consumers are geographically distributed. For data primarily accessed from a single region, direct blob access may be simpler and comparably performant.

## Data Transfer Patterns

### AzCopy

AzCopy is a command-line utility optimized for high-throughput data transfer to and from Azure Storage:

- **Parallel transfers**: Automatically parallelizes operations across multiple connections.
- **Resume capability**: Failed transfers resume from the last successful operation rather than restarting.
- **Sync mode**: Synchronize local directories with blob containers, transferring only changed files.
- **Benchmarking**: Built-in benchmark mode for measuring achievable throughput.

For transfers exceeding what network bandwidth cost-effectively supports, Azure Data Box (physical devices shipped to the datacenter) provides an offline alternative — terabytes to petabytes via physical media.

### Transfer Method Selection

| Data Volume   | Urgency          | Method                       | Context                         |
| ------------- | ---------------- | ---------------------------- | ------------------------------- |
| < 10 GB       | Any              | AzCopy, portal upload, SDK   | Routine operations              |
| 10 GB - 10 TB | Moderate         | AzCopy over ExpressRoute/VPN | Bulk migration, regular sync    |
| 10 TB - 1 PB  | Weeks acceptable | Azure Data Box               | Initial migration, offline bulk |
| > 1 PB        | Weeks acceptable | Azure Data Box Heavy         | Massive migrations              |
| Ongoing sync  | Continuous       | AzCopy sync, Azure File Sync | Hybrid architectures            |

Network-based transfer time depends on available bandwidth. Estimating transfer duration before committing to a method avoids surprises — 100 TB over a 1 Gbps link takes roughly 9 days of sustained transfer.

## The Economics of Storage Architecture

Storage architecture decisions compound economically over time. Key dynamics:

- **Tier selection inertia**: Data placed in the wrong tier accumulates misallocated cost daily. Lifecycle policies automate correction but only if defined proactively.
- **Redundancy cost scaling**: The percentage premium for higher redundancy applies to every byte stored. For large datasets, even small percentage differences translate to significant absolute costs.
- **Egress costs**: Reading data from storage incurs egress charges, especially across regions. Architectures that minimize cross-region data movement reduce ongoing operational costs.
- **Transaction costs**: Per-operation pricing means access patterns matter as much as data volume. Millions of small reads cost more than fewer large reads for the same data.

Effective storage architecture treats data as having a lifecycle — frequently accessed when fresh, decreasingly accessed over time, eventually retained only for compliance. Aligning storage tiers with this lifecycle minimizes total cost of ownership without sacrificing access when needed. Organizations that implement lifecycle management from the start avoid the painful and expensive retroactive classification of petabytes of undifferentiated data sitting in Hot tier.

## Monitoring and Diagnostics

Storage metrics and diagnostics inform optimization:

- **Capacity metrics**: Track growth rates for capacity planning.
- **Transaction metrics**: Identify hot containers or unexpected access patterns.
- **Latency metrics**: Detect performance degradation before it affects applications.
- **Access logs**: Audit who accessed what, when — relevant for compliance and security forensics.

Storage Analytics logs capture detailed per-request information but generate significant log volume themselves. Sampling strategies and log retention policies prevent monitoring costs from rivaling the storage costs being monitored.
