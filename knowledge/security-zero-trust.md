# Zero Trust Architecture

## Overview

Zero trust is a security model that eliminates implicit trust based on network location. Every request is authenticated, authorized, and encrypted regardless of origin. The perimeter is identity, not the network.

## Core Principles

### Never Trust, Always Verify

- No implicit trust for any entity (user, device, service, network)
- Every access request is fully authenticated and authorized
- Continuous verification — not just at connection establishment
- Context-aware decisions: identity + device + location + behavior + time

### Least Privilege

- Grant minimum permissions required for each task
- Time-bound access (just-in-time, just-enough)
- Automatic revocation when context changes
- No standing privileges for admin access

### Assume Breach

- Design as if attackers are already inside the network
- Minimize blast radius through segmentation
- Encrypt all data in transit (even internal)
- Comprehensive logging and anomaly detection
- Lateral movement prevention

## Architecture Components

```
┌──────────────────────────────────────────────────────────┐
│                    Policy Decision Point                  │
│  ┌──────────┐  ┌──────────────┐  ┌─────────────────┐    │
│  │ Identity  │  │ Device Trust │  │ Risk Engine     │    │
│  │ Provider  │  │ Assessment   │  │ (Behavioral)    │    │
│  └──────────┘  └──────────────┘  └─────────────────┘    │
└──────────────────────┬───────────────────────────────────┘
                       │ Allow / Deny / Step-up
                       ▼
┌──────────────────────────────────────────────────────────┐
│                  Policy Enforcement Point                 │
│  ┌──────────────┐  ┌────────────┐  ┌──────────────┐     │
│  │ Identity-Aware│  │ Service    │  │ Micro-        │     │
│  │ Proxy         │  │ Mesh       │  │ Segmentation  │     │
│  └──────────────┘  └────────────┘  └──────────────┘     │
└──────────────────────────────────────────────────────────┘
                       │
                       ▼
              ┌─────────────────┐
              │   Resources     │
              │ (Apps, Data,    │
              │  Services, APIs)│
              └─────────────────┘
```

## Identity-Centric Access

### Identity as the New Perimeter

| Traditional                 | Zero Trust                |
| --------------------------- | ------------------------- |
| VPN = trusted               | VPN irrelevant to trust   |
| Internal network = safe     | All networks hostile      |
| Firewall rules by IP        | Policies by identity      |
| Once authenticated, trusted | Continuous authentication |
| Broad network access        | Per-resource access       |

### Strong Identity Requirements

- **Multi-factor authentication (MFA)** for all users, all access
- **Phishing-resistant MFA**: FIDO2/WebAuthn, passkeys, hardware keys
- **Single sign-on (SSO)** with centralized identity provider
- **Service identities**: x509 certificates, SPIFFE IDs, workload identity
- **Identity governance**: lifecycle management, access reviews, certification

### Conditional Access Policies

```
IF user.role == "developer"
   AND device.compliance == true
   AND device.managedBy == "corporate"
   AND location.risk < "high"
   AND signInRisk < "medium"
THEN allow access to dev-portal
   WITH session_duration = 8h
   AND re-auth_on_sensitive_action = true
ELSE
   IF user.role == "developer" AND device.compliance == false
   THEN redirect to device-remediation
   ELSE deny AND log
```

## Device Trust

### Device Posture Assessment

| Signal              | Check                            | Action if Failed    |
| ------------------- | -------------------------------- | ------------------- |
| OS version          | Minimum supported version        | Block or quarantine |
| Patch level         | Critical patches applied         | Require update      |
| Disk encryption     | FileVault / BitLocker enabled    | Block access        |
| Firewall            | Host firewall active             | Warn or block       |
| Endpoint protection | EDR/AV running and updated       | Block access        |
| Jailbreak/root      | Device integrity check           | Block access        |
| Certificate         | Valid device certificate present | Block access        |
| Management          | MDM-enrolled                     | Limit access scope  |

### Device Trust Tiers

| Tier               | Trust Level                       | Access                        |
| ------------------ | --------------------------------- | ----------------------------- |
| **Fully managed**  | Company-issued, MDM, compliant    | Full access to all resources  |
| **BYOD enrolled**  | Personal, MDM-enrolled, compliant | Access to most resources      |
| **BYOD unmanaged** | Personal, no MDM                  | Web-only access, limited data |
| **Unknown**        | No device assessment possible     | Public resources only         |

## BeyondCorp / ZTNA

### Google BeyondCorp Model

