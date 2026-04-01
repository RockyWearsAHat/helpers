# Nginx

## Architecture

Nginx uses an event-driven, asynchronous, non-blocking architecture — fundamentally different from Apache's process/thread-per-connection model.

```
┌─────────────────┐
│  Master process  │  reads config, binds ports, manages workers
├─────────────────┤
│  Worker 1       │  event loop handling thousands of connections
│  Worker 2       │  each worker is single-threaded
│  Worker N       │  (N = number of CPU cores typically)
└─────────────────┘
```

**Master process**: runs as root (to bind port 80/443), spawns workers, handles config reloads (`nginx -s reload` — graceful, zero-downtime), manages signals.

**Worker processes**: run as unprivileged user (`www-data`/`nginx`). Each handles thousands of concurrent connections via epoll (Linux) / kqueue (BSD/macOS). No thread context switching overhead.

```nginx
worker_processes auto;        # match CPU cores
worker_rlimit_nofile 65535;   # file descriptor limit per worker

events {
    worker_connections 4096;  # max connections per worker
    multi_accept on;          # accept multiple connections at once
    use epoll;                # Linux (auto-detected)
}
```

## Configuration Structure

```nginx
# Main context (global)
user nginx;
pid /run/nginx.pid;
error_log /var/log/nginx/error.log warn;

# Events context
events { ... }

# HTTP context
http {
    include /etc/nginx/mime.types;
    default_type application/octet-stream;

    # Logging
    log_format main '$remote_addr - $remote_user [$time_local] '
                    '"$request" $status $body_bytes_sent '
                    '"$http_referer" "$http_user_agent" '
                    '$request_time';
    access_log /var/log/nginx/access.log main;

    # Server context (virtual host)
    server {
        listen 80;
        server_name example.com;

        # Location context (URL matching)
        location / {
            root /var/www/html;
            index index.html;
        }
    }

    # Include separate configs
    include /etc/nginx/conf.d/*.conf;
}

# Stream context (TCP/UDP proxying)
stream {
    upstream db_backend {
        server db1:5432;
        server db2:5432;
    }
    server {
        listen 5432;
        proxy_pass db_backend;
    }
}
```

**Inheritance**: directives in outer contexts cascade inward. Inner contexts can override. Arrays (like `add_header`) do NOT merge — inner block completely replaces outer.

## Location Matching

Evaluated in this priority order (NOT config file order):

| Priority | Prefix | Type                     | Example            | Behavior                                      |
| -------- | ------ | ------------------------ | ------------------ | --------------------------------------------- |
| 1        | `=`    | Exact match              | `= /favicon.ico`   | Exact URI only. Stops search immediately.     |
| 2        | `^~`   | Preferential prefix      | `^~ /static/`      | Longest prefix match. Skips regex evaluation. |
| 3        | `~`    | Regex (case-sensitive)   | `~ \.php$`         | First regex match in config order wins.       |
| 3        | `~*`   | Regex (case-insensitive) | `~* \.(jpg\|png)$` | Same priority as `~`.                         |
| 4        | (none) | Prefix                   | `/api/`            | Longest prefix match, but regex can override. |

```nginx
# Exact match — fastest, use for high-traffic specific URLs
location = / {
    # Only matches exactly "/"
}

# Preferential prefix — files from /static/ always here, no regex override
location ^~ /static/ {
    alias /var/www/static/;
    expires 30d;
}

# Regex — match file extensions
location ~* \.(js|css|png|jpg|svg|woff2?)$ {
    expires 1y;
    add_header Cache-Control "public, immutable";
}

# Standard prefix — regex CAN override this
location /api/ {
    proxy_pass http://backend;
}

# Catch-all for SPA routing
location / {
    try_files $uri $uri/ /index.html;
}
```

**`try_files`**: check each path in order, serve first that exists. Last argument is internal redirect (no `=` prefix) or status code (`=404`).

## Reverse Proxy

