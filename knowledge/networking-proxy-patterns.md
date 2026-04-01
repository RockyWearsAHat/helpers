# Proxy Patterns — Interception, Relay, and Protocol Translation

## Overview

A proxy is an intermediary that receives requests from clients, forwards them to servers (or other proxies), and returns responses. Proxies sit at network boundaries, enabling filtering, caching, protocol translation, and load distribution.

Proxy architecture and configuration are varied by placement (forward vs. reverse), visibility (transparent), and protocol handling. Understanding the distinctions is essential for network design, security, and debugging.

## Forward Proxy

A forward proxy sits between clients and the public internet. Clients explicitly configure it as a gateway; traffic flows:

```
Client → Forward Proxy → Public Server
         ↑                    ↓
      (configured)         (direct/relayed)
```

### Use Cases

- **Egress filtering**: Organization blocks outbound access to certain sites or protocols
- **Caching**: Proxy caches responses (e.g., CDN, HTTP cache); repeated requests served locally
- **Privacy**: Server sees proxy origin, not client origin (client obtains privacy)
- **Malware scanning**: Proxy intercepts and scans files in flight
- **HTTP tunneling**: Client needs to tunnel non-HTTP traffic (SSH, VNC) over HTTP (e.g., corporate firewall allows only HTTP)

### Client Configuration

Explicit: HTTP/HTTPS proxy settings in OS or browser. SOCKS proxy: generic tunneling (UDP, TCP, any protocol).

### Limitations

- **Application must be aware or configured**: Not transparent to most apps; requires proxy configuration
- **Privacy illusion**: Forward proxy operator (organization) sees all traffic
- **No correlation to identity downstream**: Server cannot distinguish which internal client made request (sees proxy origin)

## Reverse Proxy

A reverse proxy sits between clients and internal servers. Clients contact the proxy directly (often don't know/care it's a proxy); the proxy forwards to backend servers:

```
Client → Reverse Proxy → Backend Server
      (direct)           (may be hidden)
```

### Use Cases

- **Load balancing**: Distribute requests across multiple backend servers
- **SSL termination**: Proxy handles TLS handshakes; internal traffic unencrypted (if not mTLS)
- **Ingress control**: Single entry point for multiple backend services (API gateway pattern)
- **Rate limiting**: Proxy enforces per-client quotas
- **Request transformation**: Add/remove headers, rewrite URIs before forwarding
- **Caching**: Cache responses from backends
- **Geographic distribution**: Different reverse proxies in different regions; clients directed to nearest

### Implementations

- **NGINX**: High-performance, event-driven. Handles 100K+ connections per machine.
- **HAProxy**: Specialized for L4/L7 load balancing; very fast proxy.
- **Envoy**: Modern, extensible (filter chains). Used in service meshes (Istio).
- **Apache HTTPD**: Full-featured; heavier than NGINX/HAProxy.

### Architectural Patterns

**Single Proxy**: One reverse proxy receives all traffic. Single point of failure; often paired with failover (Keepalived, DNS).

**Proxy Pool**: Multiple proxies behind load balancer (hardware LB, DNS round-robin). Distributes incoming load.

**Distributed Proxies**: Proxies in multiple geographic regions. Clients connect to nearest; backends replicate data or join distributed system.

## Transparent Proxy

A transparent proxy intercepts traffic at the network layer without client or server knowledge. Traffic is redirected to the proxy (via iptables, firewall rule, or physical network tap) and forwarded as if the proxy were the destination.

```
Client --→ Network (iptables/VLAN)--→ Transparent Proxy → Server
           ↑                                 ↓
      redirect to proxy              send to original destination
```

### How It Works

**Linux example**:
```bash
iptables -t nat -A PREROUTING -p tcp --dport 80 -j REDIRECT --to-port 8080
```

All port 80 traffic is redirected to port 8080, where the transparent proxy listens. The proxy reads the original destination (via SO_ORIGINAL_DST socket option) and connects to it.

### Use Cases

- **ISP/carrier-grade filtering**: Intercept and filter/cache traffic without client participation
- **Network monitoring**: Passive observation and logging
- **Malware scanning**: DPI (deep packet inspection) inspection
- **Protocol enforcement**: Block non-compliant protocols

### Limitations

- **Opacity**: Clients unaware they're being proxied. Can be contentious (privacy concerns, breaking assumptions).
- **Administrative access required**: Must run on network boundary (router, firewall).
- **Limited to network layer**: Cannot intercept encrypted traffic (HTTPS) without MITM certificate injection.

## SOCKS Proxy

