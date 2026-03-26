# Security — Logging & Monitoring: SIEM, Detection Rules, Threat Hunting & Alert Correlation

## Overview

Security logging and monitoring collects events from across the infrastructure (auth systems, networks, applications, endpoints) and correlates them to detect attacks, investigate incidents, and maintain compliance. A SIEM (Security Information and Event Management) system centralizes logs, applies detection rules, and alerts on suspicious patterns. This note covers log sources, detection rule frameworks, threat hunting methodology, and incident detection strategies.

---

## Log Sources and Event Types

### Authentication Logs

**Captures:**
- Login attempts (successful and failed)
- Password changes, account lockouts
- Multi-factor authentication events
- API token generation and revocation
- Service account usage

**Example:**
```
timestamp=2026-03-25T10:32:15Z
event_type=authentication.login
user_id=user123
email=user@example.com
ip=203.0.113.42
result=success
mfa_used=true
location=San Francisco
device_id=device-abc123
session_id=sess-xyz789

timestamp=2026-03-25T10:33:42Z
event_type=authentication.login
user_id=unknown
email=attacker@corp.com
ip=198.51.100.99
result=failure
failure_reason=invalid_credentials
attempt_count=15
```

**Value:** Detects brute force, credential stuffing, unauthorized access, account compromise

### Network Logs (Flow Data)

**Captures:**
- Source/destination IP, port, protocol
- Bytes transferred, connection duration
- Encrypted flow (TLS/SSL) metadata (SNI, certificate info)
- Firewall allow/deny decisions

**Example (NetFlow v5):**
```
src_ip=10.1.2.100
dst_ip=198.51.100.10
src_port=49152
dst_port=443
protocol=TCP
duration_ms=5000
bytes_out=1024
bytes_in=65536
```

**Value:** Detects data exfiltration, lateral movement, command & control communication, policy violations

### DNS Logs

**Captures:**
- Query domain, query type (A, MX, CNAME, etc.)
- Resolver IP, client IP
- Response code (NOERROR, NXDOMAIN, etc.)
- Query timestamp

**Example:**
```
timestamp=2026-03-25T10:35:10Z
client_ip=10.1.2.100
query=malware-command-c.xyz
query_type=A
response_code=NOERROR
response_ip=192.0.2.1

timestamp=2026-03-25T10:35:12Z
client_ip=10.1.2.100
query=eicar.com
query_type=A
response_code=NXDOMAIN
```

**Value:** Detects malware callback attempts, domain information gathering, DNS tunneling

### Endpoint (Host) Logs

**Windows / macOS / Linux:**
- Process execution (parent process, command-line arguments, working directory)
- File access and modification
- Network connection initiation
- Security policy changes
- User logon/logoff, privilege escalation
- Service installation/modification

**Example (Windows Event ID 4688 - Process Creation):**
```
event_id=4688
timestamp=2026-03-25T10:37:00Z
hostname=desktop-user42
user=DOMAIN\user123
new_process=c:\windows\system32\cmd.exe
parent_process=c:\program files\app\launcher.exe
command_line=cmd /c cd %temp% && powershell -enc [large_base64_string]
process_id=4256
parent_process_id=2840
privilege_level=user
```

**Value:** Detects malware execution, lateral movement, privilege escalation, suspicious automation

### Application Logs

**Captures:**
- API calls (endpoint, method, parameters, response code)
- Authentication/authorization decisions
- Data access (who accessed what)
- Error and exception events
- Business logic anomalies (unusual transfers, permission changes)

**Example:**
```
timestamp=2026-03-25T10:40:15Z
service=payment-api
event=payment_processed
user_id=user456
amount=10000 USD
recipient_account=external-account-xyz
ip=201.0.113.10
status=success
fraud_score=0.89

timestamp=2026-03-25T10:40:30Z
service=admin-api
event=user_role_change
admin_user=user789
target_user=user456
old_role=customer
new_role=administrator
timestamp=now
```

**Value:** Detects unauthorized data access, fraud, authorization bypass, privilege escalation

---

## SIEM Architecture

### Components

```
Event Sources (Auth, Network, Endpoint, Application)
           |
           v
      Log Collection (Syslog, API, Agent)
           |
           v
      Parsing & Normalization
      (Extract fields: timestamp, user_id, action, result)
           |
           v
       Data Lake / Warehouse
           |
           v
   Detection Engine (Apply Rules)
           |
           v
      Alerting & Escalation
           |
           v
   Incident Response & Forensics
```

### Data Normalization

Raw logs have different formats. Normalize to a common schema:

```
Source: Syslog (Apache)
Raw: 203.0.113.42 - user@example.com [25/Mar/2026:10:45:12 +0000] "POST /api/login HTTP/1.1" 401 512

Normalized (Common Schema):
timestamp=2026-03-25T10:45:12Z
source_ip=203.0.113.42
user_id=user@example.com
service=apache_http
action=authentication
resource=/api/login
method=POST
protocol=HTTP/1.1
status_code=401
status=failure
bytes=512
```

