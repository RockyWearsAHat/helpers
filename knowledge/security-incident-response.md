# Incident Response — NIST Framework, Investigation & Recovery

## Overview

Security incidents disrupt confidentiality, integrity, or availability of information systems. Incident response governs how organizations detect, contain, investigate, and recover from attacks. The NIST Framework (SP 800-61) provides the foundational lifecycle; organizations add legal, communication, and forensic protocols on top.

## NIST Incident Response Lifecycle

### Phase 1: Preparation

Readiness before incidents occur.

**Activities:**
- Establish incident response team (incident commander, analysts, forensics, communications)
- Define communication plan: internal escalation, executive notification, external law enforcement/media
- Deploy detection infrastructure (SIEM, intrusion detection systems, endpoint monitoring)
- Conduct tabletop exercises and incident response drills
- Maintain playbooks for common incident types
- Inventory critical assets and backup systems
- Retain logs for forensic analysis (minimum 90 days; longer for compliance-heavy industries)

**Key output:** Incident response plan, team roles, detection baselines, communication templates.

### Phase 2: Detection & Analysis

Identify that an incident has occurred and characterize it.

**Detection triggers:**
- Alert from SIEM or security monitoring tool
- Report from security researcher or bug bounty program
- User complaint ("I can't log in," "my data looks wrong")
- Third-party notification (payment processor, cloud provider, law enforcement)
- Suspicious log patterns (failed logins, privilege escalation, unusual data access)

**Analysis tasks:**
- Determine incident type (data breach, ransomware, DDoS, insider threat, supply-chain compromise, etc.)
- Identify affected systems and scope (how many users, how much data, which applications)
- Estimate time of first compromise (initial access time)
- Collect indicators of compromise (IOCs): file hashes, IP addresses, domain names, email headers
- Assess impact: is the system still compromised, is data exfiltrating, are other systems affected

**Output:** Incident classification, scope, preliminary timeline, list of IOCs.

### Phase 3: Containment, Eradication & Recovery

Stop the bleeding, remove the attacker, restore systems.

**Short-term containment:** Isolate affected systems (disconnect from network, disable accounts, revoke sessions) to prevent spread while preserving evidence.

**Long-term containment:** Patch vulnerabilities, deploy compensating controls, harden configurations.

**Eradication:** Remove attacker access and backdoors.

- Change all credentials (passwords, API keys, SSH keys) for affected accounts
- Remove attacker persistence mechanisms (cron jobs, scheduled tasks, reverse shells, web shells)
- Patch the vulnerability that allowed initial access
- Search for and remove similar backdoors across the environment

**Recovery:** Restore systems from clean backups or rebuild.

- Verify backups are not themselves compromised (ideally, air-gapped backups older than initial compromise)
- Restore systems incrementally, monitoring for re-compromise
- Validate system functionality
- Re-enable logging and monitoring

**Timeline risk:** Attackers sometimes maintain persistence even after initial eradication; monitor for re-compromise for weeks post-incident.

### Phase 4: Post-Incident Activity (Lessons Learned)

Conduct root-cause analysis and improve controls.

**Questions to answer:**
- How did the attacker gain initial access? (vulnerable application, phishing, weak credentials, supply-chain)
- Why wasn't the attack detected sooner? (gap in logging, alerting threshold set too high, lack of correlation rules)
- What time elapsed between initial compromise and detection? (mean time to detect — MTTD; shorter is better)
- What controls failed? (segmentation, access control, password policy, multi-factor authentication)
- How did the attacker move laterally?

**Actions:**
- Document lessons learned (root cause, detection gap, control failure)
- Create tickets for remediation (patch, deploy monitoring, update policy)
- Update incident playbooks with new threats/patterns observed
- Share IOCs with security team and threat intelligence partners (when permitted by law enforcement or data sensitivity)
- Report to incident response plan stakeholders (executives, board, customers)

## Severity Classification

Incident priority determines response time and resource allocation.

**Common classification (Low → Critical depends on organizational risk appetite):**

- **Critical:** Data breach affecting 10,000+ records, payment system outage, attacker in production databases, ransomware spreading
  - Response time: < 15 minutes
  - Escalation: Executive, legal, board

- **High:** Unauthorized access to sensitive systems, active attacker in the network, 1,000–10,000 records affected
  - Response time: < 1 hour
  - Escalation: CISO, legal

- **Medium:** Compromised employee account, malware on workstation, data exfiltration suspected but unconfirmed
  - Response time: < 4 hours
  - Escalation: Incident response team lead

- **Low:** Phishing attempt blocked, vulnerability discovered in non-critical system, unsuccessful login probe
  - Response time: < 1 business day
  - Escalation: Security operations center (SOC)

**Factors influencing severity:**
- Data sensitivity (PII, financial, trade secrets vs. public marketing materials)
- Scope (1 system vs. company-wide)
- Impact (CIA: confidentiality, integrity, availability)
- Affected user population (external customers vs. internal)
- Regulatory/compliance implications (HIPAA, PCI-DSS, GDPR)

## Detection & Investigation Tools

### SIEM (Security Information & Event Management)

Centralizes log collection, indexing, and correlation from disparate sources (firewalls, endpoints, servers, applications).

**Capabilities:**
- Aggregates events into searchable database
- Runs correlation rules (e.g., "alert if 10 failed logins from same IP in 5 minutes")
- Baseline normal behavior; alert on deviations

**Limitations:** SIEM is reactive; requires humans to investigate alerts. High false-positive rate if not tuned.

### SOAR (Security Orchestration, Automation & Response)

Runs playbooks that automatically respond to alerts without human intervention.

