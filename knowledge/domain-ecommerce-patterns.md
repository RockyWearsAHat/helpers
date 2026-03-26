# E-Commerce Application Patterns: Cart, Inventory, Checkout & Order Lifecycle

## Overview

E-commerce systems orchestrate product discovery, shopping cart management, inventory reservation, pricing, tax calculation, checkout, and order fulfillment. Understanding these patterns prevents race conditions, inventory corruption, payment failures, and tax compliance issues.

## Cart Management

### Stateless Cart (Token-Based)

Cart contents stored server-side, referenced by session token. User receives token (JWT, opaque ID), submits it with cart operations.

```
POST /cart/add
{
  "cart_token": "ABC123",
  "product_id": "SKU-456",
  "quantity": 2
}

// Server returns updated cart
{
  "cart_token": "ABC123",
  "items": [
    {"product_id": "SKU-456", "quantity": 2, "price": 29.99}
  ],
  "subtotal": 59.98
}
```

**Storage:** Redis hash (fast reads/writes), or database (PostgreSQL JSONB).

**TTL:** Session expires (e.g., 30 days inactivity). Abandoned carts tracked for recovery campaigns.

**Benefits:** Secure (no client-side manipulation), supports real-time price updates, integrates inventory checks.

**Tradeoffs:** Server-side state; requires session management.

### Client-Side Cart (Encrypted Token)

Cart encoded/encrypted on client, submitted with each request. Server decrypts, validates, proceeds.

```
GET /checkout?cart=eyJ...encrypted_base64...
```

**Benefits:** Stateless (no server-side cart storage), can scale horizontally.

**Tradeoffs:** Cart data exposed in URL/logs; must encrypt/sign to prevent tampering; client can forget cart (no persistence on server).

**Hybrid:** Store in both client and server; use client copy for UX, server as source of truth.

### Cart Persistence Across Sessions

Cart persisted in database (`user_id` → cart items), not session. User logs in, cart reappears.

```sql
CREATE TABLE shopping_carts (
  user_id INTEGER PRIMARY KEY,
  items JSONB,
  subtotal NUMERIC(19, 4),
  updated_at TIMESTAMP
);
```

**Benefits:** Cart survives logout/browser close; synced across devices.

**Tradeoffs:** Requires user account; deletes are async (items expire after X inactivity).

## Inventory Reservation & Allocation

### Reservation State Machine

```
Available -> Reserved -> Confirmed -> Fulfilled
          \__________ Cancelled
```

**Available:** Inventory can be sold.

**Reserved:** Customer added to cart; inventory held but not committed. If cart abandoned or checkout fails, reverts to Available.

**Confirmed:** Payment succeeded; inventory committed. Moves to fulfillment queue.

**Fulfilled:** Shipped to customer. Final state (or returned → Available again).

### Race Condition Problem

Without locking, two customers can both purchase the last item:

```
Customer A: Check inventory (5 left), add to cart (still shows 5)
Customer B: Check inventory (5 left), add to cart (still shows 5)
Customer A: Checkout, succeeds
Customer B: Checkout, SHOULD fail but might succeed (race)
```

### Solutions

**1. Optimistic Locking (Version-Based)**

```sql
UPDATE inventory SET quantity = quantity - 1, version = version + 1
WHERE product_id = 'SKU-123' AND quantity > 0 AND version = 42;
```

If version no longer 42 (another client updated), update fails. Client retries, re-reads current quantity.

**2. Pessimistic Locking (Row Lock)**

```sql
BEGIN TRANSACTION;
SELECT * FROM inventory WHERE product_id = 'SKU-123' FOR UPDATE; -- locks row
IF quantity > 0 THEN
  UPDATE inventory SET quantity = quantity - 1 WHERE product_id = 'SKU-123';
ELSE
  RAISE EXCEPTION 'Out of stock';
END IF;
COMMIT;
```

Lock held until transaction commits. Serializes all updates to this product.

