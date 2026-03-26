# Sampling Methods — Probability Sampling, Bootstrap, MCMC & Experimental Design

Sampling techniques are the foundation of statistical inference and experimental design. They enable estimation from populations that are too large, expensive, or destructive to measure in full. Different methods balance cost, representativeness, and variance reduction.

## Probability Sampling Designs

Probability sampling ensures every unit has a known, non-zero probability of selection. This allows unbiased estimation and quantifiable uncertainty.

### Simple Random Sampling (SRS)

Every sample of size $n$ from a population of size $N$ has equal probability. Achieved by drawing indices uniformly without replacement.

- **Unbiased**: Sample mean $\bar{x}$ estimates population mean $\mu$ with $E[\bar{x}] = \mu$.
- **Variance**: $\text{Var}(\bar{x}) = \frac{\sigma^2}{n} \left(1 - \frac{n}{N}\right)$ where $\sigma^2$ is population variance. The factor $(1 - n/N)$ is the finite population correction.
- **Efficiency**: When population is very large $(N \gg n)$, the correction is negligible.
- **When it fails**: If population has strata with very different means, SRS spreads samples inefficiently across strata.

### Stratified Sampling

Partition population into $L$ strata (subgroups) with similar characteristics. Allocate $n_l$ samples to stratum $l$. Sample uniformly within each stratum.

**Stratification by post-stratification variable** (e.g., income level, geographic region, age bracket):
- Reduces variance compared to SRS when strata means differ widely.
- Ensures representation from rare subgroups.
- Estimate: $\hat{\mu}_{\text{st}} = \sum_{l=1}^{L} W_l \bar{x}_l$ (weighted average of stratum means; $W_l$ is proportion of population in stratum $l$).
- Variance: Lower than SRS if strata are well-chosen (within-stratum variance is small).

**Allocation strategies**:
- **Proportional**: $n_l = n \cdot W_l$ (allocate proportional to stratum size). Simple, often good.
- **Optimal (Neyman)**: $n_l = n \cdot \frac{W_l \sigma_l}{\sum_k W_k \sigma_k}$ (allocate more to larger, more variable strata). Minimizes variance for fixed $n$, but requires known stratum SDs.

### Cluster Sampling

Population divided into clusters (e.g., geographic areas, schools). Randomly select clusters; sample all (or sub-sample) units within chosen clusters.

- **Advantage**: Cheaper when units within clusters are geographically close (reduced travel cost).
- **Disadvantage**: Increases variance compared to SRS if within-cluster similarity is high (units within clusters are homogeneous). Effective sample size is reduced.
- **Use case**: Large, geographically dispersed populations where contact cost dominates.

Multi-stage cluster sampling: Sample clusters, then sub-sample from each. Often combined with stratification.

### Systematic Sampling

Select every $k$-th unit from an ordered list, starting from a random position in $[1, k]$.

- **Practical**: Avoids need for random number generation; simple to implement.
- **Risk**: If list has hidden periodicity (e.g., wage data sorted by gender, which has periodic structure), systematic sampling can introduce bias.
- **Equivalent to SRS** under most practical conditions if list is random or has minimal structure.

## Reservoir Sampling (Vitter's Algorithm R)

Efficiently select $k$ items uniformly from a stream of unknown size $n$ (seen only once, sequentially).

### Algorithm

```
For i = 1 to k: reservoir[i] = stream[i]
For i = k+1 to n:
    j = random(1, i)
    if j <= k: reservoir[j] = stream[i]
```

Each item $i$ has probability $k/i$ of entering reservoir (when $i \geq k$), declining as stream lengthens. After processing all $n$ items, each has probability exactly $k/n$ of being in reservoir.

**Complexity**: O(n) time, O(k) space — optimal for streaming.

**Use case**: Sampling from logs, infinite streams, or data too large to fit in memory. Foundation for MapReduce sampling, log analysis, approximation algorithms.

## Importance Sampling

Sample from a convenient distribution $q(\mathbf{x})$, then weight samples to estimate quantities under target distribution $p(\mathbf{x})$.

To estimate $E_p[f(\mathbf{x})]$:

$$E_p[f(\mathbf{x})] = \int f(\mathbf{x}) p(\mathbf{x}) d\mathbf{x} = \int f(\mathbf{x}) \frac{p(\mathbf{x})}{q(\mathbf{x})} q(\mathbf{x}) d\mathbf{x} \approx \frac{1}{n} \sum_{i=1}^{n} f(\mathbf{x}_i) \frac{p(\mathbf{x}_i)}{q(\mathbf{x}_i)}$$

where $\mathbf{x}_i \sim q$.

**Weights**: $w_i = \frac{p(\mathbf{x}_i)}{q(\mathbf{x}_i)}$

**Design of $q$**: Should be similar to $p$ (low variance) but easy to sample from. Poor choice (e.g., $q$ has light tails while $p$ has heavy tails) causes extreme weights, high variance estimates, numerical instability.

**Use case**: Rare event simulation (e.g., estimating tail probabilities), inference under intractable likelihoods.

## Bootstrap Resampling

Non-parametric method for estimating sampling distributions and confidence intervals from data without assuming a parametric model.

### Procedure

Given sample $\mathbf{x} = (x_1, \ldots, x_n)$:

1. Resample $n$ items uniformly **with replacement** from $\mathbf{x}$.
2. Compute statistic $T$ on resample (e.g., mean, median, correlation).
3. Repeat steps 1-2 many times (1000-10000 replicates).
4. Empirical distribution of $T$ approximates sampling distribution.

