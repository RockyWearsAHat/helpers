# SSH (Secure Shell) — Protocol, Authentication, Tunneling & Hardening

## Overview

SSH (RFC 4253) is an encrypted protocol for secure command-line access and file transfer over untrusted networks. Three layers: transport (encryption, compression, MAC), authentication (password, public-key, certificate), and connection (channels, port forwarding). Replaced telnet and rsh. Ubiquitous in DevOps; the standard remote shell.

## Protocol Architecture

### Transport Layer
Establishes an encrypted channel:
1. **Server identification**: Both sides exchange version strings and supported algorithms (key exchange, ciphers, MACs, compression).
2. **Key exchange**: Server and client agree on a shared symmetric key using a key exchange algorithm (Diffie-Hellman, ECDH, Elliptic Curve Diffie-Hellman). The server's host key (public key) signs the exchange to prevent man-in-the-middle attacks.
3. **Encryption & authentication**: All subsequent data is encrypted with the shared key using the negotiated cipher (AES-128-CTR, ChaCha20-Poly1305, etc.). Each packet includes an HMAC to detect tampering.

### Authentication Layer
After the transport layer is encrypted, the client proves identity:
- **Password**: User types password; server verifies against stored hash.
- **Public-key**: Client proves possession of a private key by signing data the server provides (challenge-response).
- **Host-based**: Client proves it's connecting from a trusted host (rarely used; weak).
- **Keyboard-interactive**: Server sends prompts (e.g., "Password:", "2FA code:"); client responds. Flexible for multi-factor schemes.

### Connection Layer
After authentication, the client can open **channels** (logical streams within the SSH connection):
- **Session channel**: interactive shell or command execution
- **Forwarding channels**: tunneled TCP connections (see Port Forwarding)
- **X11 channels**: X11 display forwarding

Multiple channels share one SSH connection (multiplexing). Each channel type is negotiated; server can refuse (e.g., disable X11 forwarding).

## Key Exchange Algorithms

### Diffie-Hellman (DH) & ECDH
**Diffie-Hellman group exchange**: Client and server agree on modulus p and base g (often from predefined groups like RFC 3526). Client sends random a, computes g^a mod p. Server sends pre-computed public key and signs both keys' hash (host identity). Mutual computation via exponentiation yields a shared secret; this is hashed into symmetric keys.

Legacy DH (group1-sha1) with 1024-bit primes is weak; modern implementations use group14 (2048-bit) or larger.

**ECDH (Elliptic Curve Diffie-Hellman)**: Uses elliptic curve points instead of modular exponentiation (smaller keys, same security). Curves P-256, Curve25519 (modern standard; resistant to side-channel attacks). Faster than DH; preferred.

