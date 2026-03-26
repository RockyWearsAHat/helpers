# SRE Capacity Planning — Load Modeling, Forecasting & Provisioning

## The Capacity Planning Problem

Capacity planning answers: **How much infrastructure do we need, and when?** It sits at the intersection of three concerns: cost (infrastructure spending), risk (being overwhelmed by traffic), and velocity (deploying changes and scaling without delays). The stakes are concrete—undersizing means incidents and poor user experience; oversizing wastes money; sizing "just right" requires careful forecasting and reserving headroom.

Capacity planning is not a one-time event but a continuous cycle: forecast demand, model system behavior, provision resources, measure actual utilization, update forecasts, repeat.

## Demand Forecasting

### Forecasting Methods

**Historical trend analysis** is the starting point. Plot metrics over time (requests per second, data stored, user count), fit curves, and extrapolate. Common patterns:

- **Linear growth**: Steady, predictable increase (rare in early-stage systems)
- **Exponential growth**: Compounding adoption; common in viral products or new markets
- **Seasonal patterns**: Predictable spikes (holiday shopping, tax season, school calendar)
- **Step changes**: Sudden shifts from product launches, marketing campaigns, or competitor events
- **Oscillating behavior**: Hourly, daily, or weekly cycles overlaid on trend

**Time series methods** (exponential smoothing, ARIMA, Prophet) handle seasonality and trend decomposition automatically. Start simple—a 90-day moving average often outperforms complex models when data is noisy.

### Sources of Uncertainty

Forecasts always fail at the tail. Confidence intervals widen as you look further ahead. Strategies:

- **Shorter planning horizons**: Forecast 3–6 months ahead with high confidence; beyond that, plan for scenario ranges instead (pessimistic, optimistic, expected)
- **Anchor to leading indicators**: Track product roadmap announcements, marketing spend, customer onboarding cohort sizes—signals arriving before traffic spikes
- **Bound your forecast**: Set a maximum believable growth rate (e.g., "we cannot grow faster than X% per month due to sales velocity")
- **Review and recalibrate monthly**: When forecast deviates from actual, investigate why. Either the model broke, or your assumptions changed

### Demand is not Uniform

Traffic varies by dimension:
- **Geographic region** (time zones, regulatory adoption, CDN regions)
- **Customer tier** (free vs. paid, enterprise vs. SMB)
- **Time of day/week** (business hours, weekends)
- **Application flow** (read- vs. write-heavy, cache hit patterns)

Build forecasts per dimension, not in aggregate. A single "requests per second" forecast hides critical detail about which database is overheating or which region is saturated.

## Load Modeling & Queuing Theory

### Little's Law

The foundation of load modeling. **Little's Law states: In a system at steady state, the average number of items in the system equals the average arrival rate multiplied by the average time each item spends in the system.**

```
L = λ × W

L = average items in system
λ = arrival rate (e.g., requests/sec)
W = average time in system (latency)
```

**Practical implications:**

- If request rate is 1000 req/sec and average latency is 100ms (0.1s), then on average 100 requests are "in flight"
- If latency jumps to 200ms, 200 requests are in flight—more memory consumed, more thread pools needed
- Higher latency feels like capacity; reducing latency is equivalent to gaining capacity

**Limits of Little's Law**: It applies at equilibrium. During traffic surges or when a system is overloaded, queues grow, latencies spike, and the relationship holds but within different bounds. Use it to model steady-state behavior, not transient failures.

### M/M/1 Queuing Model

The simplest stochastic model. Assumes:
- **Arrivals follow Poisson distribution** (random, independent requests)
- **Service times are exponentially distributed** (memoryless; a request taking 50ms has no correlation with the next)
- **One server**
- **FIFO queue**

Key results for M/M/1:

| Metric | Formula | Insight |
|--------|---------|---------|
| Traffic intensity | ρ = λ / μ (arrival rate / service rate) | Must be < 1 or queue grows unbounded |
| Average queue length | ρ / (1 - ρ) | Grows dramatically as ρ approaches 1 |
| Average time in system | 1 / (μ - λ) | Latency grows nonlinearly as utilization climbs |
| P(wait > t) | e^(-(μ-λ)t) | Probability of excessive delay |

**The cliff effect**: At ρ = 0.7 (70% utilization), average wait time is 2.3× service time. At ρ = 0.9, it's 9×. Small utilization gains near capacity have massive latency costs—this is why reliable systems target 50–70% steady-state utilization on peak-traffic dimensions.

**Why M/M/1 breaks in practice**: Real requests aren't Poisson (they cluster), service times aren't exponential (some queries are always fast, others always slow), and systems aren't single-server. But the mental model—that high utilization drives nonlinear latency growth—holds broadly and justifies headroom.

## Capacity Testing

