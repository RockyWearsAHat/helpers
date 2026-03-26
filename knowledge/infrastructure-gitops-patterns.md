# GitOps Patterns — Pull-Based Deployment, Drift Detection, Multi-Environment Promotion

## Overview

GitOps extends declarative infrastructure principles with continuous reconciliation: Git becomes the source of truth, and automated systems detect and repair divergence between Git state and runtime state. Two architectural models dominate: **pull-based** (cluster pulls from Git via controllers) and **push-based** (external system pushes to cluster). Each trades off safety, scalability, and operational complexity differently.

## Deployment Models

### Pull-Based Deployment (Cluster-Initiated)

The cluster continuously polls Git and reconciles desired state. Control plane runs entirely inside the cluster.

**Architecture:**
1. Git repository contains all desired infrastructure and application state
2. Reconciliation controller inside cluster (ArgoCD, Flux CD) polls Git at intervals or watches webhooks
3. Controller compares Git state with actual cluster state
4. On divergence, controller applies changes to bring cluster into sync

**Advantages:**
- **Zero external credentials**: Cluster never exposes credentials to external CI/CD systems; reduces blast radius of compromised secrets
- **Multi-cluster simplified**: Each cluster independently pulls desired state; no orchestration layer needed
- **Failure isolation**: Failed deployments don't block other clusters
- **Familiar environment**: Reconciliation logic runs in cluster; uses standard Kubernetes patterns

**Disadvantages:**
- **Higher latency**: Default poll interval 5-15 minutes; new changes don't apply instantly
- **Eventual consistency**: Temporary divergence between Git and runtime allowed
- **Drift window**: Manual hotfixes have time to propagate before being reverted
- **Scaling many clusters**: Requires GitHub webhooks or polling across all clusters

**Tools:** Flux CD, ArgoCD (pull mode), Weave GitOps

### Push-Based Deployment (External-Initiated)

External CI/CD system (GitHub Actions, GitLab CI, Jenkins) detects changes and pushes directly to cluster.

**Architecture:**
1. CI/CD pipeline detected Git push
2. Pipeline pushes changes directly to cluster API (via kubectl, Helm, Kustomize)
3. Cluster accepts and applies changes immediately
4. No controller reconciles; Git and runtime assumed in sync

**Advantages:**
- **Instant feedback**: Changes apply within seconds; tight feedback loop
- **Familiar paradigm**: Extends traditional CI/CD workflows
- **No new controllers**: Uses existing CD tools

**Disadvantages:**
- **Credential exposure**: Cluster API credentials live in CI/CD system; compromise = cluster compromise
- **Single point of failure**: CI/CD system down = deployments blocked
- **Drift undetected**: Manual changes on cluster remain undetected; Git becomes stale
- **Multi-cluster complexity**: Each cluster needs its own CI/CD trigger or shared credentials

## Hybrid Approach: Push + Pull

Many teams use **push for deployment, pull for reconciliation**:
1. CI/CD pipeline pushes change to Git
2. Pull-based controller detects Git change and reconciles to cluster
3. Reconciliation controller detects manual drift and reports it

Benefit: Instant feedback + continuous drift detection and repair.

## Drift Detection and Reconciliation

**Drift** = divergence between Git state (desired) and cluster state (actual). Causes include:
- Manual `kubectl apply` or ClickOps on cluster
- Operator or controller logic that modifies state outside Git workflow
- Failed rollout (cluster unable to reconcile due to external constraints)
- Stale controller that hasn't checked Git recently

### Detection Strategies

**Polling-based**: Controller periodically compares Git manifest with cluster state. At interval $T$, controller queries cluster, fetches Git, computes diff:

$$\text{drift} = \text{cluster state} - \text{git state}$$

If drift ≠ empty, reconciliation triggers. Trade-off: smaller $T$ detects drift faster but increases API load and network bandwidth.

**Webhook-triggered**: External system (GitHub, cluster event stream) notifies controller of changes. Tighter loop but more infrastructure.

**Three-way merge** (Kubernetes-native): Controller compares three versions:
1. Previous manifest (from last apply)
2. Current Git manifest
3. Current cluster state

