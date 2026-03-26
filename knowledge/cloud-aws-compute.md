# AWS Compute Services

## EC2

### Instance Families

| Family       | Optimized For   | Use Cases                                                   |
| ------------ | --------------- | ----------------------------------------------------------- |
| C (c7g, c7i) | Compute         | Batch processing, HPC, ML inference, gaming servers         |
| M (m7g, m7i) | General purpose | Web servers, app servers, small databases                   |
| R (r7g, r7i) | Memory          | In-memory caches, real-time analytics, large databases      |
| T (t3, t4g)  | Burstable       | Dev/test, small workloads, microservices with variable load |
| G (g5, g6)   | GPU graphics    | Video encoding, 3D rendering, game streaming                |
| P (p5, p4d)  | GPU compute     | ML training, deep learning, HPC                             |
| I (i4i)      | Storage I/O     | NoSQL databases, data warehousing, Elasticsearch            |
| D (d3)       | Dense storage   | HDFS, MapReduce, distributed file systems                   |
| Hpc (hpc7g)  | HPC             | Tightly-coupled HPC workloads                               |

Naming: `c7g.2xlarge` → family(c) generation(7) processor(g=Graviton) . size

Graviton (g suffix) instances are 20-40% better price/performance than Intel equivalents for most workloads. Default choice unless software requires x86.

### Purchasing Options

| Model                   | Discount  | Commitment                | Flexibility                   |
| ----------------------- | --------- | ------------------------- | ----------------------------- |
| On-Demand               | 0%        | None                      | Full                          |
| Spot                    | Up to 90% | None (can be interrupted) | Instance type/AZ flexible     |
| Reserved (Standard)     | Up to 72% | 1 or 3 year               | Locked to instance type/AZ    |
| Reserved (Convertible)  | Up to 66% | 1 or 3 year               | Can change instance family    |
| Savings Plans (Compute) | Up to 66% | 1 or 3 year               | Any instance family/region/OS |
| Savings Plans (EC2)     | Up to 72% | 1 or 3 year               | Locked to family/region       |

Spot interruption handling:

- 2-minute warning via instance metadata + CloudWatch Events
- Use `capacity-optimized` allocation strategy (not `lowest-price`)
- Spot Fleet or EC2 Fleet for diversified pools
- Suitable for: batch, CI/CD, stateless web tiers, training jobs

### IMDSv2 (Instance Metadata Service)

IMDSv1 uses simple GET to `169.254.169.254` — vulnerable to SSRF attacks. IMDSv2 requires a session token:

```bash
# Get token (TTL in seconds, max 21600 = 6 hours)
TOKEN=$(curl -X PUT "http://169.254.169.254/latest/api/token" \
  -H "X-aws-ec2-metadata-token-ttl-seconds: 21600")

# Use token for metadata requests
curl -H "X-aws-ec2-metadata-token: $TOKEN" \
  http://169.254.169.254/latest/meta-data/instance-id
```

Enforce IMDSv2-only: `HttpTokens: required` in launch template. Set `HttpPutResponseHopLimit: 1` to block container SSRF (containers add a network hop).

### EBS Volume Types

| Type              | IOPS                  | Throughput                | Latency         | Use Case                              |
| ----------------- | --------------------- | ------------------------- | --------------- | ------------------------------------- |
| gp3               | 3000 base, up to 16K  | 125 MB/s base, up to 1000 | Single-digit ms | Default for most workloads            |
| gp2               | 3 IOPS/GB, burst 3000 | Up to 250 MB/s            | Single-digit ms | Legacy, migrate to gp3                |
| io2 Block Express | Up to 256K            | Up to 4000 MB/s           | Sub-ms          | Critical databases                    |
| io2               | Up to 64K             | Up to 1000 MB/s           | Sub-ms          | Databases needing consistent IOPS     |
| st1               | Up to 500             | Up to 500 MB/s            | —               | Big data, log processing (sequential) |
| sc1               | Up to 250             | Up to 250 MB/s            | —               | Cold storage, infrequent access       |

gp3 is almost always the right choice — decouples IOPS and throughput from volume size. gp2's IOPS scales with size (need 10K IOPS = need 3.3 TB volume), gp3 lets you provision independently.

EBS Multi-Attach (io2 only): attach one volume to up to 16 Nitro instances in the same AZ. Requires cluster-aware filesystem.

### AMIs and Launch Templates

AMI baking strategy:

1. **Golden AMI**: Base OS + security hardening + monitoring agents → rebuild weekly
2. **Application AMI**: Golden AMI + app runtime + dependencies → rebuild per release
3. **Hybrid**: Golden AMI at launch, pull app code via user data (slower boot, more flexible)

Launch templates > launch configurations (legacy). Templates support versioning, multiple instance types, mixed instances policy.

### Auto Scaling Groups

Scaling policies:

- **Target tracking**: Maintain metric at target (e.g., CPU at 50%) — simplest, handles most cases
- **Step scaling**: Different scaling actions at different thresholds
- **Simple scaling**: One adjustment per alarm, cooldown period — avoid, use step instead
- **Scheduled**: Time-based for predictable patterns
- **Predictive**: ML-based, forecasts from historical patterns

