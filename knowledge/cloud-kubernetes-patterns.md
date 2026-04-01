# Kubernetes Deployment Patterns — Sidecars, Operators, Autoscaling & Resource Management

## Overview

Kubernetes patterns describe repeatable solutions for pod composition, lifecycle management, and operational automation. These patterns abstract away infrastructure details and enable declarative, intent-driven deployment. Key patterns address **sidecar injection** (co-locate companion containers), **operator automation** (custom resource controllers), **resource governance** (requests/limits, QoS), and **scaling** (HPA/VPA/KEDA based on metrics/events).

## Pod Composition Patterns

### Sidecar Pattern

Co-locate a main application container with a "sidecar" container in the same pod. Sidecar shares network namespace (localhost access), storage volumes, and lifecycle (both start/stop together).

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: web-app
spec:
  containers:
  - name: main
    image: myapp:1.0
    ports:
    - containerPort: 8080
  - name: sidecar
    image: envoy:latest  # proxy, monitoring agent, log forwarder, etc.
    ports:
    - containerPort: 8001
  shareProcessNamespace: true  # optional: sidecar can signal main container
volumes:
  - name: shared-logs
    emptyDir: {}
```

**Use cases:**
- **Service mesh proxy (Envoy, Linkerd):** Intercept network traffic for mTLS, load balancing, retry logic.
- **Logging sidecar:** Tail logs from main container, forward to external system (Datadog, Splunk, ELK).
- **Monitoring agent (Prometheus node-exporter):** Collect metrics on behalf of main container.
- **Init/cleanup sidecar:** Run privileged setup before main container starts.

**Tradeoffs:**
- Pro: Separation of concerns (app + infrastructure logic decoupled); reusable sidecar image; shared IPC/volumes.
- Con: Resource overhead (2+ containers per pod); debugging harder (logs scattered); startup order can be implicit (main container may start before sidecar is ready).

**Pattern variants:**
- **Init container:** One-time setup container that runs to completion before main container starts (same pod, linear lifecycle).
- **Pod disruption budget (PDB):** Guarantee minimum pod replicas available during node drain/upgrade.

### Ambassador Pattern

Sidecar proxy that acts as local endpoint for external services. Main container connects to localhost; ambassador proxies to remote service.

```
Main app → localhost:9000 → Ambassador sidecar → Remote service (with auth, routing, retry)
```

**Common scenario:**
- Main app: Generic CRUD service, no auth handling.
- Ambassador: Handles OAuth, rate limiting, service discovery.
- Benefit: App logic decoupled from infrastructure; ambassador is generic/reusable.

**Tradeoff vs. Sidecar:** Both co-locate; ambassador emphasizes the proxy role (represent external service locally).

### Adapter Pattern

Sidecar that normalizes/transforms output from main container for external consumers.

```
Main container produces logs in custom format → Adapter sidecar → Standard log format → log aggregator
```

**Example:** Main container exposes metrics on port 8000 in proprietary format; adapter sidecar translates to Prometheus / OpenMetrics format on port 9090.

**Tradeoff:** Adapter adds transformation layer; if main container can output standard format natively, adapter overhead is wasted.

### Init Container Pattern

Runs to completion before main containers start. Useful for sequential setup.

```yaml
spec:
  initContainers:
  - name: db-migration
    image: myapp-migrate:1.0
    command: ["./migrate.sh"]  # runs to completion
  containers:
  - name: app
    image: myapp:1.0
    dependsOn:  # waits for init to complete before starting
```

**Use case:** Database schema migration before app starts; setting up shared volumes; validating dependencies.

## Kubernetes Operator Pattern

Operators extend Kubernetes with custom resource definitions (CRDs) and controllers. An **operator** is a set of CRDs + controller that automates operational tasks.

### CRD (Custom Resource Definition)

Define custom resources beyond built-in Pods, Deployments, Services.

```yaml
# Define the custom resource schema
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: databases.mycompany.io
spec:
  group: mycompany.io
  names:
    kind: Database
    plural: databases
  scope: Namespaced
  versions:
  - name: v1alpha1
    served: true
    storage: true
    schema:
      openAPIV3Schema:
        properties:
          spec:
            properties:
              version:
                type: string
              replicas:
                type: integer
              backupPolicy:
                type: string

