# AWS S3 Patterns — Storage Classes, Lifecycle, Replication, and Access Control

## Overview

Amazon S3 (Simple Storage Service) is an object storage service accessible via HTTP. Objects (files) are stored in **buckets** (top-level containers) and are immutable after write (no in-place edits). S3 patterns cover **data organization** (storage classes, lifecycle policies), **availability** (versioning, replication), **access control** (bucket policies, ACLs, VPC endpoints), and **performance** (presigned URLs, Transfer Acceleration, S3 Select).

## Storage Classes

S3 offers multiple storage classes trading **cost** (per-GB/month and retrieval) for **availability** and **access speed**. AWS recommends moving data through classes over its lifecycle: hot data in Standard, aging data in Glacier or Glacier Deep Archive.

### Standard

- **Cost**: highest per-GB/month (~$0.021/GB/month)
- **Retrieval**: instant; no time penalty
- **Availability**: 99.99%; designed for frequent access
- **Min storage duration**: none
- **Use case**: active data, frequently accessed files, cache-like workloads

### Intelligent-Tiering

- **Cost**: varies by tier; AWS tiers data automatically based on access frequency
- **Retrieval**: depends on tier
- **Availability**: 99.9% (frequent), 99% (infrequent), 99% (archive), 99.9% (deep archive)
- **Min storage duration**: per tier; frequent = 0 days, infrequent = 30 days, archive = 90 days
- **Use case**: unpredictable access patterns; let AWS optimize

Automatically transitions objects between tiers without application intervention. Small per-object monitoring cost.

### Standard-IA (Infrequent Access)

- **Cost**: low per-GB/month (~$0.0125/GB/month) but high retrieval cost (~$0.01 per 1000 requests)
- **Retrieval**: instant but charged
- **Availability**: 99.9%
- **Min storage duration**: 30 days; charging minimum even if deleted earlier
- **Use case**: backups, older data accessed sometimes

If you retrieve an 1GB object from Standard-IA, you pay: storage ($0.0125) + retrieval ($0.01) ≈ $0.0225/month total. Breakeven vs. Standard-IA: if object accessed <~2x per month, IA is cheaper.

### One Zone-IA

- **Cost**: ~$0.01/GB/month (cheaper than Standard-IA)
- **Retrieval**: instant but charged
- **Availability**: 99.5% (single AZ); **NOT replicated across AZs**
- **Min storage duration**: 30 days
- **Use case**: replicated/reproducible data, disaster recovery secondary copies

Cheaper than Standard-IA but higher availability risk (AZ outage = data loss).

### Glacier (Instant Retrieval)

- **Cost**: ~$0.004/GB/month (very cheap storage)
- **Retrieval**: instant; no time penalty but charged (~$0.03 per 1000 requests)
- **Availability**: 99.9%
- **Min storage duration**: 90 days
- **Use case**: archival data, compliance archives, rarely accessed but must be quick when needed

### Glacier Flexible Retrieval

- **Cost**: ~$0.0036/GB/month (cheapest storage)
- **Retrieval**: expedited (1-5 min), standard (3-5 hours), bulk (5-12 hours); retrieval cost varies
- **Availability**: 99.99%
- **Min storage duration**: 90 days
- **Use case**: archival data, regulatory compliance; time flexibility pays off

### Glacier Deep Archive

- **Cost**: ~$0.00099/GB/month (cheapest available)
- **Retrieval**: standard (12 hours), bulk (48 hours); no expedited
- **Availability**: 99.99%
- **Min storage duration**: 180 days
- **Use case**: long-term archive (7+ years), regulatory hold data rarely retrieved

## Lifecycle Policies

**Lifecycle policies** automatically transition objects between storage classes or delete them based on age, prefix, or tags. Example: new files → Standard → Standard-IA (after 30 days) → Glacier (after 90 days) → delete (after 365 days).

### Transition Rules

Define conditions (object age, prefix, tag):

```
IF object age > 30 days THEN move to Standard-IA
IF object age > 90 days THEN move to Glacier
IF object age > 365 days THEN delete
```

Transitions are **asynchronous**; S3 runs lifecycle tasks once per day (not real-time). Can take up to 24 hours for transition to occur.

