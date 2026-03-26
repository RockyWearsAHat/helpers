# GCP Storage Concepts — Object, Block, File & Archive

## Storage Model Taxonomy

Storage services in cloud environments fall into three fundamental models, each reflecting a different abstraction over persistent data:

| Model          | Conceptual Unit                         | Access Pattern                    | GCP Service                |
| -------------- | --------------------------------------- | --------------------------------- | -------------------------- |
| Object storage | Opaque blobs with metadata              | HTTP API (GET/PUT/DELETE by key)  | Cloud Storage              |
| Block storage  | Fixed-size blocks on a virtual disk     | OS-level mount, filesystem driver | Persistent Disk, Local SSD |
| File storage   | Hierarchical files via network protocol | NFS/SMB mount                     | Filestore                  |

These models serve different workload needs. Object storage scales horizontally for unstructured data. Block storage provides low-latency random I/O for databases and operating systems. File storage provides shared POSIX-compliant access across multiple compute instances.

## Cloud Storage — Object Storage

### Buckets and Objects

Cloud Storage organizes data into **buckets** (globally unique namespace containers) and **objects** (the actual data blobs, up to 5 TB each). Despite the flat namespace, object names containing `/` create the illusion of hierarchy in tooling and console UIs.

Key bucket-level properties:

- **Location type** — region (single), dual-region (two specific regions), multi-region (continent-level). Affects availability, latency, and cost.
- **Default storage class** — applied to objects that do not specify their own class at upload time.
- **Public access prevention** — organization policy or bucket-level enforcement that blocks ACLs granting public access.
- **Uniform bucket-level access** — when enabled, disables per-object ACLs and relies exclusively on IAM. Simplifies access control at the cost of per-object granularity.

### Storage Classes

Storage classes reflect a trade-off between access cost and storage cost:

| Class    | Min Storage Duration | Storage Cost | Retrieval Cost           | Access Cost (ops) | Suited For                         |
| -------- | -------------------- | ------------ | ------------------------ | ----------------- | ---------------------------------- |
| Standard | None                 | Highest      | None                     | Lowest            | Frequently accessed data           |
| Nearline | 30 days              | Lower        | Per-GB retrieval fee     | Moderate          | Monthly access patterns            |
| Coldline | 90 days              | Lower still  | Higher per-GB retrieval  | Higher            | Quarterly access patterns          |
| Archive  | 365 days             | Lowest       | Highest per-GB retrieval | Highest           | Yearly access, compliance archives |

The minimum storage duration is a billing construct — objects deleted before the minimum period incur an early deletion charge equivalent to storing the object for the full minimum period. This creates a cost optimization consideration: moving data to colder classes too aggressively can increase costs if access patterns are misjudged.

