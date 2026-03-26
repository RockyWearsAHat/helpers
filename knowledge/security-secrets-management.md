# Secrets Management

## Overview

Secrets (API keys, database credentials, TLS certificates, encryption keys) are the highest-value targets in any system. Proper secrets management eliminates hardcoded credentials, enforces rotation, controls access, and provides audit trails. The hierarchy: dedicated vault > cloud-native secret manager > encrypted config > environment variables >> hardcoded.

## HashiCorp Vault

### Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Client     │────▶│  Vault API   │────▶│  Storage     │
│  (CLI/SDK)   │     │  (TLS)       │     │  Backend     │
└──────────────┘     └──────┬───────┘     └──────────────┘
                            │
                     ┌──────┴───────┐
                     │  Auth Methods │
                     │  Secret Eng.  │
                     │  Audit Log    │
                     └──────────────┘
```

### Secret Engines

| Engine      | Purpose                                      | Dynamic? |
| ----------- | -------------------------------------------- | -------- |
| `kv` (v2)   | Static key-value with versioning             | No       |
| `database`  | Dynamic DB credentials with TTL              | Yes      |
| `aws`       | Dynamic IAM credentials / STS tokens         | Yes      |
| `pki`       | X.509 certificate issuance                   | Yes      |
| `transit`   | Encryption-as-a-service (no secret exposure) | N/A      |
| `ssh`       | Signed SSH certificates                      | Yes      |
| `totp`      | TOTP code generation/validation              | N/A      |
| `transform` | Tokenization / format-preserving encryption  | N/A      |

### KV v2 Operations

```bash
# Enable KV v2
vault secrets enable -version=2 -path=secret kv

# Write secret
vault kv put secret/myapp/db username="admin" password="s3cret"

# Read secret
vault kv get -format=json secret/myapp/db

# Version history
vault kv metadata get secret/myapp/db

# Rollback
vault kv rollback -version=2 secret/myapp/db

# Soft delete + undelete
vault kv delete secret/myapp/db
vault kv undelete -versions=3 secret/myapp/db

# Permanent destroy
vault kv destroy -versions=3 secret/myapp/db
```

### Dynamic Database Credentials

```bash
# Configure database connection
vault write database/config/mydb \
  plugin_name=postgresql-database-plugin \
  connection_url="postgresql://{{username}}:{{password}}@db:5432/mydb" \
  allowed_roles="readonly" \
  username="vault_admin" \
  password="admin_pass"

# Create role
vault write database/roles/readonly \
  db_name=mydb \
  creation_statements="CREATE ROLE \"{{name}}\" WITH LOGIN PASSWORD '{{password}}' VALID UNTIL '{{expiration}}'; GRANT SELECT ON ALL TABLES IN SCHEMA public TO \"{{name}}\";" \
  default_ttl="1h" \
  max_ttl="24h"

# Get dynamic credential (auto-expires)
vault read database/creds/readonly
# Key             Value
# lease_id        database/creds/readonly/abc123
# username        v-token-readonl-xyz789
# password        A1b2C3d4E5f6
```

### Policies

```hcl
# policy: app-readonly.hcl
path "secret/data/myapp/*" {
  capabilities = ["read", "list"]
}

path "database/creds/readonly" {
  capabilities = ["read"]
}

# Deny access to admin paths
path "sys/*" {
  capabilities = ["deny"]
}
```

```bash
vault policy write app-readonly app-readonly.hcl
```

### Auth Methods

| Method       | Use Case                                       |
| ------------ | ---------------------------------------------- |
| `token`      | Programmatic access, wrapped responses         |
| `approle`    | Machine/application authentication             |
| `kubernetes` | K8s service account JWT                        |
| `jwt/oidc`   | GitHub Actions, GitLab CI, cloud provider OIDC |
| `aws`        | EC2 instance or IAM role identity              |
| `ldap`       | Enterprise directory integration               |
| `userpass`   | Human users (dev/test)                         |

### AppRole Authentication

```bash
# Enable and configure
vault auth enable approle
vault write auth/approle/role/myapp \
  token_policies="app-readonly" \
  token_ttl=1h \
  token_max_ttl=4h \
  secret_id_ttl=10m \
  secret_id_num_uses=1

# Get role_id (stable, like a username)
vault read auth/approle/role/myapp/role-id

# Generate secret_id (ephemeral, like a password)
vault write -f auth/approle/role/myapp/secret-id

# Login
vault write auth/approle/login \
  role_id="xxx" secret_id="yyy"
