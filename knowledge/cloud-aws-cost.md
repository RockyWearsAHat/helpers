# AWS Cost Management & Optimization

## Pricing Models

### Compute Pricing Tiers

| Model                        | Discount  | Commitment                | Flexibility                               | Best For                          |
| ---------------------------- | --------- | ------------------------- | ----------------------------------------- | --------------------------------- |
| On-Demand                    | 0%        | None                      | Full                                      | Unpredictable, short-term         |
| Savings Plans (Compute)      | Up to 66% | 1 or 3 year $/hr          | Any instance family/region/OS/tenancy     | Stable baseline across services   |
| Savings Plans (EC2 Instance) | Up to 72% | 1 or 3 year $/hr          | Specific instance family + region         | Predictable EC2 usage             |
| Reserved Instances           | Up to 72% | 1 or 3 year               | Specific instance type + AZ (or regional) | Legacy, use Savings Plans instead |
| Spot Instances               | Up to 90% | None (can be interrupted) | Any, with 2-min warning                   | Fault-tolerant batch, CI/CD, HPC  |

**Savings Plans vs Reserved Instances**: Savings Plans are strictly better for new commitments — more flexible, same or better discount. RIs still useful for capacity reservations in specific AZs.

### Payment Options

| Option          | Discount                   | Cash Flow              |
| --------------- | -------------------------- | ---------------------- |
| All Upfront     | Highest                    | Pay 100% upfront       |
| Partial Upfront | Medium                     | ~50% upfront + monthly |
| No Upfront      | Lowest (still significant) | Monthly only           |

### Service-Specific Pricing Patterns

```
Lambda:     $0.20/1M requests + $0.0000166667/GB-second
S3:         $0.023/GB (Standard), $0.0125/GB (IA), $0.004/GB (Glacier)
DynamoDB:   $1.25/million WCU, $0.25/million RCU (on-demand)
RDS:        Instance hours + storage + I/O + backup beyond free tier
EBS:        gp3: $0.08/GB/month + IOPS/throughput if above free tier
NAT GW:     $0.045/hour + $0.045/GB processed ← often a surprise cost
```

## Cost Explorer

### Analysis Capabilities

```
Group by: Service, Account, Region, Instance Type, Usage Type,
          Tag, API Operation, Availability Zone, Purchase Option
Filter:   Any dimension above + linked account, charge type
Granularity: Daily, Monthly, Hourly (last 14 days only)
Lookback: 14 months historical + 12 months forecast
```

### Key Views

**Right-Sizing Recommendations**: identifies underutilized EC2 instances based on CloudWatch metrics (CPU, memory via CloudWatch Agent, network). Shows current cost, recommended instance, and projected savings.

**Savings Plans Recommendations**: analyzes historical usage to recommend optimal commitment level:

```
Your On-Demand spend (last 30 days):  $15,000
Recommended Savings Plan:             $8.50/hr (Compute, 1yr, No Upfront)
Estimated monthly savings:            $4,200 (28%)
Coverage:                             72% of eligible usage
```

**Reservation Utilization**: monitors whether you're using what you've committed to. Target >80% utilization. Low utilization = selling RIs on Marketplace or modifying.

### Anomaly Detection

```bash
aws ce create-anomaly-monitor \
  --anomaly-monitor '{
    "MonitorName":"service-monitor",
    "MonitorType":"DIMENSIONAL",
    "MonitorDimension":"SERVICE"
  }'

aws ce create-anomaly-subscription \
  --anomaly-subscription '{
    "SubscriptionName":"cost-alerts",
    "MonitorArnList":["arn:aws:ce::ACCT:anomalymonitor/MONITOR_ID"],
    "Subscribers":[{"Address":"ops@company.com","Type":"EMAIL"}],
    "Threshold":100.0,
    "Frequency":"IMMEDIATE"
  }'
```

ML-powered detection of unexpected cost increases. Evaluates spend patterns by service, account, region — alerts before the bill shock.

## AWS Budgets

### Budget Types

| Type                      | Monitors                    | Use Case                   |
| ------------------------- | --------------------------- | -------------------------- |
| Cost budget               | Actual and forecasted spend | Monthly spend limits       |
| Usage budget              | Service usage quantities    | Track API calls, GB stored |
| Savings Plans utilization | SP usage percentage         | Alert on low utilization   |
| Savings Plans coverage    | % of spend covered by SP    | Identify coverage gaps     |
| Reservation utilization   | RI usage percentage         | Alert on unused RIs        |
| Reservation coverage      | % of spend covered by RIs   | Identify coverage gaps     |

### Budget Actions

