# Recommendation Systems — Collaborative Filtering, Matrix Factorization & Hybrid Approaches

Recommendation systems power personalization at scale: Netflix suggesting movies, Amazon recommending products, Spotify curating playlists. These systems infer user preferences from historical behavior and similarities between users or items, then predict what users will like.

## Collaborative Filtering

Collaborative filtering assumes that users with similar past preferences will like similar future items. The system needs no content descriptions—only the user-item interaction matrix (ratings, clicks, purchases).

**User-based collaborative filtering** finds similar users (via cosine similarity or Pearson correlation in rating history), then recommends items liked by those similar users but not yet rated by the target user. Simple and interpretable: "Users like you also liked X." Weakness: requires substantial user history and doesn't scale well (computing pairwise similarity across millions of users is expensive).

**Item-based collaborative filtering** is analogous: find items similar to those the user rated highly, recommend those. Item-item similarity is typically more stable than user-user similarity (items don't change much, but user preferences can shift dramatically). Scales better horizontally.

## Matrix Factorization

The user-item interaction matrix R (m users × n items) is typically sparse: each user interacts with only a small fraction of items. **Matrix factorization** decomposes R ≈ U × V^T, where U is an m × k user-latent matrix and V is an n × k item-latent matrix. Each latent dimension captures some hidden preference pattern or item characteristic. The factorization recovers missing entries by treating unobserved interactions as predictions.

**Singular Value Decomposition (SVD)** is a classical factorization: mathematically elegant, but requires a dense matrix (handles missing values indirectly). **Funk SVD** (gradient descent on factorization) addresses sparsity directly: optimize only observed interactions, treating missing values as absent rather than zero. **Alternating Least Squares (ALS)** alternates fixing U and solving for V, then vice versa, scaling well to large sparse matrices.

**Non-negative Matrix Factorization (NMF)** constrains U and V to be non-negative, improving interpretability: latent factors represent meaningful characteristics. This comes at a cost: slightly lower accuracy but more explainable recommendations.

Factorization captures both global patterns and user-specific preferences. A user might have high weights on latent dimensions representing "action-movie enthusiast" and "independent-film appreciation," allowing nuanced recommendations.

## Content-Based Filtering

When user-item interactions are sparse, content-based approaches use item features: product descriptions, categories, genres, or learned embeddings. The system builds a user profile (aggregating features of items they liked) and recommends items similar to that profile (cosine similarity in feature space). No user-user or item-item similarity computation needed, but requires good feature engineering and misses serendipitous discoveries outside user's known interests.

## Hybrid Approaches

Pure collaborative filtering fails for new items or users (the **cold start problem**). Pure content-based approaches miss personalized discovery. Hybrid systems combine both:

- Weighted ensemble: blend collaborative scores and content-based scores with learned weights
- Feature augmentation: use content features alongside latent factors in matrix factorization
- Cascade: use content-based filtering to narrow candidates, then rank via collaborative filtering
- Ensemble of independent models: train separate models, combine predictions

Different items merit different approaches: popular items benefit from collaborative filtering; obscure items need content features.

## Cold Start Problem

**New user cold start**: the user has no interaction history; collaborative filtering has nothing to learn. Solutions: use content-based filtering initially, collect initial preferences through survey or onboarding flow (explicit feedback), infer from demographic data (age, location, language), or recommend popular items (exploit the "wisdom of crowds").

**New item cold start**: the item has no interactions yet; collaborative filtering can't rank it. Solutions: use content similarity to existing items, exploit metadata (category, tags), or allocate exploration budget (sometimes recommend items with less historical evidence to discover new hits).

**Explore-exploit tradeoff**: pure exploitation recommends within known preferences; pure exploration tries random items. Multi-armed bandit approaches balance these: Thompson sampling, upper confidence bound (UCB) algorithms.

## Implicit vs. Explicit Feedback

**Explicit feedback** is direct: star ratings (1-5), thumbs up/down, yes/no surveys. Explicit feedback is honest and information-rich but sparse (users rarely rate everything) and biased (people tend to rate extreme experiences).

**Implicit feedback** infers preferences from behavior: click, view, purchase, position, dwell time. Implicit feedback is abundant and unbiased (users don't deliberately game their clicking behavior) but noisier: a click could indicate interest or could be a mistake. Implicit systems often use confidence weighting: purchases are higher confidence than clicks; longer dwell time indicates higher confidence.

Bayesian Personalized Ranking (BPR) is a popular implicit feedback algorithm: it learns to rank items a user interacted with higher than items they didn't, modeling the ranking preference rather than explicit scores.

## Deep Learning Recommenders

Neural networks have expanded the expressiveness of recommendation systems beyond matrix factorization.

**Two-tower neural networks** embed users and items separately via deep networks, then compute dot product or cosine similarity. This scales to billions of users and items: inference is fast (precompute item embeddings, dot product search with user embedding). Layers capture complex non-linear patterns.

**Neural Collaborative Filtering (NCF)** replaces matrix factorization's bilinear interaction with learned non-linear interactions via fully connected layers. The first layers produce user and item embeddings; subsequent layers learn a neural MLP to model the interaction function.

**Sequence models** (RNNs, Transformers) capture temporal dynamics: the next item recommended depends on the sequence of past interactions, not just aggregate preferences. Attention mechanisms allow models to weight which past interactions matter most.

**Representation learning** from embeddings (user/item vectors) enables transfer learning and cold-start mitigation through side information (text descriptions, images) and auxiliary tasks.

## Evaluation Metrics

**NDCG (Normalized Discounted Cumulative Gain)** = (DCG_p) / (IDCG_p) measures ranking quality in top-k recommendations. DCG discounts lower-ranked items, emphasizing position, and rewards relevant items. Perfect ranking has NDCG=1. Suitable for ranking problems.

**MAP (Mean Average Precision)** = average precision across top-k items, where precision is computed at each relevant item position. MAP@10 focuses on the top 10.

**Recall@k** = fraction of relevant items in top-k recommendations. High recall means finding diverse relevant items; common target metric for discovery.

**Precision@k** = fraction of top-k recommendations that are relevant. For ads or curated lists, precision matters (show only items users will like).

**HR (Hit Rate)** = fraction of test interactions for which the system recommends the item in top-k. Binary indication: did the system get it?

## A/B Testing Recommendations

Online metrics diverge from offline metrics because recommendation quality depends on context and long-term effects. A/B tests run different algorithms for different user cohorts, measuring:

- Click-through rate (CTR): fraction of recommendations clicked
- Conversion rate: fraction leading to purchase or other goal
- Serendipity: fraction of recommendations outside user's norm (discovery metric)
- Long-term diversity: does recommendation history become narrow?

Recommendation algorithms can create filter bubbles (reinforcing existing preferences) or foster exploration. A/B tests reveal which trade-off users prefer.

## See Also

- machine-learning-fundamentals.md — supervised learning foundations
- ml-model-evaluation.md — recommender evaluation metrics
- ml-operations.md — deploying recommenders at scale