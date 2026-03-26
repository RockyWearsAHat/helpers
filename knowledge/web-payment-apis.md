# Web Payment APIs: Request API, Payment Handlers & PCI Compliance

## Overview

Web payment processing involves multiple standards and compliance layers. The **Payment Request API** provides a browser-native checkout UI for card/wallet payments. The **Payment Handler API** lets payment providers (Apple Pay, Google Pay, banks) integrate with that UI. **PCI DSS compliance** governs how sensitive payment data can be handled. Understanding the payment flow, tokenization, and compliance boundaries is essential for secure e-commerce.

The key mental model: **Never touch raw card data.** Tokenization and compliance exist to isolate payment processing from general application code. Frameworks like Stripe and payment protocols like 3DS shift risk to specialized processors.

---

## Payment Request API (W3C Standard)

### Core Concept

The Payment Request API lets merchants request payment through a standardized browser interface. The browser shows a selector combining multiple payment methods (cards, digital wallets, bank accounts).

```javascript
const paymentRequest = new PaymentRequest(
  [
    { supportedMethods: 'basic-card', data: { supportedNetworks: ['visa', 'mastercard'] } },
    { supportedMethods: 'https://google.com/pay', data: { /* Google Pay config */ } }
  ],
  {
    total: { label: 'Order Total', amount: { currency: 'USD', value: '19.99' } },
    displayItems: [
      { label: 'Item 1', amount: { currency: 'USD', value: '15.00' } },
      { label: 'Shipping', amount: { currency: 'USD', value: '4.99' } }
    ]
  },
  { requestShipping: true, requestBillingAddress: true, requestPayerEmail: true }
);

// Show UI
paymentRequest.show().then(paymentResponse => {
  // paymentResponse contains instrumentKey (tokenized card) + metadata
  console.log(paymentResponse.details); // { cardSecurityCode: "***", ... }
  console.log(paymentResponse.payerEmail);
  console.log(paymentResponse.shippingAddress);
  
  paymentResponse.complete('success'); // Confirm completion
}).catch(err => console.error(err));
```

### Payment Methods

The request specifies supported methods. Common ones:

| Method | Provider | Data |
|--------|----------|------|
| `basic-card` | Browser (legacy) | Card networks, billing address |
| `https://google.com/pay` | Google Pay | Merchant ID, gateway config |
| `https://apple.com/apple-pay` | Apple Pay | Merchant ID, certificate |
| Bank transfer URLs | Nordic, German banks | IBAN, account info |

### Flow

1. **create PaymentRequest** — Define total, items, accepted methods
2. **show()** — Browser displays payment UI (picker, address form)
3. **paymentResponse** — User selects method; browser returns tokenized data (not raw card number)
4. **complete()** — Confirm transaction success/failure
5. **Backend processes** — Server validates token with payment processor

### Browser Adoption

- **Chrome/Edge:** Full support
- **Safari:** Apple Pay via Payment Request (limited to native apps for native Pay)
- **Firefox:** Partial (Stripe integration in some regions)
- **Mobile:** Strongest support (payment method selection natural on phone)

---

## Payment Handler API

### Concept

Payment Handler API allows providers (fintech apps, banks, wallets) to register as payment methods alongside Apple/Google Pay. When a merchant requests payment, the browser can launch a provider's registered handler.

```javascript
// In a payment provider's service worker
self.addEventListener('paymentrequest', event => {
  event.respondWith(
    clients.matchAll({ type: 'window' }).then(clients => {
      // Send payment details to handler UI window
      clients[0].postMessage({
        type: 'payment-request',
        amount: event.total,
        items: event.displayItems
      });
      
      return new Promise(resolve => {
        self.addEventListener('message', msg => {
          if (msg.data.type === 'payment-response') {
            resolve({
              methodName: 'https://myprovider.com/pay',
              details: { token: msg.data.token, ... }
            });
          }
        });
      });
    })
  );
});
```

### Registration

Providers register via manifest:

```json
{
  "payment": {
    "supported_delegations": ["shippingAddress", "payerEmail", "payerPhone"]
  }
}
```

