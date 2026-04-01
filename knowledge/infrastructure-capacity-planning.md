# Capacity Planning — Forecasting, Modeling, Autoscaling & Right-Sizing

## Capacity Planning Problem Statement

Capacity planning answers: **How much infrastructure do we need, when?** It sits between demand (business growth, user adoption) and cost (compute, storage, network bills). Under-provisioning causes service degradation; over-provisioning wastes capital.

Capacity decisions happen on multiple timescales:

- **Real-time**: Autoscaling responding to current load (seconds to minutes).
- **Tactical**: Provisioning for next quarter's peak (weeks to months).
- **Strategic**: Long-term infrastructure roadmap, vendor choices, architectural decisions (years).

## Demand Forecasting

Accurate forecasting informs all downstream capacity decisions. Methods range from statistical to qualitative.

### Time Series Forecasting

**Trend analysis**: Project linear or polynomial growth from historical data.

```
Traffic (RPS) = 10,000 + 500 * week_number  # Linear growth
```

Simple but brittle—assumes past growth continues. Breaks during acquisition events, marketing pushes, or competitive disruption.

**Seasonal decomposition**: Separate trend from seasonal patterns (weekday peaks, holiday valleys, monthly cycles).

```
Usage = Trend + Seasonal + Irregular
# Example: Daily traffic peaks at 8am, valleys at midnight; weekly peaks Monday-Friday
```

Assumes seasonality repeats. Fails if business model changes (shift from B2B to B2C changes weekly patterns).

**Auto-regressive models (ARIMA, Prophet)**: Leverage historical autocorrelation to forecast. More sophisticated than trend, capture oscillations and anomalies.

Useful for 1-week to 3-month horizons. Beyond that, uncertainty dominates.

### Business-Driven Forecasting

Pure data extrapolation ignores business reality.

**Known events**:
- Marketing campaigns (predicted 30% traffic increase)
- Product launches (new feature, expected usage spike)
- Seasonal peaks (holiday shopping, exam periods)
- Contracts/commitments (new customer SLA guarantees)

Blend data-driven forecasts with qualitative business input:

```
Q2_peak_traffic = Q1_trend_projection * 1.3 (campaign boost) * 0.9 (holiday decline)
```

### Forecast Accuracy and Margins

Forecasts are **always wrong**. Account for uncertainty:

- **Confidence interval**: 80% likelihood traffic falls within this range.
- **Headroom**: Extra capacity beyond forecast to handle surprises (typically 20-50%).

Example:
- Forecast: 10,000 RPS
- 95% confidence interval: 8,000 - 12,500 RPS
- Headroom: +30% of max = 12,500 * 1.3 = 16,250 RPS capacity provisioned

Headroom trades cost against reliability. Higher headroom tolerates larger forecast errors.

## Resource Modeling

Map application behavior to infrastructure resources.

### CPU and Memory

Profile the application under known load to establish ratios:

```
1 RPS = 0.1 CPU cores, 256 MB memory
10,000 RPS = 1,000 CPU cores, 2.6 GB memory per instance
```

Profiling under production-representative load captures all overhead (garbage collection, framework cruft, background tasks).

**Gotchas**:
- Different workloads have different ratios (CPU-bound crypto ≠ I/O-bound database queries).
- Ratios change with code updates, framework upgrades.
- Per-CPU memory (memory / CPU) varies; some apps are memory-heavy, others CPU-heavy.
- Burst behavior: peak memory consumption differs from average.

### I/O Bandwidth and IOPS

Network-dependent applications need models:

```
1 RPS = 500 KB network I/O (request + response)
10,000 RPS = 5 GB/sec network I/O
```

Database-intensive applications profile I/O operations per second:

```
1 transaction = 3 database queries = 3 IOPS
10,000 TPS = 30,000 IOPS
```

Cloud storage (S3, EBS, DynamoDB) charges per IOPS tier. Under-provisioning causes throttling; over-provisioning wastes capacity.

### Cost per RPS

Translate capacity into cost:

```
Cost/RPS = (Instance Cost + Storage Cost + Network Cost) / Throughput

Example:
- 8-core instance: $0.30/hr = $262/month
- Serves 1,000 RPS sustainably
- Cost/RPS = $262 / (1000 * 2.6M seconds) = $0.0000001/RPS/second = ~$8.27/RPS/year
```

Unit economics reveal optimization opportunities (batch cheaper than streaming, in-memory cache cheaper than repeated DB queries).

## Headroom and Margins

Headroom is spare capacity above expected peak, providing buffer against forecast error and unexpected spikes.

### Typical Headroom Allocation

```
Target Utilization: 70%
Peak Forecast: 10,000 RPS
Required Capacity: 10,000 RPS / 0.70 = 14,286 RPS
```

70% utilization means 30% headroom—enough to absorb forecast error and handle 1.4x spike before degradation.

**Utilization curves**: Higher utilization (80%, 90%) is more cost-efficient but leaves less margin for error.

```
Utilization | Headroom | Cost Efficiency | Risk
70%         | 30%      | Low            | Medium (forecast errors manageable)
80%         | 20%      | Medium         | Medium-High
90%         | 10%      | High           | High (brittle)
```

### Headroom for Different Tiers

