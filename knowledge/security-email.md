# Email Security — Authentication, Phishing Defense, Gateway Control & Header Analysis

## Overview

Email is the oldest Internet protocol still in heavy use and was designed with minimal authentication. Spammers and phishers exploit this by impersonating legitimate senders. Modern email security layers cryptographic authentication (SPF, DKIM, DMARC), reputation signals, and gateway inspection to restore trust. Unlike HTTPS, where the client validates the server's certificate, email authentication is publisher-initiated—the sender proves they own the domain.

---

## The Authentication Stack

### SPF: Sender Policy Framework (RFC 7208)

SPF authorizes which mail servers are permitted to send mail for a domain by publishing a TXT record in DNS.

**Mechanism:** Domain owner publishes an SPF record specifying allowed senders:

```
example.com TXT "v=spf1 ip4:192.0.2.1 include:sendgrid.com -all"
```

This record says: "I only accept mail from 192.0.2.1 or SendGrid's servers. Reject (-all) anything else."

When a mail arrives claiming to be from example.com, the receiving server queries for example.com's SPF record and validates that the sender's IP matches the policy. Common mechanisms:

- `ip4:192.0.2.1` — explicit IPv4 address
- `include:example.com` — include another domain's SPF record (for delegated sending)
- `mx` — the domain's MX record IPs are permitted senders
- `ptr:example.com` — reverse DNS lookup (deprecated, unreliable)
- `~all` — softfail (accept but mark suspicious); `-all` — hardfail (reject)

**Limitations:** SPF only validates the SMTP Return-Path (Envelope From), not the visible From header. Attackers can forge the visible From while using legitimate infrastructure. SPF also doesn't prevent a compromised legitimate server from sending phishing mail.

**Complexity:** SPF records inherit the "SPF explosion" problem—each include directive queries another domain's SPF record; chains can exceed DNS lookup limits. Most SPF records support 10 mechanisms; exceeding that requires careful optimization.

### DKIM: DomainKeys Identified Mail (RFC 6376)

DKIM cryptographically signs email message bodies and headers, proving the sender owns the domain and the message hasn't been altered in transit.

**Mechanism:** A domain publishes a public key in DNS (typically under `selector._domainkey.example.com`). MTA signs outgoing mail with the corresponding private key, adding a DKIM-Signature header. Receiving servers retrieve the public key and verify the signature.

```
DKIM-Signature: v=1; a=rsa-sha256; s=default; d=example.com;
                h=from:to:subject:date;
                bh=47DEQpj8HBSa+...; b=oJeEy...
```

The signature authenticates a subset of headers (From, To, Subject, Date, etc.) specified in the `h=` field. The message body is hashed (bh field); if intermediaries modify the body, the hash won't match.

**Advantages:** Unlike SPF, DKIM validates the message content. Multiple DKIM signatures can be added by intermediaries (MSP, mailing list services), proving that multiple parties handled the message legitimately.

**Disadvantages:** DKIM doesn't specify policy (how receiving servers should handle unsigned mail). A domain can publish a DKIM public key but not require it. Key rotation requires updating DNS records; expired keys cause validation failures. Some services (mailing lists, forwarding services) strip DKIM signatures in transit, breaking validation.

### DMARC: Domain-based Message Authentication, Reporting & Conformance (RFC 7489)

DMARC specifies policy: how receivers should handle mail that fails SPF/DKIM checks, and what reports to send back.

**Mechanism:** A domain publishes a `_dmarc.example.com` TXT record specifying policy:

```
_dmarc.example.com TXT "v=DMARC1; p=reject; rua=mailto:abuse@example.com"
```

Policy modes:

- `p=none` — don't reject; report on failures (audit mode).
- `p=quarantine` — move to spam folder if SPF/DKIM fails and alignment is broken.
- `p=reject` — reject the message outright.

**Alignment:** DMARC requires either SPF or DKIM to be aligned: the domain in the visible From header must match the domain that passes SPF or DKIM. This prevents header spoofing. Example:

- SPF from `192.0.2.1` (authorized for example.com) sends mail with From: attacker@other.com → fails alignment (different domain) → rejected if p=reject.

**Reporting:** DMARC senders request aggregate reports (rua) or forensic reports (ruf) sent to specified email addresses. Aggregate reports summarize pass/fail counts; forensic reports contain failed message headers.

**Adoption:** DMARC is more widely deployed than DKIM or SPF but requires careful policy tuning. Aggressive policies (p=reject) without full sub-domain coverage often break legitimate mail. Domains roll out in p=none → p=quarantine → p=reject stages over months.

### ARC: Authenticated Received Chain (RFC 8617)

ARC chains DKIM signatures from intermediaries (mailing lists, forwarding services) that re-send mail. Traditional DKIM breaks if a mailing list modifies the body; ARC preserves the signing chain.

**Mechanism:** Each hop signs the message with an arc-signature and arc-seal header. The seal covers all prior seals, creating a tamper-proof chain.

```
ARC-Seal: i=3; cv=pass; as=rsa-sha256; s=google; ...
ARC-Message-Signature: i=3; a=rsa-sha256; c=relaxed/relaxed; ...
ARC-Authentication-Results: i=3; spf=pass; dkim=pass; ...
```

Receivers validate the chain from receiver to originator. If the chain is valid, the mail passes authentication despite body modifications.

**Adoption:** Modest adoption; mainly supported by major email providers (Gmail, Microsoft 365, Yahoo) and mailing list services. Still considered emerging compared to DKIM/SPF.

### BIMI: Brand Indicators for Message Identification

