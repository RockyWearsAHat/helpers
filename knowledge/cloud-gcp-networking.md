# GCP Networking Concepts — VPC, Load Balancing & Connectivity

## Virtual Private Cloud as a Global Resource

GCP's VPC model differs fundamentally from most cloud networking implementations. A VPC in GCP is a **global resource** — it spans all regions without requiring explicit peering between regional segments. Subnets, by contrast, are regional resources that exist within a VPC and define IP address ranges for a specific region.

This global-by-default design has several implications:

| Aspect                     | GCP Global VPC                         | Traditional Regional VPC             |
| -------------------------- | -------------------------------------- | ------------------------------------ |
| Cross-region communication | Automatic within same VPC              | Requires peering or transit gateways |
| Routing complexity         | Single routing domain                  | Per-region route tables              |
| Firewall scope             | VPC-wide rules apply globally          | Often region-scoped                  |
| IP management              | Subnets are regional within global VPC | Subnets tied to availability zones   |
| Blast radius               | Misconfiguration affects all regions   | Contained to one region              |

The global model simplifies multi-region deployments but introduces trade-offs around isolation. A single firewall misconfiguration propagates everywhere, whereas regional VPC models contain errors by default.

## Subnet Design and IP Address Management

Subnets in GCP use RFC 1918 private address space (or custom ranges) and can operate in **auto mode** or **custom mode**:

- **Auto mode** — GCP assigns a predefined `/20` subnet in each region automatically. Convenient for prototyping but limits control over CIDR allocation and can create conflicts in hybrid environments.
- **Custom mode** — Operators define subnet CIDR ranges explicitly. Required for production environments where IP planning matters, especially with VPN or Interconnect back to on-premises networks.

Key IP address management concepts:

- **Primary ranges** define the main CIDR block for VM interfaces
- **Secondary ranges** (alias IP ranges) enable multiple IP ranges per subnet, commonly used for container networking where pods need distinct IP space from nodes
- **Private Google Access** allows VMs without external IPs to reach Google APIs through internal routing
- **Subnet expansion** permits growing a subnet's CIDR range without downtime, though shrinking is not supported

IP exhaustion planning involves balancing between over-provisioning (wasting address space, creating routing table bloat) and under-provisioning (running out of addresses, requiring subnet migration). Secondary ranges for container workloads often need significantly larger allocations than initially expected.

## Firewall Rules as Stateful Packet Filtering

GCP firewall rules operate at the VPC level as **distributed, stateful packet filters**. They are not implemented at a chokepoint appliance but enforced at each VM's virtual NIC by the hypervisor.

```
Firewall Rule Components:
┌─────────────────────────────────────────────┐
│  Direction:    ingress | egress             │
│  Priority:     0 (highest) – 65535 (lowest) │
│  Action:       allow | deny                 │
│  Target:       all instances | tag | SA     │
│  Source/Dest:  IP ranges | tags | SAs       │
│  Protocol:     TCP, UDP, ICMP, or all       │
│  Ports:        specific ports or ranges     │
│  Enforcement:  enabled | disabled           │
└─────────────────────────────────────────────┘
```

Firewall design considerations:

- **Tag-based targeting** — Rules can target VMs by network tags or service accounts. Service account targeting tends to be more robust since tags can be set by anyone with instance admin permissions, while service account assignment requires IAM control.
- **Priority resolution** — When multiple rules match, the lowest priority number wins. Ties between allow and deny at the same priority resolve to deny. This priority system enables layered policy: broad deny-all at low priority, specific allows at higher priority.
- **Implied rules** — Every VPC has two implied rules: deny-all-ingress at priority 65535 and allow-all-egress at priority 65535. The deny-all-ingress means nothing reaches VMs unless explicitly permitted.
- **Hierarchical firewall policies** — Organization and folder-level policies that apply before VPC-level rules, enabling central security teams to enforce baseline controls that project-level rules cannot override.

The stateful nature means return traffic for established connections is automatically permitted, reducing rule count but occasionally complicating debugging when asymmetric routing exists.

## Load Balancing Spectrum

GCP offers a range of load balancers that differ along several axes: protocol layer, scope (regional vs global), and traffic direction (external vs internal).

### External Load Balancing

| Type                       | Layer    | Scope    | Typical Use                               |
| -------------------------- | -------- | -------- | ----------------------------------------- |
| External HTTP(S)           | L7       | Global   | Web applications, content-based routing   |
| SSL Proxy                  | L4 (TLS) | Global   | Non-HTTP TLS traffic                      |
| TCP Proxy                  | L4       | Global   | Non-HTTP TCP traffic needing global reach |
| External Network (TCP/UDP) | L4       | Regional | Non-proxied pass-through, UDP workloads   |

### Internal Load Balancing

| Type                  | Layer | Scope    | Typical Use                        |
| --------------------- | ----- | -------- | ---------------------------------- |
| Internal HTTP(S)      | L7    | Regional | Service-to-service HTTP routing    |
| Internal TCP/UDP      | L4    | Regional | Database tiers, internal services  |
| Cross-region Internal | L7    | Global   | Multi-region internal service mesh |

