# Bayesian Statistics — Inference, Priors, MCMC & Decision-Making

Bayesian statistics inverts the question of classical frequentist inference. Given observed data, what can we say about the parameter that generated it? Bayes' theorem is the mathematical engine; the framework is a coherent approach to uncertainty quantification and sequential decision-making.

## Bayes' Theorem & Core Concepts

At its foundation:

$$P(\theta | D) = \frac{P(D | \theta) P(\theta)}{P(D)}$$

- **Prior** $P(\theta)$: Belief about $\theta$ before seeing data
- **Likelihood** $P(D|\theta)$: Probability of observing $D$ given $\theta$
- **Posterior** $P(\theta|D)$: Updated belief after observing data
- **Evidence** $P(D) = \int P(D|\theta) P(\theta) d\theta$: Marginal likelihood (normalization constant)

The Bayesian assigns a probability *distribution* to $\theta$, not a point estimate. This distribution captures full uncertainty; credible intervals are straightforward: find quantiles of the posterior.

### Interpretive Contrast: Frequentist vs Bayesian

**Frequentist**: Parameter $\theta$ is fixed but unknown; we construct procedures with guaranteed long-run frequency properties (confidence intervals, p-values). A 95% CI means "this procedure, over infinite repetitions, contains the true $\theta$ 95% of the time."

**Bayesian**: Parameter $\theta$ is random (has a prior); we condition on the observed data to compute the posterior. A 95% credible interval means "posterior probability that $\theta$ lies in this interval is 0.95"—applicable to the *current* inference problem, not a procedure property.

## Conjugate Priors & Analytical Posteriors

Computing the posterior requires evaluating the evidence integral, which is often intractable. **Conjugate priors** sidestep this: choose a prior such that the posterior belongs to the same family as the prior.

### Examples

- **Bernoulli likelihood** ($n$ coin flips, $k$ heads) + **Beta prior** $\text{Beta}(\alpha, \beta)$ → **Beta posterior** $\text{Beta}(\alpha + k, \beta + n - k)$
  - Hyperparameters $\alpha - 1, \beta - 1$ interpret as pseudo-observations
  - Beta(1,1) is uniform (uninformed)
  
- **Gaussian likelihood** (known variance) + **Gaussian prior** → **Gaussian posterior**
  - Posterior mean is a precision-weighted average of prior and data estimates

- **Poisson likelihood** + **Gamma prior** → **Gamma posterior**

**Misfeature**: Conjugate priors may not reflect true prior beliefs. Modern practice uses computational methods instead.

## Inference: MCMC & Variational Methods

When the posterior is intractable (no closed form), two classes of methods approximate it:

### Markov Chain Monte Carlo (MCMC)

Sample from the posterior by constructing a Markov chain whose stationary distribution is the posterior.