Both are authenticated: the server signs the exchange result with its host key (RSA, ECDSA, or ED25519). Prevents active MITM even if key exchange is broken (attacker can't forge the signature).

## Authentication Methods: Key Exchange in Detail

### Password Authentication
Client sends username and password encrypted (inside the SSH channel). Server verifies against stored credentials. Vulnerable to weak passwords, phishing (fake SSH banner), and credential stuffing. Disabling password auth (PermitRootLogin no, PasswordAuthentication no in sshd_config) is standard hardening.

### Public-Key Authentication
Client has locally stored private key (typically ~/.ssh/id_rsa, id_ed25519). Server has the public key in ~/.ssh/authorized_keys. Challenge-response:
1. Client sends username and public key ID.
2. Server generates a random challenge, sends it to client.
3. Client signs the challenge with its private key; sends signature.
4. Server verifies the signature with the public key. If valid, client is authenticated.

No password transmitted. Private key never leaves the client (except if stolen). Robust against phishing (private key is local; attacker must compromise client to steal it).

### SSH Keys vs. Certificates

**Public-key authentication (traditional)**: Each user key pair (1024+ bit RSA, 256-bit ECDSA, 256-bit ED25519). Authorized keys list on server matches client's public key.

**SSH certificates**: X.509-like structure. Issuer (CA, organization) signs the public key with a CA key, adding metadata (username, valid hosts, expiry, custom extensions). Server trusts the CA key, accepts any cert signed by it.

Certificate advantages:
- Centralized key management: CA issues certs; no need to distribute public keys manually.
- Expiry & rotation: certs expire; rotation is automatic (new cert issued, old one rejected).
- Custom extensions: certs can encode roles, access levels, or constraints (e.g., "valid only for production servers").

Certificates require a CA infrastructure (signing key, issuance workflow); keys are simpler but require manual distribution.

## SSH Agent & Key Management

**ssh-agent**: Daemon that holds decrypted private keys in memory. Client contacts the agent via Unix socket (SSH_AUTH_SOCK) instead of decrypting the key from disk each time. Avoids repeated password prompts for encrypted keys.

**ssh-add**: Loads a key into the agent.
```
ssh-add ~/.ssh/id_rsa
Agent stores the decrypted key. Subsequent SSH connections use the agent.
```

**Agent forwarding**: Over an SSH connection, enable SSH_AUTH_SOCK remote-forwarding so the remote server can use the local agent to authenticate further hops (jump hosts). Warning: agent forwarding trusts the remote server (it can access your agent and impersonate you on other systems). Use `ForwardAgent yes` cautiously; ProxyJump is safer.

## Port Forwarding

Tunnels TCP connections over SSH. Three modes:

### Local Forwarding (-L)
Forward local port to a remote host via SSH server. Syntax: `ssh -L local_port:remote_host:remote_port server`.

Example: `ssh -L 8080:internal-db:5432 jumphost`
- Client listens on localhost:8080
- Any connection to 8080 is tunneled through SSH server (jumphost)
- Traffic emerges from jumphost, connects to internal-db:5432
- Reply traffic returns through the tunnel

Use case: accessing remote databases or internal services from behind a firewall.

### Remote Forwarding (-R)
Forward a remote port to a local host via SSH server. Syntax: `ssh -R remote_port:local_host:local_port server`.

Example: `ssh -R 9000:localhost:3000 jumphost`
- Server listens on port 9000
- Traffic to server:9000 is tunneled back to the client
- Client connects to localhost:3000
- Responses returned through the tunnel

Use case: exposing a local development server to the remote network (for sharing demos or webhooks reaching back to dev machine).

### Dynamic Forwarding (-D, SOCKS Proxy)
Client becomes a SOCKS proxy. Syntax: `ssh -D local_port server`.

Example: `ssh -D 1080 proxy-server`
- Client listens on localhost:1080 (SOCKS5)
- Applications configured to use SOCKS proxy 127.0.0.1:1080
- All traffic is tunneled through SSH server
- Server resolves hostnames; responses sent back

Use case: routing all traffic through an encrypted tunnel (VPN-like), bypassing local network filtering.

## ProxyJump & Chaining Connections

**ProxyJump (-J)**: Connect to a target through one or more intermediate hosts. Syntax: `ssh -J jumphost user@target`.

Example: `ssh -J bastion user@internal-server`
- SSH connects to bastion, opens a channel
- Through that channel, connects to internal-server
- User's SSH connection spans both hops

Advantages over ProxyCommand (older method using stdio redirection):
- Cleaner: `-J` is simpler than `-o ProxyCommand='ssh bastion nc %h %p'`
- Agent forwarding: credentials flow through the chain securely
- Native: built into ssh directly, no external tools

Multiple jumps: `ssh -J bastion1,bastion2,bastion3 user@target`

Config file (ssh_config):
```
Host internal-server
  User alice
  HostName 10.0.0.5
  ProxyJump bastion
  
Host bastion
  User bob
  HostName 203.0.113.1
```

## SSH Configuration & Security Hardening

### Server-Side (sshd_config)
```
# Disable password auth
PasswordAuthentication no

# Disable root login
PermitRootLogin no

# Restrict SSH version
Protocol 2

# Disable X11 forwarding if not needed
X11Forwarding no

# Disable agent forwarding if not trusted
AllowAgentForwarding no

# Limit port forwarding
AllowTcpForwarding no  # or "yes" if needed

# Enforce key types
PubkeyAcceptedAlgorithms ssh-ed25519,ecdsa-sha2-nistp256

# Restrict login attempts
MaxAuthTries 3
```

### Client-Side (ssh_config)
```
Host *
  AddKeysToAgent yes
  IdentityFile ~/.ssh/id_ed25519
  IdentityFile ~/.ssh/id_rsa
  StrictHostKeyChecking accept-new  # accept new keys once, then verify
  HashKnownHosts yes  # hash hostnames in known_hosts for privacy
  ServerAliveInterval 60  # keep alive
```

### Key Strengths
- **ED25519**: 256-bit, modern, resistant to side-channel attacks. Preferred for new keys.
- **ECDSA-P256**: 256-bit, standard; weaker than ED25519 only by maturity (newer algorithm).
- **RSA-4096**: 4096-bit, backward compatible, but slower. RSA-2048 is considered weak (breakable within decades).

Recommendation: ED25519 for new deployments; phase out RSA-2048 and ECDSA-P384.

## SSH Certificates vs. Keys: Practical Trade-offs

| Aspect | Keys | Certificates |
|--------|------|--------------|
| Setup | Simple (generate, distribute public key) | Requires CA infrastructure |
| Expiry | No expiry; manual revocation via known_hosts removal | Automatic expiry; rotation simple |
| Compromise | Private key compromise = full access (revocation slow) | Cert compromise limits damage to cert's lifetime |
| Centralization | Manual distribution per server | CA issues; trust CA key |
| Use Case | Small teams, few servers | Large orgs, frequent rotation, compliance |

## SFTP & SCP

**SCP (Secure Copy)**: Transfers files by invoking SSH, then executing `scp` on the remote server (proprietary protocol over SSH). Simple but not parallel-friendly.

**SFTP (SSH File Transfer Protocol)**: Proper file transfer over SSH channel. Stateful protocol (open file, read/write, close). Client libraries (libssh2, Paramiko) support SFTP. Advantages: parallel transfers, resume capability, directory operations. Standard for automated transfers.

## Real-World Hardening & Monitoring

- **Use ED25519 keys**, disable password auth, disable root login
- **Certificate-based auth** for automated systems, frequent rotation
- **SSH bastion hosts** (jump servers) for internal access; no direct inbound SSH from internet
- **Audit logging**: log auth attempts, key additions, forwarding usage
- **Fail2Ban**: monitor logs, block IPs after failed attempts
- **OpenSSH server configuration**: match blocks for per-user/host policies
- **SSH CA**: self-host or use HashiCorp Vault for automated cert issuance

## Key References

- RFC 4253 (SSH Transport Layer Protocol)
- RFC 4252 (SSH Authentication Protocol)
- RFC 4254 (SSH Connection Protocol)
- OpenSSH man pages: ssh_config, sshd_config, ssh-keygen
- NIST SP 800-63B (Authentication guidance; ED25519 recommended)
- Teleport documentation (SSH CA & bastion patterns)