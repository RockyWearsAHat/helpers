# Cloud Networking — Advanced Connectivity, Hybrid Integration & Global Load Balancing

## Overview

Cloud networking fundamentals (VPCs, subnets, routing) enable isolated virtual networks. **Advanced patterns** address multi-region connectivity (VPC peering, transit gateways), hybrid cloud (on-premises ↔ cloud), private connectivity (PrivateLink), and global application delivery (edge locations, global load balancing). Each pattern trades operational complexity, latency, cost, and failure domain isolation.

## VPC Peering

Connect two VPCs directly via private link; traffic never crosses internet.

```
VPC A (10.0.0.0/16) ←→ VPC B (10.1.0.0/16)
All traffic via private AWS/GCP backbone
```

### Setup

```yaml
# In VPC A, create peering connection to VPC B
resource "aws_vpc_peering_connection" "main" {
  vpc_id      = aws_vpc.a.id
  peer_vpc_id = aws_vpc.b.id
  auto_accept = false  # peer must accept
}

# In VPC B, accept peering
resource "aws_vpc_peering_connection_accepter" "peer" {
  vpc_peering_connection_id = aws_vpc_peering_connection.main.id
  auto_accept               = true
}

# Update route tables in both VPCs to route to peer VPC via peering connection
resource "aws_route" "a_to_b" {
  route_table_id            = aws_route_table.a.id
  destination_cidr_block    = aws_vpc.b.cidr_block  # 10.1.0.0/16
  vpc_peering_connection_id = aws_vpc_peering_connection.main.id
}
```

### Characteristics

**Latency:** Microseconds (same cloud region) to ~10ms (cross-region).

**Cost:** AWS charges per GB transferred cross-region ($0.02/GB); same-region peering free.

**Tradeoff vs. transit gateway (see below):**
- Peering: simple (1-to-1), scalable for small # of VPCs (< 10).
- Transit gateway: complex hub-and-spoke, scales to 100s of VPCs.

**Restrictions:**
- CIDR blocks cannot overlap (10.0.0.0/16 and 10.1.0.0/16 OK; 10.0.0.0/16 and 10.0.5.0/16 NOT OK, overlapping).
- Peering doesn't permit transitive routes. If VPC A peers to VPC B, and VPC B peers to VPC C, traffic from A cannot reach C via B. A and C must peer directly.

**Failure isolation:** Single peering link failure leaves VPCs disconnected. Multi-region peering requires backup peering in alternate region.

## Transit Gateway

Centralized hub for routing between VPCs, on-premises networks, and transit VPCs.

```
VPC A ─┐
VPC B ─┼─→ Transit Gateway ←─ On-Premises (VPN/Direct Connect)
VPC C ─┘

Transit Gateway acts as central router; all inter-VPC traffic flows through it.
```

### Setup

```yaml
resource "aws_ec2_transit_gateway" "main" {
  description                     = "main-tgw"
  default_route_table_association = "enable"
  default_route_table_propagation = "enable"
  amazon_side_asn                 = 64512
  tag_specifications {
    resource_type = "transit-gateway"
    tags = {
      Name = "prod-tgw"
    }
  }
}

resource "aws_ec2_transit_gateway_attachment" "vpc_a" {
  transit_gateway_id = aws_ec2_transit_gateway.main.id
  vpc_id             = aws_vpc.a.id
  subnet_ids         = aws_subnet.a.*.id  # attach subnets, not VPC
}

# On-premises VPN attachment
resource "aws_ec2_transit_gateway_attachment" "vpn" {
  transit_gateway_id = aws_ec2_transit_gateway.main.id
  vpn_connection_id  = aws_vpn_connection.on_prem.id
}
```

**Characteristics:**

**Hub-and-spoke model:** All traffic between VPCs and on-premises routed through TGW (single point of policy enforcement).

**Route propagation:** TGW automatically learns routes from attached VPCs / VPNs. VPC A's routes propagated to TGW, then available to VPC B (via TGW routing table).

**Scalability:** Supports 5000+ VPCs (vs. peering limit ~10-20 before route table explosion).

**Cost:** $0.05/attachment/hour + $0.02/GB (cross-AZ/region). More expensive than peering for small # VPCs, cheaper per connection for large hub.

