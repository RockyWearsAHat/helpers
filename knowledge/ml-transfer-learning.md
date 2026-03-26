# ML Transfer Learning — Fine-Tuning, Domain Adaptation & Few-Shot Learning

Transfer learning leverages knowledge learned from one task to improve performance on another task. Rather than training models from random initialization on each new task, transfer learning starts from a pre-trained model, reusing learned representations.

This dramatically reduces data and compute requirements: ImageNet-trained ResNet50 transferred to medical imaging outperforms training from scratch on 10K medical images. Transfer enables practical ML on resource-constrained problems.

## Pre-Trained Models & Foundation Models

### ImageNet Era (2012-2018)
Large-scale labeled datasets (1.2M images, 1000 classes) enabled training deep CNNs. Once trained on ImageNet, the learned filters (edge detectors, textures, shapes) transfer to new vision tasks: medical imaging, satellite imagery, fine-grained classification.

Why transfer works: early layers learn general-purpose features (edges, gradients). Later layers learn task-specific patterns. For a new task, reuse early layers + retrain late layers.

### Language Model Foundation Era (2018+)
NLP underwent similar shift: BERT, GPT, RoBERTa trained on massive unlabeled text (Wikipedia, Common Crawl) via self-supervising (masked language modeling). Pre-trained representations then transfer to:
- Sentiment classification (3K labeled examples)
- Named entity recognition
- Question answering
- Paraphrase detection

Key difference from supervised pre-training: self-supervised objectives (predicting masked tokens) learn richer representations than supervised objectives (predicting category).

### Foundation Models (2022+)
Large models trained on diverse data (text + code for LLMs, text + image for multimodal models) exhibit emergent abilities: few-shot learning (GPT-3 solves new task from 5 examples), instruction following (GPT-4).

Foundation models (GPT, Claude, LLaMA, CLIP) form the base layer for most modern applications. Task-specific tuning is usually lightweight (few-shot prompt, lightweight adapter).

## Fine-Tuning Strategies

### Full Fine-Tuning
Retrain **all** model weights on task-specific data.

**Pros**: maximum model capacity (all parameters update).
**Cons**: requires large labeled dataset (1000s+ examples typical), expensive (GPU hours), risks overfitting on small datasets.

**When**: abundant task-specific data (e.g., financial institution with 100K transaction labels for fraud detection).

Example (PyTorch):

```python
model = transformers.BertForSequenceClassification.from_pretrained("bert-base-uncased")
optimizer = torch.optim.Adam(model.parameters(), lr=2e-5)
for epoch in range(3):
    for batch in train_loader:
        logits = model(batch['input_ids'], attention_mask=batch['attention_mask'])
        loss = F.cross_entropy(logits, batch['labels'])
        loss.backward()
        optimizer.step()
```

All 110M BERT parameters update.

### Head-Only Fine-Tuning (Feature Extraction)
Freeze pre-trained body; retrain only final classification head (last 1-2 layers).

**Pros**: fast (fewer parameters to optimize), low risk of overfitting, cheap.
**Cons**: limited capacity for task-specific adaptation.

**When**: small labeled dataset (100-1000 examples), or features already expressive for new task.

```python
model = transformers.BertForSequenceClassification.from_pretrained("bert-base-uncased")
for param in model.bert.parameters():
    param.requires_grad = False  # freeze BERT body

optimizer = torch.optim.Adam(model.classifier.parameters(), lr=1e-3)
# Only classifier head trains
```

Trade-off: 100x fewer parameters to train (0.1M vs 110M), but weaker final performance if task requires conceptual shifts.

### Parameter-Efficient Fine-Tuning

For large models (7B+ parameters), full fine-tuning expensive. Adapter methods re-engineer the model to reduce trainable parameters.

#### LoRA (Low-Rank Adaptation)
Instead of updating weights $W$ directly, compute updates as low-rank decomposition:

$$\Delta W = AB^T$$