```bash
aws budgets create-budget \
  --account-id ACCT \
  --budget '{
    "BudgetName": "monthly-limit",
    "BudgetLimit": {"Amount": "10000", "Unit": "USD"},
    "BudgetType": "COST",
    "TimeUnit": "MONTHLY",
    "CostFilters": {"Service": ["Amazon Elastic Compute Cloud - Compute"]},
    "CostTypes": {"IncludeTax": true, "IncludeSubscription": true}
  }' \
  --notifications-with-subscribers '[
    {
      "Notification": {
        "NotificationType": "ACTUAL",
        "ComparisonOperator": "GREATER_THAN",
        "Threshold": 80,
        "ThresholdType": "PERCENTAGE"
      },
      "Subscribers": [{"SubscriptionType": "EMAIL", "Address": "ops@company.com"}]
    },
    {
      "Notification": {
        "NotificationType": "FORECASTED",
        "ComparisonOperator": "GREATER_THAN",
        "Threshold": 100,
        "ThresholdType": "PERCENTAGE"
      },
      "Subscribers": [{"SubscriptionType": "SNS", "Address": "arn:aws:sns:..."}]
    }
  ]'
```

**Budget Actions** can automatically: apply an IAM SCP to restrict provisioning, stop EC2 instances, or apply a custom IAM policy — when a threshold is breached.

## Cost Allocation Tags

### Tag Strategy

```bash
# Activate cost allocation tags (takes 24 hours to appear in billing)
aws ce update-cost-allocation-tags-status \
  --cost-allocation-tags-status '[
    {"TagKey":"Environment","Status":"Active"},
    {"TagKey":"Team","Status":"Active"},
    {"TagKey":"Project","Status":"Active"},
    {"TagKey":"CostCenter","Status":"Active"}
  ]'
```

**Required tags** (enforce via SCP or AWS Config):

| Tag           | Values                      | Purpose                |
| ------------- | --------------------------- | ---------------------- |
| `Environment` | prod, staging, dev, sandbox | Separate environments  |
| `Team`        | engineering, data, platform | Team accountability    |
| `Project`     | project-alpha, project-beta | Project-level tracking |
| `CostCenter`  | CC-1001, CC-1002            | Finance allocation     |

**AWS-generated tags**: `aws:createdBy` (CloudTrail user), `aws:cloudformation:stack-name`. Activated separately, read-only.

**Untagged resource detection**: AWS Config rule `required-tags` or Tag Editor to find untagged resources across all services.

### Tag Policies (Organizations)

```json
{
  "tags": {
    "Environment": {
      "tag_key": { "@@assign": "Environment" },
      "tag_value": { "@@assign": ["prod", "staging", "dev", "sandbox"] },
      "enforced_for": { "@@assign": ["ec2:instance", "s3:bucket", "rds:db"] }
    }
  }
}
```

## Compute Optimizer

### Recommendations

Analyzes CloudWatch metrics (14 days minimum) to recommend:

| Resource            | Metrics Analyzed              | Recommendations                |
| ------------------- | ----------------------------- | ------------------------------ |
| EC2                 | CPU, memory, network, disk    | Right-size, Graviton migration |
| EBS                 | IOPS, throughput, latency     | Volume type change, resize     |
| Lambda              | Duration, memory, invocations | Memory right-sizing            |
| ECS on Fargate      | CPU, memory utilization       | Task size optimization         |
| Auto Scaling Groups | CPU, network                  | Instance type mix, size        |

**Enhanced infrastructure metrics**: opt-in for 3-month lookback (default 14 days). More history = better recommendations for variable workloads.

**Graviton recommendations**: identifies x86 instances that would benefit from ARM migration. Graviton instances are ~20% cheaper with same or better performance for most workloads.

## Data Transfer Costs

### Data Transfer Pricing (Key Rules)

```
Inbound (Internet → AWS):         FREE
Inbound (AWS service → service):  Usually FREE (same AZ)
Cross-AZ:                         $0.01/GB each way
Cross-Region:                     $0.02/GB (varies by region pair)
Outbound (AWS → Internet):        $0.09/GB (first 10 TB/month)
NAT Gateway processing:           $0.045/GB
VPC Peering cross-AZ:             $0.01/GB each way
VPC Peering cross-Region:         $0.02/GB each way
PrivateLink:                      $0.01/GB processed
CloudFront to Internet:           $0.085/GB (cheaper than direct)
S3 Transfer Acceleration:         $0.04/GB + standard transfer
```

### Common Surprise Costs

| Source                | Why It's Expensive                     | Mitigation                                                    |
| --------------------- | -------------------------------------- | ------------------------------------------------------------- |
| NAT Gateway           | $0.045/GB for ALL traffic through it   | VPC endpoints for S3/DynamoDB ($0), minimize public API calls |
| Cross-AZ traffic      | $0.01/GB × high volume = significant   | Place communicating services in the same AZ for hot paths     |
| S3 → Internet         | Large datasets served publicly         | CloudFront distribution, S3 Transfer Acceleration             |
| Idle ELBs             | Minimum $16/month even with no traffic | Remove unused load balancers                                  |
| Unattached EBS + EIPs | Pay for provisioned but unused         | Automated cleanup scripts                                     |
| RDS Multi-AZ          | 2× compute + storage replication       | Use for prod only, single-AZ for dev                          |
| CloudWatch Logs       | Ingestion $0.50/GB + storage $0.03/GB  | Set retention policies, filter before ingestion               |

### VPC Endpoints (Gateway and Interface)

