# Privacy Engineering — Principles, Trade-Offs, and Technical Implementation

## Overview

Privacy engineering is the discipline of building systems where privacy is a foundational property, not an afterthought. It differs from privacy compliance (following rules) by embedding privacy into architecture, data flows, and decision-making from inception.

## Privacy by Design Principles

Privacy by design (PbD), formalized by Ann Cavoukian, emphasizes seven core principles:

1. **Proactive, not reactive:** Address privacy before problems occur, not after breaches
2. **Privacy as the default:** Users must not take additional steps to achieve privacy
3. **Privacy by design:** Build privacy into systems and processes, not as an add-on
4. **Full functionality:** Privacy and functionality are not mutually exclusive
5. **Security from end to end:** Protect data throughout its lifecycle
6. **Transparency:** Be clear about data practices and decision-making
7. **User-centric control:** Respect user autonomy and enable user control

These principles are aspirational rather than prescriptive; friction exists between them (e.g., transparency vs. minimization). Implementation requires engineering judgment.

## Data Minimization

Data minimization—collecting only data necessary for a stated purpose—is the most effective privacy control. Data you don't collect cannot be breached or misused.

Example trade-offs:
- Collecting exact birth dates enables precise age verification and personalization; collecting only birth year reduces precision but increases privacy
- Collecting full postal address enables geographic targeting; collecting only postal code reduces precision
- Collecting device identifiers enables user tracking; using session IDs (ephemeral) reduces tracking capability

Organizations often resist minimization due to "future use" concerns (we might want this data later). Privacy-first design inverts this: data is collected only when needed, and new use cases require explicit data collection.

## Anonymization vs. Pseudonymization

These terms are often conflated, but legally and technically they differ:

**Anonymization** irreversibly removes the ability to identify individuals. Truly anonymized data is not personal data under GDPR and CCPA; organizations can process it with minimal restrictions.

**Pseudonymization** (or deidentification) replaces direct identifiers (name, email) with codes or tokens. The original identity can be recovered if you possess a key. Under GDPR, pseudonymized data is still personal data; it requires the same protections.

The practical problem: True anonymization is rare. Research repeatedly shows that seemingly anonymized data can be re-identified by linking with other datasets. The Netflix dataset was "anonymized" until researchers linked it with IMDb reviews. The Massachusetts Group Insurance Commission dataset was re-identified by joining with voter registration.

**Regulatory interpretation:** GDPR Recital 26 requires that anonymization must withstand not only current technology but also "foreseeable future developments." This legal standard is hard to meet in practice, especially as ML/AI capabilities grow.

**Conservative approach:** Treat data as pseudonymized unless proven truly anonymous. Apply all data protection controls to pseudonymized data.

## Anonymization Techniques

### k-Anonymity

k-anonymity ensures that each record is indistinguishable from at least k-1 other records on quasi-identifiers (attributes that could reveal identity, like ZIP code + birth date + gender).

Example: If k=5 and the quasi-identifiers are {ZIP, gender, age}, then the dataset guarantees at least 5 records share each combination of these values.

Implementations use generalization (broaden age to age ranges), local recoding (suppress specific values), and suppression (delete rows).

**Trade-off:** Achieving k-anonymity distorts data. A k=5 dataset might generalize exact ages into 10-year ranges, reducing analytical utility. Research shows k-anonymity often produces over-anonymization under the "journalist" re-identification scenario (arbitrary re-identification) compared to the "prosecutor" scenario (targeting a specific individual).

### l-Diversity

l-diversity extends k-anonymity by requiring that within each quasi-identifier equivalence class, there are at least l distinct values for sensitive attributes (e.g., medical diagnoses).

Example: A k=5, l=3 dataset guarantees 5 records per quasi-identifier combination AND at least 3 distinct diagnoses per group.

Prevents homogeneity attacks where an attacker knows the quasi-identifiers and can infer the sensitive attribute because all k records have the same diagnosis.

### Differential Privacy

Differential privacy provides a formal, provable privacy guarantee. (See separate note on differential privacy.) Unlike k-anonymity (which assumes non-linkability), differential privacy makes no assumptions about auxiliary data; it is designed to resist any re-identification attack regardless of the attacker's side information.

