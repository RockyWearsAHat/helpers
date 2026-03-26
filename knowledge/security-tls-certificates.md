# TLS & Certificates

## TLS Handshake

### TLS 1.3 (Current Standard)

1-RTT handshake (down from 2-RTT in TLS 1.2):

```
Client → Server: ClientHello (supported cipher suites, key share)
Server → Client: ServerHello (chosen cipher suite, key share, encrypted extensions, certificate, CertificateVerify, Finished)
Client → Server: Finished
```

Key changes from TLS 1.2:

- Only 5 cipher suites (all AEAD: AES-128-GCM, AES-256-GCM, ChaCha20-Poly1305)
- No RSA key exchange (forward secrecy mandatory via ECDHE or DHE)
- 0-RTT resumption (with replay risk — safe only for idempotent requests)
- Encrypted handshake (certificate is encrypted, not visible to passive observers)
- Removed: RC4, SHA-1, CBC mode, static RSA, DH, export ciphers, compression

### TLS 1.2 (Still Common)

```
Client → Server: ClientHello
Server → Client: ServerHello, Certificate, ServerKeyExchange, ServerHelloDone
Client → Server: ClientKeyExchange, ChangeCipherSpec, Finished
Server → Client: ChangeCipherSpec, Finished
```

Recommended cipher suites for TLS 1.2:

```
TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384
TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384
TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256
```

### 0-RTT (TLS 1.3 Early Data)

Client sends application data in the first flight using a pre-shared key from a previous session. Risk: replay attacks — attacker can resend the 0-RTT data. Only use for idempotent, non-state-changing requests (GET, not POST).

## Certificate Chain

```
Root CA (self-signed, in OS/browser trust store)
  └── Intermediate CA (signed by root)
        └── Leaf Certificate (signed by intermediate, your domain)
```

**Root CAs**: ~150 trusted roots in major browsers/OS. Examples: DigiCert, Let's Encrypt (ISRG Root), Sectigo, GlobalSign.

**Why intermediates**: Root CA private key is kept offline. Intermediate keys do the daily signing. If compromised, only the intermediate is revoked.

## Certificate Types

| Type                           | Validation                              | Issuance Time | Visual Indicator                        |
| ------------------------------ | --------------------------------------- | ------------- | --------------------------------------- |
| DV (Domain Validation)         | Prove domain control (DNS, HTTP, email) | Minutes       | Padlock only                            |
| OV (Organization Validation)   | DV + verify organization exists         | Days          | Padlock + org in cert details           |
| EV (Extended Validation)       | OV + extensive legal verification       | Weeks         | Padlock (green bar removed by browsers) |
| Wildcard (`*.example.com`)     | Covers all subdomains (one level)       | Varies        | Same as base type                       |
| SAN (Subject Alternative Name) | Multiple specific domains in one cert   | Varies        | Same as base type                       |

**Practical recommendation**: DV certs via Let's Encrypt for most use cases. OV/EV only when legally required.

## ACME Protocol

Automated Certificate Management Environment — protocol for automated certificate issuance.

### Let's Encrypt

Free, automated, open DV certificates. 90-day validity (encourages automation).

**Challenge types**:

- `HTTP-01`: Place file at `http://domain/.well-known/acme-challenge/token`. Requires port 80. Can't do wildcards.
- `DNS-01`: Create TXT record `_acme-challenge.domain`. Works for wildcards. Requires DNS API access.
- `TLS-ALPN-01`: Respond on port 443 with special TLS certificate. Used when port 80 is blocked.

### Tools

```bash
# Certbot (Python, most popular)
certbot certonly --webroot -w /var/www/html -d example.com -d www.example.com
certbot certonly --dns-cloudflare -d '*.example.com'
certbot renew  # Cron job: twice daily

# acme.sh (shell, lightweight)
acme.sh --issue -d example.com -w /var/www/html
acme.sh --issue --dns dns_cf -d '*.example.com'
```

### cert-manager (Kubernetes)

