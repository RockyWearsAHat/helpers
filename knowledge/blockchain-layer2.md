# Layer 2 Scaling — Rollups, State Channels, Data Availability & Bridge Security

## Overview

Layer 2 (L2) protocols execute transactions off-chain ("layer 1" = Ethereum mainnet) while securing final settlement on-chain. Trade-off: transactions cheaper and faster (100-1000x throughput), but add latency and complexity to finality.

Core principle: **Compress data, verify on-chain.** Instead of executing all 1 million transactions per block, L2 batches them, compresses the data, and posts a compact proof or commitment to Ethereum. Ethereum validates the proof (or fraud) and settles final state.

This architecture enables blockchains to scale without fundamental changes to layer 1 consensus.

## Optimistic Rollups

Assume all transactions are valid ("optimistic"). Sequencer batches transactions, compresses them, posts batch root to L1. If no one challenges within a **challenge window** (typically 7 days), transactions finalize. If someone claims batch is invalid, **fraud proof** reveals the error; operator loses bond, challenger rewarded.

### Arbitrum / Optimism Architecture

**Key phases:**

1. **Transaction Submission:** User sends tx to sequencer (centralized or decentralized network). Sequencer holds and orders transactions.
2. **Batch Posting:** Sequencer periodically posts batch data + root hash to L1 smart contract.  
3. **Challenge Period:** 7-day window where anyone can challenge the batch.  
4. **Fraud Proof (if challenged):** Challenger and operator enter interactive verification game:
   - Challenger claims instruction $i$ in batch was invalid
   - Operator mist prove instruction $i$ valid (execute 1 step of state machine; cost ~$200K gas)
   - Iterative narrowing: each round cuts dispute space in half, until single instruction remains
   - Loser pays gas + slashing
5. **Finality:** After challenge window, batch canonical. Transactions permanently settled.

**Time to Finality:**
- User's tx confirmed: 2-20 seconds (sequencer inclusion)
- Full finality: 7 days

### Batch Data and Compression

Batch posted as calldata (cheaper than storage, ~16 gas/byte):
- 1MB batch ≈ 16M gas = ~$30-100 depending on gas price
- Cost distributed across 500-5000 txs ≈ $0.01-0.1/tx
- Compression: remove redundancy (e.g., sender address repeated, encode as diff)

### Fraud Proof Game (Interactive Verification)

Prevents operator lies by incentivizing challenges:

```
Dispute: Batch root disagree
Phase 1: Binary search narrows bad instruction down to single step
  Challenger: "Step 100 invalid"
  Operator: "No, here's machine state before step 100 and after"
  (Challenger sees proof; if consistent, concedes or escalates to step 50)

Phase 2: Final step disputes resolved on-chain
  Operator executes disputed step within EVM
  If Operator cannot reproduce claimed output → loses bond
```

Gas cost of final step: ~200K (one EVM instruction worth of computation).

### Operator Incentives and Capital

Sequencer must post collateral (bond). If caught lying, bond slashed. This asymmetry — cheap proof (one fraud proof costs $30K gas), expensive false claim (lose $1M+ bond) — incentivizes honesty.

## ZK Rollups (Zero-Knowledge Proofs)

Instead of fraud proofs, sequencer produces **validity proof** proving correct state transition. No challenge period needed. After proof verified on-chain, transactions immediately finalize.

### SNARK vs STARK

**SNARK (Succinct Non-Interactive Argument of Knowledge):**
- Proof size: ~100 bytes
- Verification cost on-chain: ~500K gas
- Generation requires trusted setup (quantum-resistant versions possible but more complex)
- Example: Starkware's StarkNet version 1

**STARK (Scalable Transparent Argument of Knowledge):**
- Proof size: ~100KB (larger)
- Verification cost: ~5M gas (more expensive on-chain)
- No trusted setup required
- Hash-based (quantum-resistant)
- Example: Starkware's StarkNet version 2 (Starknet)

### Finality Model

Sequencer posts batch + zero-knowledge proof. L1 smart contract verifies proof (expensive, 500K-5M gas). If proof valid → transactions **immediately final** (no challenge period).

**Speed advantage:** Deterministic inclusion + immediate finality = 2-20 min total (vs. 7 days for optimistic).  
**Cost tradeoff:** Proof generation expensive (Starkware publishes 30-40M gas worth of computation per batch to prove it; spread across txs). Roughly on-parity or cheaper than optimistic rollups, depending on batch size.

### Practical Implementations

- **zkSync:** Fork-like system, uses SNARKs, fast finality
- **Starknet:** Cairo language VMs, complex but powerful, uses STARKs
- **Scroll:** EVM-equivalent using SNARKs (enables deploying existing contracts unchanged)
- **Polygon zkEVM:** EVM monomorphic ZK system

Trade-off: EVM-equivalent ZK systems are slower and more expensive to prove than application-specific systems. Scroll generates proof time ~30-60 min for batch (post-block latency); zkSync faster.

## State Channels

Off-chain transactions between two parties. No blockchain involvement until settlement.

### Mechanism

1. Participants lock collateral on-chain
2. Exchange signed state updates off-chain (payments, atomic swaps)
3. Either party can settle on-chain by posting latest state + signatures
4. If dispute, on-chain contract enforces rules (latest timestamp state wins)

