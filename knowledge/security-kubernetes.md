# Kubernetes Security — Pod Security, RBAC, Network Policy, Image Scanning & Runtime Detection

## Overview

Kubernetes security spans the full lifecycle: admission control (what runs), enforcement (how it runs), observation (runtime behavior), and incident response. Contemporary Kubernetes security assumes that containers are not truly sandboxed; a compromised container can exploit the kernel to escape and impact the host. Layered controls (pod security policies, network isolation, runtime detection) raise the cost of lateral movement and persistence. No single control is sufficient.

---

## Pod Security Admission & Standards

### Pod Security Standards (PSS)

Pod Security Standards (formerly pod security policies) define security profiles for pod specifications: restricted, baseline, unrestricted.

**Restricted:** Strictest profile. Enforces:
- Containers run as non-root (securityContext.runAsNonRoot=true).
- No privileged containers (privileged=false).
- Kernel capabilities are dropped; only NET_BIND_SERVICE is allowed.
- Read-only root filesystem (readOnlyRootFilesystem=true).
- No hostPath mounts; no hostNetwork or hostPID.
- SELinux policy applied; AppArmor profile specified.
- No unsafe syscalls (seccomp profile set).

Most application containers pass restricted; those requiring root or kernel access fall back to baseline or unrestricted.

**Baseline:** Minimal controls. Allows:
- Containers running as root (but no privileged flag).
- Most capabilities (drops CONFIG_MODULES, SYS_BOOT, SYSLOG are excluded).
- hostPath mounts to non-sensitive paths.
- Backwards-compatible with legacy applications.

**Unrestricted:** No enforced policies (default Kubernetes behavior prior to PSS adoption).

### Enforcing PSS via Admission Controllers

Pod Security Standards are enforced via admission webhooks or built-in Pod Security admission controller. When a pod is created:

1. Admission controller evaluates the spec against the enforced standard (e.g., restricted).
2. If pod violates the standard, admission fails and the pod is rejected.
3. Failures are logged in audit logs (useful for debugging and tracking attempted violations).

**Modes:**

- `enforce` — reject violating pods.
- `audit` — allow but log to audit trail.
- `warn` — allow but warn the user.

Typical deployment: namespaces run in enforce mode with baseline or restricted. Developer namespaces might use audit mode to identify legacy apps needing refactoring.

### Common psych Gotchas

- **Ephemeral containers exemption:** Ephemeral debug containers often run as root; some policies exclude them from PSS.
- **Init containers:** Init containers run with higher privileges than app containers; policies often allow them broader permissions.
- **Volume mounts:** Mounts to /etc, /sys, /proc are powerful; restricting hostPath mounts reduces surface area.

---

## RBAC: Least Privilege Access Control

### RBAC Model

Kubernetes RBAC controls API access via Role (verb+resource+namespace) and RoleBinding (role+user/group/service account). Typical flow:

1. **ServiceAccount (identity):** Each pod has a service account (default if unspecified). Service accounts are namespaced; defaults to `default`.
2. **Role (permissions):** Defines allowed verbs (get, list, create, delete, watch) on resources (pods, secrets, services) in a namespace.
3. **RoleBinding:** Binds a role to a service account.

**Example:**

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: app-reader
  namespace: production
rules:
- apiGroups: [""]
  resources: ["pods"]
  verbs: ["get", "list"]
- apiGroups: [""]
  resources: ["configmaps"]
  resourceNames: ["app-config"]  # Can only access specific ConfigMap
  verbs: ["get"]
---
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
```

### ClusterRole & ClusterRoleBinding

ClusterRoles are cluster-scoped (apply to all namespaces); ClusterRoleBindings grant cluster-wide permissions. Use ClusterRoles for node-level access or multi-namespace permissions.

### Least-Privilege Principles

- **Narrow verbs:** Grant only get, list; avoid * (all verbs).
- **Specific resources:** Name specific resources or use resourceNames when possible.
- **Namespace isolation:** RoleBindings are namespace-local; a compromised pod can't access other namespaces.
- **Audit RBAC:** Log authorizations and denials to detect privilege escalation attempts.

### Common Misconfigurations

- **Over-permissive defaults:** Giving service accounts role binding to cluster-admin inadvertently.
- **Wildcard permissions:** Roles using `resources: ["*"]` or `verbs: ["*"]` grant excessive access.
- **Overlooking escalation:** A user who can create pods can execute arbitrary code if pods themselves have high permissions.

---

## Network Policies

Network policies restrict network traffic between pods using label selectors and CIDR rules.

**Default:** No network policy → all pods can communicate with all pods. Deploying network policies requires explicit whitelisting.

### Policy Syntax

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: deny-from-other-namespaces
  namespace: production
spec:
  podSelector: {}  # Applies to all pods in namespace
  policyTypes:
  - Ingress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: production
```

This policy allows ingress only from pods in the production namespace.

### Ingress & Egress

- **Ingress rules:** Which pods can send traffic TO pods in this namespace.
- **Egress rules:** Which pods can send traffic FROM pods in this namespace.

Egress often requires careful tuning—blocking DNS or container registry access breaks applications. Common allowances: DNS (port 53), container registries (port 443), external APIs.

### Limitations

