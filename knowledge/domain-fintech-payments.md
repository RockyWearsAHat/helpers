# Payment Systems: Architecture, Processing & Regulation

## Overview

Payment systems move money between parties, mediate risk, and verify legitimacy. Understanding card processing, gateways, acquirers, and compliance is essential for building e-commerce, SaaS, and marketplace platforms. Payments are fundamentally about **authorization** (verify funds), **capture** (move money), and **settlement** (net positions).

## Card Processing Flow: Authorization, Capture, Settlement

### Authorization

Authorization verifies that a card exists, has sufficient funds, and the cardholder hasn't explicitly blocked the transaction.

**What happens:**
1. Cardholder provides card (PAN: primary account number), expiration, CVC, billing address
2. Merchant's system sends authorization request to payment processor
3. Processor routes to card issuer (bank) via card network (Visa, Mastercard, Amex)
4. Issuer evaluates: fraud signals, velocity checks, balance, limits
5. Issuer responds: approved, declined, or challenge (call card holder)
6. Response includes authorization code (if approved)

**Timing:** 1-3 seconds typical. Network latency dominates; issuer decision is usually <100ms.

**Key fact:** Authorization does NOT move money. It reserves funds temporarily (24-28 hours, varies by issuer). This explains why an authorized charge may disappear before settlement.

**Declined reasons:** Insufficient funds, expired card, CVC mismatch, fraud block, velocity limit (too many txns), address mismatch, issuer offline.

### Capture

Capture submits a previously authorized transaction for settlement. Merchant must submit capture within authorization window or the hold expires and reauthorize.

**Mechanics:**
- Merchant sends capture request with authorization code to processor
- Processor increments settlement batch
- Most systems auto-capture after ~24 hours if not manually captured

**Critical:** Not all authorizations convert to captures. Abandoned carts, customer cancellations, or voided transactions mean authorization never becomes a charge. Reconciliation must handle auth-to-capture ratios.

**Partial capture:** Common in split shipments or subscription downgrades. Capture $50 of a $100 authorization; remainder expires.

### Settlement

