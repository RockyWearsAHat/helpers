# Kubernetes

## Architecture

### Control Plane

| Component                    | Role                                                                                                              |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **kube-apiserver**           | REST API gateway, all communication flows through it. Validates and persists to etcd. Horizontally scalable.      |
| **etcd**                     | Distributed key-value store. Single source of truth for cluster state. Always run odd number (3 or 5) for quorum. |
| **kube-scheduler**           | Assigns unscheduled pods to nodes. Considers resource requests, affinity, taints, topology spread.                |
| **kube-controller-manager**  | Runs reconciliation loops: Deployment, ReplicaSet, Node, Job, ServiceAccount controllers.                         |
| **cloud-controller-manager** | Cloud-specific controllers: node lifecycle, route, service (LoadBalancer), volume.                                |

### Node Components

| Component             | Role                                                                                                              |
| --------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **kubelet**           | Agent on each node. Ensures containers match PodSpec. Reports node/pod status. Manages liveness/readiness probes. |
| **kube-proxy**        | Network proxy implementing Service abstraction. iptables or IPVS mode.                                            |
| **Container runtime** | CRI-compliant runtime: containerd (standard), CRI-O. Docker (dockershim) removed in 1.24.                         |

## Pods

Smallest deployable unit. One or more containers sharing network namespace (localhost) and storage volumes.

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: web
  labels:
    app: web
spec:
  initContainers:
    - name: init-db
      image: busybox:1.36
      command: ["sh", "-c", "until nc -z db-svc 5432; do sleep 2; done"]
  containers:
    - name: app
      image: myapp:1.2.0
      ports:
        - containerPort: 8080
      resources:
        requests:
          cpu: 100m # 0.1 CPU core
          memory: 128Mi
        limits:
          cpu: 500m
          memory: 256Mi
      livenessProbe:
        httpGet:
          path: /healthz
          port: 8080
        initialDelaySeconds: 10
        periodSeconds: 15
      readinessProbe:
        httpGet:
          path: /ready
          port: 8080
        periodSeconds: 5
      startupProbe: # gates liveness/readiness until app starts
        httpGet:
          path: /healthz
          port: 8080
        failureThreshold: 30
        periodSeconds: 10
    - name: sidecar # sidecar pattern
      image: fluent-bit:2.1
      volumeMounts:
        - name: logs
          mountPath: /var/log/app
  volumes:
    - name: logs
      emptyDir: {}
```

**Resource units**: CPU in millicores (1000m = 1 core). Memory in Mi/Gi. Requests = scheduling guarantee. Limits = hard ceiling (OOMKilled if exceeded for memory, throttled for CPU).

### Pod Patterns

| Pattern            | Description                                      |
| ------------------ | ------------------------------------------------ |
| **Sidecar**        | Helper container (logging, proxy, config reload) |
| **Ambassador**     | Proxy for external service connections           |
| **Adapter**        | Transform output format (metrics, logs)          |
| **Init container** | Run-to-completion setup before main containers   |

## Deployments

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
spec:
  replicas: 3
  revisionHistoryLimit: 5
  selector:
    matchLabels:
      app: web
  strategy:
    type: RollingUpdate # or Recreate
    rollingUpdate:
      maxSurge: 1 # extra pods during update
      maxUnavailable: 0 # zero-downtime
  template:
    metadata:
      labels:
        app: web
    spec:
      containers:
        - name: app
          image: myapp:1.3.0
```

### Deployment Strategies

| Strategy          | Mechanism                                                           | Use Case                        |
| ----------------- | ------------------------------------------------------------------- | ------------------------------- |
| **RollingUpdate** | Gradual pod replacement                                             | Default, zero-downtime          |
| **Recreate**      | Kill all, then create new                                           | DB migrations, breaking changes |
| **Canary**        | Manual: two Deployments, shift traffic via Service weights or Istio | Risk-sensitive releases         |
| **Blue-Green**    | Two full environments, switch Service selector                      | Instant rollback                |

