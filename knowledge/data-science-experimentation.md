# Data Science Experimentation — A/B Testing, Statistical Rigor, and Experimentation Platforms

## Hypothesis Testing and Significance

A/B testing is controlled experimentation where two variants (A: control, B: treatment) are deployed to random samples, and outcomes are compared statistically. The goal: distinguish genuine effects from random noise.

**Hypothesis framework**:

- **Null hypothesis** (H₀): Treatment has no effect; observed differences arise by chance
- **Alternative hypothesis** (H₁): Treatment has an effect
- **Type I error** (α): Reject H₀ when it's true (false positive). Typical α = 0.05
- **Type II error** (β): Fail to reject H₀ when H₁ is true (false negative). Statistical power = 1 - β

**Test selection**:

| Scenario | Test | Assumption |
|----------|------|-----------|
| Continuous metric (revenue, time-on-page) | **t-test** | Normal distribution, equal variance |
| Binary outcome (click, conversion) | **Chi-square** or logistic regression | Counts ≥ 5 in each cell |
| Rates (clicks per user, CTR) | **Poisson test** | Assumes Poisson distribution |
| Multi-variant (3+ arms) | **ANOVA** → post-hoc tests | Normal, homogeneous variance |

The p-value is **not** the probability the null hypothesis is true; it's the probability of observing this data (or more extreme) *if* H₀ were true. Low p-values (< 0.05) provide weak evidence against H₀, not proof of effect size.

## Sample Size and Power

**Sample size formula** (for binary outcomes, simplified):

$$n = \frac{2(Z_{\alpha/2} + Z_{\beta})^2 \cdot p(1-p)}{\delta^2}$$

Where:
- $Z_{\alpha/2}$ = critical value for desired Type I error (1.96 for α=0.05)
- $Z_\beta$ = critical value for desired power (0.84 for 80% power)
- $p$ = baseline conversion rate
- $\delta$ = minimum detectable effect size (relative or absolute)

**Practical example**: To detect a 10% relative lift on a 5% baseline (absolute: 5% → 5.5%) with 80% power and α=0.05, need ~15,600 users per arm.

This calculation drives **runtime** (time until sample size is reached). Underpowered tests frequently fail to detect real effects; overpowered tests waste resources on trivial effects.

## Multiple Comparisons and Correction

When testing many hypotheses simultaneously, false positives multiply. If each test has α=0.05 and there are 20 independent tests, the probability of ≥1 false positive exceeds 64%.

**Bonferroni correction**: Divide α by number of comparisons. For m tests, use α/m per test. Conservative; reduces power.

**False Discovery Rate (FDR)** (Benjamini-Hochberg): Control the proportion of false positives among all significant findings. Less conservative than Bonferroni; preferred when many tests are expected to be true.

**Practice**:
- Pre-register primary metrics before running the experiment
- Treat secondary/exploratory metrics as hypotheses for follow-up experiments
- Adjust always; omitting corrections is misleading

## Advanced Methods: CUPED and Variance Reduction

**CUPED** (Controlled User Experiment with Pre-Experiment Data): Use pre-treatment user behavior as a covariate to reduce variance.

Standard approach:
$$\text{Adjusted metric} = \text{Treatment metric} - \theta \cdot (\text{Pre-treatment metric} - \text{Mean pre-treatment})$$

Where θ is fit to minimize variance. Can reduce sample size requirements by 20-60% when pre-experiment and treatment metrics are correlated.

**Stratification**: Pre-assign users to strata (e.g., by cohort, geography) and randomize within strata. Improves precision when strata correlate with the outcome.

**Jackknife/bootstrap**: Estimate confidence intervals without distributional assumptions.

## Switchback and Network Experiments

**Switchback experiments** (time-based): Alternate between control and treatment on a schedule (e.g., A on Monday, B on Tuesday). Used when user-level randomization is infeasible (e.g., taxi supply, restaurant delivery).

Challenges:
- **Carryover effects**: User behavior from period A influences period B
- **Temporal confounds**: Day-of-week effects, seasonal trends, competitor actions

