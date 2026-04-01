# Email Infrastructure — Protocols, Authentication, Deliverability & Security

## Overview

Email is a decentralized store-and-forward system: clients submit messages to an outbound server (SMTP), which routes to the recipient's server (MTA), which stores the message. Clients retrieve messages via IMAP or POP3. Authentication, encryption, and abuse prevention are bolted on via protocol extensions (SPF, DKIM, DMARC) and transport security (TLS). The architecture, unlike HTTP, is asynchronous and adversarial—spammers abuse the system constantly, forcing continual hardening.

## SMTP (Simple Mail Transfer Protocol)

### Core Protocol
SMTP (RFC 5321) is a text-based protocol on TCP 25 (submission port 587 for authenticated clients). Client connects, transmits message headers and body in CRLF format, then sends `QUIT`. Server responds with numeric codes (2xx success, 4xx temporary error, 5xx permanent error).

```
MAIL FROM:<alice@example.com>
RCPT TO:<bob@example.com>
DATA
To: bob@example.com
From: Alice <alice@example.com>
Subject: Hi

Body text.
.
```

The server validates `RCPT TO:` immediately (real-time validation reduces bounce processing). After `DATA`, the server accepts the message body line-by-line (lines ending in `.` are subject to escaping—a line containing only `.` terminates input). If validation passes, the server queues the message for delivery to the next hop (MTA).

### Limitations & Abuse
SMTP has no built-in authentication (in its original form, anyone could submit from anyone). Relay open to the internet = open mail relay (attacker's spam gateway). Early abuse led to:

1. **Authenticated SMTP** (RFC 2554): `AUTH` command requires username/password or certificate. Submission ports (587, 465) enforce authentication; MTA-to-MTA usually does not (open relay between domains was normal, now phased out).
2. **Bounce handling**: Invalid RCPT TO rejected; delivery errors return to sender as bounces (non-delivery reports, NDRs). False rejections (greylisting, temporary errors) cause retries over hours or days.

## IMAP & POP3 (Message Retrieval)

### IMAP (RFC 3501)
Stateful protocol (connection persistent). Client logs in, selects a mailbox, then searches, fetches, and deletes messages without downloading full bodies. Server maintains state (current mailbox, *seen* flags, custom flags per message). Supports multiple clients simultaneously (concurrent access, conflict resolution via flags and unseen counts).

Advantages: bandwidth-efficient (fetch only headers or parts), server-side search, folder hierarchy. Disadvantages: server holds connection state, requiring server-side resources.

### POP3 (RFC 1939)
Stateless protocol. Client logs in, retrieves all new messages, usually deletes them from the server. Simple, lightweight, but doesn't support folders or selective retrieval. Suitable for mobile clients downloading once daily. Deprecated in favor of IMAP; still used for legacy systems.

## Email Authentication: SPF, DKIM, DMARC, ARC

Spam and phishing rely on spoofed sender addresses. Authentication proves the sender is legitimate.

### SPF (Sender Policy Framework, RFC 7208)
Domain publishes a DNS TXT record specifying which IP addresses are authorized to send mail from the domain:

```
example.com.  TXT  "v=spf1 ip4:192.0.2.0 include:mail.example.com ~all"
```

Receiving server queries the sender domain's SPF record, checks if the SMTP client IP matches. If not, the message fails SPF. The `~all` (soft fail) or `-all` (hard fail) specifies behavior for non-matching IPs. Limitations: SPF only checks the SMTP `MAIL FROM:` envelope sender, not the `From:` header (which users see). User-visible address can still be spoofed.

### DKIM (DomainKeys Identified Mail, RFC 6376)
Signs the email body and headers with the sender domain's private key. Receiver validates the signature using the public key from DNS. Sender adds a `DKIM-Signature:` header including:

```
DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed;
  d=example.com; s=selector1;
  h=from:to:subject:date;
  bh=<base64-hash-of-body>;
  b=<base64-signature>
```

The hash covers the body (allowing minor whitespace changes); the signature covers headers (From, To, Subject, etc.) and the body hash. Different selectors (versions/keys) allow key rotation. Advantages: proves origin and integrity, sign the user-visible From: header. Disadvantages: if the message is forwarded or mailinglist-munged, signature breaks (though `c=relaxed` allows whitespace normalization).

### DMARC (Domain-based Message Authentication, Reporting and Conformance, RFC 7489)
Policy layer. Sender domain publishes a DMARC policy:

```
_dmarc.example.com.  TXT  "v=DMARC1; p=reject; rua=mailto:admin@example.com"
```

Policies:
- `p=none` — monitor; don't reject
- `p=quarantine` — deliver to spam folder
- `p=reject` — reject entirely

DMARC checks whether SPF or DKIM aligns with the user-visible `From:` header domain. Alignment:
- SPF: `MAIL FROM:` domain matches or is a subdomain of `From:` domain
- DKIM: signing domain matches or is subdomain of `From:` domain

Reports (RUAs) are sent daily to the policy author, detailing authentication results (e.g., percentage passing SPF/DKIM). Allows monitoring of spoofing attempts and subdomain abuse.

### ARC (Authenticated Received Chain, RFC 8617)
Preserves authentication results as a message traverses intermediaries (mailing lists, forwarding services). Without ARC, intermediate processing (adding headers, modifying body) breaks DKIM signatures. ARC adds a chain of custody:

```
ARC-Seal: i=1; cv=none; a=rsa-sha256; ...; b=<signature-of-ARC-chain>
ARC-Message-Signature: i=1; a=rsa-sha256; ...; b=<signature-of-original-message>
ARC-Authentication-Results: i=1; <original-SPF/DKIM/DMARC-results>
```

Each intermediary adds a new member to the chain (incremented `i`), signing the previous chain and the original message. Receivers verify the chain integrity and the original authentication results. Critical for mailing lists that must modify message while preserving proof of origin.

## MTA Architecture & Message Flow

```
Client (MUA)
  ↓ SMTP (RFC 5321, auth required)
Submission server (port 587)
  ↓ SMTP (MTA-to-MTA, RFC 5321)
Relay & intermediaries
  ↓
Destination MTA (inbound port 25)
  ↓
SMTP to local store (LMTP or local delivery agent)
Local storage (Maildir or mbox format)
  ↓ IMAP or POP3
User's client (MUA)
```

MTAs (postfix, Exim, sendmail) perform routing, filtering, and retry logic. Inbound MTAs validate recipients, apply SPF/DKIM/DMARC policies, and scan for spam/malware. Retry policies (exponential backoff over hours) handle temporary failures (`4xx` SMTP codes). Permanent failures (`5xx`) generate bounces.

## Email Deliverability: Bounce Handling & Compliance

### Bounces
Two categories:
- **Hard bounces** (550 code): recipient doesn't exist, permanent rejection. Sender account (bulk mailer) should stop sending to that address.
- **Soft bounces** (421, 450 codes): temporary (mailbox full, server overloaded). Retry later. Many retries over 24-72 hours are normal.

Bulk mailers maintain bounce lists; persistent bounces degrade sender reputation.

### Sender Reputation
ISP filters (Gmail, Outlook, Yahoo) track sender IP/domain reputation based on complaint rate, bounce rate, and list-unsubscribe compliance. High reputation = inbox, low reputation = spam folder. Reputation is domain and IP specific:

- Dedicated IP (one customer): reputation directly tied to that customer's sending behavior.
- Shared IP: reputation averaged across customers (one bad customer hurts all).

### Compliance
CAN-SPAM (US), GDPR (EU), and Canada's CASL require bulk senders to:
- Include physical address.
- Honor unsubscribe requests within 10 days.
- Accurate subject and From: header.
- Opt-in prior to sending (varies: opt-in for permission emails, opt-out for transactional).

Violations trigger legal liability; major ISPs reject non-compliant senders.

## Email Security: Encryption & End-to-End Protection

### TLS (Transport Layer Security)
SMTP over TLS (STARTTLS, port 587, or implicit TLS, port 465) encrypts the connection, protecting credentials and message content in transit. Opportunistic TLS (common): attempt TLS if available, fall back to plaintext if TLS fails. Risky against downgrade attacks; strict TLS (MTA-STS, RFC 8461) forces TLS:

```
_mta-sts.example.com.  TXT  "v=STSv1; id=<id>"
Fetch policy from https://mta-sts.example.com/.well-known/mta-sts.txt
```

MTA-STS policy enforces TLS for mail to the domain or rejects delivery. Prevents downgrade attacks at delivery-time.

### S/MIME (Secure/Multipurpose Internet Mail Extensions)
End-to-end encryption and signing. Sender encrypts the message with the recipient's public key (RSA, encrypted with recipient's cert). Recipient decrypts with private key. Sender also signs with their private key (non-repudiation). Requires certificate infrastructure: users must obtain and manage X.509 certificates. Complex, rarely used outside enterprise and government (which mandate it).