BIMI allows a domain to associate a logo (SVG or raster file) with authenticated mail, enabling receivers to display the brand logo in the inbox if SPF/DKIM/DMARC passes.

**Use:** Domains publish a `default._bimi.example.com` record pointing to a logo URL. Email clients that support BIMI fetch and display the logo for authenticated mail from that domain.

**Adoption:** Minimal; supported by a few email providers and webmail interfaces. Primarily a marketing feature rather than a security mechanism.

---

## Phishing & Header Spoofing

### Phishing Techniques

Phishing exploits the visual similarity between legitimate and forged mail. Common vectors:

- **Domain similarity:** attacker@examp1e.com (I instead of 1) or attacker@example-security.com (uses dash instead of dot).
- **Visible From impersonation:** SPF/DKIM/DMARC failure caught by filter; attacker hopes user ignores the warning.
- **Compromised account:** attacker gains credentials to a legitimate domain; no authentication fails. Difficult for filters to detect.
- **Display name abuse:** From: "Bank of America" <attacker@bank.com> (spoofed display name, visible sender is different).

### Defense Models

**Sender-side:** Publish SPF/DKIM/DMARC policies (p=reject) and monitor reports for unauthorized senders. Disable Less Secure Apps or enforce stronger authentication. Monitor for account takeovers via unusual sending patterns (new IP, bulk sending).

**Receiver-side:** Implement strict DMARC validation. Apply machine learning to detect phishing content (unusual URL formatting, suspicious links). Scan attachments for malware. Use threat intelligence feeds (URL reputation, sender reputation) to block known phishing campaigns. Require multi-factor authentication for account access.

**Content inspection:** Some phishing uses legitimate images or PDFs with embedded links. Scanners may deactivate hyperlinks in email or require users to preview links before clicking.

---

## Email Gateway Security

### MTA Filtering & Scanning

Email gateways (e.g., ProofPoint, Mimecast, Cisco ESA) sit between external MTAs and internal mail servers. They:

- **Rate-limit:** Block senders with abnormal sending patterns.
- **Reputation check:** Query sender IP/domain against RBLs (Real-time Blackhole Lists) to block known spammers.
- **Attachment scanning:** Run antivirus and sandboxing on executable attachments (exe, zip, macro-enabled docs).
- **URL rewriting:** Log HTTP requests triggered by email links to detect click patterns indicative of compromise.
- **DLP (Data Loss Prevention):** Block outbound mail containing credit card numbers, SSNs, or other sensitive patterns.

### RBL (Real-time Blackhole Lists)

RBLs are DNS-based blacklists of sender IPs or domains known to send spam or malware. When a mail arrives, the gateway queries the RBL (e.g., spamhaus.org). Common RBLs:

- **SPAMHAUS:** PBL (Provider Block List) lists residential ISP IPs; PSL (Policy Block List) lists IPs with policy violations.
- **Barracuda Reputation Block List (BRBL):** Community-driven list of spam sources.

RBLs are often combined: if an IP is on ANY RBL, reject the mail.

**Limitations:** RBLs have false positives (sometimes compromised residential ISPs list IPs incorrectly). Sophisticated attackers rotate through many IPs, bypassing RBLs. RBLs primarily catch bulk spammers, not targeted phishing.

---

## Header Analysis & Forensics

Email headers contain the full path of the message: originating server, intermediate MTAs, final delivery. Headers reveal:

```
Received: from mx.example.com (192.0.2.1) by mail.org (192.0.2.2)
	with SMTP id 12345
	for recipient@corp.com; Mon, 1 Jan 2024 12:34:56 +0000
DKIM-Signature: v=1; a=rsa-sha256; s=default; d=example.com; ...
SPF-Result: pass
Authentication-Results: spf=pass, dkim=pass, dmarc=pass
From: sender@example.com
```

**Key insights:**

- **Originating IP:** First Received header lists the sending server's IP. Cross-check against SPF policy.
- **MTA chain:** Received headers trace the message path. Unexpected MTAs or unusual geographic hops may indicate compromise.
- **Timing:** Timestamp deltas between Received headers show delivery latency. Unusual delays or time reversals may indicate forgery or relay hijacking.
- **Authentication results:** DKIM, SPF, DMARC pass/fail status. Failures combined with warnings suggest phishing.
- **Return-Path:** The Envelope From (SMTP MAIL FROM command). This is what SPF validates. Different from the visible From header.

**Forensic tools:** Email clients (Outlook, Gmail) display full headers when requested. Security teams parse headers programmatically to detect spoofing or trace attack campaigns.

---

## Operational Challenges

### Sieve Failures Under SPF

SPF subdomains often have separate policies. Mail from subdomain.example.com uses its own SPF record, not example.com's. Misconfigurations (e.g., subdomain lacks SPF record) cause mail to fail SPF validation.

### Forwarding Breakage

Mail forwarding services intercept mail and re-send it from their servers. This breaks SPF (the forwarded mail comes from forwarder's IP, not the original sender's). DKIM often breaks if the forwarder modifies headers. Solutions: use ARC (if supported) or deploy a service-specific forwarding configuration (Some forwarding services authenticate with the original domain's credentials).

### Configuration Complexity

Deploying SPF/DKIM/DMARC requires DNS record creation and MSP (Mail Service Provider) integration. Each sending channel (transactional email, mailing lists, forwarding) may require separate configuration. Under-deployment (partial coverage) creates false negatives, undermining trust in authentication.

---

## Related Topics

See also: [networking-email](networking-email.md), [security-network](security-network.md), [cryptography-key-management](cryptography-key-management.md), [infrastructure-dns-security](infrastructure-dns-security.md).