---
# Use the custom resource
apiVersion: mycompany.io/v1alpha1
kind: Database
metadata:
  name: prod-postgres
spec:
  version: "14.5"
  replicas: 3
  backupPolicy: "daily"
```

### Controller

Watches CRDto instances and reconciles desired state with actual state. Controller loop:

```
1. Watch Database.v1alpha1 resources
2. For each Database instance:
   a. Read spec (desired state): version, replicas, backupPolicy
   b. Check actual state: running pods, StatefulSet, PersistentVolumes
   c. If mismatch: create/update/delete Kubernetes resources to match spec
   d. Update status subresource with actual state, errors
```

**Example operator: PostgreSQL Operator (Zalando)**

```
CRD: PostgresCluster(name, version, replicas, storage, backups)
  ↓
Controller watches PostgresCluster instances
  ↓
For each PostgresCluster:
  - Create StatefulSet with correct # replicas
  - Create PersistentVolumeClaims for storage
  - Create ConfigMap with PostgreSQL config
  - Run init containers to bootstrap cluster
  - Manage failover, backups, upgrades (via sidecar scripts)
```

**Tradeoff:** Operators encode operational expertise (procedures for backup, scaling, upgrade). Benefit: infrastructure as code; scaling Postgres becomes `kubectl patch postgrescluster prod --replicas=5`. Cost: managing operator itself (updates, bugs, CRD compatibility).

**Operator maturity levels (OperatorHub classification):**
- **Basic Install:** CRD created; basic provisioning.
- **Seamless Upgrades:** Operator manages version upgrades without data loss.
- **Full Lifecycle:** Backup, restore, disaster recovery, scaling.
- **Deep Insights:** Anomaly detection, proactive remediation.

## Resource Management: Requests, Limits, QoS

Kubernetes schedules pods based on **requests** (minimum guaranteed resources) and enforces **limits** (hard caps).

### Requests vs. Limits

```yaml
spec:
  containers:
  - name: app
    resources:
      requests:
        cpu: 100m        # 0.1 CPU cores, guaranteed
        memory: 128Mi    # 128 MiB, guaranteed
      limits:
        cpu: 500m        # hard cap; process throttled if exceeded
        memory: 256Mi    # hard cap; process killed (OOMKilled) if exceeded
```

**Scheduler behavior:**
- Pod is scheduled only on nodes with available resources ≥ pod's requests.
- Multiple pods' requests can sum to > node capacity if limits differ (oversubscription possible).

### Quality of Service (QoS) Classes

**Guaranteed:** requests = limits. Highest priority; only evicted if node memory critical.

```yaml
requests: {cpu: 500m, memory: 512Mi}
limits:   {cpu: 500m, memory: 512Mi}
```

**Burstable:** requests < limits. Medium priority; evicted before Guaranteed if node memory pressure.

```yaml
requests: {cpu: 100m, memory: 128Mi}
limits:   {cpu: 500m, memory: 512Mi}  # can burst
```

**BestEffort:** no requests/limits. Lowest priority; evicted first.

```yaml
# no resources section
```

**Tradeoff:** Guaranteed costs more (reserve full limit); Burstable is cheaper (pay for request, burst occasional). BestEffort is cheapest but risky (first to die).

### Namespace Resource Quotas

Limit total resource consumption per namespace.

```yaml
apiVersion: v1
kind: ResourceQuota
metadata:
  name: compute-quota
  namespace: production
spec:
  hard:
    requests.cpu: "10"      # all pods' requests.cpu sum ≤ 10 cores
    requests.memory: "20Gi"
    limits.cpu: "20"
    limits.memory: "40Gi"
    pods: "100"             # max 100 pods in namespace
```

Prevents single team from consuming all cluster resources.

## Horizontal Pod Autoscaling (HPA)

Auto-scale pod replicas based on observed metrics.

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: app-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: myapp
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70  # scale up if avg CPU > 70%
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300  # wait 5 min before scaling down
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
      - type: Percent
        value: 100
        periodSeconds: 60
```