Determines whether cluster change was user-made or previous apply's result. More sophisticated; used in kubectl and Helm.

### Reconciliation Strategies

**Overwrite**: If drift detected, reapply Git manifest unconditionally. Simplest; loses cluster-only state (annotations, status fields).

**Patch**: Apply only the fields that differ. Preserves cluster metadata and status.

**Manual approval**: Report drift to operator; require explicit approval to reconcile. Safe but not fully "continuous"; introduces latency.

**Conflict resolution**: If cluster state has local changes, keep them and report to operator. Used in GitOps when cluster has owner-managed state.

## Multi-Environment Promotion Patterns

### Environment Definition

Environments = distinct configurations for different deployment stages (dev, staging, production) or regions. Each environment has:
- Different resource limits (prod: more replicas than dev)
- Different secrets and configuration (prod: different database credentials)
- Different ingress hosts or routing
- Different update frequency or canary thresholds

### Kustomize: Overlay-Based Promotion

**Structure:**
```
base/           # Common manifests
  deployment.yaml
  config.yaml
overlays/       # Per-environment customizations
  dev/
    kustomization.yaml    # patches for dev config
  staging/
    kustomization.yaml
  prod/
    kustomization.yaml
```

**Promotion flow**: Build manifests from base + overlay for target environment. Git history shows different overlays applied per environment. Example: promoting to prod applies `prod/kustomization.yaml` patches.

**Advantage:** Single source of truth in base; variations expressed as deltas. Easy to review what differs between environments.

### Helm: Chart Versioning + Values

**Structure:**
```
values.yaml              # Default (dev) values
values-staging.yaml      # Staging overrides
values-prod.yaml         # Production overrides
Chart.yaml
templates/
```

**Promotion flow**: Deploy same chart with different `values-{env}.yaml`. Version bumps in `Chart.yaml` trigger upgrades across environments sequentially.

**Advantage:** Templating language (Go templates) allows conditional logic. Chart versioning ensures coordinated rollouts.

### GitOps Promotion: Environment Branching

Different Git branches for each environment:
```
main (dev, automatically deployed)
staging-branch (approval required, deployed to staging)
production-branch (approval required, deployed to prod)
```

**Promotion**: Merge from main → staging-branch → production-branch via pull requests. Git history = deployment audit trail. Example: bug fix PR merges to main, auto-deploys to dev, then cherry-picked to staging for testing.

**Advantage:** Git workflow = deployment workflow. Easy to review promotion gates (PR reviews = deployment approval).

**Disadvantage:** Branch management complexity for many environments.

### Progressive Delivery Integration

GitOps synergizes with **canary deployments**: Git-controlled manifest specifies canary weight (5% traffic → new version). Observability platform (Prometheus, DataDog) monitors canary health. On success, increment weight until 100%; on failure, fast rollback via Git revert.

Example workflow:
1. New version committed to Git
2. GitOps controller deploys canary (5% traffic)
3. After 5 min, if error rate < 1%, increment to 50%
4. If error rate > 1%, GitOps rolls back by reverting manifest

## Secrets in GitOps

Git should never contain plaintext secrets. Four approaches:

### 1. ExternalSecrets Operator (ESO)

GitOps manifests reference external secret store (Vault, AWS Secrets Manager). Operator fetches at runtime.

**Manifest:**
```yaml
apiVersion: external-secrets.io/v1beta1
kind: SecretStore
metadata:
  name: vault-backend
spec:
  provider:
    vault:
      auth:
        kubernetes: {}
      path: "secret"
      server: "https://vault.example.com"
---
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: app-secret
spec:
  secretStoreRef:
    name: vault-backend
  target:
    name: app-secret
  data:
  - secretKey: password
    remoteRef:
      key: app-password
```

**Advantage:** No secrets in Git; runtime fetch from authoritative source; supports dynamic rotation.

**Disadvantage:** External service dependency; adds latency.

### 2. Sealed Secrets (Kubeseal)

Secrets encrypted with cluster-specific public key; GitOps controller (sealing controller) decrypts.

