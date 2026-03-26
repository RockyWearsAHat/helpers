# Mobile Security — Storage Protection, Transport Security, Jailbreak Detection, Code Obfuscation & RASP

## Overview

Mobile applications (iOS, Android) face distinct threat models compared to desktop or web. Users expect apps to work offline, requiring local data storage—but on a shared device where other apps and attackers may attempt to access sensitive data. Additionally, mobile OSs and web browsers are attack targets; compromised devices expose app data directly. Mobile security requires protecting data at rest, in transit, during execution, and against tampering. Industry standards like MASVS and MASTG codify requirements; RASP (Runtime Application Self-Protection) and jailbreak detection detect attacks at runtime.

## Platform-Level Data Protection: Keychain and Keystore

### iOS Keychain

The Keychain is iOS's secure credential storage. Data is encrypted using device-specific keys derived from the device passcode and hardware UID (Unique ID).

**Access models**:
- **Accessible when unlocked** — Default; data available when device is unlocked (user has authenticated). App can access during normal use.
- **Accessible when unlocked, this device only** — Data tied to this device's hardware; won't transfer via iCloud backup/restore.
- **Accessible always** — Available even with locked device (e.g., for VoIP call handling). Rare; reserved for background functionality. Significantly weakens security.
- **Accessible after first unlock** — Data accessible after first user unlock since device reboot (covers most use cases).

**Appropriate use**:
- **Store**: API tokens, session cookies, biometric enrollment data
- **Don't store**: Large data (e.g., app databases) — inefficient; use file system with file-level encryption instead

**Keychain attributes**:
- `kSecAttrAccessible` — Access control (when is data available?)
- `kSecAttrAccessGroup` — Sharing across apps (same developer, entitlements). Risky; avoid unless necessary.
- `kSecAttrSynchronizable` — Sync via iCloud. Attacker may intercept; disable for highly sensitive keys.

**Attacks**:
- **Jailbroken device** — Attacker can read `/var/Keychains` directly.
- **Memory dump** — If app doesn't clear decrypted secrets from memory, attacker can read heap/stack.
- **Debugger access** — If debugger can attach, Keychain access can be inspected.

### Android Keystore

Android's equivalent to Keychain. Stores keys in the system's credential storage, backed by hardware (TEE/Trusted Execution Environment if available).

**Key capabilities**:
- **Hardware-backed** — If device has secure hardware (most modern Android), keys never leave the TEE. Cryptographic operations happen inside the secure enclave.
- **Key attestation** — Verify key was generated in the TEE (useful for enterprise security).
- **User authentication required** — Require biometric or PIN before key use (`KeyProperties.PURPOSE_SIGN`, biometric requirement).

**Appropriate use**:
- **Store**: API tokens, encryption keys, signing keys
- **Don't store**: Plaintext secrets in SharedPreferences; always use Keystore

**Access control**:
- `KeyProperties.USER_PRESENCE_REQUIRED` — User must be present (unlock or biometric)
- `KeyProperties.INVALID_USER_PROPERTIES_ONLY_DEVICE_CREDENTIALS` — Require device PIN/pattern/biometric

**Attacks**:
- **Unpatched devices** — Implementations have bugs (CVEs in TEE software)
- **No hardware backing** — Older devices; keys stored in software keystore (less secure)

## Certificate Pinning

Certificate pinning prevents Man-In-The-Middle (MITM) attacks by verifying the server's certificate against a pinned copy.

### How It Works

- App bundles the server's public key (or certificate) at build time.
- When connecting, app verifies: (1) certificate chain is valid, (2) pinned key is present in the chain.
- If attacker uses a different certificate (even if valid for the domain), the pin check fails; connection is rejected.

### Pinning Strategies

**Certificate pinning** — Pin the server's leaf certificate. If server rotates the certificate, app must be updated. Requires monitoring certificate expiration and coordinating updates.

**Public key pinning** — Pin the server's public key (decoded from certificate). More flexible: certificate can be re-issued with the same key. Still requires updates if the key is rotated.

**Pin hashing** — Store hash of certificate/key, not the full data. Reduces bundled data size (negligible benefit).

**Backup pins** — Include multiple pins (current + upcoming certificate or key). Allows rotation without forcing immediate app update.

**Static pinning** — Hardcode pins in app code or configuration. Simple but inflexible; requires app update to change pins.

**Dynamic pinning** — Fetch pins from server (via a secure channel) and update locally. More flexible but introduces chicken-and-egg problem: how to validate the pin-fetching connection itself?