Settlement nets all captured transactions and submits to card networks. Networks debit issuer accounts, credit acquiring bank account (where merchant's money goes).

**Daily batches:** Most processors settle once daily, typically overnight. Funds appear in merchant account next business day (1-2 days for international).

**Net settlement:** If merchant has 100 sales ($1,000) and 3 refunds ($30), settlement is $970 net credit to merchant account.

**Interchange fees:** Card networks (Visa, MC) take a cut (1.5-3% typical), paid from merchant's sales. Acquirer takes another cut. Merchant sees net proceeds.

**Fees deducted at settlement:** Interchange, network fees, processor fees, gateway fees—all removed before merchant deposit. Reconciliation must separate gross sales from net deposits.

## Payment Processors, Gateways & Acquirers

### The Main Players

**Payment Gateway** (Stripe, PayPal, 2Checkout): Merchant-facing software. Handles:
- PCI-compliant card collection (tokenization)
- API to submit authorizations
- Dashboard for settlements, refunds, disputes
- Sits between merchant and the deeper network

**Payment Processor/Acquirer** (Stripe, Square, most gateways run their own): Routes transactions to card networks, manages merchant accounts, settles funds. Larger gateways run their own acquirer operations; smaller ones use white-label processing.

**Card Issuer** (Bank, issuing an Amex or Visa card): Makes approve/decline decision.

**Card Network** (Visa, Mastercard, Amex, Discover): Sets rules, operates switch infrastructure, manages acquiring & issuing banks.

### Gateway Responsibilities

1. **PCI Compliance:** Never touch raw card data. Use tokenization: customer submits card to gateway, receives token, merchant stores token. On payment, use token to request charge, gateway handles card.
2. **API for charge submission:** `POST /charges` with token, amount, currency, metadata.
3. **Idempotency:** Support idempotency keys so duplicate requests (network failures) don't double-charge.
4. **Webhooks:** Notify merchant of charge status changes (charge.succeeded, charge.failed, charge.refunded).
5. **PCI scope reduction:** Merchant avoids PCI audit by tokenizing early and never storing raw cards.

### Acquirer Responsibilities

1. **Merchant account:** Legal contract establishing merchant as ISO (Independent Sales Organization) or sub-merchant.
2. **Banking relationship:** Merchant deposit account at acquirer's bank; acquirer settles net proceeds daily.
3. **Risk management:** Monitor chargeback rates, fraud patterns, suspicious activity. Can freeze merchant accounts if risk exceeds threshold.
4. **Network routing:** Integrate with Visa, Mastercard, Amex switching infrastructure.

## PCI DSS Compliance

PCI DSS (Payment Card Industry Data Security Standard) is mandatory for any organization storing, processing, or transmitting card data.

### Key Requirements (12 pillars)

1. **Firewall:** Network segmentation; card data never exposed to internet.
2. **No hardcoded credentials:** Encryption keys, passwords in vaults or HSMs.
3. **Data encryption:** In transit (TLS 1.2+), at rest (AES-256).
4. **Access controls:** Least privilege; separate DEV/PROD. Audit logs for all data access.
5. **Antivirus:** Deployed on systems touching card data.
6. **Application security:** Regular vulnerability scans, penetration tests. Secure coding practices.
7. **Restricted access:** Only employees needing card data get access. Two-factor auth for admin.
8. **Cardholder Data Environment (CDE):** Tightly controlled zone; regular assessments.
9. **Physical security:** Locked server rooms, surveillance, visitor logs.
10. **Monitoring & logging:** Real-time monitoring, 1-year log retention.
11. **Policies:** Security policies, acceptable use agreements, incident response plan.
12. **Assessment:** Annual audits by QSA (Qualified Security Assessor, third-party auditor).

### Compliance Levels

- **Level 1:** >6M transactions/year OR Visa/Mastercard flag: Full PCI audit by QSA
- **Level 2:** 1-6M transactions/year: Self-assessment (SAQ) or auditor review
- **Level 3-4:** <1M transactions/year: Self-assessment questionnaire

**Most businesses use tokenization or hosted payment forms (hosted by gateway) to reduce PCI scope to zero.** This shifts compliance burden to the payment processor (Stripe, Square, PayPal), who are PCI-audited.

## Authentication & Authorization: 3D Secure & SCA

### 3D Secure (3DS)

Original scheme (3D Secure 1.0) required cardholder verification (password) for CNP (Card Not Present) transactions to reduce fraud and chargebacks. Merchant wasn't liable if customer performed 3DS. Now evolved.

**3DS 1.0:** Redirect to issuer's site, enter password. Blocking UX, high abandonment.

**3DS 2.0:** Risk-based authentication. Most transactions (low-risk) pass silently; suspicious ones trigger challenge (password, SMS, biometric). Reduces friction while improving security.

**Challenge types:** Password, SMS OTP, biometric (fingerprint, Face ID), app-based (push notification to banking app).

### Strong Customer Authentication (SCA)

PSD2 (EU) and subsequent regulations mandate SCA: users must authenticate with 2+ of 3 factors (something you know: password; something you have: phone; something you are: biometric) for online payments. 3DS 2.0 fulfills SCA.

**Implementation:** Merchant redirects to issuer or adapter for authentication challenge, then submits authenticated session to capture.

**Exceptions:** Recurring payments (subscriptions) with pre-authorization, low-value transactions (<€30 in some jurisdictions—threshold varies), whitelisted merchants.

**Liability shift:** If merchant implements SCA/3DS and authentication succeeds, issuer usually bears chargeback liability. Without it, merchant bears liability.

## Payment Orchestration & Routing

**Payment orchestration** abstracts multiple acquirers, gateways, and risk engines. Merchant submits charge once; orchestrator routes to best acquirer based on:
- Lowest fees for this transaction type
- Success rate (some acquirers decline more aggressively)
- Geographic presence (local acquirer in cardholder's country)
- Card type (Amex, Visa, etc.; some acquirers specialize)
- Risk flags (high-risk txn → use specialized fraud-tolerant processor)

**Benefits:** Redundancy (retry with secondary if primary fails), cost optimization, fraud resilience.

**Tools:** Stripe Routing, Spreedly, Adyen, Checkout.com offer orchestration layers.

## Reconciliation & Chargeback Management

### Reconciliation

**Challenge:** Authorizations happen in real-time; settlements batch daily. Merchant must match:
- Card charges (real-time API events)
- Deposits (daily ACH/wire into bank account)
- Disputes/chargebacks (days later, reduce deposit)

**Process:**
1. Export daily sales report from gateway
2. Cross-reference with bank settlement statement
3. Flag mismatches: authorizations without capture, captures without deposit, missing refunds
4. Investigate: was capture submitted? Did settlement include it? Did chargeback reduce deposit?

**Tools:** Accounting software (QuickBooks, Xero) integrates gateway APIs for auto-reconciliation.

### Chargebacks

Cardholder disputes a charge with issuer (not merchant directly). Issuer debits merchant account, investigates.

**Reasons:** "I didn't authorize this," "Product not received," "Wrong amount," "Duplicate charge," fraud.

**Merchant response window:** Typically 5-10 days to submit chargeback evidence (shipping proof, delivery confirmation, authorization records, customer communications).

**Merchant must prove:** Cardholder authorized, goods/services delivered, or not eligible for dispute.

**Chargeback ratio:** If >1% of volume (varies by acquirer), account flagged. >2% may trigger freeze or termination.

**Prevention:** Require CVV+address match, use 3DS/SCA, send order and shipping notifications, require signature on physical delivery, keep customer service logs, process refunds promptly (before chargeback escalation).

## Multi-Currency & International Payments

### Dynamic Currency Conversion (DCC)

Allows cardholder to see charge in their home currency at point of sale, converting from merchant's currency. Bank/processor sets exchange rate (often unfavorable). Cardholder bears FX risk.

**Regulation:** Requires opt-in; many jurisdictions restrict or require disclosure. Some gateways phase it out due to poor user sentiment.

### Cross-Border Settlement

Merchant selling to foreign customers faces:
- **Interchange fees:** Vary by country (capped in EU, higher elsewhere)
- **Local acquiring requirements:** Some countries require a local merchant account
- **Regulatory approval:** Some countries restrict foreign payment processing
- **FX hedging:** Merchant bears FX risk if not hedged

Solutions: Use global gateways (Stripe, Adyen) that localize payments, or integrate local PSPs per country.

## Event-Driven Architecture for Payments

Payments are inherently event-driven:
- Payment authorized → emit "payment.authorized"
- Payment captured → emit "payment.captured"
- Refund issued → emit "payment.refunded"
- Chargeback received → emit "payment.chargedback"

**Benefits:** Downstream systems (fulfillment, accounting, CRM) subscribe to events, react asynchronously. Decouples payment processing from business logic.

**Storage:** Immutable event log ensures audit trail and allows temporal queries ("Were we refunded before charge settled?").

## Practical Patterns

### Idempotency

Payment APIs must be idempotent: submitting the same charge twice should not double-charge. Use idempotency keys:

```
POST /charges
{
  "idempotency_key": "user-123-order-456",
  "amount": 5000,
  "currency": "usd"
}
```

If API sees `idempotency_key` again within 24 hours, returns cached response (same charge) instead of processing again.

### Webhook Replay

Gateways should replay failed webhooks. Merchant must be idempotent on webhook receipt (same `charge_id` posted twice doesn't double-refund).

### Reserve & Commit Pattern

For marketplace or complex workflows:
1. Authorize but don't capture
2. Perform business logic (verify inventory, check fraud rules)
3. Capture if logic passes, void if logic fails

Prevents charging for failed orders.

## See Also

- [Blockchain and Distributed Ledger Technology](knowledge/blockchain-distributed-ledger.md) — alternative payment infrastructure
- [API Design Principles](knowledge/api-design.md) — payment API design
- [Security — Compliance Frameworks](knowledge/security-compliance-frameworks.md) — PCI DSS in depth
- [Architecture — Event-Driven Systems](knowledge/architecture-event-driven.md) — event-driven payment processing