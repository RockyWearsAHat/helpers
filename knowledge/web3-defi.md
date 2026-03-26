# DeFi Protocols — Market Mechanisms, Lending, Stablecoins & Extraction

## Overview

Decentralized Finance (DeFi) comprises open protocols for trading, lending, and asset management operating on public blockchains without custodial intermediaries. Unlike traditional finance, DeFi trades intermediary control for programmable rules enforced by smart contracts. Key innovation: composability—protocols stack into complex strategies ("money legos").

Core tension: DeFi removes counterparty trust but replaces it with smart contract risk, market volatility risk, and extraction incentives (MEV).

## Automated Market Makers (AMMs)

### The Constant Product Formula

Classical AMM mechanism discovered by Uniswap v1 (2018). Replaces order books with liquidity pools.

**Mechanism:**

```
x * y = k
```

Where:
- `x` = reserve of token A
- `y` = reserve of token B
- `k` = constant (invariant)

When a trader swaps Δx of token A for token B, the pool maintains the product invariant:

```
(x + Δx * (1 - fee)) * (y - Δy) = k
```

Solving for output: `Δy = y - k / (x + Δx)`

**Properties:**

- **No slippage floor.** As Δx grows, slippage increases nonlinearly; trades of size k/2 suffer ~33% slippage.
- **Liquidity provider (LP) profit.** LPs earn fees on every trade; proportional to share of pool reserves.
- **LP impermanent loss.** If token prices diverge significantly from entry price, LP's dollar value can fall below their initial deposit despite fee earnings. Example: entering at 1:1 ratio, token A doubles in price → LP recovers ~70% of initial value despite fees (with typical 0.3% fee).

### Concentrated Liquidity (Uniswap v3)

Prior AMMs suffered capital inefficiency: most swaps occur near the current price; liquidity deep out-of-the-money is unused.

**Concentrated liquidity** lets LPs specify a price range (lower tick, upper tick). Capital deployed only within that range.

**Trade-off:** Higher capital efficiency but exposure to larger impermanent loss if price moves outside the range. LPs with narrow ranges profit more from fees but face liquidity withdrawal risk; those with wide ranges behave like classical AMMs.

**Capital efficiency:** 1 unit of capital in concentrated liquidity [lower, upper] is equivalent to ~1970 units in a classical AMM near the current price.

### Alternative Curve Families

**Stableswap curve** (Curve Finance): Optimized for low-slippage swaps between stablecoins and pegged assets. Curve resembles a flat line near 1:1 until price deviates sharply, reducing slippage on typical pairs (USDC/USDT).

**Hybrid curves:** Mix multiple invariants to balance capital efficiency with stable-pair trading.

## Lending Protocols

### Core Abstraction

Lending protocols tokenize debt. Users deposit collateral in exchange for a loan or deposits in exchange for yield.

**Mechanism:**

1. Depositor sends token X → receives interest-bearing token iX
2. Borrower deposits collateral (e.g., ETH) → borrows token X at interest
3. Protocol maintains collateral ratio (CR): `CR = collateral_value / borrow_value`
4. If CR falls below threshold, collateral is liquidated

### Interest Rate Determination

**Variable rate:** Utilization-based. As utilization U (loaned / available) rises, interest rate increases. Example: Aave uses piecewise linear rate curve:

- U ≤ optimal: rate = baseRate + U/optimal * multiplier
- U > optimal: rate = baseRate + multiplier + (U - optimal) / (1 - optimal) * slope

Equilibrium assumed near optimal utilization; if capital drains (U drops), rates drop, attracting new borrowing.

**Flash loan risk:** A borrower can loan entire pool balance within a single transaction, provided they repay (with fees) by end-of-block. This enables arbitrage but also governance manipulation and liquidation cascades.

### Liquidation Mechanics

**Liquidator role:** Any third party. If CR falls below threshold:

1. Liquidator repays portion of borrower's debt
2. Liquidator receives collateral at discount (e.g., 5-10% below market)

**Incentives:** Liquidators profit from discount; keepers (bots) compete to identify liquidatable positions first. In volatile markets, liquidation cascades can occur: one liquidation triggers others via price movement, potentially exhausting liquidator capital and leaving bad debt (under-collateralized positions).

**Protocol design:** Governance votes on liquidation thresholds, discount percentages, and reserve factors (fraction of interest routed to protocol treasury).

## Stablecoins

### Collateralized Stablecoins

**Mechanism:** Over-collateralized. Holder locks collateral worth $N, receives $X stablecoins where $X < $N.

**Examples:**

- **DAI (MakerDAO):** Backed by ETH, USDC, and other collateral. When collateral value drops, MKR token holders vote to raise stability fees (borrowing interest). Protocol charge liquidation penalties.
- **LUSD (Liquity):** Over-collateralized by ETH alone. Fixed 0.5% borrowing fee and 10% liquidation bonus; decentralized governance replaced by algorithmic parameters.

