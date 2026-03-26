# Cryptographic Hash Functions — SHA-2, SHA-3, BLAKE2/3, Merkle Trees & Password Hashing

## Overview

Cryptographic hash functions map arbitrary-length inputs to fixed-length digests (128–512 bits) deterministically. A single bit change in input completely changes the output (avalanche effect). Hash functions are **one-way**: computing input from output is computationally infeasible (pre-image resistance). The same input always produces the same output (determinism). Used in digital signatures, message authentication codes (HMACs), integrity verification, commitment schemes, and blockchain systems.

---

## Cryptographic Properties

### Pre-image Resistance
Given digest D, finding any input that hashes to D is computationally infeasible. For a digest of n bits, an attacker needs ~2^n hash evaluations (brute force).

### Collision Resistance
Finding two distinct inputs with identical digests is computationally infeasible. For n-bit digests, birthday paradox suggests ~2^(n/2) evaluations are needed. A 256-bit hash has theoretical collision difficulty of ~2^128 operations.

### Second Pre-image Resistance
Given input M producing digest D, finding a different input M' that hashes to the same D is infeasible. Related to pre-image resistance but distinct in some constructions.

---

## Hash Function Families

### SHA-2: FIPS 180-4 Standard

The Merkle-Damgård construction: input is padded, split into 512-bit (or 1024-bit for SHA-512) blocks, and processed sequentially through a compression function. Each block depends on the previous block's state, creating a chain of dependencies.

**Variants:**
- **SHA-256**: 256-bit output. Industrial standard for digital signatures, TLS, and blockchain (Bitcoin). No known practical attacks.
- **SHA-512**: 512-bit output. Stronger collision resistance; used in security-critical applications.
- **SHA-224, SHA-384**: Truncated versions (256-bit and 512-bit outputs respectively).

**Strengths:** Well-vetted, vetted by cryptographic community for 20+ years, hardware support (SHA-NI), standard in TLS 1.3, trusted for HMAC and key derivation.

**Weaknesses:** Merkle-Damgård construction has length-extension vulnerability (attacker can compute H(M ∥ extra_data) knowing only H(M) and length of M). Not critical for most applications but requires careful HMAC implementation.

### SHA-3 (Keccak): FIPS 202 Standard

Sponge construction (fundamentally different from Merkle-Damgård). Unlike SHA-2's sequential block chaining, sponge maintains an internal state (1600-bit permutation for SHA-3). Input is absorbed into state via XOR-and-permutation, then output is squeezed from state.

**Variants:**
- **SHA3-256, SHA3-512**: Compete directly with SHA-256/SHA-512 but with sponge design.
- **SHAKE128, SHAKE256**: Extendable output format (XOF). Can produce arbitrary-length outputs. Useful for key derivation and other applications needing variable-length digests.

**Advantages:** No length-extension attacks (sponge design inherently resistant). Parallelizable (theoretically, different parts of state can be computed independently). More flexible in output length (SHAKE variants).

**Adoption:** Slower adoption than SHA-2 (established after 2015). NIST's official SHA-3 standard. Used mainly where length-extension matters or arbitrary output length is needed.

### BLAKE2 and BLAKE3

Modern hash design emphasizing security and speed. BLAKE2 (RFC 7693) is based on ChaCha20 stream cipher's core operations (ARX: addition, rotation, XOR). Faster than MD5 on modern CPUs while maintaining security properties stronger than SHA-2.

**BLAKE2b:** 512-bit output, 64-bit state.
**BLAKE2s:** 256-bit output, 32-bit state (embedded systems).
**BLAKE3:** 2023 iteration improving parallelism. Merkle tree structure allows concurrent hashing of large data. Each 1KB block is hashed independently, then parent hashes combine in a tree. Extremely fast for large files.

**Advantages:**
- 2–3× faster than SHA-2 on general-purpose CPUs
- Simpler design (easier to audit, smaller codebases)
- Built-in HMAC-like functionality (keyed hash mode)
- BLAKE3 enables parallelism without sacrificing locality

**Current status:** BLAKE2 is gaining adoption in cryptography-aware communities (Signal, WireGuard libraries). BLAKE3 still gaining traction (widely referenced but not yet standardized by NIST). Used in newer protocols where speed and flexibility matter more than institutional standardization.

---

## Broken Hash Functions: MD5 and SHA-1

### MD5

Produces 128-bit digests. Designated NIST standard in 1992. **Completely broken for cryptographic use.**

**Known breaks:**
- Practical collision attacks (2004, Wang et al.): two different inputs on a supercomputer in hours.
- By 2020, trivial collision generation on commodity hardware.
- **Recommendation:** Do not use for signatures, integrity checks, or any cryptographic operation. Use only for non-security checksums (file deduplication, disk integrity) when collision attacks are irrelevant.

### SHA-1

