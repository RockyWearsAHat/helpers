# Ethereum Virtual Machine (EVM) — Stack Machine, Opcodes, Gas, and Storage

## Overview

The Ethereum Virtual Machine is a stack-based computation engine that executes smart contract bytecode deterministically across all network nodes. It's the execution layer that makes Ethereum a "world computer" — not just a ledger, but a state machine. The EVM transforms the blockchain from a data structure into a computational platform.

Core insight: **The EVM is a distributed state transition function.** Given a state (all accounts, balances, code) and a set of transactions, it produces a deterministic new state. This guarantees that all nodes, running the same bytecode, arrive at identical results — critical for blockchain consensus.

## Stack-Based Architecture

The EVM is fundamentally a stack machine, not a register machine (unlike modern CPUs). Execution operates on a **1024-item depth stack** where each item is a 256-bit word (32 bytes). All computation flows through this stack:

```
PUSH1 5        # Stack: [5]
PUSH1 3        # Stack: [5, 3]
ADD            # Pop 3, 5; push 8 → Stack: [8]
```

**Why 256 bits?** Native alignment with cryptographic primitives: Keccak-256 hashes, secp256k1 signatures, and arithmetic operate on 256-bit values. This choice avoids conversion overhead.

The stack is ephemeral — cleared at the end of each transaction. Values that need persistence go to **storage**.

## Three Memory Models

### Memory (Volatile, Word-Addressed)

Linear byte array per transaction. Persists only during execution. Resizable dynamically. Gas cost grows quadratically with size, discouraging unbounded allocation. Used for function arguments, local variables, intermediate computation.

```
MSTORE   # Store word → Memory[offset] (32 bytes)
MLOAD    # Load word from Memory[offset]
```

### Storage (Persistent, Merkle Patricia Trie)

Per-account key-value store, embedded in the Ethereum state tree. Every write is cryptographically committed. Vastly more expensive than memory (20,000+ gas vs. 3 gas). Modified via `SSTORE`/`SLOAD`.

**Storage Layout Rules** (Solidity-level, relevant to gas optimization):
- State variables packed into 32-byte slots
- Multiple small types share one slot if they fit (e.g., uint8 + uint8 = one slot)
- Dynamic arrays and mappings reserve a slot, with actual data stored at derived addresses
- Packing order matters; poor organization wastes slots

### Transient Storage (Introduced Shanghai)

TSTORE/TLOAD opcodes provide key-value access that persists across internal calls within a single transaction but is discarded after. Gas-efficient temporary state sharing (100 gas vs. 20,000). Enables patterns like reentrancy guards without storage overhead.

## Opcodes and Execution Model

The EVM instruction set includes ~140 opcodes. Categories:

- **Stack operations**: PUSH (1-32), POP, DUP (1-16), SWAP (1-16)
- **Arithmetic**: ADD, SUB, MUL, DIV, SDIV, MOD, SMOD, ADDMOD, MULMOD, EXP, SIGNEXTEND
- **Comparison**: LT, GT, SLT, SGT, EQ, ISZERO
- **Bitwise**: AND, OR, XOR, NOT, SHL, SHR, SAR
- **Environmental**: ADDRESS (contract address), BALANCE (account balance), ORIGIN, CALLER, CALLDATASIZE, CALLDATALOAD, CODESIZE
- **Blockchain**: BLOCK­HASH, COINBASE, TIMESTAMP, NUMBER, DIFFICULTY, GASLIMIT, CHAINID
- **Storage**: SSTORE, SLOAD, TSTORE (Shanghai), TLOAD (Shanghai)
- **Memory**: MSTORE, MLOAD, MSTORE8
- **Control flow**: JUMP, JUMPI, JUMPDEST, REVERT, STOP, SELFDESTRUCT

Each opcode has a fixed gas cost. Some operations (like EXP, KECCAK256) have variable costs proportional to operand size. Execution halts on invalid jumps, stack underflow, or explicit REVERT.

## Gas Metering and Resource Allocation

Gas is the abstraction layer for computational cost. Every opcode consumes gas; transactions specify a gas limit. If gas exhausted before completion, execution reverts and the sender pays for the gas burned.

**Gas shapes incentives:**
1. **Denial-of-service prevention** — infinite loops cost unbounded gas
2. **Fair pricing** — expensive operations (storage writes, hashing) cost more
3. **Economic finality** — miners prioritize high-gas-price transactions