```
┌──────────┐    ┌──────────────┐    ┌───────────────┐    ┌──────────┐
│  User +  │───▶│  Access      │───▶│  Access       │───▶│  App     │
│  Device  │    │  Proxy       │    │  Control      │    │          │
└──────────┘    └──────────────┘    │  Engine       │    └──────────┘
                                    │  ┌──────────┐ │
                                    │  │ Device   │ │
                                    │  │ Inventory│ │
                                    │  ├──────────┤ │
                                    │  │ User     │ │
                                    │  │ Groups   │ │
                                    │  ├──────────┤ │
                                    │  │ Trust    │ │
                                    │  │ Scoring  │ │
                                    │  └──────────┘ │
                                    └───────────────┘
```

Key principles:

- Access determined by device and user, not network
- All access through identity-aware proxy
- No privileged internal network
- Device inventory and trust scoring
- Continuous monitoring and re-evaluation

### ZTNA Products

| Product                      | Type            | Key Feature                      |
| ---------------------------- | --------------- | -------------------------------- |
| Google BeyondCorp Enterprise | SaaS            | Chrome integration, DLP          |
| Zscaler Private Access       | SaaS            | App connector, no inbound ports  |
| Cloudflare Access            | SaaS            | Edge network, cheap              |
| Palo Alto Prisma Access      | SaaS            | Full SASE integration            |
| Tailscale                    | Overlay network | WireGuard-based, easy deploy     |
| OpenZiti                     | OSS             | Embeddable zero trust networking |

## Micro-Segmentation

### Network-Level

```yaml
# Kubernetes NetworkPolicy — default deny all
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: default-deny-all
  namespace: production
spec:
  podSelector: {}
  policyTypes: [Ingress, Egress]

---
# Allow specific communication
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-frontend-to-api
spec:
  podSelector:
    matchLabels:
      app: api-server
  ingress:
    - from:
        - podSelector:
            matchLabels:
              app: frontend
      ports:
        - port: 8080
          protocol: TCP
```

### Service Mesh (Istio)

```yaml
# AuthorizationPolicy — allow only specific service
apiVersion: security.istio.io/v1
kind: AuthorizationPolicy
metadata:
  name: api-server-policy
spec:
  selector:
    matchLabels:
      app: api-server
  rules:
    - from:
        - source:
            principals: ["cluster.local/ns/default/sa/frontend"]
      to:
        - operation:
            methods: ["GET", "POST"]
            paths: ["/api/v1/*"]
```

## Service-to-Service Authentication

### Mutual TLS (mTLS)

```
┌──────────┐                        ┌──────────┐
│ Service A │──── TLS Handshake ───▶│ Service B │
│ (Client)  │                       │ (Server)  │
│           │◀── Server Cert ───────│           │
│           │──── Client Cert ─────▶│           │
│           │◀── Verify + Encrypt ──│           │
└──────────┘                        └──────────┘
```

Both sides present and verify X.509 certificates. Service meshes (Istio, Linkerd) automate mTLS with transparent sidecar proxies.

### SPIFFE / SPIRE

**SPIFFE** (Secure Production Identity Framework For Everyone) provides a standard for service identity:

```
# SPIFFE ID format
spiffe://trust-domain/path

# Examples
spiffe://example.com/ns/production/sa/api-server
spiffe://example.com/region/us-east/service/payments
```

**SPIRE** (SPIFFE Runtime Environment) implements the SPIFFE standard:

```
┌──────────────────────────────────────────┐
│                SPIRE Server              │
│  ┌──────────┐  ┌────────────────────┐    │
│  │ CA       │  │ Registration       │    │
│  │ (Signing)│  │ Entries            │    │
│  └──────────┘  └────────────────────┘    │
└──────────────────┬───────────────────────┘
                   │ Attestation
                   ▼
┌──────────────────────────────────────────┐
│              SPIRE Agent (per node)       │
│  ┌───────────┐  ┌─────────────────────┐  │
│  │ Workload  │  │ SVID Cache          │  │
│  │ Attestor  │  │ (x509 + JWT SVIDs)  │  │
│  └───────────┘  └─────────────────────┘  │
└──────────────────────────────────────────┘
         │
         ▼
   ┌──────────┐
   │ Workload  │  ← Gets SVID via Workload API
   └──────────┘
```

**SVIDs** (SPIFFE Verifiable Identity Documents): X.509 certificates or JWT tokens with SPIFFE IDs as subject.

## Policy Engines

### Open Policy Agent (OPA) / Rego

```rego
# policy.rego — API authorization
package authz

default allow := false

# Allow if user has required role
allow if {
    input.method == "GET"
    input.path == ["api", "v1", "users"]
    "admin" in input.user.roles
}

# Allow users to read their own data
allow if {
    input.method == "GET"
    input.path == ["api", "v1", "users", user_id]
    input.user.id == user_id
}

# Deny access outside business hours
deny if {
    not is_business_hours
    not "emergency" in input.user.roles
}

is_business_hours if {
    hour := time.clock(time.now_ns())[0]
    hour >= 8
    hour < 18
}
```

OPA integrates with:

- **Kubernetes**: Gatekeeper admission controller
- **Envoy**: External authorization filter
- **Terraform**: Policy-as-code for infrastructure
- **API gateways**: Kong, NGINX, custom middleware

### Cedar (AWS)

```cedar
// Allow users to read their own profile
permit(
    principal,
    action == Action::"ReadProfile",
    resource
) when {
    principal == resource.owner
};

// Admins can perform any action
permit(
    principal in Group::"admins",
    action,
    resource
);

// Deny access from untrusted devices
forbid(
    principal,
    action,
    resource
) when {
    !context.device.compliant
};
```

Cedar provides formal verification — you can mathematically prove policy properties (e.g., "no policy allows unauthenticated access").

## Identity-Aware Proxy

### Implementation Pattern

```
┌──────────┐     ┌──────────────────┐     ┌──────────┐
│  Client  │────▶│  Identity-Aware  │────▶│  Backend │
│          │     │  Proxy (IAP)     │     │  Service │
│          │     │  - AuthN (OIDC)  │     │          │
│          │     │  - AuthZ (policy)│     │          │
│          │     │  - Device check  │     │          │
│          │     │  - Audit log     │     │          │
└──────────┘     └──────────────────┘     └──────────┘
```

Backend services receive pre-authenticated requests with identity headers:

- `X-Forwarded-User`: authenticated user ID
- `X-Forwarded-Email`: user email
- `X-Forwarded-Groups`: group memberships

### OSS Options

| Tool                 | Features                                          |
| -------------------- | ------------------------------------------------- |
| **OAuth2-proxy**     | OIDC auth, cookie sessions, upstream headers      |
| **Pomerium**         | Context-aware access, device trust, policy engine |
| **Oathkeeper** (Ory) | API gateway, rule-based access, identity pipeline |

## Implementation Roadmap

### Phase 1: Foundation (Months 1-3)

- [ ] Inventory all users, devices, services, and data flows
- [ ] Deploy centralized identity provider (IdP)
- [ ] Enforce MFA for all user accounts
- [ ] Implement SSO for all applications
- [ ] Enable comprehensive logging and SIEM ingestion
- [ ] Classify data by sensitivity level

### Phase 2: Visibility (Months 3-6)

- [ ] Deploy device posture assessment
- [ ] Map all service-to-service communication
- [ ] Implement network flow logging
- [ ] Create baseline behavioral profiles
- [ ] Define and document trust boundaries
- [ ] Establish conditional access policies (monitor mode)

### Phase 3: Control (Months 6-12)

- [ ] Enforce conditional access policies
- [ ] Deploy micro-segmentation (default deny)
- [ ] Implement mTLS for service-to-service
- [ ] Deploy identity-aware proxy for internal apps
- [ ] Implement just-in-time (JIT) access for admin
- [ ] Automate device compliance enforcement

### Phase 4: Optimization (Months 12-18)

- [ ] Deploy behavioral analytics / UEBA
- [ ] Implement continuous authentication
- [ ] Automate incident response playbooks
- [ ] Remove VPN dependency for application access
- [ ] Conduct red team exercises against ZTA controls
- [ ] Continuous policy refinement from telemetry

## Zero Trust Anti-Patterns

| Anti-Pattern             | Why It Fails                                   | Fix                                          |
| ------------------------ | ---------------------------------------------- | -------------------------------------------- |
| "Zero trust = no VPN"    | Removing VPN without replacement leaves gaps   | Deploy ZTNA before decommissioning VPN       |
| Network-only approach    | Firewall rules without identity are incomplete | Identity-first, network as defense-in-depth  |
| Big-bang migration       | Disrupts operations, creates shadow IT         | Phased rollout, start with high-value assets |
| Ignoring legacy systems  | Unprotectable systems create backdoors         | Isolation + enhanced monitoring for legacy   |
| MFA fatigue exploitation | Push-bomb attacks bypass MFA                   | Number matching, phishing-resistant MFA      |
| Over-reliance on agents  | Agent-based only excludes BYOD/contractors     | Agentless options for unmanaged devices      |
| Static policies          | Fixed rules can't adapt to new threats         | Continuous risk scoring, adaptive policies   |

## Maturity Assessment

| Capability  | Initial           | Developing            | Advanced               | Optimal              |
| ----------- | ----------------- | --------------------- | ---------------------- | -------------------- |
| Identity    | Passwords         | MFA                   | Phishing-resistant MFA | Continuous auth      |
| Device      | No assessment     | Basic posture         | Real-time compliance   | Behavioral trust     |
| Network     | Perimeter only    | Macro-segmentation    | Micro-segmentation     | Identity-based       |
| Application | Network access    | SSO + basic authz     | Context-aware          | Adaptive             |
| Data        | No classification | Manual classification | Automated DLP          | Real-time protection |
| Visibility  | Basic logs        | SIEM                  | UEBA                   | Automated response   |
