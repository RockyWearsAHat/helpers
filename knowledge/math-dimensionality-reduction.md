# Dimensionality Reduction — PCA, t-SNE, UMAP, Autoencoders & Projections

Dimensionality reduction techniques transform high-dimensional data into lower-dimensional representations while preserving meaningful structure. This reduces computational cost, mitigates the curse of dimensionality, aids visualization, and can improve downstream model performance.

## Conceptual Framework

A $n \times p$ dataset has $n$ samples and $p$ features. Dimensionality reduction seeks a low-dimensional embedding (typically $d \ll p$) that captures the most important variation or structure in the data.

**Trade-offs**:
- **Information loss**: Lower $d$ = smaller model, faster computation, but more information discarded.
- **Interpretability**: Linear reductions (PCA) have transparent mathematical structure; non-linear (t-SNE, UMAP) are harder to interpret but can reveal complex cluster structure.
- **Computational cost**: PCA scales well; t-SNE is O(n²) without approximations; UMAP and autoencoders are moderate.

## Principal Component Analysis (PCA)

PCA finds orthogonal directions (principal components) along which data varies most.

### Mathematical Framework

Given centered data $\mathbf{X}$ ($n \times p$), compute the covariance matrix:

$$\mathbf{C} = \frac{1}{n-1} \mathbf{X}^T \mathbf{X}$$

Eigendecomposition yields eigenvalues $\lambda_1 \geq \lambda_2 \geq \cdots \geq \lambda_p$ and orthonormal eigenvectors $\mathbf{u}_1, \ldots, \mathbf{u}_p$.

The $k$-th principal component is the direction $\mathbf{u}_k$ along which data has variance $\lambda_k$.

**Projection**: Project data onto the first $d$ components:

$$\mathbf{Z} = \mathbf{X} \begin{pmatrix} \mathbf{u}_1 & \cdots & \mathbf{u}_d \end{pmatrix}$$

Result: $\mathbf{Z}$ is $n \times d$, with maximum variance preserved.

### Explained Variance

Total variance: $\sum_{j=1}^{p} \lambda_j$

Variance retained in $d$ components: $\sum_{j=1}^{d} \lambda_j$

**Explained variance ratio**: $\frac{\sum_{j=1}^{d} \lambda_j}{\sum_{j=1}^{p} \lambda_j}$

In practice, choose $d$ such that 80-95% of variance is retained. Scree plot (eigenvalues vs. component number) reveals the "elbow" where adding more components yields diminishing returns.

### Properties and Limitations

- **Optimal for reconstruction** (under Frobenius norm) among linear projections.
- **Orthogonality**: Principal components are uncorrelated — useful for downstream models that struggle with multicollinearity.
- **Interpretability**: Each component is a linear combination of original features; examine weights to understand what drives each component.
- **Linearity**: Assumes data lies on or near a linear subspace. Fails on manifolds with non-linear structure (e.g., Swiss roll, images of faces).

### Computational Notes

For large $p$, compute SVD directly: $\mathbf{X} = \mathbf{U} \mathbf{\Sigma} \mathbf{V}^T$. The first $d$ principal components are columns of $\mathbf{U}$ scaled by $\mathbf{\Sigma}$.

Standardization matters: PCA is sensitive to scale. Standardize features (zero mean, unit variance) unless differences in scale are meaningful.

## t-Distributed Stochastic Neighbor Embedding (t-SNE)

t-SNE maps high-dimensional data to 2D or 3D for visualization by preserving local neighborhood structure.

### Algorithm Outline

1. **High-dimensional similarities**: For each pair of points $i, j$, compute similarity $p_{ij}$ (typically Gaussian kernel):
   $$p_{ij} = \frac{\exp(-\|x_i - x_j\|^2 / (2\sigma_i^2))}{\sum_{k \neq i} \exp(-\|x_i - x_k\|^2 / (2\sigma_i^2))}$$
   
   Bandwidth $\sigma_i$ is chosen so that local neighborhood has a specified perplexity (entropy-like measure; typically 30-50).

2. **Low-dimensional similarities**: Model similarities in embedding space using Student's $t$-distribution (heavier tail than Gaussian):
   $$q_{ij} = \frac{(1 + \|y_i - y_j\|^2)^{-1}}{\sum_{k \neq i}(1 + \|y_i - y_k\|^2)^{-1}}$$

3. **Gradient descent**: Minimize KL divergence $\sum_{i,j} p_{ij} \log(p_{ij} / q_{ij})$ to learn embedding coordinates $\mathbf{y}$.

### Key Parameters

- **Perplexity**: Roughly interpolates between focusing on local (low perplexity) vs. global (high perplexity) structure. Higher perplexity emphasizes broader patterns; lower focuses on local clusters. Too low → fragmented; too high → homogeneous. 5-50 is typical.
- **Learning rate & iterations**: t-SNE is non-convex; choice affects final quality. Usually requires tuning.

### Properties

- **Crowding problem**: As data gets compressed into 2D, intermediate distances collapse. Nearby points become indistinguishable. Mitigation: use UMAP (see below).
- **Non-deterministic**: Results vary with random initialization and sampling. Not reproducible without fixed seed.
- **Local structure emphasis**: Excels at revealing clusters and local neighborhoods. Global topology can be artifact.
- **Not a generative model**: Cannot embed new points; must recompute entire embedding.