```nginx
upstream backend {
    server 10.0.0.1:3000;
    server 10.0.0.2:3000;
    keepalive 32;            # persistent connections to upstream
}

server {
    listen 80;
    server_name app.example.com;

    location / {
        proxy_pass http://backend;
        proxy_http_version 1.1;

        # Pass client info to backend
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Keepalive to upstream
        proxy_set_header Connection "";

        # Timeouts
        proxy_connect_timeout 5s;
        proxy_read_timeout 60s;
        proxy_send_timeout 30s;

        # Buffering
        proxy_buffering on;
        proxy_buffer_size 4k;
        proxy_buffers 8 8k;
    }
}
```

### WebSocket Proxying

```nginx
location /ws/ {
    proxy_pass http://backend;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
    proxy_read_timeout 3600s;     # long timeout for persistent connections
}
```

### gRPC Proxying

```nginx
location /grpc/ {
    grpc_pass grpc://backend:50051;
    error_page 502 = /error502grpc;
}
```

## Load Balancing

```nginx
upstream backend {
    # Algorithms
    # (default) round-robin — equal distribution
    # least_conn — fewest active connections
    # ip_hash — sticky sessions by client IP
    # hash $request_uri consistent — consistent hashing

    least_conn;

    server 10.0.0.1:3000 weight=3;      # 3x traffic
    server 10.0.0.2:3000 weight=1;
    server 10.0.0.3:3000 backup;         # only when others are down
    server 10.0.0.4:3000 down;           # marked offline

    # Health checks (Nginx Plus only — OSS uses passive)
    # Passive: max_fails=3 fail_timeout=30s (default)
    server 10.0.0.1:3000 max_fails=3 fail_timeout=30s;
}
```

## SSL/TLS

```nginx
server {
    listen 443 ssl http2;
    server_name app.example.com;

    # Certificates
    ssl_certificate /etc/letsencrypt/live/app.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/app.example.com/privkey.pem;

    # Modern TLS config (Mozilla recommended)
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;

    # Session resumption
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 1d;
    ssl_session_tickets off;

    # OCSP stapling
    ssl_stapling on;
    ssl_stapling_verify on;
    ssl_trusted_certificate /etc/letsencrypt/live/app.example.com/chain.pem;
    resolver 1.1.1.1 8.8.8.8 valid=300s;

    # HSTS (1 year, include subdomains)
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload" always;
}

# HTTP → HTTPS redirect
server {
    listen 80;
    server_name app.example.com;
    return 301 https://$host$request_uri;
}
```

## Caching

```nginx
http {
    # Define cache zone
    proxy_cache_path /var/cache/nginx
        levels=1:2                 # directory depth
        keys_zone=app_cache:10m    # 10MB metadata (holds ~80k keys)
        max_size=1g                # max disk usage
        inactive=60m               # purge after 60min unused
        use_temp_path=off;

    server {
        location /api/ {
            proxy_pass http://backend;
            proxy_cache app_cache;
            proxy_cache_valid 200 10m;       # cache 200 for 10 minutes
            proxy_cache_valid 404 1m;
            proxy_cache_key "$scheme$request_method$host$request_uri";
            proxy_cache_use_stale error timeout http_500 http_502 http_503;
            proxy_cache_lock on;             # prevent stampede
            proxy_cache_min_uses 2;          # cache after 2nd request

            add_header X-Cache-Status $upstream_cache_status;
            # HIT, MISS, BYPASS, EXPIRED, STALE, UPDATING
        }

        # Bypass cache
        proxy_cache_bypass $http_cache_control;
        # curl -H "Cache-Control: no-cache" to bypass
    }
}
```

## Rate Limiting

```nginx
http {
    # Define rate limit zones
    limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
    limit_req_zone $binary_remote_addr zone=login:10m rate=1r/s;

    server {
        location /api/ {
            limit_req zone=api burst=20 nodelay;
            # 10 req/s sustained, burst of 20, no delay (reject excess)
            proxy_pass http://backend;
        }

        location /login {
            limit_req zone=login burst=5;
            # 1 req/s sustained, burst of 5, excess delayed (queued)
            limit_req_status 429;
            proxy_pass http://backend;
        }
    }

    # Connection limiting
    limit_conn_zone $binary_remote_addr zone=addr:10m;

    server {
        limit_conn addr 100;     # max 100 connections per IP
    }
}
```

