# Model Evaluation — Classification Metrics, Regression Metrics & Statistical Testing

Evaluating machine learning models accurately is essential: a model that performs well on training data but fails on unseen data is useless in production. Evaluation requires multiple perspectives—different metrics reveal different aspects of performance—and must account for class imbalance, cost asymmetries, and business context.

## Classification Metrics

**Accuracy** is the most intuitive metric: the fraction of predictions that are correct. Accuracy = (TP + TN) / (TP + TN + FP + FN), where TP=true positives, TN=true negatives, FP=false positives, FN=false negatives. Accuracy is misleading with imbalanced datasets: a model predicting all negatives will appear highly accurate if the negative class dominates. For example, if 99% of cases are negative, predicting always-negative achieves 99% accuracy despite making no useful predictions.

**Precision** = TP / (TP + FP) answers: "Of the positive predictions, how many were correct?" Use precision when false positives are costly (spam detection, medical false alarms). High precision means low false alarm rate.

**Recall** = TP / (TP + FN) answers: "Of the actual positives, how many did we find?" Use recall when false negatives are costly (disease detection, fraud prevention). High recall means few missed cases.

**F1 score** = 2 × (precision × recall) / (precision + recall) harmonically balances precision and recall. When you care equally about both, F1 provides a single number. With weighted classes, macro-averaging (unweighted mean across classes) and weighted-averaging (weighted by support) offer different trade-offs.

**Precision-Recall (PR) curves** plot precision vs. recall as the classification threshold varies. Area under the PR curve (PR-AUC) summarizes this trade-off; higher is better. PR curves are preferable to ROC curves with severe class imbalance because they focus on the minority class.

**ROC curves** plot true positive rate (recall) vs. false positive rate (FP / (FP + TN)) across thresholds. Area under the ROC curve (AUC-ROC) ranges from 0.5 (random) to 1.0 (perfect). AUC represents the probability that the classifier ranks a randomly chosen positive example higher than a randomly chosen negative. ROC curves are less sensitive to class imbalance than accuracy but can be misleading with extreme imbalance (PR-AUC is more appropriate).

**Confusion matrices** display the 2×2 (or larger for multi-class) table of predictions vs. actuals, providing a complete view of all error types. This is the foundation for computing other metrics.

## Regression Metrics

**Mean Squared Error (MSE)** = (1/n) Σ(y_i - ŷ_i)² averages squared residuals. Squaring penalizes large errors heavily, so outliers have outsized influence. MSE is in squared units of the target (hard to interpret).

**Root Mean Squared Error (RMSE)** = √MSE returns to original units, making interpretation easier. A model with RMSE=5 predicts within ±5 units on average.

**Mean Absolute Error (MAE)** = (1/n) Σ|y_i - ŷ_i| averages absolute residuals. MAE treats all errors equally (linear scale), so it's less influenced by outliers than MSE. Choose MSE for penalizing large errors, MAE for robustness to outliers.

**R² (coefficient of determination)** = 1 - (SS_res / SS_tot) measures the proportion of variance explained. R² ranges from -∞ to 1: R²=1 is perfect fit, R²=0 means the model predicts as well as a baseline (always predicting the mean), negative R² means the model is worse than a baseline. R² is scale-invariant and interpretable as "percent of variance explained."

## Cross-Validation Strategies

**K-fold cross-validation** partitions data into k folds, trains on k-1 folds, tests on the remaining fold, repeating k times. Final metrics are the average across folds. This uses data more efficiently than a single train-test split and reduces variance in performance estimates. Common values: k=5 or k=10.

**Stratified k-fold** maintains class distribution in each fold, essential for imbalanced datasets. Random splitting can produce highly unbalanced folds with imbalanced data; stratification ensures consistent class ratios across all folds.

**Time-series cross-validation** avoids information leakage in temporal data. Forward-chaining validates on future time periods using only past data (train on weeks 1-4, test on week 5; train on weeks 1-5, test on week 6). Random splitting would use future information to predict the past, producing unrealistically optimistic estimates.

**Leave-one-out cross-validation (LOOCV)** trains n times, each time on n-1 examples, testing on the holdout. LOOCV is computationally expensive (n training iterations for n examples) but provides low-bias estimates. Use only for small datasets.

## Statistical Significance Testing

**Confidence intervals** quantify uncertainty in metrics. A 95% confidence interval for accuracy means: if you repeated the experiment many times, the true accuracy would fall in this interval 95% of the time. Bootstrap resampling estimates confidence intervals without strong distributional assumptions. Standard error of a binary metric (accuracy, precision) with n examples is roughly √(metric × (1 - metric) / n): smaller datasets have larger confidence intervals.

**Hypothesis testing** compares two models. The null hypothesis is that both models have equal performance. Paired t-tests (paired when models evaluated on the same test set) or McNemar's test (for classification) compare models. Significance tests answer: "Is the observed difference likely by chance?" Not significant differences could be noise; significant differences suggest one model is genuinely better.

**Multiple comparison correction** (Bonferroni, Benjamini-Hochberg) is necessary when comparing many models or metrics. Without correction, random noise produces false positives. For k comparisons, Bonferroni divides the significance threshold by k (conservative); Benjamini-Hochberg controls false discovery rate (less conservative).

## Confusion Matrix and Calibration

The confusion matrix is the bedrock of classification evaluation. From it derive precision, recall, specificity (TNR = TN / (TN + FP)), and balanced accuracy. Visualizing the matrix reveals which classes the model confuses and whether errors are symmetric or asymmetric.

**Calibration** measures whether predicted probabilities match true frequencies. A well-calibrated model predicting 70% confidence is correct 70% of the time. Poorly calibrated models can have calibration plots skewing above or below a diagonal line. Isotonic regression or Platt scaling post-processes predictions to improve calibration without retraining.

## Bias-Variance Decomposition

Expected prediction error decomposes as: E[(y - ŷ)²] = bias² + variance + irreducible noise.

**Bias** is how far the average prediction is from the true value. High-bias models (weak learners, high regularization) systematically underfit, making consistent but wrong predictions.

**Variance** is the sensitivity to training data fluctuations. High-variance models (complex, low regularization) overfit, making different predictions with small data changes.

The bias-variance tradeoff governs model selection: simple models have high bias, lower variance; complex models have low bias, higher variance. The sweet spot depends on data quantity and noise.

## Learning Curves

Learning curves plot training and validation metrics vs. training set size. They diagnose underfitting and overfitting:

- **High bias** (underfitting): both curves plateau at poor performance; adding data doesn't help. Solution: increase model complexity.
- **High variance** (overfitting): training performance is high but validation is poor with a large gap; adding more data improves validation performance. Solution: add regularization or collect more data.
- **Healthy**: curves converge at good performance; larger gaps suggest more data would help.

## See Also

- machine-learning-fundamentals.md — supervised learning foundations
- ml-feature-engineering.md — feature construction for models
- ml-operations.md — model evaluation in production