# ML Experiment Tracking — MLflow, W&B, Neptune & Reproducibility

Machine learning development requires systematic tracking of experiments: the code, hyperparameters, training data, metrics, and artifacts produced by each run. Experiment tracking enables comparison, reproducibility, debugging, and collaboration across teams.

## The Experiment Problem

Training a single model produces many outputs: final weights, intermediate checkpoints, training logs, visualizations, evaluation metrics, and metadata. Untracked experiments become invisible once models ship:

- **Irreproducibility**: "What parameters created model v3?" Weeks later, you can't rebuild it.
- **Hidden regressions**: Model A's 92% F1 isn't compared rigorously to Model B's 91.8%. Was it statistically significant? On which splits?
- **Lost context**: A team member trained 47 models. Which hyperparameters mattered? Why did learning rate 0.01 outperform 0.001?
- **Audit friction**: Regulators ask "which data trained this deployed model?" If code is versioned but data isn't, the answer becomes work.

Experiment tracking systems record **runs** (individual training executions) as immutable records, making it tractable to reason about a model's provenance.

## Core Tracking Dimensions

An effective experiment tracker captures:

1. **Parameters** — configuration space: learning rate, batch size, architecture choices, regularization constants. Immutable after training starts.
2. **Metrics** — time-series measurements: loss, validation accuracy, F1, AUC, custom domain metrics. Logged at intervals (per epoch, per batch).
3. **Artifacts** — files: trained model weights (.pkl, .h5, .pth), confusion matrices (PNG), YAML configs, dataset checksums.
4. **Metadata** — context: git commit hash, git branch, start/end timestamp, user, compute environment (GPU model, CPU count), Python version.
5. **Code** — versioned source: tracked via git integration to link runs to exact source state.

Runs are compared in a matrix: rows are runs, columns are parameters/metrics. This enables filtering ("show runs with learning_rate > 0.001") and sorting ("order by validation_acc DESC").

## MLflow: Lifecycle Management

MLflow is an open-source framework for managing ML lifecycles. It consists of four components:

### Tracking
Logs parameters, metrics, and tagged artifacts to a persistent backend (local file system or SQL database). The Python API is minimal:

```python
import mlflow
mlflow.log_param("learning_rate", 0.01)
mlflow.log_metric("loss", 0.25, step=100)
mlflow.log_artifact("model.pkl")
```

MLflow tracks **active runs** — each `mlflow.start_run()` context opens a record. Multiple runs can be grouped into **experiments**, organizing by project or hypothesis.

### Model Registry
A centralized repository for models: stores staging (dev → staging → production), versions, and stage transitions. Enables governance: "which model is in production today?"

### Projects
Packaging reproducibility: a project specifies entry points, dependencies (conda.yaml), and parameters. Re-running is `mlflow run` with new parameters — deterministic, no manual env setup.

### Models
Standardized format for model serialization (MLmodel). Agnostic to framework: supports sklearn, PyTorch, TensorFlow, custom Python. Enables model versioning independent of code.

Tradeoff: MLflow is **lightweight** but requires manual integration into training code. No UI for hyperparameter search.

## Weights & Biases (W&B): Visualization & Sweeps

W&B emphasizes real-time dashboard visualization and automated hyperparameter search.

### Core Features
- **Dashboard**: live metric plots, tables, parallel coordinates plot (visualizing high-dimensional parameter space). Shareable with teams.
- **Sweeps**: automated hyperparameter search via grid, random, or Bayesian search. W&B trains N parallel jobs (configurable), logs metrics, selects next parameters based on prior runs.
- **Artifacts & Media**: log images, audio, videos, tables directly. Useful for segmentation masks, generated samples, attention visualizations.
- **Reports**: narrative documents mixing prose, plots, and run tables. Shareable execution summaries.

W&B is **proprietary SaaS** (cloud account required) but includes free tier for public projects and academic research.

Integration:

```python
import wandb
wandb.init(project="catkin", config={"lr": 0.01})
wandb.log({"loss": 0.25})
wandb.log({"confusion_matrix": wandb.plot.confusion_matrix(y_true, y_pred, class_names)})
wandb.finish()
```

