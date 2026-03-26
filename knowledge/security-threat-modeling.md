# Threat Modeling

## Overview

Threat modeling systematically identifies, quantifies, and addresses security threats to a system. It answers four questions: What are we building? What can go wrong? What are we going to do about it? Did we do a good enough job? Effective threat modeling happens early (design phase) and continuously (each change).

## STRIDE

Microsoft's STRIDE framework categorizes threats by type:

| Threat                     | Property Violated | Description                         | Example                                    |
| -------------------------- | ----------------- | ----------------------------------- | ------------------------------------------ |
| **S**poofing               | Authentication    | Pretending to be another entity     | Forged JWT, session hijacking              |
| **T**ampering              | Integrity         | Modifying data or code              | SQL injection, modified binary             |
| **R**epudiation            | Non-repudiation   | Denying performed actions           | Missing audit logs, unsigned transactions  |
| **I**nformation Disclosure | Confidentiality   | Exposing data to unauthorized party | Error stack traces, directory listing      |
| **D**enial of Service      | Availability      | Making system unavailable           | Resource exhaustion, amplification attacks |
| **E**levation of Privilege | Authorization     | Gaining unauthorized access level   | IDOR, broken role checks, container escape |

### STRIDE Per Element

Apply STRIDE to each element in a data flow diagram:

| Element         | Applicable Threats           |
| --------------- | ---------------------------- |
| External entity | S, R                         |
| Process         | S, T, R, I, D, E             |
| Data store      | T, R, I, D                   |
| Data flow       | T, I, D                      |
| Trust boundary  | (Analyze all crossing flows) |

## DREAD Scoring

Risk quantification model (1-10 scale per factor):

| Factor              | Question                 | Low (1-3)               | High (7-10)            |
| ------------------- | ------------------------ | ----------------------- | ---------------------- |
| **D**amage          | How bad is the impact?   | Minor data leak         | Full system compromise |
| **R**eproducibility | How easy to reproduce?   | Race condition needed   | Always works           |
| **E**xploitability  | How much skill needed?   | Expert + custom tooling | Script kiddie          |
| **A**ffected Users  | How many users impacted? | Single user             | All users              |
| **D**iscoverability | How easy to find?        | Obscure internal flow   | Public API endpoint    |

**Risk = (D + R + E + A + D) / 5**

| Score | Priority                     |
| ----- | ---------------------------- |
| 9-10  | Critical — fix immediately   |
| 7-8   | High — fix in current sprint |
| 4-6   | Medium — fix in next release |
| 1-3   | Low — accept or backlog      |

> Note: DREAD is somewhat subjective. Use it for relative prioritization, not absolute measurement. Some organizations replace it with CVSS.

## Attack Trees

Hierarchical decomposition of goals an attacker might have:

```
[Goal: Steal User Credentials]
├── [OR: Phishing Attack]
│   ├── [AND: Craft convincing email]
│   └── [AND: Host fake login page]
├── [OR: Exploit XSS]
│   ├── [AND: Find injection point]
│   └── [AND: Steal session cookie]
├── [OR: Compromise Database]
│   ├── [AND: SQL Injection]
│   └── [AND: Exploit unpatched DB]
└── [OR: Intercept Network Traffic]
    ├── [AND: ARP spoofing on LAN]
    └── [AND: Compromised CA certificate]
```

Each leaf node can be annotated with:

- **Cost** to attacker
- **Technical difficulty**
- **Detection likelihood**
- **Prerequisite conditions**

## Data Flow Diagrams (DFDs)

### Elements

```
┌─────────┐     ┌─────────┐     ┌─────────┐
│ External │────▶│ Process │────▶│ Data    │
│ Entity   │     │         │     │ Store   │
└─────────┘     └─────────┘     └─────────┘
    ◇               ○               ═══
```

| Symbol         | Meaning                                    |
| -------------- | ------------------------------------------ |
| Rectangle      | External entity (user, third-party system) |
| Circle/Rounded | Process (code that transforms data)        |
| Parallel lines | Data store (database, file, cache)         |
| Arrow          | Data flow (labeled with data type)         |
| Dashed line    | Trust boundary                             |

### Example: Web Application DFD

```
                    ┌ ─ ─ ─ ─ Trust Boundary ─ ─ ─ ─ ─ ─ ─ ─ ┐
                    │                                           │
┌──────────┐  HTTPS   ┌──────────┐  SQL    ┌──────────────┐   │
│  Browser │───────▶│  │  Web App │───────▶ │  Database    │   │
│  (User)  │◀───────│  │  Server  │◀─────── │  (PostgreSQL)│   │
└──────────┘  HTML    │ └──────────┘ Results └──────────────┘   │
                    │       │                                   │
                    │       │ REST API                          │
                    │       ▼                                   │
                    │  ┌──────────┐                             │
                    │  │  3rd Party│                            │
                    │  │  Payment  │                            │
                    │  └──────────┘                             │
                    └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
```