**Tradeoff:** Optimistic assumes conflicts rare (high concurrency, OK to retry); pessimistic assumes conflicts frequent (low throughput but consistent).

**3. Event-Streaming Approach**

All reservation attempts published as events. Single consumer (event handler) processes sequentially:

```
Event: ReservationRequested(order_123, SKU-456, qty 2, timestamp)
Handler:
  - Read current inventory (1 row, no lock)
  - If available, emit ReservedConfirmed & deduct inventory
  - If unavailable, emit ReservationFailed
```

Single handler ensures no race; other requests queue. Slower but simpler consistency model.

### Deallocation (Expiring Reservations)

Reserved inventory expires if not confirmed:

```sql
-- Nightly job
UPDATE inventory SET quantity = quantity + reserved_qty
WHERE product_id = 'SKU-123'
  AND reserved_at < NOW() - INTERVAL '1 hour'
  AND status = 'RESERVED';
```

Alternatively, reservation rows with expiration timestamps; background job cleans up expired rows and returns inventory to available pool.

## Order Lifecycle & State Machine

### Typical States

```
PENDING -> CONFIRMED -> PROCESSING -> SHIPPED -> DELIVERED
       \-> FAILED
       \-> CANCELLED -> REFUND_IN_PROGRESS -> REFUNDED
```

**PENDING:** Order created; payment not yet authorized.

**CONFIRMED:** Payment authorized (but not captured). Inventory reserved.

**PROCESSING:** Payment captured; preparing shipment.

**SHIPPED:** Carrier in possession; tracking provided to customer.

**DELIVERED:** Delivery confirmed by carrier or customer receipt.

**FAILED:** Payment declined; order will not proceed.

**CANCELLED:** Customer or system cancelled; inventory released, refund queued.

**REFUNDED:** Refund settled to customer's original payment method.

### State Transitions

