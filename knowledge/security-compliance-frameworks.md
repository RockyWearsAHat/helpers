# Security — Compliance Frameworks: SOC 2, ISO 27001, PCI DSS, HIPAA, FedRAMP

## Overview

Compliance frameworks are structured collections of security, availability, processing integrity, confidentiality, and privacy controls. Frameworks differ by industry, geography, data type, and risk profile. This guide covers major frameworks: SOC 2 (trust services), ISO 27001 (information security management), PCI DSS (payment card security), HIPAA (healthcare privacy), and FedRAMP (US government cloud).

## SOC 2 (Service Organization Control)

**Scope**: Controls for service organizations processing customer data. Issued by AICPA (American Institute of Certified Public Accountants). Two types.

### SOC 2 Type I

**What**: Statement that controls are designed effectively.

**When**: Point-in-time assessment. "As of [date], controls are designed to achieve [trust service criteria]."

**Timing**: 6 weeks to audit; report valid for ~1 year. Quick but provides limited evidence of actual implementation.

**Audit process**: Review control designs, interview personnel, inspect configurations.

**Use case**: Early-stage companies needing fast compliance proof.

### SOC 2 Type II

**What**: Statement that controls are designed effectively AND operated effectively over time.

**When**: Continuous assessment. "From [date] to [date], controls operated effectively."

**Timing**: Minimum 6 months of operational testing. Significantly more rigorous; demonstrates ongoing control performance.

**Audit process**: Continuous monitoring, testing of actual control execution over period, review of logs and monitoring output.

**Use case**: Mature companies, required by customers for ongoing vendor risk management.

### Trust Service Criteria (TSC)

| Category | Criteria | Examples |
|----------|----------|----------|
| **Security** | Protect against unauthorized access, use, disclosure | System monitoring, access controls, encryption, audit logs |
| **Availability** | System available 99.9%+ uptime, backup/recovery | Infrastructure redundancy, disaster recovery, monitoring |
| **Processing Integrity** | Data accurately processed, completely, timely | Input validation, transaction logging, completeness checks |
| **Confidentiality** | Sensitive data disclosed only per policy | Encryption at rest/in-transit, access controls, data classification |
| **Privacy** | Personal data collected, used, retained per policy | Consent management, data retention, subject access requests |

**Audit scope**: Organization selects which criteria are relevant. E-commerce company might audit Security + Availability + Processing Integrity. Healthcare might add Privacy and Confidentiality.

## ISO 27001 (Information Security Management System)

**Scope**: International standard for information security management. Applies to organizations of any size, industry.

**Certification process**: Organization implements ISMS, third-party certifier audits annually, certification valid 3 years with annual surveillance audits.

### ISMS (Information Security Management System)

Five phases:
1. **Plan**: Define scope, policy, risk assessment
2. **Do**: Implement controls, training, awareness
3. **Check**: Monitor, measure, audit
4. **Act**: Correct deficiencies, continuous improvement
5. **Maintain**: Documentation, change management

### Annex A Controls

**114 controls** across 14 domains:

| Domain | Example Controls |
|--------|------------------|
| **Organization** | Information security policy, roles/responsibilities, security awareness training |
| **People** | Competence, screening, acceptable use, employee termination procedures |
| **Assets** | Asset inventory, ownership, classification, media disposal |
| **Access Control** | Authentication, authorization, privilege management, session logging |
| **Cryptography** | Encryption of sensitive data, key management |
| **Physical & Environmental** | Secure premises, visitor management, clean desk policy |
| **Operations** | Change management, incident management, backup & recovery, system monitoring |
| **Communications** | Network segregation, handling of removable media, data transfer |
| **System Acquisition/Development** | Security in development, secure coding, testing, vulnerability management |
| **Supplier/Vendor Management** | Third-party risk assessment, contracts, monitoring |
| **Information Security Incidents** | Detection, response, containment, recovery, lessons learned |
| **Business Continuity** | Recovery plans, testing, restoration procedures |
| **Compliance** | Legal obligations monitoring, intellectual property, personal data handling |

**Customization**: Organizations select controls relevant to risk profile. Not all 114 required; scope-driven.

## PCI DSS (Payment Card Industry Data Security Standard)

**Scope**: Any organization processing, transmitting, or storing payment card data (credit cards).

**Versions**: PCI DSS 3.2.1 (legacy, ending support 2024), PCI DSS 4.0 (current, effective 2024).

### 12 Core Requirements (PCI DSS 4.0)

