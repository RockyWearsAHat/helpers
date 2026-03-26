# Cryptographic Key Management — Generation, Distribution, Storage & Rotation

## Overview

Key management is the discipline of securely generating, storing, distributing, rotating, and retiring cryptographic keys. Weak key management undermines even the strongest cipher. A compromised key breaks all security derived from it—confidentiality, integrity, authentication. Modern key management involves Hardware Security Modules (HSMs), Key Management Systems (KMS), envelope encryption, and formal key ceremonies.

---

## Key Generation: Entropy and HSMs

### Entropy Requirements

Cryptographic keys must be generated from **high-entropy randomness**. Insufficient entropy (e.g., using system time or weak PRNGs) produces predictable keys vulnerable to brute force.

**Entropy sources:**
- **/dev/urandom** (Unix/Linux): OS kernel entropy pool (sufficient for most applications)
- **CryptographicRandom** (OS APIs): Windows CryptGenRandom, macOS SecRandomCopyBytes
- **Hardware RNGs:** RDRAND (Intel), specialized entropy boards
- **Combination:** Multiple entropy sources mixed to eliminate single points of failure

**Key size recommendations:**
- **Symmetric keys:** 256 bits (AES-256)
- **RSA:** 2048 bits minimum (4096 bits for long-term security)
- **Elliptic curve:** 256 bits (equivalent to ~3072-bit RSA)
- **Hash functions:** Output size determines collision resistance (256 bits = ~128-bit security)

### Hardware Security Modules (HSMs)

Physical devices storing private keys in tamper-resistant hardware. Keys are generated inside the HSM and never leave it in plaintext. Cryptographic operations (sign, decrypt) happen inside the device; results are returned to the application.

**Benefits:**
- Private keys never in application memory (resistant to memory dumps, side-channel attacks)
- Tamper detection: device erases keys if physically attacked
- Compliance: HSMs satisfy regulatory requirements (PCI-DSS, FIPS 140-2)
- Audit trails: operations logged

**Limitations:**
- **Cost:** $10K–$100K+ per device (high barrier for small deployments)
- **Network latency:** Slower than local keys (round-trip to HSM for each operation)
- **Complexity:** Different APIs, vendor lock-in, operational overhead

**Practical use:** High-value certificate authorities, banking infrastructure, enterprise PKI. Less common in cloud-native systems (where cloud provider KMS is preferred).

---

## Key Distribution and Exchange

### Diffie-Hellman Key Exchange

Protocol allowing two parties to establish a shared secret over an insecure channel without pre-shared secrets.

**Process:**
1. Alice and Bob publicly agree on large prime p and generator g
2. Alice picks private a, computes A = g^a mod p, sends A to Bob
3. Bob picks private b, computes B = g^b mod p, sends B to Alice
4. Alice computes shared secret S = B^a mod p = g^(ab) mod p
5. Bob computes S = A^b mod p = g^(ab) mod p (same secret)

**Eavesdropper sees A and B but cannot compute S without knowing a or b (discrete log problem).**

**Variants:**
- **ECDH:** Uses elliptic curves instead of modular exponentiation (smaller keys, same security)
- **Perfect Forward Secrecy (PFS):** Diffie-Hellman repeated per session; session compromise doesn't expose past/future sessions

**Weaknesses:**
- Vulnerable to **man-in-the-middle (MITM)** attacks: attacker intercepts A and B, substitutes their own values. Prevents authentication.
- **Solution:** Combine with digital signatures or pre-shared public keys for authentication (e.g., TLS handshake)

### Key Wrapping and Transport

Protecting keys in transit. Typically used when moving keys between systems or storing them.

**Envelope encryption:** User data encrypted with **data key** (DEK), DEK encrypted with **key encryption key** (KEK). KEK is stored separately and accessed only when needed.

Example:
```
plaintext → [AES-256] → ciphertext (encrypted with DEK)
DEK → [AES-256 with KEK] → wrapped key
Both ciphertext and wrapped key stored together
Decryption: decrypt wrapped key with KEK → get DEK → decrypt ciphertext
```

**Key wrapping standards:**
- **PKCS#1 v1.5:** RSA key transport (deprecated, vulnerable to padding oracle attacks)
- **PKCS#8:** Key structure (used with AES key wrap)
- **RFC 3394 (AES Key Wrap):** AES-based key wrapping (standard for envelope encryption)

---

## Key Storage and Key Management Systems

### At Rest

Keys must be encrypted when stored. Storage locations:

1. **Local filesystem:** Keys encrypted with KEK; KEK protected by passphrase or HSM
2. **Cloud KMS (AWS KMS, GCP Cloud KMS, Azure Key Vault):**
   - Keys never leave the provider's HSM
   - Encryption/decryption API (keys stay in the cloud)
   - Access control via IAM
   - Encryption context: additional data authenticated with the ciphertext (prevents key confusion)

3. **Sealed Secrets, Sealed Boxes:** Kubernetes secrets encrypted with cluster key; decryption only on the cluster
4. **HashiCorp Vault:** Self-hosted KMS; supports multiple auth methods, encryption backends, dynamic secrets

### Access Patterns

**Least privilege:** Services only access keys they need. JIT (just-in-time) access; time-limited tokens; cryptographic binding to the requesting principal (user/service identity).

