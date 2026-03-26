# Data Science Time Series — Analysis, Forecasting, and Anomaly Detection

## Stationarity and Differencing

A time series is **stationary** if its statistical properties (mean, variance, autocorrelation) are constant over time. Most classical time series methods (ARIMA, exponential smoothing) assume stationarity.

**Graphical checks**:
- Visual inspection: No obvious trend, no changing variance
- **ACF plot**: Auto-correlation should decay quickly; slow decay signals non-stationarity
- **KPSS test**: Null hypothesis is stationarity (reject = non-stationary)
- **ADF test** (Augmented Dickey-Fuller): Null hypothesis is unit root / non-stationarity (reject = stationary)

**Differencing** achieves stationarity:
- **First difference**: $\Delta y_t = y_t - y_{t-1}$ (removes linear trends)
- **Seasonal difference**: $\Delta_s y_t = y_t - y_{t-s}$ (removes seasonal patterns)
- **Log transformation**: Stabilizes variance when variability increases with level

Example: Raw daily revenue may trend upward and increase in variance → log-transform → first-difference → stationary.

## Autocorrelation (ACF) and Partial Autocorrelation (PACF)

**ACF** (Auto-Correlation Function): Correlation between $y_t$ and $y_{t-k}$ for lag k. Decaying ACF suggests AR (autoregressive); cutoff suggests MA (moving average).

**PACF** (Partial Auto-Correlation Function): Correlation between $y_t$ and $y_{t-k}$ *after removing* intermediate lags. Cutoff in PACF suggests AR order; cutoff in ACF suggests MA order.

**Interpretation**:
- ACF decays, PACF cuts off → AR process
- ACF cuts off, PACF decays → MA process
- Both decay → ARMA process
- No decay (non-stationary) → difference and re-examine

## ARIMA: The Classical Framework

**ARIMA(p,d,q)** combines three components:

1. **AR(p)** (AutoRegressive): $y_t = c + \phi_1 y_{t-1} + \phi_2 y_{t-2} + ... + \phi_p y_{t-p} + \epsilon_t$
   - Regress on past values
   - PACF cutoff at lag p suggests AR(p)

2. **I(d)** (Integrated): Differencing order
   - d=0: No differencing (already stationary)
   - d=1: First difference (removes trend)
   - d=2: Second difference (removes quadratic trend)

3. **MA(q)** (Moving Average): $y_t = c + \epsilon_t + \theta_1 \epsilon_{t-1} + ... + \theta_q \epsilon_{t-q}$
   - Regress on past shocks
   - ACF cutoff at lag q suggests MA(q)

**Selection algorithm** (Box-Jenkins):
1. Plot series and differencing plots; determine d
2. Inspect ACF/PACF to estimate p and q
3. Fit multiple ARIMA(p,d,q) candidates, compare AIC
4. Check residuals: should be white noise (ACF of residuals should show no autocorrelation)

Limitations: Assumes linear relationships, linear trend only, no external predictors.

## Exponential Smoothing and Holst-Winters

Exponential smoothing gives more weight to recent observations:

$$\hat{y}_{t+1} = \alpha y_t + (1-\alpha) \hat{y}_t$$

Where α ∈ [0,1] is the smoothing parameter. α=1 uses only the latest value; α=0 ignores new data.

**Simple exponential smoothing** (SES): For level-only stationary series. Forecast is flat.

**Holt's method** (linear trend):
$$\text{Level: } \ell_t = \alpha y_t + (1-\alpha)(\ell_{t-1} + b_{t-1})$$
$$\text{Trend: } b_t = \beta (\ell_t - \ell_{t-1}) + (1-\beta) b_{t-1}$$

Forecast: $\hat{y}_{t+h} = \ell_t + h \cdot b_t$

**Holt-Winters** (seasonal): Adds seasonal component:
$$s_t = \gamma \frac{y_t}{\ell_{t-1}} + (1-\gamma) s_{t-m}$$ (multiplicative)

With multiplicative seasonality: $\hat{y}_{t+h} = (\ell_t + h \cdot b_t) \cdot s_{t+h-m}$

Advantage: Computationally fast, closed-form, automatically handles level/trend/seasonality. Disadvantage: Limited accuracy on complex patterns, no uncertainty quantification without bias adjustment.

## Prophet and Bayesian Forecasting

**Prophet** (Facebook) combines components:
$$y_t = T(t) + S(t) + E_t$$

- **Trend T(t)**: Piecewise linear segments with automatic change-point detection
- **Seasonality S(t)**: Fourier series (multiple harmonics capture complex seasonal patterns)
- **Holidays/Events**: Additive regressors for known shocks
- **Error E_t**: Modeled as Gaussian noise initially, but robust forecasts use Student-t to reduce outlier sensitivity

Fitting uses Bayesian inference (Stan); provides credible intervals, not just point forecasts.