Total transaction fee = gas used × gas price (wei/gas). After London hardfork (2021), part of gas revenue is burned (EIP-1559), reducing supply.

**Storage gas overhaul (Berlin, 2021):** Cold storage access (first access in a transaction) costs 2,100 gas; warm (subsequent) costs 100 gas. Discourages single massive state read/write.

## Call Types and Contract Interaction

Contracts invoke other contracts via different call mechanisms, each with semantic differences:

### CALL
Standard contract interaction. Sender context changes: the callee sees `msg.sender = caller` and `msg.value = call value` (ether transfer). Storage context changes (callee can modify its own storage, not caller's). Used for most interactions.

### DELEGATECALL (EIP-7)
Executes callee code **in the context of the caller's storage**. Caller's `msg.sender`, `msg.value` unchanged. Callee can modify caller's storage. Enables proxy patterns: a proxy contract calls an implementation's code, which modifies the proxy's storage. Essential for upgradeable contracts.

**Risk:** Delegatecalled code can manipulate proxy storage, so implementation code must be trusted.

### STATICCALL (EIP-214, Byzantium)
Read-only call variant. Callee cannot modify storage or call state-changing operations. Reverts if attempted. Useful for view/pure function enforcement.

### CREATE and CREATE2
Contract creation mechanisms:

- **CREATE:** New contract address derived from sender nonce. Non-deterministic — address depends on transaction order. If you revert and retry, you get a different address.
- **CREATE2 (EIP-1014, Constantinople):** Deterministic contract address = `keccak256(sender, salt, bytecode)`. Enables counterfactual contract prediction. Critical for layer 2 and protocol design.

```
address = keccak256(0xff, sender, salt, keccak256(bytecode))
```

## Precompiles

Built-in contracts at fixed addresses (0x01-0x0c) that execute native code for expensive cryptographic operations:

- `0x01` — ECRECOVER (recover public key from signature)
- `0x02` — SHA2-256
- `0x03` — RIPEMD-160
- `0x04` — IDENTITY (copy data)
- `0x05` — MODEXP (modular exponentiation)
- `0x06-0x08` — Elliptic curve additions/pairings (alt_bn128, for zero-knowledge proofs)
- `0x09` — BLAKE2b-256 (Istanbul)
- `0x0a` — BLS12-381 multipoint validation (Dencun)
- `0x0b` — BLS12-381 map to curve (Dencun)

Precompiles cost much less than implementing the same logic in bytecode, but cost enough to prevent abuse.

## EVM Evolution

The EVM has undergone major upgrades without breaking existing contracts:

### Shanghai (2023): Transient Storage
- TSTORE/TLOAD opcodes for gas-efficient per-transaction key-value access
- Enables reentrancy guards and cross-call communication without storage overhead

### Cancun (2024): Blob Data and EOF
- EIP-4844 introduces blobs — cheap data availability for layer 2 rollups
- Blobs exist for 18 days then are pruned, reducing storage burden
- EOF (EVM Object Format) preparation for more structured bytecode

### Pectra (2025): Improvements Pending
- Account abstraction features
- Further gas efficiency improvements
- More ALT_BN128 operations for zero-knowledge integration

## Formal Verification and Semantics

The EVM is formally specified in the **Yellow Paper** (Gavin Wood, technical specification) and implemented in multiple languages (go-ethereum, erigon, reth, etc.). Verification tools exist (Coq formalization, K semantics) for proving bytecode correctness. However, most dApps rely on static analysis tools and test suites rather than formal proofs.

## Key Constraints and Implications

- **Stack depth limit (1024)** — nesting calls/loops can exhaust stack
- **Calldata size** — arguments to a function cost 4 gas/byte if zero, 16 if non-zero (EIP-2028)
- **Code size limit (24KB)** — deployed contract bytecode capped; large contracts must use libraries/proxies
- **Gas limit per block** — ~30 million gas, throttles throughput (Ethereum mainnet: ~10-15 tx/sec)
- **Deterministic execution** — same input, same output on all nodes; no randomness, no floating-point

These constraints drive most design patterns: storage optimization, library dependencies, upgradeability patterns.

## See Also

- [blockchain-smart-contracts.md](blockchain-smart-contracts.md) — contract languages and patterns
- [language-solidity.md](language-solidity.md) — Solidity compiler targeting EVM
- [web3-layer2.md](web3-layer2.md) — layer 2 solutions moving computation off EVM
- [formal-verification.md](formal-verification.md) — proving bytecode correctness