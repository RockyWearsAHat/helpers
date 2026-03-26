# TLS Handshake Deep Dive — TLS 1.2 vs 1.3, Cipher Suites, Key Exchange & Session Resumption

## Overview

The **TLS (Transport Layer Security) handshake** is the initial negotiation phase where client and server agree on:
- Which cipher suite to use (symmetric encryption, key derivation, HMAC/AEAD mode)
- How to exchange keys securely (ECDHE, RSA, PSK)
- Server identity verification (X.509 certificates)
- Optional compression and protocol upgrades

TLS 1.3 (RFC 8446, 2018) dramatically simplified and accelerated the handshake vs. TLS 1.2 (RFC 5246, 2008). Modern browsers default to TLS 1.3; TLS 1.2 remains for compatibility.

## TLS 1.2 Handshake Flow (2-RTT)

```
Client                                    Server
  |  ClientHello                            |
  |  (supported versions, cipher suites,    |
  |   EC curves, key shares if supported)  |
  |---------------------------------------->|
  |                                      ServerHello
  |                                      (chosen cipher suite,
  |                                       curve, key share)
  |                                      Certificate[]
  |                                      ServerKeyExchange
  |                                      (ServerHelloDone)
  |<----------------------------------------|
  |                                         
  | ClientKeyExchange                     
  | (key share or RSA-encrypted secret)   
  | [ChangeCipherSpec]                    
  | Finished (MAC over handshake messages)
  |---------------------------------------->|
  |                                      [ChangeCipherSpec]
  |                                      Finished (MAC over
  |                                       handshake messages)
  |<----------------------------------------|
  |                                         
  | Encrypted application data
  |<--------- Symmetric encryption -------->|
```

### Key Exchange Methods (TLS 1.2)

- **ECDHE (Ephemeral Elliptic Curve Diffie-Hellman)**: Client and server each generate both a static key pair and an *ephemeral* key pair for this session. They exchange public ephemeral keys, each derives a shared secret via ECDH. **Forward secrecy**: Even if the server's private key is stolen later, past sessions cannot be decrypted (ephemeral keys are discarded). **Recommended** because of forward secrecy.
- **RSA**: Client generates a random master secret, encrypts it with the server's RSA public key. **No forward secrecy**: If server's private key is compromised, all past sessions are exposed.
- **DHE (Diffie-Hellman Ephemeral)**: Similar to ECDHE but using larger primes; deprecated due to computational cost and group size attacks (impersonation of weak DH parameters).
- **PSK (Pre-Shared Key)**: Client and server pre-share a secret. Rarely used in browser; common in IoT. Vulnerable if the shared secret is compromised.

### Cipher Suite Negotiation (TLS 1.2)

Client sends a **list of supported cipher suites** in order of preference; server picks **one** and announces it:

Example cipher suite: `ECDHE-RSA-AES-128-GCM-SHA256`
- **ECDHE**: Key exchange method
- **RSA**: Certificate signature algorithm (server proves key possession)
- **AES-128-GCM**: Symmetric cipher (128-bit AES in Galois/Counter Mode — authenticated encryption)
- **SHA256**: HMAC/PRF algorithm for key derivation

TLS 1.2 supports 37+ standard cipher suites. Weak suites (RC4, MD5, NULL encryption, export-grade) are disabled on modern systems.

### Server Certificate Verification (TLS 1.2)

Server sends `Certificate` message containing a chain: end-entity cert → intermediate(s) → root CA.

Client verifies:
1. **Chain validity**: Each cert signs the next; root is self-signed and trusted.
2. **Expiration**: Cert not before or after current time.
3. **Name matching**: Cert's Common Name or Subject Alternative Name (SAN) matches the requested hostname.
4. **Key constraints**: Cert key type (RSA, ECDSA) is acceptable.
5. **Extended Key Usage (EKU)**: Cert marked for TLS server use.

If verification fails, handshake is aborted. A client can choose to proceed (TOFU — Trust On First Use) in development but is risky.

### Session Resumption (TLS 1.2)

After a full handshake, the server may send a **session ID** or **session ticket** allowing the client to skip future handshakes:

- **Session ID**: Opaque 32-byte identifier. Server stores session state in RAM. Client sends session ID in ClientHello; server finds it in cache and sends `ServerHello` with matching ID. Both sides skip key exchange and immediately resume with the cached master secret. **Benefit**: ~1 RTT faster. **Cost**: Server-side session cache, memory overhead.
- **Session Ticket** (RFC 5077): Server encrypts session state and sends it to client (not stored on server). Client includes ticket in ClientHello; server decrypts and resumes. **Benefit**: Server-side stateless, scales to many clients. **Cost**: Ticket encryption key rotation is critical; leaked key can expose all issued tickets.

