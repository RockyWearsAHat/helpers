# Chaos Testing for Applications

## Concept

Chaos testing (often called chaos engineering or fault injection testing) deliberately injects failures into a system to discover how it behaves under degradation. Instead of assuming dependencies work, chaos testing asks: "What happens when this fails?"

Unlike traditional testing (which validates happy paths), chaos testing validates resilience—the ability to degrade gracefully, recover, or alert when things break.

## Scope: Application vs. Infrastructure

### Infrastructure Chaos (Kubernetes, Cloud Services)
Failures injected at the infrastructure layer: kill pods, drop network packets, induce latency, exhaust CPU/memory.

**Tools:** Gremlin, Chaos Mesh, Kube-Chaos, AWS Fault Injection Simulator.

### Application-Layer Chaos
Failures injected at the application layer: fail specific service calls, corrupt data, timeout dependencies, return error codes.

**Tools:** Toxiproxy, request interceptors, custom middleware, feature flags.

**Distinction:** Application-layer chaos is faster to iterate during development and testing. Infrastructure chaos tests operational readiness. Both are valuable.

## Fault Injection Patterns

### Network Faults
- **Latency injection:** Add delays to service calls (50ms, 500ms, 5s). Tests timeout handling and user perception.
- **Packet loss:** Drop X% of requests/responses. Tests retry logic and idempotency.
- **Connection failure:** Close connections before completion. Tests error recovery.
- **Bandwidth throttling:** Simulate slow networks (2G, 3G). Tests timeouts under load.

### Dependency Failures
- **Service unavailable:** Return 5xx or connection refused. Tests fallbacks and circuit breakers.
- **Slow dependency:** Return correct data but slowly. Tests timeout boundaries.
- **Corrupt response:** Return malformed or unexpected data. Tests input validation and error handling.
- **Partial failure:** Some subset of requests fail. Tests graceful degradation.

### Data Failures
- **Stale cache:** Return cached data when live data should be fetched. Tests cache invalidation.
- **Missing fields:** Return incomplete responses. Tests required-field validation.
- **Type mismatches:** Return wrong data types (string instead of number). Tests type checking.

## Tools: Toxiproxy Example

Toxiproxy is a TCP proxy that sits between an application and its dependencies, simulating network conditions in development, testing, and CI.

**Mechanism:** Acts as a middleware—requests flow through Toxiproxy, which injects faults before forwarding to the real dependency.

**Capabilities:**
- Add latency
- Drop connections
- Introduce timeouts
- Simulate bandwidth limits
- Inject errors mid-response

**Usage:** Developers configure Toxiproxy rules in test setup, then assert that the application handles failures correctly.

**Scope:** Works for any TCP-based communication (HTTP, databases, message queues). Not language-specific.

## Chaos Testing in CI/CD

Chaos tests integrate into CI as a gate:
- Defined as test cases (setup faults, verify behavior)
- Run reproducibly in CI environments (where network conditions are controlled)
- Validate that failures don't cascade or cause data corruption

**Consideration:** Chaos tests are slower than unit tests (often involve timeouts or retry delays). Usually run separately from fast feedback tests, triggered on PR or nightly.

## Game Day Testing

A manual, operational variant of chaos testing: teams simulate real incidents in a controlled environment, observing how systems and people respond. Often includes:
- Communication drills (who gets paged, how escalation works)
- Observability validation (can you detect the failure from logs/metrics)
- Runbook accuracy (does the documented recovery procedure work)
- Team coordination (does the on-call engineer know what to do)

**Difference from automated chaos tests:** Game days test human response and operational procedures, not just code resilience. Automated chaos tests validate code behavior.

## Blast Radius and Staging

Chaos testing scales from isolated component tests to system-wide resilience tests.

### Component-Level Chaos
Test a single service with its dependencies in failure states. Validates that timeouts, retries, and circuit breakers work.

**Risk:** Low. Run in any environment. Fast iteration.

### Integration Chaos
Test a cluster of services with cascading failures. Validates that failure in one service doesn't cascade to bring down others.

**Risk:** Medium. Requires staging with realistic load. May create false alerts.

### Production Chaos
Inject faults into production systems under careful observation. Validates real-world resilience.

**Risk:** High. Requires monitoring, dashboards, kill switches, and experienced operators. Limited blast radius (e.g., target 0.1% of traffic). Often done during low-traffic windows.

## Graceful Degradation vs. Cascading Failure

Chaos testing reveals which failures degrade gracefully vs. cascade:

**Graceful degradation:** Service detects dependency failure and serves a degraded response (fallback data, cached response, retry with backoff). Users experience slowness but not a complete outage.

**Cascading failure:** Dependency failure propagates up the call stack, bringing down upstream services. One broken service breaks everything.

Chaos tests validate that designs support degradation:
- Timeouts prevent waiting indefinitely
- Circuit breakers prevent wasted retry storms
- Fallbacks provide acceptable degraded behavior
- Bulkheads isolate failures to specific requests/users

## Mental Model

Chaos testing answers: "What's the weakest link?" By purposely breaking things in controlled ways, you discover failure modes before they happen in production. It's not paranoia; it's preparedness.

The discipline is simple:
1. **Hypothesize:** "If service X fails, users should see Y experience"
2. **Inject:** Break service X
3. **Observe:** Did users see Y experience?
4. **Fix:** If not, add resilience (timeout, retry, fallback)
5. **Repeat:** Until the system is resilient enough

See also: architecture-resilience, testing-integration-e2e, sre-incident-management