### PGP/GPG (Pretty Good Privacy)
Alternative to S/MIME. Users generate RSA key pairs, share public keys (via PGP keyservers or direct exchange). Message encrypted with recipient's public key, signed with sender's private key. Decryption and signature verification requires recipient's trust in the keyserver. Lower adoption than S/MIME outside technical communities; keyserver history (privacy-violating key-disclosure) reduced adoption.

### DANE (DNS-based Authentication of Named Entities, RFC 6698)
TLSA records in DNS publish server certificates:

```
_25._tcp.mail.example.com.  TLSA  3 1 1 <sha256-of-cert>
```

Receiving MTA fetches the TLSA record (via DNSSEC), verifies the sender's certificate matches. Eliminates need for certificate authorities (CA-less). Requires DNSSEC to prevent DNS spoofing. Limited deployment (DNSSEC is not pervasive).

## Anti-Spam & Modern Email Protocols

### Techniques
- **Greylisting**: reject mail from unknown senders (4xx), re-accept from legitimate senders after a short delay. Spam bots rarely retry; legitimate MTAs do.
- **RBLs (Real-time Blacklists)**: databases of known-spam IP ranges. Inbound MTA queries RBL; if sender IP is listed, reject.
- **Content filtering**: Bayesian filters, character-encoding analysis, URL whitelisting/blacklisting.
- **Rate limiting**: reject if sender exceeds messages-per-minute threshold.

### JMAP (JSON Meta Application Protocol, RFC 8620)
Modern replacement for IMAP/SMTP/POP3. Stateless, batch-oriented API (JSON over HTTP). Single endpoint handles mail, calendar, contacts. No connection state. Advantages: firewall-friendly, batch efficiency, modern (supports push via WebSocket). Disadvantages: not widely implemented; IMAP still dominant.

## Key References

- RFC 5321 (SMTP), RFC 3501 (IMAP), RFC 1939 (POP3)
- RFC 7208 (SPF), RFC 6376 (DKIM), RFC 7489 (DMARC), RFC 8617 (ARC)
- RFC 8461 (MTA-STS), RFC 8620 (JMAP)
- RFC 5322 (Email message format), RFC 2045–2049 (MIME)
- "Email Explained from First Principles" (detailed architecture walkthrough)