Produces 160-bit digests. NIST standard until 2010. **Cryptographically broken but not yet fully exploited at scale.**

**Known breaks:**
- Theoretical attacks (2005, Joux) reduced effort to ~2^69 from 2^80.
- SHAttered collision (2017): two different PDF files with same SHA-1. Computed using ~9.2 quintillion SHA-1 calls on GPU cluster.
- NIST deprecated SHA-1 for cryptographic use in 2011. EU recommended complete avoidance by 2016.

**Current guidance:** Avoid for new applications. Legacy systems using SHA-1 for TLS signatures or code signing should migrate. SHA-1 persists in git (object identifiers), but collision risk is lower there because attackers cannot control both inputs.

---

## Merkle Trees and Hash-Based Signatures

### Merkle Trees

Organizing n data items into a binary tree where leaf nodes are item hashes, and parent nodes are hashes of concatenated children. Root hash is the "fingerprint" of the entire dataset.

**Properties:**
- Root changes if any leaf changes
- Proof of membership: O(log n) hashes needed to verify one leaf
- **Merkle path:** sequence of sibling hashes from leaf to root

**Applications:**
- **Git:** Uses Merkle tree of blobs/trees; root commit hash identifies entire repository history
- **Blockchain:** Bitcoin blocks group transactions into Merkle tree; root (Merkle root) is stored in block header
- **Torrenting:** Verification that downloaded chunks match expected hash tree

### Hash-Based Signatures (Lamport, Merkle signatures)

Signatures without public-key infrastructure overhead. Lamport signature: sign one bit at a time. Issue: signatures are as long as the message. Merkle signatures organize many Lamport keys in a tree, signing with the tree root.

**Practical use:** Post-quantum cryptography (hash-based schemes are considered quantum-resistant because cryptanalysis depends only on hash function strength, not number theory). SPHINCS+ is a stateless hash-based signature scheme standardized for post-quantum use.

---

## Password Hashing: bcrypt, scrypt, Argon2

Password hashing differs from general cryptographic hashing. Goals: slow down brute-force attacks (computational cost), resist GPU/ASIC optimization (memory requirements), derive strong keys from weak passwords.

### bcrypt

Based on Blowfish cipher. Includes adjustable **cost factor**: number of hashing iterations (currently 10–12 is standard; cost factor of 12 = 2^12 iterations). Each additional cost factor doubles computation time.

**Structure:** Salt (16 bytes) + cost factor + hash. Output: 60-character string.

**Advantages:** Slow by design, built-in salt, resistant to GPU acceleration (requires Blowfish implementation, not just table lookups).

**Disadvantages:** Fixed to 72-byte password input (longer passwords truncated), no memory hard requirement, designed in 1999 (Blowfish not the fastest modern cipher).

### scrypt

Key derivation function (RFC 7914). Combines salt, password, and memory-hard computation: purposefully uses large amounts of RAM to resist GPU/ASIC attacks.

**Parameters:**
- **N:** CPU/memory cost (typically 2^14–2^20)
- **r:** Block size
- **p:** Parallelization
- Derived key can be any length

**Advantages:** Memory-hard; salted; tunable parameters; suitable for deriving encryption keys from passwords.

**Disadvantages:** Research showed potential GPU optimization (not as memory-hard as originally hoped in practice on modern hardware).

### Argon2

OWASP's recommended choice (2024). Memory-hard KDF with built-in resistance to GPU/ASIC attacks. Two variants: Argon2i (side-channel resistant, slower), Argon2id (balanced).

**Parameters:**
- **m:** Memory usage (MiB)
- **t:** Time cost (iterations)
- **p:** Parallelism

**Advantages:**
- Designed for modern threats (GPU/ASIC attacks)
- Flexible output length
- Tunable for different security/performance tradeoffs
- Standard (selected as password hashing champion in 2015)

**Recommended settings (OWASP 2024):** m=19 MiB, t=2 iterations, p=1 parallelism for typical web services.

---

## Practical Recommendations

**Signatures/integrity:** SHA-256 (conservative, well-established). SHA-3-256 if length-extension matters. BLAKE3 for high-throughput systems.

**Passwords:** Argon2id with OWASP settings. If forced to use older systems: bcrypt (cost factor ≥ 12).

**Key derivation:** HKDF (RFC 5869) for key expansion; Argon2 for password-to-key derivation.

**Performance-critical:** BLAKE2b or BLAKE3 (2–10× faster than SHA-2).

**Legacy systems:** Migrate SHA-1 signatures immediately. Continue using SHA-1 for git only until replacements are universal.

---

## See Also

- security-cryptography-symmetric.md (HMAC, authenticated encryption)
- security-cryptography-asymmetric.md (signatures, key exchange)
- cryptography-key-management.md (key derivation, storage)
- security-secrets-management.md (password storage infrastructure)