### Expiration

Objects deleted after condition met (age, tag match). Expiration is permanent: no restore from lifecycle deletion (soft delete not available for Standard). If versioning enabled, expiration marks version as noncurrent; noncurrent versions can have separate expiration rules.

### Cost Optimization

Lifecycle policies save storage cost on aging data but incur transition overhead (per-transition cost ~$0.001-$0.01). Worthwhile for large datasets with clear access patterns (hot → warm → cold). Not worthwhile for files <100GB or files with unpredictable access.

## Versioning

**Versioning** keeps multiple versions of an object. Each write creates a new version with a unique version ID; previous versions remain accessible. Disabling versioning stops creating new versions but does not delete existing versions (they become **noncurrent versions**).

### Use Cases

- **Accidental delete recovery**: retrieve previous version
- **Audit trail**: access object history
- **Compliance**: immutable storage (all versions retained)
- **Blue-green deployments**: rollback by reverting object version

### Cost

Each version is charged separately. Versioning can **double storage cost** if frequently overwritten files accumulate many versions. Use lifecycle policies to delete noncurrent versions after N days.

### Deletion Behavior

With versioning:
- `DELETE object.txt` does not remove object; instead, creates **delete marker** (invisible to `GET`, visible to list if soft-delete listing enabled)
- To permanently delete a specific version, specify version ID: `DELETE object.txt?versionid=xyz`

## Replication

S3 offers two replication modes: **Cross-Region Replication (CRR)** and **Same-Region Replication (SRR)**.

### Cross-Region Replication (CRR)

Automatically copies objects from source bucket (region A) to destination bucket (region B) upon upload. **Destination must be in different region.**

### Use Cases for CRR

- **Disaster recovery**: maintain copy in geographically distant region; if source region destroyed, destination available
- **Data residency**: replicate to region closer to users for lower latency reads
- **Compliance**: maintain copy in specific region for regulatory reasons

### Same-Region Replication (SRR)

Copies objects within same region but different bucket. Less common; use cases include:

- **Separate bucket for logs**: replication rule: if S3 Access Logs detected, copy to logs bucket
- **Multi-tenant isolation**: replicate to tenant-specific bucket

### Replication Rules

Define source bucket prefix and destination bucket; optionally filter by tags. Example: `logs/*` → replicate only objects with prefix `logs/`.

### Replication Lag

Replication is asynchronous. Object appears in source immediately; destination copy appears typically within seconds to minutes. Large objects may take longer. No RPO guarantee (objects can be lost if source region destroyed before replication completes).

### Replication Costs

Charged per GB replicated. If source and destination in same region ≠ SRR; cross-region is expensive.

## Event Notifications

S3 can send notifications when objects are uploaded, deleted, or updated. Destinations include **SNS** (Simple Notification Service), **SQS** (Simple Queue Service), **Lambda**, and **EventBridge**.

### Use Cases

- **Image resizing**: upload image → trigger Lambda → generate thumbnail
- **Virus scanning**: upload file → trigger scan Lambda
- **Logging**: upload → SNS → email admin
- **Workflow triggers**: upload dataset → trigger data pipeline

### Filtering

Notifications can be filtered by object name prefix or tags. Example: notify only on `.jpg` files.

### Delivery Guarantee

Notifications are **at-least-once** (not exactly-once); duplicate notifications possible if S3 retries. Application should be idempotent.

## Presigned URLs

**Presigned URL** is a time-limited, signed URL granting temporary access to a private S3 object. Generated by an AWS principal (IAM user, role) with S3 permissions; receiver of URL does not need AWS credentials.

### Generation

```
s3://mybucket/myobject → presigned URL: https://mybucket.s3.amazonaws.com/myobject?X-Amz-Credential=...&X-Amz-Expires=3600&X-Amz-Signature=...
```

Presigned URL valid for specified duration (default 1 hour, max 7 days). Signature includes object name, so presigned URL is unique per object.

### Use Cases

- **Temporary download links**: generate presigned URL for client to download private file (e.g., invoice)
- **Resumable uploads**: presigned URL for `PUT` allows client to upload directly to S3 without proxy
- **Sharing single file**: send presigned URL via email instead of provisioning temporary user