**Global vs regional trade-offs:**

Global load balancers use a single anycast IP address. Traffic enters the Google network at the nearest edge point of presence and traverses Google's backbone to the closest healthy backend. This reduces latency for geographically distributed users but means all traffic routes through Google's proxy infrastructure, adding a small processing overhead.

Regional load balancers keep traffic within a single region. They suit workloads where data locality matters (compliance, latency to a specific backend) or where pass-through (non-proxied) behavior is needed for protocols that don't work well with proxies.

### Connection Draining and Health Checks

Health checks determine backend availability. They operate independently of the load balancer type and support HTTP(S), TCP, SSL, and gRPC probes. Configuration involves:

- **Check interval and timeout** — Aggressive settings detect failures faster but may cause flapping
- **Healthy/unhealthy thresholds** — Number of consecutive successes or failures before state changes
- **Health check scope** — Whether the probe goes to a specific port and path or a dedicated health endpoint

## Cloud DNS Concepts

Cloud DNS provides managed authoritative DNS with 100% SLA for authoritative name resolution. Key concepts:

- **Managed zones** map to DNS zones (public or private)
- **Private zones** resolve only within specified VPC networks, enabling split-horizon DNS where the same domain resolves differently internally vs externally
- **DNS peering** forwards queries from one VPC's private zone to another VPC's DNS resolution, useful in hub-and-spoke network topologies
- **DNSSEC** provides cryptographic verification of DNS responses for public zones
- **DNS policies** enable inbound forwarding (on-premises resolving GCP private zones) and outbound forwarding (GCP resolving on-premises DNS)

DNS resolution order in GCP: internal metadata DNS → private zones → DNS peering → forwarding zones → public resolution. Understanding this chain matters when debugging "name not found" issues in complex hybrid environments.

## Cloud CDN and Edge Caching

Cloud CDN caches HTTP(S) content at Google's edge points of presence. It integrates with the external HTTP(S) load balancer and supports backend services (VMs, containers) and Cloud Storage buckets as origins.

Caching behavior considerations:

- **Cache keys** default to the full request URI but can include/exclude query parameters, headers, and cookies for more granular control
- **Cache modes** range from automatic (respect origin Cache-Control headers) to force caching with configurable TTL
- **Signed URLs and signed cookies** restrict access to cached content with time-limited tokens
- **Cache invalidation** propagates globally but is eventually consistent — not instantaneous

The trade-off between aggressive caching (lower origin load, faster responses, potential staleness) and conservative caching (fresher content, higher origin load) depends on content volatility and tolerance for stale data.

## VPN and Cloud Interconnect for Hybrid Connectivity

Hybrid connectivity bridges GCP networks with on-premises or other cloud environments:

### Cloud VPN

- **Classic VPN** — Single tunnel per gateway, supports static and dynamic routing
- **HA VPN** — Two tunnels with 99.99% SLA, requires BGP dynamic routing, supports active/active and active/passive configurations

VPN tunnels encrypt traffic over the public internet. Bandwidth is limited per tunnel (approximately 3 Gbps per tunnel for HA VPN), though multiple tunnels can aggregate. Latency depends on internet path quality.

### Cloud Interconnect

| Type                     | Bandwidth         | Latency            | Cost Profile                    |
| ------------------------ | ----------------- | ------------------ | ------------------------------- |
| Dedicated Interconnect   | 10-200 Gbps       | Low, deterministic | High fixed cost, low per-GB     |
| Partner Interconnect     | 50 Mbps - 50 Gbps | Moderate           | Lower commitment, higher per-GB |
| Cross-Cloud Interconnect | 10-100 Gbps       | Low                | Cloud-to-cloud direct links     |

Dedicated Interconnect provides private physical connections at colocation facilities. It bypasses the public internet entirely, offering predictable latency and higher throughput. The trade-off is significant upfront commitment and the requirement to be present (or have a partner) at a supported colocation facility.

### Choosing Between VPN and Interconnect

Factors include bandwidth requirements, latency sensitivity, cost tolerance, and whether traffic encryption at the network layer is mandatory. VPN encrypts by default; Interconnect traffic traverses private connections but is not encrypted at the link layer (MACsec is available for Dedicated Interconnect). Some architectures combine both — Interconnect for bulk data transfer, VPN as backup.

## Private Google Access and Private Service Connect

Private connectivity to Google services avoids sending traffic over the internet:

- **Private Google Access** — Enables VMs without external IPs to reach Google APIs (storage, BigQuery, etc.) via internal routing. Configured per-subnet.
- **Private Service Connect** — Creates private endpoints (forwarding rules with internal IPs) that map to specific Google services or published third-party services. Provides more granular control than Private Google Access, including DNS integration and consumer-controlled IP addressing.
- **VPC Service Controls** — An orthogonal concept that creates security perimeters around Google services to prevent data exfiltration, independent of network connectivity method.

The progression from Private Google Access to Private Service Connect represents increasing control and isolation, with corresponding increases in configuration complexity.

## Network Service Tiers

GCP offers two network tiers that affect how traffic routes between end users and GCP resources:

- **Premium Tier** — Traffic enters/exits Google's network at the edge PoP closest to the user, traversing Google's private backbone for most of the path. Lower latency, higher cost, required for global load balancing.
- **Standard Tier** — Traffic routes over the public internet to the GCP region, entering Google's network at the regional entry point. Higher latency, lower cost, only supports regional load balancing.

The choice affects not just cost but architecture: services requiring global anycast IPs or global load balancers must use Premium Tier. Standard Tier suits regional workloads where latency differences are tolerable and cost optimization is prioritized.

## VPC Peering and Shared VPC

Multi-project networking in GCP uses two primary approaches:

### VPC Peering

Connects two VPC networks so that VMs in each can communicate via internal IPs. Characteristics:

- Non-transitive — If VPC-A peers with VPC-B and VPC-B peers with VPC-C, VPC-A cannot reach VPC-C through VPC-B
- Decentralized — Each project manages its own VPC and firewall rules
- Route exchange — Subnet routes are automatically exchanged; custom routes can optionally be exported/imported
- Limits — Each VPC has a maximum number of peering connections

### Shared VPC

Designates a **host project** that owns the VPC, with **service projects** that deploy resources into the host project's subnets. Characteristics:

- Centralized network administration — Network and security teams manage one VPC
- Decentralized resource deployment — Application teams deploy into designated subnets
- IAM-controlled subnet access — Fine-grained control over which service projects can use which subnets
- Simplified connectivity — All service projects share routing, firewall rules, and VPN/Interconnect connections

| Consideration        | VPC Peering                       | Shared VPC                                 |
| -------------------- | --------------------------------- | ------------------------------------------ |
| Administrative model | Decentralized                     | Centralized network, decentralized compute |
| Transitivity         | Non-transitive                    | All projects share network                 |
| Firewall management  | Per-VPC                           | Centralized in host project                |
| Scaling complexity   | Grows with peering count          | Grows with subnet/project count            |
| Organizational fit   | Independent teams, loose coupling | Central IT, strong governance              |

## Software-Defined Networking Differences

GCP's networking is fully software-defined, implemented on Google's custom infrastructure (Andromeda virtual network stack). Several conceptual differences from traditional networking emerge:

- **No physical topology mapping** — Subnets, routes, and firewalls exist as software constructs. There are no virtual switches or routers to configure. Routes are programming instructions for the distributed data plane.
- **Global routing domain** — A single VPC's routing table is globally consistent. Traditional networks require protocol convergence (OSPF, BGP) across router boundaries.
- **Firewall at the VM, not at the perimeter** — Distributed enforcement means every VM independently enforces rules. There's no concept of packets "inside the firewall" once they pass a perimeter device.
- **Identity-aware targeting** — Firewall rules can target service account identity rather than IP addresses, decoupling security policy from network topology.
- **Elastic bandwidth** — No fixed bandwidth allocation per network link. Bandwidth scales with VM size choices, not network provisioning.

These differences mean that mental models from physical networking or even other cloud providers can be misleading. Concepts like "subnet as a broadcast domain" or "VLAN segmentation" don't map directly. The abstraction is higher: operators define intent (who can talk to whom) and the platform handles implementation.

## Network Monitoring and Observability

Visibility into network behavior involves several complementary signals:

- **VPC Flow Logs** — Sampled records of network flows at each VM interface. Configurable sampling rate trades off between completeness and log volume/cost.
- **Firewall Rules Logging** — Records which firewall rules permitted or denied specific connections. Essential for auditing and debugging access issues.
- **Packet Mirroring** — Clones packets from specific instances to a collector for deep inspection. Useful for intrusion detection or protocol analysis but generates significant data volume.
- **Network Intelligence Center** — Aggregates topology, connectivity tests, and performance data for visualization and diagnosis.
- **Cloud NAT logging** — Records NAT translation events for instances using Cloud NAT for outbound internet access.

The tension in network observability is between comprehensive visibility (log everything) and cost/performance (logs themselves consume storage, generate egress charges, and require processing infrastructure). Sampling rates, log retention policies, and selective enabling per subnet are tuning mechanisms.

## Network Design Considerations

Several cross-cutting concerns shape network architecture decisions:

- **IP address planning** — Accounting for current needs, growth, hybrid connectivity overlap avoidance, and secondary ranges for containerized workloads
- **Segmentation strategy** — Choosing between multiple VPCs (stronger isolation, more management overhead) vs. fewer VPCs with firewall-based segmentation (simpler, larger blast radius)
- **DNS architecture** — Mapping service discovery needs to private zones, peering, and forwarding configurations
- **Egress cost management** — Inter-region, internet, and cross-project traffic all carry different cost profiles. Architecture choices (regional vs global, CDN placement, Interconnect vs VPN) have direct cost implications
- **Compliance boundaries** — Data residency requirements may constrain which regions can host subnets and where traffic can route
- **Disaster recovery** — Network-level redundancy through multi-region deployment, HA VPN failover, or Interconnect redundancy groups

Each of these concerns interacts with the others. IP planning constrains segmentation options. Compliance constrains region choices, which affects egress patterns and cost. Effective network design iterates across these dimensions rather than optimizing each in isolation.