Key settings:

- `HealthCheckGracePeriod`: Time before ASG checks health on new instance (default 300s, increase for slow-starting apps)
- `DefaultInstanceWarmup`: Time before new instance contributes to CloudWatch metrics
- Instance refresh: Rolling replacement of instances (min healthy percentage, skip matching)
- Warm pools: Pre-initialized stopped instances for faster scale-out

## ECS

### Task Definitions

JSON document defining containers, like a docker-compose for AWS:

```json
{
  "family": "web-app",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "512",
  "memory": "1024",
  "containerDefinitions": [
    {
      "name": "app",
      "image": "123456789.dkr.ecr.us-east-1.amazonaws.com/app:latest",
      "portMappings": [{ "containerPort": 8080, "protocol": "tcp" }],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/web-app",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      },
      "healthCheck": {
        "command": [
          "CMD-SHELL",
          "curl -f http://localhost:8080/health || exit 1"
        ],
        "interval": 30,
        "timeout": 5,
        "retries": 3,
        "startPeriod": 60
      }
    }
  ]
}
```

### Fargate vs EC2 Launch Type

| Aspect     | Fargate                                | EC2                                        |
| ---------- | -------------------------------------- | ------------------------------------------ |
| Management | Serverless, no instances               | You manage EC2 fleet                       |
| Pricing    | Per vCPU/memory/second                 | EC2 pricing + optional Spot                |
| GPU        | Not supported                          | Supported                                  |
| EBS        | Ephemeral 200GB                        | Full EBS support                           |
| Networking | awsvpc only (ENI per task)             | awsvpc, bridge, host, none                 |
| Scaling    | Simpler (no capacity providers needed) | Capacity providers manage instance scaling |
| Startup    | ~30-60s cold start                     | Faster if instances available              |

Fargate Spot: Up to 70% discount, tasks can be interrupted with 30-second SIGTERM.

### Service Discovery

AWS Cloud Map integration:

- Creates DNS records (A or SRV) in a private hosted zone
- Tasks register/deregister automatically
- Health checks via Route 53 or ECS task health
- Service connect (newer): Envoy-based service mesh with load balancing

## EKS

### Managed Node Groups

EKS manages the EC2 instances running your pods:

- Automated AMI updates with drain + cordon
- Spot instance support with multiple instance types
- Custom launch templates for GPU, storage, etc.
- Karpenter (preferred over Cluster Autoscaler): provisions right-sized nodes in seconds

### IRSA and Pod Identity

**IRSA (IAM Roles for Service Accounts)**: Maps Kubernetes service accounts to IAM roles via OIDC federation. Pod gets temporary credentials scoped to its service account.

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: s3-reader
  annotations:
    eks.amazonaws.com/role-arn: arn:aws:iam::123456789:role/s3-reader-role
```

**EKS Pod Identity** (newer, simpler): No OIDC provider setup needed. Associate role with service account via EKS API. Supports cross-account without OIDC. Preferred for new clusters.

## Lambda

### Cold Starts

Cold start = new execution environment initialization. Happens on:

- First invocation after deploy
- Scaling up to handle concurrent requests
- Invocation after ~15 min idle (environment recycled)

Mitigation strategies:

- **Provisioned Concurrency**: Pre-initialized environments, eliminates cold starts entirely. Costs: you pay for allocated concurrency whether used or not.
- **SnapStart** (Java only): Snapshots initialized environment after init phase. Restores from snapshot instead of re-initializing. Reduces Java cold starts from ~5s to ~200ms.
- Move SDK client initialization outside handler (runs once per environment)
- Minimize deployment package size (remove unused deps)
- Use arm64 (Graviton) — faster init than x86 for many runtimes

### Lambda Layers

Shared code/dependencies packaged separately from function code:

- Up to 5 layers per function
- Total unzipped size limit: 250 MB (including function code)
- Layer versions are immutable
- Use for: shared libraries, custom runtimes, common utilities

### Function URLs

Built-in HTTPS endpoint without API Gateway:

- IAM auth or no auth (public)
- Supports streaming responses (response payload streaming)
- No custom domain (use CloudFront in front)
- No request validation, throttling, or API keys — use API Gateway for those

### Limits and Configuration

| Setting               | Default             | Max                  |
| --------------------- | ------------------- | -------------------- |
| Memory                | 128 MB              | 10,240 MB            |
| Timeout               | 3 sec               | 900 sec (15 min)     |
| Concurrent executions | 1000/account/region | Requestable increase |
| Deployment package    | 50 MB zipped        | 250 MB unzipped      |
| /tmp storage          | 512 MB              | 10,240 MB            |
| Environment variables | 4 KB total          | —                    |
| Layers                | 5                   | —                    |

vCPU scales linearly with memory: 1,769 MB = 1 full vCPU. At 10 GB you get ~6 vCPUs.

### Lambda Design Patterns

- Keep functions focused (single responsibility)
- Use environment variables for configuration, Secrets Manager for secrets
- Enable X-Ray tracing for debugging
- Set reserved concurrency to prevent one function from starving others
- Use SQS/EventBridge for async invocation to decouple and retry
- Prefer ARM64 runtime — ~20% cheaper, comparable or better performance