## Consent Management and Legal Requirements

### GDPR (EU)

GDPR requires lawful basis for processing personal data. Key concepts:

- **Affirmative, informed consent**: Users must explicitly opt-in (checking a pre-checked box doesn't count). Consent must be freely given, specific, and informed.
- **Withdrawal**: Users can withdraw consent at any time; you must stop processing immediately
- **Scope limitation**: Data collected for consent X cannot be processed for purpose Y without new consent
- **Data subject rights**: Users have the right to access, correct, delete (right to be forgotten), and port their data

From an engineering perspective:
- Store consent records with timestamp, scope, and version of privacy document
- Implement deletion pipelines: when a user requests deletion, remove their data from all systems (backup, archives, analytics datasets)
- Log all consent changes and withdrawals for compliance auditing

### CCPA (California)

CCPA grants residents control over personal information collected by businesses. Key requirements:

- **Disclosure**: Businesses must inform consumers what data is collected, how it's used, and with whom it's shared
- **Opt-out**: Consumers can opt-out of data sales and cross-context behavioral advertising (no affirmative opt-in required for most uses, unlike GDPR)
- **Right to delete**: Businesses must delete personal info upon request, except where retention is required by law
- **Non-discrimination**: Businesses cannot discriminate against users who exercise CCPA rights (no price increases or reduced service)

Unlike GDPR, CCPA distinguishes between "personal information" (broadly) and "sensitive personal information" (health, exact geolocation, genetics). Sensitive data requires explicit opt-in even under CCPA's looser baseline.

## Privacy-Preserving Technologies

### Homomorphic Encryption

Homomorphic encryption allows computation on encrypted data without decryption. A server can process encrypted data and return encrypted results; only the client who holds the decryption key can recover the plaintext result.

Example: A bank encrypts customer data, an analytics company runs ML models on encrypted data producing encrypted predictions, and the bank decrypts results.

Drawback: Computational overhead is high (1000x-1000000x slower than unencrypted computation). Practical only for specific use cases where the overhead is acceptable.

### Secure Multi-Party Computation (SMPC)

Multiple parties collectively compute a function without any party seeing the other's input. Example: Two hospitals compute the intersection of patient populations without sharing individual records.

Overhead is high; deployment is limited to specialized scenarios.

### Zero-Knowledge Proofs

A Zero-Knowledge Proof allows one party to prove a statement is true without revealing the statement's contents. Example: Proving you are over 18 without revealing your birth date.

Emerging applications in blockchain and authentication; less common in conventional systems.

## Privacy-Preserving Analytics

Standard analytics directly query personal data. Privacy-preserving alternatives include:

- **Aggregated statistics**: Report only aggregates (e.g., average age) rather than individual records
- **Differential privacy**: Add calibrated noise to query results (see separate note)
- **Federated analytics**: Compute aggregates locally and combine results centrally, never centralizing raw data
- **Synthetic data**: Generate artificial data matching the statistical distribution of sensitive data; share synthetic data for analysis

Synthetic data is attractive but imperfect: if the synthetic data is too similar, it may enable re-identification; if too different, analysis results don't reflect reality.

## Implementation Challenges

1. **Organizational misalignment:** Engineering teams prioritize features and speed; privacy is perceived as friction. Resolve by embedding privacy engineers early and assigning privacy owners per system.

2. **Purpose creep:** Data collected for X is later repurposed for Y. Prevent via strict purpose limitation: systems should not permit use beyond the stated scope without explicit approval.

3. **Data retention:** Legacy systems often retain data indefinitely. Implement automated retention policies (delete after 90 days unless explicitly approved for longer) and audit data holdings.

4. **Debugging and testing:** Production data cannot be used for testing due to privacy risk. Build realistic synthetic or masked data pipelines for testing.

## See Also

- [privacy-differential.md](privacy-differential.md) — Formal differential privacy framework
- [data-engineering-quality.md](data-engineering-quality.md) — Data governance and quality including privacy aspects
- [security-network.md](security-network.md) — Network-level privacy (encryption, Tor, VPNs)
- [architecture-resilience.md](architecture-resilience.md) — Fault tolerance and data availability interact with privacy