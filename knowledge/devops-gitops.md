# GitOps — Declarative Infrastructure as Source of Truth

GitOps applies version control principles to infrastructure and application deployment: Git becomes the single source of truth, and automated systems continuously reconcile desired state (in Git) with actual state (in production). Rather than operators manually applying changes, changes flow through Git, code review, and CI/CD pipelines.

## Core Principles

### 1. Declarative

The entire desired system state is described declaratively in Git repositories — not imperative step-by-step instructions. Configuration manifests (Kubernetes YAML, Terraform, Helm charts) describe *what* the system should be, not *how* to get there.

**Why:** Declarative descriptions enable idempotent reconciliation. Applying the same manifest multiple times produces the same result, regardless of current state.

### 2. Versioned and Immutable

All infrastructure and application definitions live in Git. Every change is:
- Committed with history
- Reversible (git revert)
- Auditable (who changed what, when)
- Diff-able (what changed)

Git becomes the audit log for all production changes.

### 3. Automated Continuous Reconciliation

A control plane watches both Git and the production environment. When they diverge, the system automatically brings production back into sync with Git. This prevents *drift* — unintended configuration changes applied directly to production (hotfixes, manual edits) that are not reflected in Git.

**Reconciliation window:** Tools vary. Flux uses periodic polling (default ~10 minutes). ArgoCD supports both polling and Webhook-driven event-based reconciliation.

### 4. Pull-Based Deployment

The control plane *pulls* desired state from Git and applies it to the cluster (as opposed to push-based CI/CD, where external systems push changes). Pull-based deployments are generally safer:
- The cluster never exposes credentials to external systems
- Deployment logic runs inside the cluster (familiar environment)
- Easier to scale across multiple clusters (each cluster independently pulls)

## Deployment Models: Flux vs ArgoCD

Both are GitOps operators for Kubernetes, but with different architectural philosophies.

### Flux CD (Distributed Reconciliation)

- **Architecture:** Toolkit of CLI tools and controllers. Each controller handles one concern (source sync, image updates, kustomization, helm release)
- **Scope:** Reconciliation logic runs inside each cluster. Git repository structure is per-cluster (or shared with environment branching)
- **Model:** Modular, composable. Use only the controllers you need
- **Configuration:** CRDs (FluxCD resources); heavy use of Git repo for configuration versioning
- **Control plane:** None. Decentralized
- **Drift detection:** Periodic polling (by default ~10 minutes)

**Advantages:** No central point of failure. Easier multi-cluster setup (each cluster self-reconciles). Lightweight.

**Disadvantages:** Harder to get a cross-cluster view of state. Reconciliation happens independently on each cluster.

### ArgoCD (Centralized Control Plane)

- **Architecture:** Central ArgoCD server manages applications. Agents deployed to clusters for pull-based reconciliation
- **Scope:** Single Git repository (or multiple repos) describes all applications across environments/clusters
- **Model:** Application-centric. Defines what "Application" objects contain
- **Configuration:** ArgoCD manifests in Git; UI console for visualization
- **Control plane:** ArgoCD server (web UI, API, RBAC)
- **Drift detection:** Event-driven (webhook) or polling; can reconcile immediately or on schedule

**Advantages:** Single pane of glass across clusters. Easier to visualize and manage state. UI-first experience. Sophisticated RBAC.

**Disadvantages:** Central control plane is a single point of failure. Scaling to many clusters requires cluster registration.

**Hybrid trend:** Some teams run Flux for cluster-level reconciliation and ArgoCD for higher-level application orchestration across clusters.

## Drift Detection and State Reconciliation

Drift occurs when production state diverges from Git:
- Manual kubectl apply by an operator
- Out-of-band changes (e.g., someone edits a ConfigMap directly)
- External system modifying cluster resources
- Cluster autoscaler adding nodes

**Drift strategies:**
- **Periodic polling:** Controller reconciles every N minutes (default 10 for Flux, variable for Argo). Simple, resource-friendly
- **Event-driven (Webhook):** Git repository triggers a webhook when commits are pushed. Reconciliation happens immediately. Requires cluster to be reachable from Git provider (security tradeoff)
- **Hybrid:** Polling as fallback, webhooks for fast response

**Automatic remediation:** Most tools automatically reapply the desired state when drift is detected, bringing production back into sync.

**Manual holds:** Both Flux and ArgoCD allow suspending reconciliation to perform manual interventions without constant re-reconciliation.

## Multi-Cluster Patterns

As organizations scale beyond one cluster, GitOps faces practical challenges:

### Single Git Repository, Multiple Clusters

All clusters share one Git repository. Organize by:
- **Directories:** `clusters/prod-us-east/`, `clusters/prod-eu-west/`, etc. Each directory contains kustomizations or Helm releases for that cluster
- **Branches:** `main` for production replicas, `staging`, `dev` branches for lower environments
- **Git tags:** Tag a release and each cluster syncs when it's ready

