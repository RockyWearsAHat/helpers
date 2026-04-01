# Queueing Theory — M/M/1, M/M/c, Little's Law & Capacity Planning

Queueing theory models systems where customers (requests, jobs, packets) arrive at a service facility, wait if all servers are busy, and depart after service. It provides mathematical tools for capacity planning, latency prediction, and system optimization. Applications span data centers, telecommunications, retail operations, and databases.

## Notation and Basic Concepts

**Kendall notation**: A/S/c/N/D describes a queue where:
- **A**: Arrival process (M = Markovian/Poisson; D = Deterministic; G = General)
- **S**: Service time distribution (M = Exponential; D = Deterministic; G = General)
- **c**: Number of servers
- **N**: Capacity (queue size limit; omitted if infinite)
- **D**: Discipline (FIFO by default; LIFO, priority)

**Arrival rate** $\lambda$: Mean arrivals per unit time.
**Service rate** $\mu$: Mean completions per server per unit time.
**Utilization** $\rho = \lambda / (c \mu)$: Fraction of server capacity in use. For stability, $\rho < 1$ (otherwise queue grows infinitely).

**Performance metrics**:
- $L$: Expected number of customers in system (in queue + in service).
- $L_q$: Expected number in queue (waiting, not being served).
- $W$: Expected time in system (wait + service).
- $W_q$: Expected time in queue.

## M/M/1 Queue (Single Server, Exponential Arrivals & Service)

Simplest tractable queue: Poisson arrivals (rate $\lambda$), exponential service times (rate $\mu$), one server.

### Steady-State Probabilities

Probability of exactly $n$ customers in system:

$$P_n = (1 - \rho) \rho^n, \quad n = 0, 1, 2, \ldots$$

where $\rho = \lambda / \mu < 1$ for stability.

$P_0 = 1 - \rho$ is the probability the server is idle.

### Key Performance Metrics

$$L = \frac{\rho}{1 - \rho}, \quad L_q = \frac{\rho^2}{1 - \rho}$$

$$W = \frac{1}{\mu(1 - \rho)}, \quad W_q = \frac{\rho}{\mu(1 - \rho)}$$

**Intuition**: As $\rho \to 1$ (queue near capacity), all metrics diverge to infinity — a discontinuous phase transition. At $\rho = 0.9$, $L = 9, W = 10/\mu$ times longer than at $\rho = 0.1$.

### Example

Call center receives $\lambda = 100$ calls/hour. Agents handle one call in average 3 min ($\mu = 20$ calls/hour per agent). With one agent, $\rho = 100/20 = 5 > 1$: queue explodes (unstable). With $c$ agents, stability requires $c > 100/20 = 5$; need at least 6 agents.

## M/M/c Queue (Multiple Servers)

Multi-server queue: $c$ identical servers, Poisson arrivals, exponential service.

### Steady-State Probabilities

For $n < c$ (servers not fully utilized):

$$P_n = \frac{(\lambda/\mu)^n}{n!} P_0$$

For $n \geq c$ (all servers busy, queue forms):

$$P_n = \frac{(\lambda/\mu)^n}{c! \, c^{n-c}} P_0$$

where $P_0$ is determined by normalization: $\sum_{n=0}^{\infty} P_n = 1$.

### Erlang C Formula

Probability an arriving customer must wait (all servers busy):

$$C(c, \rho) = \frac{\left(\frac{c\rho}{1}\right)^c / c!}{P_0}$$

where $\rho = \lambda / (c\mu)$ and $P_0$ normalizes probabilities.

Erlang C is fundamental in telecommunications: used to dimension telephone switches, determine staffing levels in call centers, allocate resources in cellular networks.

### Key Metrics

Expected number in queue:

$$L_q = \frac{\left(\frac{\lambda}{\mu}\right)^c}{c!} \cdot \frac{c \mu}{c\mu - \lambda} \cdot P_0$$

Expected wait in queue:

$$W_q = \frac{L_q}{\lambda}$$

Expected time in system: $W = W_q + 1/\mu$.

**Dimensioning example**: To achieve $W_q \leq 2$ minutes at $\lambda = 1000$ requests/min, $\mu = 100$ req/min per server:

$\rho = 1000/(c \cdot 100) = 10/c$.

For $c = 12$: $\rho = 0.833$. Solve Erlang C and related formulas to find exact $W_q$. If $W_q$ is too high, increase $c$ until requirement met.

## M/G/1 Queue (General Service Distribution)

Service times are arbitrary (not necessarily exponential). Arrivals remain Poisson; one server.

### Pollaczek-Khinchin Formula

Expected time in system:

$$W = \frac{1}{\mu} + \frac{\lambda(C_s^2 + 1)}{2(1 - \rho)} \cdot \frac{1}{\mu}$$

where $C_s^2 = \text{Var}(S) / E[S]^2$ is the squared coefficient of variation of service time.

