# MLOps — Model Lifecycle, Serving & Production Concerns

## The Gap Between Training and Serving

A model that performs well in a notebook often fails in production for reasons unrelated to its architecture or accuracy. The transition from experimentation to reliable service involves dependency management, data pipeline integrity, monitoring infrastructure, and operational practices that are absent from the training loop.

Research environments optimize for iteration speed and model quality. Production environments optimize for reliability, latency, cost, and maintainability. These differing objectives create a structural tension:

| Concern      | Research/Training      | Production/Serving         |
| ------------ | ---------------------- | -------------------------- |
| Priority     | Model quality          | Reliability, latency, cost |
| Data         | Static snapshots       | Live, evolving streams     |
| Compute      | Burst, GPU-heavy       | Sustained, cost-aware      |
| Iteration    | Fast experiments       | Controlled releases        |
| Failure mode | Wrong result           | Downtime, cascading errors |
| Dependencies | Loose, latest versions | Pinned, auditable          |

MLOps emerged to bridge this gap — applying software engineering practices to machine learning while accounting for ML's unique properties: data dependency, statistical behavior, gradual degradation, and the experimental nature of model development.

## Model Versioning and Experiment Tracking

ML experiments produce artifacts across multiple dimensions: code, data, hyperparameters, trained weights, evaluation metrics, and environment specifications. Version control for code alone is insufficient.

**What needs versioning:**

- **Code** — Model architecture, training scripts, preprocessing logic
- **Data** — Training sets, validation splits, feature definitions
- **Configuration** — Hyperparameters, training schedules, augmentation strategies
- **Artifacts** — Model weights, optimizer state, tokenizer files
- **Metrics** — Training curves, evaluation results, comparison baselines
- **Environment** — Package versions, hardware specification, random seeds

**Experiment tracking** records these dimensions for every training run, enabling comparison, reproduction, and rollback. The challenge is granularity: tracking too little makes reproduction impossible, tracking too much makes storage and navigation unwieldy.

Model registries extend versioning with lifecycle stages — models move through stages like "experimental," "staging," "production," and "archived." This provides a governance layer: who approved this model, what validation did it pass, when was it deployed.

The tension between reproducibility and practicality is real. Perfect reproducibility (identical results bit-for-bit) requires controlling GPU non-determinism, floating-point order of operations, and library internals — constraints that can significantly reduce training throughput.

## Feature Stores — Shared Feature Computation

Feature engineering often consumes more effort than model building. Feature stores address the problem of redundant, inconsistent, and poorly documented feature computation:

**Core capabilities:**

- **Centralized definitions** — Features are defined once and shared across teams and models
- **Dual serving** — The same feature definition serves both training (batch, historical) and inference (low-latency, point-in-time)
- **Point-in-time correctness** — When generating training data, feature values are retrieved as they existed at the time of each example, preventing data leakage from the future
- **Discovery** — Teams can browse and reuse existing features rather than reimplementing them

**Feature computation patterns:**

| Pattern              | Latency          | Freshness          | Use Case                                         |
| -------------------- | ---------------- | ------------------ | ------------------------------------------------ |
| Batch precomputation | High (scheduled) | Minutes to hours   | Stable features, large aggregations              |
| Streaming            | Low-medium       | Seconds to minutes | Time-windowed aggregations, counts               |
| On-demand            | Lowest           | Real-time          | Request-specific features, user input transforms |

The value proposition of a feature store depends on organizational scale. A single team with one model may find the overhead unjustified. Multiple teams sharing features across dozens of models often find the consistency and deduplication essential.

Feature stores introduce their own operational complexity — they become critical infrastructure that requires monitoring, schema evolution, storage management, and access control.

## Training-Serving Skew

Training-serving skew occurs when the data a model sees at inference differs systematically from what it saw during training. This is one of the most pervasive and insidious problems in production ML.

**Sources of skew:**

