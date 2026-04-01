# Deployment & Database Migrations — Zero-Downtime Schema Evolution

## Overview

Database migrations are schema changes deployed alongside application code. The challenge: schema must be compatible with both old and new application code during deployment, avoiding downtime or data corruption.

The solution is the **expand-contract pattern**: expand the schema to support both old and new code, deploy new code, then contract (clean up) old schema. This decouples code deployment from schema cleanup, enabling zero-downtime updates.

## The Core Problem: Coupling Code and Schema

Naive workflow (causes downtime):

```
1. Stop application
2. Run migration (ALTER TABLE, CREATE INDEX, etc.)
3. Deploy new code that expects new schema
4. Start application
```

Downtime is proportional to migration duration. For large tables (100GB+), migrations can take hours.

Better workflow (zero-downtime):

```
1. Deploy new code + new schema support (both old and new formats work)
2. Migrate data gradually
3. Clean up old schema (old code paths now unused)
```

The database never goes down; old and new code coexist during the transition.

## The Expand-Contract Pattern

Three phases: Expand, Migrate, Contract.

### Phase 1: Expand

Add new schema **without removing old schema**. Both schemas exist simultaneously; old code ignores new schema; new code uses new schema.

**Example: Renaming a column from `user_name` to `username`**

Instead of:
```sql
ALTER TABLE users DROP COLUMN user_name;
ALTER TABLE users RENAME COLUMN username TO user_name;
```

Do:
```sql
-- Expand Phase: Add new column
ALTER TABLE users ADD COLUMN username VARCHAR(255);

-- Populate new column from old (backfill)
UPDATE users SET username = user_name;

-- Create index on new column (optional, but recommended)
CREATE INDEX idx_users_username ON users(username);
```

Both `user_name` and `username` now exist. Old code still uses `user_name`; new code uses `username`.

**Old application code:**
```sql
SELECT * FROM users WHERE user_name = 'alice'
```

**New application code:**
```sql
SELECT * FROM users WHERE username = 'alice'
```

Both work. No coordination needed.

### Phase 2: Migrate

Move data from old schema to new schema. This can be:

- **Synchronous:** Migration completes before deployment
- **Asynchronous:** Migration continues after deployment (online DDL, backfills)
- **Dual-write:** Application writes both old and new schema simultaneously during transition

**Synchronous (small tables < 1GB):**
```sql
UPDATE users SET username = user_name WHERE username IS NULL;
```

Runs quickly; migration is part of the deployment.

**Asynchronous (large tables > 1GB):**

