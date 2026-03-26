# Performance Testing — Load, Stress, Soak & Statistical Rigor

## Testing Types: Spectrum of Load Scenarios

Performance testing measures system behavior under defined load. The test type determines what load profile is applied and what metrics matter.

### Load Testing (Nominal Load)

Applies steady, expected traffic volume (e.g., "simulate 1000 concurrent users"). Measures response times, throughput, error rates at that nominal load. Answers: "Does the system meet SLAs under expected load?"

Baseline for other tests. Uncovers performance regressions (code changes that degrade latency or throughput). Typical duration: 10-30 minutes after warmup.

### Stress Testing (Breaking Point)

Gradually increases load until system fails or degrades unacceptably. Determines capacity ceiling: "At what load does this break?" and "How does failure manifest?" (hung processes, cascading errors, resource exhaustion).

Uncovers resource leaks, deadlocks, connection pool saturation. Finds the knee of the curve where system transitions from linear degradation (throughput decreases, latency increases proportionally) to cliff (cascading failure).

Typical duration: 20-60 minutes with ramp. Essential before production deployment.

### Soak Testing (Endurance)

Maintains moderate but consistent load for extended duration (hours to days). Uncovers slow-onset failures: memory leaks, connection pool exhaustion, log file growth consuming disk, thread pool starvation, GC pause accumulation.

Real-world production loads are bursty, but systems also see long periods of steady traffic. Soak tests catch bugs that don't manifest in short runs. Particularly valuable for services meant to run continuously.

### Spike Testing

Sudden, dramatic increase in load over seconds. Answers: "Can the system handle unexpected traffic spikes?" Proxies for viral events, deploy-triggered traffic. Reveals whether circuit breakers, bulkheads, backpressure mechanisms work correctly.

### Smoke Testing

Minimal load (single user) to verify system responds. Baseline sanity check; used to gather reference performance metrics. "Shakeout test."

## Benchmark Design & Statistical Rigor

Naive benchmarking (run code N times, report average) introduces systematic errors:

### Warmup Bias

CPUs and runtimes optimize compiled code after first execution. JIT compilers don't generate optimal code until functions are called repeatedly. CPU branch predictors, caches, prefetchers need time to "warm up."

If you measure from cold start, you measure cold-start penalties, not steady-state performance. Solutions:

- **Explicit warmup phase**: Execute benchmark 1000+ times before measuring.
- **JVM-specific**: JIT needs ~10k-100k invocations before stabilization.
- **JavaScript engines**: V8 stabilizes after fewer iterations (100s-1000s).
- **Discard initial results**: Track "ramp time" explicitly; separate cold, ramp, and steady states.

### Percentile Analysis vs. Averages

Reporting average latency masks outliers. A system averaging 50ms with occasional 500ms spikes looks good on average but fails users experiencing spikes.

Percentile breakdown provides distribution shape:

| Percentile | Meaning | Use Case |
|-----------|---------|----------|
| **p50** | Median; 50% of requests faster | Typical user experience |
| **p95** | 95% of requests faster; 5% slower | Tuning target; SLA boundary |
| **p99** | 99% of requests faster; 1% slower | Tail experience; worst-case planning |
| **p99.9**, **p100** | Extreme outliers; heavy tails | Rare but catastrophic failures; never SLA target |

**Why not p100 (max)?** Outlier measurements often reflect GC pauses, kernel scheduler delays, or measurement artifacts. A single 10-second outlier in 1M requests distorts the picture. Focus on p99 to balance signal vs. noise.

**Typical targets:**
- p50: ~20ms
- p95: ~100ms (SLA boundary often set here)
- p99: ~500ms (rare but acceptable spike)

Never chase p100. Set SLAs on p99 or p95.5, not max.

### Variance & Confidence Intervals

Raw measurements vary due to randomness (OS scheduling, competing processes, network jitter). Report confidence intervals, not point estimates:

- **95% CI**: Range where true mean likely lies (95% confidence)
- Requires ~30+ samples minimum; more for high-variance systems

Tools like criterion (Rust), JMH (Java) automatically compute CIs. Proper benchmarking reports:
```
Latency: 42.5 ± 3.2 ms (95% CI)
```

Not just `42.5 ms`.

### Steady State Detection