```bash
# Gateway endpoint (S3, DynamoDB) — FREE, no per-GB charge
aws ec2 create-vpc-endpoint \
  --vpc-id vpc-xxx \
  --service-name com.amazonaws.us-east-1.s3 \
  --route-table-ids rtb-xxx

# Interface endpoint (all other services) — $0.01/GB + $0.01/hour per AZ
aws ec2 create-vpc-endpoint \
  --vpc-id vpc-xxx \
  --service-name com.amazonaws.us-east-1.secretsmanager \
  --vpc-endpoint-type Interface \
  --subnet-ids subnet-xxx \
  --security-group-ids sg-xxx
```

**S3 Gateway Endpoint** is the single biggest cost win for data-heavy workloads — eliminates NAT Gateway charges for S3 traffic completely.

## Cost Optimization Patterns

### EC2 Optimization

1. **Right-size first** — Compute Optimizer + CloudWatch metrics, downsize before committing
2. **Graviton migration** — 20% cheaper, better perf for most workloads (benchmark first)
3. **Savings Plans** — cover steady-state baseline
4. **Spot Instances** — for fault-tolerant workloads (batch, CI, data processing)
5. **Scheduling** — stop dev/test instances after hours (`aws:scheduler` or Instance Scheduler)

### Storage Optimization

```
S3 Lifecycle policies → move to IA (30d) → Glacier IR (90d) → Glacier DA (365d)
S3 Intelligent-Tiering → automatic movement, small monitoring fee per object
EBS: gp3 baseline is cheaper than gp2 for most workloads
EBS: Delete unattached volumes (AWS Config rule: ec2-volume-inuse-check)
EBS Snapshots: delete old snapshots, use EBS Snapshot Archive for long-term retention
```

### Database Optimization

- **RDS**: Reserved Instances for production, stop dev instances after hours
- **DynamoDB**: on-demand for spiky, provisioned + auto-scaling for steady-state
- **ElastiCache**: Reserved Nodes for persistent caches
- **Aurora**: Serverless v2 for variable workloads, I/O-Optimized for high-IOPS

### Network Optimization

- S3 Gateway Endpoints (eliminate NAT costs for S3)
- VPC endpoints for frequently-called AWS APIs
- CloudFront for content delivery (cheaper than direct S3 egress)
- Single-AZ for non-critical workloads
- Compress data in transit (gzip API responses, compressed log shipping)

## Cost and Usage Report (CUR)

### Setup

```bash
aws cur put-report-definition \
  --report-definition '{
    "ReportName": "detailed-billing",
    "TimeUnit": "HOURLY",
    "Format": "Parquet",
    "Compression": "Parquet",
    "S3Bucket": "billing-reports-ACCT",
    "S3Prefix": "cur/",
    "S3Region": "us-east-1",
    "AdditionalSchemaElements": ["RESOURCES", "SPLIT_COST_ALLOCATION_DATA"],
    "RefreshClosedReports": true,
    "ReportVersioning": "OVERWRITE_REPORT"
  }'
```

CUR is the most granular billing data AWS provides — line-item detail for every resource, every hour. Delivered as Parquet to S3, queryable via Athena:

```sql
-- Top 10 most expensive resources this month
SELECT line_item_resource_id,
       line_item_product_code,
       SUM(line_item_unblended_cost) as total_cost
FROM cur_database.cur_table
WHERE month = '1' AND year = '2024'
  AND line_item_line_item_type = 'Usage'
GROUP BY 1, 2
ORDER BY total_cost DESC
LIMIT 10;

-- Data transfer costs by type
SELECT product_transfer_type,
       product_from_location,
       product_to_location,
       SUM(line_item_unblended_cost) as cost,
       SUM(line_item_usage_amount) as gb_transferred
FROM cur_database.cur_table
WHERE line_item_product_code = 'AWSDataTransfer'
GROUP BY 1, 2, 3
ORDER BY cost DESC;

-- Savings Plans effective rate vs On-Demand
SELECT savings_plan_savings_plan_a_r_n,
       SUM(savings_plan_savings_plan_effective_cost) as sp_cost,
       SUM(pricing_public_on_demand_cost) as on_demand_cost,
       1 - SUM(savings_plan_savings_plan_effective_cost) /
           NULLIF(SUM(pricing_public_on_demand_cost), 0) as savings_rate
FROM cur_database.cur_table
WHERE savings_plan_savings_plan_a_r_n != ''
GROUP BY 1;
```

### CUR 2.0 (Cost and Usage Report Data Exports)

Newer export mechanism via Data Exports service. Supports:

- Parquet format to S3
- Athena integration with auto-table creation
- Split cost allocation (container-level costs for ECS/EKS)
- Customizable columns — export only what you need

### FinOps Framework Summary

```
1. INFORM  → CUR + Cost Explorer + Budgets + Anomaly Detection
2. OPTIMIZE → Right-size + Savings Plans + Spot + Storage tiering
3. OPERATE → Tag enforcement + Budget Actions + automated remediation
4. REPEAT  → Monthly cost reviews + Compute Optimizer checks
```

Key metrics to track: unit cost (cost per transaction/user/request), coverage ratio (% spend on commitments), waste ratio (idle/unused resources), data transfer percentage of total bill.
