# Token Standards — ERC-20, ERC-721, ERC-1155, Governance & Soulbound

## Overview

Token standards define the interface for transferrable digital assets on Ethereum and compatible blockchains. A standard is a specification + test suite; contract implementers must expose specific functions and events. Standards enable ecosystem composability: wallets, exchanges, and protocols recognize any conforming token without custom integration.

Key tension: standards trade flexibility for compatibility. Newer standards (ERC-1155) add features; older standards (ERC-20) remain dominant due to network effects.

## ERC-20 — Fungible Tokens

### Interface

Fungible tokens represent interchangeable units (e.g., USD where each unit is identical).

**Core functions:**

```solidity
transfer(to, amount) → bool
approve(spender, amount) → bool
transferFrom(from, to, amount) → bool
balanceOf(account) → uint256
allowance(owner, spender) → uint256
totalSupply() → uint256
```

**Events:**

```solidity
Transfer(from, to, value)
Approval(owner, spender, value)
```

### Approval Mechanism

Two-step pattern for spending others' tokens:

1. Owner calls `approve(spender, amount)` → grants allowance
2. Spender calls `transferFrom(owner, spender, amount)` ← pulls funds

**Motivation:** Decoupling authorization from transfer enables smart contracts to accept and move funds on behalf of the user.

**Race condition:** Between approve and transferFrom, allowance can change. Classic attack: user calls approve(100), then approve(200). Attacker front-runs by submitting transferFrom(100) after first approve but before second, then immediately submits another transferFrom after second approve lands. Old ERC-20 implementations don't prevent this; newer contracts use `increaseAllowance` / `decreaseAllowance`.

### Variants & Extensions

**SafeERC20:** Wraps transfer/transferFrom to catch return value false (non-reverting transfers) instead of silent failure.

**Metadata:** `name()`, `symbol()`, `decimals()` —not required by spec but universally expected.

**Burn:** Implement `burn(amount)` to permanently remove tokens from supply.

**Mint:** Authority-restricted `mint(to, amount)` to increase supply.

## ERC-721 — Non-Fungible Tokens (NFTs)

### Interface

Each token is unique, identified by tokenId (uint256). One owner per token.

**Core functions:**

```solidity
transferFrom(from, to, tokenId)
safeTransferFrom(from, to, tokenId) → accepts callback
ownerOf(tokenId) → address
balanceOf(account) → uint256
approve(to, tokenId)
setApprovalForAll(operator, approved)
getApproved(tokenId) → address
isApprovedForAll(owner, operator) → bool
```

### Data Storage

Metadata (name, image, attributes) typically stored off-chain (IPFS, HTTP) and referenced via URI-based lookup.

```solidity
tokenURI(tokenId) → string
```

**Risks:**

- **Centralized hosting:** HTTP URLs can disappear; IPFS requires pinning.
- **Immutability mismatch:** Metadata can be rehosted with different content while pointing to same URI. NFT remains "immutable" but metadata changed.

### Approval Model

Two approval patterns:

- **Single token:** `approve(to, tokenId)` → grants right to transfer specific token
- **Blanket:** `setApprovalForAll(operator, true)` → grants operator right to move all tokens

**Reason for two:** Reduces number of transactions for bulk operations (e.g., marketplace listing all tokens).

## ERC-1155 — Multi-Token Standard

### Design

Single contract manages multiple token types (fungible and non-fungible intermixed). Efficient batch operations.

**Core functions:**

```solidity
balanceOf(account, tokenId) → uint256
balanceOfBatch(accounts[], ids[]) → uint256[]
safeTransferFrom(from, to, id, amount, data)
safeBatchTransferFrom(from, to, ids[], amounts[], data)
setApprovalForAll(operator, approved)
isApprovedForAll(owner, operator) → bool
```

### Efficiency Gains

**Batch transfers:** Single transaction moves multiple tokens to multiple recipients.

**Storage efficiency:** One contract holds multiple token types; reduces state bloat compared to separate ERC-20 + ERC-721 contracts.

**Atomic swaps:** Single transaction swaps fungible + non-fungible assets in one atomic operation.

### Use Cases

- **Gaming:** Player inventory mixes equipment (NFT), currency (fungible), crafting materials (semi-fungible).
- **Fractional ownership:** Single non-divisible asset split into N identical tokens (fungible), then wrapped as ERC-1155.

## Soulbound Tokens (SBTs)

### Definition

Non-transferrable tokens bound to an address (wallet). Transfer-blocking mechanism: `transfer` and `approve` revert or are removed entirely.

**Intent:** Represent credentials, identity, reputation—assets that lose meaning if transferred.

### Design Patterns

**Mint-only:** `mint(to, id)` allowed; `transferFrom` reverts permanently.

**Revocable:** Issuer (credential provider) can revoke token unilaterally. Example: diploma revoked if plagiarism discovered.

**Delegates:** Limited delegation allowed (e.g., read-only viewing of credentials by third parties) without full custody transfer.

### Challenges

