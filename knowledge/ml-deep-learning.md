# Deep Learning — Neural Network Concepts & Architectures

## The Perceptron — A Historical Starting Point

The perceptron, introduced in the late 1950s, represents the simplest possible neural computation: a weighted sum of inputs passed through a threshold function. It maps directly to a linear decision boundary — capable of separating linearly separable classes but famously unable to learn XOR.

The perceptron learning rule adjusts weights proportionally to error, a principle that persists in modern gradient-based methods. The "AI winter" triggered partly by the XOR limitation demonstrated how architectural constraints determine what a model can represent — a theme that recurs throughout deep learning history.

| Concept                | Perceptron        | Modern Neuron                    |
| ---------------------- | ----------------- | -------------------------------- |
| Combination            | Weighted sum      | Weighted sum + bias              |
| Activation             | Step function     | Non-linear (ReLU, sigmoid, etc.) |
| Training               | Perceptron rule   | Backpropagation                  |
| Representational power | Linear boundaries | Arbitrary functions (with depth) |

Multi-layer perceptrons (MLPs) resolved the XOR problem by stacking layers, but effective training of deep networks required backpropagation and careful initialization — problems that took decades to solve practically.

## Forward Propagation and Backpropagation

Forward propagation computes the network's output by passing inputs through successive layers of weighted sums and activations. Each layer transforms its input:

```
z = W · x + b          # linear combination
a = activation(z)      # non-linear transformation
```

Backpropagation applies the chain rule of calculus to compute how each weight contributes to the loss. The gradient flows backward through the network:

```
∂L/∂w = ∂L/∂a · ∂a/∂z · ∂z/∂w
```

Key considerations in backpropagation:

- **Vanishing gradients** — In deep networks with saturating activations (sigmoid, tanh), gradients shrink exponentially through layers, making early layers nearly untrainable
- **Exploding gradients** — Conversely, gradients can grow exponentially, destabilizing training. Gradient clipping addresses this by capping gradient magnitudes
- **Computational graphs** — Modern frameworks build dynamic or static computation graphs, enabling automatic differentiation without manual gradient derivation

The chain rule is mathematically elegant but numerically fragile. Deep networks amplify floating-point errors across layers, making training stability an engineering concern as much as a mathematical one.

## Activation Functions — Why Non-Linearity Matters

Without non-linear activations, any depth of linear layers collapses to a single linear transformation. Non-linearity gives networks the capacity to approximate arbitrary functions.

| Activation | Formula                     | Range       | Pros                                               | Cons                                   |
| ---------- | --------------------------- | ----------- | -------------------------------------------------- | -------------------------------------- |
| Sigmoid    | 1/(1+e⁻ˣ)                   | (0, 1)      | Probabilistic interpretation, smooth               | Vanishing gradients, not zero-centered |
| Tanh       | (eˣ-e⁻ˣ)/(eˣ+e⁻ˣ)           | (-1, 1)     | Zero-centered, stronger gradients than sigmoid     | Still saturates at extremes            |
| ReLU       | max(0, x)                   | [0, ∞)      | Computationally cheap, reduces vanishing gradient  | "Dying ReLU" — neurons stuck at zero   |
| Leaky ReLU | max(αx, x)                  | (-∞, ∞)     | Addresses dying ReLU                               | Adds a hyperparameter (α)              |
| ELU        | x if x>0, α(eˣ-1) otherwise | (-α, ∞)     | Smooth, pushes mean toward zero                    | More expensive than ReLU               |
| GELU       | x·Φ(x)                      | ≈(-0.17, ∞) | Smooth approximation of ReLU, used in transformers | Computationally heavier                |
| Swish      | x·σ(x)                      | ≈(-0.28, ∞) | Self-gated, smooth                                 | Marginal gains, added cost             |

The choice of activation interacts with initialization, learning rate, and architecture. ReLU's simplicity makes it a common starting point for feedforward and convolutional networks, while GELU has become prevalent in transformer architectures — but neither is universally optimal.

## Loss Functions as Optimization Targets

The loss function defines what "correct" means, and its choice shapes everything the model learns. The model optimizes whatever the loss measures, even if that diverges from the actual goal.

**Classification losses:**

