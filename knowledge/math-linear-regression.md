# Linear Regression — OLS, Assumptions, Diagnostics & Regularization

Linear regression models the relationship between a dependent variable $y$ and one or more independent variables $\mathbf{x}$ as a linear function. Despite its simplicity, it remains foundational in statistics, econometrics, and machine learning because it is interpretable, computationally efficient, and often surprisingly effective.

## Ordinary Least Squares (OLS)

OLS minimizes the sum of squared residuals:

$$\text{minimize} \quad \sum_{i=1}^{n} (y_i - \mathbf{x}_i^T \boldsymbol{\beta})^2$$

The closed-form solution (when $\mathbf{X}^T\mathbf{X}$ is invertible) is:

$$\hat{\boldsymbol{\beta}} = (\mathbf{X}^T \mathbf{X})^{-1} \mathbf{X}^T \mathbf{y}$$

This is the maximum likelihood estimator under the classical linear model assumptions. Computationally, OLS is solved via QR decomposition or Cholesky factorization of $\mathbf{X}^T\mathbf{X}$ to avoid numerical instability from explicit inversion.

**Key properties**: Under the classical assumptions, OLS is unbiased, consistent, and has minimum variance among linear unbiased estimators (BLUE — Best Linear Unbiased Estimator).

## Classical Assumptions

The Gauss-Markov theorem requires five assumptions for OLS to be BLUE:

### 1. Linearity
The true relationship is linear: $y_i = \mathbf{x}_i^T \boldsymbol{\beta} + \epsilon_i$

Violations mean the model is misspecified. Check with scatter plots (bivariate) or partial regression plots (multivariate). Quadratic, logarithmic, or interaction terms may be needed. Polynomial features can capture non-linear relationships.

### 2. Independence of Errors
Consecutive errors are uncorrelated: $\text{Cov}(\epsilon_i, \epsilon_j) = 0$ for $i \neq j$

Violations occur commonly in time series data. Check with Durbin-Watson test ($d = \frac{\sum_{t=2}^{n}(\epsilon_t - \epsilon_{t-1})^2}{\sum_{t=1}^{n}\epsilon_t^2}$, ranges 0-4, ideal around 2). Time series require autoregressive (AR) models or generalized least squares (GLS).

### 3. Homoscedasticity
Constant error variance: $\text{Var}(\epsilon_i) = \sigma^2$ for all $i$

Heteroscedasticity (non-constant variance) occurs when errors grow with $x$ (e.g., income vs. spending). Effects: OLS estimates remain unbiased but are inefficient; standard errors are incorrect, invalidating hypothesis tests. Detect via Breusch-Pagan test, White test, or residual plots (scatter plot residuals vs. fitted values). **Fix**: Weighted least squares (WLS) assigns higher weight to observations with lower variance. Alternatively, transform $y$ (log, square root) or use robust standard errors.

### 4. Normality of Errors
Errors follow a normal distribution: $\epsilon_i \sim N(0, \sigma^2)$

This is crucial for hypothesis tests and confidence intervals, less critical for large samples (Central Limit Theorem). Check with Q-Q plots (plot quantiles of residuals vs. normal quantiles) or Shapiro-Wilk test. Moderate violations matter less for parameter estimates; severe violations affect inference. Transformation (Box-Cox) or robust regression methods may help.

### 5. No Perfect Multicollinearity
Predictors are not linear combinations of each other: $\text{rank}(\mathbf{X}) = p$ (full rank)

Perfect multicollinearity makes $\mathbf{X}^T\mathbf{X}$ singular; $\hat{\boldsymbol{\beta}}$ cannot be computed. Rare in practice with modern data collection. Near-multicollinearity (high correlation) is common and causes:
- Large coefficient estimates
- High standard errors for affected coefficients
- Unstable estimates (small data changes cause large coefficient swings)

Detect via correlation matrix, variance inflation factor (VIF): $\text{VIF}_j = \frac{1}{1 - R_j^2}$, where $R_j^2$ is the $R^2$ from regressing $x_j$ on other predictors. VIF > 5-10 suggests problematic multicollinearity.

**Fix**: Remove or combine correlated variables, use principal component regression, or apply regularization (see below).

## Diagnostics and Model Validation

### Residual Analysis
Residuals $\hat{\epsilon}_i = y_i - \hat{y}_i$ reveal assumption violations:

- **Residuals vs. Fitted**: Should show random scatter around zero. Patterns indicate non-linearity or heteroscedasticity.
- **Q-Q Plot**: Points should follow the diagonal line. Deviations at tails indicate non-normality.
- **Scale-Location (Spread-Location)**: Square root of standardized residuals vs. fitted values. Sloped pattern indicates heteroscedasticity.
- **Residuals vs. Leverage**: Identifies high-leverage points (extreme $x$ values) and influential points. A point with high leverage AND large residual is influential.

