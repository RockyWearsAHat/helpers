# ML Model Deployment — Serving Patterns, Containers & Production Reliability

Model deployment spans the infrastructure, orchestration, and operational practices required to execute trained models in production. A model that achieves 95% accuracy in a notebook often fails in production due to deployment mismatches: infrastructure differences, data distribution shifts, scaling needs, and monitoring gaps.

## The Deployment GAP

Training produces a checkpoint file (weights + architecture). Deployment must:
1. **Serve predictions** at required latency/throughput (sub-100ms for interactive apps, hours acceptable for batch)
2. **Handle traffic variability** (100 QPS in morning, 5 QPS at 3am)
3. **Rollback safely** if model degrades, without data loss
4. **Monitor continuously** for performance drift, data shift, or infrastructure faults
5. **Update models** as new versions train, without downtime

These are orthogonal to training: a perfect model serves poorly on broken infrastructure.

## Serving Patterns

### Batch Serving
Training jobs produce predictions for a fixed dataset (e.g., nightly scoring all customers). Results written to database or data lake.

**When**: Prediction latency is not real-time sensitive (daily recommendations, next-month forecasts).
**Throughput**: High (millions of predictions per job).
**Latency**: Low priority (minutes to hours acceptable).
**Implementation**: Spark, Airflow DAG, or Kubernetes CronJob running inference on a partition.

Cost advantage: batch jobs can run on cheaper, preemptible compute (spot instances) during off-peak hours.

### Real-Time (Online) Serving
HTTP/gRPC endpoint deployed as microservice. Client sends request, receives response synchronously.

**When**: Application needs predictions immediately (classifier for ad targeting, anomaly detector).
**Throughput**: Medium to high (10s to 1000s QPS).
**Latency**: Strict requirement (10-500ms typical).
**Implementation**: Flask/FastAPI wrapper, containerized (Docker), deployed on Kubernetes or GCP Cloud Run.

Example REST endpoint:

```
POST /api/v1/predict
{
  "user_id": 12345,
  "features": [0.1, 0.5, 0.9]
}

Response:
{
  "prediction": 0.87,
  "confidence": 0.92
}
```

Scaling: horizontal (more replicas) for high QPS, load-balanced (nginx, Kubernetes Service).

### Streaming (Online Reactive)
Event stream enters one end (Kafka, event bus), model consumes, emits predictions back to stream or external system.

**When**: Predictions react to continuous event flow (fraud detection on credit card txns, anomaly detection on metrics).
**Throughput**: Very high (1000s+ events/sec).
**Latency**: Low (100ms response acceptable).
**Implementation**: Flink, Spark Streaming, or Kafka consumer with embedded model.

Trade-off: streamers are stateful and harder to rollback than batch/REST.

## Model Serving Infrastructure

### Containers & Orchestration
Modern deployment uses **Docker** to package model + runtime + dependencies. Containerization ensures "it works on my machine" transfers to production.

Dockerfile (minimal example):

```dockerfile
FROM python:3.10-slim
COPY requirements.txt /app/
RUN pip install -r /app/requirements.txt
COPY model.pkl /app/
COPY app.py /app/
EXPOSE 8000
CMD ["python", "-u", "app.py"]
```

**Kubernetes** orchestrates containers: schedules pods, manages resource allocation, auto-scales replicas, handles node failures.

Deployment spec:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: fraud-detector
spec:
  replicas: 3  # three pods
  selector:
    matchLabels:
      app: fraud-detector
  template:
    metadata:
      labels:
        app: fraud-detector
    spec:
      containers:
      - image: myrepo/fraud-detector:v2
        name: model
        resources:
          limits:
            memory: "2Gi"
            cpu: "1"
        livenessProbe:  # crash pod if unhealthy
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 30
          periodSeconds: 10
```

K8s automatically restarts failed pods, scales to match load, and performs rolling updates.

### Model Registries
Central repository for model artifacts (weights, metadata, version history). Examples: MLflow Model Registry, Hugging Face Model Hub, AWS SageMaker Model Registry.

Registry stores:
- Model name + version (e.g., `fraud-v2`, `fraud-v3`)
- Stage: Dev → Staging → Production
- Metadata: training date, metrics, dataset version, responsible owner
- Prediction schema: input/output feature names and types

Enables: "What model is in production? Can I query its metrics? Can I roll back to v1?"

## Deployment Strategies

### Blue-Green Deployment
Two identical environments (Blue and Green). Blue serves production traffic; Green has new model version deployed. Once Green passes validation, traffic switches to Green. Rollback (switch back) is instant.

**Pros**: instant rollback, no downtime, easy to debug in idle Green environment.
**Cons**: requires double infrastructure (expensive); Green can become stale.

### Canary Deployment
Route small % (e.g., 5%) of live traffic to new model while 95% uses old. Monitor metrics carefully. If canary metrics degrade, halt rollout; else increase % gradually.

**Pros**: catches issues affecting real-world distribution without large blast radius.
**Cons**: more operational complexity, longer rollout duration, metric monitoring critical.

Example: Istio VirtualService with canary routing:

```yaml
spec:
  hosts:
  - fraud-detector
  http:
  - match:
    - uri:
        prefix: /api/
    route:
    - destination:
        host: fraud-detector
        subset: v1
      weight: 95
    - destination:
        host: fraud-detector
        subset: v2
      weight: 5
