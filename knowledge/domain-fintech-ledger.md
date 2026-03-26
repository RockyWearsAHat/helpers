# Accounting for Developers: Double-Entry Bookkeeping, Ledger Patterns & Precision

## Overview

Double-entry bookkeeping is a 500-year-old paradigm that underpins all financial systems. Every transaction affects two accounts: a **debit** (reduction in right side, increase in left side) and a **credit** (increase in right side, reduction in left side). For developers, understanding ledger mechanics is essential for building payment systems, billing platforms, and financial applications.

**Fundamental equation:** Assets = Liabilities + Equity

## Double-Entry Bookkeeping Fundamentals

### Debits, Credits & Account Types

The terms "debit" and "credit" mean different things depending on account type. The sign convention comes from the balance sheet perspective:

| Account Type  | Debit Is        | Credit Is      |
|---------------|-----------------|----------------|
| Asset         | Increase (+)    | Decrease (-)   |
| Liability     | Decrease (-)    | Increase (+)   |
| Equity        | Decrease (-)    | Increase (+)   |
| Revenue       | Decrease (-)    | Increase (+)   |
| Expense       | Increase (+)    | Decrease (-)   |

**Contra accounts** (Asset-reduction like Allowance for Doubtful Accounts) flip the sign: credit increases the contra account, reducing the main asset.

**Intuition:** The balance sheet lives on the right. Assets are "on the left" mentality. A debit to Assets increases them (moving toward balance sheet); a credit to Liabilities increases them (moving toward balance sheet). Everything balances because debits equal credits.

### The Fundamental Rule

Every journal entry has at least one debit and one credit, and the sum of debits equals the sum of credits:

```
Debit: Cash Account          1000
  Credit: Revenue Account            1000
```

Money in (debit to Cash, a left-side increase) equals money out (credit to Revenue, a right-side increase). The balance sheet still balances.

### Chart of Accounts

A hierarchical list of all accounts the organization uses. Example structure:

```
Assets
  1000 Cash
    1100 Checking Account
    1200 Savings Account
  1300 Accounts Receivable
  1500 Inventory
Liabilities
  2000 Accounts Payable
  2100 Credit Card Payable
Equity
  3000 Owner's Capital
Revenue
  4000 Sales Revenue
    4100 Product Sales
    4200 Service Revenue
Expenses
  5000 Cost of Goods Sold
  5100 Salaries
  5200 Utilities
```

Each account has a numeric code (sometimes alpha-numeric), allowing programmatic queries like "sum all expenses in the 5000 range."

## Journal Entries & Posting

### Transaction Recording

**Journal** logs transactions in chronological order, before posting to ledgers. Each entry includes:
- Date
- Description
- Debit account(s) and amount
- Credit account(s) and amount
- Reference ID (invoice number, payment ID, etc.)

```
Date: 2024-03-15
Description: Sale to Customer ABC
Reference: Invoice #2024-001
Debit: Accounts Receivable  500
  Credit: Sales Revenue            500
```

### Posting to the Ledger

After journaling, entries are posted (moved) to individual account ledgers. The **General Ledger** contains all accounts:

```
Account: Sales Revenue (4000)
Date        | Description      | Debit | Credit | Balance
2024-03-15  | Invoice #2024-001|       |   500  |   500
2024-03-20  | Invoice #2024-002|       |  1200  |  1700
```

Posting creates a running balance for each account, essential for financial reporting.

### Idempotent Postings

**Critical for fault tolerance:** Posting must be idempotent. If a posting fails halfway (hardware failure, network outage), rerunning must not double-post.

**Pattern:**
1. Generate posting ID (unique per journal entry)
2. Check if posting ID already posted (query ledger)
3. If not posted, post the entry and record posting ID
4. If already posted, return cached result

Requires a `postings_log` table tracking post IDs and timestamps, preventing duplicate ledger entries.

## Trial Balance, Reconciliation & Closing

### Trial Balance

Sum all debits and all credits in the general ledger. Should equal:

```
Total Debits == Total Credits
```

If not, there's a transaction recording error. Used as an audit checkpoint before closing the accounting period.

### Reconciliation

Match accounts against external sources:
- **Cash reconciliation:** Bank statement vs. cash ledger account
- **AR reconciliation:** Customer account vs. customer payment records
- **Inventory reconciliation:** Physical count vs. inventory ledger

### Period Closing

