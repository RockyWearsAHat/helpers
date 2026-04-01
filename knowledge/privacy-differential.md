# Differential Privacy — Formal Framework and Mechanisms

## Overview

Differential privacy is a mathematical framework that quantifies privacy loss and ensures formal privacy guarantees resistant to any attack, regardless of the attacker's side information. Unlike k-anonymity (which assumes non-linkability) or encryption (which protects data at rest), differential privacy protects query results from inference attacks.

## Formal Definition

**Definition:** A randomized algorithm $M$ satisfies $(\epsilon, \delta)$-differential privacy if for all pairs of adjacent datasets $D$ and $D'$ (differing in one record), and for all subsets $S$ of the output space:

$$\Pr[M(D) \in S] \leq e^\epsilon \cdot \Pr[M(D') \in S] + \delta$$

**Intuition:** The probability of observing a particular output is similar whether a specific individual's data is included or excluded. An attacker cannot reliably infer whether your data is in the dataset.

**Parameters:**
- **$\epsilon$ (epsilon):** Privacy loss budget. Smaller $\epsilon$ = stronger privacy. $\epsilon = 0.1$ is stronger privacy than $\epsilon = 1$. There's no universal threshold for "good" privacy; context matters.
- **$\delta$ (delta):** Failure probability. In $(\epsilon, 0)$-differential privacy, the guarantee holds with probability 1. In $(\epsilon, \delta)$, it can fail with probability $\delta$. Typically $\delta \leq 1/n$ where $n$ is dataset size.

When $\delta = 0$, we have **pure differential privacy**. When $\delta > 0$, it's **approximate differential privacy** and is slightly weaker (but often more practical).

## Noise Mechanisms

Privacy is achieved by adding calibrated random noise to query results. The noise scale depends on the query's **sensitivity** — the maximum change a single individual's data can cause on the output.

### Sensitivity Analysis

For a function $f$ on datasets, the sensitivity is:

$$\Delta f = \max_{D, D'} |f(D) - f(D')|$$

where the maximum is over all pairs of adjacent datasets differing in one record.

Example: For a count query ("how many database records have condition X?"), adding or removing one record changes the count by at most 1, so $\Delta f = 1$.

For summing a column of values between 0 and 100, removing one record changes the sum by at most 100, so $\Delta f = 100$.

High sensitivity means large noise is required for privacy; queries that are naturally insensitive to individual data points are easier to privatize.

### Laplace Mechanism (Pure Differential Privacy)

The Laplace mechanism adds noise drawn from a Laplace distribution:

$$M(D) = f(D) + \text{Lap}(0, b)$$

where $b = \frac{\Delta f}{\epsilon}$.

The Laplace distribution has probability density $p(x) = \frac{1}{2b} e^{-|x|/b}$, producing a sharp peak at 0 and exponential tails. Samples near 0 dominate.

**Intuition:** Larger $\epsilon$ means smaller $b$ and less noise (lower privacy). Smaller $\epsilon$ means larger $b$ and more noise (higher privacy).

The Laplace mechanism achieves pure $(\epsilon, 0)$-differential privacy.

### Gaussian Mechanism (Approximate Differential Privacy)

The Gaussian mechanism adds noise from a normal distribution:

$$M(D) = f(D) + N(0, \sigma^2)$$

where $\sigma = \frac{\sqrt{2 \ln(1.25/\delta)} \cdot \Delta f}{\epsilon}$.

The Gaussian mechanism achieves $(\epsilon, \delta)$-differential privacy for any $\delta > 0$. It requires $\delta > 0$ (pure differential privacy is impossible with Gaussian noise for bounded sensitivity queries).

Less noise for large $\delta$; higher utility but weaker privacy guarantee. Practical sweet spot: $\delta \approx 1/n$.

### Exponential Mechanism

Not all queries output numbers. The exponential mechanism handles arbitrary output spaces (e.g., selecting the best classifier, ranking items).

The mechanism selects an outcome proportional to $\exp\left( -\frac{\epsilon u(D, r)}{2 \Delta u} \right)$ where $u(D, r)$ is a utility function and $r$ is an outcome candidate. High-utility outcomes are more likely; privacy is preserved by exponential dampening.

## Composition and Privacy Budgets

A key property: if you run multiple differentially private analyses, privacy loss composes.

### Basic Composition

If you run $k$ independent mechanisms each with $\epsilon_i$-differential privacy, the combined result has $(\sum_i \epsilon_i)$-differential privacy.

Example: Running 10 analyses each with $\epsilon = 0.1$ results in $\epsilon = 1$ total. Privacy degrades linearly with the number of queries.

This motivates **privacy budgets**: an organization allocates a total privacy budget (e.g., $\epsilon = 1$) across all queries. As queries are processed, the remaining budget shrinks. Once exhausted, no more queries can be answered with the same privacy guarantee.

### Advanced Composition

Advanced composition theorems (Dwork et al.) show that privacy loss composes sublinearly under certain conditions, allowing more queries before budget exhaustion. The exact bound depends on $\delta$ and the specific theorem.

Practical significance: with advanced composition, $k$ queries can be answered while keeping total $\epsilon$ manageable (e.g., $\epsilon = O(\sqrt{k})$ instead of $\epsilon = O(k)$).

## Local vs. Global Differential Privacy

**Global DP:** Data is centralized; a trusted curator adds noise. Assumes the curator is trusted and secure. Least noisy because the curator sees all data.

**Local DP:** Each individual adds noise to their own data before sending it to the curator. The curator never sees raw data; it only sees noisy outputs. No trust in the curator required.

Trade-off: Local DP is more resistant to breaches but requires more noise to achieve the same privacy guarantee.

Example: Apple's RAPPOR system uses local DP to collect statistics on Safari crash rates without Apple seeing individual crash reports.

## Practical Implementations

### Google's DP Systems

Google uses differential privacy internally in analytics and federated learning. Their `dp_accounting` library tracks privacy budget consumption; engineers specify $\epsilon$ and $\delta$ targets and validate that implementations meet them.

### Apple RAPPOR

RAPPOR (Randomized Aggregatable Privacy-Preserving Ordinal Regression) uses local differential privacy to collect crash statistics from millions of devices. Each device adds noise to crash reports before uploading, and Apple aggregates them.

### Federated Learning + DP

Federated learning trains ML models on decentralized devices without centralizing data. Combined with differential privacy:
- Local training on each device
- Add DP noise to model gradients before aggregation
- Aggregate noisy gradients at the server

Multiple rounds of federated learning with DP noise provides formal privacy guarantees to individual data sources while enabling centralized model training.

## Trade-offs: Privacy vs. Utility

Lower $\epsilon$ (higher privacy) requires more noise, reducing output accuracy. The privacy-utility curve is non-linear: tiny reductions in privacy budget yield large accuracy losses.

Example: Answering "how many people have condition X?" with $\epsilon = 10$ requires ~1 noise; with $\epsilon = 0.1$, ~100 noise. A dataset of 10,000 records becomes worthless if noise approaches 10,000.

Practical tradeoff: Select $\epsilon$ and $\delta$ where privacy is acceptable and utility is sufficient for the use case. Often: $\epsilon \in [0.1, 1]$ and $\delta = 1/n$.

## Limitations

1. **Sensitivity estimation:** If sensitivity is underestimated, privacy is compromised. If overestimated, utility suffers. Sensitivity depends on the query and dataset domain.

2. **Non-robust to future data:** Differential privacy protects against current attacks but makes no guarantees about attacks using future side information.

3. **Composition overhead:** Running many queries on the same dataset quickly exhausts privacy budgets.

4. **Not a cure-all:** DP protects against inference attacks but doesn't prevent other privacy violations (e.g., if the query itself is sensitive).

## See Also

- [privacy-engineering.md](privacy-engineering.md) — Broader privacy engineering principles and techniques
- [math-probability-statistics.md](math-probability-statistics.md) — Probability and statistics foundations
- [formal-verification.md](formal-verification.md) — Formal methods for verifying privacy properties
- [data-engineering-quality.md](data-engineering-quality.md) — Data governance context