```

### Shadow Mode (Dark Launch)
New model runs in parallel but predictions are logged, not served. Real traffic gets old model predictions; new model predictions stored for later analysis. No user impact if new model fails.

Once shadow metrics validate (offline evaluation good, latency acceptable), promote to shadow traffic in production later.

Use case: model with significant latency addition; validate before going live.

## Feature Flags for Models

Rather than new code branches, use **feature flags** (config toggles) that specify which model version serves requests.

```python
if config.USE_MODEL_V2:
    prediction = model_v2.predict(features)
else:
    prediction = model_v1.predict(features)
```

Combined with remote config services (LaunchDarkly, Harness), flags are updated instantly without redeployment. Enables:
- A/B testing: 50% of users see model_v1, 50% see model_v2
- Gradual rollout: 1% → 5% → 100% over hours
- Instant rollback: flag set to False, old model serves again

## A/B Testing Models in Production

Compare two model versions by randomly assigning users to treatment (new model) vs control (old model), measuring business metrics (CTR, revenue, churn).

**A/B Test Design**:
- Split: deterministic hash of user_id ensures same user always sees same variant
- Duration: run for ≥10K impressions per variant (domain-dependent)
- Metric: primary (e.g., revenue per user); secondary (user satisfaction, latency)
- Significance: t-test or Bayesian posterior on metric difference

**Pitfalls**:
- P-hacking: declaring winner too early (look-alike bias)
- Multiple comparisons: testing 20 metrics inflates false positive rate
- Selection bias: if new model only tested on subset of users, results don't generalize

Bayesian A/B testing addresses early stopping via posterior probability: "What's the probability new model beats baseline?"

## Monitoring Model Performance

Production models degrade due to **data shift** (value distribution changes) or **concept drift** (true relationship between features and label changes).

### Key Metrics
1. **Prediction latency** — 50th, 99th percentile. If p99 > SLA, page ops.
2. **Input distribution** — compare current batch features to training data distribution (KL divergence, Kolmogorov-Smirnov test). If shift detected, alert.
3. **Business metric** — revenue, churn, CTR (if ground truth available with delay).
4. **Calibration** — predicted probability vs actual outcome. If model predicts 80% win rate for each decision, 80% should actually succeed.

Ground truth arrival **lag** is key: for email campaigns, outcome (opened/not) arrives days later. Monitoring must account for this lag.

### Drift Detection
Model performance may degrade if:
- **Data distribution shifts**: credit scoring model trained on 2019 data, evaluated on 2024 customer profiles (more remote workers → different loan defaults)
- **Real-world relationship changes**: click-through prediction model in political ad targeting (user interest shifts seasonally)

Response: retrain model on recent data, or alert if drift exceeds threshold.

## Rollback & Recovery

Model v2 deploys, metrics degrade 30min later. Able to rollback?

**Instant rollback** requires blue-green deployment or feature flags: disable v2, re-enable v1, done.

**Graceful degradation**: if all models fail, fall back to rule-based baseline (e.g., "recommend best seller"). Avoids cascading failure.

**Checkpoints**: keep previous N model versions accessible (not just latest). Storage cost is cheap; downtime cost is expensive.

**Data consistency**: if model depends on external data (user profile fetch, latest inventory), ensure rollback doesn't happen mid-transaction. Transactions/idempotency keys prevent duplicates.

## Deployment Tradeoffs

| Factor | Batch | Real-Time | Streaming |
|--------|-------|-----------|-----------|
| **Latency** | hours | 10-500ms | 100ms-1s |
| **Throughput** | millions/job | 100-10k QPS | 1000s events/s |
| **Cost** | cheap (off-peak) | moderate | high (persistent) |
| **Rollback** | redeploy DAG | re-serve old image | complex (state) |
| **Monitoring** | batch accuracy | live SLO | event-level checks |
| **Complexity** | low | medium | high |

## See Also
- [MLOps — Model Lifecycle, Serving & Production Concerns](ml-operations.md) — broader lifecycle
- [Container & Orchestration Basics](container-orchestration.md) — Kubernetes deep dive (if note exists)
- [Infrastructure & Reliability](architecture-resilience.md) — fault tolerance patterns