## Uniform Manifold Approximation and Projection (UMAP)

UMAP is a newer technique that often outperforms t-SNE: faster, preserves more global structure, and handles new data.

### Core Idea

Model both high-dimensional and low-dimensional data as uniform manifolds (locally Euclidean). Use fuzzy set intersections and graph theory to construct a representation that preserves this manifold structure.

### Algorithm Sketch

1. **Build k-NN graph** in high dimension; weight edges by distances.
2. **Fuzzy set operations**: Model membership strength in high-dim and low-dim neighborhoods.
3. **Gradient descent**: Minimize difference between high-dim and low-dim fuzzy set intersections.
4. **Result**: Preserves topology better than t-SNE; smoother transitions between regions.

### Key Parameters

- **n_neighbors**: Size of local neighborhood (default 15). Lower preserves more local structure; higher emphasizes global topology.
- **min_dist**: Minimum allowed distance between points (default 0.1). Higher values push clusters apart.
- **metric**: Distance metric (Euclidean, Manhattan, Cosine, etc.).

### Properties

- **Faster**: Scales better than t-SNE; practical for 100k+ samples.
- **Generative**: Can transform new data using learned model (approximately).
- **Tunable**: Parameter choices affect local vs. global trade-off explicitly.
- **Theoretical grounding**: Grounded in manifold learning and topological data analysis.

## Autoencoders

Autoencoders are neural networks that learn low-dimensional representations by training to reconstruct input.

### Architecture

Encoder: $\mathbf{z} = f_{\text{enc}}(\mathbf{x})$ compresses $p$ dimensions to $d$.
Decoder: $\hat{\mathbf{x}} = f_{\text{dec}}(\mathbf{z})$ reconstructs from $d$ back to $p$.

Loss: $\mathcal{L} = \|\mathbf{x} - \hat{\mathbf{x}}\|^2$

### Advantages

- **Non-linear**: Can capture complex, non-linear structure (unlike PCA).
- **Flexible**: Architecture (depth, width, activation functions) can be tailored to data.
- **Embeddings useful downstream**: Learned $\mathbf{z}$ often works well in classification/regression.
- **Generative**: Encoder is a learned map; can apply to new data.

### Considerations

- **Over-parameterization**: Deep autoencoders on small datasets risk memorization without learning meaningful compression.
- **Training**: Requires backpropagation, learning rates, stopping criteria — more hyperparameters than PCA.
- **Interpretability**: Learned features are often harder to interpret than PCA components.
- **Variants**: Variational autoencoders (VAEs) add probabilistic structure; denoising autoencoders add robustness.

## Random Projections and Johnson-Lindenstrauss Lemma

Random projections reduce dimensions via random matrices, surprisingly efficient and theoretically guaranteed.

### Theorem (Johnson-Lindenstrauss)

For any $n$ points in $\mathbb{R}^p$ and $\epsilon > 0$, there exists a mapping to $\mathbb{R}^d$ with $d = O(\log(n)/\epsilon^2)$ such that all pairwise distances are preserved to within factor $(1 \pm \epsilon)$.

**Implication**: Reduce to $d = O(\log n)$ dimensions while approximately preserving geometry.

### Method

Project data onto random matrix $\mathbf{R}$ ($p \times d$) with entries drawn from normal or Rademacher distribution:

$$\mathbf{Z} = \mathbf{X} \mathbf{R}$$

### Properties

- **Extremely fast**: Just a matrix multiplication.
- **Theoretically sound**: JL lemma guarantees distances preserved.
- **Data-independent**: No fitting; same projection works for any data.
- **Practical limitation**: Assumes Euclidean structure and distances; ignores clusters or manifolds.

Used in compressed sensing, approximate nearest neighbor search, and as baseline for more sophisticated methods.

## Feature Selection vs. Extraction

**Feature extraction** (e.g., PCA, autoencoders) creates new features as combinations of originals. Trades interpretability for information retention.

**Feature selection** (e.g., backward elimination, LASSO) retains a subset of original features. Interpretation is easier; may discard information useful for prediction.

Hybrid: Dimensionality reduction followed by sparse linear model (e.g., PCA + LASSO).

## Practical Guidance

- **Visualization**: t-SNE or UMAP (2D/3D), with UMAP preferred for speed and global structure preservation.
- **Pre-processing**: PCA for linear structure, noise reduction. Works well before clustering or classification.
- **Non-linear structure**: UMAP > t-SNE. Autoencoders if you have labeled data and can afford training.
- **Interpretability**: PCA wins (components are linear combinations). UMAP/t-SNE are black boxes.
- **New data**: PCA and autoencoders generalize. t-SNE and UMAP require recomputing or approximations.
- **High-dimensional $p \gg n$**: UMAP, random projections, ICA. Avoid early PCA if you have many redundant features (LASSO first to select).

See also: ml-clustering.md, ml-deep-learning.md, math-linear-algebra.md, machine-learning-fundamentals.md