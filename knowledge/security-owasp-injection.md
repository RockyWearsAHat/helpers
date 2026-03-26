# Injection Attacks Deep Dive

## SQL Injection

### Classic (In-Band)

Attacker modifies SQL query through unsanitized input. The result is returned directly in the response.

```sql
-- Vulnerable query
SELECT * FROM users WHERE username = '$input' AND password = '$pass';

-- Attack: input = ' OR '1'='1' --
SELECT * FROM users WHERE username = '' OR '1'='1' --' AND password = '';
```

### Blind SQL Injection

No visible output — attacker infers data from boolean responses or timing.

**Boolean-based**: `' AND (SELECT SUBSTRING(password,1,1) FROM users WHERE id=1)='a' --`
Response changes (200 vs 500, content difference) reveal one character at a time.

**Time-based**: `' AND IF(SUBSTRING(database(),1,1)='a', SLEEP(5), 0) --`
Response delay reveals data. Works even when error messages are suppressed.

### Second-Order Injection

Payload stored in database, executed later in a different query context. Example: user registers with username `admin'--`, which is safely parameterized during INSERT. Later, an admin search query uses the stored value unsafely.

### ORM Injection

ORMs are NOT automatically safe:

```python
# VULNERABLE — raw SQL in ORM
User.objects.raw(f"SELECT * FROM users WHERE name = '{name}'")
User.objects.extra(where=[f"name = '{name}'"])

# SAFE
User.objects.filter(name=name)
User.objects.raw("SELECT * FROM users WHERE name = %s", [name])
```

**Sequelize**: `Op.like` with unvalidated user input can still be dangerous.
**ActiveRecord**: `where("name = '#{params[:name]}'")` is vulnerable; `where(name: params[:name])` is safe.

### UNION-Based

Combine results from injected query with original:

```sql
' UNION SELECT username, password, NULL FROM admin_users --
```

Requirements: same number of columns, compatible types. Use `ORDER BY` incrementing to find column count.

## Command Injection

### OS Command Injection

```python
# VULNERABLE
os.system(f"ping -c 1 {user_input}")
# Attack: user_input = "8.8.8.8; cat /etc/passwd"

# SAFE
subprocess.run(["ping", "-c", "1", user_input], shell=False)
```

**Key**: never use `shell=True` with user input. Use array form for command arguments.

### Argument Injection

Even without shell metacharacters, injecting flags can be dangerous:

```bash
# user_input = "--output=/etc/cron.d/evil"
curl $user_input https://example.com
```

Use `--` to terminate option parsing: `curl -- "$user_input"`.

## LDAP Injection

```
# Vulnerable filter
(&(uid=$input)(userPassword=$pass))

# Attack: input = *)(|(uid=*
# Result: (&(uid=*)(|(uid=*)(userPassword=anything))
```

Prevention: escape special characters `* ( ) \ NUL` in LDAP filters.

## Template Injection (SSTI)

Server-Side Template Injection — when user input is rendered as template code:

```python
# VULNERABLE — Jinja2
template = Template(f"Hello {user_input}")

# Attack: user_input = "{{ config.items() }}"
# Or: "{{ ''.__class__.__mro__[1].__subclasses__() }}"

# SAFE
template = Template("Hello {{ name }}")
template.render(name=user_input)
```

Affected engines: Jinja2, Twig, Freemarker, Velocity, Pebble, Smarty, ERB. Detection: inject `{{7*7}}` — if output is `49`, SSTI is confirmed.

## Header Injection (CRLF)

Injecting `\r\n` into HTTP headers to add arbitrary headers or split responses:

```
# Vulnerable: redirect based on user input
Location: /page?lang=en\r\nSet-Cookie: admin=true
```

Prevention: strip or reject `\r` and `\n` from header values.

## XPath Injection

Similar to SQL injection but targets XML data stores:

```xpath
/users/user[username='$input' and password='$pass']
# Attack: input = ' or '1'='1
```

Prevention: parameterized XPath queries or input validation.

## Prevention Techniques

### Parameterized Queries (Primary Defense)

| Language            | Safe Pattern                                                                   |
| ------------------- | ------------------------------------------------------------------------------ |
| Python (psycopg2)   | `cursor.execute("SELECT * FROM t WHERE id = %s", [user_id])`                   |
| Node.js (pg)        | `client.query('SELECT * FROM t WHERE id = $1', [userId])`                      |
| Java (JDBC)         | `PreparedStatement ps = conn.prepareStatement("SELECT * FROM t WHERE id = ?")` |
| Go (database/sql)   | `db.Query("SELECT * FROM t WHERE id = ?", userID)`                             |
| Ruby (ActiveRecord) | `User.where(id: user_id)`                                                      |
| PHP (PDO)           | `$stmt = $pdo->prepare("SELECT * FROM t WHERE id = :id")`                      |

### Input Validation

- **Allowlist** over blocklist: define what IS allowed, not what isn't
- Validate type, length, range, format
- Reject null bytes, control characters
- Integers: parse to int and use the parsed value
- Strings: restrict character set where possible (alphanumeric for usernames)

### Escaping (Last Resort)

Database-specific escaping functions exist but are fragile. Use as defense-in-depth, never as primary protection.

### Least Privilege

- Database user should have minimal permissions (SELECT only if reads only)
- No DBA or DDL permissions for application database users
- Separate read/write database users where possible

### WAF Rules

Web Application Firewalls catch common patterns but are bypassable. Use as defense-in-depth layer:

- ModSecurity Core Rule Set (CRS)
- AWS WAF managed rules
- Cloudflare WAF

### Detection

| Tool       | Type    | Coverage                                           |
| ---------- | ------- | -------------------------------------------------- |
| SQLMap     | Dynamic | Automated SQL injection detection and exploitation |
| Semgrep    | SAST    | Pattern-based, custom rules, 1000+ injection rules |
| Snyk Code  | SAST    | Dataflow analysis, taint tracking                  |
| Burp Suite | DAST    | Active/passive scanning, manual testing            |
| OWASP ZAP  | DAST    | Free, automated scanning                           |
| CodeQL     | SAST    | Deep dataflow analysis, GitHub integration         |

## Anti-Patterns

- String concatenation for ANY query construction
- Using `shell=True` or equivalent in any language
- Blocklist-only input validation ("just block semicolons")
- Trusting ORM methods to always be safe (raw queries in ORMs are still vulnerable)
- Disabling parameterized queries for "performance" (negligible difference)
- Only checking for injection at the frontend (backend is the trust boundary)