Enables **decentralized payment** — users add providers, sites don't hardcode integrations.

---

## Tokenization & Sensitive Data

### Why Tokenization Matters

Raw card data must never touch a merchant's servers. **PCI DSS compliance** prohibits it. Solution: **tokenization** — payment processors (Stripe, Square, Adyen) exchange card data for tokens valid only for that merchant.

```javascript
// DO NOT do this (PCI violation):
const cardData = { number: '4111111111111111', cvc: '123', ... };
fetch('/api/charge', { method: 'POST', body: JSON.stringify(cardData) }); // ILLEGAL

// DO this instead (PCI-compliant):
const { token } = await stripe.createToken(cardNumber, cvc, exp_date);
fetch('/api/charge', { method: 'POST', body: JSON.stringify({ token }) });
```

### Stripe Tokens & Payment Intents

**Tokens** (legacy) — Single-use card placeholders:

```javascript
const { token } = await stripe.createToken(cardElement);
// token.id: "tok_visa" — safe to send to server
```

**PaymentIntents** (modern) — Multi-step payment workflow with built-in fraud detection:

```javascript
// Client: Create intent reference
const { clientSecret } = await fetch('/create-payment-intent', { method: 'POST' }).then(r => r.json());

// Confirm with card details
const { paymentIntent, error } = await stripe.confirmCardPayment(clientSecret, {
  payment_method: { card: cardElement, billing_details: { ... } }
});

// paymentIntent.status: "succeeded" | "requires_action" | "requires_confirmation"
```

### Tokenization Scope

Tokens are:
- **Merchant-specific** — Token from Stripe for Merchant A invalid for Merchant B
- **Processor-specific** — Stripe token can't be used with Square
- **Payment-method specific** — Card token ≠ wallet token
- **Time-limited** — Tokens expire (usually 15 min for single-use, longer for stored payment methods)

---

## PCI DSS Compliance

### Levels & Scope

**PCI DSS (Payment Card Industry Data Security Standard)** defines requirements for handling cardholder data. Compliance level depends on transaction volume:

| Level | Annual Txns | Requirements | Examples |
|-------|-------------|--------------|----------|
| **1** | Any (if breach) | Full audit, on-site assessment, managed firewall | Enterprise retailers, processors |
| **2** | 1M–6M | Annual self-assessment, firewall config review | Medium e-commerce |
| **3** | 6M–20M+ | Annual self-assessment, limited network scans | Small e-commerce |
| **4** | <1M | Simple questionnaire | Micro-merchants, low-risk |

### Key Requirements

1. **Never store raw card data** — Tokenize immediately upon receipt
2. **Encrypt in transit** — TLS 1.2+ for all cardholder data flows
3. **Encrypt at rest** — If storing tokens, encrypt with strong keys
4. **Access controls** — Role-based restrictions; audit logs
5. **Network segmentation** — Payment systems isolated from general systems
6. **Vulnerability management** — Regular scanning, patching
7. **Monitoring & logging** — Track all access to cardholder data

### Exemptions via Tokenization

If you use a **PCI-compliant payment processor** and **never touch raw card data**, you avoid most L1 requirements:

```
Your app → Stripe (PCI L1) → Card network
         (tokens only)
```

You're responsible for:
- Secure communication to Stripe (TLS)
- Token storage security
- Access controls for stored tokens
- Auditing usage

Stripe assumes cardholder data responsibility; you assume token responsibility.

---

## 3D Secure (3DS) Authentication

### Why 3DS Exists

Chargebacks cost merchants 2–3% of revenue. **3D Secure** shifts liability to issuers (banks) by verifying the cardholder via their bank.

```
User initiates payment
→ Merchant requests 3DS
→ Card issuer sends verification challenge (OTP, biometric, password)
→ Cardholder completes challenge
→ Issuer returns verified status
→ Merchant charges (issuer liable for fraud if verified)
```

### 3DS Flow (Stripe Implementation)