Mitigations: Longer observation windows, washout periods between switches, model-based adjustment.

**Network experiments**: When users interact (social graphs, marketplaces), unit of randomization matters. Randomizing by user risks **interference**: treatment on Alice affects Bob's outcome. Solutions:
- Cluster randomization (randomize by community)
- Ego-network design (randomize individuals, measure spillover by network distance)
- Graph-based matching (pair similar network neighborhoods)

## Bayesian A/B Testing

Traditional (frequentist) tests answer: "Given H₀ is true, what's the probability of this data?" Bayesian tests answer: "Given this data, what's my posterior belief in H₁?"

**Benefits**:
- Incorporate prior knowledge (historical effect sizes, domain expertise)
- Continuous monitoring: Stop early if posterior probability of superiority reaches a threshold (e.g., 95%)
- Report posterior distributions (probability treatment is better **by at least X%**)
- Natural interpretation: P(treatment better | data)

**Drawbacks**:
- Prior choice is subjective and influences results
- Posterior depends on stopping rule (unlike frequentist α)
- Requires Bayesian software (harder than t-tests)

Practical: Beta-Binomial model for conversion rates, Normal-Normal for continuous metrics with known variance.

## Guardrail Metrics and Tradeoffs

Not all metrics should move in the same direction. **Guardrail metrics** are secondary KPIs that prevent harmful treatments:

Example: Optimizing for clicks may increase spam. Define guardrails:
- **CTR** (primary, maximize)
- **Spam reports per user** (guardrail, keep ≥ baseline)
- **Session duration** (guardrail, don't decrease by >5%)

If any guardrail breaks, treat defeats the experiment even if primary metric improves.

## Feature Flagging Infrastructure

Experiments require rapid deployment and rollback. **Feature flags** enable this:

- **Kill switch**: Flag setting toggles feature on/off globally
- **Ramp-up**: Percentage-based rollout (1% → 10% → 100%) catches bugs early
- **User targeting**: Flag on/off by user segment, region, device type
- **Experiment assignment**: Flag linked to experiment randomization

Platforms (LaunchDarkly, Eppo, Statsig): Store flag state centrally, sync to SDKs, log activations for analysis.

## Experimentation Platforms and Observability

Modern experimentation systems (2026) integrate:

1. **Experiment design UI**: Define hypothesis, metrics, sample size, duration
2. **Randomization**: Cryptographic user hashing ensures non-repeatable assignment
3. **Metrics computation**: Real-time aggregation (Kafka → real-time database) for live dashboards
4. **Statistical inference**: Auto-compute p-values, confidence intervals, optional Bayesian posterior
5. **Alerting**: Guardrail violations, unexpected downtimes, invalid experiments
6. **Diagnostics**: Check randomization balance, metric distributions, cohort sizes

Examples: Optimizely, Statsig, Eppo, Netflix (internal), Airbnb (XP platform).

Log schema typically captures:
- User ID, timestamp, experiment arm, metric value(s)
- Event properties (session ID, device, country)
- Experiment metadata (name, variant, hypothesis)

## Common Pitfalls and Anti-Patterns

**Peeking**: Check results mid-experiment, stop based on partial data. Violates Type I error guarantees.

**P-hacking**: Tweak metric definitions, segments, or duration post-hoc to achieve p < 0.05. Inflates false positives.

**Winner's curse**: Overestimate true effect size if stopping early after large observed lift.

**Simpson's paradox**: Aggregate direction reverses when stratified by a confounder. Always report segmented results.

**Measurement errors**: Click double-counting, timestamp misalignment, off-by-one bugs. Test logging pipelines independently.

## See Also
- [statistics-hypothesis-testing.md](statistics-hypothesis-testing.md) — Deeper statistical foundations
- [metrics-design-observability.md](metrics-design-observability.md) — Metric definition and measurement
- [data-science-causal-inference.md](data-science-causal-inference.md) — Observational alternatives when randomization isn't feasible