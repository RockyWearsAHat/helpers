# Certificate Management — PKI Lifecycle, Automation, Monitoring, and Operationalization

## Overview

Certificate management at scale spans three dimensions: **lifecycle** (issuance → renewal → revocation), **automation** (provisioning without manual work), and **operational safety** (expiry monitoring, rotation without downtime). Most outages stem not from bad certificates but from expired ones, misaligned renewal windows, and failure to automate renewal across hundreds of endpoints.

## PKI Fundamentals

### Certificate Hierarchy

Public Key Infrastructure consists of:

**Root Certificate Authority (CA)**: Self-signed certificate. Holds private key that signs intermediate CAs. Should be offline to minimize compromise risk.

**Intermediate CA**: Signed by Root CA. Issues end-entity certificates. Can be revoked and reissued without invalidating all end-entity certs. Typical deployments rotate approximately every 3-5 years.

**End-Entity (Leaf) Certificates**: Signed by Intermediate CA. Issued to servers, services, or individuals (for mTLS, client certs). Typical lifetime 30-90 days (short-lived for browser PKI; longer for internal infrastructure).

**Certificate Chain**: End-entity cert must include the issuing Intermediate CA cert. Clients download the chain to validate: `end-entity → intermediate → root`.

### Validation Path

When client validates certificate:
1. Verify end-entity certificate signature against Public Key in Intermediate CA cert
2. Verify Intermediate CA certificate signature against Public Key in Root CA cert
3. Verify Root CA self-signature or check Root cert in client's trust store

Chains with broken links (missing intermediate) fail validation.

## Certificate Lifecycle

### Issuance Phase

**Manual (Legacy)**:
```
$ openssl req -new -keyout server.key -out server.csr  # Generate CSR
# Send CSR to CA
# CA operator reviews, signs
# Receive certificate back
$ openssl x509 -in cert.pem -text                       # Verify
```

Manual issuance is rare today outside specialized scenarios. Pain points: operator bottleneck, turnaround time, easy to mistype parameters.

