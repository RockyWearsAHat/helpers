# Database Migrations

## Schema Versioning Fundamentals

Database migrations are versioned, ordered changes to a database schema. Each migration has an up (apply) and optionally a down (rollback) operation, tracked in a metadata table.

### Core Principles

| Principle                    | Description                                                           |
| ---------------------------- | --------------------------------------------------------------------- |
| Immutable history            | Never edit a migration after it's been applied in shared environments |
| Forward-only in production   | Rollbacks are risky; prefer compensating migrations                   |
| One change per migration     | Easier to review, debug, and rollback                                 |
| Idempotent when possible     | Re-running shouldn't cause errors (use IF NOT EXISTS, IF EXISTS)      |
| Test on production-like data | Schema changes that work on empty tables may fail on large tables     |

### Version Tracking

Most tools store migration state in a metadata table:

```sql
-- Typical migration tracking table
CREATE TABLE schema_migrations (
    version      VARCHAR(255) PRIMARY KEY,   -- "20250315120000" or "V1.2.3"
    description  VARCHAR(255),
    applied_at   TIMESTAMP DEFAULT now(),
    checksum     VARCHAR(64),                -- detect tampered migrations
    execution_time_ms INTEGER
);
```

## Tools by Ecosystem

### Flyway (JVM / SQL-first)

Convention-based: SQL files named `V1__description.sql`, `V2__description.sql`.

```
db/migration/
├── V1__create_users_table.sql
├── V2__add_email_column.sql
├── V3__create_orders_table.sql
├── R__refresh_materialized_views.sql  (repeatable, runs on change)
└── U2__undo_add_email_column.sql      (undo, Flyway Teams only)
```

```sql
-- V1__create_users_table.sql
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(100) NOT NULL UNIQUE,
    created_at TIMESTAMP DEFAULT now()
);

-- V2__add_email_column.sql
ALTER TABLE users ADD COLUMN email VARCHAR(255);
CREATE INDEX idx_users_email ON users (email);
```

```bash
flyway -url=jdbc:postgresql://localhost/mydb -user=admin migrate
flyway info       # show migration status
flyway validate   # verify applied migrations match local files
flyway repair     # fix metadata table after failed migration
flyway baseline   # mark existing database as V1 (adopting Flyway on existing DB)
```

**Key features**: Checksum validation, placeholder substitution, Java-based migrations for complex logic, callbacks (beforeMigrate, afterMigrate).

### Liquibase (JVM / XML/YAML/SQL)

Changelog-based with explicit changesets and richer metadata.

```yaml
# changelog.yaml
databaseChangeLog:
  - changeSet:
      id: 1
      author: alice
      changes:
        - createTable:
            tableName: users
            columns:
              - column:
                  name: id
                  type: bigint
                  autoIncrement: true
                  constraints:
                    primaryKey: true
              - column:
                  name: username
                  type: varchar(100)
                  constraints:
                    nullable: false
                    unique: true
      rollback:
        - dropTable:
            tableName: users

  - changeSet:
      id: 2
      author: alice
      preConditions:
        - onFail: MARK_RAN
        - tableExists:
            tableName: users
      changes:
        - addColumn:
            tableName: users
            columns:
              - column:
                  name: email
                  type: varchar(255)
```

```bash
liquibase update
liquibase rollbackCount 1
liquibase diff --referenceUrl=jdbc:postgresql://localhost/prod  # compare schemas
liquibase generateChangeLog  # reverse-engineer from existing DB
```

**Key features**: Preconditions, contexts (run only in specific environments), labels, automatic rollback generation for many change types, multi-database support.

### Alembic (Python / SQLAlchemy)

```bash
alembic init migrations              # scaffold
alembic revision -m "create users"   # create migration
alembic revision --autogenerate -m "add email"  # auto-detect model changes
alembic upgrade head                 # apply all
alembic downgrade -1                 # rollback one
alembic history                      # show migration history
alembic current                      # show current version
```

```python
# migrations/versions/001_create_users.py
from alembic import op
import sqlalchemy as sa

revision = '001'
down_revision = None

def upgrade():
    op.create_table('users',
        sa.Column('id', sa.BigInteger, primary_key=True),
        sa.Column('username', sa.String(100), nullable=False, unique=True),
        sa.Column('email', sa.String(255)),
        sa.Column('created_at', sa.DateTime, server_default=sa.func.now()),
    )
    op.create_index('idx_users_email', 'users', ['email'])

def downgrade():
    op.drop_index('idx_users_email')
    op.drop_table('users')
```

**Autogenerate** compares SQLAlchemy models to current DB schema and generates migration diffs. Not perfect — doesn't detect renamed columns, changes to constraints on existing columns, or data migrations. Always review generated code.

### ActiveRecord Migrations (Ruby on Rails)

```bash
rails generate migration CreateUsers username:string email:string
rails generate migration AddAgeToUsers age:integer
rails db:migrate
rails db:rollback STEP=2
rails db:migrate:status
```

