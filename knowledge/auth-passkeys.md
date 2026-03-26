# Passkeys and WebAuthn: FIDO2, Public Key Credentials & Passwordless Authentication

## Overview

**Passkeys** are cryptographic credentials stored on user devices (phone, laptop, security key) that enable passwordless authentication. Built on **WebAuthn** standard (W3C) and **FIDO2** protocol (FIDO Alliance), passkeys replace passwords with public-key cryptography: the user proves possession of a device and optionally provides a biometric or PIN to unlock it. Passkeys are phishing-resistant, cannot be compromised by server breaches, and require device possession — making them the strongest consumer and enterprise authentication mechanism.

---

## Core Architecture

### Public Key Cryptography Foundation

Passkeys use asymmetric encryption:

- **User's private key:** Stored securely on device; never leaves it; used to sign challenges
- **Public key:** Registered with the service; service uses it to verify signatures
- **Never transmitted:** Private key never reaches the server or network

Authentication flow:
1. Server sends a **challenge** (random nonce) to device
2. Device unlocks private key (biometric or PIN)
3. Device signs challenge with private key: `signature = sign(challenge, private_key)`
4. Device sends `signature` and `public_key` to server
5. Server verifies: `verify(challenge, signature, public_key)` — should be true
6. **Result:** User is authenticated; attacker cannot forge signature without private key

### Credential Registration

During account creation with passkeys:

1. User clicks "Add Passkey" or "Set Up Passwordless Login"
2. Browser/app calls `navigator.credentials.create()` with:
   ```javascript
   const credential = await navigator.credentials.create({
     publicKey: {
       challenge: new Uint8Array(32),  // Random challenge from server
       rp: { name: "Example App", id: "example.com" },
       user: {
         id: new Uint8Array(16),       // Unique user ID
         name: "user@example.com",
         displayName: "John Doe"
       },
       pubKeyCredParams: [{ alg: -7, type: "public-key" }],  // ES256
       timeout: 60000,
       attestation: "direct"           // Optional proof of key origin
     }
   });
   ```

3. Device prompts for unlock (biometric, PIN, or no prompt if security key)
4. Device generates key pair and stores private key securely
5. Device returns:
   - `PublicKeyCredential` with public key
   - `Attestation` (proof key was created on genuine hardware; optional)
   - `AttestationObject` (device metadata)

6. Server validates attestation (optional), stores public key associated with user
7. Credential now registered

### Authentication (Assertion)

When user returns to log in:

1. Server generates challenge and sends it to device
2. Browser/app calls `navigator.credentials.get()`:
   ```javascript
   const assertion = await navigator.credentials.get({
     publicKey: {
       challenge: new Uint8Array(32),  // Challenge from server
       timeout: 60000,
       rpId: "example.com"
     }
   });
   ```

3. Device lists available passkeys for the user (or website)
4. User selects credential, unlocks with biometric/PIN
5. Device signs challenge, returns `AuthenticatorAssertionResponse`:
   - `clientData`: Challenge, origin, type ({"type":"webauthn.get"})
   - `authenticatorData`: User presence flag, sign count
   - `signature`: Challenge signed with private key
   - `userHandle`: User ID (encrypted, optional)

6. Server verifies:
   - `signature` is valid for this challenge and public key
   - `origin` matches expected domain (prevents phishing)
   - `signCount` increased from previous login (detects cloned authenticators)
7. User logged in

---

## Platform vs Roaming Authenticators

### Platform Authenticators (Built-In)

Device-native credentials, synced to user's account:

| Authenticator | Platform | Sync | Unlock |
|---------------|----------|------|--------|
| **Face ID** | iPhone, Mac | iCloud Keychain | Biometric |
| **Touch ID** | iPhone, iPad, Mac | iCloud Keychain | Biometric |
| **Windows Hello** | Windows 11+ | Not synced (local) | Biometric / PIN / Windows login |
| **Android Biometric** | Android 9+ | Device-specific or Google Account | Biometric |

**Characteristics:**
- Passkey tied to device; user must approve sync across devices
- Highest user experience: one-tap biometric unlock
- Passkey can be recovered/synced if device is lost (depends on platform)
- Best choice: First authentication method for web/app login

### Roaming Authenticators (External)

Portable hardware security keys:

