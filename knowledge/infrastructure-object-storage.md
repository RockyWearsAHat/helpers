# Object Storage — S3 API, Consistency, Lifecycle & Replication

## Overview

Object storage is a cloud storage paradigm that treats data as unstructured, addressable objects (blobs) accessed via HTTP APIs, rather than as files in a hierarchical filesystem or rows in a database. Amazon S3 defined the category and API standard; the model now dominates cloud storage. Object storage trades complex transactions and hierarchical navigation for massive scalability, durability, and availability.

## The S3 API as de facto Standard

S3 defines the lingua franca for object storage. Features:

- **RESTful Access**: Objects addressed by bucket (storage namespace) + key (object identifier), accessed via HTTP verbs (GET, PUT, DELETE, HEAD, POST).
- **Metadata and Tags**: Objects carry user-defined metadata (headers) and tag key-value pairs for searchable metadata.
- **Versioning**: Optional per-bucket; when enabled, each PUT creates a new version, and GET retrieves the latest unless a version ID is specified.
- **Access Control**: ACLs (legacy, coarse-grained), bucket policies (JSON-based, fine-grained), and IAM roles.

Competitors (Azure Blob Storage, Google Cloud Storage, MinIO) expose S3-compatible APIs, allowing workloads to switch providers or run multi-cloud with minimal code changes. The API extensibility (storage classes, lifecycle, notifications) means applications rarely outgrow it.

## Consistency: The Shift from Eventual to Strong

### The Historical Model: Eventual Consistency

When S3 launched (2006), it provided **eventual consistency** for high availability and scalability:

- **Read After Write (for new objects)**: PUTs to a new key returned immediately; subsequent GETs were guaranteed to see the new object (strong).
- **Eventual Consistency (for overwrites/deletes)**: PUTs/DELETEs to existing keys returned immediately but might not be visible in subsequent GETs for seconds. Multiple replicas were slowly synchronized.

This model maximized availability: if a replicated failed or datacenter became unavailable, S3 served stale data rather than refusing requests. Applications had to architect around it:

```
Application calls PutObject(key="counter", value="1")
    → S3 returns success
GetObject(key="counter") from another region
    → Might return OLD value for up to seconds
→ Application must retry, implement exponential backoff, or use conditional writes
```

### The Current Model: Strong Consistency (December 2020)

S3 transitioned to **strong read-after-write consistency** for ALL operations (PUTs, DELETEs, conditional writes). After a PUT or DELETE returns success, all subsequent reads are guaranteed to see the new value or deletion, regardless of region or process.

**Implementation**: Achieved via AWS's internal consensus protocol (similar to Raft) that ensures writes are replicated and acknowledged before responding to the client. Trade-off: slightly higher latency (typically <50ms additional), but consistency guaranteed.

**Impact on Applications**:

- **List-After-Write**: After uploading an object, ListBucket immediately includes it. No eventual consistency delays.
- **Atomic Operations**: Conditional writes (If-Match / If-None-Match ETag checks) are now strongly consistent, simplifying concurrent update patterns.
- **Simpler Logic**: Applications no longer need retry loops, exponential backoff, or polling to detect visibility of recent writes.

**Remaining Eventual Consistency**: Bucket operations (bucket creation, ACL changes) and some metadata updates are eventually consistent, but object data is now strongly consistent.

## Storage Classes

S3 offers multiple storage classes trading availability, retrieval latency, and cost:

| Class | Availability | Min Duration | Retrieval Cost | Use Case |
|-------|--------------|--------------|----------------|----------|
| **Standard** | 99.99% | None | None | Frequently accessed, hot data |
| **Intelligent-Tiering** | 99.9% | None | None (cost varies by tier) | Unknown/dynamic access patterns; auto-moves between tiers |
| **Standard-IA** (Infrequent Access) | 99.9% | 30 days | Yes (per-retrieval cost) | Infrequent access, but fast when accessed |
| **One Zone-IA** | 99.5% | 30 days | Yes | Infrequent, reproducible data (non-critical) |
| **Glacier Instant** | 99.9% | 90 days | None | Archive; instant retrieval (expensive) |
| **Glacier Flexible** | 99.99% | 90 days | Yes (1–12 hours) | Archive; batch retrieval acceptable |
| **Glacier Deep Archive** | 99.99% | 180 days | Yes (12–48 hours) | Long-term compliance archives |

All classes: **11-9s (99.999999999%) durability** across multiple datacenters and availability zones.

**Pricing Model**: Early retrieval from IA/Glacier incurs penalties; keeping data in a class for the minimum duration ensures no early-retrieval fees. Lifecycle policies automate transitions.

**Intelligent-Tiering Mechanics**: Monitors access patterns and automatically transitions objects:

- Frequent Access (default): Standard pricing.
- Infrequent Access (30 days no access): Lower storage, retrieval cost.
- Archive Instant (90 days): Archive storage, instant retrieval.
- Archive Access (180+ days, opt-in): Very cheap, longer retrieval.

No retrieval fees for auto-transitions, only monitoring fee per object (~0.0025 USD/month per 1000 objects).

## Lifecycle Policies and Expiration

Lifecycle policies automate transitions and deletions based on time or object properties:

