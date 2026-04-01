# Kubernetes Operators — CRD Design, Controller Patterns & Maturity Levels

## Operator Fundamentals

A Kubernetes operator extends the Kubernetes API to manage domain-specific resources (databases, message queues, monitoring systems) using the same declarative, reconciliation-based patterns built into Kubernetes itself.

Operators consist of:

1. **Custom Resource Definition (CRD)**: Defines a new Kubernetes API resource type (e.g., `Database`, `Cache`, `MessageQueue`).
2. **Controller**: A control loop that reconciles desired state (spec) with actual state, taking corrective actions.
3. **Business Logic**: Domain expertise encoded in the controller (how to provision, upgrade, backup the managed system).

Traditional approach: Operators replicate Kubernetes' reconciliation model for third-party systems, enabling GitOps and declarative infrastructure for everything.

## Custom Resource Definitions (CRDs)

CRDs define the schema and behavior of new resource types.

### CRD Structure

```yaml
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: databases.example.com
spec:
  names:
    kind: Database
    plural: databases
  scope: Namespaced
  group: example.com
  versions:
    - name: v1
      served: true
      storage: true
      schema:
        openAPIV3Schema:
          type: object
          properties:
            spec:
              type: object
              properties:
                dbType:
                  type: string
                  enum: ["postgres", "mysql", "mongodb"]
                size:
                  type: string
                  pattern: "^[0-9]+(Gi|Mi)$"
                backup:
                  type: object
                  properties:
                    enabled:
                      type: boolean
                    schedule:
                      type: string
              required: ["dbType", "size"]
            status:
              type: object
              properties:
                phase:
                  type: string
                  enum: ["Pending", "Running", "Failed"]
                conditions:
                  type: array
                  items:
                    type: object
```

### Status Subresources

CRDs support a `status` subresource, separating spec (desired state) from status (observed state). Controllers write to `.status`; users write to `.spec`. This prevents feedback loops where controller actions overwrite user intent.

```yaml
subresources:
  status: {}
```

Separation of concerns: `.spec` is read-only to controllers; `.status` is read-only to users (except as output).

## Controller-Runtime and Kubebuilder

### Controller-Runtime

controller-runtime is a Go library providing building blocks for Kubernetes controllers: client interactions, caching, event handling, and reconciliation loops.

Core concepts:

- **Client**: Typed access to Kubernetes resources. Caches for performance, syncs with apiserver.
- **Manager**: Runs multiple controllers, handles leader election, webhooks, and metrics.
- **Reconciler**: Implements the reconciliation loop—given a resource, fetch state, compare intent, take corrective actions.

**Reconciliation loop pattern**:

```go
func (r *DatabaseReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
    // 1. Fetch the Database resource
    db := &examplev1.Database{}
    if err := r.Get(ctx, req.NamespacedName, db); err != nil {
        return ctrl.Result{}, client.IgnoreNotFound(err)
    }
    
    // 2. Check if marked for deletion
    if db.ObjectMeta.DeletionTimestamp != nil {
        return r.finalize(ctx, db)
    }
    
    // 3. Ensure finalizer
    if !controllerutil.ContainsFinalizer(db, "databases.example.com/finalizer") {
        controllerutil.AddFinalizer(db, "databases.example.com/finalizer")
        if err := r.Update(ctx, db); err != nil {
            return ctrl.Result{}, err
        }
    }
    
    // 4. Reconcile—compare spec with actual state
    if db.Status.Phase == "" {
        db.Status.Phase = "Pending"
        if err := r.Status().Update(ctx, db); err != nil {
            return ctrl.Result{}, err
        }
    }
    
    // 5. Requeue after interval or on error
    return ctrl.Result{RequeueAfter: 10 * time.Second}, nil
}
```

### Kubebuilder

Kubebuilder scaffolds operator projects in Go, generating CRDs, controller boilerplate, and management scripts. It reduces ceremony, letting developers focus on business logic.

```bash
kubebuilder init --domain example.com --repo=github.com/myorg/operators
kubebuilder create api --group example.com --kind Database --version v1
```

Generates:
- CRD YAML in `config/crd/`
- Controller in `controllers/`
- Webhook stubs in `api/` for validation and mutation

## Operator-SDK

Operator-SDK is a broader framework supporting Go, Ansible, and Helm-based operators. It includes scaffolding, testing, bundle publishing, and integration with Operator Lifecycle Manager (OLM).

**Helm operators**: Wrap Helm charts with reconciliation, enabling chart-driven operators without writing Go:

```bash
operator-sdk init --plugins helm --operator-name my-operator
operator-sdk create api --helm-chart=./helm-chart-repo/my-chart
```

## CRD Design Patterns

### Spec vs. Status Separation

- **`.spec`**: User-written desired state. Immutable to controllers (except via webhook mutation).
- **`.status`**: Controller-written observed state, error messages, progress metrics.

```yaml
apiVersion: example.com/v1
kind: Database
metadata:
  name: prod-db
spec:
  dbType: postgres
  size: 10Gi
  version: "14.5"
status:
  phase: Running
  observedVersion: "14.5"
  conditions:
    - type: Ready
      status: "True"
      reason: "ProvisioningComplete"
  replicas: 3
  primaryPod: "prod-db-0"
```

### Finalizers and Garbage Collection

