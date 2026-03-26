# Machine Learning Theory — Learnability, Generalization & Kernel Methods

Machine learning theory provides the mathematical foundations for when and why learning algorithms succeed. It connects data properties, model complexity, and generalization performance through formal bounds.

## PAC Learning & Learnability

**Probably Approximately Correct (PAC)** learning, introduced by Valiant (1984), formalizes learnability: a hypothesis class is learnable if an algorithm can find a hypothesis that is accurate on unseen data with high probability, using a reasonable number of samples.

### Definition

A hypothesis class $\mathcal{H}$ is PAC-learnable with sample complexity $m(\epsilon, \delta)$ if there exists an algorithm such that for any $\epsilon > 0, \delta > 0$, and any distribution $\mathcal{D}$ over input space:

With $m \geq m(\epsilon, \delta)$ samples drawn i.i.d. from $\mathcal{D}$, the algorithm outputs $h \in \mathcal{H}$ satisfying:

$$P(\text{error}_\mathcal{D}(h) \leq \epsilon) \geq 1 - \delta$$

where $\text{error}_\mathcal{D}(h) = P_{x \sim \mathcal{D}}[h(x) \neq f^*(x)]$ is the true error.

**Sample complexity**: For a finite hypothesis class $|\mathcal{H}| = N$:

$$m(\epsilon, \delta) = O\left(\frac{1}{\epsilon} \log \frac{N}{\delta}\right)$$

Proof: Union bound over all hypotheses; if hypothesis has low training error, likely has low test error.

### Key Implications

- **Finite classes are PAC-learnable**: Take any hypothesis consistent with training data; by concentration, few samples suffice
- **Infinite classes may still be learnable**: Complexity measured not by cardinality but by *expressive power*
- **Distribution-free bounds**: Hold for any distribution; often loose but worst-case-robust

## VC Dimension & Infinite Hypothesis Classes

**Vapnik-Chervonenkis (VC) dimension** extends PAC learnability to infinite classes. Measures the complexity of $\mathcal{H}$ by the largest set of points it can "shatter."

### Shattered Sets

A set $\{x_1, \ldots, x_m\}$ is **shattered** by $\mathcal{H}$ if for every subset $S \subseteq \{1, \ldots, m\}$, there exists $h \in \mathcal{H}$ such that:

$$h(x_i) = 1 \text{ for } i \in S, \quad h(x_i) = 0 \text{ otherwise}$$

**VC dimension** $\text{VCdim}(\mathcal{H})$ = size of largest shatterable set.

### Examples

- **Axis-aligned rectangles** in $\mathbb{R}^d$: VCdim = $2d$ (can shatter sets of $2d$ points)
- **Linear classifiers** in $\mathbb{R}^d$: VCdim = $d+1$ (hyperplane can shatter $d+1$ points in general position, not $d+2$)
- **Decision trees**: Depth-$k$ tree has VCdim = $O(k \log N)$ where $N$ is feature count
- **Neural networks**: VCdim $\approx$ parameter count (loose but informative)

### PAC Learning with VC Dimension

If $\mathcal{H}$ has finite VC dimension $d$, it is PAC-learnable with sample complexity:

$$m(\epsilon, \delta) = O\left(\frac{d}{\epsilon} \left(\log \frac{1}{\epsilon} + \log \frac{1}{\delta}\right)\right)$$

This is distribution-free worst-case complexity. Real algorithms often achieve better rates under specific distributions.

## Bias-Variance Tradeoff (Formal)

The **bias-variance decomposition** decomposes expected prediction error into three terms:

$$\mathbb{E}[\text{error}] = \text{Bias}(\hat{h})^2 + \text{Var}(\hat{h}) + \sigma^2$$

where predictions are $\hat{h} = \mathbb{E}[h_S]$ averaged over training sets $S$:

- **Bias** $\text{Bias}(\hat{h}) = \mathbb{E}[\hat{h}(x)] - h^*(x)$: Systematic error; how well the class can fit the true function
- **Variance** $\text{Var}(\hat{h}) = \mathbb{E}[(\hat{h}(x) - \mathbb{E}[\hat{h}(x)])^2]$: Sensitivity to training data fluctuations
- **Irreducible error** $\sigma^2$: Noise in $y$

**Tradeoff**: Simple models (low VCdim) have high bias but low variance; complex models have low bias but high variance. Optimal complexity balances the two.

Training error falls monotonically with capacity; test error is U-shaped: at low capacity, bias dominates; at high capacity, variance dominates.

## Regularization Theory

Regularization adds a penalty term to training loss to control model complexity:

$$\min_h \sum_i L(h(x_i), y_i) + \lambda \Omega(h)$$

where $\Omega(h)$ is a complexity measure (e.g., $\|w\|_2^2$ for weights, number of nonzero features, model depth).

### Theoretical Justification

