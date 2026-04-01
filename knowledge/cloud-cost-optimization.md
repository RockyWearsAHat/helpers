# Cloud Cost Optimization — Commitments, Right-Sizing, Unit Economics & FinOps

## Overview

Cloud cost optimization balances cost reduction with business value delivery. Optimization happens at three levels: **commitment discounts** (RIs, Savings Plans, spot instances), **right-sizing** (matching resource capacity to actual usage), and **unit economics** (measuring cost per business outcome). Effective optimization requires collaboration between engineering, ops, and finance via FinOps practices.

## Commitment Discount Models

Cloud providers offer three primary discount tiers: on-demand (no commitment), savings plans/RIs (advance commitment), and spot/preemptible instances (interruptible capacity).

### Reserved Instances (RIs)

Pre-pay for 1- or 3-year term; receive deep discount (30-70% off on-demand) locked to region + instance family/size.

**Discount structure (AWS EC2):**

| Term | Standard | Convertible |
| ---- | -------- | ----------- |
| 1yr  | 40%      | 31%         |
| 3yr  | 60%      | 54%         |

**Payment options:**
- All Upfront: Largest discount; pay full cost at purchase.
- Partial Upfront: ~50% upfront; remainder monthly. Discount ~5% less than All Upfront.
- No Upfront: Finance cost via monthly billing; discount ~5% less than Partial.

**Tradeoffs:**
- **Commitment risk:** Unused RI capacity is sunk cost. If production fails or workload shrinks 30%, 30% of RI cost is wasted (if unused). Mitigation: RI marketplace allows resale of unused RIs for 50-80% of purchase price.
- **Flexibility:** Standard RIs locked to instance type/region (e.g., m5.large in us-east-1a). Convertible RIs allow exchange to different instance families; ~15-20% lower discount but flexibility worth it for exploratory workloads.
- **Capitalization:** Upfront cost is capital expense; monthly reminders come later.

**Best for:** Steady-state workloads (prod databases, always-on web tiers) with stable sizing for 1-2+ years.

**Capacity reservation:** Regional RIs automatically apply discount to any matching AZ/instance size in region. Zonal RIs reserve capacity in specific AZ (prefer regional unless capacity is critical).

### Savings Plans

More flexible than RIs: commit to $ amount (not instance count/type) for 1 or 3 years; discount applies to compatible services.

**Variants (AWS):**
- **Compute Savings Plan:** Flex hourly commitment covering EC2, Fargate, Lambda. Highest flexibility, 20-60% discount.
- **EC2 Instance Savings Plan:** EC2-only, lock to instance family (m5, c6i, etc.), region flexible. 40-70% discount.
- **Database Savings Plan:** RDS, Aurora, DynamoDB, Redshift, ElastiCache. Service-specific, 15-72% discount.

**Example:** "$1000/month Compute Savings Plan" for 1 year = $12,000 commitment. Discount applies to hourly charges from any compatible service up to that rate. Excess usage at on-demand price. Unused commitment is waste.

**Tradeoffs:**
- **Grace period:** Most clouds allow 30-60 day refund window; if workload changes, exchange/cancel within grace period to reduce regret.
- **Scope:** Compute Savings Plans are "container" commitments; if business shifts from EC2 to Lambda, discount still applies.
- **Exposure:** Larger commitment required ($1000/month is significant); if workload isn't predictable, smaller commitment or no commitment safer.

**Best for:** Workloads with predictable baseline usage across flexible service mix; cost-conscious ops teams wanting to lock in savings without rigid RI locks.

### Spot / Preemptible Instances

Pay 30-90% below on-demand; cloud can reclaim capacity with 30 seconds to 5 minutes notice.

**How it works:** Cloud has excess capacity; rather than leave idle, offer at steep discount with interruption clause. When demand increases, reclaim spot instances to serve regular (paying) customers.

**Price model:** Spot price fluctuates continuously based on supply. Daily price swings are normal; pricing unpredictable, complicating budget forecasting.

**Interruption behavior (AWS):**
- 2-minute warning (EC2 Spot Instances) → graceful shutdown opportunity.
- Auto-replacement (Spot Fleet, Auto Scaling Groups) immediately launch replacement after interruption.

**Tradeoffs:**
- **Fault tolerance required:** Only for workloads tolerating interruption (batch jobs, background workers, stateless containers with orchestrator). Interactive services, databases risky.
- **Cost volatility:** Budget uncertainty; one day $0.08/hr, next day $0.25/hr. Savings unachievable if pricing spikes during peak (defeats purpose).
- **Procurement complexity:** Requires multi-instance type sprawl (fallback options if primary type unavailable).

**Practical use:** Combine spot + reserved tiers: RIs for baseline load (always available), spot for variable demand (cheap but risky). Cost: baseline via RI + peak via spot is often cheaper than all on-demand.

**Example:**
```
Baseline: 50 instances on RI @ $0.10/hr = $44K/month
Peak load: 20 instances on spot @ $0.03/hr = $5.3K/month
Total: ~$50K/month (all on-demand would be ~$80K)
```

## Right-Sizing: Matching Resources to Usage

### Analysis Workflow

**Step 1: Gather utilization data (CloudWatch, Prometheus, GCP Monitoring)**

```
For each running EC2 instance, capture:
  - CPU utilization (average, p95, p99 over 30 days)
  - Memory utilization (if CloudWatch Agent installed)
  - Network I/O
  - Disk I/O
```

**Step 2: Compare to current capacity**

