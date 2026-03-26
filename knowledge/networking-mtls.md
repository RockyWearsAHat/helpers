# Mutual TLS: Certificate-Based Authentication, PKI & Zero-Trust Architecture

## Overview

**Mutual TLS (mTLS)** is a cryptographic protocol where both client and server authenticate each other using digital certificates. Unlike standard TLS (asymmetric: server proves identity, client doesn't), mTLS requires both parties to present valid certificates signed by a trusted Certificate Authority (CA). mTLS forms the foundation of zero-trust microservice architectures, enabling fine-grained access control without network perimeter assumptions.

## Certificate Basics

### Public Key Infrastructure (PKI)

**Certificate chain:**
```
User certificate (entity)
    ↑
    Signed by

Intermediate CA certificate
    ↑
    Signed by

Root CA certificate (self-signed, trusted anchor)
```

**User certificate contents (X.509):**
```
Subject: CN=client.example.com, O=Example Corp, C=US
Issuer: CN=Example Intermediate CA
Public Key: RSA 2048-bit key
Serial Number: 12345
Valid From: 2025-01-01
Valid Until: 2026-01-01
Thumbprint (SHA-256): abc123def456...

Extensions:
  - Key Usage: Digital Signature, Key Encipherment
  - Extended Key Usage: TLS Web Client Authentication
  - Subject Alt Names: client.example.com, client-prod-1
```

**CA certificate:**
```
Subject: CN=Example Intermediate CA
Issuer: CN=Example Root CA
Basic Constraints: CA=True (marks as authoritative for signing)
```

### Private Key Security

Each certificate has an associated private key (kept secret). During TLS handshake:

```
Server: "Here's my certificate + public key"
Client: Verifies certificate's signature (using issuer's public key from chain)
Client: Generates random session key, encrypts with server's public key
Server: Decrypts session key using its private key (only entity with private key can do this)
```

**Private key protection:**
- Stored on filesystem with restricted permissions (mode 0600)
- Or in hardware security module (HSM) / key management vault
- Never transmitted over network
- If compromised, attacker can impersonate the certificate holder

## mTLS Handshake

### 1. Standard TLS: Client Authenticates Server

```
Client: ClientHello (supported cipher suites, etc.)
Server ← ClientHello

Server: ServerHello + Certificate (server's cert + chain)
Client ← ServerHello + Certificate

Client: Verifies server certificate:
  1. Check signature of server's cert using issuer's public key
  2. Check certificate not expired
  3. Check certificate valid for requested hostname
  4. Check issuer is in client's trust store (root CA is trusted)
  5. Check Subject Alt Names match hostname

Client: Sends ClientKeyExchange (encrypted session key)
Server: Decrypts session key using its private key
Both: Establish encrypted session with session key
```

**Result:** Client knows it's talking to genuine server (server proved ownership of private key).

### 2. mTLS: Client Also Authenticates

After server presents certificate:

```
Server: Sends CertificateRequest (tells client to also present cert)
Server ← Sends CertificateRequest

Client: ClientCertificate (client's cert + chain)
Server ← ClientCertificate

Server: Verifies client certificate (same checks as above):
  1. Signature valid
  2. Not expired
  3. Issuer in server's trust store
  4. Verify client identity from cert subject/SAN

Client: ClientKeyExchange + Finished
Server: Finished

Result: Server knows client's identity; client knows server's identity
Both communicate with encrypted session
```

## Identity from Certificates

### Subject-Based Identity

Certificate's **Subject** and **Subject Alt Names (SAN)** fields encode identity:

```
Subject: CN=api-service.production.svc.cluster.local
SAN: ["api-service.production.svc.cluster.local", "api-service"]

Server uses this to identify client:
  "This request came from certificate with CN=api-service"
  → Check ACL: does api-service have permission to call this endpoint?
```

### mTLS in Kubernetes (Istio Example)

```
Pod A (service: checkout)
  ├─ Envoy sidecar proxy
  │   ├─ Certificate: CN=checkout.default.svc.cluster.local
  │   └─ Private key

Pod B (service: payment)
  ├─ Envoy sidecar proxy
  │   ├─ Certificate: CN=payment.default.svc.cluster.local
  │   └─ Private key

Checkout → Payment:
  1. Checkout's Envoy initiates mTLS to Payment's Envoy
  2. Checkout cert presented; Envoy verifies against Payment's CA bundle
  3. Payment Envoy verifies Checkout cert
  4. Both authenticate; request forwarded to Payment container
  5. Payment ACL checks: "checkout has permission to read orders" → Yes → Process
```

Traffic inside cluster encrypted and authenticated; no unencrypted or unauthenticated traffic.

## Certificate Rotation & Expiration

### Manual Rotation (Traditional)

```
Day 1: Certificate created, valid until Day 365
Day 355: Operator notices expiration 10 days away
Day 356: Operator generates new certificate + key, distributes to servers
Day 357: Servers restart with new cert; old cert used until expiration
Day 365: Old cert expires; connections still work if server has new cert
```

**Risk:** Operator misses deadline → certificate expires → connection failures until noticed.

### Automated Rotation (Modern)

**cert-manager** (Kubernetes):
```yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: api-tls
spec:
  secretName: api-tls-secret
  issuerRef:
    name: letsencrypt-prod
  duration: 2160h (90 days)
  renewBefore: 720h (30 days before expiration)
  commonName: api.example.com
  dnsNames:
  - api.example.com
  - api-prod.example.com
```

**Workflow:**
```
Day 1: cert-manager creates certificate (valid 90 days)
Day 60: cert-manager detects cert is 30 days from expiration
Day 60: Renews certificate with CA (gets new cert + key)
Day 60: Updates Kubernetes secret with new cert
Day 60: Pod/service automatically reloads secret (watches Kubernetes API)
Day 90: Old cert expires; service already using new cert → No disruption
```

### Short-Lived Certificates (Minutes to Hours)

**Benefit:** Limits impact of compromised private key.

```
Traditional (1-year certs):
  Day 1: Certificate issued
  Day 200: Private key compromised (attacker steals key from disk)
  Day 365: Certificate expires (certificate still valid!)
  Risk window: 165 days of unauthorized access possible

Short-lived (1-hour certs):
  Day 1, Hour 1: Certificate issued
  Day 1, Hour 1:30: Private key compromised
  Day 1, Hour 2: Certificate expires; attacker can't renew (needs CA access)
  Risk window: 30 minutes of unauthorized access
```

Short-lived certs require continuous renewal (e.g., every hour or every request). Practical in automated environments (Kubernetes, cloud functions) where renewal is managed by infrastructure, not humans.

## SPIFFE & SPIRE

**SPIFFE** (Secure Production Identity Framework For Everyone): Open standard for identity in dynamic environments.

**SPIRE** (Secure Production Identity Runtime Environment): Reference implementation.

### SPIFFE Identity

**SPIFFE ID:** Cryptographically-bound identity URI

```
spiffe://example.com/api-service
spiffe://example.com/checkout/web-frontend
spiffe://example.com/database/mysql-primary
```

**Properties:**
- Globally unique within trust domain
- Not tied to IP address or hostname (portable across cloud providers, clusters)
- Represents *workload* (software component) not *machine*

### SPIRE Architecture

```
SPIRE Agent (runs on each node)
  ├─ Obtains workload identity via attestation
  ├─ Requests certificate from SPIRE Server
  └─ Provides certificate to local workload (via Unix socket)

SPIRE Server (central)
  ├─ Manages CA for the organization
  ├─ Approves workload registration
  ├─ Issues certificates to agents
  └─ Tracks SPIFFE → certificate mappings
```

**Attestation:** SPIRE Agent proves it's legitimate before receiving certificate:
```
Node: "I'm running in this AWS region, with this instance ID"
SPIRE Server: Verifies AWS signature (proves node is genuine AWS instance)
SPIRE Server: Issues certificate for workloads on this node
```

### SPIRE Certificate Format

```
Subject: spiffe://example.com/api-service
SAN: spiffe://example.com/api-service

Server reads SAN:
  "Request authenticated as spiffe://example.com/api-service"
  → Look up ACL for this service
  → Grant/deny access
```

## Istio mTLS & Service Mesh

**Istio PeerAuthentication** enforces mTLS:

```yaml
apiVersion: security.istio.io/v1beta1
kind: PeerAuthentication
metadata:
  name: default
  namespace: production
spec:
  mtls:
    mode: STRICT # All traffic must be mTLS
```

**Enforcement:**
```
Pod A → Envoy (downstream proxy)
        ├─ Intercepts outbound traffic
        ├─ Initiates mTLS to Pod B's Envoy
        └─ Certificate from /var/run/secrets/workload-spiffe/cert.pem
        
Pod B → Envoy (upstream proxy)
        ├─ Receives mTLS connection
        ├─ Validates Pod A's certificate
        └─ Forwards to Pod container (internal, unencrypted)
```

**Backward compatibility:** Istio can run in permissive mode (accepts both mTLS and plaintext), enabling gradual migration.

## Trust Bundles & Chain Validation

### Trust Store

Operating system or application maintains **trust store**: set of root CA certificates considered authoritative.

```
macOS trust store: /System/Library/Keychains/SystemRootKeychain.keychain
Linux: /etc/ssl/certs/ca-bundle.crt
Windows: Certificate Manager (certmgr.msc)

Contains ~150-200 root CAs (Mozilla, IdenTrust, DigiCert, etc.)
```

When verifying certificate:
```
1. Check certificate's issuer field
2. Look for issuer cert in trust store
3. Verify issuer cert's signature on user cert (using issuer's public key)
4. Recursively verify issuer cert (if intermediate CA, lookup its issuer, etc.)
5. Until reaching root CA (which is self-signed; check against trust store)
```

### Chain of Trust

```
Server cert (api.example.com)
  ↓ Signed by
Intermediate CA cert
  ↓ Signed by
Root CA cert (self-signed, in trust store)

Client verifies: Can I trace from api.example.com cert → root cert I trust?
Yes → Server is authentic
```

### Organization-Private CAs

Internal microservices use custom (non-public) CA:

```
Organizational Root CA
  ├─ Used internally, not in public trust stores
  ├─ Issues certificates for internal services
  └─ Distributed to services in trust store

External HTTPS (cdn.example.com)
  ├─ Uses public CA (DigiCert, Let's Encrypt)
  ├─ Certificate in public trust stores
```

Workflow:
```
Service A (internal) wants to verify Service B (internal) cert:
  Load trust store: [Org Root CA]
  Verify Service B cert against Org Root CA
  → Verified ✓
```

## Zero-Trust Architecture with mTLS

**Traditional perimeter-based security:**
```
Firewall (protected zone)
  ├─ Service A
  ├─ Service B
  └─ Service C
  
Inside firewall: trust all traffic
Outside firewall: block all traffic
```

**Zero-trust with mTLS:**
```
No perimeter; every connection authenticated/authorized
Service A ← mTLS → Service B (verify B's certificate, check ACL)
Service B ← mTLS → Service C (verify C's certificate, check ACL)
Service C ← mTLS → Service A (verify A's certificate, check ACL)
Service A ← mTLS ← External (revoked; certificate verification fails)

All traffic encrypted; all identities verified; no implicit trust
```

### Practical Requirements

1. **Certificate distribution:** Every service needs cert + key (automated via SPIRE, cert-manager)
2. **Revocation:** When service is compromised, revoke its cert immediately
3. **ACL management:** Define "Service A is allowed to call Service B"
4. **Debugging:** Tools to inspect certificate, verify chain (openssl, grpcurl with -cacert)

### Common Misconceptions

- **mTLS ≠ complete security:** Prevents unauthorized services from connecting; doesn't prevent authorized services from abusing permissions (authorization layer needed)
- **mTLS ≠ encryption proof:** Encrypts in transit; doesn't prove encryption at rest, in logs, or during processing
- **mTLS adoption is gradual:** Large organizations spend months/years transitioning to mTLS

## Debugging & Troubleshooting

### Check Certificate

```bash
openssl x509 -in cert.pem -text -noout
# Shows: Subject, Issuer, Validity dates, Public Key, Extensions

openssl x509 -in cert.pem -noout -dates
# Subject: CN=api-service
# Issuer: CN=Example CA
# Not Before: Jan  1 00:00:00 2025 GMT
# Not After : Jan  1 00:00:00 2026 GMT
```

### Verify Chain

```bash
openssl verify -CAfile ca.pem cert.pem
# Returns 0 on success; non-zero if verification fails
```

### Test mTLS Connection

```bash
# Using openssl s_client (with client cert)
openssl s_client -connect api.example.com:443 \
  -cert client.pem -key client-key.pem \
  -CAfile ca.pem

# Using gRPC (grpcurl)
grpcurl -cacert ca.pem \
  -cert client.pem -key client-key.pem \
  list api.example.com:443
```

See also: security-tls-certificates.md, api-authentication.md, infrastructure-certificate-management.md, security-cryptography-asymmetric.md