**Failure:** TGW downtime leaves all attached networks disconnected. Mitigation: multi-region TGW (failover to alternate region).

## PrivateLink / Private Service Connect

Expose service from producer VPC to consumer VPCs without internet exposure.

```
Producer VPC (service)
  → NLB (Network Load Balancer) on private IPs
  → VPC Endpoint Service
  
Consumer VPCs
  → VPC Endpoint (request access to endpoint service)
  → Private DNS resolves service-name.vpce-region.amazonaws.com to consumer's private IP
  → Traffic: consumer → service via AWS backbone, never internet
```

### Setup (AWS)

```yaml
# Producer side: expose service
resource "aws_lb" "service" {
  name_prefix = "svc-"
  load_balancer_type = "network"
  subnets = aws_subnet.producer.*.id
  internal = true
}

resource "aws_vpc_endpoint_service" "main" {
  network_load_balancer_arns = [aws_lb.service.arn]
  acceptance_required = true  # whitelist consumers
}

# Consumer side: connect to service
resource "aws_vpc_endpoint" "service" {
  vpc_id = aws_vpc.consumer.id
  service_name = aws_vpc_endpoint_service.main.service_name
  vpc_endpoint_type = "Interface"
  subnets = aws_subnet.consumer.*.id
  security_groups = [aws_security_group.consumer_to_service.id]
}

resource "aws_route53_record" "service_dns" {
  zone_id = aws_route53_zone.internal.id
  name    = "service.example.com"
  type    = "CNAME"
  ttl     = 60
  records = [aws_vpc_endpoint.service.dns_entries[0].dns_name]
}
```

### Characteristics

**Isolation:** Service not exposed to internet; only whitelisted consumers can access. Prod service safe from internet threats.

**Latency:** ~1ms (same availability zone via AWS backbone).

**Cost:** $0.01/endpoint + $0.02/GB transferred (same region). Cheaper than NAT Gateway for high volume.

**Multi-region:** Each region requires separate endpoint service + endpoint. Cross-region connectivity requires peering/TGW.

**Use case:** Shared services (logging, monitoring, key management) accessed by multiple teams/accounts without exposing to internet.

**DNS resolution:** Producer can enable private DNS; `serviceexample.com` resolves to PrivateLink IP automatically in consumer VPCs (simpler UX than manual CNAME).

## Hybrid Connectivity: VPN vs. Direct Connect vs. Interconnect

### VPN (Virtual Private Network)

Encrypt traffic between on-premises and cloud over internet.

```
On-Premises Router (OpenVPN, StrongSwan) 
  ←[IPsec tunnel, encrypted]→ 
Cloud VPN Gateway
  → Routes to VPC subnets
```

```yaml
resource "aws_vpn_connection" "main" {
  type = "ipsec.1"
  customer_gateway_id = aws_customer_gateway.on_prem.id
  vpn_gateway_id = aws_vpn_gateway.main.id
  static_routes_only = true
  static_routes = [
    {
      destination_cidr_block = "10.100.0.0/16"  # on-prem LAN
    }
  ]
}
```

**Characteristics:**

**Latency:** 40-100ms typical (internet dependent); variable.

**Bandwidthth:** 1.25 Gbps (AWS VPN limit). Bottleneck if large data transfer needed.

**Cost:** $0.05/connection-hour + $0.09/GB (data transfer).

**Tradeoff:** Easy to set up (no carrier involvement), but performance dependent on ISP internet quality.

**Use case:** Low-bandwidth hybrid connectivity (management traffic, backup), acceptable latency (non-interactive workloads).

### AWS Direct Connect / GCP Interconnect / Azure ExpressRoute

Dedicated physical connection between on-premises and cloud. Leased from network provider (Equinix, Digital Realty, AT&T, etc.).

```
On-Premises Data Center
  ← [physical fiber, dedicated 1/10/100 Gbps]
  ← Equinix/carrier meets AWS region
  → Network Border Group (AWS availability zone)
  → VPC
```

**Setup complexity:** 4-12 weeks (order fiber, ship hardware, coordinate carrier + cloud provider).

