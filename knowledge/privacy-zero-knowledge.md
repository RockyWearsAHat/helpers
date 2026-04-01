# Zero-Knowledge Proofs — Interactive Proofs, SNARKs, STARKs, and Applications

## Overview

Zero-knowledge proof (ZKP) allows one party (prover) to convince another party (verifier) that a statement is true without revealing any information other than the statement's truth. Example: prove ownership of a Bitcoin private key without disclosing the key itself. Applications: blockchain, identity verification, voting, secure computation, privacy-preserving authentication.

---

## Core Properties

### Completeness

If the statement is true and both prover and verifier follow the protocol, the verifier *always* accepts. Protocol has no false negatives: honest prover can always convince honest verifier.

### Soundness

If the statement is false, no prover (even a cheating one) can convince the verifier, except with negligible probability (typically ≤ 2^-128 for cryptographic soundness). Protocol has no false positives: dishonest prover cannot fool verifier.

### Zero-Knowledge

Verifier learns only that the statement is true; nothing else is revealed. Formally: for any verifier algorithm V, there exists a simulator S that produces transcripts indistinguishable from real interactions without knowing the prover's secret.

**Intuition:** The verifier cannot extract the prover's witness (secret information) from the proof. If verifier could, the proof would reveal information.

---

## Interactive Proofs

Original ZKP model (introduced by Goldwasser, Micali, Rackoff, 1985): prover and verifier exchange messages in rounds.

### Fiat-Shamir Protocol (Identification)

Prover convinces verifier of knowledge of secret x where y = g^x mod p.

**Protocol:**
1. Prover: Generate random r, compute u = g^r mod p, send u
2. Verifier: Send random challenge c ∈ {0,1}
3. Prover: Compute z = r + c×x mod p, send z
4. Verifier: Check g^z = u × y^c mod p

**Verification:** If holds, prover knew x; otherwise prover would need to predict the challenge c (impossible—sent after u).

**Iterations:** Single round has soundness ≤ 1/2 (prover guesses correctly with prob 1/2). Repeat protocol k times → soundness ≤ 2^-k.

### Schnorr Protocol

More efficient variant (single-round soundness 1/q instead of 1/2 for large challenge space):

1. Prover sends u = g^r mod p
2. Verifier sends large random challenge c
3. Prover sends z = r + c×x, verification: g^z = u × y^c

Used in practice: Schnorr signatures (formalized version), used in Taproot (Bitcoin), SSH keys.

---

## Non-Interactive Proofs: SNARKs

**SNARK = Succinct Non-Interactive Argument of Knowledge.**

Characteristics:
- **Succinct:** Proof size is constant or logarithmic (independent of statement size)
- **Non-interactive:** Single message from prover to verifier (no back-and-forth)
- **Arguments:** Soundness only against computationally bounded provers (not information-theoretic)
- **Knowledge:** Prover proves knowledge of witness, not just truth of statement

### Trusted Setup Problem

Most SNARKs require a **trusted setup phase**: generating public parameters from random secret information. If setup secret is compromised, soundness breaks (attacker can forge proofs).

**Setup process:**
1. Secret randomness θ is generated (usually by ceremony with multiple parties, each contributing entropy)
2. Public parameters computed: g^θ, g^(θ^2), ..., g^(θ^n)
3. Secret θ is destroyed (hardest part—must be cryptographically erased)

**Result:** Public parameters enable anyone to generate proofs; but if θ leaks, proofs can be forged.

**Multi-party computation (MPC):** Ceremony with 100+ participants; each contributes randomness; θ = θ₁ + θ₂ + ... + θ₁₀₀. Even if one party is compromised, θ remains secret (XOR property).

### Pairing-Based SNARKs

Common construction (Pinocchio, Groth16): rely on bilinear pairings — mathematical operations on elliptic curves allowing certain computations that verify ZKP structure.

**Proof size:** ~288 bytes (constant-size).
**Verification time:** ~milliseconds (fast).
**Prover time:** Polylogarithmic in circuit size (feasible for moderate-size programs).

**Example:** Prove SHA-256(x) = y without revealing x.
- Circuit: 20,000+ gates
- Proof: ~300 bytes
- Verification: <100 ms

**Soundness:** 2^-128 (computational security).

---

## STARKs: Transparent, Hash-Based Proofs

**STARK = Scalable Transparent Argument of Knowledge.**

Key differences from SNARKs:

### Transparent (No Trusted Setup)

Public parameters generated without secret randomness (e.g., hash output). Security independent of setup assumptions.

### Hash-Based Security

Rely on collision resistance of cryptographic hash (SHA-256), not on algebraic assumptions (pairings, discrete log). Believed quantum-resistant (security depends on hash strength, not number-theoretic hard problems).

### Performance

- **Proof size:** ~100–200 KB (larger than SNARKs, but logarithmic in execution steps)
- **Verification time:** ~seconds
- **Prover time:** O(n log n) where n = circuit size (slower than SNARKs but still feasible)
- **Recursion:** STARKs can prove other STARKs (composable).

### Concrete Examples