**Workflow:**
- SOAR receives alert from SIEM
- Runs playbook: isolate system, block IP, revoke credentials, collect logs, notify team
- Escalates to analyst if playbook is unclear or manual action needed

**Result:** Faster response, reduced human toil, consistent execution.

### Forensic Collection

Capture evidence from compromised systems for analysis and legal proceedings.

**Artifacts:**
- File system (deleted files, timestamps, permissions)
- Memory (running processes, network connections, keystroke logs)
- Logs (application logs, OS logs, firewall logs, DNS queries)
- Email headers (sender IP, message routes, attachment signatures)
- Network traffic (packet captures; protocol analysis)

**Tools:**
- Memory forensics: Volatility, WinDbg
- Disk forensics: FTK, EnCase, dd + analysis tools
- Log analysis: grep, Splunk, ELK Stack

**Challenges:**
- Time-sensitive; memory volatile (lost if system powered off)
- Legal admissibility requires chain of custody
- Large data volumes (terabytes of logs)
- Encryption and obfuscation complicate analysis

## Chain of Custody

Legally mandated documentation of evidence handling to ensure admissibility in court.

**Record every touchpoint:**
- Who collected the evidence (name, role, timestamp)
- What evidence (device, file, hash)
- When collected (date, time)
- Where stored (secure facility, locked room)
- Why accessed (analysis, legal review)
- How transferred (documented handoff, cryptographic verification)

**Example chain of custody form:**

```
Evidence: Windows Server 2022 (192.168.1.100) hard drive
Collected: 2026-03-20 14:30 UTC by Analyst A
Hash (SHA-256): abc123...
Storage: Locked evidence locker, room 301
Transfer 1: Handed to Forensics Specialist B on 2026-03-20 15:45
Transfer 2: Sent to Law Enforcement on 2026-03-21 10:00 (signed receipt)
```

**Failure consequence:** Evidence ruled inadmissible; criminal charges dismissed; civil lawsuit lost.

## Legal Obligations & Breach Notification

**Varies by jurisdiction:**

- **GDPR (Europe):** 72-hour notification to data protection authority; individual notification if personal data of EU residents affected. Fines up to 4% of global revenue.

- **CCPA (California):** Notification to California residents and attorney general if personal information breached. Consumers have right to opt-out of data sales.

- **HIPAA (US Healthcare):** Notification to individuals and HHS if protected health information (PHI) breached; fines $100–$50,000 per record.

- **PCI-DSS (Payment Card Industry):** Notification to acquiring bank and cardholder if credit card data breached; fines $100,000+ per month; potential card network suspension.

**Common obligations:**
- Timeline (24–72 hours typical)
- Content (type of data, likely uses of data by attacker, recommended protective actions)
- Affected user notification (direct email, credit monitoring offer)
- Regulatory notification (government agency)

**Strategic impact:** Breach notification triggers lawsuits, customer churn, regulatory investigation, reputational damage. Time spent in breach (undetected) directly correlates to liability exposure.

## Communication Plan

**Escalation ladder:**
1. SOC detects alert; initial assessment by on-call analyst
2. Analyst escalates to incident commander if severity ≥ Medium
3. Incident commander notifies CISO, legal, public relations
4. CISO/CEO notify board and customers (if applicable)

**Key audiences & messages:**

- **Internal (employees):** "We detected suspicious activity; we're investigating and will restore systems."
- **Customers:** "Your data may have been affected. Here's what to do (password reset, credit monitoring offer)."
- **Board:** "Incident cost estimate, impact on revenue, remediation timeline, insurance coverage."
- **Law enforcement:** (if criminal activity involved) Cooperation on investigation, evidence preservation.
- **Media:** Transparent, factual statement; no speculation; acknowledge impact.

**Avoid:** Admitting fault prematurely (legal liability); contradicting previous statements (damages credibility); overpromising recovery timeline.

## Post-Incident Review & Lessons Learned

Conducted 1–2 weeks after incident declared resolved (not while still fighting attacker).

**Attendees:** Incident commander, responders (ops, security, forensics), leadership, external advisors (law enforcement, incident response firm if engaged).

**Review agenda:**
1. Timeline: When did each event occur? (initial access, detection, containment, eradication, recovery)
2. Root cause: How did attacker gain access? (vulnerability, phishing, insider, supply-chain)
3. Detection gap: Why wasn't this detected earlier? Technical gap (no logging) or threshold gap (alert rule too loose)?
4. Containment effectiveness: Did containment actions stop the attack? Why/why not?
5. Evidence: What forensic artifacts reveal attacker actions?

**Output:** Written report with root cause, remediation tickets, updated playbooks.

## Incident Response Resources

### External Support

- **Incident response firms:** Contract forensics/cyber-warfare specialists for large breaches (e.g., Mandiant, CrowdStrike)
- **Law enforcement:** FBI, secret service available for certain threats; can provide threat intelligence
- **Insurance:** Cyber liability insurance may cover investigation, legal, and notification costs

### Metrics & KPIs

Track to improve over time:

- **MTTD (Mean Time to Detect):** How long between initial compromise and detection? Target: hours, not days.
- **MTTR (Mean Time to Respond):** How long between detection and containment? Target: < 4 hours for Critical incidents.
- **False positive rate:** What percentage of alerts are not real incidents? Low = fewer distractions; high = tuning needed.
- **Incident count by type:** Are certain attack vectors trending up? (e.g., ransomware prevalent last quarter)

## See Also

- SRE Incident Management (operations-focused counterpart)
- Zero Trust Architecture
- Security secrets management
- Network segmentation (prevents lateral movement)