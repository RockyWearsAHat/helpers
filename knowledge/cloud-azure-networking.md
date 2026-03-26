# Azure Networking Concepts — VNets, Load Balancers & Connectivity

## Virtual Networks (VNets)

Azure Virtual Networks provide logically isolated network environments within the Azure cloud. Each VNet defines an address space using CIDR notation, subdivided into subnets that segment workloads by function, security tier, or compliance boundary.

### Address Space and Subnets

| Concept            | Role                                        | Considerations                                                               |
| ------------------ | ------------------------------------------- | ---------------------------------------------------------------------------- |
| Address space      | Defines the private IP range for the VNet   | Overlapping ranges prevent peering; plan across environments                 |
| Subnets            | Subdivisions within the address space       | Each subnet gets a contiguous range; some services require dedicated subnets |
| Reserved addresses | Azure reserves 5 IPs per subnet             | First four and last address unavailable for workloads                        |
| Delegated subnets  | Subnets assigned to specific Azure services | Required by services like Azure Container Instances, SQL Managed Instance    |

Planning address spaces across VNets, on-premises networks, and future growth is one of the more consequential early decisions — mistakes compound as peering relationships and hybrid connections multiply.

### Network Security Groups (NSGs)

NSGs act as stateful packet filters applied at the subnet or NIC level. Rules evaluate by priority (lower numbers first), matching on source/destination IP, port, and protocol.

```
Priority  Direction  Action  Source         Dest          Port   Protocol
100       Inbound    Allow   10.0.1.0/24    10.0.2.0/24   443    TCP
200       Inbound    Allow   VirtualNetwork VirtualNetwork *      *
65000     Inbound    Allow   AzureLB        *              *      *
65500     Inbound    Deny    *              *              *      *
```

Applying NSGs at the subnet level provides broad coverage; applying at the NIC level provides per-workload granularity. Organizations often combine both, though this can create debugging complexity when traffic is blocked at an unexpected layer.

### Service Endpoints and Service Tags

Service endpoints extend the VNet identity to Azure PaaS services, routing traffic over the Azure backbone rather than the public internet. Service tags abstract Azure service IP ranges into named groups (e.g., `Storage`, `Sql`, `AzureCloud`) for use in NSG rules, reducing the maintenance burden of tracking IP ranges.

Service endpoints improve security posture but have limitations — they don't provide private IP addresses for the target service, and they only affect traffic originating from within the VNet. Private Endpoints (covered below) address these gaps at higher complexity.

## Load Balancing — Layer 4 vs Layer 7

Azure offers multiple load-balancing services operating at different network layers, each suited to different traffic patterns and architectural requirements.

### Azure Load Balancer (Layer 4)

Operates at the transport layer, distributing TCP/UDP flows based on a 5-tuple hash (source IP, source port, destination IP, destination port, protocol). Does not inspect application-layer content.

**Characteristics:**

- Ultra-low latency — no TLS termination or content inspection overhead
- Supports internal (private) and external (public) configurations
- Health probes detect backend pool member availability
- Outbound rules control SNAT behavior for internet-bound traffic
- Cross-zone redundancy with zone-redundant frontends

**Common contexts:** Database clusters, non-HTTP protocols, high-throughput east-west traffic between services, NVA (network virtual appliance) load balancing.

### Application Gateway (Layer 7)

Operates at the application layer with full HTTP/HTTPS awareness. Inspects headers, URL paths, cookies, and host names for routing decisions.

**Capabilities:**

- Path-based and host-based routing to different backend pools
- TLS termination and re-encryption (offloading or end-to-end)
- Web Application Firewall (WAF) integration with OWASP rule sets
- Session affinity via cookie-based routing
- URL rewriting and header manipulation
- Autoscaling based on traffic patterns

**Trade-offs vs L4 load balancing:**