- **Feature computation differences** — Training features computed in batch (e.g., Spark) while serving features computed differently in a different framework or language
- **Data distribution shift** — The world changes after training. User behavior evolves, seasonality affects patterns, new categories appear
- **Preprocessing inconsistency** — Tokenization, normalization, or encoding logic duplicated between training and serving pipelines with subtle differences
- **Temporal leakage** — Training inadvertently uses information from the future relative to each example's timestamp
- **Schema drift** — Upstream data sources change column names, types, or semantics without notification

**Mitigation approaches:**

- Sharing preprocessing code between training and serving paths (single source of truth)
- Logging serving inputs and comparing distributions to training data
- Integration tests that feed known examples through both pipelines and compare outputs
- Feature stores that enforce consistent computation

Skew is difficult to detect because models degrade gradually rather than failing outright. A model experiencing input distribution shift may still produce plausible-looking outputs — just increasingly wrong ones.

## Model Serving Patterns

How a model serves predictions depends on latency requirements, throughput needs, and cost constraints.

### Batch Inference

Predictions are computed for a large set of inputs on a schedule (hourly, daily). Results are stored and looked up at request time.

- Low per-prediction cost due to hardware utilization efficiency
- Predictions can be stale between batch runs
- Appropriate for recommendation precomputation, periodic scoring, report generation
- Simpler infrastructure — no real-time serving layer needed

### Real-Time Inference

Predictions are computed on-demand for each request.

- Enables personalization based on current context
- Introduces latency constraints (often <100ms end-to-end)
- Requires serving infrastructure: model servers, load balancing, autoscaling
- Cost scales with request volume, not dataset size

### Edge Inference

Models run on devices (phones, IoT, browsers) rather than servers.

- Zero network latency, works offline
- Severe model size and compute constraints
- Update and rollback are more complex than server-side deployment
- Privacy advantages — data need not leave the device

### Hybrid Patterns

Many systems combine patterns: batch precomputation for common cases with real-time fallback for novel inputs, or edge inference with periodic server-side model updates.

| Pattern   | Latency           | Freshness               | Cost Model  | Complexity  |
| --------- | ----------------- | ----------------------- | ----------- | ----------- |
| Batch     | N/A (precomputed) | Minutes-hours           | Per-run     | Low         |
| Real-time | Low (ms)          | Immediate               | Per-request | Medium-high |
| Edge      | Lowest            | Depends on update cycle | Per-device  | High        |
| Hybrid    | Varies            | Varies                  | Mixed       | Highest     |

## A/B Testing and Canary Deployments for Models

Deploying a new model affects user experience and business metrics. Controlled rollout strategies limit blast radius.

**A/B testing** splits traffic between the current model and a candidate, measuring metrics over a statistically significant period. Considerations unique to ML:

- Metric selection matters deeply — offline metrics (accuracy, AUC) may not correlate with online metrics (engagement, revenue, user satisfaction)
- Statistical significance takes time, during which a subtly worse model serves some users
- Novelty effects can inflate short-term metrics for new models
- Interaction effects between multiple simultaneous experiments complicate analysis

**Canary deployment** gradually increases the candidate model's traffic share (1% → 5% → 25% → 100%) with automated rollback if key metrics degrade.

**Shadow mode (dark launch)** runs the new model on production traffic without serving its predictions. Both models process every request, but only the incumbent's predictions reach users. This reveals performance characteristics, latency, and error patterns without risk — but cannot measure user-facing metrics.

**Multi-armed bandit** approaches dynamically allocate traffic toward better-performing variants, reducing the cost of exploration compared to fixed-split A/B tests. The trade-off is added system complexity and less statistical clarity about individual variant performance.

## Model Monitoring — Drift and Degradation

Models degrade silently. Without monitoring, degradation is discovered only when downstream effects become visible — often too late.

### Types of Drift

- **Data drift (covariate shift)** — The distribution of input features changes. A model trained on summer data sees winter patterns
- **Concept drift** — The relationship between inputs and outputs changes. What constitutes "spam" evolves as adversaries adapt
- **Label drift (prior probability shift)** — The distribution of outcomes changes. Fraud rates spike during economic disruption

