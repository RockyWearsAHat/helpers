# Security — Application-Level Protection: CAPTCHA, Bot Detection, and Account Takeover Prevention

## Overview

Application-level protection deters automated attacks and account takeover through human verification (CAPTCHA), fingerprinting, behavioral analysis, and challenge-response mechanisms. These layers operate above rate limiting and network defenses—they prevent an attacker from succeeding even when they bypass throttling. The tradeoff is user friction: legitimate users must prove humanness or pass behavioral checks to continue.

---

## Challenge-Response Mechanisms

### CAPTCHA (Completely Automated Public Turing Test to Tell Computers and Humans Apart)

**Purpose:** Verify that a request originates from a human, not a bot.

**Types:**

| Type | Mechanism | False Negative (Bot Passes) | False Positive (Humans Fail) |
|------|-----------|----|---|
| Text distortion ("Type these 5 letters") | Recognition of OCR-resistant characters | OCR + ML (80%) | Low; humans 99.8% success |
| Image selection ("Click all traffic lights") | Object recognition | ML models trained on same dataset | Medium; ambiguous edge cases |
| Puzzles ("Rotate to correct angle") | Spatial reasoning | 50–70% solved by ML | Low; intuitive for humans |
| Device behavior ("Click the checkbox") | Challenge + device fingerprinting | Detector-proof bots emulate behavior | Very low; single click |
| Invisible (reCAPTCHA v3) | Behavioral scoring without UX friction | Learning-based evasion; 0% false negatives possible | 0; user unaware |

**Vendor Comparison:**

| Vendor | Model | Cost | Accessibility | Evasion |
|--------|-------|------|---|---|
| Google reCAPTCHA v2 | Text/Image distortion + puzzle | Free (usage-tracked) | Screen reader unsupported | Known bypasses via crowdsourcing |
| Google reCAPTCHA v3 | Behavioral scoring (invisible) | ~$1/1000 requests | Native (zero friction) | Harder (requires behavioral learning) |
| hCaptcha | Privacy-focused, image labeling | ~$5/10k requests | Better (built-in accessibility) | Similar to reCAPTCHA v2 |
| Cloudflare Turnstile | Lightweight, hardened | Free (for Cloudflare users) | Built-in accessibility | Optimized for bot resistance |

**Integration Pattern:**
```javascript
// Client-side: Render CAPTCHA widget
<script src="https://challenges.cloudflare.com/turnstile/v0/api.js"></script>
<div class="cf-turnstile" data-sitekey="YOUR-SITEKEY"></div>

// Server-side: Verify token
fetch('https://challenges.cloudflare.com/turnstile/v0/siteverify', {
  method: 'POST',
  body: new URLSearchParams({
    secret: TURNSTILE_SECRET,
    response: TURNSTILE_TOKEN
  })
})
.then(r => r.json())
.then(d => {
  if (d.success) {
    // Process login/etc
  }
});
```

**When to Challenge:**
- After N failed login attempts (3–5)
- First login from new geography/device
- Suspicious behavioral signals (impossible travel, timing anomalies)
- Unusual API usage patterns (requests from residential proxies, unusual geographies)

### Device Fingerprinting

Builds a unique profile of the requesting device without persistent cookies (browser fingerprinting).

**Components Collected:**
- Browser: User-Agent, rendering engine, JavaScript APIs
- Hardware: GPU model, CPU cores (via worker performance), screen resolution
- Network: WebGL renderer, DNS resolution time
- Software: Font list, installed languages, timezone
- Behavior: Mouse speed, keystroke timing, touch pressure

**Entropy per component:**
- User-Agent: ~32 bits (millions of combinations)
- DNS+WebGL: ~16 bits each
- Font list: ~20 bits (thousands of combinations)
- Mouse dynamics: ~8–15 bits (continuous variance)
- **Total:** 90–130 bits of entropy (far exceeds cookie security)

**Limitations:**
- Privacy-mode browsing, VPNs, and Tor change fingerprints
- Legitimate users on shared machines (library, family) yield identical fingerprints
- Browser updates (new User-Agent, new fonts) change fingerprints
- Tracking regulations (GDPR, CCPA) limit persistent use

**Use Cases:**
- Detect account takeover: New fingerprint = suspected compromised account
- Distinguish users behind same NAT: Two device IDs from same IP = multiple people
- Catch hijacked sessions: Session token + fingerprint mismatch = flag