**SOCKS** (RFC 1928) is a generic proxy protocol enabling any TCP/UDP application to tunnel through a proxy without protocol-specific knowledge.

### SOCKS5 Negotiation

```
Client                               SOCKS5 Server
  |                                       |
  |--- Greeting + Auth Methods ---→      |
  |← Auth response (if required) ←-------  |
  |--- Connect Request (host, port) ---→ |
  |← Reply (status, bound address) ←----- |
  |                                       |
  |← encrypted/arbitrary data tunneled via proxy →|
```

### Authentication

SOCKS5 supports username/password, GSSAPI (Kerberos), or no auth. Authentication is negotiated before connection.

### UDP Support

SOCKS5 can tunnel UDP by opening relay association; client sends UDP packets to proxy, proxy forwards to target.

**Use case**: SSH tunneling, VPN tunneling, game clients behind proxy.

## HTTP CONNECT Tunneling

HTTP clients can use the `CONNECT` method to tunnel TCP connections through HTTP proxies:

```
CLIENT                        PROXY                        SERVER
  |                             |                             |
  |--- CONNECT host:port HTTP/1.1 -→                         |
  |                             |--- TCP connection to host:port
  |                             |← connection established
  |← HTTP/1.1 200 Connection Established ←                   |
  |                             |                             |
  |------ HTTPS/encrypted ------→ | --- HTTPS/encrypted ----→ |
```

Client sends `CONNECT server.com:443`. Proxy establishes TCP connection to target and returns 200. All subsequent data flows transparently (tunnel mode). Used for HTTPS through corporate proxy.

**Limitation**: Only works for TCP; typically used for HTTPS (port 443) but extensible to any port.

## Proxy Protocol (v1/v2)

When a reverse proxy forwards traffic to a backend, the backend loses information about the original client. **Proxy Protocol** is a simple header that restores it:

**v1 (human-readable)**:
```
PROXY TCP4 203.0.113.5 192.0.2.1 12345 80\r\n
```
Signals: connection originated from `203.0.113.5:12345`, destination was `192.0.2.1:80`.

**v2 (binary, extensible)**:
Compact format with TLV (type-length-value) extension fields for certificates, VLANs, etc.

### Use

Reverse proxy sends this line before any protocol data. Backend extracts original client IP for logging, rate-limiting, or security policy.

**Limitation**: Non-standard; only recognized by configured backends. NGINX, HAProxy, Envoy support it.

## Split Tunneling

A client connects to a VPN/proxy but routes specific traffic directly (bypassing proxy) based on rules:

```
Client → Traffic for 10.0.0.0/8 → Direct
Client → Other traffic → Proxy/VPN
```

User defines which traffic uses which path. **Advantage**: Reduces latency for local traffic. **Disadvantage**: Traffic not proxied may be sniffed if not encrypted end-to-end.

## Proxy Chaining

Proxies connect in series. Client connects to proxy A, which connects to proxy B, which connects to server.

```
Client → Proxy A → Proxy B → Server
         [forward]  [forward]
```

**Use**: Nested security policies (corporate proxy → ISP proxy → external network), load distribution, or geographic routing.

**Overhead**: Each hop adds latency; throughput degrades. SOCKS supports chaining natively (downstream proxy specified in CONNECT request).

## Performance Considerations

| Proxy Type        | Throughput | Latency | Connection Overhead |
|-------------------|-----------|---------|---------------------|
| Forward (cached)  | 10K–1M req/s | +10–50ms | Low (connection reuse) |
| Reverse (simple)  | 100K–1M req/s | +5–20ms | Low (keepalive) |
| Transparent       | 10K–100K req/s | +10–50ms | Kernel overhead |
| SOCKS             | 10K–100K req/s | +20–100ms | High (bidirectional) |

Reverse proxy (NGINX, HAProxy) preferred for performance; overhead is connection multiplexing and HTTP parsing, both optimized.

## Security Implications

- **Proxy can see plaintext**: Unless traffic is encrypted end-to-end, proxy operator sees content
- **Proxy can MITM**: Transparent proxy + certificate injection can intercept HTTPS
- **IP spoofing**: Proxy Protocol v1 is unencrypted; unreliable for trust decisions without mTLS

## See Also

- [devops-nginx.md](devops-nginx.md) — NGINX configuration and internals
- [devops-service-mesh.md](devops-service-mesh.md) — Service mesh proxy architecture
- [architecture-api-gateway.md](architecture-api-gateway.md) — API gateway patterns
- [networking-tcp-ip.md](networking-tcp-ip.md) — TCP/IP layer model