**Confidence intervals** (95%): Take empirical 2.5th and 97.5th percentiles.

**Advantages**:
- Non-parametric: No distributional assumptions.
- General: Works for any statistic (mean, median, regression coefficient, correlation).
- Intuitive: Empirical distribution directly visualizable.

**Limitations**:
- Cannot estimate tail probabilities well (rare events not represented in original sample).
- Assumes sample is representative of population.
- Parametric bootstrap: If you adopt a model (e.g., $\hat{\mathbf{x}} \sim N(\mu, \sigma^2)$), bootstrap from fitted model instead. Faster for large samples, but incorrect if model is misspecified.

**Use case**: Confidence intervals for non-standard statistics (e.g., ratio of medians), comparing models, assessing robustness.

## Monte Carlo Markov Chain (MCMC) Sampling

Generates samples from a complex posterior distribution $p(\boldsymbol{\theta} | \mathbf{data})$ by constructing a Markov chain that (asymptotically) has the posterior as its stationary distribution.

### Metropolis-Hastings Algorithm

1. Start at $\boldsymbol{\theta}_0$.
2. Propose $\boldsymbol{\theta}' \sim q(\cdot | \boldsymbol{\theta}_t)$ (proposal distribution, often multivariate normal centered at current state).
3. Compute acceptance ratio: $\alpha = \min\left(1, \frac{p(\boldsymbol{\theta}') q(\boldsymbol{\theta}_t | \boldsymbol{\theta}')}{p(\boldsymbol{\theta}_t) q(\boldsymbol{\theta}' | \boldsymbol{\theta}_t)}\right)$
4. Accept $\boldsymbol{\theta}' \to \boldsymbol{\theta}_{t+1}$ with probability $\alpha$; otherwise $\boldsymbol{\theta}_{t+1} = \boldsymbol{\theta}_t$.
5. Repeat.

**Gibbs sampling**: Special case where $p(\boldsymbol{\theta} | \mathbf{data})$ is complex but full conditionals $p(\theta_j | \boldsymbol{\theta}_{-j}, \mathbf{data})$ are tractable. Sample each component sequentially from its conditional; acceptance rate = 1.

### Diagnostics

- **Burn-in**: Discard first 1000-5000 iterations (chain is "warming up" to stationary distribution).
- **Effective sample size (ESS)**: Autocorrelated chain samples are less informative. ESS < n indicates poor mixing. $\hat{ESS} = n / (1 + 2 \sum_{lag} \rho_{lag})$ where $\rho_{lag}$ is autocorrelation.
- **Convergence**: Run multiple chains from different starting points. Gelman-Rubin $\hat{R}$ statistic: ratio of between-chain to within-chain variance. $\hat{R} < 1.01-1.05$ indicates convergence.
- **Trace plots**: Plot samples vs. iteration. Good mixing = noise-like; poor = trends, stuck segments.

**Scalability**: MCMC becomes slow for high-dimensional problems. Extensions like Hamiltonian Monte Carlo (HMC) and variational inference address this.

## A/B Testing and Sample Sizing

### Setup

Two groups (treatment vs. control). Measure outcome (conversion rate, engagement, revenue). Determine if observed difference is statistically significant or due to chance.

### Sample Size Calculation

For comparing two proportions $p_1$ vs. $p_2$:

$$n = \frac{2(z_{\alpha/2} + z_{\beta})^2 p(1-p)}{(p_1 - p_2)^2}$$

where $p = (p_1 + p_2)/2$, $z_{\alpha/2}$ is critical value for desired significance level $\alpha$ (e.g., 0.05), $z_{\beta}$ is critical value for desired power $1-\beta$ (e.g., 0.90, so $\beta=0.10$).

**Interpretation**: Larger effect size $(p_1 - p_2)$ allows smaller samples. Higher power demands larger samples. Same calculation exists for means (using SD).

### Practical Considerations

- **Duration**: Run test long enough to collect $n$ samples. Stopping early inflates false positive rate (multiple comparisons / peeking problem).
- **Allocation**: Balanced (equal $n$ per group) is optimal. Unbalanced designs widen confidence intervals.
- **Stratification**: Allocate users to control/treatment uniformly within strata (e.g., geography, device) for lower variance.
- **Multiple testing**: If testing many hypotheses, apply Bonferroni or FDR correction to maintain overall type-I error rate.

## Practical Framework

| Method           | When to Use                                        | Strength                          | Weakness                   |
| ---------------- | -------------------------------------------------- | --------------------------------- | -------------------------- |
| SRS              | Homogeneous population, simple implementation      | Unbiased, straightforward         | Inefficient if strata exist |
| Stratified       | Multiple subgroups with different means            | Lower variance, ensures coverage  | Requires stratum info      |
| Cluster          | Geographically dispersed population                | Cost-efficient                    | Higher variance            |
| Reservoir        | Streaming / unknown population size               | Single pass, O(k) space           | Applicable only for $k$ samples |
| Importance       | Rare events, intractable distributions             | Flexible, general                 | High-variance if $q$ poor  |
| Bootstrap        | Non-parametric confidence intervals, robustness   | Model-free, intuitive             | Poor for tail estimation   |
| MCMC             | Bayesian inference, complex posteriors             | Principled probabilistic inference | Computationally expensive  |
| A/B Tests        | Comparing treatments in production                 | Direct causal evidence            | Requires careful design    |

See also: math-probability-statistics.md, ml-model-evaluation.md, algorithms-randomized.md