# Symmetric Cryptography — Block Ciphers, Modes, and Key Derivation

## Overview

Symmetric encryption uses a single shared key for both encryption and decryption. Fast and practical for bulk data, but key distribution is a challenge (how do sender and receiver share the key securely?). Modern symmetric cryptography combines **block ciphers** (AES) with **modes of operation** (CTR, GCM) and **authentication** (AEAD) to provide both confidentiality and integrity.

---

## Block Ciphers: AES and ChaCha20

### AES (Advanced Encryption Standard)

Industrial standard cipher, selected by NIST in 2001. Based on the Rijndael design. Operates on 128-bit blocks.

**Key sizes:** 128, 192, 256 bits. Larger keys resist brute force; 128-bit keys are still considered secure (2^128 exhaustive search is infeasible), but 256-bit is preferred for defense against future technologies (hypothetical quantum speedups, unforeseen classical breaks).

**Internal structure:** Iterated substitution-permutation network. Data is arranged in 4×4 byte matrix, then processed through rounds (10 rounds for 128-bit key, 12 for 192-bit, 14 for 256-bit). Each round applies:
- **SubBytes:** Byte-wise substitution via S-box
- **ShiftRows:** Permutation of matrix rows
- **MixColumns:** Mixing within columns
- **AddRoundKey:** XOR with round key

This design provides strong diffusion and confusion; small input changes propagate throughout the block.

**Hardware support:** Modern CPUs (Intel AES-NI, ARM PMUL) include AES instructions. Encryption speed: ~3-10 cycles/byte. Software-only: 10-20x slower.

**Cryptanalysis:** No practical breaks. Theoretical attacks (e.g., biclique attack) require 2^254 operations for 256-bit keys — computationally infeasible. AES is considered secure through at least 2030s assuming quantum computing does not mature.

### ChaCha20

Stream cipher design (though usually used in authenticated mode ChaCha20-Poly1305). Based on ARX operations (addition, rotation, XOR) rather than substitution-permutation like AES.

**Structure:** Constant + nonce + counter are mixed with key through 20 rounds of ARX operations. Produces 512-bit (64-byte) keystream per block. Parallelizable: different counter values produce independent blocks.

**Advantages:**
- No hardware support needed (ARX is portable across all CPUs)
- Constant-time implementation is simpler than AES (fewer lookup tables, no data-dependent memory access)
- Simpler than AES, smaller code footprint
- No cache-timing side-channel vulnerability (unlike AES S-box lookups that depend on data)

**Adoption:** Preferred by cipher designers (Bernstein, Twofish author) for side-channel robustness. Used in WireGuard, TLS 1.3, Signal, and other applications prioritizing simplicity and side-channel resistance.

**Performance:** On general-purpose CPUs without AES-NI, ChaCha20 is competitive with AES-GCM. With AES-NI, AES-GCM is faster. On embedded systems without AES acceleration, ChaCha20-Poly1305 is preferred.

---

## Block Cipher Modes of Operation

Block ciphers encrypt fixed-size blocks (128 bits for AES). Real data is longer. Modes define how to encrypt multi-block messages and combine with authentication.

### ECB (Electronic Codebook) — Do Not Use

Simpliciter: encrypt each block independently with the same key. Same plaintext block always produces same ciphertext.

**Vulnerability:** Patterns in plaintext are preserved in ciphertext. Encrypting bitmap image with ECB reveals the image structure (identical areas become identical ciphertext). Attacker can sometimes guess plaintext blocks without cryptanalysis.

**Historical significance:** ECB demonstrates why modes are necessary. It is cited as example of insecure design in every textbook.

### CBC (Cipher Block Chaining)

Each block is XORed with **previous ciphertext** before encryption. First block uses **IV** (initialization vector).

```
C[1] = AES(P[1] ⊕ IV)
C[2] = AES(P[2] ⊕ C[1])
...
```

Decryption is parallelizable (each ciphertext block depends only on previous CT, not future ones), but encryption is sequential.

**IV requirement:** Must be **random and unpredictable** (not reused for same key). If attacker controls IV, they can craft chosen-plaintext attacks.