Storage class applies per-object, not per-bucket (the bucket's default class is just a default). A single bucket can contain objects across all classes.

### Lifecycle Management

Lifecycle rules automate storage class transitions and object deletion based on configurable conditions:

```
Lifecycle Rule Examples (conceptual):
─────────────────────────────────────
Condition: age > 30 days, storage class = STANDARD
Action: SetStorageClass NEARLINE

Condition: age > 90 days, storage class = NEARLINE
Action: SetStorageClass COLDLINE

Condition: age > 365 days, isLive = true
Action: SetStorageClass ARCHIVE

Condition: age > 2555 days (7 years)
Action: Delete

Condition: numNewerVersions > 3
Action: Delete (applies to noncurrent versions)
```

Lifecycle rules evaluate daily (not in real-time). The transition chain Standard → Nearline → Coldline → Archive is one-directional in terms of lifecycle automation — moving objects to warmer classes requires explicit API calls or re-upload.

### Consistency Model

Cloud Storage provides strong global consistency for:

- **Read-after-write** — a successful write (create or overwrite) is immediately visible to subsequent reads from any location
- **Read-after-metadata-update** — metadata changes are immediately consistent
- **Read-after-delete** — deleted objects return 404 immediately
- **Bucket listing** — reflects the latest state

This strong consistency model simplifies application logic compared to eventually consistent systems, where read-after-write could return stale data. Earlier versions of Cloud Storage had eventual consistency for some operations — the evolution toward strong consistency reduced the need for application-level consistency workarounds.

### Access Control

Cloud Storage supports multiple access control mechanisms, which can be layered:

**IAM (Identity and Access Management):**

- Bucket-level and project-level roles (`roles/storage.objectViewer`, `roles/storage.objectCreator`, `roles/storage.admin`, etc.)
- Conditions can scope access by resource name prefix, enabling pseudo-directory-level permissions
- Integrates with organization policies for governance

**Access Control Lists (ACLs):**

- Per-object and per-bucket grants to specific users, groups, or special identifiers (allUsers, allAuthenticatedUsers)
- Finer granularity than IAM but harder to audit and manage at scale
- Disabled when uniform bucket-level access is enabled

**Signed URLs:**

- Time-limited URLs that grant specific access (read, write, delete) without requiring the requester to have GCP credentials
- Generated by a service account's credentials; the URL encodes the permission and expiration
- Useful for enabling direct browser uploads, sharing with external parties, or pre-authenticated CDN origins

**Signed Policy Documents:**

- Constrain uploads via HTML forms — enforce size limits, content types, and key prefixes
- More restrictive than signed URLs for upload scenarios

The general pattern: use IAM for internal service-to-service access, signed URLs for external or time-bounded access, and avoid ACLs in new designs unless per-object granularity is essential.

### Object Composition and Transformation

**Compose operations** combine up to 32 existing objects into a single new object without downloading and re-uploading. This enables parallel upload patterns:

```
Upload strategy for large objects:
1. Split locally into N chunks
2. Upload chunks in parallel as separate objects
3. Compose chunks into final object
4. Delete chunk objects

Benefit: total upload time ≈ time to upload largest chunk
Trade-off: more API operations, temporary storage for chunks
```

**Object versioning** maintains historical versions of objects when overwritten or deleted:

- Every overwrite creates a new live version; previous versions become noncurrent
- Deleted objects become noncurrent (retrievable) rather than permanently removed
- Noncurrent versions accrue storage costs and can be managed by lifecycle rules
- Object holds (event-based and temporary) prevent deletion even by bucket owners — essential for compliance and legal-hold scenarios

### Retention Policies and Object Locks

Retention policies enforce a minimum retention period during which objects cannot be deleted or overwritten:

- **Bucket-level retention policy** — applies a minimum age to all objects
- **Object-level retention** — per-object retention configurations (available with object lock)
- **Bucket lock** — permanently locks the retention policy itself, making it irrevocable. Once locked, the retention period can only be increased, never decreased or removed.

These mechanisms support regulatory compliance (WORM — Write Once Read Many) requirements in industries like finance and healthcare.

## Persistent Disk — Block Storage

Persistent Disks provide network-attached block storage for Compute Engine VMs and GKE nodes. Unlike local storage, Persistent Disks exist independently of VM lifecycle — they persist through VM stops and can be detached and reattached.

### Disk Types

| Type              | Performance Profile | Max IOPS (read)       | Max Throughput (read) | Use Cases                              |
| ----------------- | ------------------- | --------------------- | --------------------- | -------------------------------------- |
| pd-standard (HDD) | Cost-optimized      | ~7,500                | ~1,200 MB/s           | Sequential I/O, cold data, boot disks  |
| pd-balanced (SSD) | Balanced            | ~80,000               | ~1,200 MB/s           | General workloads, databases           |
| pd-ssd            | Performance         | ~100,000              | ~1,200 MB/s           | High-IOPS databases, latency-sensitive |
| pd-extreme        | Highest             | ~120,000 configurable | ~2,400 MB/s           | Mission-critical databases, SAP        |

Performance scales with disk size — larger disks provide higher IOPS and throughput, up to the per-VM limits. A 100 GB pd-ssd provides fewer IOPS than a 1 TB pd-ssd. This creates a tension: provisioning more capacity than needed for storage in order to meet performance requirements, or choosing a higher-performance disk type.

### Multi-Attach and Read-Only Sharing

Persistent Disks can be attached to multiple VMs simultaneously in **read-only mode**, enabling patterns like:

- Shared dataset distribution across a fleet of worker VMs
- Content serving from a common disk image
- Reference data accessible without duplication

Read-write attachment remains single-VM only. Multi-writer block storage (where multiple VMs write to the same disk) requires a distributed filesystem or cluster-aware filesystem layer.

### Snapshots

Snapshots capture the state of a Persistent Disk at a point in time:

- **Incremental** — after the initial full snapshot, subsequent snapshots store only changed blocks
- **Global resource** — snapshots can create disks in any region, enabling cross-region disaster recovery and migration
- **Snapshot schedules** — automated periodic snapshots with configurable retention (daily, hourly, etc.)
- **Consistency** — application-level consistency requires flushing buffers or freezing the filesystem before snapshotting. For databases, this means either stopping writes, using the database's backup mechanism, or leveraging VSS/fsfreeze integration.

Snapshots are stored in Cloud Storage under the hood, with pricing based on the delta between snapshots rather than the full disk size.

### Encryption

All Persistent Disks are encrypted at rest by default:

- **Google-managed encryption keys (default)** — GCP handles all key management transparently
- **Customer-managed encryption keys (CMEK)** — keys stored in Cloud KMS, customer controls key lifecycle (rotation, destruction)
- **Customer-supplied encryption keys (CSEK)** — customer provides the raw key with each API call; GCP never stores the key. Losing the key means permanent data loss.

The encryption hierarchy: data encryption key (DEK) encrypts the disk data; key encryption key (KEK) encrypts the DEK. CMEK and CSEK control the KEK layer.

## Local SSD — Ephemeral High-Performance Storage

Local SSDs are physically attached to the host machine running the VM, providing:

- Very low latency (~100 μs vs ~1-3 ms for Persistent Disk)
- High IOPS (up to 2.4M read IOPS across 24 Local SSD partitions)
- Fixed 375 GB per partition, up to 24 partitions per VM

Critical caveat: Local SSD data is **ephemeral**. Data is lost when:

- The VM is stopped (not just restarted, but stopped)
- The VM is preempted (Spot VMs)
- The host machine experiences a failure
- The VM undergoes live migration (data preserved during live migration, but not guaranteed across host failures)

Local SSDs suit temporary data: scratch space, caches, temporary processing buffers, shuffle storage for distributed data processing. Any data that must survive VM lifecycle events belongs on Persistent Disk or in Cloud Storage.

## Filestore — Managed NFS

Filestore provides fully managed NFS file shares accessible from Compute Engine VMs and GKE clusters. The use case: workloads requiring traditional filesystem semantics with shared access.

### Service Tiers

| Tier       | Capacity Range | Performance                   | Use Case                     |
| ---------- | -------------- | ----------------------------- | ---------------------------- |
| Basic HDD  | 1-63.9 TB      | Lower IOPS, cost-efficient    | File sharing, backups        |
| Basic SSD  | 2.5-63.9 TB    | Higher IOPS                   | Application data             |
| Zonal      | 1-100 TB       | High performance, scalable    | Databases, analytics         |
| Enterprise | 1-10 TB        | Highest reliability, regional | Mission-critical, multi-zone |

Filestore instances are zonal (except Enterprise, which is regional). Multi-zone availability for non-Enterprise tiers requires application-level replication or backup strategies.

### NFS Semantics and Implications

NFS provides POSIX-compliant file access: read, write, seek, lock, directory traversal. This makes Filestore suitable for:

- Legacy applications expecting local filesystem paths
- Shared home directories across VM fleets
- Content management systems requiring file-level access patterns
- GKE workloads mounting shared volumes via ReadWriteMany PersistentVolumeClaims

The trade-off compared to object storage: NFS is a stateful protocol with connection overhead, file locking semantics, and scalability limits. Object storage's stateless HTTP model scales more broadly for new application designs.

## Transfer Service — Data Migration

Moving data into Cloud Storage at scale involves several mechanisms:

| Method                           | Data Scale       | Source                                 | Offline/Online                 |
| -------------------------------- | ---------------- | -------------------------------------- | ------------------------------ |
| gsutil / gcloud CLI              | GB to low TB     | Local machines                         | Online                         |
| Storage Transfer Service         | TB scale         | AWS S3, Azure, HTTP, other GCS buckets | Online                         |
| Transfer Appliance               | Tens of TB to PB | On-premises data centers               | Offline (ship physical device) |
| Transfer Service for on-premises | TB scale         | On-premises POSIX filesystems          | Online (agent-based)           |

The economics of data transfer favor offline methods at very large scales, where network bandwidth cost and transfer time exceed the logistics cost of shipping physical hardware. The breakeven point depends on available bandwidth, urgency, and transfer pricing.

### Transfer Pricing Considerations

- **Ingress** to GCP is generally free — no charge for uploading data to Cloud Storage
- **Egress** from GCP incurs charges — downloading data or transferring between regions costs per-GB
- **Inter-region transfer** within GCP carries lower costs than internet egress but is not free
- **Same-region** access between Cloud Storage and Compute Engine in the same region incurs no network charges

This asymmetric pricing model incentivizes data locality and influences architectural decisions about where to process data relative to where it is stored.

## Choosing Between Storage Models

### Decision Factors

| Factor                     | Object (Cloud Storage)            | Block (Persistent Disk)                 | File (Filestore)        |
| -------------------------- | --------------------------------- | --------------------------------------- | ----------------------- |
| Access pattern             | Key-value (HTTP API)              | Random I/O via filesystem               | Hierarchical via NFS    |
| Latency                    | 10s-100s ms (first byte)          | Sub-ms to low ms                        | Low ms                  |
| Throughput                 | Very high (parallel)              | Scales with disk size/type              | Moderate                |
| Sharing                    | Multi-client via HTTP             | Single-writer, multi-reader (RO attach) | Multi-client read-write |
| Max size                   | 5 TB per object, unlimited bucket | 64 TB per disk                          | 100 TB per instance     |
| Cost                       | Lowest per GB at scale            | Moderate, varies by type                | Higher per GB           |
| Durability                 | 99.999999999% (11 nines) annual   | 99.999% with snapshots                  | Varies by tier          |
| Application changes needed | Must use API or FUSE adapter      | None (block device)                     | Minimal (mount point)   |

### Common Architectural Patterns

**Hot/warm/cold tiering:**

```
User uploads → Standard class (immediate access)
       → 30 days → Nearline (lifecycle rule)
              → 90 days → Coldline (lifecycle rule)
                     → 365 days → Archive (lifecycle rule)
```

**Data lake pattern:**

```
Raw ingestion → Cloud Storage (Standard, regional)
ETL processing: Dataflow/Dataproc reads from GCS, writes results back
Analytics: BigQuery external tables query directly from GCS
ML training: Vertex AI reads training data from GCS
```

**Database storage stack:**

```
Database engine → Persistent Disk (pd-ssd or pd-extreme)
WAL/redo logs → Local SSD (performance) or separate PD (durability)
Backups → Cloud Storage (Nearline or Coldline)
Snapshots → PD snapshots for point-in-time recovery
```

**Shared content serving:**

```
CMS application → Filestore (shared media/assets)
CDN origin → Cloud Storage (public bucket or signed URLs)
Static site → Cloud Storage + Cloud CDN
```

## Edge Caching and CDN Integration

Cloud CDN caches responses from Cloud Storage (and other backends) at Google's edge locations worldwide. The integration model:

- Cloud Storage buckets can serve as **backend buckets** for a Cloud CDN-enabled load balancer
- CDN caching respects Cache-Control headers on objects
- Signed URLs and signed cookies enable CDN-cached content with access control
- Cache invalidation propagates globally but is not instantaneous — eventual consistency for cache clearing

CDN-accelerated Cloud Storage suits read-heavy workloads where the same objects are requested from diverse geographic locations: static websites, media streaming, software distribution, and API response caching.

The cost trade-off: CDN cache hits avoid Cloud Storage operation charges and reduce egress costs (CDN egress is priced differently from standard egress), but CDN introduces its own per-request pricing. For content with high cache-hit ratios, CDN reduces total cost; for content with low reuse, direct Cloud Storage access may be more economical.

## Versioning and Immutability Patterns

Object versioning and retention policies enable several data governance patterns:

**Audit trail:**

- Enable versioning — every modification creates a recoverable version
- Lifecycle rules limit version accumulation (delete versions older than N days or keep only last M versions)

**Compliance archive:**

- Retention policy with bucket lock — objects cannot be deleted until retention period expires
- Event-based holds — freeze objects until an external process releases the hold

**Ransomware resilience:**

- Versioning means overwrites do not destroy previous versions
- Retention policies prevent deletion even by compromised credentials
- Combined with restrictive IAM, provides defense-in-depth

The tension: stronger immutability guarantees increase storage costs (retained versions consume space) and operational complexity (locked policies cannot be reversed). The appropriate level of protection depends on the regulatory environment and risk profile.

## Pricing Architecture and Optimization

Storage pricing has several dimensions that interact:

- **Storage per GB-month** — varies by class, location type, and region
- **Operations** — per-request charges (Class A: mutating operations; Class B: read operations)
- **Network** — egress charges by destination (same-region free, cross-region, internet)
- **Retrieval** — per-GB charge for Nearline, Coldline, Archive reads
- **Early deletion** — charge for objects deleted before minimum storage duration

Optimization strategies involve analyzing access patterns:

- Objects accessed daily → Standard class, same-region as compute
- Objects accessed sporadically → Nearline with lifecycle transition
- Compliance archives → Archive class with retention policy
- Large datasets accessed in bursts → consider retrieval costs vs. keeping in Standard
- Cross-region access → evaluate whether multi-region bucket or regional CDN provides better economics

The most common pricing mistake: assuming colder storage is always cheaper. If retrieval frequency and volume exceed certain thresholds, the per-GB retrieval charges for Coldline or Archive can exceed the storage savings over Standard class. Modeling expected access patterns before setting lifecycle policies avoids this pitfall.

### Requester Pays

Buckets can be configured so that the requesting project pays for operations and egress rather than the bucket owner. This enables data sharing patterns where large public datasets are hosted without the provider bearing access costs — consumers pay proportionally to their usage.
