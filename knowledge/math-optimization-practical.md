# Optimization — Gradient Methods, Convexity & Practical Algorithms

Optimization finds the best solution from a feasible set. In machine learning, this means minimizing loss over parameters. Theory (convexity, smoothness) guides algorithm choice; practice requires tuning and problem-specific adaptations.

## Gradient Descent & Convergence

**Unconstrained minimization**: Minimize $f(x)$ over $x \in \mathbb{R}^d$.

**Gradient descent update**:

$$x_{t+1} = x_t - \eta \nabla f(x_t)$$

where $\eta > 0$ is the step size (learning rate).

**Convergence (smooth, convex case)**: If $f$ is convex and $L$-smooth ($|\nabla f(x) - \nabla f(y)| \leq L \|x - y\|$), with step size $\eta \leq 1/L$:

$$f(x_t) - f(x^*) = O(1/t)$$

Linear convergence (geometric): If additionally strongly convex ($\mu > 0$):

$$\|x_t - x^*\|^2 = O((1 - \mu\eta)^t)$$

**Non-convex case**: No global optimality guarantee, but can find stationary points $\|\nabla f(x)\| \leq \epsilon$ in $O(1/\epsilon^2)$ iterations.

**Practical considerations**:
- Small $\eta$ → slow convergence; large $\eta$ → divergence
- Adaptive learning rates (below) address this
- Momentum accelerates convergence

## Gradient Descent Variants

### Stochastic Gradient Descent (SGD)

Use mini-batch gradient $\tilde{\nabla} f(x_t)$ (estimate from subset of data) instead of full gradient:

$$x_{t+1} = x_t - \eta \tilde{\nabla} f(x_t)$$

**Advantages**:
- Memory efficient (process data in chunks)
- Faster iterations (partial gradient cheaper)
- Noise can help escape local minima (beneficial for non-convex)

**Disadvantage**: Noisy updates slow convergence (variance component added).