| Dimension         | L4 Load Balancer | L7 Application Gateway      |
| ----------------- | ---------------- | --------------------------- |
| Latency           | Lower            | Higher (content inspection) |
| Protocol support  | Any TCP/UDP      | HTTP/HTTPS/WebSocket        |
| SSL offload       | No               | Yes                         |
| URL-based routing | No               | Yes                         |
| WAF capability    | No               | Yes (integrated)            |
| Cost model        | Per-rule + data  | Per-instance-hour + data    |
| Complexity        | Simpler          | More configuration surface  |

Many architectures combine both — Application Gateway for ingress from external clients, internal Load Balancer for service-to-service communication where HTTP awareness is unnecessary.

## Azure Front Door — Global Load Balancing

Front Door operates at the network edge, providing global HTTP/HTTPS load balancing with anycast routing. Clients connect to the nearest Front Door POP (point of presence), and traffic routes to the optimal backend based on latency, priority, or weighted algorithms.

**Key concepts:**

- **Anycast acceleration** — TCP and TLS handshakes happen at the edge POP, reducing round-trip time for distant clients
- **WAF policies** — rate limiting, geo-filtering, bot protection, and custom rules at the edge
- **Caching** — static content served from edge POPs without reaching origin servers
- **Session affinity** — optional sticky sessions for stateful applications
- **Private Link origins** — backends can remain fully private, reachable only through Front Door

Front Door is most beneficial when serving geographically distributed users or when edge security (DDoS, WAF) is a priority. For single-region deployments with modest traffic, Application Gateway may suffice with less operational overhead.

## DNS and Traffic Management

### Azure DNS

Hosts DNS zones on Azure's globally distributed name server infrastructure. Supports both public zones (internet-resolvable) and private zones (VNet-scoped resolution).

**Private DNS zones** enable name resolution within and across VNets without custom DNS servers. A private zone linked to a VNet allows VMs in that VNet to resolve records in the zone, which simplifies service discovery in multi-VNet architectures.

### Traffic Manager

A DNS-based traffic distribution service operating at the DNS layer — it returns different DNS responses based on configured routing methods, not proxying actual data traffic.

| Routing method | Behavior                                              | Typical use                            |
| -------------- | ----------------------------------------------------- | -------------------------------------- |
| Priority       | Failover to secondaries when primary is degraded      | Active-passive disaster recovery       |
| Weighted       | Distribute traffic by assigned weight ratios          | Canary deployments, gradual migrations |
| Performance    | Route to the endpoint with lowest latency from client | Multi-region deployments               |
| Geographic     | Route based on client's geographic origin             | Data sovereignty, localized content    |
| Multivalue     | Return multiple healthy endpoints                     | Client-side failover                   |
| Subnet         | Map specific client subnets to specific endpoints     | Enterprise network partitioning        |

Traffic Manager and Front Door both provide global routing, but at different layers — Traffic Manager at DNS (returns an IP, client connects directly to backend) vs Front Door at the application layer (proxies all traffic through edge POPs). The choice depends on whether edge processing, caching, and WAF are needed.

## Hybrid Connectivity

### VPN Gateway

Establishes encrypted tunnels between Azure VNets and on-premises networks (or other clouds) over the public internet.

**Topologies:**

- **Site-to-site (S2S)** — persistent tunnel between on-premises VPN device and Azure VPN Gateway
- **Point-to-site (P2S)** — individual client connections (developer workstations, mobile devices)
- **VNet-to-VNet** — encrypted tunnel between Azure VNets (alternative to peering when encryption in transit is required)

VPN Gateway SKUs range from basic development scenarios to high-throughput production (aggregated throughput varies from ~100 Mbps to ~10 Gbps depending on SKU and active-active configuration).

### ExpressRoute

Private connectivity between on-premises infrastructure and Azure, not traversing the public internet. Established through a connectivity provider at a peering location.

**Characteristics:**

