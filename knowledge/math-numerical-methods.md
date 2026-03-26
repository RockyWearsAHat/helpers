# Numerical Methods — Floating Point, Approximation & Computational Mathematics

## The Gap Between Mathematics and Computation

Pure mathematics operates on real numbers with infinite precision. Computers represent numbers with finite bits. This gap produces surprising behaviors — correct-looking formulas that give wrong answers, algorithms that diverge when theory says they converge, and results that depend on the order of operations. Numerical methods are the discipline of computing useful answers despite these limitations.

## Floating-Point Representation

### IEEE 754 Standard

The dominant floating-point standard represents a number as:

$$(-1)^s \times 1.f \times 2^{e - \text{bias}}$$

where s is the sign bit, f is the fractional mantissa, and e is the biased exponent.

| Format           | Sign | Exponent | Mantissa | Total bits | Decimal digits |
| ---------------- | ---- | -------- | -------- | ---------- | -------------- |
| Half (float16)   | 1    | 5        | 10       | 16         | ~3.3           |
| Single (float32) | 1    | 8        | 23       | 32         | ~7.2           |
| Double (float64) | 1    | 11       | 52       | 64         | ~15.9          |
| Quad (float128)  | 1    | 15       | 112      | 128        | ~34.0          |

### Why 0.1 + 0.2 ≠ 0.3

The decimal fraction 0.1 has an infinite binary expansion (0.0001100110011...), similar to how 1/3 = 0.333... in decimal. When stored in finite bits, it is rounded. The sum of two rounded values accumulates error:

- 0.1 in float64 ≈ 0.1000000000000000055511151231257827021181583404541015625
- 0.2 in float64 ≈ 0.200000000000000011102230246251565404236316680908203125
- Their sum ≈ 0.3000000000000000444089209850062616169452667236328125
- The nearest float64 to 0.3 ≈ 0.299999999999999988897769753748434595763683319091796875

This is not a bug — it is an inherent property of binary floating-point representation.

### Special Values

| Value     | Meaning           | Produced by                           |
| --------- | ----------------- | ------------------------------------- |
| +0, −0    | Signed zeros      | Underflow, sign-preserving operations |
| ±∞        | Infinity          | Overflow, division by zero            |
| NaN       | Not a Number      | 0/0, ∞ − ∞, √(−1)                     |
| Denormals | Subnormal numbers | Gradual underflow near zero           |

NaN has the unusual property that NaN ≠ NaN — the only floating-point value not equal to itself. This is a deliberate design choice that can be used for detection but also causes subtle bugs when equality comparisons are used carelessly.

## Machine Epsilon and Precision Limits

Machine epsilon (ε) is the smallest value such that $1.0 + \varepsilon \neq 1.0$ in floating-point arithmetic.

| Format   | Machine epsilon |
| -------- | --------------- |
| float32  | ≈ 1.19 × 10⁻⁷   |
| float64  | ≈ 2.22 × 10⁻¹⁶  |
| float128 | ≈ 1.93 × 10⁻³⁴  |

Machine epsilon bounds the relative error from rounding a single operation. For $n$ chained operations, error can grow — linearly in favorable cases, exponentially in unstable ones.

### Comparing Floating-Point Numbers

Direct equality comparison (a == b) is unreliable for computed values. Common approaches:

- **Absolute tolerance**: |a − b| < ε — works when the magnitude of values is known and bounded
- **Relative tolerance**: |a − b| / max(|a|, |b|) < ε — scale-independent but fails near zero
- **Combined**: |a − b| < max(ε_abs, ε_rel × max(|a|, |b|)) — handles both cases
- **ULP comparison**: Check if values are within a specified number of representable floats of each other

The appropriate tolerance depends on the problem — there is no universal "right" epsilon.

## Sources of Numerical Error

### Cancellation

Subtracting nearly equal numbers amplifies relative error dramatically. If a and b agree to k digits, then a − b loses those k leading digits of significance.

Example: Computing $f(x) = \frac{1 - \cos(x)}{x^2}$ near x = 0.

- Direct evaluation: 1 − cos(0.0001) ≈ 1 − 0.999999995 = 5 × 10⁻⁹ — massive cancellation
- Reformulation: $\frac{2\sin^2(x/2)}{x^2}$ — avoids the subtraction entirely

### Overflow and Underflow

- **Overflow**: Result exceeds the largest representable number → ±∞
- **Underflow**: Result is closer to zero than the smallest representable number → 0 or denormal
- **Intermediate overflow**: Even when the final answer is representable, intermediate steps may overflow. Example: computing $\sqrt{a^2 + b^2}$ when a or b is large — reformulate as $|a| \sqrt{1 + (b/a)^2}$