### Monitoring Dimensions

| What to Monitor          | Method                                          | Alert Threshold Considerations        |
| ------------------------ | ----------------------------------------------- | ------------------------------------- |
| Input distributions      | Statistical tests (KS, PSI, chi-squared)        | Domain-specific; some drift is normal |
| Prediction distributions | Distribution comparison, confidence calibration | Sudden shifts vs gradual trends       |
| Latency                  | Percentile tracking (p50, p95, p99)             | SLA-dependent                         |
| Error rates              | Ground truth comparison when available          | Delayed labels complicate this        |
| Feature coverage         | Null rate, out-of-range frequency               | Per-feature baselines                 |
| Business metrics         | Revenue, engagement, conversion                 | Causal attribution is difficult       |

**The challenge of delayed labels** — In many applications, the true outcome is known only after days, weeks, or never. Fraud labels arrive after investigation; click-through outcomes are immediate but long-term value is not. Monitoring must rely on proxy metrics and distribution checks when ground truth is unavailable.

## The Reproducibility Challenge

Reproducing an ML result requires reconstructing the exact intersection of data, code, environment, and (for GPU training) computational order.

**What must be pinned:**

- Data snapshots or hashes — not just "the training set" but the exact version of every source
- Code at exact commit — including preprocessing, augmentation, and evaluation logic
- Package versions — frameworks, CUDA, cuDNN, even OS patches can affect results
- Random seeds — for initialization, data shuffling, dropout masks
- Hardware specification — GPU architecture affects floating-point behavior
- Configuration — Every hyperparameter, schedule, and threshold

**Practical challenges:**

- Strict reproducibility constrains training speed (deterministic algorithms are often slower)
- Large datasets are expensive to snapshot and version
- External data sources (APIs, databases) are inherently non-reproducible over time
- Team velocity conflicts with documentation discipline

Many organizations settle for "reproducibility within tolerance" — the ability to retrain a model that achieves similar (not identical) performance. The appropriate tolerance depends on the stakes: a content recommendation model may accept wider variance than a medical diagnostic model.

## CI/CD for ML — What's Different

ML pipelines share CI/CD principles with traditional software but introduce additional dimensions:

**What traditional CI/CD tests:**

- Code compiles and passes unit tests
- Integration tests pass
- No regressions in behavior

**What ML CI/CD adds:**

- **Data validation** — Schema checks, distribution bounds, completeness, freshness
- **Training validation** — Model converges, metrics meet thresholds, no NaN losses
- **Model validation** — Performance on held-out data, fairness checks, latency benchmarks, comparison against baseline
- **Serving validation** — Model loads correctly, inference produces expected output shapes, latency meets SLA

**Pipeline orchestration** — ML pipelines are DAGs (directed acyclic graphs) of dependent stages: data ingestion → validation → preprocessing → training → evaluation → registration → deployment. Each stage can fail independently, and failures require different responses (data issues need investigation, training failures may need hyperparameter adjustment).

| Pipeline Stage | Trigger           | Failure Response                                  |
| -------------- | ----------------- | ------------------------------------------------- |
| Data ingestion | Scheduled / event | Alert data engineering, use last known good       |
| Validation     | After ingestion   | Block pipeline, investigate schema/distribution   |
| Training       | After validation  | Retry with last config, investigate if persistent |
| Evaluation     | After training    | Compare to baseline, block if degraded            |
| Deployment     | After approval    | Canary rollout, automated rollback                |

**Testing ML code** is complicated by non-determinism. Unit tests can verify data transformations, feature logic, and model architecture shapes. Integration tests can verify end-to-end pipeline execution. But testing that a model "learns correctly" requires statistical assertions with tolerance — flaky by nature.

## Feature Engineering as a Separate Concern

Feature engineering transforms raw data into representations that make patterns accessible to the model. It is often the highest-leverage activity in applied ML.

