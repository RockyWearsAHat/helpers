# Load Testing — Tools, Scripting, Metrics, and Distributed Generation

## Overview

Load testing measures how a system behaves under sustained or increasing traffic. Unlike performance testing (which measures response times at nominal load), load testing encompasses multiple test scenarios — load, stress, spike, soak — to identify breaking points and bottlenecks.

See [testing-performance.md](testing-performance.md) for conceptual framework and metric interpretation. This note focuses on tools, scripting patterns, and execution strategies.

## Load Testing Tools Landscape

### k6 (Grafana Labs)

k6 is a modern, CLI-first load testing tool using JavaScript for test scripting. Designed for DevOps and CI/CD integration.

**Strengths**:
- Tests written in JavaScript; closer to web developer experience
- Built-in support for distributed execution (cloud runners)
- Modules for common scenarios (HTML form submission, cookie handling)
- Real-time metrics on stdout; integrates with Grafana for visualization
- Lightweight; runs locally without heavy setup

**Example**:
```javascript
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '2m', target: 100 },  // Ramp-up to 100 VUs
    { duration: '5m', target: 100 },  // Hold at 100
    { duration: '2m', target: 0 },    // Ramp-down
  ],
  thresholds: {
    http_req_duration: ['p(95)<200'],  // 95th percentile < 200ms
  },
};

export default function () {
  const res = http.post('https://api.example.com/login', {
    username: 'user',
    password: 'pass',
  });
  check(res, {
    'login OK': (r) => r.status === 200,
  });
  sleep(1);
}
```

**Tool fit**: Modern applications, CI/CD pipelines, teams familiar with JavaScript.

### Apache JMeter

JMeter is a Java-based tool with GUI and command-line interfaces. Mature, widely used, extensive plugin ecosystem.

**Strengths**:
- GUI test builder for quick scenario creation
- Rich plugin ecosystem (database, message queues, MQTT, etc.)
- Thread group simulation (threads represent VUs)
- Detailed result analysis and reporting
- Free and open-source

**Limitations**:
- GUI-based design doesn't scale well (complex scenarios require manual XML editing)
- Test scripts (JMX files) are XML; version control and diffing is awkward
- Memory usage scales with VU count; distributed setup required for high loads
- Steep learning curve for scriptless users

**Typical workflow**:
1. Build scenario in GUI (login → browse → add to cart → checkout)
2. Configure load profile (ramp-up, hold, ramp-down)
3. Run locally or distribute via slave agents
4. Analyze results with built-in reports

**Tool fit**: Enterprise environments, non-technical test designers, complex protocol support.

### Locust (Python)

Locust is a Python-based load testing tool where test logic is written as Python code. Designed for developers.

**Strengths**:
- Python as test language; no special DSL
- Lightweight; easy to scale to high VU counts
- Web UI for real-time monitoring and on-the-fly load adjustment
- Minimal dependencies; runs anywhere Python runs
- Open-source

**Example**:
```python
from locust import HttpUser, task, between

class WebsiteUser(HttpUser):
    wait_time = between(1, 5)

    @task
    def login(self):
        self.client.post('/login', json={'user': 'alice', 'pwd': 'pass'})

    @task
    def browse_products(self):
        self.client.get('/products')
```

**Tool fit**: Python-heavy teams, quick iteration, simple to moderate scenarios.

### Gatling (Scala)

Gatling is a Scala-based load testing tool with a focus on fast simulation and intuitive reports.

**Strengths**:
- Scala DSL designed for clarity and performance
- Powerful scripting; simulations are deterministic and reproducible
- Built-in async HTTP client; handles high concurrency efficiently
- Reports are HTML; easy to share and archive
- Good protocol support (HTTP, WebSocket, SSE, JMS)