### Load Testing

Generate realistic traffic to measure how the system responds. Load tests answer: "At what traffic level do we hit our latency SLOs? Where do errors appear?"

**Test design**: Replicate actual request distribution (80% reads, 20% writes; 90% short queries, 10% long). Ramp traffic gradually to observe scaling behavior and identify breaking points. Common patterns:

- **Ramped load**: 0 → peak over minutes, measuring when latency degradation becomes noticeable
- **Sustained load**: Hold at peak for an extended period; observe memory leaks, connection pool exhaustion, cache misses
- **Spike**: Sudden traffic surge; measures how queuing handles transient overload
- **Soak**: Low traffic for days; catches leaks and corner cases

### Stress Testing

Push past expected limits. Where does the system fail? Gracefully (rejecting requests, returning errors) or catastrophically (cascading failures, resource exhaustion)? Identify failure modes so you can defend against them in production.

## Provisioning & Headroom

### Headroom Calculation

Headroom is reserved capacity not consumed by current demand. Calculate it by scenario:

**Baseline**: Current peak traffic + 20–30% headroom for foreseeable growth over the next planning period (typically 6–12 months pre-provisioned, 3–6 months on-demand).

**Safety margins** for overprovisioning:
- **Unknown unknowns** (surge from unexpected PR, campaign, competitor event): Add 50% of peak
- **Failure resilience** (losing a chunk of infrastructure): Add capacity equal to your target failure fraction (if 3 zones can lose one, add 33%)
- **Auto-scaling fallback time**: If autoscaling takes 5 minutes to spin up, maintain headroom for ~5 min of peak traffic
- **Business criticality**: High-criticality services justify higher headroom; lower-criticality can run leaner

Example: If peak is 10k req/sec and you want to handle 1.5× surge with 20% headroom, provision for: 10k × 1.5 × 1.2 = 18k req/sec capability.

## Autoscaling Strategies

### Reactive Autoscaling

Scale in response to observed metrics (CPU, memory, request queue depth). Common triggers:

- **CPU > 70%** (or custom metric)
- **Request latency p99 > 200ms**
- **Queue depth > threshold**

**Lag**: There's always a delay—time to detect, time to provision infrastructure, time for warmup. Expect 2–10 minutes on cloud platforms. During that window, requests queue up and latency degrades.

**Oscillation**: If thresholds are too aggressive, the system scales up, load drops (because requests were just queued), then scales down, causing herding behavior. Use hysteresis (different thresholds for scaling up vs. down) or gradual scaling policies.

### Predictive Autoscaling

Scale in advance using forecasts. Machine learning models learn historical patterns (time of day, day of week, business events) and provision capacity proactively. Benefits: smooth scaling, no lag-induced latency spikes. Drawback: models fail on novel events (competitor disaster, viral moment).

**Hybrid approach**: Predictive as primary, reactive as safety net.

### Scheduled Autoscaling

Hardcode scaling rules for known patterns (e.g., "Monday 8am, scale to 15k capacity; Friday 6pm, scale to 5k"). Zero surprise, but brittle—unexpected events aren't handled.

## Cost-Capacity Trade-Offs

Capacity planning is always a cost optimization problem. Three levers:

| Approach | Cost | Risk | Notes |
|----------|------|------|-------|
| Oversized static infrastructure | High spending, stable | Low; unused capacity absorbs spikes | Predictable, wastes money; common for critical services |
| Tight sizing + aggressive autoscaling | Lower base cost, high scaling costs | High; lag during scale-up, errors if scale fails | Best during steady state; risky during traffic jams |
| Smart reservations (spot instances, reserved capacity) | Medium | Medium; mixed capacity types have behavior quirks | Balance: predictable base + burst for spikes |

The break-even point depends on your traffic variability and tolerance for latency degradation.

## Provisioning Dimensions

Capacity isn't monolithic. Identify bottlenecks by component:

- **CPU**: Needed for compute-heavy tasks (request parsing, serialization, complex queries)
- **Memory**: Caching, in-memory state, data structures
- **Network I/O**: Egress bandwidth, connection pools, serialization format (JSON vs. binary)
- **Storage I/O**: Database query throughput, disk seeks, replication overhead
- **Concurrency limits**: Thread pools, connection limits, database connection limits

A system can have idle CPU while network or database is saturated. Forecast each dimension, don't just scale the whole.

## Mental Model

Capacity planning is a feedback loop: forecast demand, model steady-state behavior and tail risk using queuing intuition, provision infrastructure with headroom, autoscale smoothly, measure actual behavior, and update forecasts. The key insight is that utilization and latency are coupled nonlinearly—past ~70% utilization, modest traffic increases cause severe latency degradation. This justifies maintaining headroom and preferring smooth autoscaling over just-in-time provisioning.