**Metrics sources:**
- **Resource metrics:** CPU, memory (from kubelet via metrics-server).
- **Custom metrics:** Application-defined (requests/sec, queue depth).
- **External metrics:** Prometheus, Datadog, queue length from external system.

**Scaling algorithm:** `desiredReplicas = ceil(currentMetric / targetMetric * currentReplicas)`

If avg CPU is 90% and target is 70%, and currently 5 replicas: `desiredReplicas = ceil(90/70 * 5) = 7 replicas`

## Vertical Pod Autoscaling (VPA)

Auto-adjust resource requests/limits based on actual usage.

```yaml
apiVersion: autoscaling.k8s.io/v1
kind: VerticalPodAutoscaler
metadata:
  name: app-vpa
spec:
  targetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: myapp
  updateMode: "Auto"  # Auto, Recreate, Initial, Off
  resourcePolicy:
    containerPolicies:
    - containerName: "*"  # all containers
      minAllowed:
        cpu: 100m
        memory: 64Mi
      maxAllowed:
        cpu: 1
        memory: 1Gi
```

**Difference from HPA:** HPA adjusts replica count; VPA adjusts resource requests/limits per pod. Often used together: VPA rightsizes each pod, HPA scales number of pods.

**Tradeoff:** VPA requires pod recreation (to update requests); causes brief downtime if not managed carefully. Recommender runs separately and suggests request changes.

## KEDA (Kubernetes Event Autoscaling)

Scale workloads based on event sources: SQS queue depth, Kafka lag, HTTP request rate, etc.

```yaml
apiVersion: keda.sh/v1alpha1
kind: ScaledObject
metadata:
  name: sqs-scaler
spec:
  scaleTargetRef:
    name: worker-deployment
    kind: Deployment
  minReplicaCount: 1
  maxReplicaCount: 100
  triggers:
  - type: aws-sqs-queue
    metadata:
      queueURL: https://sqs.us-east-1.amazonaws.com/123456789/myqueue
      awsRegion: us-east-1
      queueLength: 5  # scale up if average queue length > 5 per pod
```

**Use case:** Background job processing, event-driven autoscaling beyond CPU/memory metrics.

**Tradeoff:** Adds custom scaler (KEDA controller); requires monitoring integration; event lag (queue length measured every 15 seconds) means scale reacts slower than HPA (every 30 seconds).

## Topology Spread Constraints

Distribute pods across topology domains (zones, nodes) to reduce blast radius.

```yaml
spec:
  topologySpreadConstraints:
  - maxSkew: 1  # difference between zones ≤ 1 pod
    topologyKey: topology.kubernetes.io/zone
    whenUnsatisfiable: DoNotSchedule
    labelSelector:
      matchLabels:
        app: myapp
  - maxSkew: 2
    topologyKey: kubernetes.io/hostname
    whenUnsatisfiable: ScheduleAnyway
```

**Effect:** Scheduler tries to spread pods evenly across zones. If zone A has 3 pods and zone B has 1, new pod likely scheduled to zone B to minimize skew.

**Tradeoff:** Strict topology constraints (maxSkew: 0) can prevent scheduling if insufficient nodes; loose constraints (ScheduleAnyway) allow constraint violation to avoid cluster unavailability.

## Pod Disruption Budgets (PDB)

Guarantee minimum availability during voluntary disruptions (node drain, cluster upgrade).

```yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: app-pdb
spec:
  minAvailable: 2  # at least 2 pods always available
  selector:
    matchLabels:
      app: myapp
```

Or: `maxUnavailable: 1` ensures ≥ (replicas - 1) remain. Scheduler respects PDB during drain.

## See Also

- **architecture-resilience:** Kubernetes failure modes and recovery
- **containers-orchestration:** Docker and container fundamentals
- **devops-kubernetes:** Kubernetes architecture and components
- **infrastructure-service-discovery:** Kubernetes Service discovery
- **devops-service-mesh:** Istio, Linkerd operator patterns for traffic management