**Audit trails:** All key operations logged (access attempts, decryption, rotation). Enable post-breach forensics.

---

## Key Rotation and Lifecycle Management

### Rotation Strategies

**Time-based:** Rotate keys at fixed intervals (daily, monthly, yearly). Balances security (limited exposure for compromised key) against operational cost.

**Event-based:** Rotate immediately after suspected compromise, employee termination, or security incident.

**Threshold-based:** Rotate after key used N times (for session keys or ephemeral keys).

### Key Ceremony

Formal procedure for generating and initializing high-security keys (e.g., root CA or master key). Attendees (usually ≥3):
- Key custodian (witness, non-technical)
- Cryptographer/security engineer (technical)
- Compliance officer (audit)

**Process:**
1. Gather in secure facility (air-gapped room, Faraday cage)
2. Generate key using hardware RNG or HSM (attendees don't handle plaintext directly)
3. Split key into shares (Shamir's secret sharing) distributed to different custodians
4. Store shares (encrypted) in different locations
5. Document: serial number, creation date, key material properties
6. Destroy all ephemeral data (RAM, temporary files)
7. All attendees sign the record

**Rationale:** No single person can access the key; quorum requirement prevents insider theft; ceremony creates audit trail.

### Split Knowledge / Secret Sharing

**Shamir's Secret Sharing:** Share a secret K into n shares such that any m shares (threshold) can reconstruct K, but fewer shares reveal nothing about K.

Example: 5 shares, threshold 3 means any 2 shares are worthless, but any 3 reveal the key.

Used in:
- Root key ceremonies (5 shares, threshold 3: 3 of 5 custodians must agree to reconstruct)
- Disaster recovery (store key backups with different teams; quorum required to restore)

**Implementation:** NIST SP 800-38D and open protocols (LWE-based, polynomial interpolation). Practically: Vault, PKCS#11 smart cards, or custom HSM integrations.

---

## Key Derivation Functions

### HKDF (HMAC-based Key Derivation, RFC 5869)

Derives multiple keys from a single source (e.g., shared secret from key exchange). Two phases:

**Extract phase:** Hash source material with salt to produce pseudorandom key (PRK)
```
PRK = HMAC(salt, input_key_material)
```

**Expand phase:** Generate multiple keys from PRK
```
key_1 = HMAC(PRK, info ∥ 0x01)
key_2 = HMAC(PRK, key_1 ∥ info ∥ 0x02)
key_3 = HMAC(PRK, key_2 ∥ info ∥ 0x03)
```

**Use cases:** TLS 1.3 (derive session keys), Noise Protocol, Signal protocol.

### PBKDF2 (Password-Based Key Derivation, NIST SP 800-132)

Derives encryption keys or authentication codes from passwords. Iterative HMAC: applies HMAC (salt, password) for N iterations (tunable for computational cost).

```
DK = PBKDF2(password, salt, iterations, length)
```

**Tuning:** Iterations ≥ 1M (typical: 10M–100M to slow down brute-force). Trade-off: slower for legitimate users, exponentially slower for attackers.

**Limitations:** GPU-friendly (simple iteration, no memory requirement). **Superseded by Argon2** for password-to-key derivation.

---

## Key Compromise and Revocation

### Compromise Detection

- Unauthorized key access (audit log anomalies)
- Unexplained decryption/signature operations
- Post-breach investigation (attacker reports, CVEs indicating credential theft)
- Key escrow backups disappearing (physical theft)

### Revocation

**For asymmetric keys:** Certificate Revocation Lists (CRLs) or OCSP (Online Certificate Status Protocol) published by the CA. TLS clients check revocation before trusting a certificate.

**For symmetric keys:** No standard revocation. Rely on key rotation and re-encryption of old data with new keys.

**For issued credentials:** Invalidate tokens (OAuth refresh tokens, S3 access keys) immediately. Reset password/MFA. Revoke any derived keys.

---

## Practical Architecture

**Typical tier structure:**

1. **Master key:** Root HSM key; highly restricted access; rarely rotated
2. **Key encryption key (KEK):** Derived from master key or stored in tier-2 HSM; rotated periodically
3. **Data encryption keys (DEK):** Generated per data object; encrypted with KEK; rotated on-demand
4. **Session/ephemeral keys:** Generated fresh for each session (TLS, SSH); short-lived

**Operational checklist:**
- ✓ Use cloud KMS (AWS KMS, GCP KMS, Azure Key Vault) if available
- ✓ Enable encryption in transit (TLS) and at rest (DEK+KEK)
- ✓ Implement envelope encryption (don't store keys with data)
- ✓ Log all key access
- ✓ Rotate keys ≥ annually; weekly or daily for high-value keys
- ✓ Use Argon2 for password→key derivation; HKDF for key expansion
- ✓ Implement quorum access (≥2 approvals) for sensitive operations

---

## See Also

- security-cryptography-symmetric.md (DEK/KEK, encryption algorithms)
- security-cryptography-asymmetric.md (elliptic curves, key exchange protocols)
- security-secrets-management.md (workflow automation, credential lifecycle)
- devops-secrets-rotation.md (operational rotation, dynamic secrets, automation)