---

## Behavioral Analysis

### Risk Scoring Models

Real-time models combine signals into a risk score (0–100) to decide: allow, challenge, or block.

**Signals:**

| Signal | Details | Weight |
|--------|---------|--------|
| Geolocation jump | Logged in 1,000 miles away in 30 minutes | High |
| New device | Fingerprint never seen before for account | High |
| Impossible travel | Time/distance inconsistent with human movement | High |
| Time anomaly | Login at 3am when user always logs in at 9am | Medium |
| Proxy/VPN | Request from known proxy provider | Medium |
| New IP subnet | /24 block never associated with account | Medium |
| Suspicious User-Agent | Headless browser, ancient browser, suspicious string | Medium |
| Failed login burst | 10 failed attempts in 2 minutes | High |
| Cross-device correlation | Multiple accounts from same fingerprint/IP | High |
| Residential proxy | Request from residential ISP associated with abuse | Medium |

**Scoring Example:**
```
Base score: 0
+ Geolocation jump (3000 miles): +40
+ New fingerprint: +30
+ Residential proxy: +15
+ Time 2am (anomalous): +10
= Total: 95 (action: BLOCK or require MFA)

vs.

Base score: 0
+ New IP (same city): +5
+ Expected browser: +0
= Total: 5 (action: ALLOW)
```

