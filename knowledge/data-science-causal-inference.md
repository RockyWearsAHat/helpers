# Data Science Causal Inference — Methods for Observational and Experimental Studies

## Causation vs. Correlation

**Causation**: X causes Y if intervening on X changes Y. Example: Taking medication C causes recovery R.

**Correlation**: X and Y co-vary. Causation requires correlation, but correlation can arise from confounding.

**Confounding**: An unobserved variable Z influences both X and Y, creating spurious correlation without causal effect. Example: Ice cream sales → sunburns (confounder: summer heat). Causal diagram: Z → X, Z → Y.

**Causal inference** distinguishes genuine effects from confounding using data and assumptions. No method proves causality from data alone; all methods require untestable assumptions.

## Randomized Controlled Trials (RCTs)

RCTs break confounding by **random assignment**. If units randomly assigned to treatment and control, treatment (X) is independent of all confounders (Z):

$$P(Z | X=1) = P(Z | X=0)$$

**Causal effect** (ATE, Average Treatment Effect):
$$\text{ATE} = E[Y | X=1] - E[Y | X=0]$$

Under randomization, simple group mean difference estimates ATE unbiasedly. No modeling, no assumptions about functional form.

**Limitations**:
- Ethical or practical barriers (can't randomize smoking, surgery side effects)
- High cost (requires expensive randomization infrastructure)
- External validity: RCT populations often unrepresentative
- Compliance issues: Some assigned to treatment refuse; some assigned to control cross over

## Observational Studies and Confounding

When randomization is infeasible, observational data is available but confounded. **Causal graphs** (DAGs) formalize confounding structure:

```
Z (confounder) → X (treatment) → Y (outcome)
         ↓
         Y
```

A **collider** (node with two incoming arrows) requires different handling:
```
X → Z ← Y
```
Conditioning on Z creates spurious correlation between X and Y. **Selection bias** arises from filtering (e.g., "patients who recovered").

## Propensity Score Matching

**Propensity score** (PS): Probability of receiving treatment given covariates.
$$PS(X=1 | Z) = P(X=1 | Z)$$

Estimated via logistic regression. **Matching** pairs treated and control units with similar propensity scores, forming a pseudo-randomized sample.

**One-to-one nearest neighbor matching**: For each treated unit, find closest control. Discard unmatched units.

**Caliper matching**: Discard pairs with PS distance > threshold (e.g., 0.1).

**Stratification**: Bin units by PS quintiles; compute ATE within each stratum, then average.

Advantages: Computationally simple, intuitive. Disadvantages: Discards data (wasted units with no matches), high variance, requires strong overlap in PS distribution (if all treated have high PS and controls have low PS, no match possible → unsupported subspace).

## Inverse Probability Weighting (IPW)

**IPW** reweights observations to create a pseudo-population where treatment is independent of confounders.

Weight formula:
$$w_i = \frac{X_i}{PS_i} + \frac{1 - X_i}{1 - PS_i}$$

For treated unit: weight = 1/PS (downweight similar-to-control units). For control: weight = 1/(1-PS) (downweight similar-to-treated units). Result: Treated and control have identical covariate distributions in weighted sample.

ATE estimate (weighted):
$$\text{ATE} = \frac{\sum_{i: X_i=1} w_i Y_i}{\sum_{i: X_i=1} w_i} - \frac{\sum_{i: X_i=0} w_i Y_i}{\sum_{i: X_i=0} w_i}$$

Advantage: Uses all data (no discarding). Disadvantage: High variance when PS is near 0 or 1 (units get extreme weights); requires parametric PS model (if model is wrong, estimate is biased).

## Instrumental Variables (IV)

An **instrumental variable** Z satisfies:
1. **Relevance**: Z affects X
2. **Exclusion**: Z affects Y only through X (not directly)

Example: Distance to school (Z) → attendance (X) → earnings (Y). Distance is relevant (affects attendance) and valid (only affects earnings through attendance).

**Two-stage least squares (2SLS)**:
- Stage 1: Regress X on Z, get predicted values Ŷ
- Stage 2: Regress Y on predicted Ŷ

Estimates causal effect even when X is confounded, as long as Z is valid.

Limitations: Weak instruments (low relevance) inflate variance; many confounders require multiple instruments (hard to find); exclusion assumption is untestable.

## Difference-in-Differences (DiD)

Exploits **staggered policy rollout** (e.g., legislation in State A but not State B) for causal inference.

Pre-treatment and post-treatment mean comparison:
$$\text{DiD} = (Y_{A,\text{post}} - Y_{A,\text{pre}}) - (Y_{B,\text{post}} - Y_{B,\text{pre}})$$

Logic: Trends in untreated state (B) approximate counterfactual trend in treated state (A). Subtracting removes confounding time trends.

**Parallel trends assumption**: Treated and control groups would follow parallel trends absent treatment. Unfalsifiable but visually inspected (plot pre-treatment trends; if parallel, more credible).

Advantages: Simple, intuitive. Disadvantages: Requires 2 time periods minimum; parallel trends assumption often violated (e.g., treated states were already growing faster).

## Regression Discontinuity (RD)

Exploits **sharp cutoff** in treatment assignment. Example: College admission via test score; students just above cutoff admit, just below reject.

If treatment is determined by rule $X = 1[Z > c]$ where Z is a running variable:
$$\text{Causal effect} = \lim_{z \to c^+} E[Y|Z=z] - \lim_{z \to c^-} E[Y|Z=z]$$

Estimated via local linear regression around cutoff. Validity: Assumes units cannot precisely manipulate Z. Test via density check (smooth distribution of Z across cutoff → no sorting).

Advantages: Minimal assumptions. Disadvantages: Effect identified only at the cutoff (not representative of average effect); requires large sample near cutoff for precision.

## Causal Inference with DAGs and do-Calculus

**Directed Acyclic Graph (DAG)** encodes causal assumptions. Nodes = variables, arrows = direct effect.

**Backdoor criterion** (Pearl): A set of variables S **blocks** confounding of effect of X on Y if:
1. No variable in S is a descendant of X
2. Every path from X to Y that has an arrow into X is blocked by S

If satisfied, conditioning on S removes confounding; this justifies regression on S.

**Frontdoor criterion**: Handles situations where no set of confounders is sufficient. X → Z → Y where all confounding of X → Y is mediated by Z.

**do-operator** (Pearl): $P(Y | \text{do}(X=x))$ is the probability of Y if we intervene to set X=x, distinct from observational $P(Y | X=x)$.

**Adjustment formula**: Under backdoor criterion,
$$P(Y | \text{do}(X=x)) = \sum_z P(Y | X=x, Z=z) P(Z=z)$$

Practical: Graph structure determines which variables to include in regression; misspecification (including colliders, omitting confounders) biases estimates.

## Synthetic Control and Panel Methods

**Synthetic control**: Constructs a weighted combination of control units to match treated unit pre-treatment characteristics and trends. Post-treatment divergence estimates effect.

Advantages: Transparent (weights published), visual comparison of factual vs. synthetic. Disadvantages: Selecting control pool is subjective; assumes no unmeasured confounding in post-treatment period.

**Fixed effects (panel regression)**:
$$Y_{i,t} = \alpha_i + \gamma_t + \beta X_{i,t} + \epsilon_{i,t}$$

- $\alpha_i$: Unit fixed effect (removes time-invariant confounding)
- $\gamma_t$: Time fixed effect (removes time-varying confounding common to all units)
- β: Causal effect (requires variation in X within-unit over time)

Valid under assumption: any confounding is unit- and time-specific, not time-varying-unit-specific.

## Heterogeneous Treatment Effects and Bayesian Additive Regression Trees (BART)

**Heterogeneous effects**: Treatment may benefit some subgroups more than others. Average treatment effect (ATE) masks this variation.

**Causal forest** (Athey & Wager): Random forest adapted for causal forests. Estimates treatment effect at each leaf node, then averages. Flexible, non-parametric; can detect effect modification by covariates.

**BART**: Bayesian tree ensemble. Provides posterior distribution of effects; naturally handles uncertainty.

Both methods estimate $\tau_i = E[Y | X_i=1, Z_i] - E[Y | X_i=0, Z_i]$ (individual treatment effect).

Limitations: Require large samples to credibly estimate effects for many subgroups; multiple comparisons problem (many subgroups → some false positives by chance).

## Uplift Modeling

Direct application of causal inference to marketing: **Should we target this customer?**

Standard prediction learns $P(\text{conversion} | \text{features})$. Uplift models estimate **individual treatment effect**: 
$$\uplift_i = P(\text{conversion} | \text{treatment}, Z_i) - P(\text{conversion} | \text{control}, Z_i)$$

Methods:
1. **Two-model approach**: Train separate models on treatment and control cohorts; difference is uplift
2. **Transformed outcome**: Create synthetic target $Y' = Y \cdot \frac{X}{p_X} - (1-Y) \cdot \frac{1-X}{1-p_X}$; train single model on Y'
3. **Causal forests**: Directly estimate individual effects

Deployment risk: Targeting only high-uplift customers wastes persuadable-but-unmarked customers; avoid if uplifts are stable only in-sample.

## Assumptions and Sensitivity Analysis

No causal method works without assumptions:

| Method | Key Assumption | Testability |
|--------|---|---|
| RCT | Randomization performed | Empirical (check balance) |
| Propensity score | Conditional independence (no unmeasured confounding) | Untestable |
| IV | Exclusion (no direct effect of Z→Y) | Untestable |
| DiD | Parallel trends | Visual check only |
| RD | No manipulation of running variable | Density test (visual) |

**Sensitivity analysis**: How robust is estimate to violations of key assumptions? Methods exist for unmeasured confounding (e-value, bounds analysis) and overlap violations (extrapolation sensitivity).

## See Also
- [data-science-experimentation.md](data-science-experimentation.md) — Randomized experiments (strongest causal setting)
- [statistics-hypothesis-testing.md](statistics-hypothesis-testing.md) — Statistical foundations
- [machine-learning-fundamentals.md](machine-learning-fundamentals.md) — Prediction vs. causation distinction