Normalization enables cross-source correlation (a successful login on one system followed by data access on another).

---

## Detection Rules: Sigma Framework

**Sigma** is a SIEM rule format that expresses detection logic in YAML, **independent of the SIEM platform** (Splunk, ELK, ArcSight). Sigma rules are portable; they can be compiled to Splunk SPL, ELK Query Language, or other SIEM query languages.

### Sigma Rule Structure

```yaml
title: Brute Force Login Attempts
id: 0001-brute-force-login
description: Detects multiple failed login attempts from single IP in short time
author: Security Team
date: 2026/03/25
logsource:
  service: authentication
  category: auth_login
detection:
  selection:
    event_type: authentication.login
    result: failure
  timeframe: 5m
  condition: selection | count(user_id) by src_ip > 10
falsepositives:
  - User repeatedly entering wrong password (mistype)
  - Legitimate password manager retries
level: high
status: experimental
```

Translation to Splunk SPL:
```
source=auth event_type=authentication.login result=failure
| stats count by src_ip
| where count > 10
```

### Common Detection Patterns

**1. Threshold-Based**
Alert when event count exceeds threshold:

```yaml
detection:
  selection:
    event_type: authentication.failed_login
  condition: selection | count() > 10
timeframe: 5m
```

**2. Anomaly-Based**
Alert when behavior deviates from baseline:

```yaml
detection:
  baseline:
    event_type: authentication.login
    result: success
  selection:
    event_type: authentication.login
    result: failure
    user_id: $user
  condition: |
    (selection | count() > 3 * avg(baseline | count() by user_id))
```

**3. Correlation-Based**
Alert when multiple conditions occur together:

```yaml
detection:
  login:
    event_type: authentication.login
    result: success
  data_access:
    event_type: data_access
    sensitive_data: true
  condition: login and data_access | within 2m
timeframe: 10m
```

**4. Timeline-Based**
Alert on sequence of events:

```yaml
detection:
  step1:
    event_type: authentication.login
    result: success
  step2:
    event_type: privilege_escalation
  step3:
    event_type: data_exfiltration
  condition: step1 and step2 and step3 | within 1h
```

---

## MITRE ATT&CK Mapping and Threat Hunting

### MITRE ATT&CK Framework

**MITRE ATT&CK** is a taxonomy of adversary tactics and techniques, based on real-world observations. It organizes attack steps into a hierarchy:

```
Tactic (What adversary is trying to do)
  ├─ Reconnaissance
  ├─ Resource Development
  ├─ Initial Access
  ├─ Execution
  ├─ Persistence
  ├─ Privilege Escalation
  ├─ Defense Evasion
  ├─ Credential Access
  ├─ Discovery
  ├─ Lateral Movement
  ├─ Collection
  ├─ Command & Control
  ├─ Exfiltration
  ├─ Impact

Technique (Specific way to accomplish a tactic)
  ├─ T1078: Valid Accounts
  ├─ T1071: Application Layer Protocol
  ├─ T1087: Account Discovery
  ├─ T1543: Create or Modify System Process (Persistence)
  ...

Sub-technique
  ├─ T1078.001: Default Accounts
  ├─ T1078.002: Domain Accounts
```

### Detection Rule Mapping

Link each detection rule to MITRE ATT&CK tactics/techniques:

```yaml
title: Brute Force Login Attempts
mitre_attack:
  - tactic: credential_access
    technique: T1110.001 (Password Guessing)
  - tactic: defense_evasion
    technique: T1078 (Use Alternate Authentication Material)
sigma_rule: ...
```

### Threat Hunting Workflow

**Goal:** Proactively search logs for evidence of adversary techniques, even without a triggering alert.

**1. Hypothesis Formation**
Based on threat intelligence or known attack patterns:

> "An attacker may use a compromised admin account to access the data warehouse and exfiltrate customer records."

**2. Asset Inventory**
Identify systems that could be affected:
- Admin accounts and their owners
- Data warehouse access points
- Sensitive data repositories
- Network egress points

**3. Evidence Collection**
Query logs for indicators of compromise:

```
Query 1: Identify admin logins from unusual locations
  Where: authentication = admin_group AND location != known_office

Query 2: Find admin access to data warehouse
  Where: user_id IN admin_accounts AND service = data_warehouse AND action = query

Query 3: Detect large data transfers from data warehouse
  Where: service = data_warehouse AND bytes_out > 1GB AND direction = outbound

Query 4: Correlate: admin_login → dw_access → data_exfil in same session
```

**4. Analysis and Validation**
Is the activity suspicious or benign?

```
Finding: Admin user login from Moscow, followed by 50GB data warehouse export
Analysis:
  - Check admin's known locations (office in SF, working from home in CA)
  - Check if data export is planned maintenance
  - Check if timezone is inconsistent with user's typical work hours
  - Check if export went to known partners (legitimate) or unknown IPs (suspicious)
```

