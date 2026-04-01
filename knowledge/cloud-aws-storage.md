# AWS Storage Services

## S3

### Storage Classes

| Class                | Availability | Min Duration | Retrieval                                              | Use Case                              |
| -------------------- | ------------ | ------------ | ------------------------------------------------------ | ------------------------------------- |
| Standard             | 99.99%       | None         | Instant                                                | Frequently accessed data              |
| Intelligent-Tiering  | 99.9%        | None         | Instant (frequent/infrequent), minutes-hours (archive) | Unknown/changing access patterns      |
| Standard-IA          | 99.9%        | 30 days      | Instant                                                | Infrequent but needs fast access      |
| One Zone-IA          | 99.5%        | 30 days      | Instant                                                | Reproducible infrequent data          |
| Glacier Instant      | 99.9%        | 90 days      | Instant (ms)                                           | Quarterly access archives             |
| Glacier Flexible     | 99.99%       | 90 days      | 1-12 hours (expedited: 1-5 min)                        | Backup, disaster recovery             |
| Glacier Deep Archive | 99.99%       | 180 days     | 12-48 hours                                            | Compliance archives, tape replacement |

All classes: 99.999999999% (11 9s) durability. Durability = chance of not losing data. Availability = chance of being able to access data.

Intelligent-Tiering monitors access patterns and moves objects automatically:

- Frequent Access tier (default)
- Infrequent Access tier (30 days no access)
- Archive Instant Access tier (90 days)
- Archive Access tier (90 days, opt-in)
- Deep Archive Access tier (180 days, opt-in)

No retrieval fees — only a small monthly monitoring/automation fee per object.

### Lifecycle Policies

Automate transitions and expirations:

```json
{
  "Rules": [
    {
      "ID": "archive-old-logs",
      "Filter": { "Prefix": "logs/" },
      "Status": "Enabled",
      "Transitions": [
        { "Days": 30, "StorageClass": "STANDARD_IA" },
        { "Days": 90, "StorageClass": "GLACIER_IR" },
        { "Days": 365, "StorageClass": "DEEP_ARCHIVE" }
      ],
      "Expiration": { "Days": 2555 },
      "NoncurrentVersionTransitions": [
        { "NoncurrentDays": 30, "StorageClass": "GLACIER_IR" }
      ],
      "NoncurrentVersionExpiration": { "NoncurrentDays": 90 }
    }
  ]
}
```

Transition waterfall rule: can only move "down" (Standard → IA → Glacier → Deep Archive). Minimum 30 days before transitioning from Standard to IA.

### Versioning

- Once enabled, cannot be disabled — only suspended
- Delete markers hide objects but don't remove them
- MFA Delete: requires MFA to permanently delete versions or change versioning state
- Combine with lifecycle rules to expire old versions

### Replication

**Cross-Region Replication (CRR)**: Compliance, latency reduction, cross-account backup.
**Same-Region Replication (SRR)**: Log aggregation, live replication between accounts, data sovereignty.

Requirements:

- Source and destination buckets must have versioning enabled
- IAM role with replication permissions
- Can replicate to different account, different storage class
- Existing objects not replicated retroactively (use S3 Batch Replication)
- Delete markers can optionally be replicated
- No chaining: A→B→C replication requires explicit A→C rule

Replication Time Control (RTC): SLA for 99.99% of objects replicated within 15 minutes.

### Presigned URLs

Temporary access to private objects without making them public:

```python
import boto3

s3 = boto3.client('s3')

# Download URL (GET)
url = s3.generate_presigned_url('get_object',
    Params={'Bucket': 'my-bucket', 'Key': 'file.pdf'},
    ExpiresIn=3600)  # 1 hour

# Upload URL (PUT)
url = s3.generate_presigned_url('put_object',
    Params={'Bucket': 'my-bucket', 'Key': 'uploads/file.pdf',
            'ContentType': 'application/pdf'},
    ExpiresIn=3600)
```

URL validity inherits the credential lifetime of the signer:

- IAM user: up to 7 days
- IAM role/STS: up to 36 hours (limited by session duration)
- Revoking the signer's permissions invalidates all their presigned URLs

### Multipart Upload

Required for objects > 5 GB, recommended for > 100 MB:

1. Initiate multipart upload → get upload ID
2. Upload parts in parallel (5 MB to 5 GB each, up to 10,000 parts)
3. Complete multipart upload → S3 concatenates parts

Abort incomplete uploads via lifecycle rule (`AbortIncompleteMultipartUpload`) — incomplete parts still incur storage costs.

`aws s3 cp` automatically uses multipart for large files. For SDK, use `TransferManager` (Java) or `s3transfer` (Python).

