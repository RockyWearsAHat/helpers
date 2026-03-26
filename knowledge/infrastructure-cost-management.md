# Infrastructure — Cost Management in Cloud: FinOps Lifecycle, Visibility, Optimization Levers

## Overview

Cloud cost management balances efficiency with business value. Costs are granular (millions of line items per month) and often invisible until bill arrives. FinOps (Financial Operations) provides a structured lifecycle: **Inform** (visibility), **Optimize** (reduce waste), **Operate** (governance + accountability). This note covers cost visibility (tagging, allocation models), optimization levers (reserved instances, spot, right-sizing), showback/chargeback models, and unit economics.

## FinOps Lifecycle: Three Phases

### Phase 1: Inform (Cost Visibility)

**Goal**: Understand where money goes. Establish baseline. Enable cost-aware decision-making.

#### Tagging Strategy

Apply consistent tags to all resources (compute, storage, databases). Tags enable cost allocation.

**Tag dimensions** (choose per organization):

- **Business**: cost-center, department, product-line
- **Technical**: environment (prod/staging/dev), team, application, component
- **Operational**: managed-by, data-classification, compliance-domain

**Example tag set (AWS)**:
```json
{
  "Environment": "production",
  "Team": "platform-eng",
  "CostCenter": "engineering",
  "Application": "checkout-service",
  "ManagedBy": "terraform",
  "DataClassification": "customer-pii"
}
```

**Enforcement**: IAM policies or CloudFormation/Terraform linters reject resources without required tags.

#### Cost Allocation Models

**Chargeback** (direct billing): Teams receive invoice for their usage. Builds cost consciousness. Requires mature tagging + clear business unit hierarchy.

**Showback** (dashboard only): Display costs to teams without billing. Low friction, high visibility, less accountability. Useful for awareness phase.

**Hybrid**: 3-month showback baseline; then transition to chargeback with monthly budgets and variance alerts (cap unexpected spikes to ±20% of baseline).

**Service catalog cost transparency**: Publish cost per transaction/user/request for each internal service:
- Database queries: $0.0001 per 1M requests (RDS with RI amortized)
- Cache hits: $0.00001 per 1M requests (ElastiCache)
- ML inference: $0.05 per request (SageMaker)

Teams use catalog to estimate cost of new features before building.

#### Visibility Tools

- **AWS Cost Explorer**: Aggregate costs by tag, time period, service. Visual trends and anomaly alerts.
- **Cloudability/CloudHealth**: Third-party vendor; multi-cloud cost visualization + budget enforcement.
- **Open-source**: OpenCost (CNCF); Kubecost (Kubernetes-specific resource cost tracking).

**Setup time**: 1-2 weeks to design tag taxonomy, implement enforcement, train teams. Payoff: 10-20% cost reduction just from awareness.

### Phase 2: Optimize (Reduce Waste)

**Goal**: Eliminate inefficiency; shift spending from waste to value.

#### Right-Sizing

Match resource capacity to actual usage. Common waste patterns:

- **Over-provisioned VM**: t2.xlarge instance with 10% CPU utilization (provision t2.small instead)
- **Idle databases**: RDS with zero connections (shut down or downgrade storage)
- **Unused data transfer**: Inter-AZ data transfer billed at $0.02/GB (consolidate to single AZ if latency allows)

**Discovery**:
1. Enable CloudWatch detailed monitoring (5-minute granularity)
2. Query 30-day utilization (CPU, memory, network) per resource
3. Identify outliers (< 20% utilization)
4. Right-size: new instance tier, downgrade storage, or terminate

**Risk**: Right-sizing too aggressively causes production incidents. Use staged approach: first 10%, then 30%, then 50% of identified inefficiency.

#### Commitment Discounts

Cloud providers offer 30-70% discounts for advance commitment (1- or 3-year terms).

| Model | Discount | Flexibility | Commitment Risk |
|-------|----------|-------------|-----------------|
| On-demand | 0% | Full | None |
| 1-year RI | 30-40% | Locked instance type + region | Sunk cost |
| 3-year RI | 50-70% | Locked instance type + region | High sunk cost |
| Convertible RI (1yr) | 25-35% | Change instance family | Medium |
| Savings Plan (compute, 1yr) | 30-50% | Any compute service same region | Low |
| Savings Plan (compute, 3yr) | 50-70% | Any compute service any region | Unknown future workload |
| Spot instances | 70-90% | Interruptible; can be reclaimed | Workload must tolerate interruption |

**Purchasing strategy**:
1. Baseline stable workload: 3-year RIs (baseline × 80%) for prod databases, always-on web servers
2. Variable workload: 1-year Savings Plan (covers predictable 60% of compute)
3. Burst: Spot instances + on-demand flex (dynamic scaling)
4. Exploration: On-demand (full flexibility; pay premium for learning)

**Example (AWS EC2)**: Historically 60-70% on-demand utilization identified; commit 65% via 3yr RIs; scale remaining 35% with spot/on-demand.

#### Spot Instance Strategy

Interruptible VMs at 70-90% discount; AWS can terminate with 2-minute notice.

**Workload fit**:
- ✅ Stateless batch processing (data pipeline, CI runner)
- ✅ Auto-scaling clusters (add/remove nodes reactively)
- ❌ Stateful databases (termination = data loss)
- ❌ Long single transactions (kills mid-run)