```

### Auto-Unseal

| Method                  | Provider                               |
| ----------------------- | -------------------------------------- |
| AWS KMS                 | `seal "awskms" { kms_key_id = "..." }` |
| Azure Key Vault         | `seal "azurekeyvault" { ... }`         |
| GCP Cloud KMS           | `seal "gcpckms" { ... }`               |
| Transit (another Vault) | `seal "transit" { ... }`               |

## Cloud-Native Secret Managers

### AWS Secrets Manager

```python
import boto3
import json

client = boto3.client('secretsmanager')

# Create secret
client.create_secret(
    Name='myapp/database',
    SecretString=json.dumps({'username': 'admin', 'password': 's3cret'}),
    Tags=[{'Key': 'Environment', 'Value': 'production'}]
)

# Retrieve
response = client.get_secret_value(SecretId='myapp/database')
secret = json.loads(response['SecretString'])

# Rotation — attach Lambda function
client.rotate_secret(
    SecretId='myapp/database',
    RotationLambdaARN='arn:aws:lambda:...:function:rotate-db',
    RotationRules={'AutomaticallyAfterDays': 30}
)
```

### Azure Key Vault

```bash
# Create vault
az keyvault create --name myapp-vault --resource-group myapp-rg

# Set secret
az keyvault secret set --vault-name myapp-vault \
  --name db-password --value "s3cret"

# Get secret
az keyvault secret show --vault-name myapp-vault --name db-password

# RBAC — assign "Key Vault Secrets User" role
az role assignment create \
  --role "Key Vault Secrets User" \
  --assignee <principal-id> \
  --scope /subscriptions/.../resourceGroups/myapp-rg/providers/Microsoft.KeyVault/vaults/myapp-vault
```

### GCP Secret Manager

```bash
# Create secret
echo -n "s3cret" | gcloud secrets create db-password --data-file=-

# Access secret
gcloud secrets versions access latest --secret=db-password

# Grant access
gcloud secrets add-iam-policy-binding db-password \
  --role=roles/secretmanager.secretAccessor \
  --member=serviceAccount:myapp@project.iam.gserviceaccount.com
```

## Kubernetes Secrets

### The Problem with Native Secrets

Kubernetes Secrets are base64-encoded (not encrypted), stored in etcd (possibly unencrypted), and accessible to anyone with RBAC `get` on secrets in the namespace. They are NOT secure by default.

### External Secrets Operator (ESO)

```yaml
# SecretStore — connect to external provider
apiVersion: external-secrets.io/v1beta1
kind: SecretStore
metadata:
  name: vault-backend
spec:
  provider:
    vault:
      server: "https://vault.example.com"
      path: "secret"
      auth:
        kubernetes:
          mountPath: "kubernetes"
          role: "myapp"

---
# ExternalSecret — sync external secret → K8s Secret
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: db-credentials
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: vault-backend
  target:
    name: db-credentials
  data:
    - secretKey: username
      remoteRef:
        key: secret/data/myapp/db
        property: username
    - secretKey: password
      remoteRef:
        key: secret/data/myapp/db
        property: password
```

### Sealed Secrets (Bitnami)

```bash
# Encrypt secret client-side (only cluster can decrypt)
kubeseal --format yaml < secret.yaml > sealed-secret.yaml

# Safe to commit sealed-secret.yaml to git
kubectl apply -f sealed-secret.yaml
# Controller decrypts → creates regular Secret in cluster
```

### SOPS (Mozilla)

```bash
# Encrypt with age key
sops --encrypt --age age1... secrets.yaml > secrets.enc.yaml

# Decrypt
sops --decrypt secrets.enc.yaml

# Edit in-place (decrypts → editor → re-encrypts)
sops secrets.enc.yaml

# .sops.yaml — rules per path
creation_rules:
  - path_regex: .*\.enc\.yaml$
    age: age1publickey...
  - path_regex: production/.*
    kms: arn:aws:kms:...
    age: age1publickey...
```

## CI/CD Secrets

### GitHub Actions

```yaml
# Repository / organization / environment secrets
jobs:
  deploy:
    environment: production # scoped secrets
    steps:
      - name: Deploy
        env:
          DB_PASSWORD: ${{ secrets.DB_PASSWORD }}
        run: ./deploy.sh

      # OIDC federation — no long-lived secrets
      - uses: aws-actions/configure-aws-credentials@v4
        with:
          role-to-assume: arn:aws:iam::123456789:role/github-deploy
          aws-region: us-east-1