When is your system truly stable? Latency often improves or stabilizes after initial ramp. Proper analysis:

1. Discard warmup phase (first N% of samples)
2. Analyze ramp phase to find when variance stabilizes
3. Compute metrics only on stable region

Techniques: CUSUM (cumulative sum control charts), change-point detection algorithms.

### Noise & Isolation

Benchmark in isolation:
- Stop background processes
- Disable power management, CPU frequency scaling
- Pin to CPU cores (avoid context switching)
- Use dedicated hardware or cloud instances with reserved capacity

Single measurements on production hardware are unreliable; add 20-50% margin for safety.

## Load Testing Tools Landscape

**JMeter** (Java/GUI):
- Venerable, widely deployed in enterprises
- GUI-based configuration; steep learning curve for complex scenarios
- Best for teams that prefer graphical workflow; weaker for CI/CD as code

**Gatling** (Scala DSL, Java):
- Code-first (Scala DSL or Java); integrates into build pipelines
- Strong reporting; supports distributed testing
- Good for teams comfortable with code-based configuration

**k6** (Go, JavaScript DSL):
- Modern, lightweight, developer-centric
- JavaScript for test scenarios; fast local execution
- Cloud integration (k6 Cloud) for distributed testing
- Excellence for API/HTTP testing; excellent DX

**Locust** (Python):
- Python-based; good for teams already in Python ecosystem
- Distributed testing across multiple nodes
- Lower barrier to entry than Gatling; less feature-rich

**Comparison matrix:**
| Aspect | JMeter | Gatling | k6 | Locust |
|--------|--------|---------|-----|--------|
| Configuration | GUI/XML | Code (Scala) | Code (JS) | Python |
| Learning curve | Steep | Medium | Low | Low |
| CI/CD friendly | Weak | Good | Excellent | Good |
| Distributed | Yes (complex) | Yes | Cloud or DIY | Yes (good) |
| Real-time dashboards | Limited | Good | Excellent | Basic |
| Community size | Large | Medium | Growing | Medium |

## Flame Graphs & Profiling Integration

Performance test failures require root cause analysis. Flame graphs visualize CPU time distribution across the call stack, showing where time is spent.

During load test, capture CPU profiles (sampling-based, low overhead). Generate flame graph: each horizontal block is a function, width proportional to CPU time. Instant visual identification of bottlenecks.

Integration pattern:
1. Run load test under profiler (e.g., perf on Linux, Instruments on macOS, Java Flight Recorder for JVM)
2. Identify regressions or unmet latency targets
3. Generate flame graph from profile
4. Pinpoint hot paths; diff against baseline to find what changed

Tools: `flamegraph.pl`, `async-profiler` (JVM), kernel perf tools.

## Integration with CI/CD

**Pipeline strategy:**
1. **Per-commit baseline**: Run smoke test (minimal load) to catch obvious regressions
2. **Nightly load tests**: Full load profile; capture detailed metrics
3. **Weekly stress tests**: Find capacity ceilings; alert on regressions
4. **Soak tests**: Run over weekend or dedicated environment; alert on leaks

**Gating policy:**
- Block merge on latency regression (p95 > baseline + threshold, e.g., 10%)
- Block on error rate increase
- Warn on but don't block on throughput decline (may be acceptable trade-off)

**Artifact retention:**
- Store reports, profiles, metrics time-series
- Compare against historical baseline (month ago, 3 months ago)
- Alert on slow degradation (5% per week compounding)

## Relationship to Production Monitoring

Performance testing is lab work; production monitoring is field work. Tests model expected conditions; real production has surprises (traffic patterns, cascading failures, dependency latency).

Use tests to:
- Set SLA targets and confidence in them
- Validate changes before deployment
- Find regressions early

Use monitoring to:
- Validate assumptions (do lab SLAs hold in production?)
- Alert when actual p99 breaches SLA
- Correlate performance with business metrics (error rate, revenue impact)

Feedback loop: production anomaly → new performance test scenario → prevent recurrence.

## See Also

- [Web Performance](web-performance.md) — Client-side performance principles
- [Testing Philosophy](testing-philosophy.md) — Test purposes and trade-offs
- [Testing Strategies](testing-strategies.md) — Test pyramid and SLO hierarchy