| Device | Type | Auth Method |
|--------|------|-------------|
| **YubiKey 5** | USB-A / NFC | Touch button |
| **Titan Security Key** | USB-C / NFC | Touch button |
| **Titan 2 (Titan M2)** | Dedicated | Touch button / NFC |
| **Solo** | USB-C / NFC | Touch button |

**Characteristics:**
- Independent device; works with any computer/phone (via USB-C, NFC, Bluetooth)
- No biometric unlock (security key itself is the authenticator; possession = proof)
- Best for: High-security accounts, multiple devices, organizations

**Hybrid Transport (Bluetooth):**
- User has phone + desktop
- Desktop requests passwordless login
- Phone displays permission prompt
- User approves on phone (nearby Bluetooth)
- Desktop completes authentication
- Best user experience for cross-device scenarios

---

## Passkey Sync and Recovery

### Synced Passkeys (Platform)

Some platforms allow passkey sync across user's devices:

- **iCloud Keychain (Apple):** Passkeys synced across all user's Apple devices; recovered via iCloud account
- **Google Password Manager:** Passkeys synced across Android + web; recovered via Google account
- **Microsoft Account:** Windows Hello credentials can be synced via Microsoft account
- **Samsung Pass:** Synced across Samsung devices; recovered via Samsung account

**Trade-off:**
- ✅ Convenience: Access passkey from multiple devices
- ❌ Dependency: Account recovery tied to cloud provider
- ❌ If cloud provider account compromised, passkeys are at risk

### Non-Synced Passkeys

- **Windows Hello (local):** Passkey only on this device; cannot sync
- **Security Keys:** Physical possession required; recovery = buy backup key

---

## Conditional UI (Autofill)

Modern platforms (iOS 16+, Android, Windows 11) support **conditional UI**, enabling seamless passkey autofill:

```javascript
const assertion = await navigator.credentials.get({
  publicKey: { challenge: new Uint8Array(32) },
  mediation: "conditional"  // Allow autofill
});
```

When user taps password field:
1. Autofill suggestions appear (including passkeys for this site)
2. User selects passkey, unlocks with biometric
3. Form auto-fills, authentication completes
4. UX: Same as saved password, but phishing-resistant

**Result:** Users can use passkeys without understanding WebAuthn; it feels like password managers but without the password.

---

## Attestation and Attestation Conveyance

**Attestation** is optional proof that the passkey was created on a legitimate device (e.g., genuine Apple device, authorized Yubikey).

### Why Attestation Matters

- Prevents cloned or impersonated authenticators
- Enterprises verify hardware source before allowing login
- Regulatory: Proof that credential meets compliance standards

### Attestation Formats

| Format | What It Proves | Use Case |
|--------|----------------|----|
| **Direct** | Device vendor signs: "I created this key" | High-security, enterprise |
| **Indirect (Surrogate)** | Intermediary (CA) signs key origin | Common, balance privacy + assurance |
| **None** | No proof; server trusts device type | Consumer apps (default) |
| **Enterprise** | Corporate authenticator; key created on managed device | Enterprise / compliance |

### Attestation Conveyance Strategies

```javascript
// High-security enterprise: require attestation
attestation: "direct"      // Fail if device can't prove identity

// Consumer app: optional attestation
attestation: "indirect"    // Accept if available; proceed anyway

// Most common: no verification
attestation: "none"        // Ignore attestation (privacy-first)
```

**Privacy concern:** Direct attestation can reveal authenticator model; browsers may redact to `"indirect"` for privacy.

---

## User Presence and User Verification

### User Presence (UP)
- Did the user physically interact with authenticator?
- Security key: User pressed a button
- Platform: System detected biometric/PIN entry
- Flag: `flags.userPresent` in `authenticatorData`
- Prevents: Unattended device from silently authenticating

### User Verification (UV)
- Did the system verify user identity via biometric/PIN?
- Security key: No built-in UV (button press != verification)
- Platform: Biometric/PIN verified identity
- Flag: `flags.userVerified` in `authenticatorData`
- Difference from UP: UV is stronger (proves who, not just possession)

### Server-Side Validation

```javascript
const flags = result.response.authenticatorData[flags_byte];
const userPresent = (flags & 0x01) !== 0;
const userVerified = (flags & 0x04) !== 0;

// Depending on security requirements:
if (riskLevel === "high" && !userVerified) {
  throw new Error("User verification required");
}
```

