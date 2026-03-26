# Smart Contracts: Ethereum, Solana, and Cross-Chain Considerations

## Overview

Smart contracts are programs stored on a blockchain that execute automatically when conditions are met, extending blockchains from pure ledgers to general-purpose computation platforms. Different chains expose different execution models, languages, and security tradeoffs.

## Ethereum Virtual Machine (EVM) and Solidity

Ethereum is the dominant smart contract platform. The EVM is a stack-based virtual machine that executes bytecode. Contracts are written in Solidity, a higher-level language that compiles to EVM bytecode.

### Gas and Computational Economics

Every EVM operation costs "gas," an abstract unit of computation. Transactions pay gas fees (in ETH) that go to miners/validators. This creates an economic disincentive for infinite loops and denial-of-service attacks: running expensive code becomes prohibitively expensive.

Gas costs vary by operation: simple arithmetic; storage reads and writes are expensive (thousands of gas). Developers optimize by minimizing storage operations and using efficient patterns. The block gas limit (currently ~30 million gas per block on Ethereum) prevents any single transaction from consuming unbounded resources.

Gas estimation is difficult: static analysis cannot determine runtime gas consumption if contract behavior is data-dependent. Developers must test extensively or use gas profilers to identify expensive code paths.

## Common Vulnerabilities and Patterns

### Reentrancy

Reentrancy is the classic EVM vulnerability. When a contract calls an external function, control passes to that external contract. If the external contract calls back into the original contract before the original completes, the contract's state may be inconsistent.

Example: A withdrawal function transfers funds via `call()`, which forwards all available gas. The receiving contract calls `withdraw()` again before the first withdrawal completes, draining the contract.

**Prevention:** Use the Checks-Effects-Interactions pattern:
1. Check all preconditions (input validity)
2. Update internal state (effects)
3. Make external calls (interactions)

This ensures state is finalized before control passes to external code. Alternatively, use reentrancy guards (mutexes) or reduce contract to read-only after fund transfer.

### Integer Overflow and Underflow

Early Solidity versions didn't check arithmetic overflow. Adding 1 to `uint256` max would wrap to 0. Solidity 0.8+ added checked arithmetic by default, but unchecked blocks can disable checks for backwards compatibility or optimization.

### Access Control and Authorization

Use of `tx.origin` for authorization is a common bug. `tx.origin` is the original sender of the transaction; `msg.sender` is the immediate caller. A contract can use `tx.origin` as authorization when it shouldn't, allowing a phishing contract to forward calls and pass the `tx.origin` check.

### Gas Limit Loops

Unbounded loops over storage-backed collections can exceed the block gas limit, causing transactions to fail or halting the contract. Loops over user-supplied arrays are particularly dangerous. Pagination patterns (processing k items per transaction) mitigate this risk.

## Solidity Design Patterns

### Proxy Contracts

Proxy contracts separate logic from storage: the proxy delegates all calls to an implementation contract but stores data in its own storage. This allows upgrading the implementation contract without migrating state.

The Diamond pattern extends proxies: a single contract routes calls to multiple implementation contracts (facets), allowing very large contracts to be decomposed.

Danger: Proxies introduce storage layout hazards. If the implementation contract adds a state variable without accounting for the proxy's storage, the new variable overwrites the proxy's critical state.

### Factory Contracts

Factory contracts programmatically deploy new contracts. Instead of manually deploying each contract, a factory creates clones or full deployments. This pattern is used for token sales, market contracts, and governance.

### Token Standards

The ERC-20 standard defines a simple interface (transfer, approve, transferFrom) enabling interoperable tokens. Extensions like ERC-721 (non-fungible tokens) and ERC-1155 (multi-token) handle specialized use cases.

## Solana: An Alternative Model

Solana uses a fundamentally different execution model: accounts and programs.

### Accounts Model

Every piece of data on Solana is an account. Accounts hold data and can execute code. Unlike Ethereum's contract-storage model, Solana separates code (stateless programs) from data (accounts).

A transaction explicitly lists all accounts it will read or modify. The runtime parallelizes transactions that touch disjoint account sets. This enables high throughput (up to 65,000 TPS theoretically) by avoiding global state serialization.

### Rust and Program Structure

Solana programs are written in Rust and compiled to BPF (bytecode format portable across systems). Programs cannot directly modify account data; they receive account references and modify them via syscalls.

The execution model is deterministic: given the same account state and program inputs, execution always produces the same result. This avoids many of Ethereum's non-determinism pitfalls.

### Security Implications

Solana's account model makes some Ethereum vulnerabilities impossible (e.g., unexpected state changes from distant calls) but introduces others. Program bugs can corrupt arbitrary accounts passed to the program, and account ownership confusion can lead to fund theft.

## Other Languages and Platforms

**Move (Aptos, Sui):** A language designed for resource-oriented programming. Assets are first-class types that cannot be duplicated or accidentally lost; the compiler enforces resource semantics statically.

**Aiken (Cardano):** Functional language targeting an account-based model but with explicit data access patterns.

## Cross-Chain Bridges and Interoperability

Bridges enable asset and data movement across blockchains. Two models:

**Lock-and-mint bridges:** Users lock assets on chain A; the bridge mints equivalent assets on chain B. Redemption burns chain B assets and unlocks chain A assets.

**Validator-signed bridges:** A set of validators observe chain A, collectively sign a state proof, and validators on chain B verify and execute. Chainlink CCIP uses this model with protocol-level risk management.

Bridge security is critical: a compromised bridge can mint unlimited tokens on the destination chain or lock user funds indefinitely. Bridge exploits have resulted in billion-dollar losses.

## Oracles and External Data

Smart contracts cannot directly fetch external data (HTTP requests). Oracles solve this by posting data on-chain.

**Chainlink:** Decentralized oracle networks where independent node operators fetch data from multiple sources, sign it, and post it on-chain. Contracts reference the signed data. Node operators are incentivized by fees and slashed if data is fraudulently reported.

Chainlink's Price Feeds provide price data for common assets; the network ensures consensus across multiple sources and detects flash crashes or outlier values.

**Centralized oracles** (trusted nodes operated by dApps) are faster and cheaper but reintroduce trusted intermediaries, weakening decentralization.

Oracle manipulation is a common attack vector: if a contract relies on a single oracle source (especially a DEX price), attackers can artificially move that price via flash loans or low-liquidity trades and extract value from the contract.

## Formal Verification

Formal verification proves contract correctness against a formal specification. Tools like Certora (Rule-based) and Coq (proof assistant) can verify entire programs or critical functions.

Verification is expensive and requires deep expertise but can provide confidence for contracts managing large amounts of capital. Most projects verify only core financial logic, not the entire codebase.

## See Also

- [blockchain-consensus-mechanisms.md](blockchain-consensus-mechanisms.md) — Underlying consensus models
- [blockchain-distributed-ledger.md](blockchain-distributed-ledger.md) — Broader blockchain architecture
- [formal-verification.md](formal-verification.md) — General formal verification concepts
- [security-zero-trust.md](security-zero-trust.md) — Defense in depth principles applying to contracts