```yaml
# Request Direct Connect connection
resource "aws_dx_connection" "main" {
  name              = "connectivity-to-aws"
  location          = "Equinix SV1"  # co-location facility
  bandwidth         = "10Gbps"
  connection_state  = "requested"
}

# Virtual interface (VLAN + BGP) to VPC
resource "aws_dx_virtual_interface" "vpc" {
  connection_id           = aws_dx_connection.main.id
  name                    = "vif-to-vpc"
  vlan                    = 100  # VLAN ID allocated by carrier
  asn                     = 65000
  auth_key                = var.bgp_auth_key
  customer_address        = "10.0.0.1/30"
  amazon_address          = "10.0.0.2/30"
  address_family          = "ipv4"
  virtual_interface_type  = "private"  # private (VPC) or public (S3, etc.)
}
```

**Characteristics:**

**Latency:** ~10ms (consistent, backbone).

**Bandwidth:** 1, 10, 100 Gbps (no throttle like VPN).

**Cost:** $0.30/Gbps-month + $0.30/Gbps-hour (on-demand). 10 Gbps = $3000/month + $2200/month usage (if always on). High upfront commitment.

**Reliability:** 99.9% SLA; redundant connections recommended (2 connections to different regions = $6K+/month cost for redundancy).

**BGP routing:** On-premises router announces routes via BGP to cloud gateway; cloud gateway announces routes back. Dynamic failover if one connection fails.

**Tradeoff:** High cost justifies only for large data volume (100+ TB/month) or performance-critical hybrid workloads.

**Use case:** Large hybrid deployments (SAP, Oracle ERP in data center, cloud analytics), high-bandwidth replication, consistent low-latency access.

## DNS Resolution: Private vs. Hybrid

### Route 53 Hosted Zone (AWS)

DNS for domains managed by AWS.

```yaml
resource "aws_route53_zone" "internal" {
  name = "internal.example.com"
  vpc {
    vpc_id = aws_vpc.main.id
  }
  vpc {
    vpc_id = aws_vpc.dr.id  # accessible in multiple VPCs
  }
}

resource "aws_route53_record" "db" {
  zone_id = aws_route53_zone.internal.id
  name    = "db.internal.example.com"
  type    = "A"
  ttl     = 60
  records = [aws_db_instance.main.address]
}
```

**Private hosted zone:** Records accessible only within associated VPCs. `db.internal.example.com` resolves to RDS endpoint inside VPC, not visible to internet.

**Hybrid DNS:** On-premises DNS server can forward queries to Route 53 (via VPN/Direct Connect), allowing seamless resolution across sites.

```
On-Prem DNS (bind) forwards *.internal.example.com queries to Route 53
Route 53 authoritative for internal.example.com
Result: both on-prem and cloud can resolve hybrid resources
```

### Google Cloud DNS / Azure DNS

Analogous to Route 53. GCP Cloud DNS supports private DNS zones (VPC-scoped); Azure Private DNS integrates with Virtual Networks.

**Cross-provider hybrid DNS:** More complex; typically use Route 53 or ISC BIND on-premises with forwarders.

## Global Load Balancing

Route traffic globally across regions; lower latency via geographic proximity, active-active failover.

### AWS Global Accelerator

Anycast IP on AWS global network; routes traffic to nearest regional endpoint.

```
User in Tokyo
  → Global Accelerator anycast IP (199.7.X.X, AWS managed)
  → Routes via AWS edge location to nearest regional endpoint (Tokyo region)
  → Application (ALB/NLB in Tokyo)

User in New York
  → Same anycast IP
  → Routes to NYC region endpoint
  → Application (ALB/NLB in us-east-1)
```

```yaml
resource "aws_globalaccelerator_accelerator" "main" {
  name = "my-app-ga"
  ip_address_type = "IPV4"
  enabled = true
}

resource "aws_globalaccelerator_listener" "main" {
  accelerator_arn = aws_globalaccelerator_accelerator.main.arn
  port_ranges = [
    {
      from_port = 80
      to_port   = 80
    }
  ]
  protocol = "TCP"
}

resource "aws_globalaccelerator_endpoint_group" "tokyo" {
  listener_arn          = aws_globalaccelerator_listener.main.arn
  endpoint_group_region = "ap-northeast-1"  # Tokyo
  endpoint_configurations = [
    {
      endpoint_id = aws_lb.tokyo.arn
      weight      = 100
    }
  ]
}
```

**Characteristics:**

**Latency:** Reduced via AWS backbone and edge routing (vs. DNS-based geolocation) ~10-50ms improvement.