```
// Example: Transition to Glacier after 30 days, delete after 1 year
{
  "Rules": [
    {
      "Id": "Archive old logs",
      "Status": "Enabled",
      "Filter": {"Prefix": "logs/"},
      "Transitions": [
        {"Days": 30, "StorageClass": "GLACIER"}
      ],
      "Expiration": {"Days": 365}
    }
  ]
}
```

Policies filter by prefix, tags, or size (e.g., "delete all objects < 1 MB older than 90 days"). Versioning integrates: current vs. previous versions follow separate rules, enabling log rotation (keep current 30 days, archive previous versions).

## Versioning and Version Lifecycle

When versioning is enabled, PUT creates a new version; previous versions are retained. Deletes are non-destructive—a delete marker is created, and previous versions remain (visible with `?versionId`).

**Version Lifecycle**:

- Transition to cheaper storage classes per-version.
- Delete old versions after retained period (e.g., keep current 7 days, delete versions older than 90).

**Use Case**: Rolling deployments where old app versions are deployed as versioned objects; lifecycle cleans up versions older than retention policy.

## Presigned URLs and Temporary Access

Presigned URLs are time-limited, cryptographically signed URLs allowing temporary access without deploying AWS credentials to clients:

```
PUT presigned_url  // Client uploads directly to S3 without API key
POST presigned_url // Client downloads from S3 with temporary permission
```

**Mechanics**: The issuer (app server) signs a URL using their AWS secret key; the signature includes the HTTP method, resource, and expiration time. S3 validates the signature and expiration; if valid, grants access.

**Typical Flow**:

1. User requests an upload slot from your app.
2. App generates presigned PUT URL (expires in 5 minutes) and returns to user.
3. User's browser directly uploads to S3 using presigned URL.
4. S3 validates signature; no app involvement needed.

Presigned URLs eliminate bandwidth proxying through your app and scale download/upload to S3's capacity.

## Multipart Upload and Delete

**Multipart Upload**: Large objects (and any file >100 MB recommended) are uploaded in parts:

1. Initiate upload → get upload ID.
2. Upload parts in parallel, out-of-order.
3. Complete upload → S3 assembles parts into final object.

Reduces latency (parallel uploads) and enables retries (only failed parts re-uploaded, not entire file).

**Multipart Delete**: Batch delete API removes up to 1000 objects per request, cheaper than individual delete calls.

## S3 Select and Glacier Select

S3 Select (query language: SQL or PARQUET) allows filtering rows/columns *before* transferring to client, reducing bandwidth:

```
SELECT * FROM s3object WHERE age > 18  // Only matching rows transferred
```

Glacier Select retrieves data from archive without full restore, enabling ad-hoc queries on cold data.

Trade-off: S3 Select computation costs more per query but saves bandwidth; use for large analytical workloads on smaller result sets.

## Cross-Region Replication (CRR)

CRR automatically replicates objects to a destination bucket in another region. Asynchronous; replicas appear within seconds-to-minutes.

- **Use Cases**: Disaster recovery, geographic distribution, compliance (data residency).
- **Versioning Required**: Both buckets must have versioning enabled.
- **Replication Rules**: Filter by prefix/tags; choose destination region and storage class (e.g., replicate to Glacier in DR region).
- **Costs**: Replication incurs data transfer costs (GB replicated) + destination storage.

## MinIO: Self-Hosted S3-Compatible Object Storage

MinIO is an open-source object storage server implementing the S3 API, deployable on-premises or private cloud.

- **Deployment**: Container or binary; single instance (for dev) or distributed (multiple machines for HA).
- **Erasure Coding**: Data split across N drives with M parity drives; survives M drive failures.
- **Scaling**: Add drives or machines; cluster rebalances data automatically.
- **Cost**: No egress fees (contrast with cloud S3); all costs are infrastructure and operations.

MinIO is suitable for private datacenters, Kubernetes environments, or organizations avoiding cloud lock-in. Trade-off: operational complexity (hardware management, updates, tuning) vs. cloud convenience.

## Storage Gateway Patterns

**AWS Storage Gateway** bridges on-premises applications and cloud S3:

- **File Gateway**: NFS/SMB mount presents S3 as a filesystem; caches frequently-accessed files locally.
- **Tape Gateway**: Virtual tape library emulating legacy tape systems; data written to VTL is asynchronously uploaded to S3 Glacier.
- **Volume Gateway**: Block storage (EBS snapshots) backed by S3.

Used for hybrid environments where legacy apps require filesystem/block semantics but data is stored in S3.

## Immutability and Compliance

**Object Lock**: Enables write-once-read-many (WORM) mode; objects cannot be deleted or overwritten (even by admin) for a specified retention period or indefinitely. Common for compliance (FINRA, SEC regulations require immutable records).

**Legal Hold**: Freezes deletion independently of retention calendar; useful for litigation or incident investigations.

## Consistency Caveats and Application Patterns

Despite strong consistency for object data, applications must still handle:

- **Bucket Listing Eventual Consistency**: ListBucket reflects recent changes eventually, not immediately (for metadata scalability).
- **Metadata Eventual Consistency**: Tag and ACL changes propagate asynchronously.
- **Network Failures**: Application must retry on network errors; S3 return codes distinguish retryable (5xx, timeout) from not-retryable (4xx user errors).

**Idempotency**: Successful upload returns ETag; client can use ETag in conditional requests to ensure exactly-once processing, even after retries.

See also: cloud-aws-storage, distributed-replication, system-design-distributed, database-internals-storage.