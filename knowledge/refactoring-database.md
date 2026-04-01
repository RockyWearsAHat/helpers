# Database Refactoring — Evolutionary Schema Changes and Zero-Downtime Migrations

Database refactoring is fundamentally different from code refactoring: you must migrate existing data while the system continues operating. Scott Ambler and Pramod Sadalage's foundational work (2006) introduced disciplined patterns for schema evolution in production.

## Core Challenge: The Deployment Constraint

**Code can be deployed atomically** — old version out, new version in, tests verify correctness. **Databases cannot.** One system migration takes hours or days, yet the application must continue reading and writing data while the schema changes. Incompatible changes require a careful dance.

## The Expand-and-Contract Pattern

The most practical approach for zero-downtime, reversible schema changes: **expand → migrate → contract.**

### Phase 1: Expand — Introduce New Structure

```sql
-- Add new schema elements without removing old ones
ALTER TABLE users ADD COLUMN email_new VARCHAR(255);
ALTER TABLE users ADD COLUMN phone_new VARCHAR(20);

-- Application change: write to both old and new
UPDATE users SET email_new = email WHERE email_new IS NULL;
UPDATE users SET phone_new = phone WHERE phone_new IS NULL;
```

**State:** Data exists in both old and new structure. Application writes to both, reads from old.

**Advantages:**
- Easily reversible: drop new columns if problems arise.
- Old clients continue working unchanged.
- New clients can be deployed gradually (canary).

**Cost:** Temporary storage overhead, increased write load.

### Phase 2: Migrate Data (Background Job)

```sql
-- Non-blocking data migration, can run during business hours
UPDATE users SET email_new = email WHERE email_new IS NULL AND updated_at > ?;
```

Or use a separate batch job (e.g., Flyway with background tasks, `pt-online-schema-change` for MySQL):

```bash
pt-online-schema-change \
  --alter "CHANGE COLUMN email email_old VARCHAR(255), \
           RENAME AS users_backup" \
  D=mydb,t=users
```

**Key considerations:**
- Migration runs in background; application still reads old structure.
- Must handle concurrent writes from both old and new code paths.
- Idempotent: safe to restart if it fails partway through.
- For large tables, use incremental batching to avoid locking.

### Phase 3: Switch Reads to New Structure

```javascript
// Application code: now read from new columns
// SELECT email_new, phone_new FROM users
// Still writing to both
```

**State:** Application reads from new, writes to both old and new. Old clients running old code still work (reading from old, writing to old).

**Risk:** Can roll back by reverting application code, restoring reads to old structure.

### Phase 4: Stop Writing to Old Structure

```javascript
// Application only writes to new columns
// SELECT email_new, phone_new FROM users
// Only INSERT/UPDATE new columns
```

**State:** Nothing reads or writes from old structure. It's dead data.

**Warning:** Reverting from here requires restoring from a backup. Little point in rolling back.

### Phase 5: Delete Old Structure

```sql
ALTER TABLE users DROP COLUMN email, DROP COLUMN phone;
```

**State:** Schema is clean. Only new structure remains.

## Common Database Refactorings

### Rename Column

Traditional approach: causes downtime if done naively.

**Expand-and-contract:**
1. **Expand:** Add new column with desired name.
2. Populate new column from old column.
3. **Switch:** Update application to use new column.
4. **Contract:** Delete old column.

```sql
-- Phase 1: Expand
ALTER TABLE customers ADD COLUMN full_name VARCHAR(255);
UPDATE customers SET full_name = name;

-- Phase 3: Switch (app deployed)
-- SELECT full_name FROM customers

-- Phase 5: Contract
ALTER TABLE customers DROP COLUMN name;
```

### Split Column (Remove Coupling)

A column stores multiple concepts (e.g., "author_and_date"). Split into semantic pieces.

```sql
-- Phase 1: Expand
ALTER TABLE posts ADD COLUMN author_id INT;
ALTER TABLE posts ADD COLUMN created_at TIMESTAMP;

-- Phase 2: Migrate
UPDATE posts SET author_id = CAST(SUBSTRING_INDEX(author_and_date, '|', 1) AS INT),
               created_at = SUBSTRING_INDEX(author_and_date, '|', -1);

-- Phase 3: Switch
-- SELECT author_id, created_at FROM posts

-- Phase 5: Contract
ALTER TABLE posts DROP COLUMN author_and_date;
```

### Move Column (Denormalization)

Move a column from one table to another to reduce joins.

```sql
-- Phase 1: Expand
ALTER TABLE user_profiles ADD COLUMN email VARCHAR(255);

-- Phase 2: Begin dual-writing
INSERT INTO user_profiles (user_id, email)
SELECT id, email FROM users WHERE id NOT IN (SELECT user_id FROM user_profiles WHERE email IS NOT NULL);

-- Phase 3: Switch reads
-- SELECT email FROM user_profiles JOIN users ON ...

-- Phase 5: Contract
ALTER TABLE users DROP COLUMN email;
```

### Add Index (Non-blocking in Most DBs)

Create index without locking table:

```sql
-- MySQL 5.7+, PostgreSQL: full concurrency
CREATE INDEX CONCURRENTLY idx_users_email ON users(email);

-- If locking is unavoidable, do at low-traffic time
LOCK TABLE users IN SHARE MODE;
CREATE INDEX idx_users_email ON users(email);
UNLOCK TABLES;
```

## Tools and Automation

### Flyway

Java-based migration tool with support for versioning and rollback:

```javascript
// V1__initial.sql
CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(255));

// V2__expand_email.sql
ALTER TABLE users ADD COLUMN email_new VARCHAR(255);
UPDATE users SET email_new = SUBSTRING_INDEX(email, '@', 1);

// V3__contract_email.sql
ALTER TABLE users DROP COLUMN email;
ALTER TABLE users RENAME COLUMN email_new TO email;
```

Flyway enforces ordering, tracks applied migrations, and supports callbacks (pre/post migration hooks).

### Liquibase

XML/YAML-based, database-agnostic migration tool:

```yaml
databaseChangeLog:
  - changeSet:
      id: 1
      author: dev
      changes:
        - addColumn:
            tableName: users
            columns:
              - column:
                  name: email_new
                  type: VARCHAR(255)
```

### Online Schema Change Tools

**gh-ost (GitHub's tool):**
- Triggers-based approach; creates ghost table, syncs via triggers, swaps in low-downtime cut-over.
- Used at scale; handles large tables gracefully.

**pt-online-schema-change (Percona Toolkit):**
- Perl-based; similar approach, well-tested, heavy on documentation.

**Native Online DDL:**
- MySQL 8.0+, PostgreSQL 10+ support `ALGORITHM=INSTANT` or `CONCURRENTLY` clauses.
- Preferred if your DB version supports it.

## Testing Migrations

### Unit Test the Migration Script

```sql
-- Test: create table, populate, migrate, assert
BEGIN TRANSACTION;
CREATE TABLE test_users (id INT, name VARCHAR(255), email VARCHAR(255));
INSERT INTO test_users VALUES (1, 'Alice', 'alice@example.com');

ALTER TABLE test_users ADD COLUMN email_new VARCHAR(255);
UPDATE test_users SET email_new = email;

SELECT * FROM test_users;
-- Assert email_new = 'alice@example.com'

ROLLBACK;
```

### Test Backward Compatibility

Ensure old application code still works during the expand phase:

```javascript
// Old code (still in production canary)
const user = db.query("SELECT id, email FROM users WHERE id = ?", [1]);
console.assert(user.email === "alice@example.com");

// New code (gradually rolled out)
const user = db.query("SELECT id, email_new FROM users WHERE id = ?", [1]);
console.assert(user.email_new === "alice@example.com");
```

### Verify Data Integrity (Post-Migration)

Run consistency checks:

```sql
-- Example: verify all emails moved correctly
SELECT COUNT(*) as moved FROM users WHERE email_new IS NOT NULL;
SELECT COUNT(*) as original FROM users WHERE email IS NOT NULL;
-- Both should match
```

## Rollback Strategy

**Each phase is reversible except Phase 4+:**

- **After Phase 1 (Expand):** Drop new columns, reverse application deployment.
- **After Phase 2 (Migrate):** Re-migrate or repopulate from old data.
- **After Phase 3 (Switch):** Revert application to reading old columns, reverting is safe.
- **After Phase 4 (Stop Writing Old):** Data loss risk; restore from backup.
- **After Phase 5 (Contract):** Data loss; full restore required.

**Guideline:** Keep both old and new structure in production for a deprecation period (days/weeks) to allow easy rollback.

## Coordination with Application Deployment

The expand-and-contract pattern enables gradual application rollout:

```
Day 1 (10% canary):
  - New app reads/writes new columns; old app reads/writes old columns
  - Data dual-writes ensure consistency
  
Day 3 (50% canary):
  - Data still migrating in background
  - New app handles 50% of traffic
  
Day 7 (100% new app):
  - All traffic on new app
  - Begin contract: stop writing to old columns
  
Day 10 (cleanup):
  - Drop old columns
  - New schema clean
```

This gradual approach catches bugs early and allows rollback if needed.

## Anti-Patterns

- **Big bang rewrites:** One migration script that renames, splits, and moves columns in one go. High risk, hard to roll back.
- **Skipping the migration phase:** Expanding and immediately contracting without a buffer period for dual-writing. Races between old/new code.
- **No rollback plan:** Assuming migrations will succeed perfectly. Plan for failure.
- **Uncoordinated app/schema changes:** Deploying new app code before schema is ready, or vice versa. Tight coordination needed.

## State of the Art (2026)

Modern databases increasingly support online DDL:
- **PostgreSQL 14+:** Concurrent index creation, column additions without rewrites.
- **MySQL 8.0+:** `INSTANT` algorithm for instant online schema changes.
- **CockroachDB, TiDB:** Built for distributed online schema evolution.

However, expand-and-contract remains the safest, most portable pattern for multi-step, multi-instance production systems where reversibility and gradual rollout matter.