**Categories of features:**

- **Raw features** — Direct measurements (pixel values, text tokens, sensor readings)
- **Derived features** — Transformations of raw features (ratios, differences, polynomial expansions)
- **Aggregated features** — Summaries over time windows or groups (7-day average, per-user count)
- **Interaction features** — Combinations of multiple features (price × quantity, day-of-week × hour)
- **Embedding features** — Learned dense representations of categorical or textual data

**Trade-offs in feature engineering:**

| Approach               | Advantages                              | Disadvantages                          |
| ---------------------- | --------------------------------------- | -------------------------------------- |
| Manual engineering     | Domain knowledge encoded, interpretable | Labor-intensive, doesn't scale         |
| Automated (AutoML)     | Discovers unexpected transformations    | May produce uninterpretable features   |
| Deep learning features | Learns representations end-to-end       | Requires more data, less interpretable |
| Mixed                  | Combines domain expertise with learning | More complex pipeline                  |

Feature engineering exists in tension with end-to-end learning. Deep learning's appeal partly lies in learning features directly from raw data, but in tabular data and domain-specific applications, engineered features often outperform learned representations — or provide meaningful improvements when combined with them.

## The Human-in-the-Loop Pattern

For high-stakes predictions, full automation may be inappropriate. Human-in-the-loop patterns keep humans as decision makers while using models to augment capability.

**Common patterns:**

- **Model suggests, human decides** — The model ranks or recommends, but a human makes the final call. Common in medical diagnosis support, content moderation, and legal document review
- **Confidence-based routing** — High-confidence predictions are automated; uncertain cases are routed to human reviewers. The confidence threshold becomes a policy decision balancing efficiency against error tolerance
- **Active learning** — The model identifies examples where human labels would be most informative, optimizing annotation effort. Selection strategies include uncertainty sampling, query-by-committee, and expected model change
- **Exception handling** — Automated predictions with human review of flagged anomalies, appeals, or random audits

**Design considerations:**

- Human reviewers experience fatigue; interface design and workload management affect quality
- Feedback loops between human decisions and model retraining can amplify biases if not carefully designed
- The cost of human review constrains how much traffic can be routed to review
- Disagreement between model and human provides valuable training signal

## Cost Considerations — Training vs. Inference

The economic structure of ML differs from traditional software where marginal cost per request is negligible.

**Training costs:**

- GPU/TPU hours scale with model size, dataset size, and number of experiments
- Hyperparameter search multiplies base training cost
- Failed experiments are a real cost — efficient experiment design matters
- Large model pretraining can cost millions in compute

**Inference costs:**

- Scale with request volume and model complexity
- Latency constraints limit batching efficiency
- Continuous cost unlike one-time training
- Often dominates total cost at production scale

**Cost optimization levers:**

| Strategy                   | Applies To | Trade-off                                                          |
| -------------------------- | ---------- | ------------------------------------------------------------------ |
| Spot/preemptible instances | Training   | Risk of interruption, need checkpointing                           |
| Quantization               | Inference  | Slight accuracy loss for major speedup                             |
| Distillation               | Inference  | Compressed model may miss edge cases                               |
| Caching                    | Inference  | Memory cost, cache invalidation complexity                         |
| Batching                   | Inference  | Increased latency for individual requests                          |
| Cascade models             | Inference  | Small fast model handles easy cases, large model handles hard ones |
| Auto-scaling               | Inference  | Configuration complexity, cold start latency                       |

The compute cost landscape evolves as hardware, cloud pricing, and model efficiency research advance. Decisions made about model size and architecture have long-term cost implications that compound over the model's production lifetime.

## Model Compression

Production constraints — device memory, latency budgets, power consumption, cost — often require smaller or faster models than what training produces.

### Quantization

Reduces numerical precision of model weights and/or activations.