**Cost:** $0.025/accelerator-hour + $0.006/GB (data processed).

**Traffic steering:** Support weighted endpoints (primary region 80%, secondary 20% for canary).

**Health checks:** If Tokyo endpoint unhealthy, traffic reroutes to next closest region.

**TCP/UDP:** Works for any protocol; DNS-independent (anycast ensures routing).

### Cloud CDN + Global HTTP(S) Load Balancer

Cache content at edge sites; route requests to nearest cache.

```yaml
resource "google_compute_backend_bucket" "static" {
  name            = "static-assets"
  bucket_name     = google_storage_bucket.static.name
  cdn_policy {
    client_ttl = 3600
    default_ttl = 3600
    max_ttl = 86400
    negative_caching = true
    serve_while_stale = 604800  # serve stale 7 days while origin down
  }
}

resource "google_compute_url_map" "lb" {
  name            = "my-app-lb"
  default_service = google_compute_backend_service.app.self_link
  path_rule {
    paths   = ["/assets/*"]
    service = google_compute_backend_bucket.static.self_link
  }
}

resource "google_compute_target_https_proxy" "proxy" {
  name             = "my-app-proxy"
  url_map          = google_compute_url_map.lb.self_link
  ssl_certificates = [google_compute_ssl_certificate.cert.self_link]
}

resource "google_compute_global_forwarding_rule" "lb" {
  name       = "my-app-lb-rule"
  ip_version = "IPV4"
  load_balancing_scheme = "EXTERNAL"
  port_range = "443"
  target     = google_compute_target_https_proxy.proxy.self_link
}
```

**Characteristics:**

**Cache hit ratio:** Depends on content (static images/CSS 90%+; HTML 30-50% if personalized).

**Cost:** Free cache, but per-request pricing if cache miss. Cheaper than origin bandwidth for high-volume CDN.

**Geo-steering:** Route based on location (EU requests → EU cache, US requests → US cache) for GDPR compliance.

**Failover:** If origin region down, serve stale content from edge (user sees slight stale data vs. complete outage).

## Network Security: Security Groups & NACLs

### Security Groups (Stateful Firewall)

Per-instance rules; stateful (return traffic automatically allowed).

```yaml
resource "aws_security_group" "web" {
  name = "web-sg"
  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]  # HTTPS from anywhere
  }
  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]  # HTTP from anywhere
  }
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"  # all protocols
    cidr_blocks = ["0.0.0.0/0"]  # allow all outbound
  }
}
```

**Characteristics:**

**Stateful:** Return traffic (responses) automatically permitted. Inbound HTTPS rule implicitly allows outbound HTTPS response.

**Deny-by-default:** All traffic denied except explicitly allowed (deny rules unnecessary, default drop).

**Granularity:** Attachable to instances, ENIs, RDS databases.

### NACLs (Stateless Firewall)

Subnet-level rules; stateless (must explicitly allow return traffic).

```yaml
resource "aws_network_acl" "public" {
  subnet_ids = aws_subnet.public.*.id
  
  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_block  = "0.0.0.0/0"
    rule_no     = 100
    action      = "allow"
  }
  
  egress {
    from_port   = 0
    to_port     = 65535
    protocol    = "tcp"
    cidr_block  = "192.168.0.0/16"
    rule_no     = 100
    action      = "allow"
  }
}
```

**Stateless:** Outbound rule required for return traffic. NACL doesn't track connections; must allow ephemeral port range (1024-65535) for responses.

**Evaluation:** Rule numbers processed in order; first match wins (deny rules can block before allow).

**Use case:** Coarse-grained subnet-level policy (block all traffic to suspicious subnet); security groups for instance-level granularity.

**Tradeoff:** NACLs less common than security groups; rules quickly complex if fine-grained (many allow rules for ephemeral ports). Security groups sufficient for most cases.

## See Also

- **cloud-gcp-networking:** GCP-specific networking (Cloud VPC, Cloud Interconnect).
- **cloud-aws-networking:** AWS-specific networking (VPC design, security).
- **cloud-azure-networking:** Azure-specific networking (VNets, ExpressRoute).
- **infrastructure-dns-architecture:** DNS design patterns.
- **security-network:** Network security principles, DDoS mitigation.