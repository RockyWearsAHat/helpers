# Border Gateway Protocol (BGP) — Autonomous Systems, Path Attributes, Route Selection & Security

## Overview

Border Gateway Protocol (BGP) is the routing protocol that powers the internet's interdomain routing. It enables routers in different Autonomous Systems (AS) to exchange reachability information and determine the best paths for traffic forwarding based on administrative policies and network topology. Unlike interior gateway protocols (OSPF, IS-IS), BGP operates on *policy* first and *optimization* second—the "best" path is not necessarily the shortest, but the path that meets business logic and contractual relationships.

BGP is a **path-vector protocol**: vs. distance-vector or link-state, it carries full path information through the ASes a route has traversed, enabling rich policy decisions and loop prevention.

## Autonomous Systems and AS Numbers

An **Autonomous System** is a collection of IP prefixes and routers under a single administrative domain (typically an ISP, content provider, or enterprise network). Each AS is identified by an **ASN** (Autonomous System Number):
- **2-byte ASN** (original): 0–65535. Numbered 64512–65534 are reserved for private use (analogous to RFC 1918 addresses).
- **4-byte ASN** (RFC 5396, 2008+): 0–4,294,967,295. Enables vastly more public ASN allocations. Represented in AS-path as consecutive 2-byte values when needed (upstream systems must support 4-byte ASN to interpret correctly).

ASNs are allocated regionally by RIRs (APNIC, RIPE, ARIN, LACNIC, AFRINIC). Public ASNs are heavily guarded; private ASNs used for private peering or customer edges where AS-path is not exposed globally.

## BGP Peering: iBGP vs eBGP

- **eBGP (External BGP)**: Peers in *different* autonomous systems. Exchanged between ISP routers and their customers, or between ISP routers and peers. eBGP peers are typically directly connected (connected via one hop). **TTL is 1** by default to prevent accidental routing across the internet; multi-hop eBGP requires explicit configuration and is rare (usually only between data center sites under same administrative control).
- **iBGP (Internal BGP)**: Peers within the *same* autonomous system. All iBGP speakers must establish a full-mesh of connections, or use route reflectors to avoid O(n²) complexity. iBGP routes received from one peer are **not re-advertised to other iBGP peers** by default (split-horizon rule) — a critical difference from eBGP. This prevents redundant flooding; external routes are advertised to eBGP peers instead.

## BGP Attributes and Path Selection

Each BGP route carries **path attributes** metadata used during selection:

### Well-Known Mandatory Attributes
- **AS-Path**: Ordered list of ASN's the route has traversed. Prevents loops (any router receiving its own ASN discards the route). Format in UPDATE messages is typically in AS-path segments (e.g., sequences or sets of ASNs). Shorter AS-paths are generally considered "better."
- **Next-Hop**: IP address of the neighboring router that advertised this route. Used to determine where to forward packets.
- **Origin**: Source of the route: `i` (IGP — learned from iBGP/eBGP within AS), `e` (EGP — obsolete protocol), or `?` (incomplete — learned by other means).

### Well-Known Discretionary Attributes
- **Local-Preference**: Integer (typically 0–100, but can be higher) set locally within an AS. Higher values are preferred. Default is 100. Used to prefer certain exit points or providers. *Never* leaves the AS; discarded when sending eBGP.
- **MED (Multi-Exit Discriminator)**: Hint to a peer AS about which entry point is preferred (e.g., "use this link, not that one"). Lower MED is better. Unlike Local-Pref, MED is sent to eBGP peers. Can be risky if misused—an attacker or misconfigured peer can influence your inbound traffic engineering.
- **Communities**: Optional tag (typically `ASN:value` format, 32-bit) attached to routes. Communities enable flexible policy grouping without requiring AS-to-AS negotiation. Standard communities are well-documented (e.g., `65001:100` = "prepend once"). Large communities (RFC 8092) extend this to 96-bit for future scalability.

