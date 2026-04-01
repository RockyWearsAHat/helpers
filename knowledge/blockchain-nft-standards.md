# NFT Standards — ERC-721, ERC-1155, Token-Bound Accounts, and Metadata

## Overview

NFT standards define smart contract interfaces for representing digital ownership on blockchains. Unlike fungible tokens (ERC-20), where 1 unit ≡ any other unit, NFTs are individually unique and non-divisible (or partially divisible in newer standards). Each token carries identity, ownership, and often metadata.

Standards enable interoperability: all ERC-721 wallets, marketplaces, and tools recognize each other's tokens. **This composability is more valuable than the standard itself.**

## ERC-721: Single-Token Contract

The foundational NFT standard. Each contract manages many NFTs; each NFT identified by unique token ID.

### Interface

```solidity
function transferFrom(address from, address to, uint tokenId)
function approve(address to, uint tokenId)
function setApprovalForAll(address operator, bool approved)
function balanceOf(address owner) returns (uint)
function ownerOf(uint tokenId) returns (address)
function safeTransferFrom(...)  // Safe variant checks if recipient is contract
```

### Ownership Model

- **Single owner per token:** `tokenId → owner`. Transfer changes owner atomically.
- **Approval pattern:** Owner can approve another address to transfer on their behalf. Enables: marketplaces (owner approves marketplace contract, marketplace transfers), token sales.
- **Safe transfer:** Calls `onERC721Received` hook on recipient if it's a contract. Prevents accidental sends to contracts that don't support tokens.

### Metadata Extension (ERC-721 Metadata)

Optional but standard. Returns JSON metadata:

```json
{
  "name": "Cryptopunk #1234",
  "description": "...",
  "image": "https://ipfs.io/ipfs/QmXXX",
  "attributes": [{"trait_type": "background", "value": "blue"}]
}
```

`tokenURI(uint tokenId)` returns metadata URL. Can be:
- **Immutable:** Hardcoded IPFS hash
- **Mutable:** Server-based URL (subject to centralized control)
- **On-chain:** Metadata stored in contract, URL routes to bytecode decoder

On-chain metadata immutability is premium but expensive (~200K gas to store JSON per token).

## ERC-1155: Multi-Token Contract

Single contract manages multiple token types (fungible, semi-fungible, NFTs). More gas-efficient for batch operations.

### Key Differences from ERC-721

- **Token type flexibility:** Token ID can represent multiple units (`balanceOf(owner, tokenId)` returns count, not ownership)
- **Fungible NFT hybrid:** Can issue 1 million copies of token ID #5 (fungible) and 1 copy of token ID #6 (NFT)
- **Batch operations:** Transfer 50 different tokens in one transaction
- **Callback efficiency:** Single `onERC1155Received` guards both fungible and non-fungible transfers

### Data Model

```solidity
mapping(uint => mapping(address => uint)) balances;  // tokenId → owner → balance
```

### Use Cases

- **Gaming:** 1M fungible potions (consumables) + unique swords (NFTs) in one contract
- **Portfolios:** Index tokens combining multiple assets
- **Fractional NFTs:** Mint 10,000 shares (ERC-1155) of a valuable NFT (held by contract)

## ERC-6551: Token-Bound Accounts (Emerging Standard)

Assigns each NFT its own smart contract account (wallet). The NFT controls the account; holder of the NFT controls the NFT, hence the account.

### Architecture

**Singleton registry** (`0x02...` address canonical) maps:
$$\text{(chain_id, nft_contract, token_id)} \to \text{account_address}$$

Deterministic: same NFT always owns same account address.

### Account Capabilities

Token-bound accounts can:
- **Hold assets:** ETH, tokens, other NFTs
- **Execute code:** Call contracts, participate in protocols
- **Sign transactions:** Act as agents on behalf of NFT holder

### Use Cases

**Character as agent:**
```
Game NFT (character) → bound account → holds inventory items (ERC-1155)
                                   → earns yield farming returns
                                   → signs trades on DEX
```

**Composable assets:**
```
Car NFT → bound account holds Wheel NFTs, Seat NFTs 
                          → sells individual components via marketplace
                          → car remains functional composite
```

**Portable identity:**
```
Social NFT → bound account → cross-chain asset ownership
                          → reputation/achievement records
```

### Security Implications

- **No additional security:** Account safety depends on NFT contract security and holder key management
- **Cascading risk:** Comprise NFT contract → all bound accounts compromised
- **Cycle prevention:** Standard prevents ownership cycles (NFT A cannot own account owning NFT A)