### Encryption

| Type    | Key Management                            | Performance         | Use Case                                  |
| ------- | ----------------------------------------- | ------------------- | ----------------------------------------- |
| SSE-S3  | AWS manages everything                    | No overhead         | Default, simplest                         |
| SSE-KMS | You control KMS key, audit via CloudTrail | KMS API rate limits | Compliance, key rotation control          |
| SSE-C   | You provide key with every request        | No KMS overhead     | Full key control, no AWS key storage      |
| CSE     | Client encrypts before upload             | Client CPU cost     | Zero-trust, data never unencrypted in AWS |

SSE-S3 is the default encryption for all new buckets. SSE-KMS adds per-request CloudTrail logging and KMS rate limits (5,500-30,000 requests/sec depending on region).

Bucket keys (SSE-KMS): Reduces KMS API calls by generating a bucket-level key that creates data keys locally. Cuts KMS costs up to 99%.

### Access Points

Named network endpoints attached to buckets with their own access policy:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": { "AWS": "arn:aws:iam::123456789:role/analytics" },
      "Action": ["s3:GetObject"],
      "Resource": "arn:aws:s3:us-east-1:123456789:accesspoint/analytics-ap/object/data/*"
    }
  ]
}
```

- Simplify complex bucket policies (one access point per application team)
- VPC-restricted access points (only accessible from specific VPC)
- S3 Multi-Region Access Points: single global endpoint, routes to nearest bucket

### S3 Performance

- 5,500 GET and 3,500 PUT per prefix per second
- Spread across prefixes for higher aggregate throughput
- S3 Transfer Acceleration: Uses CloudFront edge → AWS backbone for faster long-distance uploads
- S3 Select / Glacier Select: Query with SQL to retrieve subset of object (CSV, JSON, Parquet)
- Byte-range fetches: Download specific byte ranges in parallel

## EFS (Elastic File System)

POSIX-compliant NFS filesystem, mountable by multiple EC2/ECS/Lambda:

| Feature           | Detail                                                                                |
| ----------------- | ------------------------------------------------------------------------------------- |
| Protocol          | NFSv4.1                                                                               |
| Performance modes | General Purpose (default, lower latency), Max I/O (higher throughput, higher latency) |
| Throughput modes  | Bursting, Provisioned, Elastic (recommended)                                          |
| Storage classes   | Standard, Infrequent Access, One Zone, One Zone-IA                                    |
| Encryption        | At rest (KMS), in transit (TLS mount helper)                                          |
| Max size          | Petabyte scale, grows automatically                                                   |

Elastic throughput mode: Automatically scales throughput up/down based on workload. Recommended for unpredictable workloads — you pay for what you use.

Lambda + EFS: Mount EFS to Lambda for shared file storage across invocations. Access point required (provides POSIX user/group).

Lifecycle management: Move files to IA after 7/14/30/60/90 days of no access. Move back to Standard on next access (Intelligent-Tiering equivalent).

## FSx

Managed file systems for specific workloads:

| Service              | Protocol        | Use Case                             |
| -------------------- | --------------- | ------------------------------------ |
| FSx for Lustre       | Lustre          | HPC, ML training, media processing   |
| FSx for Windows      | SMB             | Windows workloads, AD integration    |
| FSx for NetApp ONTAP | NFS, SMB, iSCSI | Multi-protocol, data management      |
| FSx for OpenZFS      | NFS             | Linux workloads needing ZFS features |

FSx for Lustre + S3 integration: Lazy-loads S3 objects on first access, writes back to S3. Seamless hot-tier for compute-intensive processing of S3 data.

FSx for NetApp ONTAP: Most versatile — supports all protocols, snapshots, cloning, compression, deduplication, tiering to S3. Good for migration from on-prem NetApp.

## Storage Gateway

Hybrid cloud storage — on-premises access to cloud storage:

| Mode                    | Interface | Backend                 | Use Case                                       |
| ----------------------- | --------- | ----------------------- | ---------------------------------------------- |
| S3 File Gateway         | NFS/SMB   | S3                      | File shares backed by S3                       |
| FSx File Gateway        | SMB       | FSx for Windows         | Low-latency access to FSx                      |
| Volume Gateway (Cached) | iSCSI     | S3 + local cache        | Primary data in cloud, hot data cached locally |
| Volume Gateway (Stored) | iSCSI     | Local + async S3 backup | Primary data local, snapshots to cloud         |
| Tape Gateway            | iSCSI VTL | S3 Glacier              | Backup software tape replacement               |

Deployed as VM (VMware/Hyper-V) or hardware appliance. Local cache provides low-latency access to frequently used data while keeping full dataset in cloud.