**Key insight**: If service is deterministic ($C_s = 0$), $W$ is lower; if highly variable ($C_s > 1$), $W$ increases. Variability in service times increases congestion.

### Special Cases

- **M/D/1** (deterministic service): $C_s = 0$, reduces formula.
- **M/M/1** (exponential): $C_s = 1$, recovers known M/M/1 formula.
- **M/H/1** (hyperexponential, e.g., mixture of fast + slow jobs): $C_s > 1$, higher congestion.

## Little's Law: The Fundamental Relationship

**Little's Law**: In any queue at steady state,

$$L = \lambda W$$

Conversely, $L_q = \lambda W_q$.

**Derivation intuition**: If flow rate is $\lambda$ (arrivals per unit time) and average time in system is $W$, then average number in system is $\lambda W$ (by conservation of flow).

**Importance**: Model-independent. **True for any queueing discipline, service distribution, or arrival process** as long as the system is stationary and has finite values. Used to validate simulations, cross-check calculations, and reason about systems without detailed models.

**Example**: Arrival rate 10/min, average response time 6 min $\Rightarrow$ average queue size is $L = 10 \times 6 = 60$ customers. If average service time is 4 min, then approximately 40 wait and 20 are being served (roughly 5 servers).

## Load Balancing and Queueing

Queueing informs load balancing strategies:

- **Round-robin**: Spread load equally across servers. Simple, but ignores server speed differences and request heterogeneity.
- **Least connections**: Route to server with fewest active connections. Better than round-robin if request durations vary, but needs state tracking.
- **Weighted load balancing**: Allocate traffic proportional to server capacity. Reduces variance in response times across servers.
- **Join shortest queue (JSQ)**: Each arriving request chooses the queue with fewest customers. Better performance than random selection, but requires global state visibility (difficult in distributed systems).

Queueing theory shows how load imbalance increases latency. In multi-server systems, M/M/c analysis predicts when adding servers yields diminishing returns (as $\rho \to 1/c$, benefits flatten).

## Latency Modeling and SLOs

Queueing theory connects arrival rate, service capacity, and latency:

- **Per-server capacity**: Determined by $\mu$ (service rate). Database queries, cache lookups have typical $\mu$ values (e.g., 1000 req/sec per core).
- **Target P99 latency**: Use distribution of $W$ to estimate percentiles. For M/M/1, $P(W > w) = \rho e^{-\mu(1-\rho) w}$. Higher $\rho$ makes tail latencies worse (exponential increase).
- **Provisioning under peak load**: If peak $\lambda_{\text{peak}} / \mu_{\text{total}} = 0.8$, system can absorb spikes. If $\rho \geq 0.9$, small increases in $\lambda$ cause disproportionate latency jumps.

**Rule of thumb**: For predictable latency and responsiveness, keep $\rho < 0.7$. For cost-sensitive systems that tolerate occasional high latency, $\rho \approx 0.8$ acceptable.

## Heavy Tail Distributions and Queueing

Real service times often have heavy tails (e.g., web request sizes, file transfers). Heavy tails mean $P(\text{service} > x)$ decays slowly.

**M/G/1 with heavy tails**: Pollaczek-Khinchin formula predicts high $C_s$ (variance huge), so $E[W]$ increases dramatically. A few very long service times monopolize servers, delaying many others.

**Consequences**:
- Mean latency may be acceptable, but P99/P999 are extreme.
- Single slow request can block a queue (head-of-line blocking).
- Priority queues help: short jobs get higher priority, reducing latency for most.

**Mitigation**:
- Request size clipping (cap service time; queue excess processing).
- Preemption (interrupt long-running jobs, resume later).
- Isolation (separate fast-path queues for short jobs; background queue for long jobs).

## Queueing in Distributed Systems

**Fan-out queueing**: Request forks into sub-requests across multiple servers (e.g., database query → index + data partition lookups). Total latency is determined by slowest sub-request (tail latency amplification).

$E[W_{\text{max}}] > E[W]$ for a single queue; the gap grows with the number of sub-requests and tail heaviness of individual queues.

**Mitigation**: Redundancy (send to multiple servers, use first response) traded against overhead and tail propagation (more servers = worse tail latency).

## Practical Guidance

| Scenario                     | Model       | Key Insight                                      |
| ---------------------------- | ----------- | ------------------------------------------------ |
| Call center staffing         | M/M/c       | Use Erlang C to set staff levels for SLA        |
| Single database server       | M/M/1       | If $\rho > 0.8$, plan scale immediately          |
| Web service variable latency | M/G/1       | High variance drives congestion; cap/prioritize |
| Backend scaling              | M/M/c       | Add servers; diminishing returns near capacity   |
| Task system (batch jobs)     | M/D/1       | Deterministic service yields lower latency       |
| Real-time systems            | M/M/c+P     | Priorities reduce mean latency for critical jobs |

See also: architecture-microservices.md, system-design-distributed.md, database-performance.md, monitoring-opentelemetry-deep.md