### Accumulation and Ordering

Summing many floating-point numbers depends on the order of addition. Adding small values to a large accumulator loses the small values entirely when their magnitude falls below the accumulator's ULP.

Strategies for more accurate summation include:

- **Sorted summation**: Add values in ascending order of magnitude
- **Kahan (compensated) summation**: Track and correct the running error — yields nearly full precision at the cost of 4× the arithmetic
- **Pairwise summation**: Recursively sum pairs — O(log n) error growth vs O(n) for naive summation

## Numerical Stability

An algorithm is numerically stable if small perturbations in the input produce proportionally small perturbations in the output. Mathematically equivalent formulations can have vastly different stability properties.

### Forward vs Backward Stability

- **Forward stable**: The computed answer is close to the true answer
- **Backward stable**: The computed answer is the exact answer to a slightly perturbed input — a stronger and more practical guarantee

Many well-designed numerical algorithms are backward stable: they don't guarantee the exact right answer, but they guarantee the exact right answer to a problem very close to the one you asked.

### The Quadratic Formula — A Stability Example

The standard formula $x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$ suffers from cancellation when one root is much smaller than the other. If $b > 0$ and $b^2 \gg 4ac$, then $-b + \sqrt{b^2 - 4ac}$ subtracts nearly equal quantities.

The stabilized approach: compute the larger root directly, then use $x_1 x_2 = c/a$ to find the smaller root without subtraction.

## Root Finding

Finding values where $f(x) = 0$ is fundamental to computational mathematics.

### Methods Comparison

| Method                | Convergence          | Requires                       | Robustness | Notes                                               |
| --------------------- | -------------------- | ------------------------------ | ---------- | --------------------------------------------------- | -------- | ---------------------------------------------------- |
| Bisection             | Linear (1 bit/step)  | Bracket [a,b] with sign change | Very high  | Always converges; slow but reliable                 |
| Newton's              | Quadratic            | f'(x), good initial guess      | Low        | Fast near root; can diverge or cycle                |
| Secant                | Superlinear (~1.618) | Two initial points             | Moderate   | No derivative needed; less robust than bisection    |
| Brent's               | Superlinear          | Bracket                        | High       | Combines bisection + inverse quadratic; widely used |
| Fixed-point iteration | Linear               | x = g(x) form with             | g'         | < 1                                                 | Variable | Convergence depends on contraction mapping condition |

### Convergence Order

A method has convergence order p if the error at step n+1 satisfies $|e_{n+1}| \leq C|e_n|^p$.

- Linear (p = 1): Error reduces by a constant factor each step
- Quadratic (p = 2): Correct digits roughly double each step — extremely fast near the root
- Superlinear (1 < p < 2): Between linear and quadratic

Higher-order convergence sounds appealing but often requires stronger assumptions (smoothness, derivative availability, good starting point). Hybrid methods that begin with robust low-order steps and switch to fast high-order steps near convergence combine the strengths.

## Interpolation

Constructing a function that passes through given data points.

### Polynomial Interpolation

Given n+1 points, a unique polynomial of degree ≤ n passes through them (Lagrange or Newton form). However:

- **Runge phenomenon**: High-degree polynomials oscillate wildly between equally-spaced points, especially near the endpoints
- **Chebyshev nodes**: Clustering points near the endpoints (at zeros of Chebyshev polynomials) dramatically reduces oscillation
- **Conditioning**: The Lebesgue constant measures how sensitive interpolation is to data perturbation — it grows logarithmically with Chebyshev nodes but exponentially with equidistant nodes

### Spline Interpolation

Piecewise polynomials (typically cubic) joined with continuity conditions at the knots.

| Spline type   | Continuity   | Degrees of freedom  | Trade-off                     |
| ------------- | ------------ | ------------------- | ----------------------------- |
| Linear        | C⁰           | n intervals         | No oscillation but non-smooth |
| Cubic natural | C²           | n+1 unknowns        | Smooth; standard choice       |
| B-spline      | Configurable | Local support       | Good for design curves        |
| Hermite       | C¹           | Matches derivatives | When slope data is available  |