| Requirement | Focus |
|-------------|-------|
| 1 | Network segmentation; firewall; no direct internet access to cardholder data |
| 2 | Default security parameters; no default passwords; hardened configurations |
| 3 | Protect stored card data via encryption, truncation, hashing; never store sensitive data (CVV, PIN) |
| 4 | Encrypt data in transit (TLS 1.2+) |
| 5 | Protect against malware; antivirus, asset monitoring |
| 6 | Secure development; secure SDLC; vulnerability testing; patch management; attack surface reduction |
| 7 | Restrict cardholder data access to minimum necessary (least privilege) |
| 8 | Identify access by user; strong authentication; MFA for admin access |
| 9 | Restrict physical access to systems; visitor logs; media disposal |
| 10 | Audit logging; tamper-proof logs; log retention ≥1 year |
| 11 | Regular testing; penetration testing annually; vulnerability scanning quarterly |
| 12 | Information security policy; risk management; compliance monitoring |

### SAQ (Self-Assessment Questionnaire)

Three levels of scope/complexity:

- **SAQ A**: Minimal scope (e.g., merchant using hosted payment processor with no data storage on premises)
- **SAQ B**: Small organizations with limited data processing
- **SAQ D**: Large, complex, full cardholder data environment—requires third-party QSA (Qualified Security Assessor) audit

## HIPAA (Health Insurance Portability and Accountability Act)

**Scope**: Organizations handling protected health information (PHI) in US: healthcare providers, insurance companies, cloud providers for healthcare.

### Technical Safeguards (Security Rule)

| Area | Control |
|------|---------|
| **Access Control** | Unique user ID, authentication, authorization, audit controls |
| **Audit Controls** | PHI access logging, log review, log retention ≥6 years |
| **Integrity Controls** | Mechanisms to verify PHI not altered/destroyed |
| **Data Encryption** | Encryption at rest and in transit; encryption strength commensurate with risk |
| **Transmission Security** | VPN, TLS for all data transmissions |

**Business Associates**: If organization uses contractors (cloud provider, payroll processor, analytics), BA must have business associate agreement (BAA) requiring HIPAA compliance.

**Breach Notification**: If PHI is compromised, notify affected individuals within 60 days, report to HHS, notify media (if >500 affected).

## FedRAMP (Federal Risk and Authorization Management Program)

**Scope**: Cloud services used by US federal agencies.

**Three authorization levels**:
- **Low**: Unclassified data, low impact if compromised
- **Moderate**: Sensitive unclassified data, medium impact if compromised
- **High**: Sensitive unclassified data, high impact if compromised (close to classified)

**Controls**: 325+ NIST SP 800-53 security controls (assessment, identification and authentication, incident response, system monitoring, etc.).

**Continuous monitoring**: Annual assessment plus continuous monitoring via ISCM (Information Security Continuous Monitoring).

**Marketplace**: FedRAMP-authorized cloud services listed in FedRAMP marketplace. Agencies can use authorized services; other services require separate authorization.

## Compliance Automation and Audit Preparation

**Challenges**: Manual audit processes are expensive, error-prone. Emerging automation patterns:

**Infrastructure as Code (IaC) scanning**: Validate cloud configs comply with regulatory requirements (e.g., S3 buckets not public, encryption enabled) before deployment.

**Continuous compliance monitoring**: Systems continuously verify controls are operating (e.g., Kubernetes RBAC configurations, network policies, log retention).

**Security information & event management (SIEM)**: Aggregate logs across systems to satisfy audit logging requirements and detect violations.

**Secrets rotation automation**: Automatically rotate database passwords, API keys, cloud credentials to ensure compliance with rotation schedules.

**Vulnerability scanning & patch automation**: Continuous scanning identifies vulnerable software versions; automation triggers patching/remediation.

**Access reviews automation**: Quarterly user access attestation automated via workflow tools; detects over-provisioned access.

**Audit trail export**: On-demand reports demonstrating control operation over compliance period.

## Compliance Trade-offs

**False sense of security**: Compliance ≠ Security. Meeting PCI DSS requirements doesn't prevent determined attackers; it establishes minimum baseline. Compliance is a floor, not a ceiling.

**Cost**: Audit fees ($10k-$500k+ depending on scope), control implementation, staffing.

**Operational friction**: Controls add process overhead (approval workflows, documentation, access requests).

**Liability**: SOC 2/ISO 27001 audits and penetration tests create documented vulnerabilities. Documentation discovered in litigation can increase liability.

**Scope creep**: Meeting one framework (PCI DSS) often requires controls from others (ISO 27001). Multiple overlapping requirements across frameworks.

See also: security-devsecops.md, security-best-practices.md, security-pentest-methodology.md, security-identity.md