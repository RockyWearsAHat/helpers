# Clustering — Partitioning, Density, Hierarchical & Evaluation

Clustering partitions data into groups (clusters) where instances within a group are similar to each other and dissimilar to instances in other groups. The notion of "similarity" is defined by a distance metric. Clustering is unsupervised: there are no labels; the algorithm infers group structure from data alone.

## K-Means: Centroid-Based Partitioning

K-means partitions data into $k$ non-overlapping clusters by minimizing the **within-cluster sum of squared distances** (WCSS): $\sum_{i=1}^{k} \sum_{x \in C_i} \|x - \mu_i\|^2$, where $\mu_i$ is the centroid (mean) of cluster $i$.

The algorithm alternates between two steps: **(1) assign each point to the nearest centroid**, and **(2) update centroids as the mean of assigned points**. This is guaranteed to converge (WCSS decreases or stays constant on each iteration), but convergence is to a **local minimum**, not the global optimum.

### Initialization Matters
The choice of initial centroids critically affects final clustering. Poor initialization can trap the algorithm in a bad local minimum with high WCSS. Common strategies:

- **Random initialization**: Pick $k$ random points as centroids. Fast but unreliable; often requires multiple runs.
- **K-means++**: Pick the first centroid randomly, then iteratively pick each subsequent centroid with probability proportional to the squared distance to the nearest existing centroid. This spreads centroids apart and typically reaches better minima with fewer iterations.
- **Provide domain knowledge**: If domain experts can suggest initial centroids, seeding with those often works well.

Best practice: Run k-means with k-means++ initialization multiple times (5–10 runs) and select the clustering with the lowest WCSS.

### The Elbow Method & Choosing k
Since k-means requires specifying $k$ upfront, practitioners often fit models for $k = 1, 2, \ldots, k_{\max}$ and plot WCSS vs. $k$. WCSS always decreases as $k$ increases (more clusters, less within-cluster distance). The **(elbow point)** is where the curve flattens—where additional clusters contribute diminishing WCSS reduction. The "elbow" is subjective and may not always be clear, especially in high dimensions or on datasets with no natural clusters.

Alternative methods: silhouette score (see Evaluation), gap statistic (compares within-cluster variance to expected variance under a null distribution), or domain knowledge.

### Limitations
K-means assumes clusters are **convex, similarly-sized, and spherical** in the feature space. It struggles with elongated clusters, clusters with very different sizes, or non-convex shapes. It also requires a distance metric (Euclidean is default); choice of metric can dramatically affect results.

## DBSCAN: Density-Based Clustering

DBSCAN (Density-Based Spatial Clustering of Applications with Noise) groups points based on **local density**. A point is a **core point** if its $\epsilon$-neighborhood (all points within distance $\epsilon$) contains at least `minPts` points. A **border point** is close to a core point but is not core itself. **Noise points** are neither core nor border.

The algorithm: (1) mark all core points, (2) form a cluster by connecting core points that are within $\epsilon$ of each other (transitive closure), and (3) assign border points to their closest core point's cluster.

### Parameter Selection
DBSCAN has two hyperparameters:

- **$\epsilon$ (eps)**: The neighborhood radius. Too small and most points are noise; too large and the algorithm clusters everything together. A common heuristic: compute the k-distance graph (the distance to the kth nearest neighbor for each point) for $k = \text{minPts}$, sort it, and look for an elbow. The eps value at the elbow is often reasonable.
- **minPts**: Minimum points in a neighborhood to form a cluster. Common default is $2 \times \text{dimensions}$. Smaller minPts is more permissive (more clusters); larger minPts requires denser clusters and labels more points as noise.