Tools like **gh-ost** (GitHub's Online Schema Migration) copy data row-by-row without locking the table:

```bash
# Copy data to temporary table, apply changes, swap
gh-ost \
  --user root \
  --password secret \
  --host localhost \
  --database myapp \
  --table users \
  --alter "DROP COLUMN user_name" \
  --execute
```

While gh-ost runs:
- Old code reads/writes `user_name`
- Temporary table is gradually populated
- No table lock; traffic not affected
- After completion, tables are swapped atomically

**Dual-write pattern:**

During migration, application writes both old and new columns:

```python
def update_user(user_id, name):
    query = """
    UPDATE users 
    SET user_name = %s, username = %s 
    WHERE id = %s
    """
    db.execute(query, (name, name, user_id))
```

Both columns stay in sync; migration completes gradually as updates occur.

### Phase 3: Contract

Remove old schema after confirming new code is stable and no rollback to old code is possible.

```sql
-- Contract Phase: Remove old column
ALTER TABLE users DROP COLUMN user_name;
```

All applications must be on new code before this phase. If rollback to old code is needed, contraction must be skipped or reversed.

## Expand-Contract Examples

### Example 1: Adding a Required Column

**Goal:** Add a non-nullable `email_verified_at` timestamp to `users` table.

**Step 1: Expand**
```sql
-- Add column as nullable
ALTER TABLE users ADD COLUMN email_verified_at TIMESTAMP NULL;

-- Backfill with a default value (current time for all users)
UPDATE users SET email_verified_at = NOW();

-- Make it not null after backfill
ALTER TABLE users MODIFY COLUMN email_verified_at TIMESTAMP NOT NULL;
```

**Step 2: Migrate**

Application writes `email_verified_at` on every login or email verification.

**Step 3: Contract**

N/A; column was always intended to exist. Once code is deployed and rolled out, we're done.

### Example 2: Changing a Column Type

**Goal:** Change `user_age` from `INT` (stores age in years) to `Date` (stores birthdate).

**Step 1: Expand**
```sql
-- Add new column with desired type
ALTER TABLE users ADD COLUMN birthdate DATE NULL;

-- Backfill: convert age to approximate birthdate (e.g., age 30 → birthdate 30 years ago)
UPDATE users SET birthdate = DATE_SUB(NOW(), INTERVAL age YEAR);

-- Create index
CREATE INDEX idx_users_birthdate ON users(birthdate);
```

**Step 2: Migrate**

Application code now calculates age from birthdate:
```python
def get_age(user):
    return (today - user.birthdate).days // 365
```

Dual-write during transition (app writes both `user_age` and `birthdate`), or async backfill tool handles it.

**Step 3: Contract**
```sql
-- After all code is on new version:
ALTER TABLE users DROP COLUMN user_age;
```

### Example 3: Renaming a Table

**Goal:** Rename `user_login_logs` to `login_events`.

**Step 1: Expand (Phase 0: Create dual-write state)**

```sql
-- Create new table with new name
CREATE TABLE login_events LIKE user_login_logs;

-- Trigger: when new table receives writes, also write to old table
-- Or handled via application dual-write
```

**Step 2: Migrate**

Add dual-write to application:
```python
def log_login(user_id):
    db.insert('user_login_logs', {...})
    db.insert('login_events', {...})
```

**Step 3: Contract**

After all readers moved to `login_events` and all writers dual-write:
```sql
DROP TABLE user_login_logs;
```

## Online DDL Tools

For large tables, raw SQL ALTER TABLE commands can lock the table for minutes or hours. **Online DDL tools** use triggers and table copying to avoid locks.

### gh-ost (GitHub Online Schema Migration)

Triggerless online schema migration for MySQL. Works by:

1. Creating a temporary table with desired schema
2. Copy old table to temporary (row-by-row, can start/stop)
3. Apply DML changes (INSERT, UPDATE, DELETE) via binary log replication
4. Swap tables atomically
5. Drop old table

**Usage:**
```bash
gh-ost \
  --user=root \
  --password=secret \
  --host=localhost \
  --database=myapp \
  --table=users \
  --alter="ADD COLUMN email_verified_at TIMESTAMP NULL DEFAULT NOW()" \
  --execute
```

**Advantages:**
- No locks; table always readable/writable
- Can be paused/resumed
- Throttling support (slow down if replication lag or server load high)
- Reliable (used by GitHub at scale)

### pt-online-schema-change (Percona Toolkit)

Similar approach; uses trigger-based replication:

1. Create new table with new schema
2. Create triggers on old table to keep new table in sync
3. Copy data row-by-row
4. Swap tables once caught up

**Usage:**
```bash
pt-online-schema-change \
  --alter "ADD COLUMN email_verified_at TIMESTAMP NOT NULL DEFAULT NOW()" \
  D=myapp,t=users
```

**Difference from gh-ost:**
- Uses triggers (slightly more overhead)
- Supports cascading foreign keys
- More mature for MySQL edge cases

### Online DDL (Native MySQL 8.0+, PostgreSQL 11+)

Modern databases have built-in online DDL support.

**MySQL 8.0+:**
```sql
-- ALGORITHM=INSTANT (fastest, some operations only)
ALTER TABLE users ADD COLUMN email BIGINT, ALGORITHM=INSTANT;

-- ALGORITHM=INPLACE (no table copy, REDO log only)
ALTER TABLE users ADD INDEX idx_email (email), ALGORITHM=INPLACE, LOCK=NONE;
```

`ALGORITHM=INSTANT` (MySQL 8.0.29+) adds/removes columns without table copy or rebuilding; microsecond-scale.

**PostgreSQL 11+:**
```sql
-- Add column with default (requires rewrite, but can be deferred)
ALTER TABLE users ADD COLUMN email TEXT DEFAULT 'unknown' NOT NULL;
```

Rewriter runs asynchronously in background; table is usable.

## Advisory Locks: Coordinating Multiple Migrations

When multiple microservices share a database, migrations can conflict. Advisory locks ensure one migration runs at a time:

**PostgreSQL:**
```sql
-- At start of migration script
SELECT pg_advisory_lock(123456);  -- Acquire lock

-- Run migration (guaranteed to be alone)
ALTER TABLE users ADD COLUMN birthdate DATE;

-- At end of migration
SELECT pg_advisory_unlock(123456);  -- Release lock
```

**MySQL:**
```sql
-- Use GET_LOCK / RELEASE_LOCK
SELECT GET_LOCK('migration_lock', 600);  -- Wait up to 600s

-- Run migration

SELECT RELEASE_LOCK('migration_lock');
```

## Migration Ordering and Idempotency

### Idempotent Migrations

Each migration must be idempotent: running it twice produces the same result as running it once.

**Bad (not idempotent):**
```sql
ALTER TABLE users ADD COLUMN email VARCHAR(255);
-- If run twice, second run fails: "Column already exists"
```

**Good (idempotent):**
```sql
ALTER TABLE users ADD COLUMN IF NOT EXISTS email VARCHAR(255);
-- Safe to run multiple times
```

Tool support (Flyway, Liquibase, Alembic, Prisma) handle this automatically if written correctly.

### Ordering

Migrations are versioned and applied in order:

```
001_initial_schema.sql
002_add_users_table.sql
003_add_auth_columns.sql
004_add_email_verified_column.sql
```

Each migration runs exactly once. If migration fails, the entire deployment fails, and the schema is left in an inconsistent state (requires manual intervention).

## Migration Frameworks

### Flyway (Java, Go, Node.js)

Version control for database schemas. Migrations stored as SQL files:

```
src/main/resources/db/migration/
├── V1__Create_users_table.sql
├── V2__Add_email_column.sql
└── V3__Add_birthdate_column.sql
```

```java
Flyway flyway = Flyway.configure().dataSource(datasource).load();
flyway.migrate();  // Applies all pending migrations
```

**Strengths:**
- Simple, language-agnostic (just SQL files)
- Tracks applied migrations in schema_version table
- Deterministic; same SQL always produces same schema

**Weaknesses:**
- Only supports "up" migrations; rollback requires manual SQL
- No built-in online DDL support (must use gh-ost separately)

### Liquibase (Java, others)

XML/YAML-based migration definitions with rollback support:

```xml
<changeSet id="001" author="team">
  <createTable tableName="users">
    <column name="id" type="BIGINT">
      <constraints primaryKey="true" />
    </column>
    <column name="email" type="VARCHAR(255)" />
  </createTable>
</changeSet>

<changeSet id="002" author="team">
  <addColumn tableName="users">
    <column name="birthdate" type="DATE" />
  </addColumn>
</changeSet>
```

Liquibase generates platform-specific SQL (MySQL vs PostgreSQL) automatically.

**Strengths:**
- Supports rollback (undo a migration)
- Multi-database support (same change set generates correct SQL for all DBs)

**Weaknesses:**
- Verbose XML/YAML
- Slower startup (parses all change sets)

### Alembic (Python; SQLAlchemy)

ORM-based migrations for Python:

```python
def upgrade():
    op.add_column('users', sa.Column('email', sa.String(255)))

def downgrade():
    op.drop_column('users', 'email')
```

Less verbose than Liquibase; Python developers find it familiar.

### Prisma Migrate (Node.js; TypeScript)

Schema-first migrations. Define schema in `schema.prisma`; Prisma generates SQL:

```prisma
model User {
  id    Int     @id @default(autoincrement())
  email String  @unique
}
```

```bash
npx prisma migrate dev --name add_email_column
```

Generates migration SQL; apply to database.

**Advantage:** Single source of truth (schema); migration is derived.

**Disadvantage:** Less control over exact SQL; some complex migrations require manual SQL.

### Knex Migrations (Node.js)

JavaScript-based migrations:

```javascript
exports.up = (knex) => {
  return knex.schema.createTable('users', (table) => {
    table.increments('id');
    table.string('email').unique();
  });
};

exports.down = (knex) => {
  return knex.schema.dropTable('users');
};
```

**Strengths:**
- JavaScript-familiar
- Chainable API; readable

**Weaknesses:**
- Less control than raw SQL; edge cases require `.raw()` for SQL

## Testing Migrations

### Unit Tests

Test migrations in isolation against a test database:

```python
# Python: pytest
def test_add_email_column():
    # Run migration
    migrate()
    
    # Check schema
    columns = db.inspect.get_columns('users')
    assert any(c['name'] == 'email' for c in columns)
    
    # Check data integrity
    user_count = db.query("SELECT COUNT(*) FROM users").scalar()
    assert user_count == 1000  # Expected row count intact
```

### Integration Tests

Test migrations against a real database in CI:

1. Create test database (schema from production)
2. Run migration
3. Run application tests against new schema
4. If all pass, consider migration safe

```bash
# In CI:
docker run postgres:14
psql -h localhost -U root -d testdb < migrations/001.sql
pytest tests/  # Run app tests
```

### Rollback Tests

Ensure migrations can roll back without data loss:

```python
def test_migration_rollback():
    migrate(from=0, to=1)  # Apply migration
    assert schema_v1()
    
    rollback(from=1, to=0)  # Undo migration
    assert schema_v0()
    assert data_intact()  # Data unchanged
```

Not all frameworks support rollback (Flyway doesn't natively).

## Coordinating Deployments with Migrations

### Migrations Before Code

**Safer approach:** Deploy migrations first, then code.

```
1. Run migrations (schema is expanded/compatible)
2. Wait for replication lag to clear
3. Deploy new code (code expects new schema)
```

If code deployment fails, schema is already expanded; can roll back code and try again without re-running migrations.

### Migrations After Code

**Riskier:** Deploy code first, then migrations.

```
1. Deploy new code (code expects new schema, but it doesn't exist yet)
2. New code reads/writes based on new schema (missing columns → errors)
3. Run migrations
```

If code deployment happens before migration, there's a window where code expects a schema that doesn't exist (crashes).

### Zero-Downtime Pattern

Recommended:

```
1. Expand schema (add new columns, indexes, tables)
2. Wait N minutes for replication
3. Deploy new code + new queries
4. After stability (e.g., 1 hour), contract schema (remove old columns)
```

Small teams: combine expand + deploy in one release; delay contract to next release.

Large teams: expand → small release just for schema → code → async contract.

## Common Issues

**Issue: "Migration ran for 6 hours locking the table."**
- Cause: Used raw `ALTER TABLE` on billion-row table
- Fix: Use online DDL tool (gh-ost, pt-online-schema-change) or native online DDL

**Issue: "We rolled back code but schema is new; app crashes."**
- Cause: Didn't use expand-contract; code and schema got out of sync
- Fix: Always write code to handle both old and new schema during transition

**Issue: "Replication lag during migration caused inconsistency between replicas."**
- Cause: Ran migration too fast; replicas couldn't catch up
- Fix: Use online DDL with throttling; monitor replication lag; don't contract until replicas caught up

**Issue: "We can't run the migration twice; second run errors."**
- Cause: Migrations are not idempotent
- Fix: Use `IF NOT EXISTS`, `IF NOT NULL`, etc. Frameworks like Flyway track migration state

## See Also

- **deployment-strategies-deep.md** — Blue-green, canary, rolling deployments
- **devops-cicd-patterns.md** — CI/CD integration for migrations
- **database-migrations.md** — General database migration concepts
- **database-patterns.md** — Schema design patterns that enable migrations