- **Development**: 10-20% headroom (cheap, fault-tolerant, cost-conscious)
- **Staging**: 20-30% headroom (mirrors production demand, fast feedback)
- **Production**: 30-50% headroom (critical, less tolerance for outages)

## Autoscaling

Autoscaling translates short-term demand into dynamic capacity, reducing need for overprovisioning.

### Scaling Triggers

**CPU-based**: Scale up when CPU > 70%, down when < 30%. Common but crude—CPU doesn't always correlate with business relevance (authentication CPU-heavy but user-impact low).

**Metric-based**: Scale on application-specific metrics (RPS, transaction latency, queue depth):

```
Scale up if: (Average RPS) > 5,000 for 2 minutes
Scale down if: (Average RPS) < 2,000 for 10 minutes
```

**Event-driven**: Preemptive scaling based on predicted demand (known events, time-of-day patterns).

```
Scale to 20,000 RPS capacity every weekday 7am-9am
```

### Scaling Policies

**Simple scaling**: Linear scale-up/down. Slow, reactive.

**Target tracking**: Maintain a specific metric target (70% CPU). Responsive, handles variability automatically.

**Step scaling**: Different scaling rates based on magnitude of breach (small breach = +1 instance; large breach = +5 instances). Balances responsiveness and cost.

```
If CPU > 90%: Add 5 instances immediately
If CPU > 75%: Add 2 instances
If CPU > 70%: Add 1 instance
If CPU < 30%: Remove 1 instance (with 5-minute delay)
```

### Autoscaling Limits

- **Minimum instance count**: Prevents under-capacity during scale-down.
- **Maximum instance count**: Cost control, prevents runaway scaling.
- **Cooldown period**: Prevents thrashing (scaling up/down repeatedly). Typically 5-10 minutes.

### Regional and Geographic Scaling

Distribute load across regions to absorb regional failures and reduce latency:

- **Active-active**: Traffic routed to nearest region, scales independently.
- **Active-passive**: Standby region receives failover traffic on primary outage.

Scaling across regions requires data consistency and network cost assumptions.

## Load Testing for Capacity Planning

Load tests validate capacity models and scaling behavior under production-like conditions.

### Load Test Scenarios

**Steady-state load**: Run at expected peak for 30-60 minutes, observe:
- Average response latency
- P95, P99 latencies (tail behavior)
- Error rate
- Resource utilization (CPU, memory, disk I/O)

**Ramp-up**: Gradually increase load from baseline to peak (e.g., 1,000 → 10,000 RPS over 10 minutes), observe when bottlenecks emerge:
- At what load does latency degrade?
- Where does error rate spike?

**Burst**: Sudden load spike (normal → 2x peak in 1 minute), observe:
- How long to autoscale?
- Do requests queue or fail?

**Soak test**: Run at moderate-high load (80% peak) for hours, observe:
- Memory leaks (heap grows over time)
- Thread leak in connection pools
- Cache pollution

**Multi-tier**: Load multiple components simultaneously (frontend + backend + database), observe:
- Which tier is the constraint?
- Does one tier's saturation cascade to others?

### Tools

- **Apache JMeter**: HTTP load generation, monitoring, reporting.
- **Locust**: Python-based, scenarios-as-code, distributed load generation.
- **k6**: Modern, ES6 scripting, cloud-based execution.
- **gatling**: Scala-based, high-throughput, detailed metrics.

## Right-Sizing

Identify instances that are over-provisioned or mis-sized.

### Utilization Analysis

Monitor CPU, memory, network utilization over 1-3 months:

```
Instance type t3.large (2 CPU, 8 GB RAM):
- Average CPU: 15%
- Peak CPU: 35%
- Average memory: 1.2 GB
- Peak memory: 2.8 GB
• Recommendation: Downsize to t3.medium (2 CPU, 4 GB RAM, half cost)
```

More aggressive right-sizing (move 15% avg CPU to t3.small, 2 CPU 2b RAM) risks inadequacy if workload changes.

### Cost vs. Performance Trade-offs

```
Scenario A: Large instances (fewer, more headroom)
- 4 x t3.large = $3.40/hour
- Scales poorly, long spin-up, less granular resource control
- Better for latency-sensitive, cache-heavy workloads

Scenario B: Many small instances (more, tighter utilization)
- 16 x t3.micro = $3.07/hour
- Scales better, faster failover, more operational complexity
- Better for embarrassingly parallel, stateless workloads
```

Choose based on workload elasticity and operational preferences.

### Reserved Instances and Commitments

Commit to predictable baseline capacity with Reserved Instances (RIs) or compute commitments:

```
On-demand: $0.30/hour
1-year RI: $0.20/hour (33% discount)
3-year RI: $0.18/hour (40% discount)
Savings Plan: Per-compute-hour discount across instance types/regions
```

RIs apply to predicted **baseline** capacity. Burst above baseline uses on-demand. RIs are cost-efficient for predictable, sustained workloads; wasteful if demand shrinks.

## Summary

Capacity planning combines forecasting (business + data-driven), modeling (resources per unit load), load testing (validation), and dynamic scaling (autoscaling). Headroom buffers forecast error; right-sizing optimizes cost. Most mature deployments use a hybrid: baseload covered by RIs/commitments (cost-efficient), burst handled by autoscaling (responsive). Capacity decisions require collaboration between engineering (scaling mechanisms, monitoring), product (demand forecasts, growth plans), and finance (budget, ROI).