## Compression

```nginx
http {
    # gzip
    gzip on;
    gzip_vary on;
    gzip_proxied any;
    gzip_comp_level 4;            # 1-9 (4-6 is sweet spot)
    gzip_min_length 256;
    gzip_types
        text/plain
        text/css
        text/javascript
        application/javascript
        application/json
        application/xml
        image/svg+xml;

    # Brotli (requires ngx_brotli module)
    # brotli on;
    # brotli_comp_level 6;
    # brotli_types text/plain text/css application/javascript application/json image/svg+xml;
}
```

## Security Headers

```nginx
server {
    # Clickjacking protection
    add_header X-Frame-Options "SAMEORIGIN" always;

    # XSS protection (legacy, CSP is better)
    add_header X-Content-Type-Options "nosniff" always;

    # Content Security Policy
    add_header Content-Security-Policy "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self';" always;

    # Referrer policy
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;

    # Permissions policy
    add_header Permissions-Policy "camera=(), microphone=(), geolocation=()" always;

    # Hide server version
    server_tokens off;

    # Limit request body size
    client_max_body_size 10m;
}
```

## Performance Tuning

```nginx
http {
    # File I/O
    sendfile on;                  # kernel-level file transfer (bypass userspace)
    tcp_nopush on;                # send headers and beginning of file in one packet
    tcp_nodelay on;               # disable Nagle's algorithm for keepalive

    # Keepalive
    keepalive_timeout 65;
    keepalive_requests 1000;      # max requests per keepalive connection

    # Client timeouts
    client_body_timeout 12;
    client_header_timeout 12;
    send_timeout 10;

    # Buffer sizes
    client_body_buffer_size 16k;
    client_header_buffer_size 1k;
    large_client_header_buffers 4 8k;

    # Open file cache
    open_file_cache max=10000 inactive=20s;
    open_file_cache_valid 30s;
    open_file_cache_min_uses 2;
    open_file_cache_errors on;
}
```

## Common Patterns

### SPA with API Proxy

```nginx
server {
    listen 80;
    root /var/www/app;

    # API requests → backend
    location /api/ {
        proxy_pass http://backend:3000;
    }

    # Static assets with long cache
    location /assets/ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    # SPA catch-all
    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

### Custom Error Pages

```nginx
error_page 404 /404.html;
error_page 500 502 503 504 /50x.html;

location = /50x.html {
    root /var/www/errors;
    internal;                      # only for internal redirects
}
```

## Comparison to Alternatives

| Feature               | Nginx                      | Caddy                      | Traefik                        |
| --------------------- | -------------------------- | -------------------------- | ------------------------------ |
| **Config**            | Manual files               | Caddyfile (simple) or JSON | Labels/tags (auto-discovery)   |
| **Auto HTTPS**        | Manual (certbot)           | Built-in (automatic)       | Built-in (Let's Encrypt)       |
| **Service discovery** | Static upstream            | None (manual)              | Docker/K8s native              |
| **Performance**       | Highest                    | Good                       | Good                           |
| **Use case**          | High-traffic, fine control | Simple sites, APIs         | Microservices, dynamic routing |
| **Hot reload**        | `nginx -s reload`          | Automatic                  | Automatic                      |
| **Language**          | C                          | Go                         | Go                             |

### Caddy Equivalent

```
# Caddyfile — auto HTTPS, much simpler for basic cases
app.example.com {
    reverse_proxy localhost:3000
    encode gzip
    header {
        X-Frame-Options SAMEORIGIN
        -Server
    }
}
```

### Traefik with Docker

```yaml
# docker-compose labels — no separate config file
services:
  app:
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.app.rule=Host(`app.example.com`)"
      - "traefik.http.routers.app.tls.certresolver=letsencrypt"
```

## Debugging

```bash
nginx -t                          # test config syntax
nginx -T                          # test and dump full config
nginx -s reload                   # graceful reload
nginx -s quit                     # graceful shutdown

# Debug logging (temporary — very verbose)
error_log /var/log/nginx/error.log debug;

# Check what's happening
curl -I https://app.example.com   # response headers
curl -v https://app.example.com   # full request/response
```