**Convergence**: Convex, smooth case gives $O(1/\sqrt{t})$ (halve error requires 4x more iterations; compare GD's $O(1/t)$).

**Variance reduction** (SAG, SVRG): Store gradients at past iterates to reduce variance; recovers $O(1/t)$ rate while keeping per-iteration cost of SGD.

### Momentum & Nesterov Acceleration

**Heavy ball momentum**:

$$x_{t+1} = x_t - \eta \nabla f(x_t) + \beta (x_t - x_{t-1})$$

Accumulates velocity; overshoots in directions of consistent gradient, dampens oscillations in noisy directions.

**Nesterov accelerated gradient (NAG)**:

$$x_{t+1} = (x_t - \eta \nabla f(x_t + \beta (x_t - x_{t-1}))) + \beta(x_t - x_{t-1})$$

Evaluates gradient at lookahead position; optimal acceleration for smooth convex (rate $O(1/t^2)$).

**Typical parameters**: $\beta = 0.9$ or momentum coefficient schedule $\beta_t = 1 - 3/(t+5)$.

### Adaptive Learning Rates

**AdaGrad** (Duchi et al., 2011):

$$x_{t+1} = x_t - \frac{\eta}{\sqrt{g_t + \epsilon}} \odot \tilde{\nabla} f(x_t)$$

where $g_t = \sum_{s=1}^t (\tilde{\nabla} f(x_s))^2$ accumulates squared gradients; $\odot$ is element-wise product; $\epsilon$ prevents division by zero.

Per-coordinate learning rates scale inversely with past gradient magnitude. **Problem**: $g_t$ grows unbounded; learning rate → 0, eventually stops learning.

**RMSprop** (Hinton, 2012):

$$g_t = \rho g_{t-1} + (1-\rho) (\tilde{\nabla} f(x_t))^2, \quad x_{t+1} = x_t - \frac{\eta}{\sqrt{g_t + \epsilon}} \odot \tilde{\nabla} f(x_t)$$

Exponential moving average of squared gradients (decay memory). Fixes AdaGrad's decay problem.

**Adam** (Kingma & Ba, 2014):

$$m_t = \beta_1 m_{t-1} + (1-\beta_1)\tilde{\nabla} f(x_t), \quad v_t = \beta_2 v_{t-1} + (1-\beta_2)(\tilde{\nabla} f(x_t))^2$$

$$\hat{m}_t = m_t / (1-\beta_1^t), \quad \hat{v}_t = v_t / (1-\beta_2^t)$$

$$x_{t+1} = x_t - \frac{\eta}{\sqrt{\hat{v}_t} + \epsilon} \hat{m}_t$$

Combines momentum ($m_t$, first moment) and RMSprop ($v_t$, second moment). Defaults $\beta_1 = 0.9, \beta_2 = 0.999$ work well across many problems.

**Lion** (Chen et al., 2023): Uses sign of momentum instead of magnitude; reduces memory, often outperforms Adam on large models.

## Convexity & Smoothness

**Convex function**: $f(\lambda x + (1-\lambda)y) \leq \lambda f(x) + (1-\lambda)f(y)$ for $\lambda \in [0,1]$.

- **Strongly convex**: $f(y) \geq f(x) + \nabla f(x)^\top(y-x) + \frac{\mu}{2}\|y-x\|^2$ (curvature $\mu > 0$)
- **Smooth (L-smooth)**: $\|\nabla f(x) - \nabla f(y)\| \leq L\|x-y\|$ (gradient doesn't change too fast)

**Condition number**: $\kappa = L/\mu$ (well-conditioned: $\kappa$ small; ill-conditioned: $\kappa$ large).

Gradient descent converges faster with better condition number: $O(\kappa \log(1/\epsilon))$ to $\epsilon$-precision.

**Non-convex**: No global optimality; descent methods find stationary points or saddle points. Escaping saddles requires noise or curvature information.

## Constrained Optimization

**Constrained problem**:

$$\min_x f(x) \quad \text{subject to } g_i(x) \leq 0, h_j(x) = 0$$

### Lagrange Multipliers

Form the **Lagrangian**:

$$L(x, \lambda, \mu) = f(x) + \sum_i \lambda_i g_i(x) + \sum_j \mu_j h_j(x)$$

**Karush-Kuhn-Tucker (KKT) conditions** (necessary at optimality):

$$\nabla_x L = 0, \quad \lambda_i \geq 0, \quad \lambda_i g_i(x) = 0 \text{ (complementary slackness)}, \quad g_i(x) \leq 0, \quad h_j(x) = 0$$

For **convex** problems, KKT is also sufficient.

**Dual problem**: $\max_{\lambda, \mu} \min_x L(x, \lambda, \mu)$. Dual feasibility ($\lambda \geq 0$) and strong duality (primal = dual) used in algorithms.

### Methods

**Projected gradient descent**: $x_{t+1} = \Pi_C(x_t - \eta \nabla f(x_t))$ where $\Pi_C$ projects onto feasible set $C$. Converges for convex, smooth $f$.

**Augmented Lagrangian**: Iteratively minimize Lagrangian with penalty, then update multipliers. Balances satisfaction of constraints and objective.

## Linear Programming & Interior Point Methods

**Linear program**:

$$\min_x c^\top x \quad \text{subject to } Ax \leq b, \quad x \geq 0$$

### Simplex Method

Move along edges of polytope, pivoting to adjacent vertices with lower objective.

- **Efficiency**: Exponential worst-case, but empirically fast
- **Degeneracy**: Can cycle; handled by perturbation or lexicographic rules

### Interior Point Methods

Stay inside the feasible region (polytope interior), moving toward optimum.

**Central path**: $\mathcal{C}(t) = \{\min c^\top x + \frac{1}{t}\sum_i \log x_i : Ax \leq b\}$ (barrier function includes $\log x$ penalty).

Path converges to optimum as $t \to \infty$. Newton's method on barrier problem finds points on path.

**Computational complexity**: $O(n^3 L)$ where $n$ = variables, $L$ = bit length. Polynomial; outperforms simplex on large problems.

## Evolutionary & Population-Based Algorithms

For non-differentiable, discrete, or multimodal problems:

**Genetic algorithms**: Population of candidate solutions; select (fitness-based), mutate, recombine. Maintains diversity; escapes local optima. Slow; sampling inefficient.

**Particle swarm optimization (PSO)**: Particles move through space, attracted to best-found positions. Parameter-sensitive but parallelizable.

**Differential evolution**: Population members propose mutations; accept if better. Robust, simple; widely used in practice.

## Hyperparameter Optimization

Optimizing meta-parameters (learning rate, regularization, architecture) is a black-box optimization problem (usually non-convex, no gradients).

**Grid search**: Exhaustive over discrete grid. Exponentially expensive in dimension count.

**Random search**: Sample uniformly. Often competitive with grid; avoids axis-aligned inefficiency.

**Bayesian optimization**: Model objective as Gaussian process; acquisition function (e.g., expected improvement) guides next evaluation. Sample-efficient; standard for AutoML.

**Population-based training**: Evolve population of configurations, periodically adopting hyperparameters of well-performing members. Efficient for large-scale training.

## Practical Trade-offs

- **Batch vs online**: Large batches are compute-efficient but may generalize worse; small batches add beneficial noise
- **Momentum tuning**: Helps but requires care; decay schedules (e.g., step-wise) balance robustness
- **Adaptive methods**: Adam is default; converges faster early but may generalize worse than SGD with momentum on some tasks
- **Second-order (Newton)**: $O(d^3)$ per iteration; escapes plateaus faster; rarely used due to cost unless Hessian-free approximations employed