**Model Training:**
- Supervised: Collect labeled data (user "this was me" vs. "not me" on logins)
- Unsupervised: Cluster user behavior (learn each user's patterns independently)
- Hybrid: Supervised baseline + per-user deviation detection

**Feedback Loops:**
- User reports unauthorized login: Model downweights that signal combination for future
- User confirms login from VPN (false positive): Model reduces VPN weighting for that account

### Velocity Checks

Detects rapid exploitation by tracking action rates:

```
User clicks "forgot password" 10 times in 60 seconds → Likely account enumeration attack
User attempts login from 50 IPs in 2 minutes → Credential stuffing
User downloads data for 10 accounts in 30 seconds → Bulk data exfiltration
```

**Implementation:**
```sql
SELECT COUNT(*) FROM login_attempts
WHERE account_id = ? AND timestamp > now() - INTERVAL '1 minute'

IF count > 5: BLOCK
IF count > 3: RATE_LIMIT + CAPTCHA
```

---

## Bot Detection Techniques

### Heuristic Detection

**Headless Browser Markers:**
- `navigator.webdriver === true` (Selenium, Puppeteer)
- Missing `navigator.plugins` (headless Chrome)
- `window.outerHeight === 0` (some headless setups)
- Unusual timing (all responses in <10ms, unrealistic for humans)

**Detection Code:**
```javascript
function isHeadlessBrowser() {
  // Easily defeated—attackers patch these
  if (navigator.webdriver) return true;
  if (/headslesschrome|phantomjs|headlessbrowser/.test(navigator.userAgent)) return true;
  return false;
}
```

**Limitation:** Easily defeated by patching chromium's startup flags.

### Challenge-Based Detection

**JavaScript Challenge:**
Require client-side JavaScript execution + computation to proceed. Bots must parse HTML, understand JavaScript, execute it, and submit results.

```html
<script>
// Encode challenge response
const challenge = Math.random();
document.getElementById('auth_token').value = btoa(challenge);
// Server verifies base64-decoded value makes sense
</script>
<form>
  <input type="hidden" id="auth_token" name="token" />
</form>
```

**Effectiveness:**
- Stops basic HTTP clients (curl, wget)
- Defeats many commodity bots (OpenBullet, Sentry)
- Detected by advanced bots (Puppeteer, Selenium) which execute JS
- Browser-like bots render JavaScript and submit

### Behavioral Signals

**Low-friction detection patterns:**

| Behavior | Normal Human | Bot |
|----------|---|---|
| Form interaction time | 5–30 seconds (read, think, type) | <100ms (automated fill) |
| Mouse movement | Curved, variable speed, pauses | Linear, constant speed, or absent |
| Keystroke timing | 50–200ms between chars (variable) | Uniform, unrealistic gaps |
| Tab+Enter sequence | Human uses mouse or Alt+Tab | Bots use Tab consistently |
| Hover behavior | Hovers over fields before/while typing | No hover (direct input) |

**Implementation:**
```javascript
const behaviors = {
  focusChangeTime: Date.now(),
  keyPressCount: 0,
  mouseMoveCount: 0,
  timeFromFocusToFirstKey: Infinity
};

input.addEventListener('focus', () => {
  behaviors.focusChangeTime = Date.now();
});

input.addEventListener('keypress', () => {
  behaviors.timeFromFocusToFirstKey = Math.min(
    behaviors.timeFromFocusToFirstKey,
    Date.now() - behaviors.focusChangeTime
  );
});

// timeFromFocusToFirstKey < 50ms = suspicious
```

---

## Account Takeover Prevention

### Multi-Factor Authentication (MFA)

**Coverage:** Stops ~99.9% of automated attacks by requiring a second factor attacker doesn't possess.

**Types:**
| Type | Examples | Compromise Risk |
|------|----------|---|
| TOTP (Time-based OTP) | Google Authenticator, Authy | Low (attacker has phone, would know SMS) |
| SMS/Email OTP | 6-digit codes | Medium (SMiShing, email compromise) |
| Push notification | "Approve this login?" on trusted device | Low (requires app on trusted device) |
| WebAuthn/Passkeys | Biometric, hardware key | Very low (requires physical hardware or biometric) |

**Deployment:**
- Mandatory MFA for privileged accounts (admin, finance)
- Optional MFA for all users (gradually increase adoption)
- Force MFA after risk events (new device, impossible travel)

### Suspicious Login Policies

```
IF login from new geography OR new device:
  REQUIRE MFA

IF multiple failed attempts:
  RATE LIMIT + require CAPTCHA + MFA if enabled

IF suspicious behavior score > threshold:
  REQUIRE additional verification:
    - MFA code
    - Security questions
    - Email verification link
```

### Credential Breach Monitoring

**Process:**
1. Hash user's password using bcrypt/scrypt
2. Query HaveIBeenPwned API (or self-host data) to check if hash matches leaked passwords
3. Force password reset if breached (user doesn't know their password was leaked elsewhere)

**Privacy:** Query via k-anonymity—send first 5 chars of hash to API; server returns all matching hashes; compare locally.

```bash
# User password: mypassword123
# bcrypt hash: $2b$12$... (first 5 chars: $2b$1)

# Query: GET https://api.pwnedpasswords.com/range/2B$12...
# Response: All password hashes starting with $2B$12*
# Client: Check if user's full hash in response → if yes, password was breached
```

---

## Rate Limiting vs. Resource Limits

### Adaptive Rate Limiting

Scale rate limits based on account risk or behavioral trust:

```
New account, unproven email: 10 API calls/min
Verified account, 6-month history: 1,000 API calls/min
Account with MFA: 5,000 API calls/min
```

### Resource Quotas

Separate rate limits per resource:

```
Login endpoint: 5 requests/min per IP
Password reset: 3 requests/hour per email
File upload: 100MB/hour per user
Data export: 1 export per day
```

---

## Deployment Considerations

### CAPTCHA Friction Metrics

**Monitor:**
- Solve rate: % of CAPTCHAs humans successfully complete (expect 95%+)
- Abandonment rate: % of users who leave after CAPTCHA (expect <5%)
- Solve latency: Average time to solve (target <10 seconds)

**Tradeoff:**
- Stricter CAPTCHAs → fewer bots, higher human abandonment
- Lenient CAPTCHA → lower friction, more bots pass

### Regional Considerations

- Accessibility requirements: `alt` text, keyboard navigation, audio alternatives
- Language: Present challenges in user's language
- Infrastructure: CAPTCHA API availability in region (China blocks Google reCAPTCHA)

### False Positive Costs

Each block or MFA challenge risks legitimate user abandonment:
- E-commerce: 1% additional friction = 5–10% conversion loss
- Recovery codes: Users who lose MFA device and don't have recovery codes cannot log in

**Mitigation:**
- Transparent fallback: If CAPTCHA fails 10x, offer support contact or recovery flow
- Recovery codes: Issue at MFA enrollment; store securely (not in password manager)
- Account recovery: Verify identity via email, phone, or security questions

---

## See Also

- [Authentication Attacks](security-authentication-attacks.md) — Credential stuffing, brute force, MFA circumvention
- [Rate Limiting for Defense](security-rate-limiting-defense.md) — Infrastructure and WAF rate limiting
- [Secrets Management](security-secrets-management.md) — Protecting credentials in transit and at rest
- [Security API](security-api.md) — API authentication and authorization patterns