```bash
# Rollback
kubectl rollout undo deployment/web
kubectl rollout undo deployment/web --to-revision=3
kubectl rollout status deployment/web
kubectl rollout history deployment/web
```

## Services

```yaml
apiVersion: v1
kind: Service
metadata:
  name: web-svc
spec:
  type: ClusterIP # default
  selector:
    app: web
  ports:
    - port: 80 # service port
      targetPort: 8080 # container port
      protocol: TCP
```

| Type             | Scope                     | Notes                                                               |
| ---------------- | ------------------------- | ------------------------------------------------------------------- |
| **ClusterIP**    | Internal only             | Default. Cluster-internal virtual IP.                               |
| **NodePort**     | External via node IP:port | Range 30000-32767. Exposes on every node.                           |
| **LoadBalancer** | External via cloud LB     | Provisions cloud load balancer. Superset of NodePort.               |
| **ExternalName** | DNS alias                 | CNAME to external service. No proxying.                             |
| **Headless**     | DNS only (no virtual IP)  | `clusterIP: None`. Returns pod IPs directly. Used for StatefulSets. |

## Ingress

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: web-ingress
  annotations:
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  ingressClassName: nginx
  tls:
    - hosts: [app.example.com]
      secretName: app-tls
  rules:
    - host: app.example.com
      http:
        paths:
          - path: /api
            pathType: Prefix
            backend:
              service:
                name: api-svc
                port:
                  number: 80
          - path: /
            pathType: Prefix
            backend:
              service:
                name: frontend-svc
                port:
                  number: 80
```

**Gateway API** (successor to Ingress): `Gateway`, `HTTPRoute`, `GRPCRoute`. More expressive, role-oriented (infra vs app team), multi-tenant.

## ConfigMaps and Secrets

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: app-config
data:
  DATABASE_HOST: "db.internal"
  config.yaml: |
    log_level: info
    cache_ttl: 300
---
apiVersion: v1
kind: Secret
metadata:
  name: app-secrets
type: Opaque
data:
  DB_PASSWORD: cGFzc3dvcmQxMjM= # base64 encoded (NOT encrypted)
```

**Mounting**: as environment variables (`envFrom`/`env.valueFrom`), as volume files, or both. Secrets are base64-only by default — use **Sealed Secrets**, **External Secrets Operator**, or **SOPS** for actual encryption.

## Storage

```yaml
# PersistentVolumeClaim — requests storage
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: data-pvc
spec:
  accessModes: [ReadWriteOnce] # RWO, ROX, RWX
  storageClassName: gp3 # maps to StorageClass
  resources:
    requests:
      storage: 20Gi
---
# StorageClass — defines provisioner and parameters
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: gp3
provisioner: ebs.csi.aws.com
parameters:
  type: gp3
  iops: "3000"
reclaimPolicy: Delete # Delete or Retain
volumeBindingMode: WaitForFirstConsumer
allowVolumeExpansion: true
```

| Access Mode      | Abbrev | Description                          |
| ---------------- | ------ | ------------------------------------ |
| ReadWriteOnce    | RWO    | Single node read-write               |
| ReadOnlyMany     | ROX    | Multiple nodes read-only             |
| ReadWriteMany    | RWX    | Multiple nodes read-write (NFS, EFS) |
| ReadWriteOncePod | RWOP   | Single pod read-write (1.27+)        |

**CSI** (Container Storage Interface): standard for storage plugins. EBS CSI, EFS CSI, Ceph CSI, etc.

## RBAC

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  namespace: production
  name: pod-reader
