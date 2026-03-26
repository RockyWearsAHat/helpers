# Zero Trust Network Architecture — BeyondCorp Model, Micro-Segmentation, Identity-Based Access, ZTNA, Implementation

## Overview

Zero trust networking is a security model that replaces the perimeter-based defense (firewall at the edge) with identity-based access control and continuous verification at every request. This note focuses on network-layer implementation: micro-segmentation, software-defined perimeters, device trust, and ZTNA (Zero Trust Network Access) products. For identity and authentication details, see `security-zero-trust.md`.

## Traditional Perimeter Model vs Zero Trust

**Traditional Model:**
```
┌─────────────────────────────────────────────────────────┐
│                   Protected Network                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  Database    │  │  Fileserver  │  │   Services   │   │
│  └──────────────┘  └──────────────┘  └──────────────┘   │
│                                                           │
└──────────────────────────┬────────────────────────────────┘
                           │
                 ┌─────────▼──────────┐
                 │  Firewall          │
                 │  (VPN required)    │
                 └──────────────────┘
                           │
              ┌────────────▼────────┐
              │  Internet / Untrusted│
              └──────────────────────┘
```

Everyone inside the perimeter is trusted. The firewall blocks external threats. This assumes:
- Employees connect from known networks (office)
- Threats come from outside
- VPN = full access to internal systems

This model breaks with:
- Remote work (employees access from wherever)
- Cloud services (systems are outside the firewall)
- Third-party access (vendors, contractors need selective access)
- Insider threats (motivated insider has full network access)

**Zero Trust Model:**
```
┌──────────────────────────────────────────────────────┐
│  Every Request Evaluated                             │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │  Identity    │  │  Device      │  │ Context    │ │
│  │  (User/App)  │  │  Trust Score │  │ (Location, │ │
│  │              │  │  (posture)   │  │  time, ...)│ │
│  └──────────────┘  └──────────────┘  └────────────┘ │
│         │                 │                 │        │
│         └────────┬────────┴────────┬────────┘        │
│                  │                 │                 │
│            ┌─────▼─────────────────▼────┐            │
│            │ Policy Decision Point (PDP) │            │
│            │ Allow / Deny / MFA / Step-Up│            │
│            └────────────┬─────────────────┘           │
│                         │                             │
│  ┌──────────────┐  ┌────▼───────────┐  ┌──────────┐ │
│  │  Database    │  │  Fileserver    │  │ Services │ │
│  └──────────────┘  └────────────────┘  └──────────┘ │
│                                                       │
└───────────────────────────────────────────────────────┘
```

Every request (internal or external) is authenticated, authorized, and encrypted.

## BeyondCorp: Google's Framework

Google published "BeyondCorp: A New Approach to Enterprise Security" (2014-2015), documenting the shift from VPN-based access to identity-based access. Key insights:

1. **VPN is not security:** VPN grants network access; it doesn't mean the device is trustworthy or the user is authorized for that resource.

2. **Security should move to data:** Instead of assuming everything inside the firewall is safe, encrypt data, enforce access at the application/data layer, and verify every request.

3. **Trust the device, not the network:** Inspect device posture (OS security patches, antivirus status, screen lock enabled) and revoke access from compromised devices.

4. **Access is granted per resource, not per network:** Instead of "inside the firewall" = full access, grant granular access: user X on device Y can access resource Z.

**BeyondCorp Implementation at Google:**
- Removed VPN entirely
- Deployed proxy services at the edge (replacement for the firewall)
- All applications verify identity at the request level
- Cancelled employee certificates; moved to device identity
- Reduced firewall rules from 100k+ to a few hundred

## Micro-Segmentation: Fine-Grained Network Boundaries

Micro-segmentation divides the network into small zones, each with its own access policy. Instead of trusting all internal traffic, each zone is a mini-perimeter.

**Traditional segmentation:**
```
┌─────────────────┐          ┌──────────────────┐
│  Database Zone  │          │  Applications    │
│                 │◄────────►│  Trust all       │
│ Trust all       │  traffic │  traffic         │
└─────────────────┘          └──────────────────┘
```

**Micro-segmentation:**
```
┌─────────────────────────────────────────────────┐
│  Micro-Segmentation                             │
│                                                  │
│  ┌─────────────────────────────────────────┐    │
│  │ Zone: Production DB                    │    │
│  │ ┌──────────────┐  ┌─────────────────┐  │    │
│  │ │ DB: Masters  │  │ DB: Read-only   │  │    │
│  │ │              │  │ (Allowed from:  │  │    │
│  │ │ Allowed from │  │  App tier only) │  │    │
│  │ │  App tier 1  │  │                 │  │    │
│  │ │  (mTLS)      │  │                 │  │    │
│  │ └──────────────┘  └─────────────────┘  │    │
│  └──────────────┬──────────────────────────┘    │
│                │                                 │
│  ┌─────────────▼──────────────────────────┐    │
│  │ Zone: Application Tier                 │    │
│  │ ┌─────────────────┐  ┌───────────────┐│    │
│  │ │ Web (Port 443)  │  │ Worker pools  ││    │
│  │ │ From: Internet  │  │ (internal)    ││    │
│  │ │ (TLS terminate) │  │               ││    │
│  │ └─────────────────┘  └───────────────┘│    │
│  └────────────────────────────────────────┘    │
│                                                  │
└─────────────────────────────────────────────────┘
```

Each zone has explicit ingress/egress rules. Traffic between zones is denied by default; only whitelisted flows are allowed. This limits lateral movement attack surface.

## Software-Defined Perimeter: Identity at the Network Edge

The **SDP (Software-Defined Perimeter)** model formalizes zero trust at the network layer:

```
┌──────────────────────────────────────────────────────┐
│  Single Packet Authorization (SPA) Controller         │
│  ┌─────────────────────────────────────────────┐     │
│  │ 1. User/Device Authenticates                │     │
│  │ 2. SPA Controller Authorizes (context)      │     │
│  │ 3. Dynamically opens firewall port for user │     │
│  └─────────────────┬────────────────────────────┘     │
│                    │ (issue JWT, port #)              │
│       ┌────────────▼───────────────┐                  │
│       │ Gateway Firewall (closed by │                 │
│       │ default; opens on SPA)      │                 │
│       └────────────┬────────────────┘                 │
│                    │                                   │
│  ┌────────────────▼────────────────────────────┐     │
│  │ Protected Services                          │     │
│  │ (Database, Fileserver, APIs)                │     │
│  │ Each service verifies JW Token              │     │
│  └──────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────┘
```

Key principles:
- Network access is denied by default (firewall closed)
- Authorization happens before network access is opened
- Each service verifies the authorization token
- Continuous verification (token expires, device posture changes trigger re-auth)

## Device Trust and Posture Checking

Device posture determines whether to grant access or require step-up authentication (MFA, location verification, etc.).

**Device Trust Signals:**
- OS fully patched? (last 30 days)
- Antivirus/EDR enabled and up-to-date?
- Screen lock enabled?
- Encryption enabled (full-disk or per-volume)?
- Firewall enabled?
- No known compromises (not flagged by threat intel)?

```
Device Context                Access Decision
┌─────────────────────┐       
│ macOS, patched      │ ──┐
│ Screen lock: YES    │   │
│ Firewall: YES       │   ├──► Policy PDP  
│ AV: YES, updated    │   │    (Allow with MFA)
│ EDR: YES            │ ──┤
│ No threats detected │   │
└─────────────────────┘   │
                          │
┌─────────────────────┐    │
│ Windows, patched    │ ──┐
│ Screen lock: NO     │   │
│ Firewall: enabled   │   ├──► Policy PDP
│ AV: outdated        │   │    (Step-up required: MFA + location pin)
│ EDR: running        │ ──┤
│ Malware flagged     │   │
└─────────────────────┘   │
```

This is **adaptive access:** healthy devices get normal access; risky devices face additional friction.

## Continuous Verification

Rather than trusting a device after one successful authentication, continuous verification re-evaluates trust during the session.

**Signals monitored continuously:**
- User location (IP geolocation; if user suddenly in different country, flag)
- Time-of-day anomalies (user usually works 9-5 PT; access at 2 AM is suspicious)
- Unusual access patterns (accessing data they've never accessed before)
- Device compromises discovered via threat intel
- Token expiry (force re-auth after X minutes/hours)

**On trust change:**
- Warn user: "Unusual login from new location"
- Require step-up: re-authenticate or approve via mobile app
- Revoke session: if device compromised, revoke active sessions immediately

## ZTNA (Zero Trust Network Access) Products

ZTNA solutions replace traditional VPN and firewalls with zero trust architecture.

**Zscaler (large enterprises):**
- Cloud-based security proxy
- All traffic routed through Zscaler cloud (SSL inspection, threat intelligence)
- Device posture checks before granting access
- Micro-segmentation policies (user X → resource Y only)

**Cloudflare Access (scale well, developer-friendly):**
- Identity provider integrations (Okta, AzureAD, etc.)
- Application-level access control (not network VPN)
- Replaces VPN with application proxies
- MFA, device posture, continuous trust evaluation
- Developer-friendly (easy setup, JSON policy rules)

**Fortinet FortiZero Trust:**
- Network segmentation via FortiGate appliances
- Integration with FortiEDR for device posture
- Adaptive access based on device health, user context

**BIG-IP (F5):**
- Edge gateway with zero trust policies
- Micro-segmentation for internal networks
- Integration with identity providers

**Common Features:**
- Identity-based access (not IP-based)
- Device posture validation
- Single packet authorization or challenge-response
- Encryption in transit (TLS)
- Centralized policy engine
- Logging/audit for compliance

## Implementation Journey: VPN → Zero Trust

**Phase 1: Parallel Run**
- Deploy ZTNA/proxy alongside VPN
- Pilot users migrate from VPN
- Collect feedback

**Phase 2: Identity Services**
- Ensure all systems have identity/authentication (LDAP/AD, Okta, etc.)
- Applications authenticate requests (not just network)
- Device posture tooling deployed (MDM, EDR)

**Phase 3: Micro-Segmentation**
- Define security zones
- Implement policies (firewall, service mesh rules)
- Gradual migration of workloads

**Phase 4: VPN Deprecation**
- VPN disabled for new employees
- Legacy apps refactored or replaced
- Full zero trust enforcement

**Timeline:** Typically 18-36 months for large enterprises; 3-6 months for smaller orgs with simpler infrastructure.

## Common Misconceptions

1. **Zero trust = no access:** No. Zero trust = *selective, verified* access. It should be frictionless for legitimate users and devices.

2. **Zero trust is expensive:** Depends. Cloud-based ZTNA (Cloudflare, Zscaler) has modest cost. On-premises micro-segmentation requires networking and security depth.

3. **Zero trust replaces encryption:** No. Encryption is orthogonal and still essential (see `security-tls-certificates.md`). Zero trust controls *who* can access; encryption protects the data itself.

4. **Zero trust = no lateral movement ever:** In practice, lateral movement is constrained, not impossible. Well-implemented zero trust makes lateral movement expensive and detectable.

## See Also

- `security-zero-trust.md` — Zero trust principles, identity, authorization frameworks
- `security-network.md` — Firewalls, IDS/IPS, defensive layers
- `devops-service-mesh.md` — Service mesh as a zero trust reinforcement at the application layer
- `infrastructure-container-networking.md` — Micro-segmentation for containerized workloads