Valid transitions (what's allowed from each state):

```
PENDING -> {CONFIRMED, FAILED}
CONFIRMED -> {PROCESSING, CANCELLED}
PROCESSING -> {SHIPPED, FAILED}
SHIPPED -> {DELIVERED, FAILED}
DELIVERED -> {REFUND_IN_PROGRESS}
CANCELLED -> {REFUND_IN_PROGRESS, COMPLETED}
REFUND_IN_PROGRESS -> {REFUNDED}
REFUNDED -> {COMPLETED}
FAILED -> {COMPLETED, RETRY}
```

**Immutability:** Once transitioned, previous state is logged but not reachable. Prevents "undo shipped" after delivery started.

### Order Idempotency

Order creation must be idempotent. If checkout request fails and user retries, second attempt should return same order, not create duplicate.

```
POST /orders
{
  "idempotency_key": "checkout-session-ABC-123",
  "items": [{...}],
  "shipping": {...}
}

// Server generates order ID, caches result with idempotency key
// If same key arrives again, return cached order instead of creating new
```

Database unique constraint on idempotency key prevents accidental duplicates.

## Pricing & Discount Systems

### Price Hierarchy

Prices can come from multiple sources (highest priority wins):

1. **Promotional override** (e.g., sale ends at 5pm, $5 off)
2. **Tiered volume pricing** (buy 10+, get 5% off)
3. **Customer segment pricing** (B2B customers get 20% off)
4. **Catalog price** (default list price)

At checkout, calculate in order; stop at first match.

### Coupons & Promo Codes

```sql
CREATE TABLE promotional_codes (
  code VARCHAR(20) PRIMARY KEY,
  discount_type VARCHAR(20), -- PERCENTAGE, FIXED_AMOUNT, BOGO, FREE_SHIPPING
  discount_value NUMERIC(19, 4),
  max_uses INTEGER,
  uses_remaining INTEGER,
  valid_from TIMESTAMP,
  valid_until TIMESTAMP,
  min_cart_value NUMERIC(19, 4),
  applicable_categories TEXT[] -- restrict to product categories
);

CREATE TABLE coupon_usage (
  coupon_id VARCHAR(20) REFERENCES promotional_codes,
  order_id BIGINT,
  used_at TIMESTAMP,
  discount_amount NUMERIC(19, 4)
);
```

At checkout:
1. Validate code (exists, not expired, not exhausted uses, min cart met, applicable to items)
2. Calculate discount
3. Increment uses_remaining (or decrement, depending on tracking)
4. Emit event (coupon_redeemed) for analytics
5. If redemption fails during final commit, no discount applied (fail safe)

### Multi-Discount Logic

When multiple discounts apply, order matters:

- **Stacking:** Discount 1 applied to full price, then Discount 2 applied to discounted price
- **Best-of:** Apply whichever discount is largest, not both
- **Prohibited combinations:** Some promos exclude others (e.g., can't stack two 20% coupons)

Document and test discount logic carefully; unintended stacking can destroy margins.

## Tax Calculation

### Tax Nexus & Rate Lookup

Tax applies based on:
- **Buying state/province** (physical location)
- **Product category** (software, digital, physical, food—different rates)
- **Customer type** (consumer, business exempt, reseller exempt)

Lookup table: (buying state, product category) → tax rate

```
(California, Physical Goods) -> 8.25%
(California, Digital Download) -> 0% (exempt in CA)
(Texas, Physical Goods) -> 8.25%
(Oregon, Physical Goods) -> 0% (no state sales tax in OR)
```

Use third-party tax APIs (TaxJar, Avalara, TaxRate.com) for live rates and compliance.

### Tax-Inclusive vs. Tax-Exclusive Pricing

**Tax-exclusive** (US, most countries):
```
Item: $100
Tax (8%): $8
Total: $108
```

**Tax-inclusive** (EU, AU, many others):
```
Item: $108 (includes 20% VAT)
Tax: $18
Price ex tax: $90
```

At checkout, clearly display what's included; different regions have different expectations.

### Handling Tax Rounding

With multicurrency and tiered items, rounding errors accumulate:

```
Item 1: $10.00 -> tax $0.8166... (rounds to $0.82)
Item 2: $10.00 -> tax $0.8166... (rounds to $0.82)
Item 3: $10.00 -> tax $0.8166... (rounds to $0.82)

Sum: $30 + $2.46 = $32.46
True tax: $30 * 0.08166... = $2.45

Off by $0.01 per 3 items
```

Solutions:
1. **Round per-item** (above), accept rounding variance
2. **Round at total** (calculate all tax, round once at end)
3. **Tax-exclusive ledger** (track tax as separate component, reconcile at settlement)

Most systems use option 2 (round at total) to minimize drift.

## Checkout Flow & Payment

### Typical Flow Sequence

1. **Cart review:** User validates items, quantities, applies coupon
2. **Shipping address:** User enters delivery address (can differ from billing)
3. **Shipping method:** User chooses (standard, express, overnight), system calculates shipping cost
4. **Tax calculation:** System calculates tax based on shipping address + item types
5. **Billing address:** User enters (or same as shipping)
6. **Payment method:** Card, PayPal, Apple Pay, etc.
7. **Order review:** Summary of items, taxes, shipping, total
8. **Payment authorization:** Submit payment; wait for approval
9. **Order confirmation:** Confirmation page + email; order moves to fulfillment

### Abandoned Cart Recovery

Cart abandoned if not completed within threshold (e.g., 1 hour). Send reminders:

1. **1 hour after abandonment:** Email with cart link + "Complete your purchase"
2. **24 hours after:** Second email with incentive (5% off)
3. **3 days after:** Final email + product reviews/social proof

Track **cart recovery rate** (abandoned carts completed): high-value metric.

```sql
SELECT 
  COUNT(*) FILTER (WHERE completed) as recovered,
  COUNT(*) as abandoned,
  100.0 * COUNT(*) FILTER (WHERE completed) / COUNT(*) as recovery_rate
FROM shopping_carts
WHERE created_at > NOW() - INTERVAL '30 days'
  AND abandoned = true;
```

### Payment Failures & Retries

Payment can fail for multiple reasons:
- Insufficient funds (user-correctable)
- Card expired (user-correctable)
- Fraud block (may auto-clear after manual review)
- Processor temporarily down (retry)

Strategy:
1. **First attempt:** Standard auth/capture
2. **Second attempt (1 hour later):** Retry (account holds often clear)
3. **Third attempt (24 hours later):** Manual review or alternative payment method
4. **After 3 failures:** Mark order failed, release inventory, notify customer

Use webhooks (payment provider notifies you of async status changes) rather than polling.

## Product Catalog Modeling

### Product Variants

Product often has variants (size, color, material):

```sql
CREATE TABLE products (
  id BIGINT PRIMARY KEY,
  sku VARCHAR(50) UNIQUE,
  name VARCHAR(255),
  description TEXT,
  category_id INTEGER REFERENCES categories
);

CREATE TABLE product_variants (
  id BIGINT PRIMARY KEY,
  product_id BIGINT REFERENCES products,
  sku VARCHAR(50) UNIQUE,
  size VARCHAR(20),
  color VARCHAR(20),
  material VARCHAR(50),
  price NUMERIC(19, 4),
  inventory_count INTEGER
);
```

When adding to cart, store variant_id, not product_id. Pricing and inventory tied to variant.

### Attributes & Faceting

Variants represented as attributes for filtering/search:

```
product_id: 123
name: "T-Shirt"
attributes: [
  {name: "size", values: ["XS", "S", "M", "L", "XL"]},
  {name: "color", values: ["Red", "Blue", "Green"]},
  {name: "material", values: ["Cotton", "Polyester"]},
]
```

At search, facets (size, color, material) allow users to filter: "Show me red cotton T-shirts in size M."

### Inventory States

Product/variant can have multiple inventory states:

- **In Stock:** Available for immediate purchase
- **Low Stock:** Available but below reorder threshold
- **Pre-Order:** Not yet available; customers can pre-order
- **Out of Stock:** Temporarily unavailable; backorder enabled
- **Discontinued:** No longer sold; remove from catalog

Different states affect checkout options (pre-order has different fulfillment timeline).

## Analytics & Metrics

### Key E-Commerce Metrics

| Metric | Formula |
|--------|---------|
| **Conversion Rate** | Completed Orders / Sessions × 100% |
| **Average Order Value (AOV)** | Total Revenue / Completed Orders |
| **Cart Abandonment Rate** | Abandoned Carts / Created Carts × 100% |
| **Return Rate** | Returned Items / Shipped Items × 100% |
| **Customer Lifetime Value (CLV)** | Sum of all profits from customer purchases |
| **Cost Per Acquisition (CPA)** | Marketing Spend / New Customers |

### Checkout Funnel

Track drop-off at each step:

```
1000 Sessions
  850 Browse products (15% drop)
  800 Add to cart (5% drop)
  600 Start checkout (25% drop)
  550 Enter shipping (8% drop)
  520 Enter payment (5% drop)
  500 Complete order (4% drop, 50% conversion)
```

Identify where users escape (payment step has highest drop?) and optimize.

## See Also

- [Architecture — State Machines](knowledge/architecture-state-machines.md) — order state modeling
- [Accounting for Developers: Double-Entry Bookkeeping](knowledge/domain-fintech-ledger.md) — revenue recognition, AR tracking
- [Payment Systems: Architecture, Processing & Regulation](knowledge/domain-fintech-payments.md) — payment processing in checkout
- [API Design Principles](knowledge/api-design.md) — idempotent cart/order APIs
- [Database — Distributed Transactions & Consensus](knowledge/database-distributed-txns.md) — inventory reservation across shards