- **Post-training quantization** — Convert a trained model from 32-bit to 8-bit (or lower) floating point. No retraining needed, but may degrade accuracy for sensitive models
- **Quantization-aware training** — Simulates quantization effects during training, allowing the model to adapt. Better accuracy preservation at the cost of training complexity
- **Mixed precision** — Different layers quantized to different precisions based on sensitivity analysis

Typical ranges: FP32 → FP16 (minimal impact), FP32 → INT8 (noticeable speedup, small accuracy loss), FP32 → INT4 (significant compression, requires careful validation).

### Pruning

Removes weights or structures that contribute minimally to model output.

- **Unstructured pruning** — Zeros out individual weights. High compression ratio but sparse matrices are inefficient on standard hardware without specialized support
- **Structured pruning** — Removes entire neurons, channels, or attention heads. Hardware-friendly but coarser granularity
- **Iterative pruning** — Prune gradually during training, allowing the model to redistribute capacity

### Knowledge Distillation

Trains a smaller "student" model to replicate the behavior of a larger "teacher" model.

```
Loss = α · CrossEntropy(student_output, true_labels)
     + (1-α) · KL_Divergence(student_softmax, teacher_softmax)
```

The teacher's soft predictions carry information that hard labels lack — relative similarities between classes, confidence gradations, and implicit knowledge about the data manifold. The temperature parameter in the softmax controls how much of this "dark knowledge" is transferred.

| Compression Method   | Typical Size Reduction | Retraining Needed      | Hardware Dependency         |
| -------------------- | ---------------------- | ---------------------- | --------------------------- |
| Quantization (PTQ)   | 2-4x                   | No                     | Low                         |
| Quantization (QAT)   | 2-4x                   | Yes                    | Low                         |
| Structured pruning   | 2-10x                  | Often                  | Low                         |
| Unstructured pruning | 10-100x                | Often                  | High (needs sparse support) |
| Distillation         | Flexible               | Yes (student training) | Low                         |

These techniques can be combined — a distilled student model can be further quantized and pruned. The interaction effects matter: compressing an already-compressed model requires more care than compressing the original.

## Pipeline Orchestration and Workflow Management

ML workflows are complex DAGs with heterogeneous steps running on different infrastructure.

**Characteristics of ML pipelines:**

- Long-running steps (training can take hours to days)
- Heterogeneous compute requirements (CPU for preprocessing, GPU for training, CPU for evaluation)
- Conditional branching (retrain only if data has drifted sufficiently)
- Artifact passing between steps (datasets, model checkpoints, metrics)
- Need for both scheduled execution and event-triggered runs

**Design considerations:**

- Idempotency — Can a step be safely rerun without side effects?
- Caching — Which steps can be skipped if inputs haven't changed?
- Failure recovery — Can the pipeline resume from the last successful step?
- Resource management — How are GPU allocations, memory limits, and timeouts handled?
- Observability — Can operators see what's running, what failed, and why?

The boundary between "ML pipeline" and "data pipeline" is often blurred. Feature computation, data validation, and training preparation may run in the data engineering stack, with model training and evaluation in the ML stack. Integration points between these systems are common sources of fragility.

## Organizational Considerations

MLOps practices don't exist in a vacuum — they interact with team structure, skill distribution, and organizational maturity.

**Team models:**

- **Embedded** — ML engineers within product teams. Close to business needs, isolated from ML platform concerns
- **Centralized** — ML platform team serving multiple product teams. Consistent infrastructure, potential bottleneck
- **Hybrid** — Platform team provides tools and standards, product teams own their models

**Maturity progression:**

- Level 0: Manual everything — notebooks, manual deployment, no monitoring
- Level 1: Pipeline automation — Automated training, basic monitoring, manual deployment
- Level 2: CI/CD for ML — Automated testing, deployment, monitoring, and retraining triggers

The right level depends on business criticality, team size, and model count. Over-investing in infrastructure for a single experimental model wastes resources; under-investing for production-critical models creates unacceptable risk. The progression is not strictly linear — organizations often advance different capabilities (monitoring, deployment, testing) at different rates based on where pain is greatest.
