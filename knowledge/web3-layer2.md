# Layer 2 Scaling — Rollups, State Channels, Sidechains & Data Availability

## Overview

Layer 2 (L2) refers to protocols that execute transactions off the main chain ("layer 1") while periodically settling or anchoring to L1 for security. Core benefit: throughput increase (100x-1000x typical) via batching; cost reduction (3-100x cheaper per transaction). Trade-off: security model weakens from L1 finality to probabilistic finality based on fraud/validity proofs.

**Rollup vs. sidechain distinction:** Rollups inherit L1 security; sidechains do not.

## Optimistic Rollups

### Design Pattern

Transactions executed off-chain by a sequencer. The sequencer batches and posts compressed transaction data to L1 as a single "batch commitment" (Merkle root or commitment hash).

**Security model:** Assume batches are valid ("optimistic"); any party can challenge via fraud proof during a challenge period (~7 days on Optimism/Arbitrum).

### Fraud Proof Mechanism

If challenger believes batch is incorrect:

1. Challenger submits fraud proof showing one transaction execution differs from sequencer claim
2. Optimistic rollup verifies proof on-chain; if valid, sequencer loses bond and becomes subject to slashing
3. Incorrect state reverted; correct receipt provided

**Simplified example:**

Sequencer claims: `state_root_0 + tx(1) → state_root_1`
Challenger proves: `state_root_0 + tx(1) → state_root_1'` (different)
L1 verifier runs: `state_root_0 + tx(1)` and confirms challenger is correct; sequencer penalized.

### Implementation Complexity

Full fraud proof requires replaying transaction bytecode (EVM interpreter) on-chain. This is expensive. Solutions:

**Interactive fraud proof:** Prover and challenger perform binary search to narrow down exact step of divergence, then only that step is verified on-chain. Reduces on-chain compute.

**Optimistic execution:** Instead of full verification, sequencer can be penalized by bond forfeiture without full proof replay. Faster but assumes bond is sufficient deterrent.

### Challenge Period (Dispute Window)

Typically 7 days before batch is considered final. Users cannot finalize withdrawals to L1 until challenge period expires.

**Trade-off:** Security window vs. exit latency. Shorter period enables faster L1 composability but reduces time for challengers to detect fraud.

### Examples

**Optimism:** ~7 day dispute window. ~5-10 second block time. ~~130x cheaper than L1 (varies with gas).

**Arbitrum:** ~1 week dispute window. Offers both fraud proof (Arbitrum One) and other validators modes. ~500x throughput increase claimed.

## Zero-Knowledge (ZK) Rollups

### Design Pattern

Transactions executed off-chain. Prover generates cryptographic proof of batch correctness (zero-knowledge proof).

**Security model:** Proof verified on-chain; if valid, batch is final. No challenge period; instant finality (after proof verification latency, typically 10-30 minutes).

### Proof Types

**SNARKs (Succinct Non-Interactive Arguments of Knowledge):**

- Proof size: ~200 bytes
- Verification cost: ~500k gas (cheaper on-chain)
- Prover setup: Requires trusted setup ceremony (one-time, but introduces centralization if ceremony is compromised)

Examples: Aztec, Polygon zkEVM (using Plonk).

**STARKs (Scalable Transparent Arguments of Knowledge):**

- Proof size: ~3-10 KB (larger than SNARKs)
- Verification cost: ~5-10M gas (more expensive on-chain)
- Setup: Transparent (no trusted ceremony)
- Assumptions: Weaker (collision-resistant hash > elliptic curve discrete log)

Examples: StarkNet (Cairo VM).

### Trade-offs

| Aspect | SNARK | STARK |
|--------|-------|-------|
| Proof size | Small (200B) | Large (3-10KB) | 
| Verification gas | Low (~500k) | High (~5-10M) |
| Prover complexity | Lower | High (requires FFT, high memory) |
| Setup ceremony | Required | Transparent |
| Post-quantum | No | Likely yes |

### Performance Characteristics

ZK rollups reduce settlement latency (no challenge period) but increase batch creation latency (proof generation time). Practical throughput:

- **Optimistic rollups:** 1000-4000 TPS (depends on sequencer capacity)
- **ZK rollups:** 500-2000 TPS (proof generation bottleneck)

## State Channels

### Concept

Two or more parties lock collateral in a smart contract on-chain. Off-chain, they exchange signed state updates (transactions) without touching the blockchain. Only two events hit L1: channel open and channel close with final state.

**Update process:** Party A sends state update to Party B; both sign; can transact without broadcasting to blockchain.

**Closure:** Either party can close channel; contract verifies signatures and enforces final state.

### Use Case

Efficient for payment channels (Lightning Network), micro-transactions between known parties.

**Limitations:**

- Requires off-chain communication infrastructure (participants must be reachable)
- Liquidity fragmented across channels (user locked $100 in channel A cannot use it in channel B without closing A)
- Requires collateral lock-up for entire channel lifetime