```
Instance m5.xlarge has:
  - 4 vCPU, 16 GiB RAM
Actual usage (avg):
  - 0.5 vCPU (12.5% of capacity)
  - 2 GiB RAM (12.5% of capacity)
```

**Step 3: Recommend right-sized instance**

Most tools (AWS Cost Explorer, CloudHealth, Densify) recommend lower-cost instance type with headroom for spikes.

```
Recommendation: m5.large (2 vCPU, 8 GiB) provides 4× headroom; cost ~50% less
```

**Step 4: Validate and apply**

Before migrating prod, test on dev. Monitor post-migration for performance regression.

### Challenges

**Seasonality:** Utilization varies by time of week/year. Summer e-commerce lower than Black Friday. Analysis must span peak season.

**Sparse data:** Instance running 1 month doesn't give 12-month trend. RIs purchased based on short history may mismatch future demand.

**Tail latency:** Right-sizing based on average CPU (50%) is wrong if p99 CPU is 95%. Average-based sizing causes performance issues during peak.

**Exception handling:** One instance may be right-sized for 11 months, then spike during quarterly report generation. Provisioning for spike wastes money 11 months. Better: use auto-scaling instead of fixed sizing.

## Cost Allocation via Tagging

Granular, unstructured billing data (millions of line items) requires structure to answer business questions:

- "Which team owns this cost?"
- "What's the cost per customer?"
- "Which projects aren't profitable?"

### Tag Strategy

Apply tags at resource creation:

```
All EC2 instances:
  - Environment: prod | staging | dev
  - Team: backend | frontend | infra
  - CostCenter: 12345
  - Application: order-service
  - Owner: alice@example.com
```

CloudFormation / Terraform can enforce tags (fail deployment if missing).

### Cost Allocation Reports

Aggregate by tag:

```
Team | Total Cost
backend | $50,000
frontend | $30,000
infra | $20,000

Environment | Total Cost
prod | $80,000
staging | $15,000
dev | $5,000
```

### Chargeback Models

**Showback:** Display costs to teams; no financial accountability. Result: teams rarely optimize ("someone else pays").

**Chargeback:** Bill teams for costs; enforce budgets. Result: cost consciousness, but risk teams `hoard` capacity to avoid spikes.

**Hybrid:** Showback first 6 months (establish baselines), then chargeback with guardrails (cap monthly overage at +20% vs. baseline to avoid sticker shock).

## Anomaly Detection

### Statistical Detection

Cloud spend is generally predictable if costs are categorized properly (prod vs. dev, services isolated). Anomalies:

```
Daily prod cost: $1000 ± $100 (normal variation)
Yesterday: $2500 (25% spike, investigate)
  → Possible cause: load spike, runaway job, forgotten dev resource in prod, DDoS
```

**Tools:** Cost anomaly detection (AWS Anomaly Detector, GCP Monitoring, Datadog) flag spend exceeding historical variance.

**Action:** Alert ops on anomaly → investigate → terminate runaway job.

## Commitment Selection Strategy

Choosing between on-demand, RI/Savings Plans, and spot:

### Predictable Baseline (70%+ utilization stable, varies < 20%)

→ **Reserved Instances / Savings Plans (60-75% discount)** for baseline + spot for peaks.

Example: Web platform with steady 100 users, spikes 200 during marketing campaigns.

```
100 users baseline: Reserve capacity, cost $10K/month
200+ users peaks: Spot instances, cost $2K/peak month
Total: ~$12K (vs. $18K all on-demand)
```

### Bursty / Unpredictable (30-50% utilization, high variance)

→ **On-demand or Compute Savings Plans** (20-30% discount). Avoid RIs (high waste risk).

Example: Data science batch jobs, triggered on-demand.

```
On-demand: pay $0.10/hr for 100 hours = $10
RI: pay $5000 upfront for 50,000 hours; use 100 hours = $0.10 + $0.10 (wasted) = $0.20/hr effective
On-demand wins.
```

### Fault-Tolerant Batch

→ **Spot instances (60-90% discount)**, multi-type fallback for interruption resilience.

Example: ML training job ($500/day on-demand, can be re-run if interrupted).

```
Spot (best price): $30/day average, interruptions require re-run (cost ~$30 per interruption, ~1x/week expected)
Average cost: $30 + $30/7 ≈ $34/day (still cheaper than on-demand)
Total: ~$1K/month (vs. $15K on-demand)
```

## Cost-Aware Architecture Patterns

### Serverless for Bursty Workloads

Lambda: pay per invocation. EC2: pay per hour (even idle).

For workload averaging 10 hours/month, serverless cheaper. For 300+ hours/month, reserved EC2 cheaper.

**Threshold:** Break-even typically 100-200 hours/month (depends on workload size, language).

### Batch Processing Windows

Instead of continuous processing, batch during off-peak hours (off-peak pricing available in some clouds).

Example: Log processing every 6 hours during night (3x cheaper than daytime processing).

### Geographic Arbitrage

Some cloud regions 40%+ cheaper (India, South America). If latency tolerance exists, relocate workloads.

Example: Analytics batch job doesn't require fast response; run in cheapest region, store results in primary region.

### Consolidation

Multiple small services = overhead per service (orchestration, management, licensing). Consolidating to larger services (microservices → monolith, or multi-tenant architecture) reduces per-unit cost but increases blast radius.

## See Also

- **cloud-finops:** Unit economics and organizational FinOps practices
- **cloud-aws-cost:** AWS-specific cost management tools
- **cloud-multi-cloud:** Multi-cloud cost comparison
- **architecture-resilience:** Cost-reliability tradeoffs