At end of month/quarter/year:
1. Post all accruals (invoice for unshipped goods, accrue utilities used but not yet billed)
2. Run trial balance; fix errors
3. Post closing entries (transfer revenue/expense to retained earnings, zero out P&L for next period)
4. Generate financial statements (P&L, Balance Sheet)
5. Lock period (no further journal entries in that period without audit trail)

## Event-Sourced & Immutable Ledgers

### Immutability Pattern

Traditional accounting forbids deletion. If a posting is wrong, you reverse it (create opposite entry), never delete. This creates audit trail.

**Immutable ledger pattern:**

```
events (immutable log)
  id, timestamp, type, account, amount, debit/credit, source_id, description

accounts (derived state)
  account_id, balance (computed from events)
```

Every change is an append-only event. Account balances are computed by aggregating events. No transaction ever overwrites or deletes.

**Benefits:**
- **Complete audit trail:** Every change has intent, timestamp, source
- **Temporal queries:** "What was a customer's balance on March 1?"—replay events up to that date
- **Debugging:** Trace exactly how a balance reached current state
- **Reversals:** Create opposite event; recompute balances

### Event Sourcing for Accounting

An order lifecycle generates multiple accounting events:

```
order.created: 2024-03-15
  Debit: Accounts Receivable 5000
    Credit: Sales Revenue        5000

order.partially_refunded: 2024-03-20 (refund $1000)
  Debit: Sales Revenue        1000
    Credit: Accounts Receivable    1000

order.shipped: 2024-03-16
  Debit: Cost of Goods Sold   2000
    Credit: Inventory             2000
```