### Implementation Challenges

**Pin expiration** — If app's pin is for a certificate that expires and is not updated, app becomes unable to connect. Test rotation in staging before production.

**Enterprise proxies** — Corporate firewalls often intercept HTTPS and re-sign certificates. Pinning prevents traffic inspection; IT policies may require disabling pinning on corporate networks.

**Third-party APIs** — If app uses multiple API endpoints from different certificate authorities, multiple pins are needed. Managing many pins is operationally complex.

**Debugging during development** — Pinning prevents monkey-patching requests in dev tools (Burp Suite, Charles proxy). Common workaround: disable pinning in debug builds (dangerous if debug builds leak to production).

## Secure Transport Configuration

Platforms provide declarative transport security policies.

### iOS App Transport Security (ATS)

**ATS (iOS 9+)** — Default-deny policy: apps can't use unencrypted HTTP. All connections must use TLS 1.2+, strong ciphers.

Configuration in `Info.plist`:
```xml
<key>NSAppTransportSecurity</key>
<dict>
  <key>NSAllowsArbitraryLoads</key>
  <false/>
  <key>NSExceptionDomains</key>
  <dict>
    <key>example.com</key>
    <dict>
      <key>NSIncludesSubdomains</key>
      <true/>
      <key>NSMinimumTLSVersion</key>
      <string>TLSv1.2</string>
    </dict>
  </dict>
</dict>
```

Best practice: enforce ATS globally; exceptions only for third-party APIs that don't support TLS 1.2 (rare nowadays).

### Android Network Security Config

Similar to ATS. Declarative policy in `network_security_config.xml`:
```xml
<?xml version="1.0" encoding="utf-8"?>
<network-security-config>
  <domain-config cleartextTrafficPermitted="false">
    <domain includeSubdomains="true">example.com</domain>
  </domain-config>
</network-security-config>
```

Default: only HTTPS. Cleartexttrafficpermitted allows HTTP for specific domains (rarely needed).

## Jailbreak and Root Detection

Jailbreak (iOS) and rooting (Android) remove OS security boundaries. Attackers gain full device access, bypassing app protections. Apps can attempt to detect (and refuse to run) on compromised devices.

### Jailbreak Detection Techniques (iOS)

**File-based checks** — Jailbreak tools install files in specific locations. Check for common Cydia/jailbreak indicators:
```swift
let jailbreakPaths = [
  "/Applications/Cydia.app",
  "/usr/sbin/sshd",
  "/Library/MobileSubstrate"
]
for path in jailbreakPaths {
  if FileManager.default.fileExists(atPath: path) {
    // Jailbroken
  }
}
```

**Sandbox checks** — Jailbroken devices may have reduced sandboxing. Attempt a harmless sandbox-violating operation (e.g., write outside app bundle); if successful, device is jailbroken.

**Code injection detection** — Dylib injection frameworks (Substrate, Theos) modify linked libraries. Check `_dyld_image_count()` or inspect loaded dylibs for suspicious names.

**Process introspection** — Debugger attached? Jailbroken devices allow multiple processes to debug one app. Check with `getppid()` or ptrace anti-debugging flag.

### Root Detection Techniques (Android)

**File presence checks** — Common rooting app footprints:
```java
String[] dangerousFiles = {
  "/system/app/Superuser.apk",
  "/system/xbin/su",
  "/system/bin/su"
};
for (String path : dangerousFiles) {
  if (new File(path).exists()) {
    // Rooted
  }
}
```

**Package detection** — Check `PackageManager` for known rooting apps (Magisk, SuperSU):
```java
PackageManager pm = getPackageManager();
try {
  pm.getPackageInfo("com.topjohnwu.magisk", 0);
  // Rooted
} catch (PackageManager.NameNotFoundException e) {
  // Not found
}
```

**su binary execution** — Attempt to execute `su`:
```java
Runtime.getRuntime().exec("su");
```
On unrooted devices, this throws an exception. On rooted devices, it succeeds.

**SELinux context** — Rooted devices often have modified SELinux policies. Check with `getenforce()`.

### Limitations and Arms Race

- **Obfuscation of checks** — Jailbreak/root detection code itself should be obfuscated; attackers reverse-engineer and patch checks.
- **Runtime patching** — Frida and similar tools intercept function calls. Detection code can be patched out at runtime.
- **Legitimate use cases** — Developers use rooted phones for testing. Too aggressive detection blocks legitimate workflows.
- **Arms race** — Jailbreak tools evolve to evade detection; detection catches up. Never foolproof.

