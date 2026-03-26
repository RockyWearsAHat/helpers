# Kubernetes Security: RBAC, Network Policies, Pod Security Standards, and Supply Chain

## RBAC: Role-Based Access Control and Least Privilege

RBAC controls who can perform which actions on which resources. Kubernetes RBAC has four core objects: roles, bindings, service accounts, cluster roles.

### Core Model: Role + RoleBinding + ServiceAccount

```yaml
# ServiceAccount: identity for pods
apiVersion: v1
kind: ServiceAccount
metadata:
  name: app-sa
  namespace: production

---

# Role: permissions (verbs on resources)
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: app-reader
  namespace: production
rules:
- apiGroups: [""]
  resources: ["secrets"]
  verbs: ["get", "list"]
  resourceNames: ["database-credentials"]  # Optional: restrict to specific secrets

- apiGroups: ["apps"]
  resources: ["statefulsets"]
  verbs: ["get", "watch"]

---

# RoleBinding: attach role to service account
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: app-reader-binding
  namespace: production
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: app-reader
subjects:
- kind: ServiceAccount
  name: app-sa
  namespace: production

---

# Pod using service account
apiVersion: v1
kind: Pod
metadata:
  name: app-pod
  namespace: production
spec:
  serviceAccountName: app-sa
  containers:
  - name: app
    image: myapp:latest
    volumeMounts:
    - name: token
      mountPath: /var/run/secrets/kubernetes.io/serviceaccount
      readOnly: true
  volumes:
  - name: token
    projected:
      sources:
      - serviceAccountToken:
          path: token
          expirationSeconds: 3600
```

**Verbs:** `get`, `list`, `watch`, `create`, `update`, `patch`, `delete`, `deletecollection`, `exec`, `log`, `port-forward`, `proxy`, `*` (all).

### ClusterRole: Cluster-Wide Permissions

For resources that don't have namespace scope (nodes, ClusterRoles, PersistentVolumes):

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: node-reader
rules:
- apiGroups: [""]
  resources: ["nodes"]
  verbs: ["get", "list"]
- apiGroups: [""]
  resources: ["nodes/metrics"]
  verbs: ["get"]

---

apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: node-reader-binding
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: node-reader
subjects:
- kind: ServiceAccount
  name: monitoring-sa
  namespace: monitoring
```

### Built-in Roles: Useful Defaults

| Role                | Use                                                   |
| ------------------- | ----------------------------------------------------- |
| `view`              | Read access to most resources (no secrets, no RBAC)   |
| `edit`              | Create/edit workloads, but not RBAC or resource quota |
| `admin`             | Full admin in a namespace; cannot modify RBAC         |
| `cluster-admin`     | Full cluster admin (all verbs on all resources)       |
| `system:aggregated-metrics-reader` | Metrics server (node metrics)         |

**Anti-pattern:** Don't use `cluster-admin` for applications. Always use least-privilege roles.

### Audit RBAC Configuration

```bash
# Check what permissions a service account has
kubectl auth can-i list pods --as=system:serviceaccount:production:app-sa --namespace=production
# Yes

kubectl auth can-i delete nodes --as=system:serviceaccount:production:app-sa
# No: not allowed

# List all permissions for an SA
kubectl describe role app-reader -n production
```

---

## Network Policies: Segmentation and Traffic Control

By default, all pods can communicate with all other pods. Network policies restrict traffic to explicit allow-lists.

### Basic Policy: Deny All Ingress, Allow Specific

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: default-deny-ingress
  namespace: production
spec:
  podSelector: {}  # Applies to all pods in namespace
  policyTypes:
  - Ingress
  # Ingress rules empty = deny all ingress traffic

---

# Allow ingress to app pods from load balancer
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-from-lb
  namespace: production
spec:
  podSelector:
    matchLabels:
      app: myapp
      tier: frontend
  policyTypes:
  - Ingress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress-nginx
    ports:
    - protocol: TCP
      port: 8080
```

### Multi-Tier Policy: Frontend → Backend → Database