```

### OIDC Federation (Keyless CI/CD)

```
┌─────────────┐    OIDC Token     ┌─────────────┐
│  GitHub      │─────────────────▶│  Cloud       │
│  Actions     │                  │  Provider    │
│  (IdP)       │◀─────────────────│  (AWS/GCP/   │
└─────────────┘   Temp Creds      │   Azure)     │
                                  └─────────────┘
```

**AWS trust policy for GitHub OIDC**:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Federated": "arn:aws:iam::123456789:oidc-provider/token.actions.githubusercontent.com"
      },
      "Action": "sts:AssumeRoleWithWebIdentity",
      "Condition": {
        "StringEquals": {
          "token.actions.githubusercontent.com:aud": "sts.amazonaws.com"
        },
        "StringLike": {
          "token.actions.githubusercontent.com:sub": "repo:org/repo:ref:refs/heads/main"
        }
      }
    }
  ]
}
```

## Rotation Strategies

### Rotation Patterns

| Pattern                | Description                                   | Use Case                   |
| ---------------------- | --------------------------------------------- | -------------------------- |
| **Dual-credential**    | Two active credentials, rotate one at a time  | API keys, service accounts |
| **Create-then-revoke** | Create new → update consumers → revoke old    | Database passwords         |
| **Dynamic**            | Short-lived credentials issued on demand      | Vault dynamic secrets      |
| **Certificate**        | Issues new cert before expiry, overlap period | TLS, mTLS                  |

### Rotation Checklist

1. New credential created and verified working
2. All consumers updated to use new credential
3. Grace period for in-flight requests
4. Old credential revoked/deleted
5. Rotation logged for audit

## Secret Detection Tools

### truffleHog

```bash
# Scan git history
trufflehog git file://./my-repo --only-verified

# Scan GitHub org
trufflehog github --org=myorg --only-verified

# Scan filesystem
trufflehog filesystem /path/to/code
```

### gitleaks

```bash
# Scan current repo
gitleaks detect --source . -v

# Pre-commit hook
gitleaks protect --staged

# Custom rules
# .gitleaks.toml
[[rules]]
  id = "custom-api-key"
  description = "Custom API Key"
  regex = '''MY_API_KEY_[A-Za-z0-9]{32}'''
  tags = ["key", "custom"]
```

### CI Integration

```yaml
# GitHub Actions pre-commit scan
- uses: gitleaks/gitleaks-action@v2
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## Exposed Secret Response Playbook

### Immediate Actions (0-15 minutes)

1. **Revoke** the exposed credential immediately
2. **Rotate** — issue replacement credential
3. **Assess scope** — what did the credential have access to?
4. **Search** for unauthorized usage in access logs

### Investigation (15-60 minutes)

5. **Determine exposure window** — when was it committed, when was it discovered?
6. **Check for abuse** — unusual API calls, data access, resource creation
7. **Scrub from git history** — `git filter-repo` or BFG Repo Cleaner
8. **Invalidate caches** — GitHub caches, CI artifacts, Docker layers

### Prevention

9. **Add pre-commit hooks** — gitleaks, truffleHog
10. **Enable GitHub secret scanning** with push protection
11. **Review and tighten** — was the credential overprivileged?
12. **Document** — post-incident review, update runbooks

### Git History Scrubbing

```bash
# BFG Repo-Cleaner (faster, simpler)
bfg --replace-text passwords.txt repo.git

# git filter-repo (more flexible)
git filter-repo --blob-callback '
  blob.data = blob.data.replace(b"EXPOSED_SECRET", b"REDACTED")
'

# Force push (destructive — coordinate with team)
git push --force --all
git push --force --tags
```

## Secrets Anti-Patterns

| Anti-Pattern                             | Risk                              | Fix                                   |
| ---------------------------------------- | --------------------------------- | ------------------------------------- |
| Hardcoded in source                      | Exposed in version control        | External secret manager               |
| Environment variables in Dockerfiles     | Leaked in image layers            | Multi-stage builds, runtime injection |
| Secrets in CI logs                       | Visible to anyone with log access | Mask outputs, never echo              |
| Shared service accounts                  | No attribution, hard to rotate    | Per-service credentials               |
| Long-lived credentials                   | Extended exposure window          | Short TTL, dynamic secrets            |
| Secrets in config files committed to git | Permanent in git history          | `.gitignore`, encrypted configs       |
| Same secret across environments          | Blast radius expansion            | Per-environment secrets               |
| No rotation                              | Accumulated risk                  | Automated rotation policies           |