rules:
  - apiGroups: [""]
    resources: ["pods", "pods/log"]
    verbs: ["get", "list", "watch"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  namespace: production
  name: read-pods
subjects:
  - kind: ServiceAccount
    name: monitoring-sa
    namespace: production
roleRef:
  kind: Role
  name: pod-reader
  apiGroup: rbac.authorization.k8s.io
```

**Role** = namespaced, **ClusterRole** = cluster-wide. **RoleBinding** binds to namespace, **ClusterRoleBinding** binds cluster-wide. Subjects: User, Group, ServiceAccount.

## Network Policies

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: api-policy
  namespace: production
spec:
  podSelector:
    matchLabels:
      app: api
  policyTypes: [Ingress, Egress]
  ingress:
    - from:
        - podSelector:
            matchLabels:
              app: frontend
        - namespaceSelector:
            matchLabels:
              env: production
      ports:
        - port: 8080
  egress:
    - to:
        - podSelector:
            matchLabels:
              app: database
      ports:
        - port: 5432
```

**Default deny all**: apply a NetworkPolicy with `podSelector: {}` and empty ingress/egress. Requires a CNI that supports NetworkPolicy (Calico, Cilium, Weave — NOT default kubenet).

## Autoscaling

```yaml
# Horizontal Pod Autoscaler
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: web-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: web
  minReplicas: 2
  maxReplicas: 20
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300 # prevent flapping
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Pods
      pods:
        metric:
          name: requests_per_second
        target:
          type: AverageValue
          averageValue: "1000"
```

**HPA**: scales replicas based on metrics. Requires metrics-server. **VPA**: adjusts resource requests/limits per pod. **KEDA**: event-driven autoscaling (queue depth, cron, external metrics).

## Scheduling

### Taints and Tolerations

```yaml
# Taint a node (repels pods)
# kubectl taint nodes gpu-node-1 gpu=true:NoSchedule

# Pod tolerates the taint
spec:
  tolerations:
    - key: "gpu"
      operator: "Equal"
      value: "true"
      effect: "NoSchedule" # NoSchedule, PreferNoSchedule, NoExecute
```

### Affinity

```yaml
spec:
  affinity:
    nodeAffinity:
      requiredDuringSchedulingIgnoredDuringExecution:
        nodeSelectorTerms:
          - matchExpressions:
              - key: topology.kubernetes.io/zone
                operator: In
                values: [us-east-1a, us-east-1b]
    podAntiAffinity:
      preferredDuringSchedulingIgnoredDuringExecution:
        - weight: 100
          podAffinityTerm:
            labelSelector:
              matchLabels:
                app: web
            topologyKey: kubernetes.io/hostname # spread across nodes
```

## CRDs and Operators

```yaml
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: databases.myorg.io
spec:
  group: myorg.io
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
                engine:
                  type: string
                  enum: [postgres, mysql]
                replicas:
                  type: integer
  scope: Namespaced
  names:
    plural: databases
    singular: database
    kind: Database
```

**Operator pattern**: CRD + controller that watches custom resources and reconciles desired state. Frameworks: Operator SDK (Go, Ansible, Helm), kubebuilder, Metacontroller.

## Helm

```bash
helm repo add bitnami https://charts.bitnami.com/bitnami
helm search repo bitnami/postgresql
helm install my-pg bitnami/postgresql \
  --namespace database --create-namespace \
  --set auth.postgresPassword=secret \
  --values custom-values.yaml
helm upgrade my-pg bitnami/postgresql -f updated-values.yaml
helm rollback my-pg 1
helm list -A
```

**Chart structure**: `Chart.yaml` (metadata), `values.yaml` (defaults), `templates/` (Go templates → K8s manifests), `charts/` (dependencies).

## Debugging Quick Reference

```bash
kubectl get pods -o wide                    # pod status + node
kubectl describe pod <name>                 # events, conditions
kubectl logs <pod> -c <container> --previous  # crashed container logs
kubectl exec -it <pod> -- /bin/sh           # shell into container
kubectl port-forward svc/web 8080:80        # local access
kubectl top pods                            # resource usage (requires metrics-server)
kubectl get events --sort-by='.lastTimestamp'
kubectl debug node/<name> -it --image=busybox  # node debugging (ephemeral container)
```