- Bandwidth options from 50 Mbps to 100 Gbps
- Predictable latency (no internet routing variability)
- Supports Microsoft peering (Microsoft 365, Dynamics) and private peering (Azure VNets)
- ExpressRoute Global Reach connects on-premises sites through the Microsoft backbone
- ExpressRoute Direct provides dedicated physical ports at peering locations

**When organizations tend toward ExpressRoute over VPN:**

- High-throughput, latency-sensitive workloads (database replication, real-time analytics)
- Compliance requirements mandating traffic stays off the public internet
- Consistent performance SLAs needed beyond what internet-based VPN can guarantee

Many architectures use both — ExpressRoute as the primary path with VPN Gateway as failover, providing redundancy across different connectivity models.

## Private Link and Private Endpoints

Private Link enables access to Azure PaaS services (Storage, SQL Database, Cosmos DB, and many others) over a private IP address within a VNet, rather than through public endpoints.

**Architecture:**

```
Consumer VNet                        Provider Service
┌──────────────┐                    ┌──────────────┐
│  VM / App    │──── Private ────▶  │  Azure SQL   │
│  10.0.1.5    │    Endpoint        │  (private IP │
│              │    10.0.2.4        │   in VNet)   │
└──────────────┘                    └──────────────┘
    No traffic traverses the public internet
```

**Private Endpoint** is the network interface that connects a VNet to a Private Link-enabled service, receiving a private IP from the VNet's address space.

**DNS resolution** is the operational nuance that trips up many implementations — when a private endpoint is created, the FQDN (e.g., `mydb.database.windows.net`) must resolve to the private IP rather than the public IP. Azure Private DNS zones automate this, but hybrid environments with on-premises DNS forwarders require careful configuration.

**Trade-offs:**

- Eliminates data exfiltration through public endpoints
- Adds DNS complexity, especially in hybrid and multi-VNet environments
- Each private endpoint consumes an IP from the subnet
- Cost is per-private-endpoint-hour plus data processing

## VNet Peering

VNet peering connects two VNets, enabling resources in either to communicate as if on the same network. Traffic between peered VNets uses the Azure backbone.

| Peering type | Scope             | Latency characteristics                 |
| ------------ | ----------------- | --------------------------------------- |
| Regional     | Same Azure region | Lowest latency, same as intra-VNet      |
| Global       | Cross-region      | Higher latency, still on Azure backbone |

**Key behaviors:**

- Peering is non-transitive — if VNet A peers with B and B peers with C, A cannot reach C unless explicitly peered or using a transit mechanism (hub VNet with routing)
- Address spaces cannot overlap between peered VNets
- NSG rules still apply to peered traffic
- Peering must be established from both sides (bidirectional configuration)

## Azure Firewall

A managed, stateful firewall service providing centralized network security with built-in high availability and cloud scalability.

**Rule types (evaluated in order):**

1. **NAT rules** — DNAT for inbound traffic
2. **Network rules** — L3/L4 filtering by IP, port, protocol
3. **Application rules** — FQDN-based filtering for outbound HTTP/HTTPS

**Capabilities:**

- Threat intelligence-based filtering (block known malicious IPs/FQDNs)
- TLS inspection for encrypted traffic analysis
- DNS proxy to centralize DNS resolution
- Forced tunneling for compliance scenarios requiring all traffic inspection
- Integration with Azure Monitor and Log Analytics for traffic visibility

Azure Firewall is commonly deployed in the hub of a hub-and-spoke topology, centralizing security policy enforcement and traffic inspection across spoke VNets.

## Hub-and-Spoke Topology

The prevailing architecture pattern for organizing Azure networking at scale:

```
                    ┌──────────────┐
                    │     Hub      │
                    │   VNet       │
     ┌──────────────┤  - Firewall  ├──────────────┐
     │              │  - VPN GW    │              │
     │              │  - Bastion   │              │
     │              └──────┬───────┘              │
     │                     │                      │
┌────▼─────┐        ┌─────▼──────┐        ┌──────▼─────┐
│  Spoke 1 │        │  Spoke 2   │        │  Spoke 3   │
│  Web tier │        │  App tier  │        │  Data tier │
└──────────┘        └────────────┘        └────────────┘
```