### Trust Boundaries

Lines where the level of trust changes. Every data flow crossing a trust boundary needs scrutiny:

| Boundary            | Examples                       |
| ------------------- | ------------------------------ |
| Internet ↔ DMZ      | Load balancer, WAF             |
| DMZ ↔ Internal      | API gateway, firewall          |
| App ↔ Database      | Connection pool, query layer   |
| Process ↔ Process   | Service mesh, mTLS             |
| User ↔ Admin        | Role check, privilege boundary |
| Cloud ↔ On-premises | VPN, encrypted tunnel          |

## PASTA Methodology

Process for Attack Simulation and Threat Analysis — 7-stage risk-centric approach:

| Stage                        | Activity                                                  | Output                            |
| ---------------------------- | --------------------------------------------------------- | --------------------------------- |
| 1. Define Objectives         | Business impact analysis, compliance requirements         | Risk profile, security objectives |
| 2. Define Technical Scope    | Architecture diagrams, technology stack enumeration       | Technical scope document          |
| 3. Application Decomposition | DFDs, trust boundaries, entry points, assets              | Decomposition diagrams            |
| 4. Threat Analysis           | Threat intelligence, attacker profiling, threat libraries | Threat matrix                     |
| 5. Vulnerability Analysis    | Vulnerability scanning, code review, pentest results      | Vulnerability list                |
| 6. Attack Modeling           | Attack trees, attack simulation, exploit analysis         | Attack scenarios                  |
| 7. Risk & Impact             | Risk scoring, countermeasure mapping, residual risk       | Prioritized mitigations           |

PASTA integrates business context (stages 1-2) with technical analysis (3-5) and attack simulation (6-7), making it suitable for organizations needing business-aligned threat analysis.

## MITRE ATT&CK Framework

### Tactics (The "Why")

| ID     | Tactic               | Purpose                  |
| ------ | -------------------- | ------------------------ |
| TA0001 | Initial Access       | Gain foothold            |
| TA0002 | Execution            | Run malicious code       |
| TA0003 | Persistence          | Maintain access          |
| TA0004 | Privilege Escalation | Gain higher permissions  |
| TA0005 | Defense Evasion      | Avoid detection          |
| TA0006 | Credential Access    | Steal credentials        |
| TA0007 | Discovery            | Learn about environment  |
| TA0008 | Lateral Movement     | Move through network     |
| TA0009 | Collection           | Gather target data       |
| TA0010 | Exfiltration         | Steal data out           |
| TA0011 | Command and Control  | Communicate with implant |
| TA0040 | Impact               | Disrupt/destroy          |

### Using ATT&CK in Threat Modeling

1. **Identify relevant techniques** for your architecture
2. **Map existing controls** to technique mitigations
3. **Find gaps** where no detection or prevention exists
4. **Prioritize** based on likelihood and impact
5. **Test** with purple team exercises

### Cloud-Specific Matrices

ATT&CK includes matrices for:

- **Enterprise** (Windows, Linux, macOS, Cloud)
- **Mobile** (Android, iOS)
- **ICS** (Industrial Control Systems)
- **Containers** (Docker, Kubernetes)

## Threat Modeling Tools

### Microsoft Threat Modeling Tool

- Template-based DFD creation
- Auto-generates STRIDE threats per element
- Built-in mitigations library
- Outputs HTML/CSV reports
- Best for: Windows/.NET ecosystems, structured processes

### OWASP Threat Dragon

- Open source, web-based
- STRIDE and custom threat types
- Diagram editor with trust boundaries
- JSON-based threat model storage
- Best for: teams wanting OSS, simpler workflow

### Other Tools

| Tool      | Type            | Best For                        |
| --------- | --------------- | ------------------------------- |
| IriusRisk | Commercial SaaS | Enterprise, compliance-driven   |
| Threagile | OSS, YAML-based | Infrastructure-as-code teams    |
| pytm      | Python library  | Developer-driven, code-as-model |
| CAIRIS    | OSS             | Requirements-driven security    |

### pytm Example

```python
from pytm import TM, Server, Datastore, Dataflow, Boundary, Actor

tm = TM("Web Application")
tm.description = "E-commerce threat model"

internet = Boundary("Internet")
dmz = Boundary("DMZ")
internal = Boundary("Internal")

user = Actor("User")
user.inBoundary = internet

web = Server("Web Server")
web.inBoundary = dmz
web.protocol = "HTTPS"
web.sanitizesInput = True

db = Datastore("Database")
db.inBoundary = internal
db.isEncryptedAtRest = True

user_to_web = Dataflow(user, web, "HTTPS Request")
user_to_web.protocol = "HTTPS"

web_to_db = Dataflow(web, db, "SQL Query")
web_to_db.protocol = "TCP"

tm.process()  # Generates threats and DFD
```

