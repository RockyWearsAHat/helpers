# DeFi Protocols — Automated Market Makers, Lending, Stablecoins & MEV

## Overview

Decentralized Finance (DeFi) replicates traditional financial services (trading, lending, derivatives) as open-source smart contracts. Unlike centralized exchanges or banks, DeFi protocols are stateless: no company custody, no permission to transact, no ability to freeze funds. Transactions are atomic — either fully execute or fully revert.

Core innovation: **Automated mechanisms replace order books and intermediaries.** Traders, lenders, and liquidity providers interact with algorithms instead of counterparties. This transparency and composability ("money Legos") enable complex strategies, but also create new attack surfaces: sandwich attacks, liquidations, flash loans.

## Automated Market Makers (AMMs)

An AMM is a smart contract pool holding two (or more) assets. Instead of an order book matching buyers and sellers, a mathematical formula determines price. Anyone can provide liquidity (deposit both assets) and earn fees on trades.

### Constant Product Formula (Uniswap v2)

The most commonly deployed model:

$$x \cdot y = k$$

where $x$ and $y$ are reserve amounts and $k$ is an invariant (constant). When a trader buys asset $y$ with asset $x$:

- Trader sends amount $\Delta x$ (input)
- Pool balance $x$ increases by $\Delta x$
- New product: $(x + \Delta x) \cdot y' = k$
- Solve for $y'$: trader receives $\Delta y = y - k/(x + \Delta x)$

**Example:** Pool has 100 ETH, 200K USDC (k = 20M). Trader sends 1 ETH.
- New balance: 101 ETH
- New USDC: k/101 ≈ 198,019 USDC
- Trader receives: 200K - 198,019 = 1,981 USDC

The formula ensures slippage: larger trades have worse prices. This incentivizes splitting large trades and prevents sandwiching (partially; see MEV section).

**Fees:** Uniswap v2 takes 0.3% cut (variable in v3/v4), paid to liquidity providers proportionally to their stake. On 1,981 USDC, ~5.94 USDC goes to LPs, trader gets ~1,975.