## ERC-2981: Royalty Standard

Defines on-chain royalty metadata so marketplaces can pay creators on resales.

```solidity
function royaltyInfo(uint tokenId, uint salePrice) 
  returns (address recipient, uint royaltyAmount)
```

Returns: creator address + royalty amount (e.g., 10% of sale price). Marketplace calls it before finalizing sale.

**Trust model:** Optional; marketplace *can ignore*, but loses creator goodwill. No enforcement. Centralized marketplaces (OpenSea, Blur) usually honor; on-chain DEX contracts cannot enforce (no identity).

**Recent trend:** Many creators now distribute via token rewards rather than royalties, due to royalty enforcement erosion after 2023.

## Metadata Storage Architectures

### IPFS (Content-Addressed)

Token URI points to IPFS hash:
```
ipfs://QmXXXX.../metadata.json
```

**Advantages:** Immutable, decentralized, verifiable by hash  
**Disadvantages:** Requires IPFS pinning service (Infura, Protocol Labs, Filecoin) to guarantee persistence. Pinning can be unpinned.

### Arweave (Permanent Storage)

Permanent data storageprotocol. Metadata stored permanently for one-time fee (~$0.01 per KB in 2024).

```
ar://XYZ.../metadata.json
```

**Advantages:** True permanence (not dependent on pinning service)  
**Disadvantages:** Cost, less adoption

### On-Chain

Metadata bytecode stored in contract. Immutable and permanently available. Expensive (~15K-50K gas per token).

### Centralized Server (URL)

```
https://nft.example.com/api/metadata/1234
```

**Advantages:** Flexible, cheap  
**Disadvantages:** Subject to downtime, edits, rug-pull (creator removes content)

## Soulbound Tokens and Non-Transferable NFTs

**Soulbound tokens (SBTs):** NFTs locked to owner address. No transfer mechanism. Represent identity, credentials, attestations.

```solidity
// No transferFrom; reverts on transfer attempts
function mint(address soul, ...) { ... }
```

Use cases:
- Educational credentials (degree from university)
- Reputation badges (DAO member since 2021)
- DeFi risk scores (collateral history)

Controversies:
- **Verification problem:** How do you know a soulbound credential is real?
- **Remediation problem:** If compromised, account is tainted permanently; no token recovery
- **Privacy problem:** Soulbound identity trails are permanent public records

Current adoption: Limited. Most live examples are centralized issuer initiatives (universities, DAOs).

## Dynamic NFTs and Upgradeable Metadata

NFT metadata can change if metadata URL points to mutable source (smart contract, centralized server). Enables:

- **Leveling up:** Game character NFT gains experience, metadata updated
- **Portfolio rebalancing:** Index token composition changes, metadata reflects new holdings
- **Rental:** NFT temporarily changes metadata to reflect current renter

Tradeoff: Immutability (permanence, trust) vs. functionality (games, evolving assets).

## Scaling and Multi-Chain Interoperability

### Rollup Deployment

NFT contracts deploy on L2 (Arbitrum, Optimism, Base) with cheaper minting (~$0.10 vs. $5-50 on mainnet). Tokens don't automatically exist on mainnet — requires bridge (wrapped token, or burn-and-mint).

### Bridges

- **Custodial bridges:** Wrap token on chain A, send wrapped version to chain B; original locked (held in bridge contract)
- **Hub-and-spoke:** All tokens lock on mainnet, operate as wrapped representations on L2
- **Risk:** Bridge smart contracts become centralization point and attack target

## Performance Limits and Scaling Tradeoffs

**Bottlenecks:**
- Minting: ~50K gas per ERC-721 token (tx cost $5-50 depending on gas price)
- Marketplace transactions: Transfer + approval = 2 txs
- Metadata resolution: If on IPFS, network hop required

**Solutions:**
- Batch minting (ERC-1155, minimal overhead per token)
- Account abstraction (eliminate approval step)
- Pre-generated metadata + static serving (reduce lookup latency)

## See Also

- [blockchain-smart-contracts.md](blockchain-smart-contracts.md) — smart contract patterns and ERC standards implementations
- [web3-token-standards.md](web3-token-standards.md) — ERC-20, ERC-721, ERC-1155 specifications
- [blockchain-defi-protocols.md](blockchain-defi-protocols.md) — fractional NFT protocols
- [web3-dao.md](web3-dao.md) — governance NFTs and membership