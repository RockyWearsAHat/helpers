# Asymmetric Cryptography — RSA, Elliptic Curves, and Key Exchange

## Overview

Asymmetric cryptography uses two keys: a **public key** (shared openly) and a **private key** (kept secret). Public key encrypts; private key decrypts. Enables secure communication without pre-shared secrets and provides **digital signatures** (private key signs, public key verifies). Computationally expensive compared to symmetric encryption; typically used for key exchange, signatures, and authentication rather than bulk encryption.

---

## RSA (Rivest-Shamir-Adleman)

Oldest and most widely deployed asymmetric algorithm. Security rests on difficulty of **integer factorization**: given large `n = p * q`, finding `p` and `q` is computationally hard.

### Key Generation

1. Choose two large random primes `p` and `q` (each 1024-2048 bits typical)
2. Compute `n = p * q`
3. Compute Euler's totient `φ(n) = (p-1)(q-1)`
4. Choose exponent `e` (public, typically 65537). Must be coprime with `φ(n)`
5. Compute `d` (private) such that `e*d ≡ 1 (mod φ(n))`

Public key: `(n, e)`. Private key: `d`. Factorization of `n` must remain secret; if attacker factors `n`, they can compute `φ(n)` and recover `d`.

### Encryption/Decryption

Encryption: `C = M^e mod n`
Decryption: `M = C^d mod n`

Signature: `Sig = M^d mod n` (private key operation)
Verification: `M = Sig^e mod n` (public key operation)

### Key Sizes

- **1024-bit keys:** Factorization is now feasible (estimated 5000-40000 core-years with modern techniques). Not recommended.
- **2048-bit keys:** Factorization estimates: millions of core-years. Secure through ~2030.
- **4096-bit keys:** Very secure; overkill for most purposes, but used for long-term signatures (e.g., CAs, document archival).

Larger keys increase both key generation and encryption time; 2048-bit is practical compromise.

### Practical Implementations

RSA is typically used with **padding schemes**:

- **PKCS#1 v1.5 (deprecated):** Older padding. Known attacks (Bleichenbacher padding oracle). Should not be used for new applications.
- **OAEP (Optimal Asymmetric Encryption Padding):** Modern padding for encryption. Adds randomness, prevents chosen-ciphertext attacks.
- **PSS (Probabilistic Signature Scheme):** Modern padding for signatures. Adds randomness per signature, improves security proof.

### Performance

RSA is 100-1000x slower than AES for same security level. Encryption of all data with RSA is impractical; hybrid encryption is standard: RSA encrypts symmetric key, symmetric cipher encrypts data.

---

## Elliptic Curve Cryptography (ECC)

Security rests on **elliptic curve discrete logarithm problem:** given points `P` and `Q = k*P` on an elliptic curve, computing `k` is hard.

### Key Sizes

Elliptic curves provide equivalent security with much smaller keys:

| Equivalent Security | RSA | ECC |
|---|---|---|
| 128-bit | 3072-bit | 256-bit |
| 192-bit | 7680-bit | 384-bit |

256-bit ECC ≈ 3072-bit RSA in security strength. Smaller keys enable faster operations and storage savings.

### ECDSA (Elliptic Curve Digital Signature Algorithm)

Standard signature algorithm over elliptic curves.

**Key generation:** Choose random private key `d` (scalar), compute public key `Q = d*G` (scalar multiplication of generator point).

**Signature:** 
1. Hash message `M` to get `h = H(M)`
2. Choose random `k`
3. Compute `R = k*G`, extract x-coordinate `r`
4. Compute `s = k^{-1}(h + d*r) mod n`
5. Signature: `(r, s)`

**Verification:**
1. Hash message `M` to get `h = H(M)`
2. Compute `w = s^{-1}`, `u1 = h*w`, `u2 = r*w`
3. Compute `R' = u1*G + u2*Q`, extract x-coordinate `r'`
4. Accept if `r' == r`

