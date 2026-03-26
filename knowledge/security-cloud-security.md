# Cloud Security — Shared Responsibility, IAM, VPC Design, Encryption & Posture Management

## Overview

Cloud security spans shared infrastructure layers (compute, storage, networking) and application-level protections. Unlike on-premise infrastructure, public clouds (AWS, Azure, GCP) handle physical security, hardware hardening, and basic network isolation; clients are responsible for access control, data encryption, application patching, and configuration. Understanding this shared responsibility model is the foundation for cloud security architecture.

## Shared Responsibility Model

The cloud provider and customer jointly secure the system. Responsibilities split by layer:

| Layer | Provider Responsibility | Customer Responsibility |
|-------|------------------------|------------------------|
| **Physical infrastructure** | Data center security, physical access, power, cooling | N/A |
| **Network infrastructure** | DDoS protection, segment isolation, backbone security | VPC design, security groups, NACLs |
| **Host & hypervisor** | Hardware patches, hypervisor hardening, firmware | VM/container patching, OS hardening |
| **OS & kernel** | Base OS images, kernel patches | Custom OS config, userland patching |
| **Application & data** | N/A | Code security, data encryption, access control |
| **Identity & access** | IAM infrastructure | Configuration, least privilege, MFA |
| **Compliance & audit** | Infrastructure logs, service-level audit | Application logs, retention policies |

**Key insight**: A misconfigured S3 bucket (publicly readable) is the customer's responsibility, not the provider's fault. Provider supplies tools (bucket policies, ACLs, encryption); customer must use them correctly.

## Identity & Access Management (IAM)

IAM determines who can do what on which resources. Cloud IAM is role-based (RBAC) or attribute-based (ABAC).

### Least Privilege Principle

Grant only the minimum permissions necessary for a role to function. Example: a Lambda function that reads from one S3 bucket should have a policy allowing only `s3:GetObject` on that specific bucket, not wildcard `*` permissions.

**Enforcement mechanisms**:
- **Explicit allow** — Policies whitelist permissions; anything not allowed is denied
- **Explicit deny** — Override allows (rarely used, but canonical security boundary)
- **Resource-level granularity** — Apply policies to specific resources (VPCs, S3 buckets) not entire accounts
- **Time-bound credentials** — Temporary keys with expiration (reduce blast radius of leaked credentials)

### Types of Identities

**Users** — Correlate to individual humans or service accounts. Long-lived credentials (passwords, API keys) pose risks if exposed; rotate regularly.

**Service principals** — Represent applications or CI/CD pipelines. Assigned specific permissions for that workload. Each service principal has its own identity.

**Roles** — Collections of permissions assigned to users or services. Easier to manage than individual permissions.

**Federation** — Delegate authentication to external identity providers (Active Directory, Okta, SAML). Reduces credential management burden; centralizes MFA.

### Credential Management Anti-Patterns

**Hardcoded credentials** — AWS keys, passwords in source code exposed in version control. Always use secrets management (AWS Secrets Manager, HashiCorp Vault, Azure Key Vault).

**Overly broad credentials** — Service account with root permissions. Violates least privilege; leaked credential can destroy infrastructure.

**Missing MFA** — Root account accessible with password alone. Attackers brute-force or phish credentials. MFA (TOTP, hardware key) significantly raises attack cost.

**Credential sprawl** — Teams using shared passwords or hardcoded keys. Audit trail breaks; can't identify who made harmful changes.

## Virtual Private Cloud (VPC) Design

VPCs provide network isolation and fine-grained traffic control.

### Segmentation and Subnets

- **Public subnets** — Resources with internet-facing addresses (load balancers, NAT gateways, bastion hosts). Higher risk profile.
- **Private subnets** — Databases, internal services, no direct internet access. Traffic must transit through gateways (NAT, jumpbox).
- **Multiple AZs** — Distribute subnets across availability zones for redundancy and compliance (isolate blast radius of AZ failure).

### Network Access Control