**Authentication:** CBC does not authenticate. Ciphertext can be modified; attacker crafts new blocks that decrypt to nonsense but are syntactically valid. Requires separate MAC for authentication (HMAC). Combining HMAC-then-CBC led to **padding oracle** vulnerabilities (PKCS#7 padding leaks whether decryption succeeded, allowing byte-by-byte decryption of unknown ciphertext).

**Status:** Dated for new applications. Effective but replaced by authenticated modes (GCM).

### CTR (Counter Mode)

Treat block cipher as a **keystream generator.** Counter is incremented for each block, encrypted with key, producing one block of keystream. XOR plaintext with keystream.

```
Keystream[i] = AES(Key, Nonce ‖ Counter[i])
Ciphertext = Plaintext ⊕ Keystream
```

**Advantages:**
- **Random access:** Can decrypt block i without decrypting blocks 1..i-1
- **Parallelizable:** All blocks are independent
- **Streaming:** Can encrypt data of any length (no padding needed)
- **Deterministic:** Same (key, nonce, counter) always produces same keystream

**Security requirement:** Nonce+counter pair must be **unique per message**. Reusing nonce with same key leaks plaintext (XOR of two messages encrypted with same keystream).

**Authentication:** Like CBC, CTR alone does not authenticate. Requires MAC. But CTR-MAC constructions like GMAC or Poly1305 are simpler than HMAC over CBC (no padding oracle).

### GCM (Galois/Counter Mode) — AEAD

Combines CTR encryption with **Galois field multiplication** for authentication. Produces authenticated ciphertext in single pass.

**Structure:**
1. Encrypt data using CTR mode
2. Compute **authentication tag** over ciphertext (and optionally associated authenticated data, AAD, that is not encrypted)
3. Tag is verification that ciphertext is unchanged and authentic

**Tag computation:** Uses Galois field (GF(2^128)) polynomial evaluation. Ciphertext and AAD are represented as polynomials, evaluated at secret point derived from key.

**Advantages:**
- **One-pass:** Encryption and authentication happen simultaneously
- **Parallelizable:** Both encryption and tag computation can be parallelized
- **Efficient:** Authentication is 1-2 multiplications per 128-bit block
- **Standard:** NIST approved, widely deployed (TLS 1.2/1.3, IETF protocols)

**Nonce-reuse vulnerability:** **Critical: If same (key, nonce) pair is used twice, all security is lost.** Attacker can XOR two ciphertexts to get XOR of plaintexts. Tag verification also fails, but attacker can forge any ciphertext. This is a protocol-level vulnerability — the cipher is correct, but users often misuse it.

**Tweak:** XPN-GCM (extended-nonce GCM) improves nonce handling but is less deployed.

---

## Authenticated Encryption (AEAD)

AEAD (Authenticated Encryption with Associated Data) is the modern paradigm. Encryption algorithm produces both ciphertext and an authentication tag. Decryption verifies tag; if invalid, rejects ciphertext (does not leak partial plaintext).

### ChaCha20-Poly1305

Combines ChaCha20 (stream cipher) with Poly1305 (one-time MAC). Poly1305 is a polynomial evaluation in modular arithmetic, keyed with unique key derived from ChaCha20 keystream.

**Advantages:** Both ChaCha20 and Poly1305 are designed for side-channel resistance. No lookup tables, all operations are data-oblivious (time independent of data). TLS ChaCha20-Poly1305 is widely supported.

**Nonce handling:** Nonce must still be unique, but Poly1305 constructs per-message key from nonce; nonce reuse does not fully compromise security (though authentication may fail).

---

## Key Derivation Functions (KDFs)

Symmetric encryption requires keys, but passwords are weak (low entropy). KDFs stretch passwords into strong cryptographic keys using **key stretching**: deliberately expensive computation.

### PBKDF2 (Password-Based Key Derivation Function 2)

Applies HMAC repeatedly: output = HMAC-SHA256(password, salt) iterated N times. Each iteration requires one HMAC computation.

**Parameters:**
- **N (iteration count):** 100,000+ recommended (as of 2025). Higher values slow brute-force password cracking.
- **Salt:** Random, unique per password. Prevents rainbow table attacks (precomputed tables of password→key for common passwords).
- **Output length:** Arbitrary (can fill 256-bit AES key)

**Trade-offs:**
- Simple, standardized (NIST approved, PKCS#5)
- Slow enough to discourage password guessing, but GPUs can evaluate millions/sec
- Does not use much memory (sequential computation), so GPU/ASIC can crack efficiently

**Status:** Dated. Still acceptable but superseded by memory-hard functions.

### Scrypt

Deliberate memory-hard KDF. Uses PBKDF2 in a way that requires **large temporary memory** (default 16 MB). Attacker building specialized hardware must provision expensive RAM.

**Parameters:**
- **N:** Memory cost (2^14 = 16 MB typical)
- **r, p:** CPU cost parameters
- **Salt:** Random unique value per password

**Strength:** Memory hardness raises cost of GPU/ASIC attacks (memory is expensive in specialized hardware).

**Weakness:** Vulnerable to cache-timing side-channel attacks. Memory access patterns depend on input; eavesdropper observing CPU cache behavior can extract information. Also vulnerable to time-memory trade-off attacks (Hellman tables).

**Adoption:** Widely used (cryptocurrency wallets, password managers).

### Argon2

State-of-the-art password hashing (won Password Hashing Competition in 2015). Memory-hard and explicitly designed against side channels.

**Variants:**
- **Argon2d:** Maximizes GPU resistance (data-dependent memory access), but vulnerable to side-channel timing attacks
- **Argon2i:** Resistant to side-channel attacks (data-independent access), but vulnerable to GPU/time-memory trade-offs
- **Argon2id:** Hybrid; two-pass algorithm uses d-mode first (GPU resistance) then i-mode (side-channel resistance). Recommended default.

**Parameters:**
- **M (memory):** Bytes allocated (100-1000 MB typical)
- **T (time):** Number of iterations
- **P (parallelism):** Number of parallel threads
- **Salt:** Random unique per password

**Strength:** Memory-hard + side-channel resistant + GPU-resistant. Tunable to future hardware speeds. Argon2id is cryptographically recommended as of 2025.

**Adoption:** Growing (frameworks, password managers, Kubernetes secrets).

---

## Side-Channel Attacks

Cryptographic algorithms are theoretically strong but implementations leak information through:

### Timing Side-Channels

Execution time leaks information. Example: comparing password byte-by-byte with early exit.

```
for i in range(len(stored_password)):
    if input_password[i] != stored_password[i]:
        return False  # Early exit
return True
```

If attacker can measure response time, they learn first differing byte (first correct byte takes long to return False). Byte-by-byte password recovery requires O(256 * password_length) attempts instead of O(256^password_length).

**Constant-time comparison:** Compare all bytes, then check final equality.

### Cache-Timing Attacks

CPU caches store recently accessed memory. L1/L2 cache misses take 10-100x longer than hits. Attacker observing cache occupancy infers data access patterns.

Example: AES S-box lookups depend on data (s_box[plaintext_byte]). Cache timing reveals which bytes were accessed, potentially leaking information about key or plaintext.

**Mitigation:** Use implementations with **constant-time S-box** (precomputed results in full, no conditional access) or no S-box at all (ChaCha20 uses only ARX, no data-dependent lookups).

### Power Analysis

Differential power analysis (DPA): Attacker measures power consumption during encryption. Power consumption correlates with number of bits being processed or data being moved. Can distinguish hypotheses about key values.

**Mitigation:** Use hardware with power randomization, random masking of intermediate values, or constant-power implementations (rare/expensive).

---

## Hardware Acceleration and Implementation

### AES-NI (Intel AES New Instructions)

Intel processors include AESENC, AESDEC, AESKEYGENASSIST instructions. Single instruction encrypts/decrypts one block. Enables AES-GCM at 1-5 cycles/byte (vs. 50-200 for software).

**Adoption:** All modern Intel/AMD processors include AES-NI. ARM processors include similar (PMUL for polynomial multiplication used in GCM authentication).

### Software vs. Hardware

- **Software:** Portable, flexible, easier to update. Vulnerable to side-channels if not careful.
- **Hardware:** Fast, potential for side-channel resistance (operations are isolated from other code), but inflexible if hardware is found broken (cannot patch).

Best practice: Use hardware engines when available, fall back to thoroughly tested software libraries (libsodium, OpenSSL) on platforms without acceleration.

---

## Practical Deployment

### Key Size Selection

For symmetric encryption:
- **128-bit keys:** Sufficient against classical computers through ~2030
- **256-bit keys:** Recommended for long-term confidentiality (decades)

Lattice-based quantum-resistant algorithms require much larger keys; symmetric cryptography is not threatened by quantum computers (Grover's algorithm provides only √N speedup, making 256-bit key equivalent to 128-bit classical).

### Authenticated Encryption Preference

Always use AEAD (GCM, ChaCha20-Poly1305, or AES-GCM). Never use bare stream ciphers or block cipher modes without authentication. Authenticated encryption prevents tampering, ensures ciphertext integrity, and detects truncation attacks.

### Nonce Management

- **CTR, GCM, ChaCha20-Poly1305:** Nonce must be unique per (key, message) pair
- Collision-resistant: Use random 96-bit nonce (probability of collision among 2^96 messages is negligible)
- Counter-based: Use monotonically increasing nonce (works if sender never crashes; crash creates risk of reuse)

### Key Rotation

Periodically replace keys to limit damage if key is leaked. Schedule: annually for long-lived services, more frequently for high-value data. Decryption must support historical keys (old messages were encrypted with old keys).

---

## See Also

- [security-cryptography-asymmetric.md](security-cryptography-asymmetric.md) — Key exchange and digital signatures
- [security-best-practices.md](security-best-practices.md) — Cryptography selection principles
- [cryptography-practical.md](cryptography-practical.md) — Developer-focused implementation guidance
- [math-number-theory.md](math-number-theory.md) — Mathematical foundations of number theory and groups