where $A \in \mathbb{R}^{d \times r}$ and $B \in \mathbb{R}^{d' \times r}$, $r \ll d$ (rank $r = 8$ or $16$ typical).

Forward pass: $(Wx + ABx)$ — original projection + low-rank update.

**Pros**: reduces parameters from 7B to 0.1B (100x fewer), no inference latency overhead.
**Cons**: slightly lower final performance vs full fine-tuning (often <1% gap).

**Use**: fine-tuning 7B+ models on modest hardware (single GPU). LoRA enables researchers to adapt LLMs without enterprise clusters.

Libraries: `peft` (Parameter-Efficient Fine-Tuning, HuggingFace).

#### QLoRA (Quantized LoRA)
Further compress by quantizing base model (e.g., 4-bit), apply LoRA on top.

Extreme parameter efficiency: fit 65B model on single consumer GPU (VRAM ~24GB).

Trade-off: base model slightly degraded by quantization (typically <1% accuracy loss).

#### Prefix Tuning
Prepend trainable "prefix" tokens before input. Only prefix updated.

Output tokens condition on both prefix + input. Variant of soft prompting: prefix is learnable instead of hand-written.

**Pros**: moderate parameter reduction, straightforward implementation.
**Cons**: less studied; empirically LoRA often superior.

### Learning Rate & Regularization

Fine-tuning hyperparameter choice crucial:

- **Learning rate**: typically **1-2 orders lower** than from-scratch training (1e-3 from-scratch → 1e-5 fine-tuning). High LR unlearns pre-training.
- **Warmup**: few steps of rising LR (prevents distribution shock). 
- **Regularization**: L2 less critical (pre-training regularizes); dropout sometimes harmful (reduces capacity).
- **Epochs**: fewer (2-3) avoids overfitting on small datasets.

### Multi-Task Fine-Tuning
Instead of single-task, fine-tune on multiple related tasks simultaneously:

```python
loss = 0.5 * sentiment_loss + 0.3 * paraphrase_loss + 0.2 * toxicity_loss
loss.backward()
```

Shared representations benefit all tasks; helps generalization, prevents catastrophic forgetting.

## Domain Adaptation

Transfer learning and domain adaptation are orthogonal concerns: BERT pre-trained on general English transfers poorly to biomedical text without biomedical fine-tuning.

**Domain**: distribution of inputs + implicit task knowledge. Shift between domains when:
- **Covariate shift**: P(X) changes, P(Y|X) same (distribution of emails shifts, spam detection rule remains)
- **Label shift**: P(Y) changes, P(X|Y) same (fraud rate increases)
- **Concept drift**: P(Y|X) changes (user interests evolve)

### Domain-Specific Pre-Training
Train language model on domain corpus (biomedical papers, legal contracts, code). Result: SciBERT, BioBERT, CodeBERT.

Tradeoff: expensive (pre-training costs ~GPU weeks), but enables strong performance on domain tasks.

### Unsupervised Domain Adaptation
Source labeled, target unlabeled. Learn representations invariant across domains:

**Maximum Mean Discrepancy (MMD)**: regularize loss to match source + target distributions:

$$\text{Loss} = \text{supervised\_loss} + \lambda \cdot \text{MMD}(\text{source\_reps}, \text{target\_reps})$$

MMD = mean difference between kernel embeddings. Minimizing MMD aligns representations.

Other methods: domain adversarial training (minimax game between domain classifier and feature extractor).

### Instance Weighting
Reweight source examples by likelihood under target distribution. Gives more importance to "hard" source examples similar to target.

Mathematics: importance weights $w_i = P(Y_i | X_i) / P(X_i)$ (hard to compute; approximated via density ratio estimation).

## Few-Shot & Zero-Shot Learning

### Few-Shot Learning
Train model on **N examples per class** (N=1, 5, 10 typical), evaluate on new classes unseen during training.

**N-way K-shot**: N classes, K examples per class. Standard benchmark: 5-way 5-shot (25 labeled examples total).

Approaches:
- **Prototypical networks**: compute class prototype (mean embedding), classify by nearest prototype.
- **Matching networks**: meta-learn metric for similarity.
- **MAML (Model-Agnostic Meta-Learning)**: meta-learn initial weights enabling fast adaptation.

**Scale**: few-shot enables personalization (learn user preferences from 5 past interactions).

### Zero-Shot Classification
**No** examples from target classes during training. Classify via semantic transfer: class names (text) + pre-trained embeddings.

CLIP (Contrastive Language-Image Pre-training): given image, compute similarity to "photo of a [CLASS]" for all classes, pick highest.

Example:

```python
import clip
model, preprocess = clip.load("ViT-B/32")
image = preprocess(img).unsqueeze(0)
text = clip.tokenize(["a photo of a dog", "a photo of a cat", "a photo of a bird"])
with torch.no_grad():
    image_features = model.encode_image(image)
    text_features = model.encode_text(text)
    logits = image_features @ text_features.T
print(logits.softmax(dim=-1))  # [0.9, 0.05, 0.05] → likely dog
```

**Pros**: works on unseen classes, scalable (N classes, no retraining).
**Cons**: accuracy lower than fine-tuned classifiers.

**Use**: open-vocabulary classification (classify anything, not bounded set).

## Adapter Methods (Modular Transfer)

Instead of single monolithic model per task, use modular adapters:

```
Input → Shared Backbone → Adapter A → Output A
                       → Adapter B → Output B
                       → Adapter C → Output C
```

Adapters (~5-10% parameters each) stack in series or parallel. Teams can independently develop task-specific adapters; shared backbone (99% parameters) remains stable.

**Benefit**: parameter efficiency (1000 tasks = 100% params of backbone + 5% per adapter).

Example: HuggingFace Transformers + AdapterHub.

## When NOT to Transfer Learn

- **Task drastically different**: pre-training on satellite imagery may not help microscopy (scale, lighting fundamentally different)
- **Extreme data imbalance**: if task has abundant labeled data vs pre-training dataset, from-scratch training may win
- **Inference constraints**: adapter overhead or larger model size violates serving requirements

In practice: transfer learn unless strong evidence against. Marginal compute cost of fine-tuning small models justifies the attempt.

## See Also
- [Machine Learning Fundamentals](machine-learning-fundamentals.md) — core learning concepts
- [LLM Fine-Tuning — From Full Updates to Efficient Adaptation](genai-fine-tuning.md) — LLM-specific tuning
- [Computer Vision — Convolution, Detection, Segmentation, and Transfer Learning](cs-computer-vision.md) — vision transfer patterns
- [ML Operations — Model Lifecycle, Serving & Production Concerns](ml-operations.md) — deployment after transfer