- **Cross-entropy** — Measures divergence between predicted and true probability distributions. Heavily penalizes confident wrong predictions. Standard for classification but sensitive to label noise
- **Focal loss** — Down-weights easy examples, focusing training on hard cases. Developed for object detection where class imbalance is extreme
- **Hinge loss** — Maximizes margin between classes. Related to SVMs, less common in deep learning

**Regression losses:**

- **Mean squared error (MSE)** — Penalizes large errors quadratically. Sensitive to outliers
- **Mean absolute error (MAE)** — Linear penalty. More robust to outliers but has non-smooth gradient at zero
- **Huber loss** — Quadratic near zero, linear far from zero. Balances MSE sensitivity with MAE robustness

Loss function design is itself a research area. In generative models, adversarial losses, perceptual losses, and reconstruction losses are often combined — with weighting coefficients that significantly affect output quality.

## Gradient Descent Variants

Gradient descent iteratively adjusts parameters in the direction that reduces loss. The variants differ in how much data informs each update:

| Variant          | Data per Update         | Trade-offs                                                          |
| ---------------- | ----------------------- | ------------------------------------------------------------------- |
| Batch (full)     | Entire dataset          | Stable gradients, expensive per step, can get stuck in sharp minima |
| Stochastic (SGD) | Single sample           | Noisy but escapes local minima, high variance in updates            |
| Mini-batch       | Subset (32–512 typical) | Balances noise and stability, enables GPU parallelism               |

Modern optimizers build on mini-batch SGD with adaptive mechanisms:

- **Momentum** — Accumulates gradient history, smoothing oscillations in ravines
- **AdaGrad** — Per-parameter learning rates that decrease for frequently updated parameters. Effective for sparse data but can stop learning prematurely
- **RMSProp** — Addresses AdaGrad's diminishing rates with exponential moving average
- **Adam** — Combines momentum with RMSProp. Widely used as a default, though some evidence suggests SGD with momentum generalizes better in certain settings
- **AdamW** — Decouples weight decay from gradient updates, correcting a subtle bug in Adam's regularization

No optimizer dominates across all problems. Adam converges faster in practice but SGD with careful tuning sometimes finds flatter minima that generalize better — a trade-off between training efficiency and final model quality.

## Learning Rate — A Fundamental Hyperparameter

The learning rate controls step size during optimization. Too large and training diverges; too small and training stalls or gets trapped.

**Scheduling strategies:**

- **Constant** — Simple but rarely optimal across full training
- **Step decay** — Reduce by a factor at fixed intervals
- **Cosine annealing** — Smooth decay following a cosine curve, sometimes with warm restarts
- **Warmup** — Start small and ramp up, stabilizing early training especially with large batches or transformers
- **One-cycle** — Increase then decrease over training, allowing aggressive peak rates
- **Reduce on plateau** — Monitor a metric and reduce when improvement stalls

Learning rate interacts with batch size, optimizer choice, and model architecture. Larger batches generally tolerate higher learning rates. The relationship between batch size and learning rate is approximately linear in some regimes — doubling batch size allows roughly doubling learning rate — but this scaling breaks down at extremes.

## Overfitting, Underfitting, and the Bias-Variance Trade-off

- **Underfitting (high bias)** — The model is too simple to capture patterns. Training and validation errors are both high
- **Overfitting (high variance)** — The model memorizes training data, including noise. Training error is low but validation error is high

The classical bias-variance decomposition:

```
Total Error = Bias² + Variance + Irreducible Noise
```

In deep learning, the relationship between model size and generalization is more nuanced than classical statistics suggests. Very large models can exhibit "double descent" — generalization improves again after the interpolation threshold, challenging traditional intuitions about model capacity.

Diagnosing the regime matters more than defaulting to complex solutions:

| Signal                      | Likely Issue        | Potential Approaches                                   |
| --------------------------- | ------------------- | ------------------------------------------------------ |
| High train + high val error | Underfitting        | Increase capacity, train longer, reduce regularization |
| Low train + high val error  | Overfitting         | More data, regularization, reduce capacity             |
| Low train + low val error   | Good fit            | Monitor for distribution shift                         |
| Erratic training curves     | Optimization issues | Adjust learning rate, check data quality               |