Finalizers ensure cleanup when a resource is deleted. Without finalizers, rapid deletion can orphan infrastructure.

```go
const FinalizerName = "databases.example.com/finalizer"

func (r *DatabaseReconciler) finalize(ctx context.Context, db *examplev1.Database) (ctrl.Result, error) {
    // Perform cleanup: delete cloud resources, drain connections, etc.
    if err := r.deleteCloudResources(ctx, db); err != nil {
        return ctrl.Result{}, err
    }
    
    // Remove finalizer to allow deletion
    controllerutil.RemoveFinalizer(db, FinalizerName)
    return ctrl.Result{}, r.Update(ctx, db)
}
```

### Condition Reporting

Conditions provide structured status messages, enabling external systems to react to state changes:

```yaml
status:
  conditions:
    - type: Provisioning
      status: "False"
      reason: "ProvisioningComplete"
      message: "Database provisioned successfully"
      observedGeneration: 1
      lastTransitionTime: "2026-03-25T10:00:00Z"
    - type: Ready
      status: "True"
      reason: "AllHealthChecksPass"
      message: "Database is healthy and accepting connections"
      observedGeneration: 1
      lastTransitionTime: "2026-03-25T10:05:00Z"
```

Conditions decouple status reporting from imperative polling.

## Validation and Mutation Webhooks

### Validating Webhooks

Intercept resource creation/update to enforce business rules before persistence:

```go
// +kubebuilder:webhook:path=/validate-example-com-v1-database,mutating=false,failurePolicy=fail,groups=example.com,resources=databases,verbs=create;update,versions=v1,name=vdatabase.example.com

func (r *Database) ValidateCreate() error {
    if r.Spec.Size == "" {
        return fmt.Errorf("size is required")
    }
    return nil
}
```

Validating webhooks reject invalid state before it reaches storage, preventing corruption.

### Mutating Webhooks

Transform resources before storage—set defaults, inject sidecars, add labels:

```go
// +kubebuilder:webhook:path=/mutate-example-com-v1-database,mutating=true,failurePolicy=fail,groups=example.com,resources=databases,verbs=create;update,versions=v1,name=mdatabase.example.com

func (r *Database) Default() {
    if r.Spec.BackupSchedule == "" {
        r.Spec.BackupSchedule = "@daily"
    }
}
```

Mutation happens before validation, allowing webhooks to set defaults that pass validation.

## Reconciliation Loop Patterns

### Level-Triggered vs. Edge-Triggered

Kubernetes reconciliation is **level-triggered**: the controller reacts to the current state, not transitions. If a reconciliation fails, the controller retries—it's not an event stream.

This means:

- Controllers must be **idempotent**—applying the same reconciliation multiple times produces the same result.
- External events (API calls, webhooks) trigger reconciliation via re-queuing.

### Requeue Strategies

```go
// Immediate requeue on error
return ctrl.Result{}, err

// Requeue after delay (exponential backoff for transient failures)
return ctrl.Result{RequeueAfter: 5 * time.Second}, nil

// No requeue (wait for external event via watch)
return ctrl.Result{}, nil
```

## Operator Lifecycle Manager (OLM)

OLM automates operator discovery, installation, updates, and dependency management.

**ClusterServiceVersion (CSV)**: Describes operator metadata, dependencies, and permissions:

```yaml
apiVersion: operators.coreos.com/v1alpha1
kind: ClusterServiceVersion
metadata:
  name: my-operator.v1.0.0
  namespace: operators
spec:
  displayName: My Operator
  description: Manages custom infrastructure
  version: 1.0.0
  maturity: alpha
  install:
    strategy: deployment
    spec:
      permissions:
        - serviceAccountName: my-operator
          rules:
            - apiGroups: ["example.com"]
              resources: ["databases"]
              verbs: ["*"]
  customresourcedefinitions:
    owned:
      - name: databases.example.com
        version: v1
        kind: Database
```

OLM enables marketplace-like operator discovery and lifecycle management but requires packaging via OLM bundle format.

## Operator Maturity Levels

The Operator Framework defines maturity levels reflecting operator sophistication:

| Level        | Capabilities                                                             | Example                    |
|--------------|--------------------------------------------------------------------------|----------------------------|
| **1. Basic Install** | Deploys operation without UI or upgrade logic; handles single instances | Simple stateless sidecar |
| **2. Seamless Upgrades** | Handles version upgrades, migration; multiple instances                | Database with schema migra  |
| **3. Full Lifecycle** | Backups, restores, failover, scaling, monitoring integration           | Production database ops    |
| **4. Deep Insights** | OLM integration, metrics, helm export, rich dashboards                 | Enterprise data platform   |
| **5. Auto Pilot** | Machine learning, predictive scaling, self-healing, cost optimization  | Advanced managed services  |

Most production operators target levels 2-3.

## Summary

Operators extend Kubernetes' declarative model to domain-specific systems. They combine CRDs (schema), controllers (reconciliation logic), and webhooks (validation/mutation) to automate operational tasks. Controller-runtime and Kubebuilder reduce boilerplate; Operator-SDK adds bundling and marketplace integration. Effective operators are idempotent, use condition reporting for status, and employ finalizers for safe cleanup. Maturity levels reflect operational sophistication—production operators typically reach level 2-3.