**Security Groups** — Stateful firewall at instance level. Default-deny: only allow explicitly listed ports. Example: web tier allows inbound 80/443 (HTTP/HTTPS), outbound 3306 (MySQL to database tier).

**Network ACLs (NACLs)** — Stateless firewall at subnet level. Both inbound and outbound rules required; ephemeral port ranges must be open for responses. Less commonly tuned than security groups.

**Flow logs** — Capture metadata about VPC traffic (source, destination, port, accept/reject). Useful for auditing network activity and debugging firewall rules.

### Restricted Access Patterns

**Bastion hosts (jump servers)** — Single entry point to private infrastructure. All connections to private resources tunnel through bastion. Centralized audit trail, easier to harden.

**VPN and Site-to-Site connectivity** — Encrypted tunnels for connecting on-premise infrastructure to cloud VPC. Avoid exposing databases directly to internet.

**Private endpoints** — Services (S3, DynamoDB, Secrets Manager) accessible via private IP without internet gateway. Reduces surface area.

## Data Encryption

Encryption protects data across its lifecycle: at rest (stored), in transit (moving), in use (processed).

### Encryption at Rest

**Symmetric encryption** — AES-256 (AWS S3, EBS) is standard. Key management is the hard part: how to store the key so it's not accessible to attackers?

**Key management services** — AWS KMS, Azure Key Vault, GCP Cloud KMS handle key lifecycle: generation, rotation, access control, audit logs. Applications don't see keys; they request encryption/decryption operations.

**Customer-managed vs. provider-managed keys** — Provider-managed keys are simpler (no operational burden) but limit audit/compliance control. Customer-managed keys are complex but enable stricter policies (e.g., rotate keys quarterly).

**Database encryption** — Most cloud databases support transparent encryption (TDE): data encrypted on disk, decrypted on access. Protects against stolen disks but not compromised database connections.

**Default encryption** — Cloud resources should default to encryption; explicitly disable only when needed.

### Encryption in Transit

**TLS 1.2+** — All service-to-service communication and client-to-service should use TLS. Verify certificates (avoid self-signed in production).

**VPN and IPsec** — For on-premise to cloud communication, use VPN or site-to-site IPsec tunnels; encrypts data crossing the internet.

**VPC peering** — Connecting VPCs within the same cloud provider; data stays on provider's network (not internet). Generally safe but consider encryption if crossing trust boundaries.

### Encryption in Use

Harder to achieve; data must be decrypted for processing.

**Confidential computing** — Hardware-backed enclaves (Intel SGX, AMD SEV, AWS Graviton3 Confidential Instances) encrypt data in memory and in-CPU caches. Attacker can't read even with kernel access.

**Homomorphic encryption** — Compute on encrypted data without decryption. Theoretically powerful but computationally expensive (impractical for most workloads).

**Per-record encryption** — Encrypt individual database records; keys associated with ownership (users, applications). Only the record owner can decrypt. Trade-off: can't query encrypted columns, no joins across encrypted data.

## Audit, Monitoring, and Threat Detection

### CloudTrail and Audit Logs

**CloudTrail (AWS)** — Records API calls to AWS services: who called, what action, when, from where, with what result. Immutable audit trail (write-once storage). Enables compliance reporting and forensics.

**Blob auditing (Azure)**, **Admin Activity logs (GCP)** — Equivalents in Azure and GCP. All provide API-level audit trails.

**Query and analysis** — Dump logs to S3 (Athena queries) or central SIEM to find anomalies (mass deletion, permission changes, failed auth attempts).

### GuardDuty and Threat Detection

**GuardDuty (AWS)** — Managed threat detection using ML. Analyzes CloudTrail, DNS logs, VPC Flow Logs to find suspicious patterns:
- Failed logins from unusual geography
- Unusual API calls
- Known malicious IP addresses accessing resources
- Cryptocurrency mining activity (high compute patterns)

**Azure Defender, GCP Security Command Center** — Equivalents offering threat intelligence, vulnerabilities, misconfiguration warnings, advanced threat detection.

