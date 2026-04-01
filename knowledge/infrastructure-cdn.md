# Content Delivery Networks (CDN) — Architecture, Edge Caching & Multi-CDN Strategy

## Overview

A CDN is a globally distributed system of edge servers that cache and serve content closer to end users, reducing latency, bandwidth costs, and origin server load. CDNs operate at multiple layers: static asset delivery (images, video, CSS/JS), dynamic content acceleration, DNS routing, and increasingly, edge compute for intelligent application logic.

## Core Architecture: Origin, POPs, and Edge Servers

CDN architecture separates **origin servers** (authoritative source of truth) from **Points of Presence (POPs)** — regional clusters of edge servers deployed near Internet eXchange Points (IXPs) and ISP networks. When a user requests content:

1. DNS resolution routes the request to the nearest or best-performing edge server
2. That edge server checks its cache
3. If missed, it queries the origin (or parent cache tier) and stores the result locally
4. Response is served from edge at low latency

**Origin Shield** is a caching intermediate between origin and edge servers. Instead of N edge servers hitting the origin directly on cache miss, they query a shared shield, reducing origin load and improving hit ratios. The trade-off: one extra hop adds latency (typically acceptable).

## Cache Control and Purge Strategy

Edge caches follow HTTP `Cache-Control` headers from the origin:

- **TTL (Time To Live)**: How long content remains valid. Short TTLs (minutes) suit frequently-updated data; long TTLs (hours/days) suit static assets.
- **Surrogate-Control**: CDN-specific directives (e.g., `stale-while-revalidate`, `stale-if-error`) that differ from browser cache behavior.
- **Cache Key Design**: CDN cache is keyed on URL (or URL + query params). A poor cache key causes unnecessary variation and misses; a good one normalizes requests (stripping tracking parameters, sorting query strings).

**Purge and Invalidation**: Most CDNs provide immediate purge (remove from all edges synchronously) and soft purge (mark stale but serve while revalidating in background). Purge is expensive operationally; designing cache keys and TTLs to minimize purge need is preferable.

## Dynamic Site Acceleration (DSA)

DSA optimizes delivery of dynamic, user-specific content (shopping carts, profiles, real-time data). Techniques include:

- **Request Collapsing**: Multiple simultaneous requests for the same non-cached resource queue at the origin instead of stampeding it.
- **Connection Pooling**: Edge servers reuse persistent connections to origin, reducing handshake overhead.
- **Compression and Optimization**: Automatic minification, image optimization, or protocol upgrades (HTTP/2 → HTTP/3).
- **Smart Routing**: Real-time latency measurement to origin, selecting fastest path.

## Edge Compute and Separation of Concerns

Modern CDNs embed compute at the edge to avoid round-trips to origin servers. Examples:

- **Compute Functions**: Modify requests/responses (add headers, rewrite URLs, serve alternate content for A/B tests) without origin involvement.
- **Compute Time vs. Bandwidth Tradeoff**: Processing at edge trades CPU for bandwidth (e.g., compressing data at edge is cheaper than serving uncompressed).
- **Decoupling**: Business logic that can execute safely at edge (authentication checks, rate limiting, routing rules) runs there; sensitive operations (database writes, secrets access) return to origin.

## TLS at Edge and Certificate Management

Edge servers terminate TLS connections from clients, decrypting traffic at the edge for analysis and modification. CDNs manage the certificate lifecycle:

- **Shared Certificates**: Multiple domains on a single certificate (cheaper, but limits granularity).
- **Dedicated Certificates**: One per domain/customer (flexibility, but operational overhead).
- **Automatic Renewal**: CDNs auto-provision Let's Encrypt or self-signed certificates to reduce certificate expiry incidents.

Breaking TLS at edge involves trust assumptions: clients must trust both their TLS CA *and* the CI's intermediate CA. Mitigations include certificate pinning or mTLS for high-security use cases.

## DDoS Protection and Filtering

CDN infrastructure absorbs and mitigates DDoS by:

- **Rate Limiting**: Drop or challenge traffic exceeding per-IP thresholds.
- **Bot Detection**: Fingerprint and challenge bots (headless browsers, tool traffic) while allowing humans.
- **Anycast Scrubbing**: Massive distributed network distributes attack traffic across many POPs, making any single attacker's flooding capacity ineffective.
- **Application Layer Attacks**: WAF (Web Application Firewall) rules detect SQL injection, XSS, path traversal and other OWASP Top 10 patterns.
- **Geographic Filtering**: Block traffic from countries or ASNs known for malicious activity.

The model: attackers can flood a link to one POP; the POP drops traffic locally rather than propagating it upstream.

## Multi-CDN Strategies

Single-CDN dependency creates vendor lock-in and regional blind spots. Multi-CDN approaches:

- **Geographic Diversity**: Different CDNs dominate different regions (Akamai strong in Asia-Pacific, Fastly strong in Europe). Customers use multiple providers for resilience.
- **Performance Comparison**: Run A/B tests routing 5-10% of traffic through alternate CDNs to detect degradation.
- **DR Failover**: If primary CDN POPs in a region become unavailable, DNS failover to backup CDN.
- **Cost Optimization**: CDNs negotiate volume discounts per region; multi-CDN allows shopping for best rates.

Trade-off: multi-CDN increases operational complexity (certificate distribution, cache invalidation across providers, traffic routing logic) and requires negotiating interconnection agreements between CDNs.

## CDN Selection and Cost Models

**Major CDN Providers**:

- **Akamai**: Largest network footprint, premium pricing, strong media delivery and advanced enterprise features (bot management, DDoS).
- **Cloudflare**: Developer-friendly pricing, integrated DDoS + WAF, DNS management, global network smaller than Akamai but rapidly growing.
- **Fastly**: Real-time cache invalidation (microseconds), popular for video streaming and live events, expensive but high-performance.
- **AWS CloudFront**: Deep integration with S3, EC2, Lambda; variable costs but no upfront commitments; best suited for AWS-native workloads.
- **Google Cloud CDN**: Leverages Google's network backbone and data centers, integrates with GCP, common for YouTube-like services.

**Pricing Models**:

- **Pay-as-You-Go**: Charge per GB egress (+ optional request fees). Scales well for variable traffic but unpredictable costs.
- **Committed/Tiered**: Volume discounts for fixed monthly commitments. Better for predictable, high-volume traffic.
- **Bundled**: Platform (origin/compute/cache) bundles included; hard to isolate component costs but simpler billing.

**Performance Considerations**:

- **POP Density**: More POPs closer to users reduce latency; Akamai and larger CDNs have denser networks.
- **Network Peering**: Direct interconnection with major ISPs (transit peering) improves path quality vs. lower-tier CDNs relying on public Internet routes.
- **Protocol Support**: HTTP/3 (QUIC), gQUIC, early hints; newer protocols reduce RTT but require edge infrastructure investment.

## Consistency and Purge Complexity

CDN data is eventually consistent. A published update may not be uniformly visible across all POPs for seconds to minutes. This affects:

- **Critical Updates**: Security patches or urgent content fixes may propagate unevenly; some users see old version temporarily.
- **Versioning Strategy**: Instead of modifying versioned resources (build artifacts), append version hashes to URLs (`app.v123.js`); vastly old URLs are always safe.
- **Stale Content**: Long TTLs trade freshness for cache efficiency; soft purge and `stale-while-revalidate` reconcile the tradeoff.

See also: web-http-caching, system-design-distributed, cloud-aws-storage, infrastructure-load-balancing.