**Implementation**:
```yaml
# Kubernetes Spot node group via Karpenter
apiVersion: karpenter.sh/v1alpha5
kind: Provisioner
metadata:
  name: spot-provisioner
spec:
  requirements:
    - key: karpenter.sh/capacity-type
      operator: In
      values: ["spot"]
  limits:
    resources:
      cpu: 1000  # Max 1000 vCPU across spot nodes
  ttlSecondsUntilExpired: 604800  # Rotate after 7 days
```

**Risk mitigation**: Mix on-demand + spot (30% on-demand floor); handle interruptions in app code (graceful shutdown, state persistence).

#### Storage Optimization

- **Intelligent tiering**: Automatically move unused data to cheaper storage class (S3 Standard → Infrequent Access → Glacier). Cuts cost 60-70% for infrequent data.
- **Compression**: Gzip/snappy reduce storage 70-90% for logs, backups (retrieval overhead minimal).
- **Lifecycle policies**: Delete old logs (S3 → delete after 90 days), archive snapshots (EBS → S3 Glacier).
- **Database storage**: Turn off automated backups for non-prod, compress backups, delete old snapshots.

**Example (S3 lifecycle)**:
```xml
<LifecycleConfiguration>
  <Rule>
    <Prefix>logs/</Prefix>
    <Transition>
      <Days>30</Days>
      <StorageClass>STANDARD_IA</StorageClass>
    </Transition>
    <Expiration>
      <Days>90</Days>
    </Expiration>
  </Rule>
</LifecycleConfiguration>
```

#### Data Transfer Minimization

- Inter-AZ transfer: $0.02/GB (vs. intra-AZ free). Consolidate to single AZ or use S3 Transfer Acceleration if necessary.
- NAT Gateway: $0.045/GB for outbound data transfer. Pre-cache large downloads, batch requests.
- CloudFront CDN: $0.085/GB but hits edge first (reduces origin bandwidth). ROI positive if > 500 TB/month.

#### License Optimization

- Bring Your Own License (BYOL): If you own software licenses, use BYOL VMs (no license included in EC2 pricing).
- Open-source alternatives: Replace paid software with equivalent OSS (e.g., PostgreSQL vs. proprietary DB).
- Unused subscriptions: Audit SaaS spend; cancel trials.

### Phase 3: Operate (Governance + Continuous Optimization)

**Goal**: Sustain optimization; prevent cost creep as system grows.

#### Budget Controls

Set budgets per team/project/environment. Alert when spending exceeds threshold.

```yaml
# AWS Budget
{
  "BudgetName": "platform-team-monthly",
  "BudgetLimit": {"Amount": "50000", "Unit": "USD"},
  "TimeUnit": "MONTHLY",
  "NotificationWithSubscribers": [
    {
      "Notification": {"ComparisonOperator": "GREATER_THAN", "NotificationType": "ACTUAL", "Threshold": 80},
      "Subscribers": [{"SubscriptionType": "EMAIL", "SubscriptionAddress": "alerts@platform.internal"}]
    }
  ]
}
```

#### Anomaly Detection

Monitor spend trends; alert on unusual spikes.

- **CloudWatch Anomaly Detector**: Baseline 2 weeks of spending; flag variance > 2σ
- **Custom logic**: If daily spend > yesterday × 1.5, investigate

**Common spikes** (false alarms):
- Backup runs (scheduled, expected)
- Load test (planned capacity test)
- DDoS mitigation (briefly scale auto-scaling)

**Tune alert thresholds** to reduce noise: set band ±30% before alerting.

#### Reserved Instance Management

1. **Utilization tracking**: RI Utilization Report in AWS Cost Explorer; target > 90%
2. **Idle RIs**: Identify unused RIs; sell on RI Marketplace (recover 50-80% cost)
3. **Renewal**: Plan renewals 2 months before expiry; revisit sizing (workload may have changed)

#### Chargeback Variance Analysis

Compare forecasted cost (from service catalog) vs. actual billed cost monthly. Investigate gaps > 10%.

**Common variances**:
- Data transfer higher than forecast (unanticipated inter-AZ traffic)
- Database storage grew faster than expected (log table explosion)
- Spot price volatility (reserve additional on-demand buffer)

## Unit Economics

Measure cost per business outcome. Examples:

- **SaaS**: Cost per user per month = (compute + storage + data transfer) / active users
- **E-commerce**: Cost per order = (web server + database + CDN + payment processing) / orders
- **ML**: Cost per inference = (GPU compute + storage + data transfer + model serving) / predictions

**Spreadsheet model** (monthly):

| Component | Cost | Driver | Unit |
|-----------|------|--------|------|
| Compute (EC2 + RDS) | $30k | CPU-hours * $0.05/hr | compute-hour |
| Storage (S3 + EBS) | $5k | GB * $0.023/GB | storage-GB |
| Data transfer | $3k | GB transferred * $0.02/GB | transfer-GB |
| **Total** | **$38k** | — | — |
| **Users (monthly)** | 100k | DAU * 20 | users |
| **Cost per user** | **$0.38** | Total / users | $/user |

**Use**: Answer "should we build feature X?" Cost per user × projected users × 12 months = annual cost of feature.

## See Also

- [FinOps Fundamentals](cloud-finops.md) — Three-phase lifecycle, tagging taxonomy, chargeback models
- [Cloud Cost Optimization](cloud-cost-optimization.md) — RIs, Savings Plans, spot instances, right-sizing detail
- [Infrastructure Capacity Planning](infrastructure-capacity-planning.md) — Forecasting and reserve planning
- [Observability Metrics](devops-observability-patterns.md) — Cost instrumentation in application code