**StarkWare (Ethereum layer 2):** StarkNet batch executes transactions, generates STARK to prove correct execution. Ethereum verifies STARK (instead of re-executing).

**zkSync (alternative):** Uses SNARKs (Groth16) for proof generation, STARKs for recursion.

---

## Bulletproofs

Range proofs and general zero-knowledge proofs with small proof size (~660 bytes) and no trusted setup.

**Key feature:** Logarithmic proof size in number of secret bits (useful for proving amounts in private transactions).

**Use case:** Monero (private cryptocurrency) uses Bulletproofs to prove transaction amounts are valid (0–2^64) without revealing amounts.

**Tradeoffs:**
- Proof size larger than SNARKs but smaller than STARKs
- Verification slower than both
- No trusted setup (advantage over SNARKs)
- Hash-based (believed post-quantum, advantage over pairing-based SNARKs)

---

## Applications

### Blockchain / Cryptocurrency

**Privacy:** Prove possession of funds without revealing identity or amounts.

- **Zcash:** Shielded transactions using SNARKs. User proves knowledge of valid spending key without revealing transaction details.
- **Monero:** Range proofs (Bulletproofs) + ring signatures for privacy.
- **zkSync, Polygon Hermez:** Batch thousands of transactions, prove via SNARK. L2 scalability: throughput increases 100-1000×.

### Identity Verification

Prove attributes (age, citizenship, credentials) without revealing identity:

- **Proof of age:** ZKP attestation: older than 18, without revealing birthdate
- **University degree:** Prove graduation from MIT without revealing transcript
- Self-sovereign identity (SSI): credentials issued, proven zero-knowledge

### Privacy-Preserving Computation

**Multi-party computation (MPC):** Parties collaboratively compute functions without revealing private inputs to each other.

**Example:** Auctions where bids are private; ZKP proves bid validity without revealing amount.

### Voting

Voter proves they cast a valid vote without revealing how they voted:

- **Privacy:** Proof of valid encryption of ballot
- **Verifiability:** Voter can verify their ballot was counted (linkable proof)

### Compliance / Audits

Prove financial properties without revealing transaction details:

- **Tax compliance:** Prove taxes were paid on income, without revealing income or account balances
- **Bank audits:** Prove liabilities balance without revealing account details

---

## Design and Implementation Challenges

### Prover Efficiency

Generating proofs is expensive (especially for large programs). Tradeoff between proof size/verification speed and prover time.

**Optimization techniques:**
- Arithmetic circuit compilers: convert program to efficient gate representation (minimize non-linear operations, gates with inputs to multiple destinations)
- Witness generation: prover must solve constraints; computationally intense for large circuits
- Parallel proving: scale across GPUs/distributed hardware

### Verifier Efficiency

Verification must be fast (milliseconds for practical systems). SNARKs achieve this; STARKs slower (seconds).

### Circuit Design

Converting any program to a ZKP circuit is non-trivial. Circuits must be:
- **Deterministic:** No loops of unknown length, no data-dependent branching (forces path)
- **Arithmetic:** Operations in finite field (GF(p)) not native 64-bit integers
- **Constraint-friendly:** Multiply only where necessary (quadratic operations expensive)

Recent languages (Circom, Cairo, Noir) abstract circuit details.

---

## Current State and Tradeoffs

| Property | SNARKs | STARKs | Bulletproofs |
| --- | --- | --- | --- |
| Trusted Setup | Yes (risky) | No | No |
| Proof Size | 300 bytes | 100 KB | 660 bytes |
| Verification | <100 ms | ~1 sec | Variable |
| Post-Quantum | Unlikely | Likely | Likely |
| Maturity | Deployed (many blockchains) | Growing | Stable |
| Implementation | Complex (pairings) | Simpler (hashes) | Moderate |

**Adoption:**
- **SNARKs:** Zcash (Sapling), Filecoin, Ethereum layer 2s prefer for speed
- **STARKs:** Emerging in L2s (StarkNet, Cairo ecosystem), privacy coins starting to adopt
- **Bulletproofs:** Monero, some privacy applications

---

## Practical Considerations

**When to use ZKPs:**

✓ Privacy required (transaction amount, identity, credentials)
✓ Scalability via rollups (batch prove, single verify on chain)
✓ Regulatory proof (prove compliance without disclosing details)
✓ Authentication (prove knowledge without revealing secret)

**When not to use:**
✗ Proof generation is too slow for your latency budget
✗ Verifier infrastructure doesn't support pairing-based crypto (STARKs may be simpler)
✗ Simple authentication suffices (passwords, signatures are simpler)

**Production challenges:**
- Circuit design errors cause soundness breaks
- Side-channel attacks during prover execution (witness extraction on malicious verifier)
- Key management for setup ceremonies (if using SNARKs)
- Witness compression for complex programs (storage, computation cost)

---

## See Also

- security-cryptography-asymmetric.md (elliptic curves, pairings, signatures)
- cryptography-hash-functions.md (hash security, pre-image resistance)
- blockchain-smart-contracts.md (on-chain verification, gas optimization)
- security-identity.md (credential verification, authentication)