Splines avoid the Runge phenomenon by keeping polynomial degree low while increasing the number of pieces. The trade-off is between smoothness (higher continuity) and local control (changes to one data point shouldn't affect distant parts of the curve).

## Numerical Integration (Quadrature)

Approximating $\int_a^b f(x)\,dx$ when no closed-form antiderivative exists.

| Method             | Error order  | Evaluations    | Best for                              |
| ------------------ | ------------ | -------------- | ------------------------------------- |
| Trapezoidal        | O(h²)        | n+1            | Smooth periodic functions             |
| Simpson's 1/3      | O(h⁴)        | 2n+1 (odd)     | Smooth functions                      |
| Simpson's 3/8      | O(h⁴)        | 3n+1           | When point count isn't odd            |
| Gaussian (n-point) | O(h²ⁿ)       | n per interval | High accuracy, smooth integrands      |
| Romberg            | Extrapolated | Adaptive       | Smooth functions; refines trapezoidal |
| Adaptive           | Variable     | Variable       | Functions with localized features     |

### Key Considerations

- **Gaussian quadrature** achieves remarkable accuracy by choosing both weights and evaluation points optimally — n points integrate polynomials of degree 2n−1 exactly
- **Adaptive methods** subdivide intervals where the integrand varies rapidly and use coarser resolution where it's smooth — essential for functions with localized spikes or rapid oscillation
- **Singularities**: Integrands with singularities or discontinuities can cause standard methods to converge slowly or fail entirely — special techniques (variable substitution, singularity subtraction) may be necessary
- **Multiple dimensions**: Quadrature in high dimensions suffers from the "curse of dimensionality" — Monte Carlo methods become competitive above ~4-5 dimensions despite their slow O(1/√N) convergence

## ODE Solvers

Numerically solving $y' = f(t, y)$ with initial condition $y(t_0) = y_0$.

### Method Families

| Method                | Order    | Steps                 | Stability                | Notes                                                        |
| --------------------- | -------- | --------------------- | ------------------------ | ------------------------------------------------------------ |
| Forward Euler         | 1        | 1                     | Conditional              | Simple; illustrative; rarely used alone                      |
| Backward Euler        | 1        | 1 (implicit)          | Unconditional (A-stable) | Requires solving nonlinear equations                         |
| Midpoint (RK2)        | 2        | 2                     | Conditional              | Better than Euler, same cost                                 |
| Classical RK4         | 4        | 4                     | Conditional              | The workhorse of ODE solving                                 |
| Dormand-Prince (RK45) | 4/5      | 6                     | Adaptive step            | Embedded pair for error estimation                           |
| Adams-Bashforth       | Variable | Multi-step            | Conditional              | Uses previous solution values; efficient for smooth problems |
| BDF methods           | Variable | Multi-step (implicit) | A-stable (low order)     | Designed for stiff problems                                  |

### Stiff Problems

A stiff ODE has components evolving on vastly different timescales. Explicit methods require prohibitively small time steps to maintain stability, even when the solution itself varies slowly. Implicit methods (backward Euler, BDF, implicit Runge-Kutta) remain stable with much larger steps but require solving a system of equations at each step.

Recognizing stiffness is important — applying the wrong solver class leads to either glacially slow computation (explicit on stiff problems) or unnecessary per-step cost (implicit on non-stiff problems).

### Adaptive Step Size Control

Embedded Runge-Kutta pairs (like Dormand-Prince) compute two estimates of different order at each step. The difference estimates the local error, which drives automatic step size selection:

- Error too large → reject step, reduce step size
- Error well within tolerance → accept step, may increase step size
- This balances accuracy against computational cost without user-specified step sizes

## Iterative Methods for Linear Systems

For large sparse systems Ax = b, direct methods (Gaussian elimination) are often impractical due to fill-in. Iterative methods compute successive approximations.

| Method             | Convergence                        | Requirements                | Notes                                                   |
| ------------------ | ---------------------------------- | --------------------------- | ------------------------------------------------------- |
| Jacobi             | Linear                             | Diagonally dominant         | Embarrassingly parallel                                 |
| Gauss-Seidel       | Linear (faster)                    | Diagonally dominant or SPD  | Uses latest values immediately                          |
| SOR                | Accelerated linear                 | Good ω parameter            | Optimal ω is problem-dependent                          |
| Conjugate Gradient | Superlinear (with preconditioning) | Symmetric positive definite | The standard for SPD systems                            |
| GMRES              | Depends on spectrum                | General nonsymmetric        | Memory grows with iterations; restart strategies needed |
| BiCGSTAB           | Variable                           | General nonsymmetric        | Fixed memory per iteration                              |

### Preconditioning

The convergence rate of iterative solvers depends on the condition number of the matrix. Preconditioning transforms the system to have a more favorable spectrum:

$$M^{-1}Ax = M^{-1}b$$

where M approximates A but is cheap to invert. The ideal preconditioner makes $M^{-1}A$ close to the identity — but then M is as hard to invert as solving the original system. Practical preconditioners (incomplete LU, algebraic multigrid, domain decomposition) balance approximation quality against cost.

## The Condition Number

The condition number $\kappa(A) = \|A\| \cdot \|A^{-1}\|$ measures how sensitive a linear system's solution is to perturbations in the input.

| Condition number | Interpretation                              |
| ---------------- | ------------------------------------------- |
| κ ≈ 1            | Well-conditioned; results are reliable      |
| κ ≈ 10⁶          | Lose ~6 digits of precision                 |
| κ ≈ 10¹⁵         | Results in double precision are meaningless |
| κ = ∞            | Singular matrix; no unique solution         |

A large condition number means the problem itself is sensitive — no algorithm can overcome ill-conditioning without reformulating the problem or using higher precision.

This concept extends beyond linear algebra: any computational problem has an inherent condition number relating input perturbation to output perturbation. The condition number is a property of the problem, not the algorithm.

## Symbolic vs Numerical Computation

| Aspect         | Symbolic                                                    | Numerical                                           |
| -------------- | ----------------------------------------------------------- | --------------------------------------------------- |
| Representation | Exact expressions (√2, π, x+1)                              | Finite-precision numbers                            |
| Results        | Closed-form when possible                                   | Approximate with bounded error                      |
| Failure mode   | "Cannot solve" (no closed form)                             | Silent inaccuracy or divergence                     |
| Performance    | Expression swell for complex problems                       | Predictable computational cost                      |
| Best for       | Deriving formulas, proving identities, small exact problems | Large-scale computation, numerical data, simulation |

The approaches complement each other. Symbolic computation can derive a formula that numerical methods then evaluate efficiently. Numerical issues in a formula (cancellation, overflow) can sometimes be resolved by symbolic simplification.

## When to Trust Numerical Results

Confidence in numerical output depends on several factors:

- **Verify against known solutions**: Test with inputs that have analytical answers — if the method fails on known cases, it will fail on unknown ones
- **Check convergence**: Run with finer resolution (smaller h, more iterations) — the answer should stabilize
- **Vary precision**: If results change significantly between float32 and float64, numerical issues are present
- **Monitor residuals**: For Ax = b, compute ‖Ax − b‖ — a small residual doesn't guarantee accuracy (ill-conditioned systems can have small residuals with large errors) but a large residual guarantees a problem
- **Sensitivity analysis**: Perturb inputs slightly — if outputs change dramatically, the problem may be ill-conditioned rather than the algorithm flawed

## Applications Across Domains

### Physics Simulations

Solving PDEs (heat equation, wave equation, Navier-Stokes) via discretization — finite differences, finite elements, spectral methods. Stability constraints like the CFL condition limit time step sizes for explicit methods.

### Financial Calculations

Decimal representation matters — binary floating point introduces errors unacceptable for monetary calculations. Options pricing (Black-Scholes) involves root finding and numerical integration. Monte Carlo simulation for portfolio risk requires understanding of PRNG quality and variance reduction techniques.

### Signal Processing

FFT as a numerical algorithm — roundoff error accumulates differently from the mathematical DFT. Windowing functions, filter design, and spectral analysis all involve numerical trade-offs between frequency resolution and spectral leakage.

### Optimization

Gradient descent, Newton's method for optimization, quasi-Newton methods (L-BFGS). Step size selection, convergence criteria, and saddle point detection are all numerical concerns. The choice between first-order (gradient only) and second-order (Hessian) methods involves a fundamental trade-off between per-iteration cost and convergence rate.

## Common Pitfalls and Diagnostic Strategies

| Symptom                       | Likely cause                            | Investigation approach                                      |
| ----------------------------- | --------------------------------------- | ----------------------------------------------------------- |
| Result is NaN                 | 0/0, ∞−∞, or √(negative)                | Trace which operation first produced NaN                    |
| Result is ±∞                  | Overflow in intermediate computation    | Check for large intermediate values; rescale                |
| Loss of significance          | Catastrophic cancellation               | Look for subtraction of nearly equal values                 |
| Algorithm diverges            | Step size too large or problem is stiff | Reduce step size; consider implicit methods                 |
| Answer changes with precision | Ill-conditioned problem                 | Compute condition number; reformulate if possible           |
| Slow convergence              | Poor initial guess or preconditioning   | Improve starting point; add preconditioning                 |
| Oscillating residuals         | Non-convergent iteration                | Check spectral radius; method may not apply to this problem |

Numerical issues often manifest far from their source. A NaN in the output might originate from a division in an early computation step, propagating silently through subsequent operations. Systematic approaches — bisecting the computation, checking intermediate values, varying precision — are more productive than guessing.