### Characteristics

- **Instant finality:** Transactions finalize when both parties sign (no block confirmation)
- **Privacy:** Channel activity hidden until settlement
- **Limited scalability:** Pairwise channels (A ↔ B need direct channel; cannot pay A → C via B unless C is hub)
- **Capital efficiency:** Collateral locked during channel lifetime; only total net owed settled on-chain

### Practical Implementations

- **Lightning Network (Bitcoin):** Hub-and-spoke topology, routed payments via HTLCs (hash time-lock contracts)
- **Raiden Network (Ethereum):** Similar to Lightning; less deployed (complexity, capital efficiency limits)

Channels dominate payments (Lightning for BTC), but rarely used for general computation due to capital efficiency and routing complexity.

## Data Availability and EIP-4844 (Blobs)

Problem: L2 batches must post data to L1 so anyone can verify. If data unavailable, no one can generate fraud proofs or verify state. **Data Availability Problem.**

### Proto-Danksharding (EIP-4844)

Introduces **blob space** separate from calldata:
- Blobs: ~125KB per block (4 blobs standard, up to 6 available)
- Blob cost: ~1 wei per byte (vs. 4-16 wei for calldata)
- Retention: Blobs auto-prune after 18 days (consensus rule)

**Impact:** Rollup batch costs drop 10-100x. January 2024 base fees dropped 60-90% overnight.

### Full Danksharding (Future)

Data availability sampling: validators don't download entire block. Instead, cryptographic proofs ensure data available without full download. Targets ~100MB per block DA, enabling true scalability (100K+ tx/sec).

## Bridge Risks and Security Models

Transactions moving from L1 to L2 or vice versa require bridges. Each bridge architecture trades off security and cost.

### Optimistic / Fraud Proof Bridges

L2 transaction requesting withdrawal. Time lock (7 days), challenge period, fraud proof. If no challenge, withdrawal approved. High security (Ethereum security assumption), high latency.

### Fast Bridges (Liquidity Networks)

L2 user posts withdrawal request on L2. Bridge operator on L1 provides liquidity immediately (pays user on L1), then waits for L2 settlement to recoup. User gets L1 funds in 5 min vs. 7+ days. Operator takes fee (0.5-2%).

Risk: Bridge operator solvency. If operator insolvent during market crash, users' withdrawals stuck.

### Validator Bridges (Proof of Authority)

Multisig committee or PoA validator set signs withdrawal transactions. Requires trust in committee. Used by centralized or early-stage L2s (Polygon PoS side chain until 2023, now transitioning to proof).

### Smart Contract Bridges (Token Wrapping)

Token locked on source chain, wrapped token minted on destination chain. Two-way: burn wrapped token on destination, unlock original on source. Simplest, suitable for ERC-20 transfers. Smart contract risk: bridge contract can be compromised.

## Sequencer Architecture

**Centralized:** Single operator (Arbitrum One, Optimism mainnet phase 1). Operator censors transactions by not including them. Mitigated by timeout rule: if sequencer inactive >24 hours, users can self-submit transactions to L1 and force inclusion.

**Decentralized (PoS sequencing):** Sequencer role rotated among stakers. Problem: ordering coordination and MEV still exist (reordering transactions, sandwich attacks). Most L2s not yet decentralized.

## Practical Throughput and Costs

| L2           | Finality  | TPS  | Cost/tx | Data Posting Method |
|-------------|-----------|------|---------|---------------------|
| Optimism    | 7d        | ~4k  | $0.1-1  | Calldata (legacy), Blobs (Ecotone) |
| Arbitrum    | 7d        | ~40k | $0.05-0.5 | Calldata, Blobs |
| zkSync Era  | 20min     | ~4k  | $0.05-0.5 | Calldata+SNARK proof |
| Starknet    | 30min     | ~1k  | $0.10-1 | Calldata+STARK proof |
| Base        | 7d        | ~10k | $0.05-1 | Blobs |

Throughput constrained by:
1. Sequencer ordering speed
2. L1 batch posting + proof verification latency
3. DA bottleneck (before blobs, severe; after blobs, less critical)

## Trade-offs Summary

| Approach       | Finality  | Latency | Security | Cost  | Capital Locking |
|---------------|-----------|---------|----------|-------|-----------------|
| Optimistic    | 7 days    | Slow    | High     | Low   | N/A             |
| ZK            | Minutes   | Medium  | High     | Mid   | N/A             |
| State Channel | Instant   | None    | Medium   | Very Low | High        |
| Sidechain     | N/A       | Instant | Lower    | Very Low | N/A        |

Choice depends on use case: payments → Lightning (finality matters, capital efficient); DeFi → Arbitrum/Optimism (security, composability matters); games → zkSync (fast finality, acceptable capital cost).

## See Also

- [blockchain-distributed-ledger.md](blockchain-distributed-ledger.md) — consensus and blockchain architecture
- [blockchain-evm.md](blockchain-evm.md) — execution semantics and gas on rollups
- [privacy-zero-knowledge.md](privacy-zero-knowledge.md) — zero-knowledge proof systems
- [formal-verification.md](formal-verification.md) — proving rollup correctness