**Example**:
```scala
class BasicSimulation extends Simulation {
  val httpProtocol = http.baseUrl("https://api.example.com")

  val scenario = scenario("Basic Load")
    .exec(http("Login").post("/login").body(StringBody("""{"user":"alice"}""")))
    .pause(1)
    .repeat(5) {
      exec(http("Get Products").get("/products"))
    }

  setUp(
    scenario.inject(
      rampUsers(100).during(60),
      constantUsersPerSec(50).during(120)
    )
  ).protocols(httpProtocol)
}
```

**Tool fit**: High-throughput scenarios, complex user journeys, performance-critical applications.

## Scripting Patterns

### 1. Realistic User Flow

Tests should model actual user behavior, not synthetic operations. Include realistic pauses:

```javascript
// k6 pattern
export default function () {
  // 1. User lands on homepage
  http.get('https://example.com/');
  sleep(3);  // Browse for 3 seconds
  
  // 2. Search for product
  const searchRes = http.post('https://example.com/search', {
    q: 'laptop'
  });
  sleep(2);
  
  // 3. Click product
  http.get('https://example.com/product/123');
  sleep(1);
  
  // 4. Add to cart
  http.post('https://example.com/cart', {
    product_id: 123,
    qty: 1
  });
}
```

Think-time between actions makes load patterns more realistic and stress-tests queuing and session management.

### 2. Data Parameterization

Load tests should use varied data, not repeat the same request. Use data sources:

```javascript
// k6 with CSV data
import { open } from 'k6/http';
const userList = open('./users.csv').split('\n');

export default function (data) {
  const user = userList[Math.floor(Math.random() * userList.length)];
  http.post('https://api.example.com/login', {
    username: user.split(',')[0],
    password: user.split(',')[1],
  });
}
```

Prevents cache-layer artifacts and ensures distributed backend load.

### 3. Correlation Variables

Website responses often contain session tokens or IDs needed in subsequent requests. Extract and reuse:

```javascript
// k6 pattern: extract token from response
export default function () {
  const loginRes = http.post('https://api.example.com/login', {...});
  const token = loginRes.json('access_token');
  
  http.get('https://api.example.com/profile', {
    headers: { 'Authorization': `Bearer ${token}` },
  });
}
```

### 4. Conditional Branching

Simulate different user paths based on response codes or randomness:

```javascript
// Different user personas
export default function () {
  const scenario = Math.random();
  
  if (scenario < 0.8) {
    // 80% browse and log out
    browseCatalog();
    logout();
  } else {
    // 20% add to cart and checkout
    browseCatalog();
    addToCart();
    checkout();
  }
}
```

### 5. Assertions and Exit Criteria

Define pass/fail criteria within the test:

```javascript
// k6 thresholds
export const options = {
  thresholds: {
    http_req_duration: ['p(95)<200', 'p(99)<500'],  // 95th % < 200ms, 99th < 500ms
    http_requests_failed: ['rate<0.1'],              // < 10% failure rate
    http_requests: ['count>1000'],                   // At least 1000 requests
  },
};

// In test, explicit checks
check(res, {
  'status is 200': (r) => r.status === 200,
  'response time < 500ms': (r) => r.timings.duration < 500,
});
```

Failing thresholds cause the test to report FAIL even if no requests error.

## Metrics and Interpretation

Common metrics reported by load testing tools:

| Metric | Meaning | Target |
|--------|---------|--------|
| **Response Time (p50, p95, p99)** | Latency percentiles. p95 = 95% of requests faster than this. | p95 < SLA; p99 < 2×SLA |
| **Throughput (req/sec)** | Requests per second. Capacity benchmark. | Match or exceed production target |
| **Error Rate** | % of requests that fail (non-2xx) or timeout. | < 0.1% (SLA dependent) |
| **Connection Errors** | Failed TCP connections. Indicates overload or system failure. | 0 under nominal load; increase at breaking point |
| **Active Users / VUs** | Virtual users currently active. | Target load level |
| **GC Pauses, CPU, Memory** | System resource usage. | Monitor for degradation as load increases |