```ruby
# db/migrate/20250315120000_create_users.rb
class CreateUsers < ActiveRecord::Migration[7.1]
  def change
    create_table :users do |t|
      t.string :username, null: false, index: { unique: true }
      t.string :email
      t.timestamps
    end
  end
end

# Reversible migration with explicit up/down
class MigrateUserData < ActiveRecord::Migration[7.1]
  def up
    User.where(role: nil).update_all(role: 'member')
  end

  def down
    # Cannot reliably reverse data migration
    raise ActiveRecord::IrreversibleMigration
  end
end
```

**Key features**: `change` method auto-generates rollback for common operations, `reversible` blocks, strong migrations gem for safety checks.

### Prisma Migrate (Node.js / TypeScript)

```bash
npx prisma migrate dev --name create_users    # create + apply (dev)
npx prisma migrate deploy                     # apply pending (production)
npx prisma migrate reset                      # drop DB + reapply all (dev only)
npx prisma migrate status                     # show current state
```

```prisma
// schema.prisma — the source of truth
model User {
  id        Int      @id @default(autoincrement())
  username  String   @unique @db.VarChar(100)
  email     String?  @db.VarChar(255)
  orders    Order[]
  createdAt DateTime @default(now())
}
```

Prisma generates SQL migrations from schema diffs. Migrations are SQL files in `prisma/migrations/`. Production deploys use `prisma migrate deploy` (no interactive prompts, no shadow database).

### Knex.js (Node.js)

```bash
npx knex migrate:make create_users
npx knex migrate:latest
npx knex migrate:rollback
npx knex migrate:status
```

```javascript
// migrations/20250315120000_create_users.js
exports.up = function (knex) {
  return knex.schema.createTable("users", (table) => {
    table.bigIncrements("id");
    table.string("username", 100).notNullable().unique();
    table.string("email", 255);
    table.timestamps(true, true); // created_at, updated_at
    table.index(["email"]);
  });
};

exports.down = function (knex) {
  return knex.schema.dropTable("users");
};
```

### golang-migrate

```bash
migrate create -ext sql -dir db/migrations create_users
migrate -database "postgres://user:pass@localhost/db?sslmode=disable" \
    -path db/migrations up
migrate ... down 1
migrate ... version    # current version
migrate ... force 3    # force version (fix dirty state)
```

```sql
-- db/migrations/000001_create_users.up.sql
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(100) NOT NULL UNIQUE,
    email VARCHAR(255),
    created_at TIMESTAMP DEFAULT now()
);

-- db/migrations/000001_create_users.down.sql
DROP TABLE IF EXISTS users;
```

Separate `.up.sql` and `.down.sql` files. `dirty` state occurs when a migration fails partway — use `force` to set version and manually fix.

## Zero-Downtime Migrations (Expand-Contract)

The expand-contract pattern applies schema changes in phases so the old and new application versions can run simultaneously.

### Phase 1: Expand (Add)

Add new structures without removing old ones. Both old and new code work.

```sql
-- Adding a column: add as nullable first
ALTER TABLE users ADD COLUMN display_name VARCHAR(255);

-- Backfill in batches (not one big UPDATE)
UPDATE users SET display_name = username WHERE id BETWEEN 1 AND 10000;
UPDATE users SET display_name = username WHERE id BETWEEN 10001 AND 20000;
-- ... or use application-level backfill
```

### Phase 2: Migrate (Transition)

Deploy new application code that writes to both old and new structures. Background job migrates existing data.

### Phase 3: Contract (Remove)

Once all data is migrated and old code is fully replaced:

```sql
-- Now safe to add NOT NULL
ALTER TABLE users ALTER COLUMN display_name SET NOT NULL;

-- Drop old column in separate migration after verification
ALTER TABLE users DROP COLUMN old_name;
```

### Renaming a Column (Zero-Downtime)

**Never** do `ALTER TABLE RENAME COLUMN` directly — breaks running application.

```
Step 1: Add new column
  ALTER TABLE users ADD COLUMN display_name VARCHAR(255);

Step 2: Deploy code that writes to BOTH columns (dual-write)

Step 3: Backfill new column from old column

Step 4: Deploy code that reads from new column only

Step 5: Drop old column (separate migration, after verification)
```

### Adding an Index Without Locking

```sql
-- PostgreSQL: CONCURRENTLY avoids table lock
CREATE INDEX CONCURRENTLY idx_users_email ON users (email);
-- Cannot run inside a transaction
-- Requires retry if it fails (leaves invalid index)

-- MySQL 5.6+: Online DDL
ALTER TABLE users ADD INDEX idx_email (email), ALGORITHM=INPLACE, LOCK=NONE;
```

## Dangerous Operations

