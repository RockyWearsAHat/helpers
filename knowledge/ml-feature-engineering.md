# Feature Engineering — Numerical Transforms, Encoding, Text Features & Feature Stores

Feature engineering—the process of constructing input features from raw data—remains one of the highest-impact activities in machine learning. The quality of features often determines model performance more than the choice of algorithm. Feature engineering bridges raw data and model capability: transforming unstructured data into meaningful signals that algorithms can learn from effectively.

## Numerical Transformations

**Scaling and normalization** adjust the magnitude of numerical features. Standardization (z-score normalization) subtracts the mean and divides by standard deviation, producing features with mean 0 and standard deviation 1. This is crucial for algorithms sensitive to feature magnitude (linear regression, logistic regression, SVMs, neural networks). Min-max normalization (scaling to [0,1]) is useful when features have meaningful bounds or when you need output in a known range.

**Log transforms** handle skewed distributions and right-tailed data common in financial and natural phenomena. Taking the logarithm compresses large values while preserving relative ordering, making skewed distributions more normal-like. This stabilizes variance and can improve model training.

**Binning and discretization** convert continuous variables into categorical ones. Equal-width binning divides the range into fixed-size intervals; equal-frequency (quantile) binning ensures balanced class distribution across bins. Binning loses information but can capture non-linear relationships and reduce noise from outliers. It's particularly useful before tree-based models or when domain knowledge suggests breakpoints.

**Polynomial features** introduce non-linearity for linear models. Creating x², x³, or interaction terms (x₁·x₂) allows linear models to fit curved decision boundaries. This increases model capacity at the cost of interpretability and computational cost. Tree-based models create non-linear boundaries implicitly, reducing the need for explicit polynomial features.

## Categorical Encoding

**One-hot encoding** transforms categorical variables into binary vectors. For a feature with k categories, create k binary columns where each row has exactly one 1. This works well with linear models and neural networks but creates sparsity and high dimensionality with high-cardinality features (many unique values).

**Ordinal encoding** assigns integers to categories (good → 3, fair → 2, poor → 1). This preserves order for ordinal features but risks introducing false ordinal relationships for nominal categories. Use only when categories have a natural ordering.

**Target encoding** (mean encoding) replaces each category with the mean target value for that category. For classification, this is the proportion of positives; for regression, the mean outcome value. Target encoding captures the relationship between a category and the target directly, often improving model performance. Risk of overfitting (especially with rare categories) can be mitigated through regularization (adding global mean with weight proportional to category frequency).

**Hashing** (feature hashing) maps categories to hash buckets, useful for high-cardinality features or streaming scenarios where new categories appear after training. Hash collisions introduce a small approximation error but allow fixed-size representations.

## Text Features

**TF-IDF** (term frequency–inverse document frequency) quantifies word importance in a document relative to a corpus. TF counts term occurrences in a document; IDF downweights common terms (the, a, is) that appear in many documents. The product gives high scores to rare, informative terms. Results are sparse, high-dimensional vectors suitable for linear models and distance-based algorithms.

**Word embeddings** (Word2Vec, GloVe, FastText) represent words as dense vectors (typically 50-300 dimensions) learned from large corpora. Words with similar meanings cluster nearby in vector space. Embeddings capture semantic relationships and work well with neural networks. Word2Vec uses skip-gram or CBOW objectives; GloVe combines matrix factorization with local context windows; FastText extends Word2Vec by learning subword representations, handling out-of-vocabulary words better.

**Contextualized embeddings** (BERT, ELMo) generate token representations that vary based on surrounding context. A word like "bank" gets different embeddings in "river bank" vs "savings bank." These capture word sense disambiguation and are typically extracted from pretrained transformer models, then used as frozen features or fine-tuned for specific tasks.

**N-grams** capture local word order. Using unigrams (single words), bigrams (word pairs), and trigrams provides more context than unigrams alone. TF-IDF is computed over n-gram vocabularies, trading dimensionality for contextual information.

## Temporal Features

Time-series and timestamp data requires careful feature engineering. **Lag features** (previous values: y_{t-1}, y_{t-2}) capture temporal dependencies for autoregressive models. **Rolling statistics** (mean, std over the last k periods) summarize recent history. **Seasonal decomposition** extracts trend, seasonal, and residual components. **Time of day, day of week, month, year** encode periodic patterns. **Holiday/event indicators** capture sudden changes in behavior. Business logic often informs which temporal features matter (retail sales spike on Fridays; electricity demand follows 24-hour cycles).

## Feature Crosses and Interactions

Multiplicative interactions (feature1 × feature2) combine signals non-linearly. A classic example: user's age × income predicts purchasing power better than either alone. Tree-based models find interactions automatically; linear models and neural networks often benefit from explicit interaction features.

## Feature Stores

Feature stores centralize feature definition, computation, and serving. **Feast** (open-source, Tecton) and similar systems address the training-serving skew problem: features computed one way during development often differ from production serving. A feature store maintains a single authoritative definition, computes features in batch (for historical data) and real-time (for serving), and versioning enables reproducibility.

Feast provides a registry of feature definitions, materializes features to offline (data warehouse) and online (Redis, DynamoDB) stores, and serves features at prediction time with minimal latency. This prevents unintended feature information leakage and reduces engineering toil.

## Automated Feature Engineering

Tools like AutoML and AutoFE systems generate features algorithmically: polynomial extensions, interactions, domain-specific transforms. These reduce manual effort but may sacrifice interpretability and sometimes produce noisy, low-value features. Human-in-the-loop approaches combine automated generation with domain expertise to filter and understand discovered features.

## Feature Importance and Selection

**Filter methods** rank features by correlation with target before training (removes redundancy, reduces noise). **Wrapper methods** iteratively train models and select features (computationally expensive). **Embedded methods** rely on model-internal feature importance (tree importance, linear coefficients). SHAP and LIME provide post-hoc explanations of how features contribute to predictions.

Feature selection reduces dimensionality, improves interpretability, reduces training time, and sometimes improves generalization by removing noise. However, it risks discarding features that interact only with other features (univariate ranking misses this).

## Trade-offs and Antipatterns

Aggressive feature engineering increases model complexity and overfitting risk, especially with small datasets. Correlated features provide redundant information. Leaking information from the target or future data into features produces models that fail in production. Careful validation (cross-validation, holdout test sets) and domain understanding mitigate these risks.

## See Also

- machine-learning-fundamentals.md — supervised learning foundations
- ml-model-evaluation.md — assessing model performance
- ml-operations.md — feature management in production