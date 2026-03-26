# Identity Management — Authentication Protocols, Providers & Session Control

## Overview

Identity management (IdM) answers: "Who are you?" (authentication) and "What can you do?" (authorization). Modern systems delegate authentication to identity providers (IdPs) that speak standard protocols (SAML, OIDC, OAuth 2.0), enabling single sign-on (SSO) and reducing password burden on users and custom code on applications.

## Authentication Protocols

### SAML 2.0 (Security Assertion Markup Language)

XML-based protocol for exchanging authentication/authorization data between an identity provider (IdP) and a service provider (SP).

**Architecture:**
1. User requests protected resource from service provider (SP)
2. SP redirects user to identity provider (IdP)
3. IdP authenticates user (password, MFA, LDAP)
4. IdP generates digitally signed SAML assertion (XML) containing user identity & attributes
5. IdP posts assertion back to SP (via user's browser)
6. SP validates signature, extracts user identity, creates session

**Assertion example:**
```xml
<saml:Assertion>
  <saml:Subject>
    <saml:NameID>alice@example.com</saml:NameID>
  </saml:Subject>
  <saml:AttributeStatement>
    <saml:Attribute Name="department" Value="engineering"/>
    <saml:Attribute Name="role" Value="admin"/>
  </saml:AttributeStatement>
</saml:Assertion>
```

**Strengths:**
- Mature (2005); widely supported by enterprise systems
- Includes attribute assertions (can carry group membership, roles)
- Signed assertions are tamper-proof

**Weaknesses:**
- XML parsing complexity (historical vulnerability surface)
- Verbose (larger payloads than JWT)
- Less suited for mobile apps (redirect-based flow)
- Limited to SSO; no user info endpoint (applications can't query IdP for user details)

**Use case:** Enterprise on-premises authentication (integrates with Active Directory); legacy application integration.

### OAuth 2.0 (Delegation of Authorization)

Framework for delegating authorization. **Not for authentication** — OAuth obtains access to resources on behalf of user, not proof of user identity.

**Flow (Authorization Code):**
1. User wants to use a third-party app (e.g., Spotify) to access their Google Drive
2. App redirects user to Google (the "authorization server")
3. User logs in to Google, consents to "Spotify accessing my Drive files"
4. Google redirects back to Spotify app with authorization code
5. Spotify exchanges code (+ secret) for access token
6. Spotify uses access token to call Google Drive API on behalf of user

**Token types:**
- **Access token:** Grants permission to API (short-lived, e.g., 1 hour)
- **Refresh token:** Obtained new access tokens without re-authenticating (longer-lived, e.g., 30 days)
- **Scope:** Permissions granted (e.g., `drive.readonly` = read-only access to Drive)

**Strengths:**
- Widely adopted (Facebook, Google, GitHub logins)
- Supports mobile & desktop apps
- User never shares password with third-party app
- Granular scopes (app asks only what it needs)

**Weaknesses:**
- Requires two secrets: client_id (identification), client_secret (authentication). If exposed, attacker can impersonate app.
- No built-in user identity assertion (cannot be used alone for authentication)
- Token uses opaque format; server validation requires hitting authorization server

### OpenID Connect (OIDC)

Adds an identity layer on top of OAuth 2.0. Solves "who are you?" in addition to "what can you access?"

**Difference from OAuth:**
- OAuth: "Grant Spotify access to my Google Drive"
- OIDC: "Prove I'm alice@example.com to Spotify"

**New artifact:**
- **ID token:** JWT signed by IdP, containing user identity claims (sub, email, name, preferred_username)

**Flow (simplified Authorization Code + OIDC):**
1. App redirects user to OIDC provider (e.g., Okta, Auth0) with `scope=openid profile`
2. User authenticates, consents
3. OIDC provider returns authorization code (same as OAuth)
4. App exchanges code for **access token + ID token**
5. App validates ID token signature (trusts claims inside)

**ID token example:**
```json
{
  "iss": "https://okta.example.com",
  "sub": "alice-123",
  "email": "alice@example.com",
  "email_verified": true,
  "name": "Alice Smith",
  "aud": "app-client-id",
  "exp": 1679865600
}
```

**Strengths:**
- True authentication (proves identity)
- OAuth + OIDC covers both use cases (authorization + authentication)
- Adopted by modern SaaS (Auth0, Okta, Keycloak)
- Stateless (app validates JWT signature; no server lookup needed)

**Trade-off:** Requires understanding two protocols.

## Identity Providers (IdPs)

### Okta

**Market position:** Enterprise-focused IdP/SSO provider. $2B valuation; common in Fortune 500.

**Features:**
- MFA (TOTP, SMS, biometric)
- SSO (SAML, OIDC, OAuth)
- User directory (LDAP, AD sync)
- Adaptive authentication (risk-based step-up)
- Lifecycle management (SCIM provisioning)

**Pricing:** Per monthly active user ($1–3/user).

**Trade-off:** Proprietary vendor; monthly cost scales with user base.

### Auth0

**Market position:** Developer-friendly IdP; acquired by Okta (2021).

**Features:**
- Rapid setup (30-minute SSO integration)
- Large rule ecosystem (middleware for custom logic)
- Passwordless authentication (email link, biometric)
- B2B/multi-tenant support

**Pricing:** Free tier (up to 7,500 users); pay-as-you-grow beyond.

**Trade-off:** Less enterprise-focused than Okta; smaller rule library than Okta Workflows.

### Keycloak (Open Source)

**Market position:** Free, self-hosted IdP; maintained by Red Hat.

**Features:**
- SAML, OIDC, OAuth 2.0
- User federation (LDAP, AD, Kerberos)
- Custom themes, extensions
- Runs on-premises or cloud

**Advantages:** No licensing cost; full control over data.

**Trade-offs:** Requires self-hosting/ops effort; smaller community than commercial IdPs.

### LDAP / Active Directory

**LDAP:** Directory protocol for user/group storage and authentication. Common in enterprises for on-premises authentication.

**Active Directory:** Microsoft's LDAP-compatible directory; manages Windows domain authentication.

**Capabilities:**
- Stores user credentials (passwords)
- Groups & organizational units
- Attribute storage (email, phone, department)

**Role in modern SSO:** Often used as backend user store for SAML/OIDC IdPs.

**Example:** Active Directory → Okta (IdP) → SaaS app (SSO).

## SCIM (System for Cross-domain Identity Management)

API for provisioning (creating/updating/deleting) user accounts across applications automatically.

**Workflow:**
1. HR system creates new employee record in Okta
2. Okta sends SCIM request to GitHub: `POST /scim/v2/Users { name: "Alice", email: "alice@example.com" }`
3. GitHub creates user account + adds to team
4. Same user now appears in GitHub without manual setup

**Benefits:**
- Eliminates manual account creation
- Ensures consistency (same attributes across tools)
- Deprovisioning (removing access) automated when employee leaves

**Standardization:** SCIM 2.0 (RFC 7643/7644).

## Passwordless Authentication

Eliminates passwords; users authenticate via:

### TOTP (Time-based One-Time Password)

User generates 6-digit code from app (Google Authenticator, Authy) that changes every 30 seconds.

**Implementation:** Shared secret between IdP and user device; both generate code based on current time + HMAC.

**Strengths:** Works without network (offline), hard to phish.

**Trade-off:** Backup codes needed if device lost.

### Email / SMS Links

IdP sends one-time link via email/SMS; user clicks to authenticate.

**Example:** User enters phone, receives text "Click here to login: example.com/auth/abc123"

**Strengths:** Simple (device used for 2FA anyway); no separate app needed.

**Trade-offs:** Links expire (usually 10 min); vulnerable if email/SMS compromised.

### WebAuthn (FIDO2)

Browser standard for hardware-based authentication. User confirms identity with fingerprint, PIN, or security key.

**Flow:**
1. App calls `navigator.credentials.get()`
2. Browser/OS prompts user biometric/PIN
3. Authenticator (built into device or USB key) signs challenge
4. Response sent to server; server verifies signature

**Strengths:**
- Phishing-resistant (cryptographic binding to domain)
- User-friendly (biometric)
- No secrets stored server-side (only public key)

**Trade-off:** Requires modern browser + compatible device.

### Passwordless Best Practices

- **Fallback:** Passwordless is primary; password + recovery codes as backup
- **Recovery:** Backup codes or recovery email (when primary method unavailable)
- **Enrollment:** Require MFA for initial passwordless setup

## Session Management

Session: Stateful assertion of user identity lasting across multiple requests.

### Session-Based (Server-Side Storage)

User logs in → server creates session object (user ID, login time) → server stores in Redis/DB → returns session cookie.

**Subsequent requests:** Browser sends cookie; server looks up session.

**Strengths:**
- Server can revoke immediately (logout updates DB)
- Can track arbitrary state (permissions changes, account status)

**Trade-off:** Requires server-side storage; doesn't scale across distributed systems (requests must hit same server or use shared session store).

### Token-Based (JWT)

User logs in → server signs JWT (sub: alice, exp: 1hour) → client stores token.

**Subsequent requests:** Client sends JWT; server validates signature (no DB lookup).

**Strengths:**
- Stateless (scales to distributed systems)
- Works across domains/APIs
- No database query per request

**Limitations:**
- Cannot revoke token mid-lifetime (once signed, valid until exp time)
- If secret leaked, all tokens at risk

**Common pattern:** Access token (short-lived: 15 min) + refresh token (long-lived: 7 days). When access token expires, client uses refresh token to obtain new one.

## Adaptive Authentication

Adjusts authentication requirements based on risk.

**Signals assessed:**
- User location (geographic anomaly: logged in Tokyo yesterday, now New York)
- Device (new device, unrecognized)
- Time of day (login at 3am vs. 3pm)
- Network (VPN, proxy detect)

**Actions:**
- Low risk: Allow (normal)
- Medium risk: Step-up (require MFA)
- High risk: Block or require additional identity verification

**Examples:**
- "Alice, you usually log in from San Francisco. Your current IP is in Moscow. Please confirm MFA."
- "Login from new device. Please use authenticator app to verify."

## Identity Governance

Ongoing management: who has access, are permissions still valid, are credentials rotated?

**Responsibilities:**
- **Access certification:** Manager reviews team's app access periodically; approves or revokes
- **Entitlement review:** Ensuring provisioned access matches job function (engineer shouldn't have access to HR records)
- **Credential rotation:** API keys rotated quarterly, service account passwords reset
- **Offboarding:** When employee leaves, all accounts disabled within hours

**Benefit:** Reduces insider threat surface; ensures access control remains least-privilege.

## SAML vs. OIDC vs. OAuth (Cheat Sheet)

| Factor | SAML 2.0 | OIDC | OAuth 2.0 |
|---------|----------|------|----------|
| **Purpose** | Authentication + attributes | Authentication (OIDC) + Authorization (OAuth) | Authorization only |
| **Format** | XML assertions | JWT ID token | Opaque access token |
| **Enterprise** | Best fit (legacy enterprises). | Modern choice; SaaS. | For delegating access (Spotify to Drive). |
| **Setup complexity** | Higher (metadata exchange). | Lower; similar to OAuth. | Medium. |
| **Revocation** | SAML LogoutRequest. | Logout endpoint. | Refresh token revocation; long-lived access tokens not revocable. |
| **Mobile** | Poor (redirect-based). | Good. | Good. |
| **Stateless** | No (assertions can be session). | Yes (JWT is stateless). | Can be stateless or stateful. |

## See Also

- OAuth 2.0 & OpenID Connect (detailed note)
- Web Authentication Patterns
- Zero Trust Architecture
- Secrets Management