## Threat Modeling in Agile

### Integration Points

| Phase                  | Activity                                        | Time            |
| ---------------------- | ----------------------------------------------- | --------------- |
| **Backlog refinement** | Tag stories with "security-relevant"            | 2 min per story |
| **Sprint planning**    | Include threat review for security stories      | 15 min          |
| **Design**             | Lightweight threat model for new features       | 30-60 min       |
| **Code review**        | Check implementation against identified threats | Part of review  |
| **Retrospective**      | Review security incidents, update threat model  | 10 min          |

### Lightweight Threat Modeling (4-Question Framework)

For each feature or change:

1. **What are we changing?** (Scope: new endpoint, data flow, integration)
2. **What could go wrong?** (STRIDE brainstorm, 5 minutes)
3. **What are we going to do about it?** (Mitigations → acceptance criteria)
4. **Did we do a good job?** (Verify mitigations in review/testing)

### Threat Modeling as Code

Store threat models alongside code:

```yaml
# threat-model.yaml
system: checkout-service
version: "2.1"
data_flows:
  - name: payment-submission
    source: browser
    destination: payment-api
    crosses_boundary: internet-to-dmz
    data: [credit_card, billing_address]
    threats:
      - type: tampering
        description: "Modified payment amount"
        mitigation: "Server-side price validation"
        status: mitigated
      - type: information_disclosure
        description: "Card data in logs"
        mitigation: "PCI-compliant logging, field masking"
        status: mitigated
```

## Common Threat Patterns by Architecture

### Microservices

| Threat              | Description                         | Mitigation                              |
| ------------------- | ----------------------------------- | --------------------------------------- |
| Lateral movement    | Compromised service accesses others | mTLS, network policies, least privilege |
| Confused deputy     | Service A tricks Service B          | Request signing, context propagation    |
| Data exfiltration   | Service with DB access leaks data   | Egress filtering, DLP                   |
| Supply chain        | Malicious container image           | Image signing, admission control        |
| Configuration drift | Secrets/config exposed              | GitOps, sealed secrets                  |

### Serverless

| Threat                    | Description                     | Mitigation                              |
| ------------------------- | ------------------------------- | --------------------------------------- |
| Event injection           | Malicious event payloads        | Input validation at function entry      |
| Over-privileged functions | Functions with broad IAM roles  | Least-privilege per function            |
| Dependency risk           | Vulnerable libraries in layers  | Dependency scanning, minimal images     |
| Insecure deserialization  | Untrusted data in event sources | Safe deserialization, schema validation |
| Cold start timing         | Timing side-channels            | Constant-time comparisons               |

### API-First

| Threat                  | Description                 | Mitigation                             |
| ----------------------- | --------------------------- | -------------------------------------- |
| Broken authentication   | Weak token validation       | OAuth 2.0 / OIDC, short-lived tokens   |
| BOLA (IDOR)             | Access other users' objects | Object-level authorization             |
| Excessive data exposure | API returns too much data   | Response filtering, field-level access |
| Rate limiting bypass    | Resource exhaustion         | Per-user rate limits, API gateway      |
| Mass assignment         | Unexpected field updates    | Explicit allowlists, DTOs              |

### Monolithic

| Threat                  | Description                   | Mitigation                           |
| ----------------------- | ----------------------------- | ------------------------------------ |
| Single point of failure | Full compromise from one vuln | Defense in depth, WAF                |
| Privilege escalation    | Shared process space          | Sandboxing, capability dropping      |
| Data mixing             | Multi-tenant data leaks       | Tenant isolation, row-level security |

## Security Requirements Derivation

Threat model findings translate to requirements:

```
Threat: SQL injection on search endpoint
→ Requirement: All database queries MUST use parameterized statements
→ Test: SQLMap scan of all endpoints in CI pipeline
→ Control: ORM-only database access policy

Threat: Credential stuffing on login
→ Requirement: Rate limiting (5 attempts/min per IP)
→ Requirement: Account lockout after 10 failures
→ Requirement: CAPTCHA after 3 failures
→ Test: Load test login endpoint with brute force patterns
```

## Threat Model Review Checklist

- [ ] All entry points identified (APIs, UI, file uploads, webhooks)
- [ ] All data stores mapped with sensitivity classification
- [ ] Trust boundaries drawn at every privilege change
- [ ] STRIDE applied to each element
- [ ] Each threat has a disposition (mitigate / accept / transfer / avoid)
- [ ] Mitigations mapped to implementation (code, config, infra)
- [ ] Residual risks documented and accepted by stakeholder
- [ ] Model updated when architecture changes
- [ ] Findings tracked in issue tracker
