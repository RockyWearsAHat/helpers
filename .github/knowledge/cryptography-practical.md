# Practical Cryptography for Developers

## Golden Rules
1. **Never roll your own crypto.** Use well-audited libraries (libsodium, OpenSSL, Web Crypto API).
2. **Encryption is not authentication.** Encrypted data can be tampered with. Use authenticated encryption (AEAD).
3. **Hashing is not encryption.** Hashes are one-way. You can't "decrypt" a hash.
4. **Keys are the hardest part.** The algorithm is public. Security comes from key management.
5. **Crypto is easy to use wrong.** One mistake (reused nonce, timing leak, weak RNG) breaks everything.

## Hashing

### Cryptographic Hash Functions
```
Input (any size) → Fixed-size output (digest)
Properties: deterministic, fast, one-way, collision-resistant, avalanche effect
```

| Algorithm | Output | Status | Use for |
|-----------|--------|--------|---------|
| MD5 | 128-bit | BROKEN | Checksums only (not security) |
| SHA-1 | 160-bit | BROKEN | Legacy, being phased out |
| SHA-256 | 256-bit | Secure | General purpose, Bitcoin |
| SHA-384/512 | 384/512-bit | Secure | When you need longer digest |
| SHA-3 | Variable | Secure | Alternative construction to SHA-2 |
| BLAKE2 | Variable | Secure | Faster than SHA-2, great for files |
| BLAKE3 | 256-bit | Secure | Parallel, extremely fast |

### Password Hashing (NOT the same as cryptographic hashing!)
Regular hashes are too fast. Attackers can try billions per second.

| Algorithm | Status | Notes |
|-----------|--------|-------|
| bcrypt | Good | Time-tested, 72-byte input limit |
| scrypt | Good | Memory-hard (resists GPU attacks) |
| Argon2id | Best | Winner of Password Hashing Competition (2015). Memory-hard + time-hard |
| PBKDF2 | Acceptable | NIST approved, but not memory-hard |
| MD5/SHA unsalted | CATASTROPHIC | Rainbow tables crack instantly |

```python
# Python: Use passlib or argon2-cffi
from argon2 import PasswordHasher
ph = PasswordHasher()
hash = ph.hash("password123")           # Includes salt automatically
ph.verify(hash, "password123")          # Returns True or raises exception

# Node.js: Use bcrypt
import bcrypt from 'bcrypt';
const hash = await bcrypt.hash('password123', 12);  // 12 rounds
const match = await bcrypt.compare('password123', hash);
```

## Symmetric Encryption (Same Key for Encrypt & Decrypt)

### AES (Advanced Encryption Standard)
The standard. 128, 192, or 256-bit keys.

**Modes of operation:**
| Mode | Properties | Use? |
|------|-----------|------|
| ECB | Identical blocks → identical ciphertext. The penguin problem. | NEVER |
| CBC | Each block XORed with previous. Needs IV. | Legacy only |
| CTR | Counter mode. Parallelizable. | With HMAC |
| **GCM** | Counter + authentication tag. AEAD. | **YES — use this** |
| **ChaCha20-Poly1305** | Stream cipher + MAC. AEAD. | **YES — alternative to AES-GCM** |

### AEAD (Authenticated Encryption with Associated Data)
```
Encrypt: plaintext + key + nonce + associated_data → ciphertext + auth_tag
Decrypt: ciphertext + key + nonce + associated_data + auth_tag → plaintext (or FAIL)
```
The auth tag ensures the data hasn't been tampered with. Associated data (e.g., headers) is authenticated but not encrypted.

```python
# Python: Use cryptography library
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
import os

key = AESGCM.generate_key(bit_length=256)
nonce = os.urandom(12)  # MUST be unique per encryption with same key
aad = b"authenticated-but-not-encrypted"

aesgcm = AESGCM(key)
ciphertext = aesgcm.encrypt(nonce, b"secret data", aad)
plaintext = aesgcm.decrypt(nonce, ciphertext, aad)
```

**CRITICAL:** Never reuse a nonce with the same key. AES-GCM with a repeated nonce is catastrophically broken (leaks authentication key).

## Asymmetric Encryption (Public/Private Key Pairs)

### RSA
- **Key sizes:** 2048-bit minimum (4096 recommended)
- **Slow:** Only encrypts small data (< key size). Encrypt a symmetric key, then use that for bulk data.
- **OAEP padding:** Use RSA-OAEP, never PKCS#1 v1.5 (padding oracle attacks)

### Elliptic Curve Cryptography (ECC)
- **Smaller keys, same security:** 256-bit ECC ≈ 3072-bit RSA
- **Faster** than RSA for signing
- **Curves:** P-256 (NIST), P-384, Curve25519 (preferred — no backdoor concerns), Ed448