**Tichonov regularization** (ridge regression): Minimizing $\|Xw - y\|^2 + \lambda\|w\|^2$ is equivalent to Bayesian MAP estimation with $w \sim N(0, (2\lambda)^{-1}I)$ prior.

**Regularization path stability**: Small $\lambda$ → low bias, high variance; large $\lambda$ → high bias, low variance. The path of empirical risks traces an optimal curve.

**Structural risk minimization** (Vapnik): Choose complexity level $d$ to minimize:

$$\text{error}_\text{test}(d) \leq \text{error}_\text{train}(d) + O\left(\sqrt{\frac{d \log(n/d)}{n}}\right)$$

The bound decreases with $n$, increases with $d$; optimal $d^*$ depends on true problem class.

## Kernel Methods & the Kernel Trick

Kernel methods lift linear methods into high-dimensional feature spaces implicitly, avoiding explicit feature computation.

### Kernel Trick

Given a feature map $\phi: \mathcal{X} \to \mathbb{R}^D$, compute inner products via a kernel $K(x, x') = \langle \phi(x), \phi(x') \rangle$ without knowing $\phi$.

**Linear classifier in feature space**:

$$f(x) = w^\top \phi(x) + b = \sum_i \alpha_i K(x, x_i) + b$$

Examples:
- **Polynomial**: $K(x, x') = (1 + x \cdot x')^d$ (degree-$d$ features)
- **RBF (Gaussian)**: $K(x, x') = \exp(-\gamma \|x - x'\|^2)$ (infinite-dimensional feature space)
- **Sigmoid**: $K(x, x') = \tanh(x \cdot x')$ (neural network-like)

**Support Vector Machines (SVM)**: Maximize margin $\frac{2}{\|w\|}$ subject to linear separation in feature space. Kernel trick allows non-linear decision boundaries while solving a convex problem.

### Reproducing Kernel Hilbert Space (RKHS)

Every kernel $K$ defines a Hilbert space $\mathcal{H}_K$ with rep lacing property:

$$f(x) = \langle f, K(x, \cdot) \rangle_{\mathcal{H}_K}$$

Kernel ridge regression minimizes:

$$\sum_i (f(x_i) - y_i)^2 + \lambda \|f\|_{\mathcal{H}_K}^2$$

Solution is finite: $f(x) = \sum_i \alpha_i K(x, x_i)$ (representer theorem).

**Kernel matrix** $K_{ij} = K(x_i, x_j)$ must be positive semi-definite (Mercer condition).

## Neural Tangent Kernel & Infinite Width Limits

**Neural Tangent Kernel (NTK)** (Jacot et al., 2018): At infinite width, a neural network training trajectory is equivalent to kernel regression with a fixed kernel.

For a net with width $m \to \infty$:

$$K_\infty(x, x') = \mathbb{E}_{w \sim N(0,I)}[\nabla_w f(x; w)^\top \nabla_w f(x'; w)]$$

where $f$ is evaluated at initialization.

Implications:
- Wide nets stay close to initialization (small parameter drift)
- Dynamics are linear: $f_t(x) = K_\infty f_0 (x) + \ldots$ (convolution)
- Interpolating (zero training error) is easy; generalization depends on NTK spectrum

**Gap**: Practical finite-width networks often generalize far better than NTK theory predicts (feature learning at scale).

## Double Descent & Benign Overfitting

**Double descent** (Belkin et al., 2019): Test error exhibits two peaks—classical U-shaped curve at intermediate interpolation threshold, then decreases again as model capacity increases further.

### Phases

1. **Under-parameterized** ($d < n$): Classical bias-variance curve; test error rises
2. **Interpolation threshold** ($d \approx n$): Test error peaks (minimal bias, maximal variance)
3. **Over-parameterized** ($d > n$): Test error falls again *despite* zero training error (benign overfitting)

**Mechanics**: High-capacity models like neural networks implicitly regularize via inductive bias (implicit bias of SGD: low-norm solutions, simple decision boundaries) rather than explicit penalty. Interpolating solutions with smallest norm generalize well.

**Scaling laws** (neural language models): Test loss $L(n) \approx An^{-\alpha}$ where $n$ is data or parameters; $\alpha \approx 0.07$ for parameters, $0.13$ for data (Kaplan et al., 2020). Optimal scaling: allocate compute between model size and training steps.

## Sample Complexity Trade-offs

**Fundamental tension**: 
- Few samples, weak guarantees: $O(1/\epsilon \cdot \log(|\mathcal{H}|/\delta))$ for finite $\mathcal{H}$
- Distribution-free, pessimistic: Worst-case over all distributions can be far from actual performance
- Specific distributions, better rates: e.g., margin-based bounds for SVM improve scaling under separation

**Cross-validation**: Estimate test error via held-out validation. Requires sacrificing training data; statistician's dilemma between fitting and evaluation.