**Hub** contains shared services: firewall, VPN/ExpressRoute gateways, Bastion host, DNS. **Spokes** contain workloads, peered to the hub. User-defined routes (UDRs) in spoke subnets direct traffic through the hub firewall for inspection.

**Advantages:** Centralized security and connectivity, workload isolation, consistent policy enforcement, cost sharing of gateway resources.

**Complexities:** UDR management grows with spoke count, transitive routing requires explicit configuration, hub becomes a throughput bottleneck if undersized.

Azure Virtual WAN offers a managed alternative that automates much of the hub-and-spoke routing and transit connectivity, trading control for operational simplicity.

## Network Watcher and Diagnostics

Network Watcher provides diagnostic and monitoring capabilities for troubleshooting Azure networking issues.

| Tool                    | Purpose                                                      |
| ----------------------- | ------------------------------------------------------------ |
| IP flow verify          | Tests whether a packet is allowed or denied by NSG rules     |
| Next hop                | Determines the next hop for a given destination              |
| Connection troubleshoot | Tests connectivity between source and destination            |
| NSG flow logs           | Captures traffic metadata passing through NSGs               |
| Packet capture          | Records packets at the VM NIC for deep analysis              |
| Connection monitor      | Continuous monitoring of connectivity between endpoints      |
| Topology view           | Visual representation of network resources and relationships |

NSG flow logs, when directed to Log Analytics, provide the telemetry foundation for traffic analysis, anomaly detection, and compliance auditing.

## Mapping to On-Premises Concepts

| On-premises concept     | Azure equivalent(s)            | Key differences                        |
| ----------------------- | ------------------------------ | -------------------------------------- |
| Physical network / VLAN | VNet / Subnet                  | Software-defined, API-managed          |
| Hardware firewall       | Azure Firewall / NSG           | Managed service, auto-scaling          |
| F5 / HAProxy (L4)       | Azure Load Balancer            | No appliance management                |
| Nginx / HAProxy (L7)    | Application Gateway            | Managed WAF, auto-scaling              |
| MPLS circuit            | ExpressRoute                   | Provider-managed, Azure backbone       |
| IPsec VPN               | VPN Gateway                    | Managed endpoints, multiple SKUs       |
| Core router             | VNet peering + UDRs            | Route tables replace routing protocols |
| DNS server              | Azure DNS (public/private)     | Globally distributed, VNet-integrated  |
| Network TAP / SPAN      | Network Watcher packet capture | API-driven, no physical cabling        |

The fundamental shift from on-premises networking: physical constraints (port counts, cable runs, device firmware) are replaced by quota limits, API rate limits, and service-specific constraints. Network-as-code through ARM templates, Bicep, or Terraform enables versioned, repeatable network deployments — a capability that changes how networking teams operate, not just what they deploy.

## Common Architectural Considerations

**IP address planning** — the most common regret in Azure networking emerges from narrow initial address space allocation. Address spaces cannot be changed on peered VNets without tearing down peering. Plan for growth across all anticipated VNets, hybrid connections, and container networking needs.

**DNS resolution chain** — traffic flows and name resolution flows are independent in Azure. A VM can reach a service by IP but fail by FQDN if DNS is misconfigured. Private endpoints, conditional forwarders, and hybrid DNS create a resolution chain that requires explicit design.

**Cost awareness** — VNet peering, VPN Gateway, ExpressRoute, Azure Firewall, and Private Endpoints each carry distinct cost models (per-GB, per-hour, per-connection). A hub-and-spoke with full firewall inspection and multiple private endpoints can accumulate significant networking costs separate from compute and storage.

**Regional constraints** — not all Azure networking services are available in all regions, and some services have per-region limits on instance counts or throughput. Multi-region architectures need to account for these asymmetries.