---

## Passwordless User Experience

### Registration Journey

**Traditional (Password):**
1. Email address
2. Password (remember it forever)
3. Confirm password
4. Done

**Passkey:**
1. Email address
2. "Set Up Passkey" → device creates key → biometric unlock → done
3. Faster, better security

### Login Journey

**Traditional:**
1. Email
2. Password (type it, be careful)
3. Click "Sign In"
4. Optional: MFA code (email/SMS/app)

**Passkey:**
1. Email (optional; some sites skip this)
2. Click "Sign In with Passkey"
3. Device shows key list → select → biometric unlock → instant
4. No MFA needed (passkey IS strong MFA)

### Recovery / Account Access

**Challenge:** User loses device with passkey.

**Solutions:**
- **Backup authenticator:** User registers second passkey on backup device during setup
- **Recovery codes:** During registration, user downloads/prints backup codes (one-time use)
- **Account recovery flow:** User verifies via email/SMS, creates new passkey
- **Synced passkeys:** User restores from platform account (iCloud, Google)

**Best practice:** Ask users to register 2+ passkeys during setup (main phone + backup).

---

## Enterprise Deployment

### Corporate Managed Devices

Enterprises can:
- Require passkeys on company-managed devices (MDM enrollment)
- Verify attestation (ensure key created on approved hardware)
- Disable passkey sync (keep credentials local)
- Enforce PIN/biometric unlock requirements
- Monitor: Track which devices are registered, detect unusual access patterns

### Conditional Access

Step-up authentication based on risk:
- User in known location with trusted device → passkey auth only
- User in new location → passkey + additional MFA
- User accessing sensitive resource → passkey + phone verification

---

## Vulnerabilities and Defense

### Phishing Resistance
**Strength:** Credential origin verified; user can't accidentally enter passkey into fake site.

- Device validates `origin` matches expected domain
- Attacker cannot intercept or impersonate credential

### Credential Cloning Risk
**Risk:** If device or cloud account compromised, attacker could clone passkey.

**Defense:**
- Secure enclave / TPM stores keys in hardware (harder to clone)
- Roaming authenticators (security keys) harder to clone physically
- Backup authenticators detection: `signCount` increases on each use; if older key used again, it's a clone

### Account Takeover via Device Loss

**Risk:** Device lost/stolen; attacker uses passkey.

**Defense:**
- Recovery codes: User-stored backup; only works once
- Account activity monitoring: Detect login from new device/location
- Backup authenticator: User has second key
- Account security: User can revoke compromised key via email verification

---

## Browser and Platform Support

| Platform | Status | Details |
|----------|--------|---------|
| **Chrome 67+** | ✅ Full support | All platforms |
| **Firefox 60+** | ✅ Full support | All platforms; requires user gesture |
| **Safari 13+** | ✅ Full support | macOS 10.15+, iOS 13.3+; best with platform authenticators |
| **Edge 18+** | ✅ Full support | Windows 10+, macOS |
| **Android Chrome** | ✅ Full support | 9+; hybrid Bluetooth transport |

**Polyfills:** Not possible for passkeys (requires OS integration); fallback must be traditional auth or security keys.

---

## Implementation Guide

### Server-Side Setup

1. Generate challenges using cryptographically secure PRNG
2. Store challenge temporarily (30-60 seconds); validate in assertion
3. Store public keys by user ID
4. Validate signatures using WebAuthn library
5. Implement credential management (list, rename, remove)

### Recommended Libraries

- **Node.js:** `@simplewebauthn/server`
- **Python:** `py_webauthn`
- **Go:** `github.com/duo-labs/webauthn`
- **Java:** `com.yubico:webauthn-server-core`

### Fallback Strategy

Assume site supports: passkeys (primary) → traditional login (secondary)

```javascript
try {
  const credential = await navigator.credentials.get({ publicKey: {...} });
  // Passkey succeeded
} catch (e) {
  // Bridge to traditional password login
  showPasswordForm();
}
```

---

## See Also

- [web-authentication-patterns.md](web-authentication-patterns.md) — broad authentication patterns
- [auth-multi-factor.md](auth-multi-factor.md) — MFA alternatives and fatigue attacks
- [security-authentication-attacks.md](security-authentication-attacks.md) — phishing and account takeover
- [security-identity.md](security-identity.md) — identity management and credential lifecycle