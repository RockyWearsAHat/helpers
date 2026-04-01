# SAML 2.0: Assertions, Protocols, Bindings & Enterprise SSO

## Overview

SAML (Security Assertion Markup Language) 2.0 is an XML-based standard for federated identity and single sign-on (SSO). It enables users to authenticate once at an **Identity Provider (IdP)** and gain access to multiple **Service Providers (SPs)** without re-entering credentials. SAML is the de facto enterprise SSO protocol, built into directory services (Okta, Azure AD, Ping) and legacy systems; OIDC/OAuth 2.0 dominates consumer and modern cloud applications.

---

## Core Concepts

### Identity Provider (IdP)
- The trusted authority that verifies user identity (e.g., Okta, Azure AD, corporate directory)
- Issues cryptographically signed **assertions** containing user identity and attributes
- Maintains user credentials, MFA, session state
- Does not typically store user data for the SP

### Service Provider (SP)
- The application or service the user wants to access (e.g., Salesforce, Jira, Box)
- Does not store passwords; trusts assertions from the IdP
- Delegates authentication to the IdP but controls authorization (e.g., role mapping)

### Assertions
XML documents signed by the IdP containing claims about the user. Three types:

| Type | Purpose |
|------|---------|
| **Authentication Assertion** | Confirms user identity at a specific time; contains `<AuthnStatement>` |
| **Attribute Assertion** | Contains user attributes (name, email, groups, roles); used for authorization |
| **Authorization Assertion** | Grants/denies specific resource access; rarely used |

Example authentication assertion structure:
```xml
<Assertion>
  <AuthnStatement AuthnInstant="2024-01-15T10:00:00Z">
    <AuthnContext>
      <AuthnContextClassRef>urn:oasis:names:tc:SAML:2.0:ac:classes:Password</AuthnContextClassRef>
    </AuthnContext>
  </AuthnStatement>
  <AttributeStatement>
    <Attribute Name="email" NameFormat="urn:oasis:names:tc:SAML:2.0:attrname-format:uri">
      <AttributeValue>user@example.com</AttributeValue>
    </Attribute>
  </AttributeStatement>
</Assertion>
```

---

## SAML Protocols

### AuthnRequest (SP → IdP)
The SP initiates login by sending an **AuthnRequest** to the IdP. The request contains:

- `ID`: Unique request identifier
- `Destination`: IdP SSO endpoint URL
- `AssertionConsumerServiceURL`: Where IdP should send the response (the SP's ACS)
- `NameIDPolicy`: What type of identifier the SP expects for the user (e.g., `EmailAddress`, `Unspecified`)

```xml
<AuthnRequest 
  ID="_request-id" 
  Destination="https://idp.example.com/sso"
  AssertionConsumerServiceURL="https://sp.example.com/saml/acs">
  <NameIDPolicy Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress" />
</AuthnRequest>
```

### Response (IdP → SP)
The IdP validates credentials and issues a **Response** containing one or more **Assertions** and metadata:

- `InResponseTo`: Echoes the AuthnRequest ID (prevents response tampering)
- `Assertions`: Contains authentication and attribute data
- `Signature`: XML signature over the entire response or individual assertions

```xml
<Response ID="_response-id" InResponseTo="_request-id" Destination="https://sp.example.com/saml/acs">
  <Assertion ID="_assertion-id" IssueInstant="2024-01-15T10:00:00Z" Issuer="https://idp.example.com">
    <!-- AuthnStatement and AttributeStatement here -->
  </Assertion>
  <Signature><!-- XML digital signature --></Signature>
</Response>
```

The SP validates the signature, checks `InResponseTo` matches the original request, and verifies the assertion hasn't expired.

---

## Bindings: How SAML Messages Travel

SAML specifies how assertions and requests are transmitted. The three main bindings:

### HTTP Redirect Binding (SP → IdP)
- AuthnRequest URL-encoded and added as query parameter: `?SAMLRequest=...`
- Compressed and encoded to keep URL under 8KB limit
- Visible in browser URL bar (not sensitive data, just XML structure)
- Fastest, most common for AuthnRequest

### HTTP POST Binding (IdP → SP)
- SAML Response embedded in HTML form, auto-submitted via POST
- Response can be much larger (no URL length limit)
- Assertion stays hidden in request body
- Used for responses to hide sensitive data from query strings

Example POST form:
```html
<form method="POST" action="https://sp.example.com/saml/acs">
  <input type="hidden" name="SAMLResponse" value="PHNhbWxwOlJlc3BvbnNlIi...">
  <noscript><button type="submit">Submit</button></noscript>
</form>
<script>document.forms[0].submit();</script>
```

### Artifact Binding
- IdP sends a small artifact (reference) instead of the full assertion
- SP retrieves the actual assertion via a back-channel SOAP request
- Most secure (keeps assertion off the browser) but complex; rarely used

---

## Metadata

SAML metadata is an XML document describing the IdP or SP's capabilities and endpoints:

```xml
<EntityDescriptor>
  <IDPSSODescriptor>
    <SingleSignOnService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect" 
                         Location="https://idp.example.com/sso"/>
    <KeyDescriptor use="signing">
      <KeyInfo><X509Data><X509Certificate>...</X509Certificate></X509Data></KeyInfo>
    </KeyDescriptor>
  </IDPSSODescriptor>
</EntityDescriptor>
```

Contains:
- **SSO Endpoint:** Where to send AuthnRequest (format, binding)
- **Signing Certificate:** Public key to verify assertion signatures
- **Entity ID:** Unique identifier for the IdP or SP
- **NameID Format:** Identity format the IdP supports

SPs download IdP metadata to configure SSO. Many IdPs expose metadata publicly (e.g., `https://idp.example.com/metadata.xml`).

---

## Single Sign-On (SSO) Flow

### Initial Login
1. User accesses SP (no session): e.g., `https://app.example.com/dashboard`
2. SP detects unauthenticated user; generates **AuthnRequest** with unique ID
3. SP redirects user to IdP: `https://idp.example.com/sso?SAMLRequest=...`
4. IdP displays login form (or skips if user already has IdP session)
5. User enters credentials; IdP validates
6. IdP generates cryptographically signed **Assertion** containing user identity + attributes
7. IdP returns HTML form with **Response** (POST binding)
8. Browser auto-submits form to SP's ACS endpoint
9. SP validates signature, checks timestamps, extracts user attributes
10. SP creates local session (session ID cookie)
11. User is logged in

### Subsequent Logins (Cross-App)
- User accesses second SP without session
- Second SP initiates AuthnRequest to **same IdP**
- IdP checks if user already has active IdP session
- If yes: IdP skips login form, immediately returns signed assertion
- Second SP validates, creates local session
- **Result:** User transparent; no re-entry of credentials

---

## Single Logout (SLO)

### Logout Initiation
1. User clicks "logout" in SP
2. SP sends **LogoutRequest** to IdP: `https://idp.example.com/slo?SAMLRequest=...`
3. IdP receives logout request, deletes IdP session
4. IdP sends **LogoutResponse** back to SP
5. SP deletes local session
6. User is logged out everywhere

### Limitations
- SLO is complex and often not fully implemented
- IdP must track all SPs user is logged into (difficult in large federations)
- Some apps skip SLO; users remain logged into some apps after logout from others
- Best practice: Use session timeouts in addition to SLO

---

## XML Signature and Certificate Pinning

SAML responses are signed using XML Digital Signature (XML-Sig). The signature proves:
- The IdP (not an attacker) created the assertion
- The assertion hasn't been tampered with

### Signature Verification Process
1. SP extracts the **Reference** (part of response being signed)
2. SP calculates hash of the reference (e.g., SHA-256)
3. SP decrypts the **SignatureValue** using IdP's public certificate
4. SP compares: does calculated hash match decrypted hash?
   - **Match:** Assertion is authentic
   - **No match:** Rejectionor attacker tampering detected

### Certificate Pinning (Advanced)
- SP hardcodes the IdP's certificate (or certificate thumbprint)
- Prevents attacker-in-middle attacks (rogue certificates)
- Requires manual cert updates when IdP rotates keys
- Uncommon; most SPs trust CA-signed certificates

---

## SAML vs OIDC/OAuth Comparison

| Aspect | SAML 2.0 | OIDC | OAuth 2.0 |
|--------|----------|------|----------|
| **Purpose** | Federated SSO + identity | Modern SSO + identity provider | Delegation / authorization |
| **Protocol** | XML-based, dated | JSON-based, modern | JSON-based, modern |
| **Target** | Enterprise, legacy systems | Consumer apps, modern cloud | API / third-party access |
| **Complexity** | High (XML, bindings, artifacts) | Low (HTTP + JSON) | Low (HTTP + JSON) |
| **User authentication** | IdP ↔ SP only | IdP ↔ App + OIDC provider | Not part of core spec |
| **Use case** | Corporate SSO (Okta → Salesforce) | Consumer login (Google Sign-In) | "Login with Facebook" + API access |
| **Learning curve** | Steep | Gentle | Gentle |
| **Standards body** | OASIS | OpenID Foundation + IETF | IETF |

**When to use SAML:**
- Enterprise customers demand it
- Integrating with corporate directories (AD, Okta)
- High-security federated scenarios
- Mature organizations with compliance requirements

**When to use OIDC:**
- Building modern web/mobile apps
- Consumer authentication
- Simplicity is priority
- Integrating with identity providers (Google, GitHub, Microsoft)

---

## Common Vulnerabilities and Defense

### XML External Entity (XXE) Injection
**Risk:** Malicious XML with embedded entity expansion consumes server resources or reads local files.

**Defense:**
- Disable XML entity resolution in parser
- Use strict XML parsers that reject DTD
- Validate XML schema against SAML spec

### Replay Attacks
**Risk:** Attacker captures an assertion and replays it later to impersonate the user.

**Defense:**
- Check `NotBefore` / `NotOnOrAfter` timestamps (keep tight, e.g., 5 minutes)
- Track response IDs seen; reject duplicates
- Validate `InResponseTo` matches the AuthnRequest ID

### Signature Wrapping / XPath Confusion
**Risk:** Attacker modifies unsigned parts of XML while keeping signature valid (XML canonicalization mismatches).

**Defense:**
- Use canonicalization algorithm consistently
- Validate entire assertion, not just signature
- Sign assertions, not just responses

### Cross-Site Request Forgery (CSRF) on AuthnRequest
**Risk:** Attacker tricks user into accessing attacker-controlled link; SP initiates login without state verification.

**Defense:**
- SP stores state (e.g., nonce) before sending AuthnRequest
- Validate state in InResponseTo
- Use same-site cookies

---

## Deployment Patterns

### Service Provider Setup
1. Obtain IdP metadata (URL provided by IdP)
2. Configure SAML assertion consumer service (ACS) endpoint: `https://myapp.com/saml/acs`
3. Configure identity provider settings in app
4. Test login flow end-to-end
5. Enable SAML login option for users

### Identity Provider Setup (If Building One)
1. Implement AuthnRequest parsing
2. Validate signature on AuthnRequest (optional but recommended)
3. Implement user authentication UI
4. Generate signed assertion with user identity + attributes
5. Validate SP endpoint matches registered ACS
6. Return Response via HTTP POST binding

---

## See Also

- [web-authentication-patterns.md](web-authentication-patterns.md) — broad authentication patterns comparison
- [security-identity.md](security-identity.md) — identity management and federated identity
- [security-oauth2-oidc.md](security-oauth2-oidc.md) — modern OAuth 2.0 / OIDC alternative
- [api-authentication.md](api-authentication.md) — API authentication patterns