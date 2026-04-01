# AWS Networking

## VPC (Virtual Private Cloud)

### Core Components

A VPC is an isolated virtual network. Key building blocks:

| Component              | Purpose                                              |
| ---------------------- | ---------------------------------------------------- |
| VPC                    | Isolated network with CIDR block (e.g., 10.0.0.0/16) |
| Subnet                 | Segment of VPC CIDR in a single AZ                   |
| Route Table            | Rules determining where traffic is directed          |
| Internet Gateway (IGW) | Enables internet access for VPC                      |
| NAT Gateway            | Outbound-only internet for private subnets           |
| Elastic IP             | Static public IPv4 address                           |

### Subnets

- **Public subnet**: Route table has `0.0.0.0/0 → IGW`. Instances need public/elastic IP for internet access.
- **Private subnet**: Route table has `0.0.0.0/0 → NAT Gateway` (for outbound internet) or no internet route.
- Each subnet exists in one AZ. Spread across AZs for HA.
- AWS reserves 5 IPs per subnet: network, VPC router, DNS, future use, broadcast.
- /24 subnet = 256 - 5 = 251 usable IPs.

Typical 3-tier layout:

```
10.0.0.0/16 (VPC)
├── 10.0.1.0/24  Public  (us-east-1a) - ALB, NAT GW
├── 10.0.2.0/24  Public  (us-east-1b) - ALB, NAT GW
├── 10.0.11.0/24 Private (us-east-1a) - App servers
├── 10.0.12.0/24 Private (us-east-1b) - App servers
├── 10.0.21.0/24 Private (us-east-1a) - Databases
└── 10.0.22.0/24 Private (us-east-1b) - Databases
```

### Security Groups vs NACLs

| Feature    | Security Groups                        | NACLs                                             |
| ---------- | -------------------------------------- | ------------------------------------------------- |
| Level      | Instance (ENI)                         | Subnet                                            |
| State      | Stateful (return traffic auto-allowed) | Stateless (must allow both directions)            |
| Rules      | Allow only                             | Allow and Deny                                    |
| Evaluation | All rules evaluated                    | Rules evaluated in number order, first match wins |
| Default    | Deny all inbound, allow all outbound   | Allow all inbound and outbound                    |
| Reference  | Can reference other SGs by ID          | CIDR blocks only                                  |

Best practice: Use security groups as primary firewall (stateful, simpler). Use NACLs for subnet-level deny rules (e.g., block known bad IPs).

SG referencing pattern:

```
ALB SG: Inbound 443 from 0.0.0.0/0
App SG: Inbound 8080 from ALB-SG  (reference by SG ID)
DB SG:  Inbound 5432 from App-SG
```

### VPC Peering

Direct private network connection between two VPCs:

- No transitive routing (A↔B and B↔C does NOT mean A↔C)
- Works cross-account and cross-region
- CIDR blocks must not overlap
- Update route tables in both VPCs
- Security groups can reference peered VPC SGs (same region only)

### Transit Gateway

Hub-and-spoke network connectivity:

- Connect thousands of VPCs, VPNs, Direct Connect
- Transitive routing (unlike peering)
- Route tables for segmentation (production vs dev)
- Cross-region peering between transit gateways
- Multicast support
- Costs: hourly per attachment + data processing per GB

When to use peering vs transit gateway:

- 2-3 VPCs: Peering (simpler, cheaper, no data processing charge)
- 4+ VPCs or complex topology: Transit Gateway

### VPC Endpoints

Private connectivity to AWS services without traversing internet:

**Gateway Endpoints** (free):

- S3 and DynamoDB only
- Route table entry pointing to endpoint
- Policy-controlled access

**Interface Endpoints** (powered by PrivateLink):

- Most AWS services (CloudWatch, KMS, SQS, etc.)
- Creates ENI in your subnet with private IP
- DNS resolution via private hosted zone
- Per-hour + per-GB charge
- Can be accessed from on-premises via VPN/Direct Connect

**Gateway Load Balancer Endpoints**:

- Route traffic through third-party virtual appliances (firewalls, IDS)
- Transparent to source/destination

### VPC Flow Logs

Capture IP traffic metadata for network interfaces:

```
2 123456789012 eni-1a2b3c4d 10.0.1.5 52.94.76.5 443 49152 6 20 4000 1620140661 1620140721 ACCEPT OK
```

Fields: version, account, eni, srcaddr, dstaddr, srcport, dstport, protocol, packets, bytes, start, end, action, log-status

Destinations: CloudWatch Logs, S3, Kinesis Data Firehose. S3 is cheapest for high-volume analysis.

Limitations: Doesn't capture DNS traffic to Route 53 resolver, DHCP, metadata (169.254.169.254), or NTP traffic.

### NAT Gateway

Managed NAT for private subnet outbound internet:

- 45 Gbps bandwidth per gateway (scales automatically)
- AZ-specific — deploy one per AZ for HA
- ~$0.045/hour + $0.045/GB processed
- No security groups (use NACLs)
- Cannot be used as bastion/jump host

Cost optimization: NAT Gateway data processing is expensive. Use VPC endpoints for AWS services (free for gateway endpoints). Use S3 gateway endpoint to avoid NAT charges for S3 traffic.

## Elastic Load Balancing

### ALB (Application Load Balancer)

Layer 7 (HTTP/HTTPS):