**Workflow:**
```bash
echo -n password | kubectl create secret generic app-secret --from-file=/dev/stdin --dry-run=client -o yaml | kubeseal > sealed-secret.yaml
```

**Result:** YAML contains encrypted secret; safe to commit to Git. Only this cluster's controller can decrypt (private key stays in cluster).

**Advantage:** Secrets in Git (encrypted); no external service; fast (decrypt in-cluster).

**Disadvantage:** Key rotation manual; can't easily share secrets across clusters; decryption logic tightly coupled to Kubernetes.

### 3. Secrets Operations (SOPS)

CLI tool encrypts specific fields in YAML with KMS (AWS, GCP, Azure) or GPG. Decrypt locally during apply.

**Workflow:**
```bash
sops -e values.yaml          # Encrypt in-place
git add/commit values.yaml   # Git stores encrypted
sops -d values.yaml | helm   # Decrypt on apply
```

**Advantage:** Field-level encryption (only secrets encrypted, rest readable); multi-key support; works outside Kubernetes.

**Disadvantage:** Manual decryption step; requires KMS or GPG key access; adds operational overhead.

### 4. HashiCorp Vault + Agent Templating

Vault stores all secrets. Vault Agent sidecar injects secrets into pods at startup.

**GitOps role:** Manifests reference Vault; Agent handles fetch + injection. No secrets in Git or Kubernetes.

**Advantage:** Centralized secret management; dynamic rotation; fine-grained RBAC.

**Disadvantage:** Vault operational overhead; external service dependency.

## Repository Structure Patterns

### Mono-Repo: Single Repository, All Environments

```
repo/
  base/
    app/
      deployment.yaml
      service.yaml
  overlays/
    dev/
      kustomization.yaml
    prod/
      kustomization.yaml
  apps/
    payment-service/
      base/
      overlays/
    user-service/
      base/
      overlays/
```

**Advantages:** Single Git history; coordinated changes; easier to find resources.

**Disadvantages:** Blast radius high (one bad commit affects all apps/envs); harder to delegate (all teams need access to single repo); slower Git operations at scale.

### Multi-Repo: Per-App or Per-Environment Repositories

```
payment-service-repo/
  base/, overlays/
user-service-repo/
  base/, overlays/
platform-config-repo/
  base/, overlays/  (shared configs)
```

**Advantages:** Independent deployment cadence per app; fine-grained access control; smaller repos (faster Git).

**Disadvantages:** Coordinating changes across repos harder; "broken main" risk if dependency changes; more operational complexity (multiple Git webhooks, multiple controllers).

**Middle ground:** One repo per team/domain, multiple apps per repo.

## Reconciliation Timing and Frequency

### Cold Start (Initial Deployment)

On controller startup, reconcile immediately to ensure desired state is applied. Creates load spike.

### Steady State

After initial sync, controller enters:
- **Full reconciliation**: Every $T$ minutes (default 5-10), compare all objects
- **Smart reconciliation**: Watch for Git changes via webhook; only reconcile affected objects
- **Exponential backoff**: On repeated failures, increase retry interval to avoid thundering herd

### Trade-Offs

- **Small $T$ (1 min)**: Fast drift detection, high API load, expensive
- **Large $T$ (30 min)**: Manual changes have time window before rollback, lower API load

Recommendation: Web hook-driven + smart reconciliation + 10 min full reconciliation fallback.

## Failure Modes

**Git branch deleted**: Controller loses desired state reference; falls back to previous sync or errors.

**Manifest syntax invalid**: Controller can't parse; skips, reports error.

**Insufficient cluster resources**: Controller can't apply due to node capacity; retries until resources available.

**Network partition**: Git unreachable; controller uses last cached manifests (safe). Restores on reconnect.

**Concurrent modification**: User modifies via kubectl while controller reconciles. Last write wins (typically controller, since it runs more frequently).

## See Also

[devops-gitops.md](devops-gitops.md), [devops-argocd.md](devops-argocd.md), [devops-secrets-rotation.md](devops-secrets-rotation.md), [architecture-event-driven.md](architecture-event-driven.md)