### Key Exchange: Diffie-Hellman / ECDH
Two parties derive a shared secret over an insecure channel without ever transmitting it.
```
Alice: generates private_a, sends public_a = g^private_a
Bob:   generates private_b, sends public_b = g^private_b
Both:  shared_secret = public_other^private_own
       Alice computes: public_b^private_a = g^(private_a * private_b)
       Bob computes:   public_a^private_b = g^(private_a * private_b)
       Same value! Attacker sees public_a and public_b but can't derive the secret.
```

## Digital Signatures

```
Sign:    message + private_key → signature
Verify:  message + signature + public_key → valid/invalid
```

| Algorithm | Notes |
|-----------|-------|
| RSA-PSS | RSA-based, use PSS padding |
| ECDSA | Elliptic curve, widely used (Bitcoin, TLS) |
| Ed25519 | Curve25519-based, fast, no nonce needed, recommended |

```bash
# Generate Ed25519 SSH key
ssh-keygen -t ed25519 -C "email@example.com"

# Sign a git commit
git config user.signingkey ~/.ssh/id_ed25519.pub
git commit -S -m "signed commit"
```

## HMACs (Hash-based Message Authentication Code)

```
HMAC(key, message) → tag
```
Proves that the message was created by someone with the key AND hasn't been modified.

**NOT the same as:** `hash(key + message)` — this is vulnerable to length extension attacks!

```python
import hmac, hashlib

tag = hmac.new(key, message, hashlib.sha256).digest()

# Verify (timing-safe comparison!)
hmac.compare_digest(tag, received_tag)  # NOT tag == received_tag
```

## Common Crypto Mistakes (Each One Has Caused Real Breaches)

### 1. Timing Attacks
```python
# WRONG — early exit leaks information about how many bytes match
if computed_mac == received_mac:  # String comparison

# RIGHT — constant-time comparison
import hmac
hmac.compare_digest(computed_mac, received_mac)
```

### 2. Using Math.random() / random.random() for Security
```python
# WRONG — predictable PRNG
import random
token = random.randint(0, 2**128)

# RIGHT — cryptographically secure
import secrets
token = secrets.token_hex(32)
```

### 3. ECB Mode (The Penguin Problem)
ECB encrypts identical blocks identically. An encrypted image of a penguin... still looks like a penguin. Always use GCM or ChaCha20-Poly1305.

### 4. Storing Passwords in Plaintext or Reversible Encryption
Passwords should be hashed (Argon2id/bcrypt), never encrypted. If you can retrieve the original password, your design is wrong.

### 5. Nonce Reuse
AES-GCM with a reused nonce leaks the authentication key. ChaCha20-Poly1305 with a reused nonce reveals XOR of plaintexts.

### 6. Rolling Custom Token Generation
```python
# WRONG — predictable "random" ID
user_id_hash = md5(str(user_id)).hexdigest()

# RIGHT — opaque, unpredictable token
token = secrets.token_urlsafe(32)
```

## Key Management

### Principles
1. **Never hardcode keys** in source code, config files, or environment variables in images
2. **Rotate keys** regularly (and have a rotation plan before you need one)
3. **Use a KMS** (Key Management Service): AWS KMS, GCP KMS, Azure Key Vault, HashiCorp Vault
4. **Separate encryption keys from data** — different storage, different access controls
5. **Key derivation**: Use HKDF to derive multiple keys from one master key

### Envelope Encryption (How Cloud KMS Works)
```
1. Generate a DEK (Data Encryption Key) locally
2. Use DEK to encrypt your data
3. Send DEK to KMS, which encrypts it with a KEK (Key Encryption Key)
4. Store encrypted DEK alongside encrypted data
5. To decrypt: send encrypted DEK to KMS → get DEK → decrypt data
```

## JWT (JSON Web Tokens)

```
header.payload.signature
```

### JWT Gotchas
- **`alg: none` attack**: Some libraries accept unsigned tokens. Always require and validate the algorithm.
- **`alg: HS256` with RSA public key**: Attacker switches from RS256 to HS256, using the public key as HMAC secret. Verify algorithm type.
- **No expiration**: Always set `exp` claim. Short-lived (minutes, not days).
- **Sensitive data in payload**: JWT payload is base64-encoded, NOT encrypted. Anyone can read it.
- **Can't be invalidated**: JWTs are stateless. Once issued, they're valid until expiration. Use short expiry + refresh tokens.

---

*"The only truly secure system is one that is powered off, cast in a block of concrete, and sealed in a lead-lined room." — Gene Spafford. Everything else is risk management.*