### Advantages & Limitations
DBSCAN discovers clusters of **arbitrary shape** and automatically identifies **noise points** (points that don't belong to any cluster). This makes it useful for outlier detection and when cluster shapes are unknown. It doesn't require specifying $k$ upfront.

Limitations: **high dimensionality** breaks DBSCAN because the concept of "distance" becomes less meaningful (curse of dimensionality). All pairwise distances in high dimensions converge to a constant value. DBSCAN also struggles with **varying density clusters**—it uses a fixed $\epsilon$ globally, so clusters of different densities can't all be found with a single parameter choice.

Variants like **HDBSCAN** (hierarchical DBSCAN) address varying density by building a hierarchy of density levels and extracting stable clusters at each level.

## Hierarchical Clustering: Agglomerative and Divisive

Hierarchical clustering constructs a **dendrogram**—a tree where each leaf is a single point and each internal node represents a merge (agglomerative) or split (divisive) of clusters. The dendrogram captures clustering at multiple resolutions: cut the dendrogram at different heights to get different numbers of clusters.

### Agglomerative (Bottom-Up)
Start with $n$ clusters (one per point). Repeatedly merge the two closest clusters until one cluster remains. The definition of "closest" (distance between cluster centroids, edges, or all pairwise distances) is governed by the **linkage criterion**.

- **Single linkage**: Distance between closest points in two clusters. Can create long **chains** where overlapping clusters merge incorrectly (chaining effect).
- **Complete linkage**: Distance between farthest points. Produces compact, tight clusters but can be sensitive to outliers in one cluster pushing it far from another.
- **Average linkage**: Mean distance between all pairs of points across clusters. Balances single and complete linkage; often preferred.
- **Ward linkage**: Merges clusters that minimize the increase in within-cluster variance (similar to k-means objective). Tends to produce evenly-sized clusters.

### Divisive (Top-Down)
Start with all points in one cluster. Recursively split clusters until each point is its own cluster. Divisive methods are less common because splitting decisions are expensive (choosing which cluster to split and how) and errors early in the hierarchy are hard to correct.

### Advantage: Interpretability at Multiple Resolutions
A dendrogram provides a **full hierarchy** of clusterings. Different applications can choose different cutting heights: a biologist might cut to get 10 clusters, a downstream application might cut to get 50. This flexibility is valuable in hierarchical data (species taxonomy, organizational structures, document hierarchies).

### Limitations
Hierarchical clustering is $O(n^2 \log n)$ or $O(n^3)$ depending on implementation, making it slow on large datasets. Agglomerative clustering is also **greedy**: once two clusters are merged, they can't be separated. Early merge mistakes propagate through the hierarchy.

## Spectral Clustering

Spectral clustering uses the **eigenvalues and eigenvectors** of the Laplacian matrix of a similarity graph to embed data into a lower-dimensional space, where simple clustering (e.g., k-means) is applied.

Construct a **similarity matrix** $W$ where $W_{ij}$ measures similarity between points $i$ and $j$ (e.g., $\exp(-\gamma \|x_i - x_j\|^2)$ with RBF kernel). Compute the **degree matrix** $D$ (diagonal matrix where $D_{ii} = \sum_j W_{ij}$) and the **unnormalized Laplacian** $L = D - W$ (or normalized variants $L_{\text{sym}} = D^{-1/2} L D^{-1/2}$).

Compute the $k$ smallest eigenvalues of $L$ and stack their corresponding eigenvectors as columns to form a $n \times k$ embedding matrix. Apply k-means to the rows (embedded points) to get $k$ clusters.

**Why it works**: The eigenvectors corresponding to small eigenvalues capture the cluster structure implicitly encoded in the similarity graph. Non-convex, intricately-shaped clusters can be separated in this spectral space where k-means succeeds.

### Limitations
Spectral clustering requires computing the full eigendecomposition ($O(n^3)$) and the similarity matrix ($O(n^2)$ space and computation), making it slow on large datasets. It's effective on medium-sized datasets with non-convex clusters where simpler methods fail.

## Gaussian Mixture Models (GMM)

GMM models the data as arising from a mixture of $k$ Gaussian distributions: $p(x) = \sum_{i=1}^{k} \pi_i \mathcal{N}(x; \mu_i, \Sigma_i)$, where $\pi_i$ is the mixture weight (probability of cluster $i$), $\mu_i$ is the mean, and $\Sigma_i$ is the covariance matrix.

The **EM (Expectation-Maximization) algorithm** fits GMM by alternating: (1) E-step: assign points to clusters with responsibilities proportional to $\pi_i \mathcal{N}(x; \mu_i, \Sigma_i)$, and (2) M-step: update $\pi_i$, $\mu_i$, $\Sigma_i$ using the current responsibilities.

### Probabilistic Interpretation
Unlike k-means (hard assignment), GMM produces **soft assignments**: each point has a probability of belonging to each cluster. This is useful for uncertainty quantification. The EM algorithm maximizes the **log-likelihood** of data under the model, providing a principled probabilistic interpretation.

### Covariance Structure
The covariance matrix $\Sigma_i$ can be full (separate covariance per cluster, many parameters, prone to overfitting), diagonal (conditional independence within clusters), or spherical (equal variance in all directions—equivalent to k-means with probabilistic interpretation). More flexible covariance structures fit data better but require more parameters and larger datasets.

### Limitations
EM is sensitive to initialization. Like k-means, it can converge to local maxima. Covariance fitting is unstable on small samples or high dimensions. The number of Gaussian components must be chosen; techniques like BIC or AIC can help, but they require fitting multiple models.

## HDBSCAN: Hierarchical Density-Based Clustering

HDBSCAN extends DBSCAN by computing a dendrogram of density levels. It builds a **minimum spanning tree** of points using distances as edge weights, then progressively relaxes the edge weights to form a hierarchy of density-connected components. Stable clusters are extracted using a stability criterion.

HDBSCAN automatically discovers the number of clusters (from hierarchy structure) and handles varying-density clusters (fixed at a single eps level). It's more robust than DBSCAN but computationally more expensive and has more moving parts (tuning the stability threshold, minimum cluster size).

## Evaluation Metrics

**Silhouette Score**: For each point, compute $s_i = \frac{b_i - a_i}{\max(a_i, b_i)}$, where $a_i$ is the mean intracluster distance (to other points in the same cluster) and $b_i$ is the mean distance to points in the nearest other cluster. $s_i$ ranges from –1 (point is in the wrong cluster) to +1 (point is well-separated from other clusters). The silhouette score is the mean over all points. Higher values indicate better clustering.

**Davies-Bouldin Index (DBI)**: $\text{DBI} = \frac{1}{k} \sum_{i=1}^{k} \max_{j \neq i} \frac{\sigma_i + \sigma_j}{d(\mu_i, \mu_j)}$, where $\sigma_i$ is the average distance from points in cluster $i$ to the cluster center, and $d(\mu_i, \mu_j)$ is the distance between cluster centers. Lower DBI indicates better clustering (compact, well-separated). Unlike silhouette, DBI doesn't require pairwise distances between all points, making it faster on large datasets.

**Calinski-Harabasz Index**: Ratio of between-cluster variance to within-cluster variance. Higher values indicate more compact, well-separated clusters. Fast to compute but assumes roughly equal cluster sizes and spherical shapes.

**These metrics are internal**, assessing clustering structure without ground truth labels. They can be misleading if cluster shapes don't match the metric's assumptions (e.g., Calinski-Harabasz favors spherical clusters). **External metrics** (purity, normalized mutual information, Adjusted Rand Index) require ground truth labels and assess how well the clustering matches known structure.

## Choosing an Algorithm

- **K-means**: Fast, simple, interpretable. Use when clusters are roughly spherical and similarly-sized, and $k$ is known or can be tuned via elbow/silhouette.
- **DBSCAN**: Non-convex clusters, automatic number-of-clusters detection, outlier handling. Use for spatial data or when cluster shapes are irregular.
- **Hierarchical clustering**: Interpretable multi-resolution clustering. Use on small-to-medium datasets where a dendrogram provides value.
- **Spectral clustering**: Non-convex, intricate cluster shapes. Use when simpler methods fail and $n < 10000$ (computational cost).
- **GMM**: Probabilistic interpretation, soft assignments, likelihood-based model selection. Use when probabilistic interpretation and uncertainty quantification matter.
- **HDBSCAN**: Varying-density clusters, automatic $k$, noise detection. Use as a more robust alternative to DBSCAN on robust density-varying data.