- **No encryption enforcement:** Network policies don't mandate mTLS; traffic is in plaintext by default (though typically within the cluster). Service meshes (Istio, Linkerd) add encryption and mutual authentication.
- **No application-layer controls:** Policies operate at L3/L4; a pod with broad ingress access can still access sensitive application endpoints.
- **Bypass via hostNetwork:** Pods with hostNetwork: true bypass network policies (they use the host's network namespace directly).

---

## Image Scanning in Admission Control

### Admission Controller Model

Kubernetes admission webhooks intercept API requests (e.g., pod creation) and make policy decisions. Image scanning webhooks prevent deployment of vulnerable images.

**Flow:**

1. User submits a pod spec with an image (e.g., myapp:v1.0).
2. Admission webhook intercepts the request.
3. Webhook pulls the image manifest, scans it for known CVEs.
4. If vulnerabilities exceed a threshold (configurable), admission fails.
5. Pod creation is rejected with an error message.

### Scanning Strategy

Common approaches:

- **Docker image layer analysis:** Tools (Trivy, Clair, Grype) scan image layers for installed packages and known CVE databases.
- **SBOM (Software Bill of Materials):** Generate or retrieve SBOMs during build time; check at admission time.
- **Container registry scanning:** Scan images at push time (artifact registries like Artifactory, Harbor, ECR often support this).
- **Signed images:** Require image signatures (cosign, Notary) to verify the image comes from a trusted builder.

### Challenges

- **Performance:** Scanning large images on every admission can slow pod startup.
- **False positives:** CVE databases include low-severity or unpatchable vulnerabilities (noise).
- **Update lag:** CVE databases lag 1-7 days behind public announcements; zero-days are initially undetected.
- **Base image risk:** Underlying base images (ubuntu:22.04, nginx:latest) often contain CVEs; changing base images frequently is operationally expensive.

---

## Runtime Threat Detection

Admission control (pod security policies, image scanning) enforces policy at deployment time. Runtime detection observes container behavior during execution, detecting suspicious activity that policies don't prevent.

### Behavioral Monitoring

- **System calls:** Monitor syscalls (via eBPF, ptrace, or Falco) to detect unusual kernel API usage (mmap with EXEC flag, socket syscalls to unexpected IPs).
- **Process spawning:** Detect unexpected child processes (shell spawning within app container, typically indicating compromise).
- **File system activity:** Detect writes to binary directories, deletion of audit logs, or unusual file access patterns.
- **Network connections:** Detect outbound connections to known C&C infrastructure, high-volume data exfiltration.

### Tools & Systems

**Falco:** Open-source runtime security tool using eBPF and supported syscall tracing. Rules define suspicious behaviors (e.g., "shell spawned inside container" or "write to /etc/shadow"). Detections can trigger webhooks or log to SIEM.

**Sysdig:** Commercial platform for container forensics and anomaly detection.

**eBPF-based detection:** Direct kernel instrumentation via eBPF provides low-overhead visibility. Programs can trigger on syscalls before they complete (not just logging after).

### Detection Blind Spots

- **Insider threats:** Legitimate credentials used maliciously are hard to detect without context.
- **Dormant malware:** Malware that activates on a trigger (future date, specific input) may evade detection.
- **Low-and-slow exfiltration:** Small, infrequent data transfers blend with normal traffic.
- **Kernel exploits:** Severely compromised kernels may disable or bypass monitoring agents.

---

## Secret Encryption & Access Control

### Secret Encryption at Rest

By default, Kubernetes stores Secrets in etcd unencrypted. Production clusters should enable **encryption at rest** (KMS plugin or encrypted etcd). Encryption key management is distinct from secret value management.

**Options:**

- **KMS encryption:** Kubernetes encrypts with a local key; the KMS provider (AWS KMS, HashiCorp Vault) encrypts the local key. Adds operational complexity but separates encryption keys from cluster secrets.
- **Encrypted etcd:** Database-level encryption; simpler but keys are stored alongside the cluster.

### Secret Distribution & Rotation

Secrets (database passwords, API keys) are typically injected into pods via:
- **Environment variables:** Simple but leaks in ps output and logs.
- **Volume mounts:** Mounted as files; better for large secrets or sensitive data.
- **External secret managers:** Pods fetch from Vault, AWS Secrets Manager at runtime; decouples secret lifecycle from pod lifecycle.

**Rotation:** Redeploying all pods with new secret values is operationally expensive. External secrets engines support rotation without pod restart; applications poll or receive webhooks on update.

### Secret Access in Audit Logs

Kubernetes apiserver logs secret access in audit logs. By default, secret VALUES are redacted; request/response bodies containing secrets can be logged if audit level is high. Configure sensitive field redaction to prevent accidental exposure.

---

## Audit Logging & Forensics

### Audit Trail

Kubernetes apiserver can log all API requests: who accessed what resource, when, and what changes were made.

**Audit levels:**

- `Metadata`: log API action only (verb, resource, names, no object data).
- `RequestResponse`: log metadata + request/response bodies (verbose; expensive).
- `None`: don't log.

**Example audit log entry:**

```json
{
  "level": "RequestResponse",
  "auditID": "12345",
  "stage": "ResponseComplete",
  "requestObject": {"apiVersion": "v1", "kind": "Pod", ...},
  "responseObject": {"metav1.ObjectMeta": {"name": "attacker-pod", ...}},
  "user": {"username": "dev@corp.com", "uid": "1234"},
  "verb": "create",
  "objectRef": {"apiVersion": "v1", "kind": "Pod", "namespace": "production"}
}
```

### Post-Incident Analysis

Audit logs enable detecting attack timelines: when suspicious pods were created, who created them, what credentials were used. Coupled with runtime logs (Falco) and container image hashes (from kubelet logs), audit logs form the foundation of forensics.

**Challenges:** Logging is verbose; a large cluster generates terabytes of audit logs. Log retention policies often expire logs after 30-90 days. SIEM integration (forwarding to splunk/ELK) is essential for long-term analysis.

---

## Related Topics

See also: [devops-kubernetes](devops-kubernetes.md), [security-container](security-container.md), [security-network](security-network.md), [infrastructure-container-networking](infrastructure-container-networking.md), [cryptography-key-management](cryptography-key-management.md).