# DAOs — Decentralized Autonomous Organizations, Governance Mechanisms & Attacks

## Overview

A Decentralized Autonomous Organization (DAO) is an organization whose governance and operational rules are encoded in smart contracts on a blockchain. No central authority; decisions made by consensus of token holders (or other voting mechanisms). Operational examples: DeFi protocol governance (Aave, Compound), treasury management (Optimism Collective), NFT communities (ConstitutionDAO).

Core tension: decentralization enables censorship-resistance but fractures decision-making; faster centralized governance often outpaces DAO deliberation.

## Governance Mechanisms

### Token-Weighted Voting (Plutocracy)

Standard model: One token = one vote. Voting power directly proportional to token balance.

**Mechanism:**

1. Voting period opens for discrete proposal (e.g., "adjust protocol fee to 0.2%")
2. Token holders call `vote(proposalId, support)` where support ∈ {for, against, abstain}
3. Period closes; tally votes; execute if threshold met (e.g., > 50% for)

**Implementations:** Aave Governor, OpenZeppelin Governor (default template).

**Issues:**

- **Whale dominance:** Large holders control outcomes; medial holders' votes negligible
- **Voter apathy:** Quorum requirements often unmet (~10-20% participation typical); governance stuck
- **Vote buying:** Protocol tokens bought pre-vote to capture governance; sold post-vote

### Quadratic Voting

Voting power = sqrt(token balance). Marginal vote strength decreases with balance; intended to reduce whale dominance.

**Example:** Alice holds 100 tokens (10 votes). Bob holds 10,000 tokens (~100 votes). Alice's marginal contribution per token much higher.

**Tradeoff:** Reduces whale control but doesn't eliminate it; marginal benefit tapers.

**Implementations:** Curve DAO (cvxCRV voting), Polkadot.

### Conviction Voting

Vote weight = tokens held × lock duration. Longer lock-up = stronger vote.

**Mechanics:**

```
vote_weight = balance × days_locked / max_lock_period
```

Incentivizes commitment; users signaling conviction must accept illiquidity risk.

**Implementation:** Polkadot governance, Optimism governance (vote escrow variant).