Both mechanisms have **forward secrecy limits**:
- Session ID/ticket uses the original master secret, which was derived from the ephemeral DH shared secret. If the server's key is compromised *before* the session, the master secret might be recoverable (depending on key exchange).
- True forward secrecy requires re-deriving ephemeral keys per session.

## TLS 1.3 Handshake Flow (1-RTT Standard, 0-RTT Optional)

```
Client                                   Server
  |  ClientHello                          |
  |  (supported versions, cipher suites, |
  |   key shares ECDHE/25519, PSK)      |
  |  + key material encrypted with      |
  |   early exporter secret (for 0-RTT) |
  |-------------------------------------->|
  |                                    ServerHello
  |                                    (chosen version, cipher,
  |                                     key share)
  |                                    {EncryptedExtensions}
  |                                    {CertificateRequest}
  |                                    {Certificate}
  |                                    {CertificateVerify}
  |                                    {Finished}
  |<--------------------------------------|
  |                                       
  |  {Certificate}                         
  |  {CertificateVerify}                   
  |  {Finished}                          
  |-------------------------------------->|
  |                                       
  | Encrypted application data (← mutually authenticated)
  |<---------- AEAD encryption ---------->|
```

### Key Improvements

1. **1-RTT Handshake**: Client sends key shares in ClientHello (not waiting for ServerHello). Server immediately replies with ServerHello + server key share. Both compute shared secret → handshake encryption keys. Finishes are encrypted; application data protected immediately.

2. **Cipher Suite Simplification**: Reduced from 37 to **5 recommended suites**:
   - `TLS_AES_128_GCM_SHA256`
   - `TLS_AES_256_GCM_SHA384`
   - `TLS_CHACHA20_POLY1305_SHA256`
   - `TLS_AES_128_CCM_SHA256`
   - `TLS_AES_128_CCM_8_SHA256`
   
   The cipher suite now specifies only the AEAD cipher and hash (key exchange and signature algorithms are negotiated separately). No more weak ciphers.

3. **Key Exchange Separation**: Key exchange (`supported_groups`), signature algorithm (`signature_algs`), and cipher suite are negotiated independently. Reduces complexity in suite naming.

4. **HKDF (HMAC-based Extract-and-Expand Key Derivation Function)**: More robust key derivation than PRF in TLS 1.2. Derives multiple keys (client handshake, server handshake, application) from the shared secret + salt in a structured way.

5. **Perfect Forward Secrecy by Default**: All TLS 1.3 handshakes use ephemeral DH (ECDHE); no static RSA or PSK-only options. The client must send key shares pre-computed before ServerHello, so doesn't delay on slow networks.

6. **Record Layer Simplification**: Removed compression (vulnerable to CRIME attack). Removed support for many legacy extensions. Streamlines code and reduces surface area.

### TLS 1.3 0-RTT (Early Data)

Client can send application data in the **ClientHello itself** using a Pre-Shared Key (PSK):

```
Client                                   Server
  |  ClientHello                          |
  |  + early_data (encrypted with PSK)   |
  |-------------------------------------->|
  |                                    ServerHello
  |                                    {Finished}
  |<--------------------------------------|
  | Application data (post-handshake)    |
```

**Design**: Client sends ClientHello + early application data (e.g., HTTP POST) encrypted with a PSK derived from a previous session. Server decrypts and processes early data before the full handshake is complete.

**Benefit**: 0-RTT — no round-trip delay for the first request. Particularly useful for HTTPS where a client reconnects to the same server.

**Risk**: **Replay attack**. An attacker can capture early data and replay it. The early data itself is authenticated (HMAC-bound to the handshake), but the server has no way to distinguish a replayed early-data from a genuine one without **replay detection**. Mitigations:
- Server-side timestamp validation or session token tracking.
- Application-level idempotency (e.g., HTTP GET is idempotent; POST should not be sent in early data).
- QUIC connections reject early data after a certain time window.

**When to use 0-RTT**:
- HTTPS with GET requests (safe, idempotent)
- Reconnecting to known servers
- Do NOT use for mutations (POST, DELETE, transfers).

## Session Resumption in TLS 1.3

TLS 1.3 removes session IDs and replaces them with **PSK (Pre-Shared Key) mode**:

- Server sends a **PSK identity** in the ServerHello and a separate **PSK session ticket** message after the main handshake.
- Ticket is opaque, encrypted by the server, and includes the session key + resumption_master_secret.
- Client stores the ticket. On the next connection, it includes the ticket in the `psk_identity` extension of ClientHello.
- Server decrypts the ticket, verifies integrity, and resumes the session without a full handshake.

**Difference from TLS 1.2**:
- TLS 1.3 tickets are **always stateless** on the server (no server-side cache).
- Tickets include the selected cipher suite and protocol version, ensuring compatibility.
- Multiple tickets can be sent (MUST issue at least one); client picks one or none.

**Forward secrecy in resumption**: TLS 1.3 derives a new master secret from the resumption_master_secret + hash of the full handshake, so even if a ticket is leaked, the attacker cannot decrypt the session traffic (the master secret is not directly stored).

## SNI, ALPN, and Certificate Pinning

### SNI (Server Name Indication, RFC 6066)

Client sends the requested **hostname** in the ClientHello (plaintext) so the server can select the correct certificate (multiple certs/domains on one IP). If SNI is missing, the server defaults to a single cert (often wrong domain).

**Privacy concern**: SNI leaks the target hostname in plaintext (visible to network observer). **ECH (Encrypted Client Hello)** in TLS 1.3 draft attempts to encrypt SNI; adoption is slow.

### ALPN (Application-Layer Protocol Negotiation, RFC 7301)

Client lists supported protocols (e.g., `["h2", "http/1.1"]`); server picks one. Enables HTTP/2, HTTP/3, or fallback to HTTP/1.1 on the same port. Negotiated during handshake, so no extra round-trip.

### Certificate Pinning

Client "pins" a certificate or public key, accepting **only that cert/key** even if signed by a valid CA. Prevents MITM via compromised CAs (e.g., a rogue CA issues a cert for your domain but signed by a legitimate root).

**Types**:
- **Public-key pinning**: Pin the leaf cert's public key or intermediate CA's public key.
- **Subject public-key pinning (SPKI)**: Pin the Base64-encoded SPKI (public key info) sent via HTTP header or embedded in app.
- **Certificate pinning**: Pin the entire cert chain.

**Pitfalls**:
- **Expiration**: If you pin a cert that renews, users cannot connect after renewal. Must carefully manage pin updates.
- **Backup pins**: Always pin 2+ certs (current + future/backup) to avoid locking out users.
- **Leaf vs intermediate vs root**: Pinning a root is fragile (used by many domains); pinning a leaf is strict but ages badly. Intermediate is a middle ground.
- **Preload lists**: Browsers maintain pinned certs for high-value sites (e.g., Google, Facebook) to prevent impersonation. User apps can define their own pins.

## TLS 1.2 vs TLS 1.3 Summary

| Aspect | TLS 1.2 | TLS 1.3 |
|--------|---------|---------|
| **Handshake RTT** | 2-RTT (or 1-RTT with session resumption) | 1-RTT (0-RTT with PSK + early data) |
| **Key Exchange** | ECDHE, RSA, DHE | ECDHE only (required) |
| **Forward Secrecy** | Optional (depends on suite) | Always via ephemeral DH |
| **Cipher Suites** | 37+ (include weak options) | 5 recommended (strong only) |
| **Session State** | Optional server-side cache | Always stateless (tickets) |
| **Verified Algorithms** | Negotiated in cipher suite | Separate negotiation (key exchange, sig alg, AEAD) |
| **Record Compression** | Supported (deprecated) | Removed |
| **Middlebox Compatibility** | Full support | `supported_versions` required for compatibility |

## Common Issues and Debugging

- **Handshake timeout**: Check network (RST/FIN), firewall (blocking certain cipher suites), or server not configured for TLS 1.3.
- **Wrong certificate**: Verify SNI is sent correctly; check server's certificate selection logic.
- **Cipher suite mismatch**: Client requests suites server doesn't support; update server config or client compatibility mode.
- **Session resumption failure**: Ticket expiration (servers set TTL on tickets), key rotation on server (tickets encrypted with old key cannot be decrypted).
- **0-RTT replay**: Application data sent in early-data is replayed; validate idempotency or add replay detection.

## See Also

- [Security — Cryptography (Symmetric, Asymmetric)](security-cryptography-symmetric.md) — encryption and key derivation internals
- [Security — Certificates](security-tls-certificates.md) — X.509 structure and CA trust models
- [Networking — HTTP](networking-http.md) — HTTP/2 and HTTP/3 which rely on TLS