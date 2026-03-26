# GDPR Compliance for Developers

## Overview

The General Data Protection Regulation (GDPR) is EU privacy law governing personal data processing. It applies globally to anyone serving EU residents. For developers, it means embedding privacy into systems: lawful processing, user consent, transparent handling, and data rights. Violations carry fines up to €20M or 4% of annual revenue.

The GDPR shifts responsibility from traditional notice-and-opt-out (e.g., US privacy laws) to consent-first, transparency-by-design, and documented safeguards. Developers implement compliance via architecture (data minimization, encryption), code (consent flows, audit trails), and operations (breach response, DPA agreements).

---

## Lawful Basis for Processing

Processing personal data requires one of six lawful bases (Article 6):

**1. Consent** — User explicitly opts in. Requires:
- Clear, affirmative action (no pre-checked boxes)
- Specific, informed, granular (separate consent for marketing, profiling, etc.)
- Easy withdrawal (must be as simple as consent)
- Documented proof of consent event (timestamp, user ID, version of terms)

*Developer responsibility:* Implement consent UI that captures intent unambiguously. Store consent records with dates. Provide clear revocation paths.

**2. Contract** — Processing necessary to deliver a service the user requested. Example: shipping address for e-commerce.

*Developer responsibility:* Limit processing to what the contract requires. If processing exceeds contract scope, shift to explicit consent.

**3. Legal obligation** — Law requires processing. Example: tax ID for invoicing.

*Developer responsibility:* Document which law and which fields it mandates. Retain only as long as law requires.

**4. Vital interest** — Protect life or health. Example: emergency contact data during crisis.

*Developer responsibility:* Rare; usually only applies during incident response. Document the emergency and when processing ends.

**5. Public task** — Necessary to perform government duty. Example: public agency processing citizen requests.

*Developer responsibility:* Verify the organization's legal authority. Implement access controls; public task doesn't mean public data.

**6. Legitimate interest** — Balancing test: does the organization's interest outweigh user privacy? Example: fraud detection.

*Developer responsibility:* Conduct and document legitimate interest assessments (LIA). Implement data minimization and technical safeguards. Be ready to defend the balance to regulators.

---

## Data Subject Rights (The Big Five)

**Right of Access (Article 15)** — Users can request all data held about them.
- Response deadline: 30 days
- Format: structured, commonly used, portable (JSON, CSV acceptable)
- Scope: all processing, including logs, inferred data, profiles
- *Developer responsibility:* Build data export functionality that gathers data from all systems and stores. Test regularly. Include metadata (source, processing purpose, recipients).

**Right to Erasure ("Right to be forgotten", Article 17)** — Users can demand deletion.
- Exceptions: legal obligation, public interest, freedom of expression, backup retention must not be searchable
- Response deadline: 30 days
- *Developer responsibility:* Implement soft-delete (mark as deleted, exclude from queries) or hard-delete (cryptographic erasure, re-encryption with new keys). Erase from backups or make unrecoverable. Track which dependent systems have copies (audits, caches, analytics).

**Right to Rectification (Article 16)** — Users can correct inaccurate data.
- Deadline: 30 days
- *Developer responsibility:* Provide UI for users to update their own data. Log corrections with before/after values. Notify recipients if data was already shared.

**Right to Restrict Processing (Article 18)** — Users can pause processing while disputing it.
- Marked data can't be stored, profiled, or shared (only kept)
- Response deadline: 30 days
- *Developer responsibility:* Implement a "restricted" flag in the database. Filter restricted records from queries. Handle access requests separately (users can still exercise other rights).

**Right to Data Portability (Article 20)** — Users can transfer their data to another service.
- Format: structured, machine-readable
- Scope: only data the user provided (not inferred data, but inferred data based on explicit consent for processing is portable)
- *Developer responsibility:* Similar to Right of Access. Provide export in common formats. Document API for third-party access if user consents to direct transfer.

**Right to Object (Article 21)** — Users can opt out of specific processing.
- Legitimate interest & direct marketing always can be objected to
- Profiling and automated decisions subject to special rules
- *Developer responsibility:* Provide opt-out mechanisms. Honor promptly. Log objection with timestamp.

---

## Consent Management

**Consent vs. Notice:** Notice ≠ consent. Users must actively choose; defaults must be "no processing."

**Consent attributes to track:**
- Timestamp (when consent was given)
- Version (which privacy policy/terms was in effect)
- Granularity (which purposes: email marketing, analytics, profiling, etc.)
- Medium (web form, app, sales call)
- Withdrawal timestamp (if applicable)
- Underlying legal basis determined by consent (if any)

**Implementation patterns:**
- **Consent banner**: Show at first visit; separate toggles for each purpose; must include link to full privacy policy; cookie consent is separate (ePrivacy Directive)
- **Segmented consent**: Marketing ≠ analytics ≠ profiling. Allow users to consent to one without others
- **Persistent records**: Store consent events in database, not just cookies (cookies can be deleted)
- **Audit trail**: Log all consent events; be ready to prove "yes, user consented on 2025-03-01 at 14:32 UTC to marketing emails"

**Cookie consent specifics:**
- Essential cookies (session, security, user preference): no consent needed
- Non-essential (analytics, tracking, advertising): require prior explicit consent
- Revocation must be as easy as consent: one-click unsubscribe, cookie toggle in settings

---

## Privacy by Design & DPIA

**Privacy by Design (Article 25)** — Embed privacy into systems from inception, not as afterthought.

Technical measures developers implement:
- **Data minimization**: Collect only what's necessary. Delete what's no longer needed (retention schedules).
- **Encryption**: At rest (AES-256), in transit (TLS 1.2+). Exclude encryption keys from backups or use key derivation.
- **Access controls**: Role-based access; principle of least privilege. Log access to sensitive data.
- **Pseudonymization**: Separate identifiable data from analytical data. Hash or tokenize personally identifiable information (PII) when possible.
- **Integrity controls**: Checksums, signed audit logs, tamper detection.