**LP Returns:** LPs earn fees but suffer **impermanent loss** — if asset prices diverge significantly, LPs end up with fewer value than if they held the assets. Example: if ETH doubles while USDC stagnates, LPs who provided 100 ETH + 200K USDC are now holding ~141 ETH + 141K USDC (less ETH than they'd have if held, less USDC gain potential).

### Concentrated Liquidity (Uniswap v3)

Instead of passive position across all prices, LPs specify a price **range** (e.g., 1,500-2,500 per ETH) and concentrate capital there. Benefits:

- Higher fees per dollar deployed (concentrated in active range)
- LPs suffer impermanent loss only within range (outside range, exposure limited)

Downside: Active management required. If market moves outside specified range, position stops earning fees and maximal impermanent loss materializes.

### Advanced AMM Curves

**Stableswap (Curve):** Designed for low-slippage swaps between stablecoins. Uses curve shifting formula:

$$\chi + y = k_{EQ} (\frac{x}{A} + \frac{y}{A}) + D$$

Flat near equilibrium (low slippage for stablecoin pairs), steep at extremes (high slippage for arbitrage). $A$ is amplification factor.

**Weighted Pools (Balancer):** Support arbitrary weightings (e.g., 80% DAI, 20% ETH). Generalization of constant product.

**Hybrid Curves (Curve v2, Uniswap v4):** Adapt curve shape dynamically based on market conditions for lower slippage across volatility regimes.

## Lending Protocols

Loans without counterparties: users deposit collateral, borrow against it. Interest accrues; borrowers pay, depositors earn.

### Aave / Compound Model

- **Pools:** Each token (DAI, USDC, ETH) has its own pool with depositors and borrowers
- **Utilization Curve:** Interest rate varies with utilization $U = \text{borrowed} / \text{total}$
  - Low utilization: low rates (cheap borrowing, low yield for depositors)
  - High utilization: high rates (expensive borrowing, attractive yield for depositors)
- **Collateral and LTV:** Borrow up to a collateral ratio (e.g., 80% LTV on ETH — borrow $0.80 value per $1 collateral)
- **Liquidation:** If collateral value drops below borrow amount / LTV, liquidators can repay debt and seize collateral (plus liquidation bonus, e.g., 5-10%). Incentivizes rapid deleveraging.

Example:
- Deposit 10 ETH (assume $2,000 each, $20,000 value)
- With 80% LTV, borrow $16,000 DAI
- Assume 5% DAI interest rate: pay $800/year
- If ETH drops to $1,600, collateral = $16,000 = debt → liquidation threshold reached.

### Utilization Curve Mechanics

```
Interest Rate vs. Utilization:
  Low U (0-80%): rate = base_rate + slope1 * U
  High U (>80%): rate = kink_rate + slope2 * (U - kink)
```

"Kink" creates distinct regimes: borrowing cheap until pool becomes stressed, then expensive. Stabilizes pool at target utilization.

## Stablecoins

Cryptocurrencies pegged to fiat (USD, EUR) or commodities. Essential for DeFi because most trades bottom out in stable unit of account.

### Collateralized (Backed)

- **USD Coin (USDC):** 1-to-1 backing: $1 USDC ↔ $1 held in bank
- **DAI:** Over-collateralized on-chain (e.g., need $1.50 collateral to mint $1 DAI)
- **Collateral diversity:** DAI can be minted against ETH, USDC, stETH, etc.; diversifies collapse risk

### Algorithmic (Un-backed)

- **Terra/Luna model:** Terra USD supply maintained by Luna token burns/mints. If Luna > threshold, mint UST; if Luna < threshold, burn UST
- **Failure mode:** Cascade failure where Luna token loses confidence → can't stabilize UST → death spiral (Luna hyperinflates, UST de-pegs)
- Most pure algorithmic stablecoins have collapsed (Basis Cash, empty set dollar, etc.)

### Hybrid (Partly Collateralized)

- Fractional reserve stablecoins (e.g., 50% collateral + 50% algorithmic)
- Intermediate risk profile; rare in practice

**Current dominant:** Tether (USDT, centralized, real fiat backing) and USDC (Circle, semi-decentralized backing). DAI (decentralized, on-chain collateral) growing but smaller TVL.

## Flash Loans

Uncollateralized loans that must be repaid within the same transaction. Enabled by atomicity: if repayment fails, entire transaction reverts, so lender has no risk.

Use cases (legitimate):
- **Arbitrage:** Borrow 1M DAI, execute profitable trade, repay + take spread
- **Liquidations:** Borrow collateral, liquidate, repay

Abuse:
- **Flash loan attacks:** Borrow massive amount, manipulate price oracles, enter position at distorted price, repay. Example: bZx attack 2020 (stole ~$350K via oracle manipulation)

Mitigation: DeFi protocols use **TWAP oracles** (time-weighted average price over blocks) instead of spot price, immune to single-block manipulation.

## MEV and Sandwich Attacks

**Maximal Extractable Value (MEV):** Profit miners/validators extract by reordering, censoring, or selectively including transactions.

### Sandwich Attack

User broadcasts a trade (e.g., 100 ETH → DAI). Before transaction is included:
1. Attacker frontruns: sends 50 ETH → DAI (same pool, same direction); pushes price higher
2. User transaction executes at worse price (slippage materializes)
3. Attacker backruns: sells DAI back to ETH, pockets the spread

Slippage tolerance setting (e.g., "accept ≥1,950 DAI for 1 ETH") mitigates to a degree but is guesswork.

### Other MEV Vectors

- **Liquidation ordering:** Validator includes liquidation that benefits their own account
- **Batching:** Validator groups transactions to maximize their extraction
- **Censoring:** Omit user transaction to lower competition for extraction

**Mitigation approaches:**
- Private mempools (MEV-Shield, MEV-Blocker) — transactions hidden until included
- MEV-Burn proposals — Protocol captures MEV instead of validators, reducing incentive
- Encrypted mempools (threshold encryption) — transactions encrypted until included

## Yield Farming and Protocol Incentives

New pools attract liquidity via token incentives. Protocol distributes governance tokens to LPs as reward (e.g., Sushi distributes SUSHI to LPs). Creates positive feedback: high rewards → capital inflow → low slippage → more trading → more fees. When rewards end, liquidity often evaporates.

Strategy risk: "yield farmer" hops between highest-APY pools, creates volatility in TVL.

## See Also

- [blockchain-smart-contracts.md](blockchain-smart-contracts.md) — smart contract patterns in DeFi
- [web3-token-standards.md](web3-token-standards.md) — ERC-20 and token transfers
- [web3-dao.md](web3-dao.md) — governance mechanisms in protocols
- [privacy-zero-knowledge.md](privacy-zero-knowledge.md) — privacy-preserving DeFi