### Selective/Transitive Attributes
- **Atomic Aggregate**: Set when a AS reduces many routes into one aggregate. Downstream ASes know not to lower Local-Pref or apply community-based policies that assume the route is more specific.
- **Aggregator**: ASN and IP of the router that created the aggregate.
- **Community, Extended Community**: Metadata for policy application across AS boundaries.

### BGP Best Path Algorithm

When a router receives multiple routes to the same prefix:

1. **Prefer higher Local-Preference** (within own AS).
2. **Prefer shorter AS-path length**.
3. **Prefer lower origin type** (`i` > `e` > `?`).
4. **Prefer lower MED** (if comparing routes from same peer AS; usually not comparing across different peers).
5. **Prefer eBGP-learned over iBGP-learned** routes (unless the iBGP route has a shorter AS-path).
6. **Prefer routes via the router with the lowest BGP router ID**.
7. **Prefer routes via the lowest next-hop IP** (tie-breaker).

This algorithm is deterministic but complex—network operators often pre-determine outcomes by tuning Local-Pref, AS-path prepending, and MED. The algorithm is configurable on modern routers (additional tie-breakers for traffic engineering).

## Route Reflectors and Confederations

### Route Reflectors (RFC 4456)

In a full-mesh iBGP topology, N routers require N×(N−1)/2 connections—scaling becomes unmanageable at 100+ routers per AS. **Route Reflectors** (RRs) break the full-mesh requirement:

- One or more RRs act as hubs. All other routers (RR clients) peer only with RRs, not each other.
- The RR receives routes from clients and **re-advertises them to other clients** (violating the normal split-horizon rule). This is called "reflection."
- RR clients and other RRs in the same cluster use a special `Originator-ID` and `Cluster-List` attributes to prevent loops.
- **RR hierarchy**: Multiple route reflectors can be structured hierarchically (RRs peering with RRs) for large networks.

Route reflectors are widely used and operationally simpler than confederations, but introduce a critical failure point if the RR goes down. Best practice: 2–3 RRs per cluster, fully meshed together.

### Confederations (RFC 5065)

An alternative to RRs. Divide a single AS into multiple **sub-AS** groups (each with a unique private sub-ASN), each running full-mesh iBGP internally:

- Sub-ASes peer with each other using eBGP but use a special attribute (`Confed-Sequence`) to track the path through the confederation.
- Routes retain the original AS-path (the real public ASN is prepended once when leaving the confederation).
- **Advantages over RRs**: Distributed architecture, no single point of failure, more explicit loop prevention.
- **Disadvantages**: More complex to configure and debug. Rarely used in practice; RRs are simpler.

## Route Filtering and Community-Based Policies

### Filtering Inbound Routes
- **Prefix-list or AS-path access list**: Accept or deny routes based on prefix, ASN, or AS-path patterns.
- **Local-Pref adjustment**: Routes matching certain criteria get boosted Local-Pref (e.g., routes from Tier-1 providers get 150, peers get 120).
- **Example**: ISP receives 10 routes to 192.0.2.0/24. Only the /24 from a preferred peer or upstream is accepted; /25s and more-specifics are filtered to avoid transit.

### Community-Based Policies
Communities allow customers and peers to "tag" routes with metadata:
- **Standard communities** (32-bit, format `ASN:value`): Well-known values documented by operators.
- **Operator-defined communities**: An ISP might define `65001:100` = "high-priority" or `65001:1000` = "do not announce to Tier-2 peers."
- **BGP Large Communities** (RFC 8092): 96-bit format, enables three 32-bit fields for richer hierarchies (administrator, community, value).

Example: A customer wants certain prefixes advertised only to specific peers. They attach a community tag; the ISP's routers check for that tag and apply corresponding export policies.

## BGP Hijacking and Security

**BGP Hijacking** (prefix hijacking) is the announcement of an IP prefix by an AS that doesn't own it:

- **Accidental**: Misconfiguration or router/operator error. Router accidentally originates a prefix it doesn't have authority to announce.
- **Malicious**: An attacker AS (or a compromised router) announces a prefix to redirect traffic. Example: AS hijacks a bank's public IP range and becomes MITM for traffic destined to that bank.
- **Impact**: Traffic intended for the legitimate owner gets routed to the hijacker. The hijacker sees cleartext or can selectively intercept.

### RPKI (Resource Public Key Infrastructure, RFC 6811 + RFC 8210)

**RPKI** is the primary defense mechanism:

- **ROA (Route Origin Authorization)**: A cryptographically signed statement issued by the IP prefix owner, listing authorized AS(es) that may originate that prefix. "192.0.2.0/24 may only be originated by AS65001."
- **ROA validation**: Routers receiving BGP routes check ROAs in the public RPKI database. A route is Valid (matches ROA), Invalid (violates ROA), or Unknown (no ROA issued).
- **Policies**:
  - **Reject Invalid routes** (strict): Drop routes that violate a ROA.
  - **Deprioritize Invalid routes** (softer): Lower Local-Pref, allowing fallback if all routes are Invalid.
  - **Monitor Unknown** (permissive): Accept Unknown routes but log for audit.

**RPKI limitations**:
- Only validates the origin AS; doesn't prevent AS-path manipulation or MED attacks.
- Deployment is still partial (many operators haven't issued ROAs; many routers don't validate).
- RPKI is eventually consistent—newly issued ROAs take hours/days to propagate to all validators.

### Other BGP Security Measures

- **BGPsec**: End-to-end path validation (signs AS-path cryptographically). Standardized but rarely deployed due to CPU cost and operational overhead.
- **MANRS (Mutually Agreed Norms for Routing Security)**: Best practices initiative. Members commit to filtering invalid prefixes, preventing customer hijacking, and coordinating incident response.
- **Filtering**: Many ISPs apply strict prefix filters at customer/peer interfaces or validate customer-originated prefixes against WHOIS/IRR.

## Convergence Behavior and Stability

BGP can take **10+ seconds to many minutes** to converge after a topology change (e.g., a link failure or new route announcement):

- **Initial convergence**: First path from a peer arrives in seconds (typically <1 RTT).
- **Flapping**: If a route disappears and reappears repeatedly (link flapping), BGP applies **route-flap dampening**. The route is suppressed for an exponentially increasing period, preventing oscillation.
- **Path exploration**: When the primary route is withdrawn, a router tries alternative routes. With many RRs or peers, this can cause temporary inconsistency—some routers take the new route, others don't yet.
- **Slow convergence factors**:
  - **Route refresh**: iBGP full updates are sent only on initial session establishment; incremental updates are sent thereafter. Misconfigured or older routers may require manual refresh.
  - **Policy processing**: Complex route maps (filtering, community detection, Local-Pref adjustment) can slow processing.
  - **Slow peers**: If a single peer sends updates slowly, the router waits.

**Best practices to improve convergence**:
- Use fast link-failure detection (BFD — Bidirectional Forwarding Detection).
- Minimize AS-path prepending depth.
- Avoid large received-route limits that force memory pressure.

## Common BGP Misconfigurations

- **Forgetting split-horizon in iBGP**: Advertising the same route to all iBGP peers causes loops.
- **Confusing Local-Pref with MED**: Local-Pref is only internal; MED sent to peers but only among routes from the same peer AS.
- **AS-path prepending too aggressively**: Prepending 10 times to deprioritize a route can backfire if all other paths are withdrawn.
- **Not filtering customer routes**: A customer announces prefixes they don't own; the ISP should validate against WHOIS before accepting.
- **RPKI ROA mistakes**: Issuing ROA with wrong ASN or prefix range can black-hole traffic.

## See Also

- [Networking Protocols — Packets to Applications](networking-protocols.md) — layered model context
- [Infrastructure Load Balancing](infrastructure-load-balancing.md) — intra-AS routing vs inter-AS BGP
- [Security — Network Security](security-network.md) — BGP hijacking and DoS from internet perspective