Strengths: Handles missing data, robust to outliers, change-points, interpretable components. Weaknesses: Assumes additive structure, can overfit multiple seasonalities.

## Seasonal Decomposition (STL)

**STL** (Seasonal-Trend decomposition using LOESS):
$$y_t = T_t + S_t + R_t$$

- **Trend**: Smoothed series (LOESS locally-weighted regression)
- **Seasonal**: Repeated pattern within a period
- **Residual**: Remainder

Advantages over ARIMA-based decomposition: Non-parametric (no model assumptions), handles multiple seasonalities, robust to outliers via median polish.

Usage: Visualize components to understand drivers of variation; deseasonalize before modeling (useful for forecasting detrended/deseasonalized separately).

## Anomaly Detection in Time Series

**Approaches**:

1. **Statistical thresholds**: Flag values > 2-3 standard deviations from rolling mean. Fast, interpretable, but assumes normality and stationary variance.

2. **Isolation Forest**: Tree-based ensemble detecting anomalies as rare data points. No distributional assumptions. Works well on multivariate time series.

3. **Seasonal decomposition + thresholds**: Decompose, apply threshold to residuals. Handles trend/seasonality.

4. **CUSUM** (Cumulative Sum Control Chart): Detect level shifts. Sum deviations from target; alert if cumsum exceeds threshold.

5. **Neural networks**: LSTM autoencoders learn normal patterns; high reconstruction error = anomaly. Captures complex temporal patterns but requires large training datasets.

Practical: Combine multiple signals (statistical + learned thresholds); annotate false positives to retrain.

## Forecasting Evaluation Metrics

Metrics chosen depend on whether minimizing absolute errors, percentage errors, or distribution calibration matters.

| Metric | Formula | Use Case | Limitation |
|--------|---------|----------|-----------|
| **MAE** | $\frac{1}{n}\sum\|\hat{y}_t - y_t\|$ | Interpretable in original units; robust | Ignores scale (hard to compare series) |
| **RMSE** | $\sqrt{\frac{1}{n}\sum(\hat{y}_t - y_t)^2}$ | Standard; penalizes large errors | Not interpretable in original units |
| **MAPE** | $\frac{1}{n}\sum\|\frac{y_t - \hat{y}_t}{y_t}\|$ | Percentage error; scale-independent | Undefined when $y_t = 0$; asymmetric (over/under estimates not equally weighted) |
| **SMAPE** | $\frac{2}{n}\sum\|\frac{y_t - \hat{y}_t}{y_t + \hat{y}_t}\|$ | Symmetric; bounds [0,200%] | Less interpretable; less common |
| **CRPS** | Continuous Ranked Probability Score | Probabilistic forecast; considers full distribution | Hard to compute and interpret |

No metric is universally best; domain determines relevance. For business decisions, forecast interval coverage (are 95% intervals actually 95%?) often matters most.

## Neural Forecasting: N-BEATS and N-HiTS

**N-BEATS** (Neural Basis Expansion Analysis): Stack-based encoder-decoder architecture.

- Encoder: Multiple stacks of fully connected layers; each stack produces basis vectors and coefficients
- Basis expansion: Synthesizes forecast from learned basis functions
- Residual connections: Each stack models residuals from previous stack

Advantages: No hand-engineered features, learns from raw sequences, strong empirical results. Disadvantages: Black box, data hungry, slow inference for very long sequences.

**N-HiTS** (Neural Hierarchical Interpolation for Time Series): Improves scalability via hierarchical structure.

- Multi-rate hierarchy: Top level models at coarse granularity; lower levels at finer resolution
- Interpolation: Upsampling projects coarse predictions to fine levels
- Faster, more interpretable than N-BEATS; handles very long sequences

Current state (2026): Neural models outperform classical methods on large datasets; classical models (Prophet, ARIMA) still dominate in practice for explainability and data efficiency.

## Practical Considerations

**Pre-processing**:
1. Handle missing values (forward-fill for short gaps, interpolation, or model separately)
2. Remove known outliers (logging infrastructure failures, code bugs)
3. Align time zones (UTC preferred)

**Validation strategy**:
- **Walk-forward**: Train on years 1-2, test on year 3; slide window forward (prevents data leakage)
- Avoid random train-test splits (time series have auto-correlation)
- Stratified folds by season when using e.g., cross-validation for hyperparameter tuning

**Deployment**:
- Refit models regularly (weekly/monthly) on growing historical data
- Monitor prediction errors; alert on out-of-distribution shifts
- Maintain baseline (naive forecast, previous year same-day) for comparison

## See Also
- [statistics-hypothesis-testing.md](statistics-hypothesis-testing.md) — Statistical foundations for time series tests
- [ml-operations.md](ml-operations.md) — Monitoring and retraining in production
- [database-timeseries.md](database-timeseries.md) — Storage and query patterns for time series data