- Path-based routing (`/api/*` → API targets, `/static/*` → S3)
- Host-based routing (`api.example.com` → API, `www.example.com` → web)
- HTTP/2, WebSocket, gRPC support
- Fixed response actions (maintenance pages without backends)
- Redirect actions (HTTP→HTTPS)
- Lambda targets
- Weighted target groups (blue/green, canary)
- OIDC authentication (Cognito, any OIDC provider)
- Sticky sessions (cookie-based)
- Access logs to S3

### NLB (Network Load Balancer)

Layer 4 (TCP/UDP/TLS):

- Ultra-low latency (~100μs vs ALB's ~400μs)
- Millions of requests per second
- Static IP per AZ (or Elastic IP)
- Preserves source IP (ALB uses X-Forwarded-For)
- TCP passthrough (no TLS termination) or TLS termination
- PrivateLink support (expose service to other VPCs)
- UDP support (DNS, gaming, IoT)

### GLB (Gateway Load Balancer)

Layer 3 (IP packets):

- Routes traffic through virtual appliances (firewalls, IDS/IPS)
- GENEVE encapsulation
- Transparent to applications
- Flow stickiness to same appliance

### Choosing a Load Balancer

| Need                             | Choose |
| -------------------------------- | ------ |
| HTTP routing, path/host-based    | ALB    |
| Ultra-low latency, static IP     | NLB    |
| gRPC                             | ALB    |
| UDP                              | NLB    |
| PrivateLink (service provider)   | NLB    |
| Third-party appliance inspection | GLB    |
| WebSocket                        | ALB    |

## CloudFront

### Edge Functions

| Feature        | CloudFront Functions         | Lambda@Edge                    |
| -------------- | ---------------------------- | ------------------------------ |
| Runtime        | JavaScript only              | Node.js, Python                |
| Execution time | < 1 ms                       | 5-30 seconds                   |
| Memory         | 2 MB                         | 128-10240 MB                   |
| Triggers       | Viewer request/response only | All 4 (viewer/origin req/resp) |
| Network access | No                           | Yes                            |
| Body access    | No                           | Yes                            |
| Scale          | Millions RPS                 | Thousands RPS                  |
| Cost           | ~$0.10/million               | ~$0.60/million + compute       |

CloudFront Functions: URL rewrites, header manipulation, A/B testing, simple auth (JWT validation).
Lambda@Edge: Complex auth, origin selection, image resizing, SSR.

### Origin Access Control (OAC)

Restrict S3 access to CloudFront only (replaces OAI):

- Supports SSE-KMS encrypted objects (OAI doesn't)
- Supports S3 access points
- Sigv4 signing
- S3 bucket policy grants `s3:GetObject` to CloudFront service principal with condition on distribution

### Cache Policies and Origin Request Policies

Separation of what determines cache key vs what's forwarded to origin:

**Cache Policy**: Define cache key components (headers, cookies, query strings, min/max/default TTL).
**Origin Request Policy**: Additional headers/cookies/query strings to forward to origin (without affecting cache key).

Example: Cache on `Accept-Language` header, forward `Authorization` to origin but don't include in cache key.

## Route 53

### Routing Policies

| Policy       | Use Case                                       |
| ------------ | ---------------------------------------------- |
| Simple       | Single resource, no health checks              |
| Weighted     | Distribute traffic by percentage (canary, A/B) |
| Latency      | Route to lowest-latency region                 |
| Failover     | Active-passive with health checks              |
| Geolocation  | Route by user's country/continent              |
| Geoproximity | Route by geographic distance with bias         |
| Multivalue   | Return multiple healthy IPs (up to 8)          |
| IP-based     | Route by client IP ranges (CIDR)               |

### Health Checks

- HTTP/HTTPS/TCP health checks (10 or 30 second intervals)
- String matching (check response body for specific string)
- Calculated health checks (combine multiple checks with AND/OR)
- CloudWatch alarm-based health checks
- Minimum 3 healthy checkers must agree for healthy status

### Private Hosted Zones

DNS for VPC-internal resources. Requires `enableDnsHostnames` and `enableDnsSupport` on VPC. Can be associated with multiple VPCs, even cross-account.

## Direct Connect

Dedicated private network connection from on-premises to AWS:

- 1 Gbps or 10 Gbps dedicated connections
- 50 Mbps to 10 Gbps via Direct Connect partners (hosted connections)
- Consistent latency (not over internet)
- Virtual interfaces: Public (AWS public services), Private (VPC), Transit (Transit Gateway)
- LAG (Link Aggregation Group): Bundle multiple connections
- Encryption: MACsec (Layer 2) on 10+ Gbps, or VPN over Direct Connect for IPsec

Setup time: Weeks to months for dedicated connections. Use VPN as immediate backup/failover.

Direct Connect Gateway: Connect to VPCs in multiple regions through a single Direct Connect connection.

## Site-to-Site VPN

IPsec VPN over internet:

- Two tunnels per connection (HA across separate AZ endpoints)
- Up to 1.25 Gbps per tunnel
- Accelerated VPN: Uses AWS Global Accelerator for better performance
- CloudHub: Hub-and-spoke VPN for multiple sites
- Failover: VPN as backup for Direct Connect (BGP priority routing)

Cost comparison: VPN ~$0.05/hour + data transfer. Direct Connect has port-hour charge + data transfer but no processing overhead and consistent performance.