**Metropolis-Hastings**: 
- Propose a candidate $\theta'$ from proposal distribution $q(\theta'|\theta_t)$
- Accept with probability $\min\left(1, \frac{P(\theta'|D) q(\theta_t|\theta')}{P(\theta_t|D) q(\theta'|\theta_t)}\right)$
- Only likelihood ratios matter; evidence $P(D)$ cancels
- **Advantage**: General-purpose, no tuning of likelihood form
- **Disadvantage**: High correlation between samples; convergence diagnostics required

**Gibbs Sampling** (special case for multivariate problems):
- Condition on all but one parameter; sample from the full conditional
- Repeat for each parameter in sequence
- Less computation per step; higher acceptance rate; requires conditional forms

**Convergence assessment**: 
- **Burn-in**: Discard initial samples until chain reaches steady state
- **Diagnostics**: $\hat{R}$ (Gelman-Rubin): ratio of between-chain to within-chain variance; $\hat{R} < 1.01$ indicates convergence
- **Autocorrelation**: Effective sample size $n_\text{eff} = n / (1 + 2\sum_{k=1}^\infty \rho_k)$ where $\rho_k$ is lag-$k$ autocorrelation

**Tools**: PyMC (Python), Stan (probabilistic language), JAGS, WinBUGS

### Variational Inference

Approximate posterior $P(\theta|D)$ with a tractable distribution $Q(\theta)$, minimizing KL divergence:

$$\text{KL}(Q || P) = \mathbb{E}_{Q} \left[\log Q(\theta) - \log P(\theta, D)\right]$$

- Rewrite as: $\log P(D) = \mathbb{E}_Q[\log P(\theta,D) - \log Q(\theta)] + \text{KL}(Q||P)$
- Maximize the *evidence lower bound* (ELBO) $\mathcal{L} = \mathbb{E}_Q[\log P(\theta,D) - \log Q(\theta)]$
- ELBO gives lower bound on marginal likelihood; minimizes KL when KL=0

**Mean-field approximation**: Assume $Q(\theta) = \prod_i Q_i(\theta_i)$—factors decouple.
- Each factor updates as: $Q_j(\theta_j) \propto \exp\left(\mathbb{E}_{Q_{-j}}[\log P(\theta, D)]\right)$
- Iteratively recompute factors until convergence
- **Fast** (no sampling), **deterministic**, but underestimates posterior variance

**Reparameterization trick** (for neural networks): Rewrite $\theta \sim Q_\phi(\theta)$ as $\theta = g_\phi(\epsilon)$ with $\epsilon \sim N(0,I)$; backprop directly through samples.

## Bayesian A/B Testing & Decision Theory

Frequentist A/B tests commit to a sample size and significance level upfront; Bayesian approaches are flexible and interpretable.

### Bayesian A/B Test

- **Prior**: Assume conversion rates $p_A, p_B$ have independent Beta priors
- **Data**: Observe $k_A$ conversions from $n_A$ trials (variant A), similarly for B
- **Posterior**: Beta posteriors for each (conjugate)
- **Decision**: Compute $P(p_B > p_A | D)$ directly from posterior samples
  - If $P(p_B > p_A) > 0.95$, declare B winner
  - Can stop early when this threshold is reached (optional stopping is valid in Bayesian framework)

**Advantages**:
- No pre-specified sample size; stop when confident
- Direct probability of superiority (not p-value)
- Can use prior information (historical experiments, domain knowledge)

**Practical consideration**: Loss functions. If launching the worse variant has cost $C_\text{wrong}$ and delay has cost $C_\text{delay}$:
$$L = C_\text{wrong} \cdot P(p_B < p_A | D) + C_\text{delay}$$

Optimal decision: launch when expected loss is minimized.

## Credible Intervals vs Confidence Intervals

- **Credible interval**: Given data, $(a, b)$ has posterior probability $\alpha = 0.95$ that $\theta \in (a,b)$. Straightforward interpretation; can be asymmetric.
- **Confidence interval**: Procedure-based; over many repetitions, covers true $\theta$ 95% of the time. Cannot say "true $\theta$ is in this interval with 95% probability."

Frequentist CIs guarantee coverage; Bayesian credible intervals do not guarantee coverage, but are calibrated if the prior is well-chosen.

## Hierarchical Models & Empirical Bayes

**Hierarchical model**: Parameters have their own distributions (hyperpriors).

Example (schools data):

$$\text{effect}_i \sim N(\mu, \tau^2), \quad \mu \sim N(0, A), \quad \tau \sim \text{HalfNormal}(\sigma)$$

Allows borrowing of strength across groups: schools with few observations shrink toward the global mean $\mu$.

**Empirical Bayes**: Estimate hyperpriors from the marginal (data from all groups), rather than specifying them. Faster than full Bayesian hierarchical models but loses some uncertainty quantification.

## Model Comparison & Bayes Factors

Bayesian model selection via Bayes factor:

$$\text{BF}_{12} = \frac{P(D|M_1)}{P(D|M_2)}$$

where $P(D|M_i) = \int P(D|\theta_i, M_i) P(\theta_i|M_i) d\theta_i$—the marginal likelihood (evidence) under each model.

- $\text{BF}_{12} > 10$: Strong evidence for $M_1$
- Automatically penalizes overfitting (Occam's razor): complex models must improve likelihood substantially to overcome prior volume

Calculation: Often intractable; approximate via bridge sampling, nested sampling, or Laplace approximation.

## Pitfalls & Practical Considerations

**Prior sensitivity**: Posterior can be dominated by subjective prior, especially with small data. Explore sensitivity by varying priors and checking if conclusions change.

**Convergence**: MCMC chains can look converged locally while missing modes. Use multiple chains, long burn-in, and diagnostic plots.

**Computational cost**: High-dimensional MCMC is slow (curse of dimensionality). Variational inference is faster but can misrepresent posterior geometry.

**Model mis-specification**: Even if true model is mis-specified, posterior values are those of the approximate model; true properties are not guaranteed unless model includes truth in its support.