```yaml
# Database pods: allow only from backend
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: postgres-ingress
  namespace: production
spec:
  podSelector:
    matchLabels:
      tier: database
  policyTypes:
  - Ingress
  ingress:
  - from:
    - podSelector:
        matchLabels:
          tier: backend
    ports:
    - protocol: TCP
      port: 5432

---

# Backend pods: deny egress to external internet (only allow to DB)
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: backend-egress
  namespace: production
spec:
  podSelector:
    matchLabels:
      tier: backend
  policyTypes:
  - Egress
  egress:
  - to:
    - podSelector:
        matchLabels:
          tier: database
    ports:
    - protocol: TCP
      port: 5432
  - to:  # Allow DNS
    - namespaceSelector: {}
    ports:
    - protocol: UDP
      port: 53
```

### CNI Plugins Supporting NetworkPolicy

Network Policies are only enforced by network plugins (CNI) that support them:

| CNI       | Support            | Notes                                 |
| --------- | ------------------ | ------------------------------------- |
| Calico    | Full               | eBPF dataplane; most feature-complete |
| Cilium    | Full + advanced    | eBPF; load balancing, observability   |
| Weave     | Full               | Encrypted tunnels                     |
| Flannel   | No                 | Basic overlay network only            |
| kubenet   | No                 | Cloud provider networking             |

**Test if NetworkPolicy is enforced:**

```bash
# Create deny-all policy, test traffic
kubectl run -it --rm client --image=alpine --restart=Never -- \
  sh -c "wget -O- http://myapp:8080 --timeout=5"
# If timeout, policy is enforced; if connects, policy not enforced (check CNI plugin)
```

---

## Pod Security Standards (PSS) and Admission Controllers

Pod Security Standards define three levels: Restricted, Baseline, Unrestricted.

### Restricted: Hardened Pods (Recommended)

```yaml
# Enforcement at namespace level
apiVersion: v1
kind: Namespace
metadata:
  name: production
  labels:
    pod-security.kubernetes.io/enforce: restricted
    pod-security.kubernetes.io/enforce-version: v1.27
```

**Restricted pods require:**
- Run as non-root user
- Set `runAsNonRoot: true`
- No `privileged` containers
- Drop all capabilities (CAP_DROP=ALL)
- Read-only root filesystem
- No hostNetwork, hostPID, hostIPC
- Seccomp profile (not unconfined)

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: app
  namespace: production
spec:
  securityContext:
    runAsNonRoot: true
    runAsUser: 1000
    fsGroup: 3000
    seccompProfile:
      type: RuntimeDefault
  containers:
  - name: app
    image: myapp:latest
    securityContext:
      allowPrivilegeEscalation: false
      capabilities:
        drop:
        - ALL
      readOnlyRootFilesystem: true
    volumeMounts:
    - name: tmp
      mountPath: /tmp
  volumes:
  - name: tmp
    emptyDir: {}
```

### Baseline: Minimal Security (Legacy Compatibility)

Allows most pods; only prohibits dangerous practices (privileged, hostPath, root).

```yaml
# Enforcement at namespace
labels:
  pod-security.kubernetes.io/enforce: baseline
```

### Audit Mode: Warn Without Enforcement

```yaml
labels:
  pod-security.kubernetes.io/audit: restricted
  pod-security.kubernetes.io/warn: restricted
```

Warnings appear in audit logs but pods are not rejected.

---

## Secrets Management: External Secrets and Vault Integration

### Built-in Kubernetes Secrets: Limited Security

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: database-credentials
  namespace: production
type: Opaque
data:
  password: cGFzc3dvcmQxMjM=  # base64 encoded (NOT encrypted by default!)
  username: YWRtaW4=
```

**Problems:**
- Data is base64-encoded, not encrypted
- Secrets are stored in etcd; must enable encryption-at-rest
- No audit trail; any RBAC-permitted user can read
- No automatic rotation
- No integration with external secret stores (HashiCorp Vault, AWS Secrets Manager, GCP Secret Manager)

### External-Secrets Operator: Bridge to External Store

```yaml
# Install: helm repo add external-secrets https://charts.external-secrets.io

---

# SecretStore: connection to vault/AWS Secrets Manager/etc
apiVersion: external-secrets.io/v1beta1
kind: SecretStore
metadata:
  name: vault-store
  namespace: production
spec:
  provider:
    vault:
      server: "http://vault.vault:8200"
      path: "secret"
      auth:
        kubernetes:
          mountPath: "kubernetes"
          role: "myapp"

---

# ExternalSecret: fetch data from SecretStore → create K8s Secret
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: database-credentials
  namespace: production
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: vault-store
    kind: SecretStore
  target:
    name: database-credentials  # K8s Secret name
    creationPolicy: Owner
  data:
  - secretKey: username
    remoteRef:
      key: database/prod/username
  - secretKey: password
    remoteRef:
      key: database/prod/password
```

