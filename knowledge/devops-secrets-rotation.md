# Secrets Rotation & Management Deep Dive — Vault, Dynamic Secrets, Envelope Encryption, Sealed-Secrets

## Overview

Secrets management at scale requires two capabilities: **secure storage** (encryption, access control, audit) and **controlled rotation** (key/credential expiration, automatic renewal, policy enforcement). This note focuses on **rotation patterns, policies, and architectures** beyond basic storage. The design choice: static vs. dynamic secrets fundamentally changes attack surface.

## Vault Architecture & Concepts

### Initialization & Sealing

Vault stores the **master encryption key (MEK)** encrypted at rest. To start, Vault must retrieve and decrypt the MEK—the "unseal" operation.

**Shamir Secret Sharing (default)**:
- MEK split into 5 shards; at least 3 required to reconstruct
- No single operator has full key
- Human-intensive: operators manually provide shards at startup

```bash
# Initialize Vault
vault operator init -key-shares=5 -key-threshold=3
# Output: 5 unseal keys, root token

# Unseal (supply 3 of 5 keys, separately)
vault operator unseal

# Seal (lock vault)
vault operator seal  # Locks MEK; invalidates all tokens
```

**Auto-unseal** (CloudHSM, AWS KMS, GCP KMS, Azure Key Vault):
- Vault stores encrypted MEK in cloud provider's HSM or KMS
- Vault authenticates to cloud KMS at startup; cloud provider decrypts MEK automatically
- No manual key distribution; Vault starts unattended
- Trade-off: Vault's security now depends on cloud provider's HSM

```hcl
# vault/config.hcl
seal "awskms" {
  region = "us-east-1"
  kms_key_id = "arn:aws:kms:..."
}
```

### Storage Backend

Vault stores encrypted secrets in a pluggable storage backend. Default: Consul (but any backend works: S3, DynamoDB, PostgreSQL).

**Data at rest**:
```
Plaintext secret (e.g., "s3cr3t")
    ↓
Envelope encryption → DEK encrypts plaintext
    ↓
KEK encrypts DEK
    ↓
Stored: encrypted_secret + encrypted_DEK
```

The KEK is protected by the MEK (which is sealed/unsealed).

### Auth Methods

Vault validates client identity. Multiple methods; all return a **token** (like a bearer token).

| Method | Use Case |
|--------|----------|
| `userpass` | Development (username/password) |
| `approle` | Applications (role_id + secret_id) |
| `aws` | AWS IAM roles (SigV4 signature) |
| `kubernetes` | Pods (service account JWT) |
| `ldap` | Enterprise (LDAP directory) |
| `oidc` | Web SSO (OpenID Connect) |
| `jwt` | External systems (JWT validation) |

**AppRole flow** (common for CI/CD):

```bash
# Configure AppRole
vault write auth/approle/role/cicd \
  token_ttl=1h \
  token_max_ttl=4h

# Get role_id (stable)
vault read auth/approle/role/cicd/role-id

# Generate secret_id (short-lived, single-use)
vault write -f auth/approle/role/cicd/secret-id

# CI/CD system authenticates
vault login \
  -method=approle \
  role_id=xxx \
  secret_id=yyy

# Returns token; CI/CD now authenticated for 1h
```

### Secret Engines

Plugins that generate, rotate, or store secrets. Each engine has unique semantics.

| Engine | Semantics | Dynamic? |
|--------|-----------|----------|
| `kv/v2` | Key-value versioned store | No |
| `database` | Generate temp DB credentials | Yes |
| `aws` | Generate temp IAM credentials, STS tokens | Yes |
| `pki` | Issue X.509 certificates | Yes |
| `transit` | Encryption as a service (data never exposed) | N/A |
| `ssh` | Sign SSH certificates | Yes |

**KV v2 with versioning**:

```bash
# Write secret
vault kv put secret/myapp/db \
  username="admin" \
  password="old_pass"

# Read current
vault kv get secret/myapp/db
# Version 1

# Update
vault kv put secret/myapp/db username="admin" password="new_pass"
# Version 2

# Historical access
vault kv get -version=1 secret/myapp/db

# Rollback to version 1
vault kv rollback -version=1 secret/myapp/db
```

## Dynamic Secrets & Automated Rotation

### Database Engine

**Architecture**: Vault connects to database as privileged admin; issues short-lived credentials.

**Setup**:

