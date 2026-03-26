# Anomaly Detection — Statistical Methods, Unsupervised Learning & Temporal Dynamics

Anomaly detection identifies instances that deviate significantly from normal patterns. "Normal" is defined either by explicit statistical models, learned density models, or domain context. Anomalies are rare by definition, making evaluation and tuning challenging: datasets are heavily imbalanced, and true anomaly rates are often unknown.

## Statistical Methods

### Z-Score and Standardization
The **Z-score** measures how many standard deviations a point is from the mean: $z = \frac{x - \mu}{\sigma}$. Points with $|z| > 3$ are typically flagged as anomalies (assuming normality, these occur ~0.3% of the time). 

This method is simple and fast but assumes data is approximately **normally distributed** and that normal behavior is centered around the mean. Real-world data rarely satisfies this assumption—they're often skewed, multimodal, or have heavy tails. Z-scores also fail on datasets with outliers in the training data (the mean and standard deviation themselves become skewed).

### Interquartile Range (IQR)
The **IQR** is the range between the 25th and 75th percentiles: $\text{IQR} = Q_3 - Q_1$. Points outside the range $[Q_1 - 1.5 \times \text{IQR}, Q_3 + 1.5 \times \text{IQR}]$ are flagged as outliers (Tukey's fences). The 1.5 multiplier is conventional; tighter or looser thresholds adjust sensitivity.

IQR is **robust**: it doesn't assume normality and is less affected by extreme outliers (it uses quantiles, not mean/variance). However, it's univariate—designed for single variables. Applying IQR to each dimension independently and flagging points that are outliers in any dimension can work but misses multivariate anomalies (a point can be normal in each dimension but unusual in combination).

### Grubbs' Test
Grubbs' test detects one outlier at a time from a univariate sample by computing: $G = \frac{\max_i |x_i - \bar{x}|}{s}$, where $\bar{x}$ is the sample mean and $s$ is the sample standard deviation. If $G > g_{\alpha, n}$ (critical value), the most extreme point is flagged as an outlier. The point is removed, and the test is repeated. This is useful for detecting one or a few outliers in a relatively normal distribution but becomes unreliable if multiple outliers are present (they bias the mean and standard deviation, masking themselves).

## Distance-Based Methods

### Isolation Forest
Isolation Forest isolates anomalies by randomly selecting features and split values, building an ensemble of isolation trees. Anomalies are easier to isolate (they require fewer splits to separate from normal points), so they have shorter average path lengths in the trees.

Each tree partitions the space by randomly choosing a feature and a split value until each point is isolated (in its own leaf). Normal points are buried deeper in the tree (farther from the root); anomalies are closer to the root. The **anomaly score** is the average path length across all trees: shorter paths indicate anomalies.

**Advantages**: No distance metric required, works in high dimensions (tree splits on individual features, not distance functions), and explicitly models how anomalies are isolated. Computationally linear in the number of points and logarithmic in dimensions (per tree).

**Limitations**: The random feature selection can mask important features if dimensions are correlated. Anomaly scores are relative (comparative across the dataset), not absolute—a point's anomaly score depends on the full dataset, so retraining on new data can change anomaly scores of old points.

### Local Outlier Factor (LOF)
LOF identifies points whose local density is significantly lower than their $k$-nearest neighbors' densities. A point in a sparse region (low local density) is an outlier; a point in a dense region surrounded by other dense points is normal.

**Computation**: For each point $p$, compute the **local reachability distance** to its $k$-nearest neighbors, capturing how far $p$ is from the $k$-neighborhood's center. The **LOF score** is the ratio of the average local reachability distance of $k$-neighbors to $p$'s own local reachability distance. LOF $\approx 1$ means $p$ is as dense as its neighbors (normal); LOF $\gg 1$ means $p$ is in a sparser region than its neighbors (anomaly).

**Advantages**: Captures **intrinsic dimensionality** of the data—detects anomalies relative to local density, not global density. A point far from the global mean but in a sparse region is still flagged as anomalous. Works in any metric space (Euclidean, Manhattan, custom distances).

**Limitations**: LOF depends on $k$ (choice of neighborhood size); small $k$ is noise-sensitive, large $k$ can miss local anomalies. Requires computing pairwise distances ($O(n^2)$), expensive on large datasets. LOF scores are relative, not absolute (retraining changes scores).

### One-Class SVM
One-Class SVM learns a decision boundary enclosing the majority of training data in high-dimensional space. It maximizes the margin between the decision boundary and the origin (in kernel space), effectively fitting a minimal enclosing sphere or hyperplane.

**How it works**: Map data into kernel space (using an RBF kernel is common), find the minimal sphere that contains most data (controlled by the $\nu$ parameter, roughly the expected fraction of anomalies), and assign points inside the sphere as normal and outside as anomalies.

**Advantages**: Assumes no probability distribution, handles high dimensions naturally (kernel methods), and has principled optimization (SVM objective). Tunable sensitivity via $\nu$.

**Limitations**: Hyperparameter sensitivity ($\nu$, kernel choice, kernel parameters like RBF's $\gamma$). Requires tuning the kernel; wrong choice degrades performance dramatically. Scales poorly on large datasets ($O(n^2)$ to $O(n^3)$ in training time). Relative scoring (relative to training data boundary).

## Deep Learning Approaches

### Autoencoders for Anomaly Detection
An autoencoder is a neural network that compresses input $x$ into a low-dimensional latent code $z$ (encoder), then reconstructs $x$ from $z$ (decoder). Training minimizes reconstruction error. The hypothesis: normal points have low reconstruction error; anomalies have high reconstruction error (they don't fit the learned pattern).

**Advantages**: Learns data manifold implicitly, handles complex non-linear patterns, and integrates with deep learning pipelines. Can model subtle patterns in high-dimensional data (images, sequences).

**Limitations**: Depends on network architecture (depth, width, latent dimension) and training hyperparameters. Autoencoders trained too long or on imbalanced data can overfit the normal class. Requires careful validation and threshold selection to avoid false positives on near-normal instances.

### Variational Autoencoders (VAE)
VAEs extend autoencoders by learning a **latent distribution** $p(z)$. During training, the encoder outputs mean and variance of a Gaussian, and samples are drawn from this distribution. The decoder reconstructs from samples. Loss includes reconstruction error and a **KL divergence term** that pushes the latent distribution toward the prior.

VAEs provide a probabilistic interpretation: points with low likelihood under the learned distribution are anomalies. Anomaly score is $-\log p(x)$ (negative log-likelihood). VAEs are more interpretable than autoencoders (explicit density model) but require tuning the KL weight (balancing reconstruction fidelity and regularity of the latent space).

## Time Series Anomaly Detection

Time series anomalies have temporal structure. An anomaly may be a **point anomaly** (single point deviates from trend), a **collective anomaly** (a subsequence deviates), or a **contextual anomaly** (point is unusual given its temporal context, even if individually normal).

### Seasonal Decomposition
Decompose the series into **trend**, **seasonality**, and **residuals**: $y_t = \text{trend}_t + \text{seasonal}_t + \text{residual}_t$. Anomalies are detected in the residuals (after removing known patterns) using statistical tests (Z-score on residuals) or isolation forests.

**Advantage**: Removes expected variation (seasonality) before anomaly detection, reducing false positives on seasonal spikes. **Limitation**: Decomposition assumes stationarity and additive/multiplicative structure; it fails on irregular time series or series with multiple periods.

### LSTM Autoencoders
An **LSTM autoencoder** encodes a time series window into a latent representation and reconstructs the window. Anomalies are windows with high reconstruction error. LSTMs capture temporal dependencies better than feedforward autoencoders, so they model normal patterns more accurately.

**Limitation**: High false-positive rate on near-normal data; threshold selection is critical. Requires stationary data or explicit trend removal (LSTMs are trained on differences, not absolute values, in non-stationary series).

### Change Point Detection
Detect points where the underlying distribution changes (concept drift). Methods include **CUSUM** (cumulative sum control chart—tracks cumulative deviations from expected value), **kernel-based change point detection** (compares distributions across windows using maximum mean discrepancy), and **Bayesian methods** (infer posterior probability of change points).

Change point detection is distinct from anomaly detection but related: a change point indicates a shift in normal behavior (the new regime is not anomalous, but the transition is). Context matters: a market regime shift from trending to mean-reverting is a change point (not an anomaly), but detecting it is valuable for time series forecasting and risk management.

## Concept Drift and Deployment Patterns

**Concept drift** occurs when the distribution of normal data changes over time. A model trained on 2020 data may flag 2024 behavior as anomalous even if it's actually normal (the baseline has shifted).

### Drift Detection Strategies
- **Retraining schedule**: Periodically retrain the model on recent data to adapt to the current normal.
- **Unsupervised drift detection**: Track anomaly rate or reconstruction error distribution over time. If both shift, retrain.
- **Feedback loops**: Collect predictions on new data, request labels for uncertain cases, and use labeled feedback to retrain.

The choice between retraining intervals (rigid, predictable) and drift-triggered retraining (reactive, data-driven) depends on the cost of retraining vs. the cost of stale models.

### Online Anomaly Detection
In streaming settings, we can't store all historical data. **Mini-batch methods** (Isolation Forest, LOF) refit on sliding windows. **Incremental methods** update model parameters without storing past data (examples: incremental PCA for dimensionality reduction, online mean/covariance for statistical methods).

Trade-off: incremental methods are memory-efficient but may be less accurate than batch methods. Typically, a compromise: update on batches of recent data (hours or days, depending on stream rate) rather than every point.

## Threshold Selection & Tuning

Anomaly detection models output **scores** (e.g., reconstruction error, LOF value, anomaly score), not binary labels. A **threshold** classifies scores above it as anomalies. Threshold selection is critical and domain-dependent:

- **Fixed threshold**: Use a percentile (e.g., top 5% of scores are anomalies) if the anomaly rate is known. Requires domain knowledge or historical data on anomaly frequency.
- **Threshold tuning**: Use a labeled validation set (if available) to select a threshold that maximizes precision-recall trade-off or F1 score. Be careful: if labeled data is scarce, the selected threshold may not generalize.
- **Dynamic thresholds**: Adjust per feature group (threshold is higher in high-variance features) or per time period (threshold adapts as normal behavior shifts).

Metrics for anomaly detection with class imbalance: **F1 score** (balances precision and recall), **AUROC** (area under receiver operating curve—threshold-agnostic), **precision-recall AUC** (better for imbalanced data). Accuracy is misleading when most data is normal.

## When to Use Each Method

- **Statistical methods (Z-score, IQR)**: Single-variable anomalies, fast screening, explainability required.
- **Isolation Forest**: High-dimensional data, interpretability needed, large datasets.
- **Local Outlier Factor**: Local density anomalies (sparse regions), no geometric shape assumptions.
- **One-Class SVM**: Spherical or kernel-shaped normal regions, moderate dataset sizes.
- **Autoencoders/VAEs**: Complex patterns, images or sequences, deep learning infrastructure available.
- **LSTM/RNN methods**: Time series with temporal dependencies.
- **Change point detection**: Detect distribution shifts, not point-level anomalies.