**Data Protection Impact Assessment (DPIA, Article 35)** — Risk assessment required before "high-risk processing."

When DPIA is required:
- Large-scale processing (>50K subjects or 25K+ per month)
- Automated decision-making with legal effect
- Systematic monitoring (behavioral tracking, profiling)
- Processing of special categories (health, biometrics, genetics, race, religion, etc.)
- Use of new technologies
- Mixing data sources

DPIA contents:
- Description of processing (what, why, recipients, storage)
- Necessity and proportionality assessment (why this data is essential)
- Risk analysis (likelihood & severity of harm: data breach, discrimination, manipulation)
- Risk mitigation: technical safeguards proposed
- Third-party impact (consultations with DPO, legal)
- Residual risk assessment

---

## Data Processing Agreements (DPA)

If your system uses third-party processors (cloud, analytics, email providers, payment gateways):

**You are the "controller" (decision-maker about processing). The vendor is the "processor" (carries out your instructions).**

Your processor contract must include:
- Processing instructions in writing
- Confidentiality obligations for processor staff
- Sub-processor approval mechanism (e.g., you can veto).
you can terminate if subcontractors change
- Assistance with data subject rights (processor must help with exports, deletions)
- Assistance with audits and compliance (access to records, certifications)
- Data deletion or return upon contract end
- Technical & organizational measures (encryption, access logs, incident response, staff training)
- International transfer clauses (if data leaves EU, additional safeguards required under SCCs or adequacy decisions)

*Developer responsibility:* Audit vendor contracts before integrating. Use GDPR data processing addendum (DPA) templates (Slack, Google, AWS provide these). Document all processors in your compliance tooling.

---

## Breach Notification

Personal data breach: unauthorized access, alteration, or loss of data.

**Notification timeline:**
- 30 days to notify data subjects (unless risk is low)
- Immediate to data protection authority (if risk is high)
- Immediate to processor, if processor discovered breach

**Contents of notification:**
- Nature of breach
- Likely consequences
- Measures taken to mitigate
- Your DPO contact
- Avoid creating panic; be factual

**Low-risk scenarios (no notification required):**
- Data was encrypted and keys remained secure
- Only metadata was exposed (user IDs), not sensitive content
- Number of affected subjects is negligible

**Developer responsibility:** Implement incident detection (anomaly monitoring, audit logs). Have a breach response playbook: scope, notify leadership, contact legal, prepare forensics, begin notification. Log all breach investigations.

---

## Cookie Consent & ePrivacy

GDPR applies to personal data. Separate **ePrivacy Directive** (US: CCPA cookie rules) governs cookies specifically.

**Cookie categories:**
- **Strictly necessary**: Session, CSRF tokens, user preferences. No consent needed.
- **Performance/Analytics**: Google Analytics, Mixpanel, Sentry errors. Requires consent before setting.
- **Functional**: Remember-me, language selection. Usually can be assumed (but be explicit).
- **Marketing/Advertising**: Retargeting pixels, social media trackers. Explicit consent required.

**Implementation:**
- Don't set non-essential cookies until user consents
- Provide granular toggles in preferences (don't require one "accept all")
- Show cookie banner before setting any tracking
- Honor geo-location: EU visitors get opt-in; US visitors get opt-out
- Privacy policy must list all scripts and cookies, their purpose, retention

---

## Data Minimization & Pseudonymization

**Data minimization (Article 5):** Collect / retain only what's necessary.

Practical steps:
- Delete logs after 90 days (or document why longer is necessary)
- Don't collect full names if first names suffice
- Don't store hashed passwords AND original passwords
- Don't archive user sessions; keep only active sessions
- Delete failed login attempts after 24 hours
- Implement user data deletion workflows (e.g., annual sweep of inactive accounts)

**Pseudonymization (Article 4(11)):** Process data without linking to individual identity.

Examples:
- Analytics: hash user IDs before sending to third parties; link back only with a secure lookup table
- A/B testing: assign users a UUID; map UUID → identity only when needed
- ML training: train models on pseudonymized records; re-identify only for validation
- Logs: remove PII, keep pseudonyms for debugging

*Pseudonymized data still falls under GDPR if identifiability is possible*. It's a technical safeguard, not an exemption.

---

## Regional Variations

**UK (post-Brexit):** UK GDPR is nearly identical but enforced by ICO, not EU DPA. Transfers to EU now require SCCs.

**California (CCPA):** Weaker than GDPR (fewer rights, narrower compliance), but overlapping with GDPR for global companies. Right to know, delete, opt-out; no right to restrict or object.

**Brazil (LGPD):** Similar to GDPR; adds consent registry requirement.

**Canada (PIPEDA):** Notice-based; federal law governs; weaker than GDPR.

For global apps: implement to GDPR standard. It's the strictest; other regions' requirements typically fall within it.

---

## Developer Checklist

- [ ] Privacy policy in plain language; update when processing changes
- [ ] Consent UI: granular toggles, easy revocation, persistent records
- [ ] Data minimization: retention schedules, automated deletion
- [ ] Encryption: TLS in transit, encryption at rest
- [ ] Access logs: who accessed what, when
- [ ] Data export: user can download their data in days not weeks
- [ ] Deletion: user can request erasure; process takes 30 days max
- [ ] Third-party contracts: DPA addendum in place, signed
- [ ] Incident response plan: breach detection, notification templates, forensics
- [ ] DPIA completed (if high-risk processing)
- [ ] Regular audits: quarterly review of data flows, processors, retention