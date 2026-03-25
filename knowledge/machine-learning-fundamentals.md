# Machine Learning Fundamentals

## Core Paradigms

### Supervised Learning

Learning from labeled examples. Given input-output pairs $(x_i, y_i)$, find a function $f$ that maps inputs to outputs.

- **Classification**: discrete outputs (spam/not-spam, digit recognition)
- **Regression**: continuous outputs (price prediction, temperature forecasting)
- Key assumption: training distribution ≈ deployment distribution (IID assumption)

### Unsupervised Learning

Finding structure in unlabeled data.

- **Clustering**: K-means, DBSCAN, hierarchical — group similar items
- **Dimensionality reduction**: PCA, t-SNE, UMAP — compress high-dimensional data
- **Density estimation**: model the probability distribution of data
- **Anomaly detection**: find points that don't fit the learned distribution

### Reinforcement Learning

Agent learns by interacting with an environment, maximizing cumulative reward.

- State → Action → Reward → New State loop
- Exploration vs exploitation tradeoff
- Policy: mapping from states to actions
- Value function: expected cumulative reward from a state
- Q-learning, policy gradients, actor-critic methods

### Self-Supervised Learning

Creates supervision signals from the data itself. The dominant paradigm for modern LLMs.

- Masked language modeling (BERT): predict masked tokens
- Next-token prediction (GPT): predict the next token autoregressively
- Contrastive learning (SimCLR, CLIP): learn representations by contrasting positive/negative pairs

## The Learning Process

### Loss Functions

Measure how wrong the model's predictions are. Training minimizes this.

| Loss Function            | Use Case              | Formula                                         |
| ------------------------ | --------------------- | ----------------------------------------------- |
| Mean Squared Error (MSE) | Regression            | $\frac{1}{n}\sum(y_i - \hat{y}_i)^2$            |
| Cross-Entropy            | Classification        | $-\sum y_i \log(\hat{y}_i)$                     |
| Binary Cross-Entropy     | Binary classification | $-[y\log(\hat{y}) + (1-y)\log(1-\hat{y})]$      |
| Huber Loss               | Robust regression     | Quadratic near 0, linear far away               |
| Contrastive Loss         | Embedding learning    | Pull similar pairs close, push dissimilar apart |

### Gradient Descent

The optimization algorithm that adjusts model parameters to minimize loss.

```
repeat:
    predictions = model(inputs)
    loss = loss_function(predictions, targets)
    gradients = compute_gradients(loss, parameters)    # backpropagation
    parameters = parameters - learning_rate * gradients
```

**Variants:**

- **Batch GD**: use entire dataset per update — slow, smooth convergence
- **Stochastic GD (SGD)**: one sample per update — noisy but fast
- **Mini-batch GD**: compromise — use batches of 32-512 samples (standard practice)
- **Adam**: adaptive learning rates per parameter + momentum — the default choice for most tasks
- **AdamW**: Adam with decoupled weight decay — preferred for transformer training

### Backpropagation

The algorithm that efficiently computes gradients through the computation graph using the chain rule.

$$\frac{\partial L}{\partial w_i} = \frac{\partial L}{\partial a_n} \cdot \frac{\partial a_n}{\partial a_{n-1}} \cdots \frac{\partial a_{i+1}}{\partial w_i}$$

Key insight: compute gradients backwards from the loss, reusing intermediate results. This is what makes training deep networks tractable.

**Problems:**

- **Vanishing gradients**: gradients shrink exponentially through many layers → fixed by ReLU activations, skip connections, normalization
- **Exploding gradients**: gradients grow exponentially → fixed by gradient clipping, careful initialization

### Regularization

Techniques to prevent overfitting (memorizing training data instead of learning patterns).

- **L2 regularization (weight decay)**: penalize large weights — $L + \lambda \sum w_i^2$
- **L1 regularization**: encourage sparsity — $L + \lambda \sum |w_i|$
- **Dropout**: randomly zero out neurons during training — forces redundancy
- **Early stopping**: stop training when validation loss starts increasing
- **Data augmentation**: artificially expand training set (flip, rotate, crop images; paraphrase text)
- **Batch normalization**: normalize layer outputs — stabilizes training, mild regularization effect

## Neural Network Architectures

### Feedforward Networks (MLPs)

Stacks of fully-connected layers with nonlinear activations. Universal function approximators (in theory). In practice, need architecture choices matched to the data structure.

### Convolutional Neural Networks (CNNs)

Exploit spatial locality in images/signals. Shared-weight filters slide across inputs detecting local patterns.

- Convolution layers → pooling layers → fully connected classifier
- Translation invariance through weight sharing
- Key architectures: ResNet (skip connections), EfficientNet, ConvNeXt

### Recurrent Neural Networks (RNNs)

Process sequential data by maintaining hidden state. Largely replaced by transformers for most tasks.

- Vanilla RNN: severe vanishing gradient problems
- LSTM: gated memory cells that selectively remember/forget
- GRU: simplified LSTM variant
- Bidirectional variants: process sequence in both directions