**Reasonable approach**: Light detection as a speed bump, not a security boundary. Sensitive operations (payments, authentication) require additional controls (server-side verification, risk scoring).

## Code Obfuscation

Obfuscation makes reverse-engineered code harder to understand (though not impossible).

### Obfuscation Techniques

**Variable/function renaming** — Replace meaningful names with meaningless ones (`calculateTotal` → `a`). Decompilers undo this easily; low value.

**Control flow flattening** — Replace nested if/loops with flat dispatch table. Makes static analysis harder but adds overhead.

**Constant encryption** — Encrypt hardcoded strings; decrypt at runtime. API keys, URLs hidden from cursory inspection.

**Dead code injection** — Include unused functions/branches. Inflates binary size, confuses analysis tools.

**Proxy methods** — Wrap real logic in stubs and proxies. Increases indirection; makes control flow analysis harder.

### Android Obfuscation: ProGuard, R8

**ProGuard** — Traditional Android obfuscator. Now legacy; largely obsolete.

**R8** — Successor to ProGuard (default in modern Android). Achieves obfuscation + minification (removes unused code) + optimization:
```gradle
android {
  buildTypes {
    release {
      minifyEnabled true
      shrinkResources true
      proguardFiles getDefaultProguardFile('proguard-android-optimize.txt'), 'proguard-rules.pro'
    }
  }
}
```

Keep rules: `keep` directives preserve certain classes/methods (e.g., public API, reflection targets):
```
-keep public class com.example.MyApp
-keepclassmembers class com.example.** { public *; }
```

### iOS Code Obfuscation

iOS tooling for obfuscation is less mature than Android. Options:
- **Bitcode obfuscation** — Apple's intermediate representation; some obfuscation but not full control.
- **Third-party tools** — e.g., Cynder, Dotfuscator (cross-platform).
- **Compile-level obfuscation** — Clang supports some LLVM passes (e.g., control flow flattening).

Generally less critical than Android because iOS apps are distributed as IPA (comparable to Android APK), but reverse-engineering barriers are lower.

## Runtime Application Self-Protection (RASP)

RASP detects and blocks attacks at runtime, from within the app.

### Attack Detection

**Hooking/instrumentation detection** — Detect Frida, Cydia Substrate, xposed framework by checking for known hooking signatures in memory or by attempting library-specific operations.

**Tamper detection** — Verify APK/binary signature at runtime. If modified, app shuts down.

**Memory integrity checks** — Periodically verify critical code/data hasn't been modified. Expensive; selective application recommended.

**Abnormal behavior patterns** — Detect SQL injection attempts (weird queries), excessive login failures, API flood. Mitigated by rate-limiting, input validation.

### Limitations

- **Attacker with device control** — On a jailbroken/rooted device, attacker can patch RASP code itself.
- **Performance overhead** — Runtime checks add latency and battery drain.
- **High false positives** — Overly aggressive detection blocks legitimate use (e.g., crash reporter that looks like memory corruption).

**Practical approach**: RASP as one layer; not sufficient alone. Must combine with server-side verification, risk scoring, and network hardening.

## Mobile Standards and Frameworks

### OWASP MASVS (Mobile Application Security Verification Standard)

Comprehensive checklist for mobile security. Organized by maturity levels:
- **Level 1** — Basic security (credentials stored securely, transport encrypted, etc.)
- **Level 2** — Advanced security (jailbreak detection, obfuscation, server-side verification)

### OWASP MASTG (Mobile Application Security Testing Guide)

Practical guide for testing and validating mobile security. Provides test cases, tools, and methodology aligned with MASVS.

## Common Anti-Patterns

**Storing secrets in code or properties** — API keys hardcoded or in config files. Extract to Keychain/Keystore.

**Logging sensitive data** — Auth tokens, PII in app logs. Logs may be readable by other apps or device owners. Always sanitize logs.

**No certificate pinning for sensitive endpoints** — Relies solely on system CA verification (vulnerable to compromised CAs, MITM on open WiFi).

**Disabled obfuscation in release builds** — Reduces effort to reverse-engineer critical logic.

**Excessive jailbreak detection** — False positives block legitimate development/testing workflows; users uninstall app.

## See Also

- security-best-practices (foundational principles)
- security-web-application (transport and encoding parallel concepts)
- web-authentication-patterns (biometric and credential patterns)
- security-devsecops (testing and release integration)