**Interpretation**:
- **Nominal load (~expected traffic)**: Response times stable, error rate near zero, resources not maxed out.
- **Stress (exceeding expected)**: response times degrade; errors rise; resource usage approaches limits.
- **Breakthrough point**: Response time spikes sharply; errors accumulate; system struggles to recover.

## Distributed Load Generation

As load grows, a single machine can't generate enough traffic. Tools support distributing load across multiple agents:

### k6 Cloud

k6 integrates with Grafana's cloud infrastructure to run tests across geographically distributed runners:

```bash
# Run locally
k6 run test.js

# Run in cloud (cross-region)
k6 cloud test.js
```

Cloud runners coordinate metrics collection; results are aggregated in real-time.

### JMeter Distributed

JMeter master-slave architecture:

1. Master defines test scenario (sent to slaves)
2. Slaves execute threads locally
3. Master collects results from all slaves
4. Results aggregated and reported

```bash
# Master coordinates, slaves execute
jmeter -n -t scenario.jmx -Rlaveclient1,slaveclient2,slaveclient3
```

Setup requires network coordination, NTP synchronization, and careful result aggregation (careful with aggregating percentiles across machines).

### Locust Distributed

Multiple Locust processes coordinate via master-worker pattern:

```bash
# Master (aggregates results)
locust -f auth.py --master

# Workers (execute load on different machines)
locust -f auth.py --worker --master-host=<master-ip>
```

Simpler than JMeter; works well for horizontally scalable test code.

### Cloud-Based Load Testing Platforms

Platforms like AWS Load Testing, Azure Load Testing, or dedicated tools (Lambda, BlazeMeter) abstract infrastructure. You define scenario; platform spins up regions, runs tests, reports results.

**Tradeoff**: Less control but simpler operations; pay-per-test pricing.

## Bottleneck Identification

Once load testing reveals performance degradation, systematically identify the bottleneck:

1. **Application layer**: CPU, memory, GC pauses (use profilers; see performance-optimization)
2. **Database**: Slow queries, connection pool exhaustion, lock contention (enable query logging)
3. **I/O layer**: Disk, network bandwidth limits (check system utilities: iostat, netstat)
4. **Cache misses**: Inefficient caching strategy (monitor hit rates)
5. **External dependencies**: Third-party API latency (measure and isolate)

Tools:
- Profilers (flamegraphs, traces) reveal CPU hotspots
- Database query logs show slow operations
- Network analysis (tcpdump) reveals bandwidth limits
- APM tools (DataDog, New Relic) correlate application, infrastructure, and external metrics

## Common Pitfalls

1. **Unrealistic think-time**: Hammering endpoints without pauses doesn't reflect user behavior; tests don't stress session management or caching.
2. **Cache warmups ignored**: First requests are often slow. Some tools support cache priming; others require baseline filtering.
3. **Misinterpreted percentiles**: 50th percentile is median; doesn't capture tail behavior. Track p95/p99 for quality SLAs.
4. **Single-region bias**: Distributed load looks different than single-machine load; network delays and resource contention vary.
5. **Non-reproducible results**: Insufficient data, noisy infrastructure, or variable external dependencies. Run tests multiple times; detect variance.

## Integration with CI/CD

Load tests should run on a schedule or after major changes:

```yaml
# GitHub Actions example
- name: Run load test
  run: k6 run test.js --out json=results.json
  
- name: Check thresholds
  run: |
    if grep -q '"failed"\s*:\s*true' results.json; then
      echo "Load test failed"
      exit 1
    fi
```

Prevents performance regressions from shipping.

## Summary

Load testing tools range from lightweight (k6, Locust) to enterprise (JMeter, Gatling). Choose based on language fit, protocol support, and team expertise. Write realistic user flows with think-time and data variation. Monitor key metrics (p95/p99 latency, error rate, throughput). Distribute load across regions as needed. Integrate into CI/CD to catch regressions early.