Sweep config (parameters to search):

```yaml
program: train.py
method: bayesian
metric:
  name: val_accuracy
  goal: maximize
parameters:
  learning_rate:
    min: 0.0001
    max: 0.1
  batch_size:
    values: [16, 32, 64]
```

W&B then auto-runs `python train.py --learning_rate X --batch_size Y` and suggests next parameter values. Useful for teams without GPU clusters for parallel search.

## Neptune & Other Platforms

**Neptune**: Similar to W&B. Emphasizes metadata tracking (code version, dataset version, environment) and team collaboration. Good for teams requiring on-prem deployment alternatives to W&B.

**DVC (Data Version Control)**: Not solely an experiment tracker but complements tracking systems. DVC versions datasets and models using git-like workflows (`dvc add data.csv` creates `data.csv.dvc` pointer + remote storage). Enables "which data version trained model v3?" answers.

Typical pipeline: MLflow for run logging, DVC for data/model versioning, git for code.

## Experiment Comparison & Reproducibility

Effective comparison requires rigor:

### Cross-Run Comparison
A matrix view shows runs R1 and R2 with metrics side-by-side. Key questions answered:
- Are metrics improvements statistically significant? (requires confidence intervals or multiple seed runs)
- Did hyperparameter X correlate with metric Y across the 50 runs?
- What was the best performing parameter combination?

### Reproducibility Contracts
A run is reproducible if re-executing with identical inputs (code, data, hyperparameters, randomness seed) produces identical outputs. Experiment trackers enable this by recording:

1. **Code**: git hash or full source snapshot
2. **Data**: dataset version identifier (DVC hash or dataset manifest)
3. **Seed**: random seed recorded, settable before training
4. **Dependencies**: Python/library versions in environment.yml or requirements.txt
5. **Hardware**: GPU model matters for numerical precision; ideally fixed

Reproducibility is often false precision: floating-point arithmetic varies slightly across GPU generations or cuDNN versions. Practical reproducibility accepts small variance in metrics (±0.0001 F1).

## Active Learning Integration

Experiment tracking informs **active learning** workflows: iteratively label high-uncertainty examples, retrain, and evaluate. Tracking enables:
- Iteration N: base model accuracy 78%
- Labeled k new examples via active query
- Iteration N+1: accuracy 81%

Comparing iterations within a single "active learning" experiment clarifies the value of the labeling strategy.

## Workflow Patterns

### Baseline → Hyperparameter Sweep → Comparison
1. Train a simple baseline model (e.g., logistic regression), log to experiment `baseline_models`
2. Run W&B sweep over neural network architectures → logged to `nn_models`
3. View dashboard: compare best NN to baseline. If NN doesn't beat baseline significantly, question if complexity is justified.

### Multi-seed Runs for Significance
Always run multiple seeds (e.g., 5 runs per config). Report mean ± std metric. Avoids overstating significance of single-run gains (78% vs 77%).

### Debugging via Artifact Preservation
When a training fails mysteriously, preserved artifacts (logs, checkpoints) enable post-hoc investigation. Experiment tracking auto-saves; manual scripts may not.

## Tradeoffs in Tool Choice

| Dimension | MLflow | W&B | Neptune |
|-----------|--------|------|---------|
| **Deployment** | self-hosted | SaaS (cloud) | SaaS + self-hosted option |
| **UI** | basic | excellent dashboards | good dashboards |
| **Cost** | free | free tier + paid | free tier + paid |
| **Learning curve** | low | low | low |
| **Hyperparameter search** | no | built-in (Sweeps) | via external tools |
| **Integration** | lightweight | heavier footprint | moderate |

No universal winner. Teams starting often use **MLflow locally** for logging, graduate to **W&B** when sweeps and collaboration matter, or self-host **Neptune** for HIPAA/privacy requirements.

## See Also
- [Machine Learning Fundamentals](machine-learning-fundamentals.md) — training concepts
- [MLOps — Model Lifecycle, Serving & Production Concerns](ml-operations.md) — deployment context
- [Data Versioning & Governance](data-engineering-governance.md) — datasets and audit