**Flow:**
1. ExternalSecret controller polls Vault every 1h
2. Fetches current secrets
3. Creates/updates Kubernetes Secret
4. Pod mounts secret as volume

**Advantages:**
- Single source of truth (Vault)
- Automatic rotation without pod restart
- Audit trail in Vault
- No long-lived credentials in etcd

### Vault Agent Injector: Sidecar Pattern

```yaml
# Vault Agent annotations on pod
apiVersion: v1
kind: Pod
metadata:
  annotations:
    vault.hashicorp.com/agent-inject: "true"
    vault.hashicorp.com/role: "myapp"
    vault.hashicorp.com/agent-inject-secret-db: "secret/database/prod"
    vault.hashicorp.com/agent-inject-template-db: |
      {{- with secret "secret/database/prod" -}}
      export DB_USER="{{ .Data.data.username }}"
      export DB_PASS="{{ .Data.data.password }}"
      {{- end }}
spec:
  containers:
  - name: app
    image: myapp:latest
    volumeMounts:
    - name: vault
      mountPath: /vault/secrets
    command: ["sh", "-c", ". /vault/secrets/db && node app.js"]
```

Vault Agent sidecar:
1. Authenticates to Vault (using pod's service account)
2. Fetches secret
3. Renders template
4. Writes to `/vault/secrets/` (shared volume)
5. Pod reads secret from volume at startup

---

## Admission Controllers: Policy Enforcement at API Request Time

Admission controllers intercept API requests before resources are persisted. Used for:
- Enforcing Pod Security Standards
- Validating resource compliance
- Mutating requests (injecting defaults, modifying specs)

### Built-in Controllers

Important ones:

| Controller                | Purpose                                                |
| ------------------------- | ------------------------------------------------------ |
| `PodSecurityPolicy`       | Deprecated; use PSS instead                            |
| `ResourceQuota`           | Enforce namespace CPU/memory/pod limits                |
| `NetworkPolicyProvider`   | NetworkPolicy enforcement (depends on CNI)             |
| `ServiceAccount`          | Auto-attach service account tokens                     |
| `ValidatingWebhookConfiguration` | Custom validation logic (webhook to external service) |
| `MutatingWebhookConfiguration`   | Custom mutation logic (modify resource before creation) |

### Custom Validation Webhook: Enforce Image Registry

```yaml
apiVersion: admissionregistration.k8s.io/v1
kind: ValidatingWebhookConfiguration
metadata:
  name: image-registry-policy
webhooks:
- name: registry.example.com
  clientConfig:
    service:
      name: image-policy-webhook
      namespace: admission-webhooks
      path: "/validate"
    caBundle: LS0tLS...  # Base64-encoded CA cert
  rules:
  - operations: ["CREATE", "UPDATE"]
    apiGroups: [""]
    apiVersions: ["v1"]
    resources: ["pods"]
  failurePolicy: Fail  # Reject pod if webhook fails
  sideEffects: None
  admissionReviewVersions: ["v1"]
  clientConfig:
    url: "https://webhook-service.admission-webhooks:8443/validate"
```

Webhook server validates: image must come from `gcr.io/myorg/*` or `docker.io/myorg/*`.

```python
@app.post("/validate")
def validate_pod():
    review = request.json
    uid = review["request"]["uid"]
    pod = review["request"]["object"]
    
    allowed = True
    for container in pod["spec"]["containers"]:
        image = container["image"]
        if not (image.startswith("gcr.io/myorg/") or image.startswith("docker.io/myorg/")):
            allowed = False
            break
    
    return {
        "apiVersion": "admission.k8s.io/v1",
        "kind": "AdmissionReview",
        "response": {
            "uid": uid,
            "allowed": allowed,
            "status": {
                "message": "Image must be from approved registry"
            }
        }
    }
```

### Mutating Webhook: Inject Default Limits

```yaml
apiVersion: admissionregistration.k8s.io/v1
kind: MutatingWebhookConfiguration
metadata:
  name: inject-resource-limits
webhooks:
- name: limits.example.com
  clientConfig:
    url: "https://webhook-service.admission-webhooks:8443/mutate"
  rules:
  - operations: ["CREATE"]
    apiGroups: [""]
    apiVersions: ["v1"]
    resources: ["pods"]
  admissionReviewVersions: ["v1"]
  sideEffects: None
```

Mutating webhook modifies pod spec before creation (add default resource limits, inject sidecar, add labels).

---

## Audit Logging: Track All API Access

```yaml
# kube-apiserver flag: --audit-policy-file=/etc/kubernetes/audit-policy.yaml

apiVersion: audit.k8s.io/v1
kind: Policy
rules:
# Log all requests to namespaces
- level: RequestResponse
  verbs: ["create", "delete", "patch"]
  resources: ["namespaces"]
  omitStages:
  - RequestReceived

# Log pod exec (shell access)
- level: RequestResponse
  verbs: ["create"]
  resources: ["pods/exec", "pods/log"]

# Log secret access
- level: Metadata
  resources: ["secrets"]

# Catch all other requests at Metadata level
- level: Metadata
  omitStages:
  - RequestReceived
```

**Levels:**
- `None` — don't log
- `Metadata` — log request/response metadata, not body
- `Request` — log metadata + request body
- `RequestResponse` — log metadata + request/response bodies

Audit logs flow to:
- File backend: `/var/log/audit/audit.log`
- Webhook backend: sent to external log aggregator (Splunk, ELK, etc.)
- Webhook backend allows real-time alerting on suspicious activity.

---

## CIS Kubernetes Benchmark Alignment

CIS Kubernetes Benchmark defines security configuration best practices. Use compliance scanning tools to evaluate clusters.

**Key checks:**
- RBAC: serviceAccountName on all pods, no cluster-admin bindings for user accounts
- Pod Security: PSS=restricted on production namespaces
- Network Policy: default-deny policies installed
- Secrets: encryption-at-rest enabled
- Audit logging: enabled and functional
- Kubelet security: --allow-privileged=false, --protect-kernel-defaults=true

**Scanning tools:**
- `kube-bench`: Open-source CIS benchmark checker (runs locally)
- `kubesec`: Scans resource YAML for security issues
- `Polaris`: Kubernetes audit tool from FairwindsOps

```bash
# Run kube-bench
kube-bench run --targets node,policies

# Run kubesec on YAML
kubesec scan deployment.yaml
```

---

## Supply Chain Security: Image Verification, Attestation, SBOM

### Image Verification in Admission Controller

```yaml
apiVersion: admissionregistration.k8s.io/v1
kind: ValidatingWebhookConfiguration
metadata:
  name: image-verification
webhooks:
- name: verify-image-signature
  rules:
  - operations: ["CREATE", "UPDATE"]
    resources: ["pods"]
  clientConfig:
    url: "https://webhook-service.admission-webhooks:8443/verify-image"
```

Webhook verifies image signature (cosign) before pod creation.

### OPA/Gatekeeper: Policy as Code

```yaml
# Gatekeeper ConstraintTemplate
apiVersion: templates.gatekeeper.sh/v1beta1
kind: ConstraintTemplate
metadata:
  name: imagemustbesigned
spec:
  crd:
    spec:
      names:
        kind: ImageMustBeSigned
  targets:
  - target: admission.k8s.gatekeeper.sh
    rego: |
      package imagemustbesigned
      
      violation[{"msg": msg}] {
        container := input.review.object.spec.containers[_]
        image := container.image
        not image_is_signed(image)
        msg := sprintf("Image %v is not signed", [image])
      }
      
      image_is_signed(image) {
        # Webhook call to cosign verifier; returns true if signature valid
        http.send({
          "method": "POST",
          "url": "http://cosign-webhook:8080/verify",
          "body": image
        }).status_code == 200
      }
```

OPA allows fine-grained, composable policy definitions.

---

## Hardening Checklist

- [ ] RBAC: All service accounts have explicit, least-privilege roles
- [ ] NetworkPolicy: default-deny ingress/egress, explicit allow-lists
- [ ] Pod Security Standards: PSS=restricted on production namespaces
- [ ] Secrets: External-secrets operator → Vault; encryption-at-rest enabled
- [ ] Admission control: ValidatingWebhook for image registry, MutatingWebhook for defaults
- [ ] Audit logging: enabled, sent to centralized log aggregator
- [ ] CIS benchmark: kube-bench results reviewed, high-risk items remediated
- [ ] Image verification: cosign signatures required + verified at admission
- [ ] SBOM: generated and attached to images