### Revocation

Presigned URLs cannot be revoked before expiration (no whitelist). Once issued, valid for duration. To revoke early, issue new credentials (rotating IAM key invalidates URLs signed with old key, but existing URLs remain valid).

## S3 Select

**S3 Select** queries objects without downloading entire object. Supports SQL queries filtering rows and columns from CSV, JSON, or Parquet files.

### Example

```sql
SELECT name, age FROM s3://mybucket/data.csv WHERE age > 30
```

S3 returns only matching rows, reducing data transfer and client processing.

### Cost

S3 Select charged per GB of data **scanned** (not downloaded). If query scans 100GB of 200GB object, charged for 100GB. Low cost if high selectivity (few matching rows).

### Format Support

- **CSV**: row-based, no schema inference; must specify column names
- **JSON**: document-based; supports nested structures
- **Parquet**: columnar format; efficient for column filtering

## Transfer Acceleration

**Transfer Acceleration** uses **CloudFront** edge locations to upload to S3 faster. Upload to edge location (lower latency) → edge optimizes route back to S3 region → object lands in bucket.

### Benefits

Useful for:
- **Long-distance uploads**: client in Australia uploading to us-east-1 bucket; transfer acceleration routes via geographically optimal edge location
- **Mobile clients**: cellular networks benefit from multiple CDN edges

### Cost

Transfer Acceleration charged per GB uploaded (~$0.04/GB); higher than standard S3 upload. Worthwhile only if upload bandwidth is bottleneck (e.g., thousands of large file uploads).

### Tradeoff

If upload bandwidth is not bottleneck or low volume, standard upload cheaper.

## Access Control

S3 supports multiple authorization mechanisms: **Bucket Policies**, **ACLs**, **IAM**, and **VPC Endpoints**.

### Bucket Policies

Resource-based policies attached to bucket. Example:

```json
{
  "Effect": "Allow",
  "Principal": {"AWS": "arn:aws:iam::ACCOUNT:user/alice"},
  "Action": "s3:GetObject",
  "Resource": "arn:aws:s3:::mybucket/*"
}
```

Grants alice permission to read all objects in mybucket. Evaluation: AWS checks both **identity-based policies** (on alice) and **resource-based policies** (on bucket); if either allows, access granted.

### ACLs (Access Control Lists)

Object-level permissions granting access to specific AWS account or public. Simpler than bucket policies but less flexible. Example: grant public read access to object via ACL.

ACLs deprecated in favor of bucket policies; use bucket policies for new deployments.

### IAM Integration

IAM policies on users/roles can grant S3 permissions. Bucket policies can reference specific IAM roles. Combination: IAM policy grants user permission + bucket policy allows role to access.

### VPC Endpoints

**S3 VPC Endpoint** allows EC2 instances in private VPC to access S3 without internet gateway. Endpoint is logical link from VPC to S3. Two types:

- **Gateway endpoint**: routes S3 traffic through endpoint; free
- **Interface endpoint**: accessible via IP address in VPC; charged per hour + data processed

Example: Lambda in private subnet uploads to S3 via gateway endpoint.

## S3 Security Considerations

### Block Public Access

S3 **Block Public Access** setting prevents accidental public bucket exposure. If enabled, bucket policies granting public access are ignored. Recommended: enable globally.

### Encryption

- **SSE-S3**: S3 manages encryption keys (AWS-managed)
- **SSE-KMS**: customer-managed KMS key; adds ~50ms latency per request but offers key rotation control
- **DSSE-KMS**: double-layer encryption; not common

### Signed Requests

S3 supports **request signing** (Signature Version 4) for secure HTTP requests. SDK handles signing automatically.

## Cost Patterns

- **Storage cost dominates**: per-GB/month × 12 months >> retrieval/request cost for most workloads
- **Lifecycle policies reduce cost**: bulk data can migrate to cold storage within months, saving money
- **Data egress expensive**: downloading 100GB costs ~$10; avoid unnecessary data transfer
- **Replication expensive**: mirroring data to second region costs per GB replicated

See also: [infrastructure-object-storage](infrastructure-object-storage.md), [cloud-aws-storage](cloud-aws-storage.md)