**Advantages:** Censorship-resistant issuance; no custodial counterparty.

**Risks:** Collateral concentration (DAI previously >70% USDC-backed), liquidation cascade under extreme volatility, governance attack via large MKR acquisition.

### Algorithmic Stablecoins

**Mechanism:** Stability maintained by elastic supply or arbitrage incentives rather than direct collateral backing.

**Rebase stablecoins:** Supply adjusts algorithmically to maintain peg. Example: Ampleforth rebases balances daily; if price > $1, everyone's balance increases (incentivizing selling); if price < $1, balances decrease.

**Seigniorage-share model:** System maintains two or more tokens. Primary token targets price; secondary tokens absorb volatility. When primary trades above peg, secondary tokens are minted (diluting primary holders); when below peg, secondaries are burned.

**Failure mode:** Bank runs. If users lose confidence in the peg or incentive mechanism, liquidity evaporates and the peg breaks irreversibly.

### Centralized Reserves

**USDC, USDT:** Issued by centralized entities backed 1:1 by fiat reserves. Not DeFi-native but integrated into all major protocols.

**Trade-off:** Reliable peg and scale; censorship risk (issuer can freeze addresses, blacklist, halt redemptions).

## Yield Farming & Composability

**Yield farming:** Users earn protocol-native governance tokens (e.g., UNI, AAVE, COMP) by depositing into liquidity pools or lending markets beyond their natural yield.

**Incentive:** Bootstraps liquidity; aligns user incentives with governance.

**Risk:** Token value collapse if farming incentives dry up. Capital chase dynamics common: when APY is high, capital floods in; when incentives reduce, capital flees, causing slippage and losses.

**Composability:** Users combine primitives. Example: borrow USDC on Aave → swap for ETH on Uniswap → deposit in Curve pool → stake LP tokens on Convex → earn compounded yield. Each layer adds smart contract risk and gas cost.

## Maximal Extractable Value (MEV)

**Definition:** Profit a user or validator can extract by reordering, inserting, or censoring transactions within a block.

### Forms

**Front-running:** Observer sees pending swap transaction, issues similar swap ahead of it, benefits from price impact.

**Sandwich attacks:** Attacker includes user transaction between two of their own transactions to extract slippage.

**Liquidation races:** Multiple liquidators submit transactions to capture liquidation profits; highest gas bidder wins.

**Arbitrage:** Exploit flash loan to arbitrage price discrepancies across pools atomically within a single block.

### Defense Mechanisms

**Private mempools (dark pools):** Transactions kept private until inclusion; Flashbots Relay, MEV-Hide.

**PBS (Proposer-Builder Separation):** Block proposer (validator) does not construct blocks; separate builder creates and proposes blocks. Reduces validator MEV at cost of centralization.

**Threshold encryption:** Transactions encrypted until after block is finalized, preventing front-running. Not yet widely deployed.

**Intent-based architectures:** Users express intent (e.g., "swap 1 ETH for 30k+ USDC") rather than exact execution path; solvers compete to execute at best price. Nascent.

## Liquidation Cascades & Systemic Risk

**Cascade mechanism:** Liquidation of position A causes collateral price to drop → triggers liquidation of position B → further price drop → position C → ...

**2023 Aave case study (ETH crash):** Rapid ETH price decline cascaded into forced liquidations across ETH-backed borrowing; liquidators exhausted capital; protocol accumulated bad debt requiring governance negotiation.

**Mitigation:**

- Diverse collateral backing (not concentrated in one asset)
- Insurance pools (e.g., Aave Safety Module reserves)
- Circuit breakers: pause liquidations during extreme volatility
- Decoupling risk tiers: high-risk collateral has higher requirements and lower borrowing caps

## Governance & Treasury Management

Most DeFi protocols issue governance tokens (UNI, AAVE, COMP) enabling decentralized parameter control.

**Typical parameters:** Fee tiers, reserve factors (revenue split), addition of new collateral, emergency pauses.

**Governance attack surface:** Flash loan attacks (adversary borrows large token quantity, votes, repays within block); large holder capture (whales control governance); voter apathy (quorum thresholds not met, governance deadlock).

**Treasury:** Protocols accumulate reserves (fees, liquidation premiums, liquidated collateral). Governance decides deployment: protocol insurance, yield generation, token buybacks, protocol development.

## Key Mental Models

**Invariant protection:** AMMs and lending protocols rely on mathematical invariants (constant product, CR threshold). Market volatility and flash loans test these invariants under extreme conditions.

**Liquidation as price discovery:** Liquidation bonds (discounted collateral) create a secondary arbitrage market. Efficient liquidation markets maintain stability; inefficient ones accumulate bad debt.

**Composition as leverage:** Building strategies across protocols amplifies returns but compounds smart contract risk. Total value locked (TVL) grows; so does systemic fragility.

**Governance as tail risk:** Decentralization introduces multi-step governance approval delays. In fast-moving crises, governance cannot respond; emergency powers (multisigs) introduce centralization.