### Cook's Distance
Measures how much removing observation $i$ changes the fitted model:

$$D_i = \frac{(\hat{\mathbf{y}} - \hat{\mathbf{y}}_{(-i)})^T(\hat{\mathbf{y}} - \hat{\mathbf{y}}_{(-i)})}{p \cdot \text{MSE}}$$

Values > 1 (or sometimes > 4/n) flag influential points. Investigate whether they are errors or represent a distinct subpopulation. Robust regression methods downweight high-Cook's-distance points.

## Multicollinearity and Feature Selection

### Feature Selection Approaches

**Subset selection**: Forward, backward, or stepwise. Simple but computationally expensive for many features. Risk of overfitting with automated selection.

**Shrinkage methods** (see Regularization below): Continuously shrink coefficients; no discrete subset chosen.

**Domain knowledge**: Preferred when available. Select features based on causal reasoning, not data-driven associations.

## Regularization: Ridge, Lasso, and Elastic Net

When $p$ is large or multicollinearity is severe, OLS overfits. Regularization adds a penalty to the loss function, trading bias for variance reduction.

### Ridge Regression (L2)
$$\text{minimize} \quad \sum_{i=1}^{n} (y_i - \mathbf{x}_i^T \boldsymbol{\beta})^2 + \lambda \sum_{j=1}^{p} \beta_j^2$$

**Effect**: Shrinks all coefficients toward zero proportionally; none are exactly zero. Keeps all features. Useful when multicollinearity is the main problem. The tuning parameter $\lambda$ controls shrinkage; higher $\lambda$ means more shrinkage. Select $\lambda$ via cross-validation.

### Lasso Regression (L1)
$$\text{minimize} \quad \sum_{i=1}^{n} (y_i - \mathbf{x}_i^T \boldsymbol{\beta})^2 + \lambda \sum_{j=1}^{p} |\beta_j|$$

**Effect**: Shrinks coefficients AND can set some exactly to zero, performing automatic feature selection. Extremely useful when $p \gg n$ (more features than samples). Solves multicollinearity AND reduces model complexity.

**Limitation**: When features are highly correlated, Lasso arbitrarily selects one and zeros others. The chosen feature can be unstable.

### Elastic Net
$$\text{minimize} \quad \sum_{i=1}^{n} (y_i - \mathbf{x}_i^T \boldsymbol{\beta})^2 + \lambda \left( \alpha \sum_{j=1}^{p} |\beta_j| + (1-\alpha) \sum_{j=1}^{p} \beta_j^2 \right)$$

Combines L1 and L2. Parameter $\alpha \in [0, 1]$ blends Lasso ($\alpha=1$) and Ridge ($\alpha=0$). Provides feature selection (like Lasso) while handling correlated features better (like Ridge).

## Hypothesis Testing and Inference

Under the classical assumptions, for each coefficient:

$$t_j = \frac{\hat{\beta}_j}{\text{SE}(\hat{\beta}_j)} \sim t_{n-p-1}$$

where $\text{SE}(\hat{\beta}_j)$ is the standard error estimated from the residual variance. Used for significance tests (null: $\beta_j = 0$) and confidence intervals.

**F-statistic** tests joint significance: $H_0: \beta_1 = \cdots = \beta_p = 0$ (all coefficients zero except intercept).

$$F = \frac{(\text{TSS} - \text{RSS})/p}{\text{RSS}/(n-p-1)} \sim F_{p, n-p-1}$$

where TSS is total sum of squares, RSS is residual sum of squares.

## Key Limitations and When OLS Fails

- **Non-linear relationships**: Use polynomial features, splines, or GAMs.
- **Heteroscedasticity without robust SEs**: Coefficients are unbiased but inference is wrong.
- **Correlated errors (time series)**: Use autoregressive models, time series cross-validation.
- **High-dimensional data** ($p > n$): OLS is undefined; use Lasso, Ridge, or elastic net.
- **Outliers and leverage**: OLS is sensitive; robust regression or M-estimation more resilient.
- **Causal inference**: Correlation ≠ causation. Omitted variable bias remains unless features are chosen for causal structure, not just prediction.

**Trade-off**: OLS is optimized for coefficient estimation under classical assumptions. For prediction on new data, regularized methods often generalize better despite introducing bias.

See also: math-linear-algebra.md, ml-model-evaluation.md, math-optimization.md, statistics-hypothesis-testing.md