## Regularization Concepts

Regularization techniques reduce overfitting by constraining the model's effective capacity.

**Dropout** randomly zeros a fraction of activations during training, forcing the network to distribute representations across neurons rather than relying on specific co-adaptations. At inference time, all neurons are active but scaled. Dropout rate is architecture-dependent — too high starves the network of capacity, too low provides minimal regularization.

**Weight decay (L2 regularization)** adds a penalty proportional to the squared magnitude of weights, discouraging large values. This biases toward simpler functions but the "simplicity" is defined in weight space, not function space — a distinction that matters theoretically.

**Early stopping** monitors validation performance and halts training when it degrades, using the model state from the best validation epoch. This implicitly regularizes by limiting the number of optimization steps.

**Data augmentation** creates training variety through transformations (rotation, cropping, noise injection, mixup). This regularizes by expanding the effective training distribution, and is often more impactful than architectural regularization for vision tasks.

**Label smoothing** softens hard targets (replacing 1.0 with 0.9 and distributing 0.1 across other classes), preventing the model from becoming overconfident and improving calibration.

These techniques interact — combining dropout with strong weight decay can over-regularize, while data augmentation can reduce the need for architectural regularization.

## Convolutional Neural Networks (CNNs) — Spatial Data

CNNs exploit spatial structure through three ideas: local receptive fields, weight sharing, and spatial pooling. A convolutional layer slides learned filters across input, producing feature maps that capture local patterns.

**Core components:**

- **Convolutional layers** — Learn spatially local features. Early layers detect edges and textures; deeper layers compose these into complex patterns
- **Pooling layers** — Downsample feature maps, providing translation invariance and reducing computation. Max pooling retains strongest activations; average pooling preserves more information
- **Stride and dilation** — Stride controls step size; dilated convolutions expand receptive field without increasing parameters

**Architectural patterns:**

- Increasing filter count with decreasing spatial dimensions
- Residual connections (skip connections) that enable gradient flow through very deep networks
- Depthwise separable convolutions that factorize standard convolutions for efficiency
- Feature pyramid networks that combine multi-scale features

CNNs assume spatial locality and translation equivariance — when these assumptions hold (images, spatial signals), they are highly parameter-efficient. When they don't (irregular graphs, long-range dependencies without hierarchy), other architectures may be more appropriate.

## Recurrent Neural Networks and LSTMs — Sequential Data

RNNs process sequences by maintaining hidden state across time steps, allowing information to persist. The basic recurrence:

```
h_t = activation(W_h · h_{t-1} + W_x · x_t + b)
```

**The vanishing gradient problem in RNNs** is acute: gradients through long sequences shrink exponentially, making it difficult to learn long-range dependencies.

**LSTMs** address this with a gating mechanism — forget, input, and output gates that control information flow through a cell state. The cell state provides a gradient highway that can preserve information over many time steps.

**GRUs** simplify the LSTM architecture with two gates instead of three, offering comparable performance with fewer parameters in many settings.

| Architecture | Parameters | Long-range Memory            | Parallelizable |
| ------------ | ---------- | ---------------------------- | -------------- |
| Vanilla RNN  | Fewest     | Poor (vanishing gradients)   | No             |
| LSTM         | Most       | Good (gated cell state)      | No             |
| GRU          | Moderate   | Good (simplified gating)     | No             |
| Transformer  | Varies     | Excellent (direct attention) | Yes            |

The sequential nature of RNNs prevents parallelization during training, limiting scalability. This computational constraint, combined with transformers' superior long-range modeling, has shifted many sequence tasks toward attention-based architectures — though RNNs remain relevant for streaming and resource-constrained scenarios.

## The Transformer and Attention Mechanism

The attention mechanism computes weighted combinations of values based on query-key similarity, enabling direct connections between any positions in a sequence regardless of distance.

**Self-attention (scaled dot-product):**

```
Attention(Q, K, V) = softmax(QK^T / √d_k) · V
```

**Multi-head attention** runs several attention functions in parallel with different learned projections, allowing the model to attend to different relationship types simultaneously.

**Why attention changed the landscape:**

- Removes the sequential bottleneck of RNNs, enabling full parallelization
- Directly models relationships between any two positions (O(1) path length vs O(n) for RNNs)
- The attention matrix is interpretable — revealing which tokens influence each other