```javascript
// 1. Create PaymentIntent with authentication required
const intent = await fetch('/create-payment-intent', {
  method: 'POST',
  body: JSON.stringify({ amount: 1999, confirm: false })
}).then(r => r.json());

// 2. Confirm payment (may require user challenge)
const { paymentIntent, error } = await stripe.confirmCardPayment(intent.clientSecret, {
  payment_method: { card: cardElement },
  receipt_email: 'user@example.com'
});

// 3. Check status
if (paymentIntent.status === 'requires_action') {
  // Bank sent challenge; use handleNextAction to complete
  const { paymentIntent: nextIntent } = await stripe.handleCardAction(intent.clientSecret);
}

if (paymentIntent.status === 'succeeded') {
  // Payment complete + verified
}
```

### 3DS Versions

- **3DS 1.x** (legacy) — Pop-up verification; high abandon rate
- **3DS 2.x** (modern) — Network fraud detection + adaptive challenges
  - Frictionless: ~95% bypassed (low-risk transactions)
  - Challenge: 5% require verification (higher risk)

Liability shift: Issuer liable (not merchant) if transaction 3DS-verified.

---

## Recurring & Subscription Payments

### Setup Intent (Mandate)

For subscriptions, store the card **once** via SetupIntent; charge repeatedly:

```javascript
// Day 1: Save card
const setupIntent = await fetch('/create-setup-intent').then(r => r.json());
const { setupIntent: confirmedSetup } = await stripe.confirmCardSetup(
  setupIntent.clientSecret,
  { payment_method: { card: cardElement } }
);
const paymentMethodId = confirmedSetup.payment_method;
// Send paymentMethodId to server; store securely

// Day 30: Charge subscription
const charge = await stripe.charges.create({
  amount: 999,
  currency: 'usd',
  customer: customerId,
  payment_method: paymentMethodId,
  off_session: true // Server-initiated, no UI
});
```

### Recurring Billing

Common patterns:
- **Subscription billing** — Fixed plan; monthly charge
- **On-demand billing** — Usage-based; charge after service
- **Installments** — Spread over multiple months

All use stored payment method + off-session processing.

---

## Apple Pay & Google Pay Integration

### Apple Pay on Web

```javascript
const request = new ApplePayRequest({
  countryCode: 'US',
  currencyCode: 'USD',
  supportedNetworks: ['visa', 'mastercard', 'amex'],
  merchantCapabilities: ['supports3DS'],
  total: { label: 'Store Name', amount: '19.99' },
  requiredBillingContactFields: ['postalAddress']
});

const session = new ApplePaySession(3, request);

session.onvalidatemerchant = event => {
  // Validate merchant with Apple
  fetch('/validate-apple-merchant').then(response => {
    session.completeMerchantValidation(response.json());
  });
};

session.onpaymentauthorized = event => {
  // Process payment token
  const token = event.payment.token.paymentData;
  fetch('/charge-apple-pay', { method: 'POST', body: JSON.stringify({ token }) })
    .then(() => session.completePayment(ApplePaySession.STATUS_SUCCESS))
    .catch(() => session.completePayment(ApplePaySession.STATUS_FAILURE));
};

session.begin();
```

Requirements:
- iOS/macOS (web limited)
- HTTPS + valid domain
- Merchant certificate from Apple

### Google Pay on Web

```javascript
const paymentsClient = new google.payments.api.PaymentsClient({
  environment: 'PRODUCTION'
});

const request = {
  apiVersion: 2,
  apiVersionMinor: 0,
  allowedPaymentMethods: [
    {
      type: 'CARD',
      parameters: {
        allowedAuthMethods: ['PAN_ONLY', 'CRYPTOGRAM_3DS'],
        allowedCardNetworks: ['VISA', 'MASTERCARD']
      },
      tokenizationSpecification: {
        type: 'PAYMENT_GATEWAY',
        parameters: { gateway: 'stripe', gatewayMerchantId: 'YOUR_MERCHANT_ID' }
      }
    }
  ],
  merchantInfo: { merchantId: 'YOUR_MERCHANT_ID', merchantName: 'Your Store' },
  transactionInfo: {
    totalPriceStatus: 'FINAL',
    totalPrice: '19.99',
    currencyCode: 'USD'
  }
};

paymentsClient.loadPaymentData(request).then(paymentData => {
  // Tokenized payment method ready
  const token = paymentData.paymentMethodData.tokenizationData.token;
  // Send to server
});
```