| Operation                     | Risk                                | Safe Alternative                                 |
| ----------------------------- | ----------------------------------- | ------------------------------------------------ |
| `DROP TABLE` / `DROP COLUMN`  | Data loss                           | Rename first, drop after verification period     |
| `ALTER COLUMN SET NOT NULL`   | Fails on existing NULLs             | Backfill NULLs first, then add constraint        |
| `ALTER COLUMN TYPE`           | Full table rewrite (Postgres)       | Add new column, migrate data, drop old           |
| `CREATE INDEX`                | Table lock (without CONCURRENTLY)   | Use `CONCURRENTLY` (Postgres) or online DDL      |
| `ADD COLUMN WITH DEFAULT`     | Table rewrite on old Postgres (<11) | Add column, then set default separately          |
| `RENAME TABLE`                | Breaks application code             | Use expand-contract pattern                      |
| `TRUNCATE TABLE`              | Data loss, lock                     | DELETE in batches if needed                      |
| Large `UPDATE`                | Lock contention, WAL bloat          | Batch updates with `LIMIT` / ranges              |
| `ALTER TABLE` on large tables | Long lock, blocks writes            | Use pt-online-schema-change (MySQL) or pg_repack |

### Tools for Large Table Changes

| Tool                        | Database   | Approach                                 |
| --------------------------- | ---------- | ---------------------------------------- |
| `pt-online-schema-change`   | MySQL      | Creates shadow table, copies data, swaps |
| `gh-ost`                    | MySQL      | Binlog-based, no triggers                |
| `pg_repack`                 | PostgreSQL | Repacks tables without heavy locks       |
| `CREATE INDEX CONCURRENTLY` | PostgreSQL | Non-blocking index creation              |

## Rollback Strategies

### Migration Rollbacks

```
Option 1: Down migration (reverse the change)
  - Fine for additive changes (drop the added column)
  - Dangerous for destructive changes (can't un-drop data)
  - Many teams skip down migrations entirely

Option 2: Compensating migration (forward fix)
  - Create a new migration that fixes the problem
  - Safer: preserves audit trail, no version ambiguity
  - Preferred in production

Option 3: Restore from backup
  - Last resort for catastrophic failures
  - Requires PITR (Point-in-Time Recovery) to avoid data loss
  - Practice this regularly
```

### Data Migration Rollback

For data changes, always capture the before state:

```sql
-- Before migration: snapshot affected data
CREATE TABLE users_backup_v42 AS SELECT * FROM users WHERE email IS NULL;

-- Run data migration
UPDATE users SET email = username || '@example.com' WHERE email IS NULL;

-- If rollback needed:
UPDATE users u SET email = NULL
FROM users_backup_v42 b WHERE u.id = b.id;
DROP TABLE users_backup_v42;
```

## CI/CD Integration

### Pipeline Structure

```yaml
# GitHub Actions example
name: Database Migration
on:
  push:
    paths: ["db/migrations/**"]

jobs:
  validate:
    steps:
      - name: Lint migrations
        run: |
          # Check for dangerous patterns
          grep -rl "DROP TABLE\|TRUNCATE\|DROP COLUMN" db/migrations/ && exit 1 || true

      - name: Dry run against shadow DB
        run: |
          flyway -url=jdbc:postgresql://shadow-db/test migrate

      - name: Run migration tests
        run: pytest tests/test_migrations.py

  deploy-staging:
    needs: validate
    steps:
      - name: Apply migrations
        run: flyway -url=$STAGING_DB_URL migrate

      - name: Smoke test
        run: ./scripts/smoke-test.sh

  deploy-production:
    needs: deploy-staging
    environment: production # requires approval
    steps:
      - name: Apply migrations
        run: flyway -url=$PROD_DB_URL migrate
```

### Migration Testing Strategies

```python
# Test that migrations apply cleanly to empty database
def test_fresh_migration():
    run_all_migrations(empty_database)
    assert_schema_matches_expected(database)

# Test that migrations apply to current production state
def test_incremental_migration():
    restore_production_snapshot(database)
    run_pending_migrations(database)
    assert_schema_matches_expected(database)

# Test that rollbacks work
def test_rollback():
    run_all_migrations(database)
    rollback_last_migration(database)
    run_pending_migrations(database)
    assert_schema_matches_expected(database)

# Test with production-scale data
def test_migration_performance():
    seed_with_production_volumes(database)
    start = time.time()
    run_pending_migrations(database)
    assert time.time() - start < 300  # must complete in 5 minutes
```

### Safety Checks

Automated checks to add to your pipeline:

| Check                            | Implementation                                           |
| -------------------------------- | -------------------------------------------------------- |
| No backward-incompatible changes | Parse SQL for DROP, RENAME without expand-contract       |
| Migration is reversible          | Verify down migration exists and runs                    |
| Performance within budget        | Run against production-sized dataset, enforce time limit |
| No data loss                     | Verify row counts before/after                           |
| Checksum integrity               | Detect modified applied migrations                       |
| Sequential ordering              | No gaps or conflicts in version numbers                  |
| Transaction safety               | Long-running DDL shouldn't hold locks                    |

### Environment Management

```
dev       → Local database, drop + recreate freely
test/CI   → Shadow database, fresh from migrations each run
staging   → Production-like, apply migrations before deployment
production → Apply with monitoring, approval gates, rollback plan ready
```

Keep a **migration runbook** for production:

1. Note estimated duration and lock impact
2. Schedule maintenance window if needed
3. Take backup / verify PITR before applying
4. Apply migration
5. Verify application health
6. Monitor for 30 minutes
7. Drop backup table after verification period (e.g., 7 days)