**Transformer architecture components:**

- Positional encodings (sinusoidal or learned) inject sequence order information
- Layer normalization stabilizes training
- Feedforward layers after attention provide per-position nonlinear transformation
- Residual connections around each sub-layer

**Trade-offs:** Self-attention has O(n²) complexity in sequence length, making very long sequences expensive. Various efficient attention variants (sparse, linear, low-rank) trade some modeling capacity for reduced complexity.

The transformer's success spans NLP, vision (ViTs), audio, protein folding, and code generation — suggesting that attention mechanisms capture something fundamental about relational reasoning, not just linguistic structure.

## Transfer Learning — Representations That Generalize

Transfer learning leverages the observation that features learned on one task are often useful for related tasks. A model pretrained on a large dataset learns general representations in early layers and task-specific representations in later layers.

**Approaches:**

- **Feature extraction** — Freeze pretrained layers, train only new output layers. Fast, requires little data, but limits adaptation
- **Fine-tuning** — Unfreeze some or all pretrained layers, training with a small learning rate. More flexible but risks catastrophic forgetting
- **Progressive unfreezing** — Gradually unfreeze layers from output toward input, each with decreasing learning rates

**Foundation models** extend transfer learning to the extreme — models pretrained on internet-scale data that serve as starting points for diverse downstream tasks. This paradigm shifts the cost structure: enormous pretraining investment amortized across many applications via relatively cheap adaptation.

The effectiveness of transfer depends on domain similarity between source and target tasks. Transferring vision features between natural images works well; transferring between natural images and medical imaging works partially; transferring between images and audio requires more architectural creativity.

## Batch Normalization and Layer Normalization

Normalization techniques stabilize and accelerate training by controlling the distribution of layer activations.

**Batch normalization** normalizes across the batch dimension for each feature. During training, it uses batch statistics; during inference, it uses running statistics accumulated during training.

- Reduces internal covariate shift (the distribution of layer inputs changing as preceding layers update)
- Enables higher learning rates and reduces sensitivity to initialization
- Acts as a mild regularizer due to mini-batch noise
- Introduces dependency on batch size — problematic for small batches or variable-length sequences

**Layer normalization** normalizes across the feature dimension for each sample independently.

- No batch dependency — works with any batch size and in recurrent/transformer architectures
- Standard choice for transformers and sequence models
- Slightly different learned representations compared to batch normalization

| Normalization | Normalizes Over               | Batch Dependent | Typical Use        |
| ------------- | ----------------------------- | --------------- | ------------------ |
| Batch norm    | Batch dimension               | Yes             | CNNs, feedforward  |
| Layer norm    | Feature dimension             | No              | Transformers, RNNs |
| Instance norm | Spatial dimensions per sample | No              | Style transfer     |
| Group norm    | Channel subgroups             | No              | Small-batch vision |

The placement of normalization (pre-activation vs post-activation, pre-layer vs post-layer) affects training dynamics. Pre-norm transformers tend to be more stable during training, while post-norm can achieve marginally better final performance with careful tuning.

## Practical Considerations

**Initialization** — Xavier/Glorot initialization scales weights based on fan-in and fan-out, maintaining variance through layers with symmetric activations. Kaiming/He initialization accounts for ReLU's asymmetry. Poor initialization can render even well-designed architectures untrainable.

**Mixed precision training** — Using 16-bit floating point for most operations while maintaining 32-bit for critical accumulations. Roughly doubles throughput on modern hardware with minimal accuracy impact, though some architectures and loss landscapes are more sensitive than others.

**Curriculum learning** — Presenting training data in meaningful order (easy to hard) can improve convergence speed and sometimes final performance. The definition of "easy" and the pacing schedule are domain-specific design choices.

**Ensemble methods** — Combining predictions from multiple models trained with different initializations or architectures improves robustness and calibration at the cost of compute. Knowledge distillation can compress ensemble knowledge into a single model.

The field evolves rapidly. Architectural innovations (mixture of experts, state space models, diffusion mechanisms) continue to expand the design space, and the interaction between architecture, data, scale, and training strategy remains an active area of both empirical and theoretical investigation.