**Limitation:** Large monolithic repo causes git clones to slow down, large manifests take time to apply, and RBAC concerns (one secret per cluster? Encrypted per environment?).

### Repository-Per-Cluster

Each cluster has its own Git repository. Control plane (ArgoCD, Flux) aggregates state across repos.

**Advantage:** Smaller repos, better team isolation, easier to rotate secrets.

**Disadvantage:** Cross-cluster consistency harder to enforce. Duplication of common patterns.

### Repository-Per-Team with GitOps Aggregator

Teams maintain their own repositories. Central team runs ArgoCD or Flux to pull and synthesize all team repos into a unified deployment.

**Advantage:** Team autonomy, clear ownership.

**Disadvantage:** Complexity. Central aggregator must handle conflicts, version mismatches, and rollback dependencies.

## Secret Management in GitOps

Storing secrets in Git violates fundamental security principles. Common approaches:

### 1. Encryption at Rest in Git

Tools like SOPS (Secrets Operations) or git-crypt encrypt secrets inside Git. Only clusters with proper keys can decrypt. Drawback: keys must live somewhere, creating bootstrap problems.

### 2. External Secrets

Operator (External Secrets Operator) runs in cluster and pulls secrets from external systems (AWS Secrets Manager, HashiCorp Vault, Azure Key Vault) into Kubernetes Secret objects. Git contains references, not values.

**Advantage:** Secrets never in Git. Rotation handled by external system.

**Disadvantage:** Extra operational dependency. Requires network access to external system.

### 3. Sealed Secrets

Kubernetes operator asymmetrically encrypts secrets. Only the cluster can decrypt. Git stores encrypted values.

**Advantage:** Self-contained per cluster.

**Disadvantage:** Per-cluster keys, harder to share across teams.

## Git as Source of Truth: Limitations

GitOps assumes Git is the source of truth. But this breaks down in practice:

### 1. Infrastructure State Managed Elsewhere

If cloud resources (databases, networking) are managed by Terraform in a separate repo or CI/CD, application manifests in Git don't capture full state. Multiple sources of truth compete.

### 2. Runtime Generated State

Observability tools (Prometheus rules), generated configs from external systems, or dynamic application behavior may not be version-controlled. Logs, metrics, and events live outside Git.

### 3. Manual Interventions Under Pressure

On-call engineers may hotfix production directly (kubectl edit, manual configuration change) under incident conditions, bypassing Git. GitOps reconciliation either reverts the fix (breaking incident response) or allows drift (violating the principle).

**Practical resolution:** Teams embed critical hotfixes into emergency procedures and backport them to Git post-incident.

### 4. Coordination Across Repos

Multi-repo GitOps requires coordinating changes across repositories (e.g., update app version in app repo, update Helm values in config repo). Atomic multi-repo commits don't exist. Failures mid-change leave inconsistent state.

### 5. Secrets and Sensitive Configuration

As noted above, secrets don't truly live in Git (they're encrypted or referenced). This undermines "Git is the source of truth" for security-sensitive decisions.

## Integration with Progressive Delivery

GitOps pairs naturally with progressive delivery (canary, blue-green, feature flags). ArgoCD and Flux plugins support:
- **Flagger:** Automated canary rollout and rollback based on metrics
- **Argo Rollouts:** Progressive delivery strategies integrated with GitOps sync

Git stores the rollout strategy (percentage, duration, success criteria). Deployment tools execute according to the manifest.

## When GitOps Works Well

- Teams with strong Git discipline (frequent commits, code review culture)
- Kubernetes-native environments with a clear declarative model
- Organizations valuing audit trails and reversibility
- Multi-cluster deployments where pull-based sync reduces operational burden
- Environments where infrastructure and application configs change together

## When GitOps is Challenging

- Non-declarative systems (legacy monoliths, databases requiring migrations)
- High-velocity organizations where Git review processes feel like bottlenecks
- Teams without strong DevOps tooling maturity
- Systems with significant runtime state or external dependencies
- Emergency situations requiring immediate changes without Git review

## Observability in GitOps

Since Git becomes the deployment authority, observability must answer:
- Is reconciliation working? (last sync time, sync status)
- Is actual state matching desired state? (drift detection metrics)
- What changed? (Git commit history)
- Who approved the change? (Git commit author, PR reviews)

Tools like Flux and ArgoCD expose metrics on reconciliation frequency, success/failure rates, and resource health.

## See Also

- Continuous delivery and deployment models: [devops-cicd.md](devops-cicd.md)
- Progressive delivery strategies: [progressive-delivery.md](progressive-delivery.md)
- Kubernetes fundamentals: [devops-kubernetes.md](devops-kubernetes.md)
- Secret management patterns: [security-secrets-management.md](security-secrets-management.md)
- Infrastructure as Code (Terraform, Helm tools): [devops-helm.md](devops-helm.md), [devops-terraform.md](devops-terraform.md)