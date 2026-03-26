# Database Migrations — Schema Versioning, Zero-Downtime, & DevOps Patterns

## Overview

Database migrations are versioned, ordered changes to database schemas deployed alongside application code. Modern migration strategies decouple deployment from release, enable rollbacks, and maintain backward compatibility between old and new application versions running simultaneously. The DevOps challenge: coordinate migrations across distributed systems without downtime or data loss.

## Schema Migration Tools

### Flyway (JVM / SQL-first)

Convention-based: versioned SQL files define migrations.

```
db/migration/
├── V1__create_users.sql     # Up migration
├── V1__create_users.undo.sql # Down (undo)
├── V2__add_email.sql
└── R__refresh_views.sql      # Repeatable (runs on change)
```

```sql
-- V1__create_users.sql
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(100) NOT NULL UNIQUE,
    created_at TIMESTAMP DEFAULT now()
);
```

Flyway tracks applied migrations in `flyway_schema_history`. Advantages: zero-config philosophy; directly runs SQL; minimal runtime overhead. Limitations: SQL-only; renaming columns requires expand-contract boilerplate.

### Liquibase (JVM / Multi-language)

XML-, YAML-, or SQL-based; generates database-specific SQL from abstract changesets.

```yaml
databaseChangeLog:
  - changeSet:
      id: "1"
      author: "alice"
      changes:
        - createTable:
            tableName: users
            columns:
              - column:
                  name: id
                  type: SERIAL
                  constraints:
                    primaryKey: true
        - addColumn:
            tableName: users
            columns:
              - column:
                  name: email
                  type: VARCHAR(255)
                  constraints:
                    unique: true
```

Liquibase generates SQL for target DB (PostgreSQL, MySQL, Oracle, etc.). Supports rollbacks and preconditions. Trade-off: more abstraction means less visibility into generated SQL; schema drifts harder to debug.

### Alembic (Python / SQLAlchemy)

Python tool for SQLAlchemy ORM. Generates migration scripts; can auto-detect schema changes.

```python
# alembic/versions/001_create_users.py
from alembic import op
import sqlalchemy as sa

revision = '001'
down_revision = None

def upgrade():
    op.create_table('users',
        sa.Column('id', sa.Integer, primary_key=True),
        sa.Column('username', sa.String(100), unique=True),
        sa.Column('created_at', sa.DateTime, default=sa.func.now())
    )

def downgrade():
    op.drop_table('users')
```

Alembic strengths:
- Native Python; integrates with SQLAlchemy models
- Auto-generates migration skeletons from ORM changes
- Supports linear or branching migration histories

Limitations: auto-generation misses renamed columns, constraint changes; requires manual review.

### Prisma Migrate (Node.js / TypeScript)

ORM-driven migrations. Schema definition is declarative; migrations auto-generated.

```prisma
// schema.prisma
model User {
  id        Int     @id @default(autoincrement())
  username  String  @unique
  email     String
  createdAt DateTime @default(now())
}
```

```bash
npx prisma migrate dev --name create_users
# Generates: prisma/migrations/TIMESTAMP_create_users/migration.sql
```

Prisma strengths:
- Schema as source of truth; migrations derived automatically
- Type-safe queries from generated Prisma Client
- Built-in shadow DB for testing migrations locally

Weaknesses: opinionated patterns; less flexible for complex DDL; shadow DB overhead.

## Zero-Downtime Migrations: Expand-Contract Pattern

**Traditional migration problem:** Rename `email_address` → `email`. Application using old column name breaks immediately after migration runs.

**Solution: expand-contract pattern**—apply schema changes in phases so old and new app versions coexist.

### Phase 1: Expand (Add New Column)

```sql
-- Migration 001
ALTER TABLE users ADD COLUMN email VARCHAR(255);
```

**State:** Old and new columns both present. Old app works unchanged. Migration script copies data:

```sql
UPDATE users SET email = email_address WHERE email IS NULL;
```

### Phase 2: Parallel Deployment Window

- Deploy new app version reading/writing `email`, falling back to `email_address`
- Monitor logs; verify new code working
- Optionally, run for a release before removing old column

### Phase 3: Contract (Drop Old Column)

```sql
-- Migration 002
ALTER TABLE users DROP COLUMN email_address;
```

**State:** Only new column exists. Old app cannot deploy; but if new app fails, rollback to previous version is safe (expanded schema still compatible).

### Backward Compatible Changes

| Change | Approach | Downtime |
|--------|----------|----------|
| Add column with default/nullable | Direct DDL | None |
| Add index | Direct DDL (online in most DBs) | None |
| Rename column | Expand-contract (alias, copy, drop) | None |
| Change column type (e.g., INT → BIGINT) | Expand-contract if used in calculations | Conditional |
| Drop column | Expand (wait 1 release) then contract | None |
| Add NOT NULL constraint | Expand (add col), backfill, contract (add constraint) | Staged |

**Rule:** Never drop, rename, or constrain columns in a single migration if old app versions may still run.

## Rollback Strategies

### Down Migrations

Every up migration should have a corresponding down:

```sql
-- V2__add_email.sql (up)
ALTER TABLE users ADD COLUMN email VARCHAR(255);

-- V2__add_email.undo.sql (down)
ALTER TABLE users DROP COLUMN email;
```

**Limitation:** Rollbacks are destructive if data was added. Cannot reliably undo `INSERT` or `UPDATE` statements.

### Compensating Transactions

For data migrations, use compensating operations instead of down migrations:

```sql
-- Migration: Backfill manager_id for orphaned users
BEGIN;
UPDATE users SET manager_id = (SELECT id FROM users LIMIT 1)
  WHERE manager_id IS NULL;
COMMIT;

-- If needed: Rollback is a revert migration
BEGIN;
UPDATE users SET manager_id = NULL WHERE created_at > '2025-01-01';
COMMIT;
```

### Staging Rollbacks

Test rollback procedure on production-like data **before** deploying migration:

```bash
# Clone production snapshot
pg_dump prod_db | psql preview_db

# Apply migration, run tests
flyway migrate

# Rollback, verify state
flyway undo  # If supported
# OR: manually apply down migration

# Validate data integrity
SELECT COUNT(*) FROM users WHERE ...
```

## Data Migration Patterns

### Bulk Data Transformation

Migrate data in batches to avoid locking entire table:

```sql
-- Migrate 10K rows at a time (PostgreSQL)
DO $$
BEGIN
  FOR i IN 1..100 LOOP
    UPDATE users SET status = 'ACTIVE' WHERE status = 'PENDING'
      LIMIT 10000;
    COMMIT;
    PERFORM pg_sleep(0.1);  -- Pause 100ms between batches
  END LOOP;
END $$;
```

Batching reduces lock contention; sleep periods prevent query queue buildup.

### Offline Migration Windows

For massive reshuffles, schedule maintenance window:

1. Set application read-only (return 503)
2. Drain existing connections
3. Run expensive migration: `ALTER TABLE large_table REORGANIZE` (MySQL)
4. Resume reads/writes
5. Log total downtime

**Trade-off:** Simplest approach; acceptable for low-traffic apps or off-peak windows. Unacceptable for SaaS systems that must always be up.

## Migration Testing Strategies

### Local Testing

Test migrations on replica of production schema:

```bash
# Create test DB from production snapshot
pg_dump -d prod_db -F d -j 4 | pg_restore -d test_db

# Apply migrations
flyway -url=jdbc:postgresql://localhost/test_db migrate

# Verify application on test DB
npm test -- --database=test_db
```

### Shadow Database (Prisma, etc.)

Tools that support shadow databases run migrations twice during development:

```bash
npx prisma migrate dev  # Applies to main dev DB + shadow DB

# Shadow DB used to detect issues without affecting dev DB
# Reset shadow DB if migration fails
npx prisma migrate resolve --rolled-back "timestamp_name"
```

### CI Pipeline Testing

```yaml
# .github/workflows/test-migrations.yml
jobs:
  test-migrations:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
    steps:
      - uses: actions/checkout@v4
      - name: Run migrations on fresh DB
        run: flyway -url=jdbc:postgresql://postgres:5432/testdb -user=postgres migrate
      - name: Run integration tests
        run: npm test
      - name: Test rollback
        run: flyway undo  # Or apply compensating migration
      - name: Verify data integrity
        run: npm run verify-db-integrity
```

### Dirty Database Handling

Migrations can fail mid-execution (network timeout, constraint violation, OOM). State becomes "dirty":

```bash
flyway info  # STATUS: Pending, Failed, Success, Undone

# Repair: manually fix DB, then mark migration as successful
flyway repair

# Or: rollback manually, delete from schema_history, retry
DELETE FROM flyway_schema_history WHERE version = '3';
flyway migrate
```

## Migration Coordination in DevOps

### Deployment Flow

```
1. Pre-deployment
   - Test migration on staging DB (fresh copy of prod)
   - Team review migration script
   - On-call engineer ready for manual rollback

2. Deployment
   - Run migration (often in separate step before app deploy)
   - Verify via smoke tests (query new column, index usage)
   - Deploy new app code

3. Post-deployment
   - Monitor query latency (new index warming up)
   - Watch application error rates
   - If problems: rollback app code (not migration)
   - Rollback migration only if absolutely necessary
```

### Handling Migration Failures

**Option A: Fail safe**  
Migration fails → deployment blocked → investigate on staging → fix script → redeploy. Safest approach; adds delay.

**Option B: Compensate**  
Migration runs; if app detects failure → run compensating transaction → retry. Requires application awareness; more complex but enables auto-recovery.

**Option C: Expand-contract fallback**  
Migration applies expand; if problems → contract migration reverted in next cycle; data preserved.

## Trade-offs & Maturity Levels

| Aspect | Young Teams | Scaling Teams | High-Assurance |
|--------|------------|---------------|---|
| Versioning | Manual tracking | Flyway/Liquibase | Declarative (Prisma/Ecto) |
| Rollbacks | Manual SQL | Tested down migrations | Expand-contract + zero-downtime |
| Testing | Ad-hoc on staging | CI pipeline + shadow DB | Production replica + chaos injection |
| Frequency | Quarterly | Monthly | Weekly or on-demand |
| Downtime tolerance | Hours OK | Seconds | Microseconds (canary) |

## Best Practices

1. **One change per migration.** Easier to review, debug, and rollback.
2. **Immutable migration history.** Never edit migrations after they're applied in shared environments.
3. **Expand-contract for breaking changes.** Zero-downtime migrations require one release cycle.
4. **Test every migration.** Local replica, shadow DB, CI pipeline — pick your level.
5. **Automate rollbacks.** Script compensating transactions; avoid manual recovery under pressure.
6. **Profile migrations.** Large tables? Run migration on production replica; time it; alert if slow.

## See Also

- [Database — Patterns & Design](database-patterns.md)
- [Database — Query Optimization](database-query-optimization.md)
- [DevOps — CI/CD Patterns](devops-cicd-patterns.md)
- [Architecture — Event Sourcing](architecture-event-sourcing.md)