Each event is idempotent (event ID is unique; reprocessing same event twice doesn't double-post). Snapshots (computed state at a point) accelerate queries.

## Precision & Floating-Point Pitfalls

### The Problem

Floating-point arithmetic is approximate, not exact. 0.1 + 0.2 != 0.3 in binary floating-point.

```
double x = 0.1 + 0.2;  // 0.30000000000000004, not 0.3
```

In financial systems, this ruins reconciliation. $100.00 might be stored as $99.99999999 due to rounding errors.

### Solutions

1. **Use fixed-point decimals, not floats:**
   ```python
   from decimal import Decimal
   price = Decimal("9.99")
   quantity = Decimal("3")
   total = price * quantity  # Decimal("29.97"), exact
   ```

2. **Store amounts as integers (cents):**
   ```
   amount_cents = 999  # represents $9.99
   // arithmetic on integers, then divide by 100 for display
   ```

3. **Use big decimal libraries (Java BigDecimal, PostgreSQL numeric):**
   ```
   SELECT amount::numeric(19, 4) FROM transactions;  // 19 digits, 4 after decimal
   ```

4. **Database choice matters:**
   - **PostgreSQL:** `numeric` type for exact arithmetic
   - **MySQL:** `decimal(19, 4)` type
   - **DynamoDB:** Store as string or integer (cents)
   - **SQLite:** Treat integers as cents

5. **Rounding rules:** Banks use specific rounding (typically "round half to even" / banker's rounding) for fairness. Document and apply consistently.

## Multi-Currency Ledgers

### Challenges

- Each transaction recorded in transaction currency (USD, EUR, etc.)
- Reporting often required in home currency (e.g., USD for US company)
- Exchange rates fluctuate; historical rates needed
- FX gains/losses are accounting entries themselves

### Pattern: Record in Both Currencies

```
Debit: Cash (USD)         1000
  Credit: Sales Revenue (USD)      1000

Debit: Cash (EUR)         850   (at today's rate, 1 USD = 0.85 EUR)
  Credit: Sales Revenue (EUR)      850

Debit: FX Gain (USD)      25    (FX difference, if applicable)
  Credit: FX Gain (EUR)           25
```

More commonly, record in transaction currency and translate for reporting:

```
Debit: Cash (EUR)         850
  Credit: Sales Revenue (EUR)      850

// During reporting (convert to USD at historical rate)
Cash balance in EUR: 850
EUR/USD rate on 2024-03-15: 1.176
Cash in USD equivalent: 850 * 1.176 = 999.60
```

### FX Accounting

When converting liabilities/assets to reporting currency, differences create **realized** (on settled transactions) and **unrealized** (on unsettled) gains/losses.

```
Original: Owe vendor 1000 EUR
EUR/USD rate when incurred: 1.10 → $1100 USD equivalent
EUR/USD rate today: 1.15 → $1150 USD equivalent
Unrealized FX loss: $50 (EUR appreciated, owe more in USD)
```

Accruals debit FX Gain/Loss account; settled transactions realize gains/losses.

## Ledger Databases & Design

### Traditional SQL Approach

```sql
CREATE TABLE accounts (
  id INTEGER PRIMARY KEY,
  code VARCHAR(20) UNIQUE,
  name VARCHAR(100),
  account_type VARCHAR(20), -- asset, liability, equity, revenue, expense
  balance NUMERIC(19, 4) DEFAULT 0
);

CREATE TABLE journal_entries (
  id BIGINT PRIMARY KEY,
  transaction_date DATE,
  description TEXT,
  created_at TIMESTAMP,
  posted BOOLEAN DEFAULT FALSE,
  posted_at TIMESTAMP
);

CREATE TABLE postings (
  id BIGINT PRIMARY KEY,
  journal_entry_id BIGINT REFERENCES journal_entries,
  account_id INTEGER REFERENCES accounts,
  debit NUMERIC(19, 4),
  credit NUMERIC(19, 4),
  posting_id VARCHAR(50) UNIQUE, -- idempotency key
  created_at TIMESTAMP
);

CREATE VIEW account_balances AS
SELECT 
  a.id,
  a.code,
  SUM(COALESCE(p.debit, 0)) - SUM(COALESCE(p.credit, 0)) as balance
FROM accounts a
LEFT JOIN postings p ON a.id = p.account_id
GROUP BY a.id;
```

**Indexes:** journal_entries(posted_at), postings(account_id, created_at), postings(posting_id).

### Event-Sourced Approach

```sql
CREATE TABLE ledger_events (
  id BIGINT PRIMARY KEY,
  event_id VARCHAR(50) UNIQUE, -- idempotency key
  event_type VARCHAR(50), -- posting, reversal, adjustment
  account_id INTEGER,
  amount NUMERIC(19, 4),
  is_debit BOOLEAN,
  transaction_date DATE,
  created_at TIMESTAMP,
  source_id VARCHAR(100), -- order_id, invoice_id, etc.
  description TEXT
);

CREATE MATERIALIZED VIEW account_balances AS
SELECT 
  account_id,
  SUM(CASE WHEN is_debit THEN amount ELSE -amount END) as current_balance,
  MAX(created_at) as last_event_at
FROM ledger_events
GROUP BY account_id;
```

Query any historical balance:

```sql
SELECT 
  account_id,
  SUM(CASE WHEN is_debit THEN amount ELSE -amount END) as balance_on_date
FROM ledger_events
WHERE created_at <= '2024-01-31'
GROUP BY account_id;
```

## Real-World Patterns

### Holds & Releases (Escrow)

For marketplace or order processing:
- Customer places order → Debit: Held Funds (asset), Credit: Customer Balance (liability)
- Order ships & completes → Debit: Merchant Revenue (equity), Credit: Held Funds (release hold)

If order cancelled:
- Debit: Customer Balance (liability), Credit: Held Funds (asset)

Held Funds is a temporary account tracking money in transit between buyer and seller.

### Accounting for Revenue Recognition

Under accrual accounting (ASC 606), revenue is recognized when performance obligation is satisfied, not when cash is received:

```
Invoice sent (not yet paid):
  Debit: Accounts Receivable  1000
    Credit: Revenue              1000

Payment received (weeks later):
  Debit: Cash                 1000
    Credit: Accounts Receivable   1000
```

AR is matched to revenue; cash is separate. This matches economics (had we shipped, revenue was earned) to accounting.

### Intercompany Transactions

Holding company with subsidiaries needs intercompany elimination:
- Sub A invoices Sub B for tooling: Debit B's Expense, Credit A's Revenue
- Consolidating statements: eliminate both entries (they net to zero across company)

Requires careful tagging and elimination journal entries during consolidation.

## See Also

- [Event Sourcing — Event Store Design, Projections & Temporal Queries](knowledge/architecture-event-sourcing.md) — ledger as event store
- [Architecture — CQRS (Command Query Responsibility Segregation)](knowledge/architecture-cqrs.md) — separating write (postings) from read (balances)
- [Payment Systems: Architecture, Processing & Regulation](knowledge/domain-fintech-payments.md) — payment-specific ledger patterns
- [Distributed Clocks & Ordering — Time, Causality, and Total Order](knowledge/distributed-clocks-ordering.md) — ordering postings across systems