```bash
# Enable database engine
vault secrets enable database

# Configure connection
vault write database/config/mydb \
  plugin_name=postgresql-database-plugin \
  connection_url="postgresql://{{username}}:{{password}}@db:5432/postgres" \
  username="vault_admin" \
  password="admin_pass" \
  allowed_roles="readonly,admin"

# Define role with SQL for credential creation
vault write database/roles/readonly \
  db_name=mydb \
  creation_statements="CREATE ROLE \"{{name}}\" WITH LOGIN PASSWORD '{{password}}' VALID UNTIL '{{expiration}}'; GRANT SELECT ON ALL TABLES IN SCHEMA public TO \"{{name}}\";" \
  default_ttl="1h" \
  max_ttl="24h"

# App requests credentials
vault read database/creds/readonly
# Key               Value
# lease_id          database/creds/readonly/abc123def456
# ttl               1h
# username          v-token-readonly-xyz789
# password          A1b2C3d4E5f6G7h8

# Credentials auto-revoke when TTL expires
```

**Automatic rotation**: Vault rotates credentials on fixed schedule or on-demand.

```bash
vault write -f database/rotate-root/mydb
# Vault changes admin password, asks DBA for new one
```

### AWS Secrets Engine

Generates temporary IAM credentials or STS tokens.

```bash
vault secrets enable aws

vault write aws/config/root \
  access_key="AKIA..." \
  secret_key="wJ21..."

vault write aws/roles/lambda \
  credential_type=assumed_role \
  assume_role_arn="arn:aws:iam::123456789:role/VaultAssumeRole" \
  ttl=15m

vault read aws/creds/lambda
# Key              Value
# access_key       ASIA2VUGQJZL3BFAKE
# secret_key       +dvFSrEaGvRQ7sInFake
# security_token   IQoDYXdzEN4a//...
# ttl              15m
# sts_endpoint     https://sts.amazonaws.com
```

**Use case**: Lambda functions, EC2 instances; never store AWS keys in config.

### Lease Management & Revocation

Every dynamic secret has a **lease**: TTL + expiration timestamp.

**Lease lifecycle**:

```
vault read aws/creds/lambda
  ↓
Lease issued with TTL=15m, expiration=2026-03-25T18:45Z
  ↓
After 15m: credentials auto-revoke (AWS key deleted)
  ↓
OR manual: vault lease revoke <lease-id> (immediate revocation)
```

**Renew credentials before expiry** (extend TTL):

```bash
# Check lease info
vault lease lookup aws/creds/lambda/abc123

# Renew (add 15m more)
vault lease renew aws/creds/lambda/abc123

# Bulk renew (all leases for a path)
vault lease renew -increment=6h aws/creds/lambda/abc123
```

**AppRole secret_id rotation**:

```bash
# Generate new secret_id (app updates its configuration)
vault write -f auth/approle/role/cicd/secret-id

# Revoke old secret_id
vault write auth/approle/role/cicd/secret-id/lookup \
  secret_id="old_secret_id_here"
vault write auth/approle/role/cicd/secret-id/destroy \
  secret_id="old_secret_id_here"
```

## Rotation Patterns

### Pattern 1: Zero-Downtime Credential Rotation (Database)

**Scenario**: Rotate database password weekly without disconnecting clients.

```
Timeline:
T0:      Vault issues old credentials (TTL=7d)
         App uses old credentials
         
T7d-1h:  Vault issues new credentials (TTL=7d, overlap=1h)
         App detects old credentials expiring soon
         App updates connection string to use new credentials
         
T7d:     Old credentials expire
         Vault revokes them from database
         No client connections remain on old credentials
```

**Implementation**:

```bash
# App polls Vault periodically
vault read database/creds/readonly
  # Returns fresh credentials; app caches with TTL
  
# When TTL approaches 80%, app renews
vault lease renew database/creds/readonly/abc123
```

**Or use Vault Agent** (pull-based):

```hcl
# vault-agent.hcl
auto_auth {
  method {
    type = "kubernetes"
  }
}

cache {
  use_cache = true
}

listener "unix" {
  address = "/tmp/vault.sock"
}

vault {
  address = "https://vault:8200"
}
```

Agent renews secrets automatically; app reads from agent socket.

### Pattern 2: Key Rotation (Envelope Encryption)

Scenario: Rotate the Key Encryption Key (KEK) without re-encrypting all secrets.

**Traditional (expensive)**: Decrypt all secrets with old KEK, re-encrypt with new KEK—causes downtime.

**Vault approach**: Support multiple KEKs simultaneously.

```
Stored secrets:
  - secret_1 (encrypted with KEK_v1)
  - secret_2 (encrypted with KEK_v2)
  - secret_3 (encrypted with KEK_v3)

Decrypt: Use KEK version embedded in ciphertext
Encrypt new: Use current KEK_v3
```

Vault handles multi-version KEK automatically. No re-encryption needed.

