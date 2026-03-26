# Argo CD — GitOps Continuous Delivery

## GitOps Principles

1. **Declarative** — entire desired system state described in Git
2. **Versioned and immutable** — Git as single source of truth with full audit trail
3. **Pulled automatically** — agents (Argo CD) pull desired state and reconcile
4. **Continuously reconciled** — drift detection and auto-correction

**Pull vs Push model:** Traditional CI/CD pushes changes to clusters. GitOps agents pull from Git and reconcile. The cluster never needs to expose credentials to CI — Argo CD has cluster access, CI only needs Git write access.

## Architecture

```
┌──────────┐     ┌──────────────┐     ┌───────────────┐
│   Git    │────►│   Argo CD    │────►│  K8s Cluster  │
│  Repo(s) │     │  Controller  │     │               │
└──────────┘     │  + Repo Svr  │     └───────────────┘
                 │  + API Svr   │
                 │  + Redis     │
                 └──────────────┘
```

- **API Server** — gRPC/REST API, web UI, SSO, RBAC enforcement
- **Repository Server** — clones Git repos, generates manifests (Helm, Kustomize, Jsonnet, plain YAML)
- **Application Controller** — watches desired state (Git) vs live state (cluster), reconciles
- **Redis** — caching layer for repo state and app state

## Application CRD

The core resource. Defines what to deploy, where to deploy it, and how to sync.

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: myapp
  namespace: argocd # Argo CD always manages apps in its namespace
  finalizers:
    - resources-finalizer.argocd.argoproj.io # cascade delete on app removal
spec:
  project: default

  source:
    repoURL: https://github.com/myorg/k8s-manifests.git
    targetRevision: main # branch, tag, or commit SHA
    path: apps/myapp/overlays/prod # path within repo

    # For Helm charts
    # chart: myapp
    # repoURL: https://charts.example.com
    # targetRevision: 1.2.3
    # helm:
    #   valueFiles:
    #     - values-prod.yaml
    #   parameters:
    #     - name: image.tag
    #       value: v2.0.1

  destination:
    server: https://kubernetes.default.svc # in-cluster
    namespace: production

  syncPolicy:
    automated:
      prune: true # delete resources removed from Git
      selfHeal: true # revert manual cluster changes
      allowEmpty: false # prevent accidental full deletion
    syncOptions:
      - CreateNamespace=true
      - PrunePropagationPolicy=foreground
      - PruneLast=true
      - ApplyOutOfSyncOnly=true
      - ServerSideApply=true
    retry:
      limit: 5
      backoff:
        duration: 5s
        factor: 2
        maxDuration: 3m

  ignoreDifferences:
    - group: apps
      kind: Deployment
      jsonPointers:
        - /spec/replicas # ignore HPA-managed replicas
    - group: ""
      kind: ConfigMap
      jqPathExpressions:
        - .data["generated-field"]
```

### Source Types

| Type      | Config                                          | Use Case               |
| --------- | ----------------------------------------------- | ---------------------- |
| Directory | `path:` with YAML files                         | Plain manifests        |
| Kustomize | `path:` with kustomization.yaml                 | Overlays, patches      |
| Helm      | `chart:` + `repoURL:` (repo) or `path:` (local) | Helm charts            |
| Jsonnet   | `path:` with .jsonnet files                     | Programmatic manifests |
| Plugin    | Custom config management plugin                 | Any custom tool        |

### Multi-Source Applications

```yaml
spec:
  sources:
    - repoURL: https://charts.example.com
      chart: myapp
      targetRevision: 1.2.3
      helm:
        valueFiles:
          - $values/apps/myapp/values-prod.yaml
    - repoURL: https://github.com/myorg/config.git
      targetRevision: main
      ref: values # reference name used above as $values
```

## ApplicationSets

Automate Application generation from templates. One ApplicationSet can produce hundreds of Applications.

### Git Generator

Generate apps from directory structure or files in a Git repo:

```yaml
apiVersion: argoproj.io/v1alpha1
kind: ApplicationSet
metadata:
  name: cluster-apps
  namespace: argocd