**Identity assumption:** Assumes account = identity hold. If private key compromised, attacker gains stolen credentials. No recovery mechanism (unlike replaceable DeFi collateral).

**Standardization gap:** No consensus specification (EIP-4973 proposed but not widely adopted). Each implementation differs in revocation, delegation, composability.

**Ecosystem friction:** DeFi protocols don't recognize SBTs; SBT holdings can't directly collateralize loans or be swapped.

## Governance Tokens

### Role

Grant holders voting rights on protocol parameters, fund allocation, and strategic decisions. Typically no inherent utility; value derives from voting power and protocol cash flows.

**Distribution models:**

- **Airdrop:** Free distribution to early users, bootstrapping decentralization
- **Emission:** Continuous issuance to liquidity providers or stakers
- **Sale:** Token sale or ICO

### Voting Mechanisms

**Plutocracy:** Votes proportional to holdings. One token = one vote. Whales have disproportionate influence.

**Quadratic voting:** Voting power = sqrt(tokens held). Reduces whale dominance; marginal vote weight decreases with balance. Introduced by Curve, Polkadot, others.

**Conviction voting:** Vote weight = tokens × lock duration. Longer commitment = more influence. Encourages long-term alignment.

**Delegation:** Token holder can delegate voting rights to another address without transferring tokens themselves. Enables participation without on-chain voting complexity.

### Governance Attacks

**Flash loan governance:** Attacker borrows governance tokens via flash loan, votes maliciously, repays within block. Mitigations: voting snapshots (vote taken at prior block), blocklock (delay between delegation and voting), or vote escrow (require token lock-up).

**Voter apathy:** Low participation (10-20% typical) concentrates voting power. Quorum requirements may not be met; governance deadlock.

**51% attack:** Majority token holder or coalition captures governance. Recent example: 2022 Beanstalk exploit where attacker acquired 51% voting power via flash loan, voted to mint new plant tokens, dumped them, collapsed protocol.

## Utility vs. Security Token Distinction

### Utility Tokens

Provide access to protocol or service (e.g., UNI for swapping on Uniswap). Typically not regulated as securities if:
- No expectation of profit from third-party efforts
- Cannot be traded on secondary markets (or treated like equity if tradeable)

Regulatory uncertainty remains; classification varies by jurisdiction.

### Security Tokens

Represent ownership or cash flow rights (e.g., equity, bonds, derivatives). Subject to securities regulation (SEC, regulatory equivalents abroad).

**Compliance requirements:**
- KYC/AML on holders
- Restricted secondary markets (accredited investors only)
- Periodic reporting

**On-chain infrastructure:** Limited; most security tokens operate on private blockchains (e.g., Avalanche subnets, Hyperledger) due to regulatory friction.

## Token Economics

### Supply Schedule

**Fixed cap:** Maximum supply defined at genesis (e.g., Bitcoin 21M). Incentivizes scarcity narratives; limits future program creation.

**Inflationary:** Continuous minting (e.g., Ethereum ~2% per year historically). Aligns incentives for long-term staking and protocol development.

**Deflationary:** Burning mechanism reduces supply (e.g., Ethereum post-EIP-1559: base fee burned). Often combined with inflation to balance incentives.

### Emission Schedules

**Linear:** Fixed emission per block (e.g., first 4 years of Bitcoin: 50 BTC/10min).

**Exponential decay:** Emission halves at intervals (Bitcoin halving every ~4 years). Incentivizes early participation; reduces value of future emissions.

**Hyperbolic:** Asymptotic curve. Early high emission, rapid decline, long tail. Used by some DeFi protocols to bootstrap, then drain incentives.

### Vesting & Lockups

**Vesting cliff:** Full unlock after fixed time (e.g., 1 year post-token launch).

**Linear vesting:** Gradual unlock over period (e.g., 4 years at 1/1461 per day).

**Motivation:** Prevent early dumping; align team incentives with long-term success; avoid supply shocks.

## Interoperability & Bridges

Multi-chain tokens require bridge contracts: custodial or algorithmic.

**Custodial bridge:** Lock token on source chain; bridge authority mints wrapped token on destination chain. Requires trust in bridge authority.

**Atomic bridges:** Cross-chain swap without trusted custodian. More complex; fewer deployed examples.

**Risk:** Bridge compromises are common attack vectors (Ronin ~$625M, Nomad ~$190M, Poly ~$611M 2022-2023).

## Standards Fragmentation

Not all token interfaces follow ERC-20 / ERC-721 / ERC-1155. Some protocols use custom interfaces optimizing for specific use cases:

- **Rebasing tokens:** Supply adjusts daily; balanceOf changes without transfer. Breaks many DeFi assumptions.
- **Fee-on-transfer:** Transfer fee deducted; actual received amount != sent amount. Pools assume 1:1 balance accounting; breaks AMMs without post-transfer checks.
- **ERC-777:** Hooks enabling receiver logic; largely obsoleted by ERC-1155 and failed to gain adoption.

Ecosystem must handle divergence via safe wrapper libraries or protocol-specific integrations.