### Transformers

The dominant architecture since 2017. Process entire sequences in parallel using attention.
See the dedicated section below.

### Graph Neural Networks (GNNs)

Operate on graph-structured data (molecules, social networks, knowledge graphs).

- Message passing: nodes aggregate information from neighbors
- GCN, GAT, GraphSAGE variants

## The Transformer Architecture

### Core Components

**Self-Attention**: each token attends to all other tokens in the sequence, learning which are relevant.

$$\text{Attention}(Q, K, V) = \text{softmax}\left(\frac{QK^T}{\sqrt{d_k}}\right)V$$

- **Q (Query)**: what am I looking for?
- **K (Key)**: what do I contain?
- **V (Value)**: what information do I provide?
- Scaling by $\sqrt{d_k}$ prevents dot products from growing too large

**Multi-Head Attention**: run multiple attention operations in parallel with different learned projections, then concatenate. Each head can learn different relationship types (syntax, semantics, positional).

**Positional Encoding**: since attention has no inherent notion of order, position information must be injected. Original paper used sinusoidal functions; modern variants use learned or rotary positional embeddings (RoPE).

**Feed-Forward Network**: after attention, each position is processed independently by a small MLP (typically with GELU activation).

**Layer Normalization + Residual Connections**: stabilize training of deep stacks. Pre-norm (normalize before attention) is now standard over post-norm.

### Encoder-Decoder Architecture

- **Encoder**: processes the full input sequence with bidirectional self-attention
- **Decoder**: generates output autoregressively with causal (masked) self-attention + cross-attention to encoder
- **Encoder-only** (BERT): bidirectional understanding — classification, NER, embeddings
- **Decoder-only** (GPT, Llama, Claude): autoregressive generation — the dominant LLM architecture
- **Encoder-decoder** (T5, BART): translation, summarization

### Modern LLM Training Pipeline

1. **Pre-training**: next-token prediction on massive text corpora (trillions of tokens). Learns language structure, world knowledge, reasoning patterns. Extremely compute-intensive.
2. **Supervised Fine-Tuning (SFT)**: train on curated instruction-response pairs to follow human intent
3. **RLHF/RLAIF**: Reinforcement Learning from Human/AI Feedback — align model behavior with human preferences using reward models
4. **Inference optimization**: quantization (FP16→INT8→INT4), KV-cache, speculative decoding, continuous batching

## Key Concepts

### Bias-Variance Tradeoff

- **Bias**: error from oversimplified model (underfitting)
- **Variance**: error from model being too sensitive to training data (overfitting)
- Sweet spot: enough complexity to capture patterns, not so much that it memorizes noise
- Modern deep learning challenges this — very large models can generalize well even with zero training loss (double descent phenomenon)

### Train/Validation/Test Split

- **Training set** (70-80%): model learns from this
- **Validation set** (10-15%): tune hyperparameters, early stopping, model selection
- **Test set** (10-15%): final evaluation only — never look at this during development
- **Cross-validation**: rotate which fold is the validation set — useful for small datasets

### Feature Engineering vs Representation Learning

- Traditional ML: humans design features (domain knowledge required)
- Deep learning: model learns features automatically from raw data
- Reality: even deep learning benefits from thoughtful input preprocessing

### Metrics

| Task           | Metrics                                  | When to Use                                          |
| -------------- | ---------------------------------------- | ---------------------------------------------------- |
| Classification | Accuracy, Precision, Recall, F1, AUC-ROC | Balanced: accuracy. Imbalanced: F1, AUC              |
| Regression     | MSE, RMSE, MAE, R²                       | MAE for robustness, RMSE if large errors matter more |
| Ranking        | NDCG, MAP, MRR                           | Search, recommendation systems                       |
| Generation     | Perplexity, BLEU, ROUGE, BERTScore       | Language tasks, translation, summarization           |

### Embeddings

Dense vector representations of discrete objects (words, images, users, products). Similar objects have nearby vectors. Foundation of modern ML.

- Word2Vec, GloVe → static word embeddings (historical)
- BERT, GPT embeddings → contextual (same word gets different vectors in different contexts)
- Sentence/document embeddings: mean pooling, [CLS] token, dedicated embedding models
- Used for: semantic search, clustering, recommendation, retrieval-augmented generation (RAG)

## Practical ML Wisdom

- **Data quality > model sophistication**. Clean, representative data matters more than the fanciest architecture.
- **Start simple**. Establish a baseline with logistic regression or a simple tree before reaching for deep learning.
- **Compute the theoretical best**. Some problems have irreducible noise — know the Bayes error rate before chasing diminishing returns.
- **Watch for data leakage**. If validation metrics look too good, suspect that test data leaked into training.
- **Deployment ≠ training**. Monitor for distribution shift, prediction latency, and failure modes in production.
- **Scaling laws exist**. For transformers, performance improves predictably with more data, parameters, and compute (Chinchilla scaling laws).
- **Fine-tuning is cheaper than pre-training**. For most applications, start from a pre-trained foundation model and adapt it.