spec:
  generators:
    - git:
        repoURL: https://github.com/myorg/k8s-manifests.git
        revision: main
        directories:
          - path: apps/* # each subdirectory becomes an app
          - path: apps/excluded # exclude specific paths
            exclude: true
  template:
    metadata:
      name: "{{path.basename}}"
    spec:
      project: default
      source:
        repoURL: https://github.com/myorg/k8s-manifests.git
        targetRevision: main
        path: "{{path}}"
      destination:
        server: https://kubernetes.default.svc
        namespace: "{{path.basename}}"
```

### Cluster Generator

Deploy to all registered clusters (or a subset by label):

```yaml
spec:
  generators:
    - clusters:
        selector:
          matchLabels:
            env: production
  template:
    spec:
      destination:
        server: "{{server}}"
        namespace: myapp
```

### Matrix Generator

Combine two generators (cartesian product):

```yaml
spec:
  generators:
    - matrix:
        generators:
          - git:
              repoURL: https://github.com/myorg/manifests.git
              revision: main
              directories:
                - path: apps/*
          - clusters:
              selector:
                matchLabels:
                  env: production
```

### Pull Request Generator

Create preview environments for open PRs:

```yaml
spec:
  generators:
    - pullRequest:
        github:
          owner: myorg
          repo: myapp
          tokenRef:
            secretName: github-token
            key: token
          labels:
            - deploy-preview
  template:
    metadata:
      name: "pr-{{number}}"
    spec:
      source:
        targetRevision: "{{head_sha}}"
        path: k8s/preview
      destination:
        namespace: "pr-{{number}}"
```

## Sync Waves & Hooks

Control ordering of resource deployment within a sync operation.

```yaml
metadata:
  annotations:
    argocd.argoproj.io/sync-wave: "-1" # lower waves sync first
```

| Wave | Typical Resources               |
| ---- | ------------------------------- |
| -3   | Namespaces, CRDs                |
| -2   | RBAC (ServiceAccounts, Roles)   |
| -1   | ConfigMaps, Secrets             |
| 0    | Deployments, Services (default) |
| 1    | Ingress, NetworkPolicy          |
| 2    | Jobs, post-deploy tasks         |

### Resource Hooks

```yaml
metadata:
  annotations:
    argocd.argoproj.io/hook: PreSync # run before sync
    argocd.argoproj.io/hook-delete-policy: HookSucceeded
```

| Hook       | Timing                            |
| ---------- | --------------------------------- |
| `PreSync`  | Before sync (e.g., db migrations) |
| `Sync`     | During sync (alongside wave 0)    |
| `PostSync` | After all resources healthy       |
| `SyncFail` | On sync failure (cleanup)         |
| `Skip`     | Skip this resource during sync    |

## Multi-Cluster Management

Register external clusters:

```bash
argocd cluster add my-context --name prod-cluster
```

Clusters managed in Argo CD:

```yaml
# Secret-based cluster registration
apiVersion: v1
kind: Secret
metadata:
  name: prod-cluster
  namespace: argocd
  labels:
    argocd.argoproj.io/secret-type: cluster
stringData:
  name: prod-cluster
  server: https://prod-api.example.com
  config: |
    {
      "bearerToken": "...",
      "tlsClientConfig": {"insecure": false, "caData": "..."}
    }
```

## RBAC & Projects

### AppProjects

Namespace-like isolation for Applications:

```yaml
apiVersion: argoproj.io/v1alpha1
kind: AppProject
metadata:
  name: team-frontend
  namespace: argocd
spec:
  description: Frontend team applications
  sourceRepos:
    - "https://github.com/myorg/frontend-*"
  destinations:
    - namespace: "frontend-*"
      server: https://kubernetes.default.svc
  clusterResourceWhitelist:
    - group: ""
      kind: Namespace
  namespaceResourceBlacklist:
    - group: ""
      kind: ResourceQuota
  roles:
    - name: frontend-devs
      policies:
        - p, proj:team-frontend:frontend-devs, applications, sync, team-frontend/*, allow
        - p, proj:team-frontend:frontend-devs, applications, get, team-frontend/*, allow
      groups:
        - frontend-team # OIDC/LDAP group
```

### RBAC Policy (Casbin)

```csv
# argocd-rbac-cm ConfigMap
p, role:readonly, applications, get, */*, allow
p, role:readonly, logs, get, */*, allow
p, role:admin, applications, *, */*, allow
p, role:admin, clusters, *, *, allow
g, my-github-team, role:admin
g, alice@example.com, role:readonly
```

## Secrets Management

Argo CD syncs from Git — secrets can't be stored in plain text. Common approaches:

| Approach                  | How It Works                                       | Complexity |
| ------------------------- | -------------------------------------------------- | ---------- |
| Sealed Secrets            | Encrypt with cluster public key, commit ciphertext | Low        |
| External Secrets Operator | K8s operator fetches from Vault/AWS SM/GCP SM      | Medium     |
| SOPS + KSOPS              | Encrypt YAML values with age/KMS, decrypt at sync  | Medium     |
| Vault Agent Injector      | Sidecar injects secrets at pod startup             | High       |
| Argo CD Vault Plugin      | Plugin decrypts during manifest generation         | Medium     |

### Sealed Secrets Example

```bash
# Encrypt locally (anyone can encrypt, only cluster can decrypt)
kubeseal --format yaml < secret.yaml > sealed-secret.yaml