**Rekey operation** (change Vault's main MEK):

```bash
# Initiate rekey
vault operator rekey -init \
  -key-shares=5 \
  -key-threshold=3

# Provide old unseal keys (3 of 5)
vault operator rekey

# Rekey complete; new unseal keys generated
```

**Cost**: One rekey typically takes ~10s; no downtime if using auto-unseal.

### Pattern 3: Privilege Rotation (Dynamic Secrets)

Scenario: Dev team has MySQL access; rotate credentials weekly to limit blast radius of leaked keys.

```bash
# Week 1
dev_user1 = vault read database/creds/developer
# Returns: user=v-dev-xyz1, password=abc123, TTL=7d

# Week 2 (no application change needed)
dev_user2 = vault read database/creds/developer
# Returns: user=v-dev-xyz2, password=def456, TTL=7d
# Old credentials auto-revoked

# Developers never touch actual database passwords
```

**Advantage**: Database access is entirely managed by Vault. Dev team can't leak secrets outside of Vault's control.

## Envelope Encryption in Practice

Envelope encryption separates data key (encrypts payload) from key encryption key (encrypts data key).

```
Application Secret: "API_KEY=xyz123"

Step 1: Generate random DEK
DEK = random_128_bits()

Step 2: Encrypt payload with DEK (AES-256)
Ciphertext = AES-256-GCM(plaintext, DEK)

Step 3: Encrypt DEK with KEK (stored in KMS)
Encrypted_DEK = RSA-OAEP(DEK, KEK)

Step 4: Store
Database:
  {
    "ciphertext": <base64 of step 2>,
    "encrypted_key": <base64 of step 3>,
    "key_version": 42
  }

Decryption:
Step 1: Decrypt encrypted_DEK with KEK
DEK = RSA-OAEP-Decrypt(encrypted_key, KEK_v42)

Step 2: Decrypt payload with DEK
Plaintext = AES-256-GCM-Decrypt(ciphertext, DEK)
```

**Advantages**:
- **Key rotation is cheap**: Change KEK; old ciphertexts still valid (DEK is encrypted with old KEK version, version is stored with ciphertext)
- **Scalability**: Don't encrypt large payloads directly; compress data first, then DEK encrypts compressed data
- **Auditability**: Can track which KEK version encrypted which secrets

**Vault's Transit Engine** (abstraction of envelope encryption):

```bash
vault secrets enable transit

vault write -f transit/keys/app-key

# Encrypt
vault write transit/encrypt/app-key plaintext=$(base64 <<< "secret")
# Returns: ciphertext=vault:v1:2+FEqVTvZ8q5...

# Decrypt (Vault handles DEK/KEK, version tracking automatically)
vault write transit/decrypt/app-key ciphertext=vault:v1:2+FEqVTvZ8q5...
# Returns: plaintext=c2VjcmV0  # base64: "secret"

# Rotate KEK (old ciphertexts still decryptable)
vault write -f transit/keys/app-key/rotate
```

## Sealed Secrets vs. SOPS vs. CSI Driver

### Sealed Secrets (Bitnami Kubernetes operator)

Encrypts Kubernetes Secret resources with cluster's public key; decrypts server-side.

```bash
# Create normal Secret
kubectl create secret generic mysecret --from-literal=password=xyz --dry-run -o yaml > mysecret.yaml

# Encrypt with cluster's public key
kubeseal -f mysecret.yaml -w mysealed.yaml

# Apply sealed secret
kubectl apply -f mysealed.yaml

# Sealed Secrets operator in cluster decrypts automatically
```

**Advantages**:
- Server-side decryption (secret never leaves cluster)
- Git-safe: encrypted secret stored in Git
- Per-namespace encryption keys (optional)

**Limitations**:
- Server-side decryption tied to cluster cert; hard to migrate/backup
- Manual private key management

### SOPS (Mozilla Secrets OPerationS)

Client-side encryption. Secrets encrypted with KMS/PGP/age; decrypted locally by authorized users.

```bash
# Encrypt YAML file
sops -e -i secrets.yaml

# File is encrypted; Git-safe

# Decrypt locally (requires KMS/PGP access)
sops -d secrets.yaml

# In CI/CD, provision AWS IAM role for CI system to decrypt
```

**Advantages**:
- No new operators; standard YAML encryption
- Portable: work locally, in CI/CD, across clusters
- Supports multiple encryption backends (KMS, age, PGP)

**Integration with GitOps**:

```yaml
# Argo CD + SOPS
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization
metadata:
  name: myapp
resources:
  - myapp
secretGenerator:
  - name: appsecrets
    files:
      - secrets.yaml
```

Argo CD plugin decrypts SOPS secrets before applying.

### CSI Secrets Store Driver

Mounts secrets from external vault (Vault, AWS Secrets Manager, Azure Key Vault) directly as Kubernetes volumes.

```yaml
apiVersion: v1
kind: SecretProviderClass
metadata:
  name: vault-secrets
spec:
  provider: vault
  parameters:
    vaultAddress: "https://vault:8200"
    vaultSkipVerify: "true"
    objects: |
      - objectName: "db-password"
        secretPath: "secret/data/myapp/db"
        secretKey: "password"
---
apiVersion: v1
kind: Pod
metadata:
  name: app
spec:
  serviceAccountName: app
  containers:
  - name: app
    image: myapp:latest
    volumeMounts:
    - name: secrets
      mountPath: /mnt/secrets
      readOnly: true
  volumes:
  - name: secrets
    csi:
      driver: secrets-store.csi.k8s.io
      readOnly: true
      volumeAttributes:
        secretProviderClass: "vault-secrets"
```

**Advantages**:
- Secrets never stored in etcd (tighter security)
- Secrets mounted as volumes; app reads from filesystem
- Centralized secret management (Vault auth/rotation)

**Limitations**:
- Tighter coupling to Kubernetes; not portable to other orchestrators
- Requires CSI driver + RBAC setup

### Comparison

| Method | Encryption | Key Location | Portability | Git-Safe |
|--------|-----------|--------------|-------------|----------|
| Sealed Secrets | Server-side | Cluster certificate | Low | Yes |
| SOPS | Client-side | External KMS/age/PGP | High | Yes |
| CSI Driver | External (Vault) | External vault | Low (K8s only) | No (mount-time resolution) |

**Pattern**: Use **SOPS for GitOps** (portable, flexible); **CSI Driver for Kubernetes-only** (tightest integration); **Sealed Secrets** for simple dev clusters.

## External Secrets Operator (ESO)

Syncs secrets from external vault into Kubernetes Secrets.

```yaml
apiVersion: external-secrets.io/v1beta1
kind: SecretStore
metadata:
  name: vault-store
spec:
  provider:
    vault:
      server: "https://vault:8200"
      auth:
        kubernetes:
          mountPath: "kubernetes"
          role: "myapp"
      path: "secret"

---
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: myapp-secrets
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: vault-store
    kind: SecretStore
  target:
    name: myapp-secrets
    creationPolicy: Owner
  data:
  - secretKey: db-password
    remoteRef:
      key: myapp/db
      property: password
```

**Effect**: External Secret reconciliation loop periodically fetches `secret/myapp/db.password` from Vault and syncs into Kubernetes Secret `myapp-secrets`.

**Advantages**:
- Secrets rotate as Vault rotates
- Standard Kubernetes Secret objects (good for tooling)
- Different vaults per namespace (multi-tenancy)

**Difference from CSI Driver**: ESO creates `Secret` objects in etcd (tighter Kubernetes integration but less secure); CSI Driver keeps secrets off-cluster (tighter security but requires CSI driver).

## Rotation Orchestration

### Automated Rotation Policy (Vault)

```hcl
# Enable automatic rotation
vault write database/config/mydb \
  rotation_period="24h" \
  rotation_statements="ALTER USER {{name}} PASSWORD '{{password}}'"
```

Vault rotates admin credentials every 24h automatically.

### Workload-Triggered Rotation (ESO)

ESO refreshes external secrets on schedule (default 1h). When secret rotates in Vault, ESO picks it up within refresh interval.

```yaml
kind: ExternalSecret
spec:
  refreshInterval: 30m  # Check for updates every 30 minutes
```

Applications receive updated secrets via environment variables or mounted files.

### Manual Rotation (AppRole)

```bash
# CI/CD: on secret expiry or manual trigger
vault write -f auth/approle/role/cicd/secret-id
# Update CI/CD environment with new secret_id
```

## Audit & Compliance

### Enable Audit Logging

```bash
vault audit enable file file_path=/var/log/vault-audit.log

# All secret access logged
vault audit list
```

**Audit entry**:
```json
{
  "time": "2026-03-25T18:30:00.000Z",
  "type": "request",
  "auth": {
    "client_token": "hmac-xxx",
    "metadata": { "role_name": "approle" }
  },
  "request": {
    "id": "abc123",
    "operation": "read",
    "path": "database/creds/readonly"
  },
  "response": {
    "secret": { ... },
    "lease_duration": 3600
  }
}
```

### Compliance Checklist

- [ ] **Seal strategy**: Auto-unseal via KMS or manual Shamir (no single-point-of-failure)
- [ ] **Auth methods**: AppRole for apps, Kubernetes for pods, SAML for users
- [ ] **Dynamic secrets**: Database, AWS, SSH engines for temporary credentials
- [ ] **Rotation**: 24h rotation interval for admin credentials; 1h for app credentials
- [ ] **Audit logging**: All secret access logged; logs sent to syslog/SIEM
- [ ] **Encryption in transit**: TLS for all Vault communication
- [ ] **HA**: Multi-node Vault cluster for availability

## See Also

- [security-secrets-management](security-secrets-management.md) — foundational secrets concepts
- [devops-gitops](devops-gitops.md) — secrets in GitOps workflows
- [security-zero-trust](security-zero-trust.md) — authentication/authorization patterns