### Security Information & Event Management (SIEM)

Aggregate logs from cloud services, applications, network devices; correlate events to detect attacks and compliance violations. Examples: Splunk, Datadog, Chronicle, Sumo Logic.

## Cloud Security Posture Management (CSPM)

CSPM continuously scans cloud configurations against policies to find misconfigurations and compliance gaps.

### Common Misconfigurations

**Public storage** — S3 buckets or blobs with public read/write access. Trivial to find and exploit; leaked credentials, customer data exposure.

**Default ports open** — Security groups allowing SSH (22), RDP (3389), or databases (3306 MySQL, 5432 PostgreSQL) from `0.0.0.0/0` (anywhere). Should restrict to known jump boxes or internal CIDR blocks.

**Missing encryption** — Databases or storage buckets without encryption enabled.

**Unused credentials** — IAM users with inactive access keys. Delete or disable; reduces blast radius of old key compromise.

**Overly broad roles** — Users or services with `admin` or `*` permissions.

**Logging disabled** — CloudTrail, VPC Flow Logs, or application logging not enabled. Audit trail missing if incident occurs.

**No MFA on root** — MFA not enforced for root account or high-privilege users.

### CSPM Tools

**AWS Config** — Continuously records resource configuration; ruleset to validate compliance (e.g., "all S3 buckets must have versioning enabled"). Integrates with AWS Security Hub for aggregated findings.

**Azure Policy** — Similar to AWS Config; define policies (JSON-based), evaluate resources, report/remediate violations.

**Prisma Cloud, Wiz, Orca Security** — Third-party CSPM platforms. Features: cross-cloud visibility, risk scoring, automated remediation, compliance frameworks (CIS, PCI-DSS, SOC2).

## Cloud Workload Protection Platforms (CWPP)

CWPP focuses on runtime protection: detecting and preventing attacks on running containers, VMs, and serverless functions.

### Capabilities

**Runtime threat detection** — Detect process injection, privilege escalation, suspicious system calls. Example: container spawning a shell when it shouldn't.

**Vulnerability scanning** — Scan container images (and running containers) for known CVEs.

**Compliance enforcement** — Prevent containers from running without required security patches or using banned libraries.

**Behavioral modeling** — Learn normal process execution; alert on deviations.

**Incident response** — Kill/isolate container, capture forensic data, alert security team.

## Compliance and Cloud-Native Security

### Frameworks

**Shared Responsibility Framework** — CSPs document which controls they provide; customers document their controls. Reduces ambiguity when certifying compliance.

**CIS Cloud Security Benchmarks** — Vendor-neutral best practices (e.g., CIS AWS Foundations Benchmark). Scored by CSPM tools.

**Well-Architected Framework** — AWS (also Azure, GCP counterparts). Provides design principles, pillars (operational excellence, security, reliability, performance, cost), and checklist for evaluating architectures.

**Data Residency & Sovereignty** — Some regulations (GDPR, HIPAA) require data stays in specific regions or countries. VPCs and encryption keys must be configured accordingly.

## Anti-Patterns and Pitfalls

**Single availability zone** — All resources in one AZ; single data center failure causes outage. Always span multiple AZs.

**Overpermissioned service accounts** — Service principal with wildcard permissions. If compromised, attacker has full account access.

**Credential reuse across environments** — DEV credentials work in PROD. Breach of dev system compromises production.

**Disabled encryption** — Legitimate use cases exist (e.g., performance-critical in-memory caches), but default assumption should be: encrypted.

**Unused resources** — Forgotten databases, NAT gateways, or load balancers incur cost and expand attack surface.

**Logs not retained** — CloudTrail disabled after 90 days; forensic evidence gone. Retention should match compliance requirements.

## See Also

- security-best-practices (foundational principles)
- security-devsecops (CI/CD integration)
- infrastructure-dns-security, security-network (complementary network controls)
- cloud-multi-cloud (cross-cloud architecture considerations)