# Commit sealed-secret.yaml to Git
# SealedSecret controller decrypts → creates Kubernetes Secret
```

### External Secrets Operator

```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: db-credentials
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: aws-secretsmanager
    kind: ClusterSecretStore
  target:
    name: db-credentials # K8s Secret created
  data:
    - secretKey: password
      remoteRef:
        key: prod/db/password
```

## Argo Rollouts

Progressive delivery controller — extends Kubernetes Deployments with canary and blue-green strategies.

### Canary with Analysis

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Rollout
metadata:
  name: myapp
spec:
  replicas: 10
  strategy:
    canary:
      canaryService: myapp-canary
      stableService: myapp-stable
      trafficRouting:
        istio:
          virtualServices:
            - name: myapp-vsvc
              routes: [primary]
      steps:
        - setWeight: 5
        - pause: { duration: 2m }
        - analysis:
            templates:
              - templateName: success-rate
            args:
              - name: service-name
                value: myapp-canary
        - setWeight: 25
        - pause: { duration: 5m }
        - setWeight: 50
        - pause: { duration: 5m }
        - setWeight: 100
      rollbackWindow:
        revisions: 2
```

### AnalysisTemplate

```yaml
apiVersion: argoproj.io/v1alpha1
kind: AnalysisTemplate
metadata:
  name: success-rate
spec:
  args:
    - name: service-name
  metrics:
    - name: success-rate
      interval: 30s
      count: 5
      successCondition: result[0] >= 0.99
      failureLimit: 3
      provider:
        prometheus:
          address: http://prometheus:9090
          query: |
            sum(rate(http_requests_total{service="{{args.service-name}}",status=~"2.."}[2m]))
            /
            sum(rate(http_requests_total{service="{{args.service-name}}"}[2m]))
```

### Blue-Green

```yaml
spec:
  strategy:
    blueGreen:
      activeService: myapp-active
      previewService: myapp-preview
      autoPromotionEnabled: false # manual promotion
      prePromotionAnalysis:
        templates:
          - templateName: smoke-tests
      scaleDownDelaySeconds: 30
```

## Argo CD vs Flux

| Feature              | Argo CD                     | Flux v2                        |
| -------------------- | --------------------------- | ------------------------------ |
| UI                   | Rich web UI + CLI           | CLI only (Weave GitOps for UI) |
| Multi-tenancy        | AppProjects with RBAC       | Namespaced controllers         |
| Multi-cluster        | Built-in                    | Via Kustomization targeting    |
| ApplicationSets      | Native generators           | Kustomization + GitRepository  |
| Progressive delivery | Argo Rollouts               | Flagger                        |
| Notifications        | Argo Notifications          | Via Notification Controller    |
| Helm support         | Native renderer             | HelmRelease CRD                |
| SSO                  | Built-in (OIDC, LDAP, SAML) | Delegated to ingress           |
| Resource footprint   | Heavier (API server, Redis) | Lighter (toolkit controllers)  |
| Drift detection      | Real-time, visible in UI    | Reconciliation interval        |
