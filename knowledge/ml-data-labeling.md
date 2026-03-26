# ML Data Labeling — Annotation Types, Quality Control & Active Learning

Machine learning models depend entirely on the quality and quantity of labeled training data. For supervised learning, labeling (annotation) is often the most expensive and time-consuming phase: humans assign ground-truth labels to raw data (images, text, audio, sequences).

## Annotation Types

### Classification
Each example receives a single categorical label from a fixed set. Common variants:

- **Binary**: positive/negative, spam/not-spam, fraud/legitimate
- **Multi-class**: single label from K categories (image tagging: cat, dog, bird, fish)
- **Multi-label**: multiple non-exclusive labels per example (document: ["politics", "technology", "opinion"])

UI: radio buttons (multi-class) or checkboxes (multi-label).

Inter-annotator agreement (IAA) typical: 85-95% for clear categories, 60-75% for subjective tasks (sentiment: strong positive vs mild positive).

### Named Entity Recognition (NER)
Identify and label named entities (people, organizations, locations, dates) within text.

Example:
```
Text: "Alice works at Google in Mountain View"
Annotations:
  Alice → PERSON
  Google → ORGANIZATION
  Mountain View → LOCATION
```

Challenges: entity boundaries (is "New York" one or two tokens?), ambiguity ("Apple" = company or fruit?).

### Segmentation (Image/Video)
Label region geometry, not just class:

- **Instance segmentation**: pixel-level mask for each object ("3 dogs in image" → 3 separate masks)
- **Semantic segmentation**: pixel class (road, sidewalk, sky)
- **Bounding box**: rectangular region around object
- **Keypoint**: labeled joints (pose estimation, landmarks on face)

UI: polygon/brush tools, automated suggestions (bounding box proposals).

Cost: 10-100x more expensive than classification (per example).

### Sequence Labeling
Label each item in sequence (part-of-speech tagging, slot filling):

```
Text: "Book me a flight to Boston tomorrow"
Tags:
  Book → action
  me → recipient
  flight → object
  Boston → destination
  tomorrow → date
```

### Subjective Tasks
Tasks inherently ambiguous: sentiment ("Is this review positive?"), offensive speech detection, humor.

Result: lower IAA, need multiple annotators per example + majority vote or aggregation logic.

## Quality Control

### Inter-Annotator Agreement (IAA)
Measure consistency across multiple annotators labeling the same example.

**Cohen's Kappa** (binary/multi-class):
$$\kappa = \frac{P(a) - P(e)}{1 - P(e)}$$

where P(a) = observed agreement, P(e) = expected by chance.

- κ > 0.8: strong agreement
- 0.6-0.8: moderate
- < 0.6: weak (consider task redesign)

**Fleiss' Kappa**: generalization to >2 annotators.

### Quality Metrics
1. **Accuracy vs gold standard**: if subset of examples have expert-verified labels, measure annotator accuracy against these. Flag low-accuracy annotators.
2. **Consistency over time**: same annotator re-labels examples weeks later; measure agreement. Drift suggests fatigue or concept drift.
3. **Coverage**: % of examples labeled within timeout window (e.g., 48 hours).

### Crowd Control
When using crowdsourcing (Amazon Mechanical Turk, Scale AI, Labelbox):

- **Qualification test**: workers must label 10 example items correctly before joining task
- **Attention checks**: embed 5% known examples; reject workers scoring <80% on checks
- **Redundancy**: each example labeled by 3+ workers; aggregate via majority vote
- **Price**: pay above local minimum wage; too-cheap tasks attract low-effort workers

Typical cost: $0.01-$1 per example, depending on complexity and geography.

## Weak Supervision

Fully manual labeling does not scale for huge datasets. **Weak supervision** uses noisy, approximate labeling functions to generate training data.

### Snorkel Framework
Labeling functions vote on class; aggregate votes via generative model:

```python
from snorkel.labeling import labeling_function, LabelingFunction

@labeling_function()
def lf_keyword(x):
    return SPAM if "viagra" in x.text else ABSTAIN

@labeling_function()
def lf_sender_domain(x):
    return SPAM if x.sender_domain=="fake-pharma.ru" else ABSTAIN

L = apply_lfs([lf_keyword, lf_sender_domain], X)
# L is matrix: rows = examples, cols = LFs, values = predicted label or ABSTAIN
```

Snorkel learns **LF accuracy** (generative model) without ground truth, then weights votes accordingly. Unreliable LFs weighted low; accurate ones weighted high.

**Output**: soft labels (probability estimates), suitable for training.

Tradeoff: weak labels degrade model vs manual labels, but enable learning on massive datasets.