**Automated (ACME)**:
[ACME](https://tools.ietf.org/html/rfc8555) (Automated Certificate Management Environment) automates issuance through machine-readable protocol:

1. Client proves control of domain (HTTP-01 or DNS-01 challenge)
2. CA verifies client controls domain
3. CA issues certificate automatically
4. Client stores certificate and key

**ACME Challenges:**

- **HTTP-01**: Client places token file at `http://domain/.well-known/acme-challenge/{token}`. CA fetches; if present, domain is proven. Simple; requires HTTP endpoint accessible from internet. Fails behind firewalls or on DNS-only services.
- **DNS-01**: Client creates DNS TXT record. CA queries DNS; if record exists, domain is proven. Works for DNS-only services; requires DNS API write access (more complex operationally).

**Providers:** Let's Encrypt (free, most common), ZeroSSL, Buypass. All support ACME.

**Rate Limits:** Let's Encrypt rate-limits ACME requests (50 new certs per domain per week) to prevent abuse. Affects high-velocity environments; can be increased on request.

### Renewal Phase

Renewals occur before expiry (typically at 2/3 lifetime):

$$\text{renewal time} = \text{issued at} + \frac{2}{3} \times \text{lifetime}$$

For 90-day cert: renew after 60 days. Grace window = 30 days before expiry to fix issues.

**Manual renewal**: Operator runs cert renewal command, downloads new cert, updates servers. Fragile; easy to miss one server. Common for infrequently updated certificates.

**Automated renewal**: Controller (cert-manager, Certbot) runs renewal on schedule, updates all endpoints. Typical interval: every day or every hour. Idempotent (issuing same cert twice is harmless; provider sends same cert if not yet renewed).

**Risk:** Certificate renewal can fail if:
- ACME provider rate-limited or offline
- Proof-of-control challenge fails (DNS misconfigured, endpoint down)
- New cert rejected for policy reasons (key length mismatch, domain mismatch)

Mitigation: Renew frequently (daily) to catch failures early. Monitor renewal latency.

### Revocation Phase

Revocation invalidates a certificate before expiry. Use cases:
- Private key compromised
- Domain control lost
- Certificate issued in error
- Service decommissioned (cleanup; not strictly required)

**Revocation mechanisms:**

1. **Certificate Revocation List (CRL)**: CA publishes list of revoked cert serial numbers. Clients download periodically. Old mechanism; high network overhead (CRL can be large).

2. **Online Certificate Status Protocol (OCSP)**: Client queries CA "is this cert revoked?". Synchronous; higher latency than CRL. Clients must trust response.

3. **OCSP Stapling**: Server attaches OCSP response to TLS handshake. Avoids client-to-CA query. Requires server to refresh OCSP response (typically every 7 days).

**Modern practice**: Use OCSP stapling if available; CRL as fallback. Revocation itself is infrequently used (certificates expire naturally in 30-90 days).

## Kubernetes: cert-manager Automation

[cert-manager](https://cert-manager.io) automates certificate issuance, renewal, and injection into Kubernetes resources.

### Architecture

**cert-manager components:**
- **Controller**: Watches Certificate CRDs; reconciles by issuing/renewing certs
- **Webhook**: Validates Certificate manifests (good UX for syntax errors)
- **CA Secret**: Stores private keys (encrypted at-rest via etcd encryption)

**Issuers**: Represent CA sources:
```yaml
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: admin@example.com
    privateKeySecretRef:
      name: letsencrypt-key
    solvers:
    - http01:
        ingress:
          class: nginx
```

**Certificate Resource**: Requests cert from issuer:
```yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: app-tls
spec:
  secretName: app-tls-secret
  commonName: app.example.com
  dnsNames:
  - app.example.com
  issuerRef:
    name: letsencrypt-prod
    kind: ClusterIssuer
  renewBefore: 480h  # Renew 20 days before expiry (90-day cert)
```

**Automatic Injection**: Ingress objects automatically generate Certificate if annotated:
```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  tls:
  - hosts:
    - app.example.com
    secretName: app-tls-secret
  rules:
  - host: app.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: app
            port:
              number: 80
```

cert-manager creates Certificate automatically; Controller reconciles to ensure cert exists and is renewed.

### Renewal Behavior

cert-manager renews certs before expiry specified in `renewBefore`. On renewal:
1. Controller generates new CSR
2. ACME provider issues new cert with same CN/SANs
3. New cert stored in secret; pod mounted with secret sees new cert
4. For TLS termination at ingress: no pod restart needed (ingress controller watches secret)
5. For app-level TLS (app directly handles cert): may need sidecar or restart

### Cross-Cluster Secrets

Private keys live in cluster-specific Secrets. To distribute cert across clusters via GitOps:

**Option 1: Per-cluster cert-manager** (recommended)
Each cluster issues its own cert via cert-manager. No shared secrets; each cluster independent.

**Option 2: Vault**
cert-manager integrates with Vault issuer. Multiple clusters fetch certs from Vault. Less common; added complexity.

## mTLS (Mutual TLS) Certificate Management

**mTLS** = both client and server authenticate via certificates. Requires:
- Server certificate (service.crt, service.key)
- Client certificate (client.crt, client.key)
- Each signed by same or trusted root CA

### Issuance at Scale

In microservice mesh or service-to-service communication, each service needs both client and server certs. At 50+ services, manual management scales poorly.

**Solutions:**
1. **Service mesh** (Istio, Linkerd, Consul): Control plane issues certificates on pod creation. Automatic CA rotations.
2. **cert-manager with RBAC**: Different namespaces use different Issuers; each Issuer trusted by specific services.
3. **Vault**: Central PKI; services authenticate to Vault for client cert.

### Typical Flow (Service Mesh)

1. Pod starts; mesh sidecar initialized
2. Sidecar generates CSR (private key stays in sidecar)
3. CSR sent to control plane CA
4. CA signs; cert injected as Secret
5. Sidecar uses cert for all outbound connections (to other pods)
6. Other pods verify sidecar cert against same root CA

**Automatic rotation**: Control plane rotates all certificates daily (typical). Pod never restarts; sidecar watches Secret, reloads cert.

## Wildcard Certificates and Common Name Limitations

**Wildcard cert** (`*.example.com`): Covers any subdomain of `example.com`.

**Advantages**: Single cert for unlimited subdomains (api.example.com, admin.example.com, etc.).

**Disadvantages:**
- Compromised private key exposes all subdomains
- ACME DNS-01 challenge required (HTTP-01 doesn't work for wildcard)
- Slightly weaker security posture (broader blast radius)

**Common Name (CN) vs Subject Alternative Names (SANs)**:

Historically, certificate validation checked only CN. Modern browsers/clients check SANs (list of authorized domain names).

**Bad practice**: Cert with CN=api.example.com but no SANs. Fails on modern clients.

**Good practice**: CN = primary domain, SANs = all expected domains (Primary + all alternatives).

Example:
```
Subject: CN = api.example.com
Subject Alt Name: api.example.com, admin.example.com, *.api.example.com
```

Certificate Manager (cert-manager) automatically includes CN in SANs.

## Certificate Transparency (CT)

CT is a system of append-only logs where CA **must** publish all issued certificates. Allows anyone to audit issued certs and detect misissued/unauthorized certificates.

**How it works:**
1. CA issues certificate
2. CA submits to CT logs (multiple logs: Google, Cloudflare, DigiCert, etc.)
3. CT log returns signed proof (SCT: Signed Certificate Timestamp)
4. CA embeds SCT in certificate or returns separately
5. Client verifies SCT during TLS handshake

**Practical impact:**
- Domains can monitor CT logs for unauthorized issuance (ct.googleapis.com logs searchable)
- Security research can audit issuer behavior
- Misissued certs detected faster (within hours vs weeks)

**For operators**: No action needed if using reputable CA (Let's Encrypt, major providers). SCT inclusion automatic.

## Certificate Pinning

Pinning = client trusts specific cert, not entire CA chain. Reduces blast radius of CA compromise.

**Public Key Pinning (HPKP)**:
```
Public-Key-Pins: pin-sha256="base64encodedkey"; pin-sha256="backup"; max-age=1000; includeSubDomains
```

Browser pins the public key. Future certs must use same key, even if signed by different CA.

**Pin placement**: HTTP response header or embedded in app config.

**Risk:** If key rotation needed (e.g., Heartbleed aftermath), pinning blocks all access until backup pin activated. Can cause outages if misconfigured.

**Modern alternative**: Reduce expiry to 30-90 days; eliminate pinning. Faster rotation on compromise; no outage risk.

**When to pin:**
- High-value targets (banking, government, sensitive infrastructure)
- Services with external clients (mobile apps)

**When not to pin:**
- Most B2B infrastructure (internal services, APIs)
- Frequently rotating certs (mTLS @ 1-day rotation)

## Expiry Monitoring and Alerting

Expired certs cause hard failures (TLS handshake fails; service down). Prevention is critical.

### Detection Methods

**Option 1: Direct Query**
```bash
echo | openssl s_client -servername example.com -connect example.com:443 2>/dev/null | openssl x509 -noout -dates
```

Scrapes all endpoints; probes in parallel.

**Option 2: Kubernetes API**
Query Secret resources; check `.spec.certificate.spec.renewBefore`.

**Option 3: Certificate Transparency Logs**
Monitor CT logs for issued certificates; alert if cert issued without notification (indicates possible attack or misconfiguration).

### Alert Thresholds

- **> 30 days to expiry**: Informational. No action needed (renewal in progress).
- **7-30 days to expiry**: Warning. Action required if renewal fails.
- **< 7 days to expiry**: Critical. Service will fail if not renewed.
- **Expired (< 0 days)**: Page on call. Service already failing.

### Metrics and Dashboards

```
certificate_expiry_days{instance="api.example.com"} = 25
certificate_renewal_failures_total{issuer="letsencrypt"} = 2
certificate_provisioning_duration_seconds = 15
cert_manager_certificate_renewal_errors_total = 0
```

Team should track:
1. Age distribution of all certs (histogram: 0-10 days, 10-30 days, 30+ days)
2. Renewal failure rate per issuer
3. Mean provisioning latency (from issuance request to cert available)

### Renewal Reliability

Common failure patterns:
- **ACME rate limit hit**: Pre-emptively batch renewals or space by issuer
- **DNS validation failure**: Misconfigured dns.provider or zone delegation
- **Authorization loss**: Domain.com expires before cert; domain control challenge fails
- **Cert-manager controller crash**: Monitor controller pod health

Regression test: Manually trigger renewal; measure latency; verify new cert loaded into service.

## Certificate Formats and Interoperability

Certificates come in multiple formats; interoperability matters for tool chain.

**PEM** (Text format, standard):
```
-----BEGIN CERTIFICATE-----
MIIDXTCCAkWgAwIBAgIJANJ2xwKqh...
-----END CERTIFICATE-----
```
Portable; human-readable; most tools support.

**DER** (Binary format): PEM without the text wrapper. Smaller; less portable.

**PKCS#12** (Binary bundle): Includes certificate + private key + intermediate chain in single file. Used for client certificates in browsers.

**JKS** (Java KeyStore): Java-specific format. Required for Java apps; must convert from PEM.

Typical flow:
```
ACME provider returns PEM → Store in Secret/Vault → Convert to DER/PKCS#12/JKS as needed for target app
```

## Orchestration: cert-manager + GitOps

Combine cert-manager + GitOps to achieve repeatable, auditable certificate infrastructure:

1. **Certificate definitions in Git**: Certificate CRDs committed to repo
2. **Issuer configuration in Git**: ClusterIssuer manifests specify ACME provider, email, challenge method
3. **GitOps controller reconciles**: Flux/ArgoCD ensures Current Certificates + Issuers deployed
4. **cert-manager handles automation**: Controller renews automatically; stores in Secrets
5. **Audit trail in Git**: Who modified cert requirements, when

Example flow: Team adds new subdomain → Edit Certificate.spec.dnsNames → Git push → ArgoCD applies → cert-manager provisions → CT logs show new cert → Within hours, cert available on new subdomain.

## See Also

[security-tls-certificates.md](security-tls-certificates.md), [devops-secrets-rotation.md](devops-secrets-rotation.md), [security-cryptography-asymmetric.md](security-cryptography-asymmetric.md), [infrastructure-gitops-patterns.md](infrastructure-gitops-patterns.md)