```yaml
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: ops@example.com
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
      - http01:
          ingress:
            class: nginx
---
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: example-tls
spec:
  secretName: example-tls-secret
  issuerRef:
    name: letsencrypt-prod
    kind: ClusterIssuer
  dnsNames:
    - example.com
    - www.example.com
```

## HSTS (HTTP Strict Transport Security)

Forces browsers to use HTTPS for all future requests to the domain:

```
Strict-Transport-Security: max-age=31536000; includeSubDomains; preload
```

- `max-age`: how long (seconds) to remember HTTPS-only (1 year = 31536000)
- `includeSubDomains`: applies to all subdomains
- `preload`: submit to browser preload list (permanent, hard to undo)

**HSTS Preload List**: hardcoded into Chrome, Firefox, Safari, Edge. Submit at hstspreload.org. Once preloaded, removing is slow (months). Only do it when confident.

## OCSP Stapling

Instead of browsers checking certificate revocation with the CA (slow, privacy concern), the server includes a signed OCSP response in the TLS handshake.

```nginx
# Nginx
ssl_stapling on;
ssl_stapling_verify on;
ssl_trusted_certificate /path/to/chain.pem;
resolver 8.8.8.8 8.8.4.4 valid=300s;
```

## Mutual TLS (mTLS)

Both server AND client present certificates. Used for service-to-service authentication.

```
Client → Server: ClientHello
Server → Client: ServerHello, Certificate, CertificateRequest
Client → Server: Certificate (client cert), CertificateVerify
```

Common in: Kubernetes pod-to-pod (Istio, Linkerd), zero trust networks, banking APIs, IoT devices.

**SPIFFE/SPIRE**: Framework for workload identity. Issues short-lived X.509 certificates (SVIDs) to services. Automatic rotation.

## Certificate Rotation

### Automated Renewal

```bash
# Cron job (certbot)
0 0,12 * * * certbot renew --deploy-hook "systemctl reload nginx"

# Systemd timer (preferred over cron)
[Timer]
OnCalendar=*-*-* 00,12:00:00
RandomizedDelaySec=3600
```

**Best practice**: renew when 1/3 of lifetime remains. For 90-day certs, renew at 60 days.

### Zero-Downtime Rotation

1. Obtain new certificate before old one expires
2. Configure server to serve new cert (hot reload or graceful restart)
3. Verify new cert is being served: `openssl s_client -connect domain:443`
4. Monitor for errors after rotation

## Common Issues

| Issue            | Symptom                                 | Fix                                                  |
| ---------------- | --------------------------------------- | ---------------------------------------------------- |
| Expired cert     | Browser warning, connection refused     | Automate renewal with cron/timer                     |
| Incomplete chain | Works in some browsers, fails in others | Include intermediate cert(s) in chain file           |
| Mixed content    | HTTPS page loads HTTP resources         | Fix resource URLs, use CSP upgrade-insecure-requests |
| SNI mismatch     | Wrong cert served                       | Check virtual host configuration, SNI support        |
| Weak cipher      | Security scanner warning                | Disable TLS 1.0/1.1, prefer TLS 1.3 cipher suites    |
| Self-signed      | Not trusted by browsers                 | Use Let's Encrypt for public, mkcert for development |
| DNS mismatch     | Cert for wrong domain                   | Verify SAN/CN matches the domain                     |

## Tools

```bash
# Check certificate
openssl s_client -connect example.com:443 -servername example.com < /dev/null 2>/dev/null | openssl x509 -noout -text

# Check expiration
openssl s_client -connect example.com:443 < /dev/null 2>/dev/null | openssl x509 -noout -dates

# Generate self-signed (testing only)
openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout key.pem -out cert.pem

# mkcert (local development — installs local CA)
mkcert -install
mkcert localhost 127.0.0.1 ::1

# step-ca (private CA for internal services)
step ca init --name "Internal CA" --address :443
step ca certificate svc.internal svc.crt svc.key

# Test TLS configuration
testssl.sh example.com
sslyze example.com
```