**Tradeoff:** Reduces whale impact (whale can't keep capital liquid and vote). Creates incentive alignment but locks up capital; less liquid voting than token-weighted.

### Vote Escrow (ve)

Tokens locked for fixed duration T become "escrow tokens" (veTokens) providing voting rights. Unlock at expiration.

**Mechanics:** User locks CRV for max 4 years → receives veCRV proportional to lock period. veCRV votes on protocol decisions; CRV holders cannot vote without locking.

**Incentive:** Aligns voter horizon with protocol performance (voter accepts illiquidity). Reduces mercenary voting (vote buying less profitable if capital must stay locked).

**Derivative markets:** LPing (e.g., Convex) lets users earn yield on locked tokens while maintaining partial vote delegation.

### Delegation

Explicit delegation without transfer: token holder authorizes another address to vote on their behalf. Holder retains custody; delegate votes.

**Use cases:** Retail holder delegates to trusted DAO contributor; liquid delegation markets.

**Risks:** Delegate can vote against delegator's interest; concentration of voting power if many delegate to few addresses.

## Proposal Lifecycle

Typical flow (Aave Governor example):

1. **Discussion:** Informal debate off-chain (Discourse forum, Snapshot)
2. **Snapshot vote:** Gasless off-chain vote using voting power snapshot at prior block
3. **On-chain proposal:** Proposer submits formal proposal on-chain with vote cooldown (e.g., 1 block delay)
4. **Voting period:** 3-10 days; token holders vote on-chain
5. **Evaluation:** If passing threshold, automatically executes after delay or manual trigger
6. **Execution:** Smart contract enacts decision (parameter change, fund transfer, etc.)

**Rationale for delays:** Prevents flash loan attacks; grants users time to review and potentially exit if extreme changes planned.

## Governance Attacks

### Flash Loan Attacks

Attacker borrows large token quantity via flash loan, votes maliciously in governance, repays within same block.

**2022 Beanstalk exploit:** Attacker borrowed BEAN via Curve, voted to mint unfavorable tokens, triggered protocol collapse. Loss: ~$76M.

**Mitigations:**

- **Voting snapshot:** Voting power determined at block N-1 (prior to current block). Flash loans taken in current block have zero voting power.
- **Vote escrow:** Requires lock-up; flash loan is too short to participate.
- **Blocklock:** Delay between receiving delegation and voting (e.g., must hold for 1+ blocks before voting).

### Large Holder / 51% Attack

Majority token holder unilaterally controls governance.

**2022 Beanstalk follow-up:** After collapse, new governance structure required to prevent repeat. Adopted decentralized validator set + timelock.

**Mitigations:**

1. Token distribution sufficiently dispersed (no single holder > critical threshold)
2. Governance timelock: Delay between vote passing and execution, enabling community response
3. Multisig veto: Emergency multisig can veto malicious proposals (re-introduces centralization but limits damage)

### Voter Apathy

Low participation concentrates voting power among engaged factions. Governance becomes oligarchic.

**Empirical:** Cosmos Hub governance ~5-10% participation; Aave ~20-30%.

**Mitigations:**

- Steeper quorum (require 40%+ participation)
- Quadratic voting incentives broader participation
- Delegation enables participation without direct involvement

### Proposal Spam / Griefing

Attacker floods governance with junk proposals, consuming validator/voter attention.

**Mitigation:** Proposal bond; proposer must stake tokens (e.g., 80k AAVE). Bond forfeited if proposal fails, otherwise returned.

### Governance Captured by External Incentives

Large DeFi player (e.g., Convex in Curve governance) accumulates voting rights and votes purely for self-interest, not protocol benefit.

**Mechanism:** Voter X holds CVE, votes according to Convex incentives rather than Curve protocol health.

**Status:** Not considered "attack" but governance misalignment; community monitors and re-distributes voting power if necessary.

## Governance Frameworks

### Snapshot

Off-chain voting infrastructure. Users vote using wallet signatures (no transaction cost). Voting power indexing from on-chain snapshots.

**Use:** Temperature checks, informal governance.

**Trust model:** Snapshot does not enforce vote execution; DAO multisig or contract must implement decisions. Snapshot is advisory.

**Examples:** Uniswap, Curve, Lido use Snapshot + on-chain.

### Tally

Governance UI and on-chain Governor contract deployment. Integrates Snapshot for temp checks, then on-chain voting for formal decisions.

Tally provides: proposal creation, voting interface, vote delegation tracking.

### Aragon

DAO operating system. Pre-built governance contracts (voting, treasury, token management). Enables non-technical founders to deploy DAO.

Lower adoption than generic Governors; Aragon DAO governance somewhat opinionated.

## Treasury Management

DAOs accumulate reserves: protocol revenue (fees), donations, grants. Treasury governance decides deployment.

**Use cases:**

1. **Insurance reserve:** Cover protocol losses (e.g., smart contract bug)
2. **Investment:** Purchase yield-generating assets (e.g., stETH, yield bearing stables)
3. **Grants:** Fund protocol development, researcher bounties, marketing
4. **Buyback/burn:** Repurchase own token to increase scarcity
5. **Liquidity:** Ensure adequate token liquidity on DEXs

**Constraint:** Suboptimal capital allocation. Committee (multisig or proposal-based voting) makes investment decisions slower and less agile than centralized fund managers. Risk: concentrated in DAO decision-makers' risk preferences rather than diversified market.

### Multi-Party Computation (MPC) Wallets

Some DAOs use MPC (secure computation across multiple signers) to custody treasury instead of centralized multisig.

**Benefit:** No single point of failure.

**Drawback:** Complex, unproven; operational failures possible (signer coordination, key loss).

## Legal Wrappers

### Wyoming DAO LLC

Wyoming state law (2021) enabled legal recognition of DAOs as Limited Liability Companies. DAO token holders become members; DAO can execute contracts, own property, sue/be sued.

**Benefit:** Legal clarity; DAO can interact with traditional finance (bank accounts, loans).

**Limitation:** Requires nomination of registered agent (person); reintroduces legal centralization.

### Entity Structures

- **Wyoming DAO LLC:** US jurisdiction, limited precedent
- **German cooperative:** Some European DAOs registered as e-V (eingetragener Verein)
- **Singapore pte ltd:** Discretionary; not explicitly DAO-friendly

**Reality:** Most DAOs operate de facto without legal characterization; ambiguity persists.

## Voting Paradoxes & Design Challenges

### Arrow's Impossibility Theorem

No voting mechanism simultaneously satisfies:
1. Unrestricted domain (any preference profile is valid)
2. Non-dictatorship (no single voter decides)
3. Pareto efficiency (if everyone prefers A over B, outcome is A)
4. Independence of irrelevant alternatives (relevance of A vs. B independent of C)

**Implication:** All voting systems have inherent tradeoffs. Plutocratic voting satisfies (1), (3), (4) but fails (2). Quadratic voting may violate (4) subtly.

### Voter Coordination Problems

**Tragedy of commons:** Voter A cares about protocol health. Voter A votes for optimal outcome. But if Voter B, C, D vote selfishly, outcome suboptimal. Voter A's rational response: also vote selfishly to compete. Result: tragedy.

**Mitigation:** Credible commitment (conviction voting, ve lock-up increases cost of defection).

## Real-World Case Studies

### Aave Governance

- ~500M AAVE in circulation; ~15% avg participation in votes
- Voting snapshots at prior blocks prevent flash loans
- Parameter decisions (interest rates, collateral acceptance) via governance
- AaveChan (governance service) helps coordinate off-chain discussion

### Optimism Retroactive PGF (RetroPGF)

Alternative model: Badge-holders (not token holders) vote on funding projects. Voting power = merit assessment, not capital. Used to fund Optimism ecosystem public goods.

**Innovation:** Decouples governance from token holdings; enables "reputation" voting.

**Risk:** Circularly defined ("who chooses badge holders?"); sybil attacks possible.

### MakerDAO Governance

Extremely complex: MKR token voting, multiple stability fees, collateral onboarding. Suffered governance attacks (flash loan) and systematic debates over reserve backing (3AC collapse exposed risk concentration).

**Observation:** Complexity enables nuance but also enables gridlock and attack surface.

## Emerging Models

### Conviction-based grants

Participants lock tokens to express support for project over time. Ongoing commitment > one-time vote.

### Futarchy

Base decisions on prediction markets. Instead of voting "should we do X?", market predicts outcome of X; governance chooses option with best predicted return.

Rarely deployed; complex mechanism design and oracle trust assumptions.

### Liquid democracy

Hybrid: voters can delegate, delegated voting cascades (A delegates to B, B delegates to C → C has A and B's vote). Enables spectrum between direct and representative democracy.

Complex to implement; cascading vulnerabilities possible (if C is bad actor, all delegators hurt).

## Mental Models

**Voting power ≠ wisdom:** Token holders motivated by profit, not protocol health. Governance misalignment inevitable.

**Decentralization is expensive:** Off-chain coordination, on-chain execution delays, security overhead. Centralized decision-makers often faster and cheaper.

**Timelock is not optional:** Delay between vote and execution is the primary defense against governance attacks; enable community response.

**Governance capture via incentives:** Whale token holders with outside incentives vote against protocol. Monitor and adjust voting distribution if necessary.