**5. Escalation and Response**
If confirmed suspicious:
- Lock the account
- Investigate what data was accessed
- Notify relevant stakeholders
- Preserve evidence for forensics

---

## Anomaly Detection and Alert Correlation

### Statistical Anomaly Detection

Train baseline models on normal behavior; flag deviations.

**Baseline Metrics:**
- Requests per user per hour
- Data volume per IP per day
- Failed login rate per domain
- API endpoint access patterns per service

**Detection:**
```
Baseline: User X averages 10 API calls/hour during work hours
Observed: User X made 500 API calls in 1 hour at 3 AM
Deviation: 50x baseline
Alert: High-risk API activity
```

### Machine Learning Anomaly Detection

Train models on historical normal behavior; classify new behavior as normal/anomalous.

**Techniques:**
- **Isolation Forest**: Isolate anomalies in isolation trees (fast, scalable)
- **Local Outlier Factor (LOF)**: Density-based; detect low-density points
- **Autoencoder**: Neural network learns compression of normal behavior; reconstruction error indicates anomaly

**Example (Isolation Forest on login data):**
```
Features: src_ip, dst_host, time_of_day, user_id, login_result
Normal cluster: office IPs, business hours, known users, success
Anomaly: unusual IP, unusual hours, mismatched user, failure rate
```

### Alert Correlation

Multiple alerts in sequence or together = higher confidence.

**Example (Chase Scenario):**
```
Alert 1 (11:00): Brute force login attempts on admin account (10 failures)
  → Response: Disable account, review for compromise

Alert 2 (11:05): Lateral movement detected (admin account accessing database servers)
  → Status: Confirmed attack in progress; lock account immediately

Alert 3 (11:07): Large data transfer from database to external IP
  → Status: Data exfiltration in progress; block IP, preserve forensics

Alert 4 (11:10): Privilege escalation on endpoint (admin account creating new admin account)
  → Status: Persistence; assume total compromise
```

**Correlation improves confidence**: Single alert = noise; chain of related alerts = confirmed incident.

---

## Incident Detection and Response Workflow

### Detection → Triage → Investigation → Response

**1. Detection**
Rule fires or anomaly detected; alert generated.

```
Alert: Brute force login attempts detected
Source IP: 198.51.100.99
Target: admin accounts
Attempts: 500 in 10 minutes
Window: 10:30 - 10:40 UTC
```

**2. Triage**
Is this alert true positive (real) or false positive (noise)?

```
Check:
- Is 198.51.100.99 a known source? (No)
- Is this a bot or penetration test? (Check if part of authorized testing)
- Is target account sensitive? (Yes, admin)
- Is customer impact? (No login succeeded)

Verdict: True positive. Confirmed attack attempt.
```

**3. Investigation**
What actually happened? How far did the attacker get?

```
Steps:
1. All 500 login attempts failed → no breach of admin account
2. Check if same IP targeted other accounts
   Result: 1000 attempts on 50 user accounts (credential stuffing)
3. Check if any succeeded
   Result: Yes, 3 successful logins (users with weak passwords)
4. Investigate those 3 accounts for lateral movement
5. Check auth logs, network logs, endpoint logs for compromise indicators
```

**4. Response**
Contain, remediate, recover.

```
Actions:
1. Immediate: Reset passwords of 3 compromised accounts
2. Immediate: Block source IP 198.51.100.99 at firewall
3. Short-term: Enable MFA on all admin accounts
4. Short-term: Enforce strong password policy for accounts with weak passwords
5. Long-term: Implement adaptive rate limiting on login endpoints
6. Long-term: Implement CAPTCHA challenge on repeated login failures
```

**5. Post-Incident Review**
- Why was this not caught sooner?
- Can we improve detection rules?
- Can we improve prevention?

---

## Best Practices

### Log Retention

- **Compliance minimum**: 1 year for most frameworks (HIPAA, PCI-DSS)
- **Forensic value**: 90 days hot (fast query); older logs archived to cold storage
- **Sensitive data**: Apply redaction/masking (PII, API keys, passwords)

### Log Ingestion Pipeline

Centralize all logs; don't rely on point-in-time queries:

```
Event source → Syslog/API ← Agent → Shipper → Kafka → Processor → Data Lake
                                                                        ↓
                                                                  Search Engine (Elasticsearch, Splunk)
```

### Alert Fatigue Mitigation

Too many alerts train analysts to ignore them. Focus on high-confidence, actionable alerts:

```
❌ Bad: Alert on every failed login attempt
✓ Good: Alert after 10 failed attempts in 5 minutes
```

### Tuning and Baselining

- Establish baseline behavior for each environment (dev, staging, prod)
- Baselines differ: dev has higher churn, prod is stable
- Adjust thresholds to reduce false positives while catching real attacks

---

## See Also

- security-threat-modeling.md (threat model development, threat identification)
- security-incident-response.md (incident response procedures, forensics)
- security-network.md (firewalls, IDS/IPS, network detection)
- security-devsecops.md (security integration into CI/CD, automated scanning)