**Side-channel vulnerability:** Early ECDSA implementations leaked `k` timing (if `k` values were biased or timing was data-dependent). Several Bitcoin wallets were broken via this attack (Sony's PlayStation 3 key recovery). Implementation must use constant-time `k` generation and arithmetic.

### EdDSA (Edwards-Curve Digital Signature Algorithm)

Modern alternative to ECDSA. Uses twisted Edwards curves (e.g., Ed25519).

**Advantages:**
- Deterministic (no random `k` generation; signature is deterministic given message and key)
- Simpler, fewer parameters to tune
- Natural resistance to many side-channel attacks (not designed around scalar multiplication; operations are uniform)
- Smaller signatures (64 bytes for Ed25519 vs. variable-length ECDSA)

**Ed25519:** Uses Curve25519 (designed by Bernstein for simplicity, auditability, and side-channel resistance).

**Adoption:** Signal protocol, SSH keys (openssh), JWTs. Growing preference for new systems.

### ECDH (Elliptic Curve Diffie-Hellman)

Key exchange: two parties agree on shared secret without pre-sharing key.

**Protocol:**
1. Alice chooses random `a`, computes `A = a*G`, sends `A` to Bob
2. Bob chooses random `b`, computes `B = b*G`, sends `B` to Alice
3. Alice computes `S = a*B = a*b*G`
4. Bob computes `S = b*A = a*b*G`
5. Both derive symmetric key from `S`

**Security:** Attacker seeing `A`, `B`, `G` cannot compute `S` (given ECDLP is hard). Vulnerable to man-in-the-middle unless `A` and `B` are authenticated (e.g., via certificates or pre-shared key).

---

## X25519 and Curve25519

Designed by Daniel Bernstein for exceptional simplicity and auditability. Not a standardized elliptic curve (like NIST P-256), but a carefully chosen Montgomery curve.

### Properties

- **Speed:** Highly optimized for 256-bit Diffie-Hellman. Optimizations exploit prime `2^255 - 19` (simple arithmetic modulo this prime).
- **Simplicity:** ~1000 lines of implementation vs. NIST P-256 implementation is 10x+ larger and complex.
- **Side-channel resistance:** Constant-time arithmetic, no special cases.
- **Implementation diversity:** Many reference implementations in different languages, all agree (unusual for crypto).

### Use Cases

**X25519:** Diffie-Hellman key exchange (ECDH).
**Ed25519:** Digital signatures (EdDSA).

Together, X25519 + Ed25519 form a minimal, auditable cryptographic suite. Used in Signal, WireGuard, TLS 1.3 (optional), and systems prioritizing simplicity over NIST standardization.

---

## Diffie-Hellman Key Exchange (Classical)

Predates elliptic curves. Two parties agree on shared secret over insecure channel.

**Setup:** Agree on large prime `p` and generator `g` (public, agreed in advance or exchanged).

**Protocol:**
1. Alice chooses random `a`, computes `A = g^a mod p`, sends `A`
2. Bob chooses random `b`, computes `B = g^b mod p`, sends `B`
3. Alice computes `S = B^a mod p = g^{ab} mod p`
4. Bob computes `S = A^b mod p = g^{ab} mod p`

**Security:** Attacker sees `A`, `B`, `p`, `g`. Computing `a` from `A` (or `b` from `B`) is the **discrete logarithm problem** — believed hard for carefully chosen `p` and `g`.

**Key sizes:** Comparable to RSA. 2048-bit `p` recommended (3072-bit for long-term). Replaced by ECDH for efficiency.

---

## Digital Certificates and PKI

Public keys must be authenticated. How does Alice know the public key she receives is Bob's, not an attacker's?

### X.509 Certificates

Standard format for binding identity to public key. Contains:

- **Subject:** Identity of key owner (name, email, domain)
- **Issuer:** Certificate Authority that signed the certificate
- **Public Key:** The key itself
- **Serial Number:** Unique identifier
- **Validity Period:** "Not before" and "Not after" dates
- **Signature:** CA's signature over all fields (CA signs with its private key)

Example flow:
1. Bob generates key pair, requests certificate from CA
2. CA verifies Bob's identity (out-of-band or by inspection), creates X.509 certificate, signs with CA's private key
3. Bob publishes certificate (along with CA certificates forming a chain up to a root)
4. Alice retrieves Bob's certificate, verifies CA's signature using CA's public key (from CA certificate)
5. If chain is valid (signatures verify and validity periods are current), Alice trusts the public key is Bob's

### Certificate Chain (Trust Hierarchy)

Root CA → Intermediate CA → End-Entity (server/user)

Each cert signs the one below it. Alice starts with root CA's public key (pre-installed in her browser/OS). Chain verification:

1. Verify leaf cert is signed by intermediate CA (using intermediate's public key from cert)
2. Verify intermediate is signed by root CA (using root's public key from cert)
3. Verify root cert is self-signed and matches pre-installed root

If any signature fails or dates are invalid, chain fails and trust is broken.

### Certificate Authority Compromise

If a CA is breached, attacker can issue fraudulent certificates for any domain. Historical examples: DigiNotar breach (2011), Comodo breach (2011). Defenses:

- **Certificate Transparency (CT):** All issued certs are logged in append-only logs. Domain owners monitor CT logs for certificates issued in their names.
- **OCSP Stapling:** Server caches revocation status, reduces reliance on OCSP responder (which could be SPOF)
- **CAA (Certificate Authority Authorization) DNS records:** Domain owner specifies which CAs are allowed to issue certs for their domain
- **Pinning:** Application pins trusted certs or public keys, ignoring CA chain (risky; complicates key rotation)

---

## Post-Quantum Cryptography

Classical asymmetric algorithms (RSA, ECC) are vulnerable to **Shor's algorithm** on large-scale quantum computers. A sufficiently powerful quantum computer can solve factorization and discrete logarithm in polynomial time.

### Threat Timeline

No large-scale quantum computers exist (2026). Timeline estimates vary: 10-30 years before cryptographically relevant quantum computers (CRQCs) exist. But "harvest now, decrypt later" attacks are relevant: adversaries record encrypted traffic today, plan to decrypt with future quantum computers.

### Post-Quantum Candidates

**Lattice-based cryptography:** Security rests on **shortest vector problem** (SVP) in lattices. LWE (Learning With Errors) problem is believed hard even under quantum attack. NIST standardized:
- **ML-KEM** (key encapsulation): Diffie-Hellman replacement
- **ML-DSA** (signatures): Digital signature replacement

**Hash-based signatures:** Sign using one-time keypair per signature, tree of signatures. Provably secure if hash function is secure. Signature size is huge (thousands of bytes), not practical for general use.

**Multivariate polynomial:** Security rests on solving system of multivariate equations. Active research area, not yet standardized.

### Transition Strategy

- **Hybrid schemes:** Use both classical (RSA/ECC) and post-quantum together. Encrypted message is large but only needs to resist one algorithm.
- **Timeline:** NIST standardized early candidates (ML-KEM, ML-DSA) in 2024. Deployment expected ~2025-2030 in TLS, SSH, etc.
- **Symmetric unaffected:** AES, ChaCha20, SHA-256 are not threatened by quantum computers (Grover's algorithm gives only √N speedup, so 256-bit symmetric key is still secure against quantum attack requiring 2^128 work).

---

## Practical Use: TLS Handshake

TLS combines asymmetric and symmetric cryptography:

1. **Server authentication:** Server sends X.509 certificate (public key + identity signed by CA). Client verifies chain.
2. **Key exchange:** Client and server perform ECDH (or RSA key transport in older TLS 1.2). Results in shared secret.
3. **Symmetric encryption:** Both derive symmetric key from shared secret, use AES-GCM to encrypt application data.

Modern TLS 1.3:
- **Mandatory:** X25519 ECDH (or P-256)
- **Signatures:** RSA-PSS, ECDSA (with Ed25519 preferred)
- **Symmetric:** AES-GCM or ChaCha20-Poly1305

This design leverages strengths of both: asymmetric for authentication (public keys solve the identity problem), symmetric for bulk encryption (speed and proven safeness for large volumes).

---

## See Also

- [security-cryptography-symmetric.md](security-cryptography-symmetric.md) — Symmetric encryption and AEAD modes
- [security-best-practices.md](security-best-practices.md) — Cryptography selection and security principles
- [networking-http.md](networking-http.md) — TLS and HTTPS implementation
- [cryptography-practical.md](cryptography-practical.md) — Developer-focused key management and common pitfalls