### Lightning Network (Bitcoin)

Payment channel layer on Bitcoin. Enables ~1M TPS routing payments with ~1 second settlement. Drawback: complex routing (multi-hop payments); capital required to run routing nodes.

## Plasma & Exit Games

### Design

Similar to rollups but with critical difference: transaction data stored off-chain (not on-chain). Users must archive data to defend themselves if operator misbehaves.

**Exit game:** User can prove they own assets by providing sparse Merkle proof of their position in a prior block. Requires users keep data; operator misbehavior forces users to run full nodes.

### Limitations

Scalability capped by "exit validity proof" cost on-chain. Plasma is fundamentally limited to ~150-200 TPS realistic throughput due to exit proof overhead.

**Status:** Largely deprecated in favor of rollups which post full data on-chain.

## Sidechains

### Definition

Separate blockchain with own validator set, consensus mechanism, and finality. "Side" to L1; not directly secured by L1.

**Bridge:** Users deposit L1 tokens, receive wrapped tokens on sidechain. Withdrawal reverses process.

**Security model:** Sidechain security depends on its own validator set. If validators collude, they can mint tokens fraudulently or stop the chain.

### Examples

- **Polygon PoS:** Initially 100+ validators (not fully decentralized). Proof-of-Stake. Users trust validator set.
- **Binance Smart Chain:** Validators selected by Binance. High throughput; centralized validator control.

### Risk

Bridge validators can steal funds; sidechain censorship or halt. Tradeoff: throughput for custody trust.

## Data Availability & Commitments

### The Data Availability Problem

Rollup posts state root on L1 but not transaction data. If data unavailable:

- Users cannot verify state root correctness
- Users cannot reconstruct their account state
- Withdrawal to L1 becomes unsafe (requires full chain history)

**Solution:**

- **Post data to L1:** Optimism, Arbitrum, Starknet post full calldata to L1. L1 gas cost is dominant (~80-85% of L2 fees). Data is permanently available.
- **Separate DA layer:** Celestia, EigenDA provide specialized data availability commitments. Cheaper (~50-80% of L2 fees) but requires trust in DA layer validators.

### Data Availability Layers

**Celestia:** Separate blockchain optimized for data storage. Users submit data; Celestia validators attest availability via Merkle proofs. Rollup posts reference to Celestia, not full data to L1. Risk: if Celestia validators collude, they can withhold data; rollup becomes unstoppable but users cannot exit.

**EigenDA (EigenLayer):** DA service using restaking. Ethereum validators opt-in to provide data holding. Risk: restaking introduces slashing conditions across different services; complex game theory.

## Bridges & Cross-Chain Communication

Rollup users eventually need to withdraw to L1 or interact across chains. Bridges enable this.

### Custodial Bridges

Lock collateral on source chain; authority mints wrapped token on destination chain.

**Trust assumption:** Bridge authority is honest. If bridge contract is exploited, funds in locked collateral are at risk.

### Light Client Bridges

Destination chain runs light client of source chain (verifying source chain headers). Transfers verified cryptographically. Examples: Rainbow Bridge (NEAR-Ethereum), IBC (Cosmos).

**Risk:** Light client trust assumptions (validators can collude).

### Message Passing

Bridge relays messages (not just tokens) enabling cross-chain smart contract calls. Example: Stargate Finance enables swaps across EVM chains via bridge messaging.

## Sequencers & Centralization

Most rollups use centralized sequencer to order transactions. Sequencer can:

- Reorder transactions (MEV extraction)
- Censor transactions
- Suffer downtime

**Mitigations:**

- **Decentralized sequencer:** Multiple sequencers proposed by Arbitrum, Optimism. Complex to implement; economic game theory still evolving.
- **Forced inclusion:** Users can force inclusion by paying fee to L1 contract; sequencer cannot censor forever.
- **Sequencer failover:** Backup sequencer takes over if primary fails.

## Scaling Comparison

| Factor | Optimistic | ZK | Channel | Sidechain |
|--------|-----------|----|---------|-----------| 
| Finality | 7 days | ~30 min | Instant | ~2 hours |
| Throughput (TPS) | 1000-4000 | 500-2000 | ~1M (local) | 1000-10000 |
| Sequencer centralization | High | High | Medium | Very high |
| Withdrawal cost | ~$50-200 | ~$50-300 (proof) | Low (~$10) | Medium |
| Complexity | Medium | Very high | Low-medium | Low |

## Future Directions

**Validity rollups (recursive STARKs):** Prove proof verification itself, enabling proofs of proofs. Reduces per-transaction cost asymptotically.

**Encrypted mempools/PBS:** Integrate with L1 MEV solutions to mitigate sequencer extraction.

**Sovereign rollups:** Consensus/data provided by external DA (Celestia, EigenDA); rollup can reorg without L1 permission. Higher autonomy, lower security guarantees.