### Other Weak Supervision Patterns
- **Rules**: heuristic labeling function (regex, threshold)
- **Distant supervision**: use external knowledge base to infer labels (Wikipedia links → entity relations)
- **Transfer learning**: use pre-trained classifier (may be wrong for domain, but better than random)
- **Semi-supervised**: learn simultaneously on labeled + unlabeled data, propagating unsure labels via consistency regularization

## Active Learning

Rather than randomly sampling examples to label, **active learning** selects the most informative examples — those where the model is most uncertain or where labeling reduces expected error most.

### Uncertainty Sampling
Train initial model on small labeled set. On unlabeled pool, score by uncertainty:

```
Confidence = max(model_probs)
Select examples with lowest confidence
```

For Bayesian approaches: use predictive variance, or Monte Carlo dropout (high variance = uncertain).

**Why**: model is most wrong where uncertain; labeling these is high value.

Iteration:
1. Label batch of k most-uncertain examples
2. Retrain model
3. Repeat until convergence or labeling budget exhausted

**Typical speedup**: active learning halves labeling cost vs random sampling for many tasks.

### Query by Committee
Maintain ensemble of models; select examples where ensemble disagrees most.

Disagreement = variance across ensemble predictions. High disagreement = informative.

### Cost-Sensitive Active Learning
Some labeling functions are cheaper than others:
- Weak rule check: $0.01 (keyword match, model confidence)
- Human verification: $0.10 (crowdworker confirms)
- Expert annotation: $5.00 (domain expert provides detailed label)

Select examples to maximize information gain per dollar spent.

## Labeling Platforms & Tools

### Label Studio (Open Source)
Web UI for image/text/audio labeling. Self-hosted or cloud. Features:
- Multi-annotator workflows
- ML-assisted labeling (model suggestions pre-fill labels)
- QA workflows (review + approve)

### Scale AI (Commercial)
Managed labeling service. Scale hires and trains annotators, manages QA. Expensive but hands-off.

### Hugging Face Datasets with Annotation GUIs
`huggingface_hub`'s community features enable collaborative labeling.

### Amazon SageMaker Ground Truth
AWS-hosted labeling with automatic worker management, qualification tests, consensus voting.

## Dataset Shift During Labeling

Large-scale labeling takes months. Dataset distribution can shift:
- **Temporal drift**: data distribution in production changes (user behavior shifts, new user cohort enters market)
- **Annotator drift**: team grows; new annotators may label differently than original team
- **Label shift**: costs of errors change (fraud detection: false negatives become more costly)

Mitigations:
- Periodically re-label held-out examples; track IAA over time
- Stratify labeling (ensure minority classes represented even if rare in data)
- Monitor label distribution; alert if class balance shifts unexpectedly

## Practical Labeling Workflows

### MVP Approach
1. **Manually label 100-500 examples** (high quality, diverse). Train baseline.
2. **Active learning**: select next 100 most-uncertain. Label manually.
3. **Iterate** until model reaches target performance.
4. **Weak rules** for remaining volume (if viable).

Cost: focused on high-value labels, avoids labeling easy/redundant examples.

### Team Scale-Up
1. Define label schema (guidelines document with examples, edge cases).
2. Have 3 annotators label ~50 examples, compute IAA. Iterate schema until IAA > 0.75.
3. Deploy: task to crowd, 3 redundant annotations per example.
4. QA: flag disagreements, have expert adjudicate.
5. Iterate: as model trains, select next batch via active learning or random within error region.

### Combining Manual + Weak
- 10K examples: weak supervision (rules, pre-trained model) → noisy labels
- 1K examples: manual annotation (high quality)
- Train: weight manual examples higher (e.g., loss multiplier 5x)
- Improves label quality + coverage

## Common Pitfalls

1. **Insufficient label schema**: annotators guess; results: low IAA, poor model.
2. **Ignoring rare classes**: imbalanced dataset (99% negative, 1% positive) leads to inactive learning focusing on majority. Stratify or use class weighting.
3. **Premature scale-up**: deploying crowd before schema is solid. Result: 50K low-quality labels requiring wasteful relabeling.
4. **Forgetting ground truth**: weak supervision valuable for bootstrap, but validate model eventually on manual labels.
5. **Labeling toxic data unsafely**: content moderation, hate speech detection can harm annotators psychologically. Pair with reviewer support, job rotation.

## See Also
- [Machine Learning Fundamentals](machine-learning-fundamentals.md) — supervised learning concepts
- [Data Quality — Validation, Governance & Observability](data-engineering-quality.md) — dataset governance
- [ML Model Evaluation](ml-model-evaluation.md) — measuring model quality post-labeling