Advantages of wallet integrations:
- User experience (one-click checkout)
- Fraud reduction (wallet providers manage identity)
- 3DS handled by provider

---

## Common Implementation Patterns

### Client-Side Flow (Recommended)

1. Merchant collects card details in **hosted iframe** (Stripe Elements, Square Web Payments)
2. Client-side JavaScript **tokenizes** (never touches raw data)
3. **Token + billing info** sent to merchant server
4. Server charges token via **payment processor API**

```javascript
// Client
const cardElement = stripe.elements().create('card');
cardElement.mount('#card-element');
cardElement.addEventListener('change', e => console.log(e.error));

document.getElementById('pay-button').addEventListener('click', async () => {
  const { token } = await stripe.createToken(cardElement);
  await fetch('/api/charge', { method: 'POST', body: JSON.stringify({ token, amount: 1999 }) });
});
```

### Server-Side Charge

```python
# Python backend (Stripe)
import stripe
stripe.api_key = "sk_live_..."

try:
    charge = stripe.Charge.create(
        amount=1999,
        currency='usd',
        source=token_id,
        description='Order #12345'
    )
    print(charge.status) # succeeded | failed
except stripe.CardError as e:
    print(e.user_message) # Card declined
```

---

## Security & Gotchas

1. **Never log card data** — Even if tokenized, log processor—not cards
2. **HTTPS mandatory** — All payment flows (browser, server, processor) must use TLS 1.2+
3. **Token expiry** — Tokens expire; implement retry logic
4. **Idempotency keys** — Prevent double-charging on network failures
5. **Webhook verification** — Processor sends events (charge.succeeded); verify signatures
6. **Rate limiting** — Prevent brute-force card testing attacks
7. **CVV never stored** — CVC is disposable per transaction; never persist it

### Idempotency Example

```javascript
// Server: Add idempotency key to prevent duplicate charges
const idempotencyKey = request.headers['idempotency-key'] || generateUUID();
const cacheKey = `charge:${idempotencyKey}`;
const cached = cache.get(cacheKey);

if (cached) return cached; // Already charged; return duplicate response

const charge = await stripe.charges.create(
  { amount, currency, source: token },
  { idempotencyKey } // Stripe deduplicates on key
);

cache.set(cacheKey, charge);
return charge;
```

---

## Architecture Patterns

### Monolith (Small)

```
Merchant frontend (payment form)
→ Merchant server (validates, charges via Stripe API key)
← Stripe (tokenizes, deposits to bank account)
```

### Decoupled (Medium)

```
Merchant frontend (hostedStripe iframe)
→ Merchant server (webhook receiver, order processor)
→ Stripe (async payment state machine)
← Webhook (charge.succeeded event)
```

### Microservices (Large)

```
Frontend (Stripe Elements)
→ API Gateway (auth check)
→ Payment Service (knows Stripe keys, PCI scope)
→ Order Service (listens to payment events)
→ Notification Service (emails receipts)
← Message queue (async event flow)
```

---

## Design Decisions

**Use Payment Request API when:**
- Supporting multiple payment methods (cards + wallets)
- Mobile checkout critical (parity with app stores)
- Browser automation handles method selection

**Use custom checkout when:**
- Specific UI/UX needed
- Existing payment relationships (direct processor connection)
- Embedded checkout in modal/page

**Tokenization strategy:**
- Processor's SDKs (Stripe Elements, Square Web Payments) — minimum PCI burden
- Processor's hosted payment pages (Stripe Checkout) — zero PCI burden for merchant
- Self-hosted + tokenization API — medium PCI burden, more control

See also: [security-compliance-frameworks.md](security-compliance-frameworks.md), [api-authentication.md](api-authentication.md), [api-error-handling.md](api-error-handling.md)