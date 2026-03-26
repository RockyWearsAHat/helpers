# Solidity — Smart Contract Programming Language

## Overview

Solidity is a statically-typed language designed specifically for the Ethereum Virtual Machine (EVM). Contracts are programs that control assets (cryptocurrency, tokens, NFTs) and execute rules enforced by blockchain consensus. Unlike traditional applications, Solidity programs run in a permissionless environment where execution is publicly verifiable and irreversible.

## Contract Structure

Every Solidity file contains contract definitions:

```solidity
pragma solidity ^0.8.0;

contract Bank {
    mapping(address => uint256) balances;
    
    function deposit() public payable {
        balances[msg.sender] += msg.value;
    }
    
    function withdraw(uint256 amount) public {
        require(balances[msg.sender] >= amount);
        balances[msg.sender] -= amount;
        payable(msg.sender).transfer(amount);
    }
}
```

**Key elements**:

- **pragma**: specifies compiler version constraint
- **contract**: defines a deployable entity (class-like)
- **state variables**: persistent storage (survives transactions)
- **functions**: entry points for execution
- **visibility**: `public` (external + internal), `internal`, `private`, `external`

Contracts are instantiated at an address on the blockchain. Calling functions costs gas (ETH).

## Storage Layout

### Storage Slots

State variables are stored in **storage slots** (256-bit slots, indexed 0, 1, 2, ...):

```solidity
contract Storage {
    uint256 x;              // slot 0 (256 bits)
    uint128 y;              // slot 1 (first 128 bits)
    uint128 z;              // slot 1 (last 128 bits) — packed with y
    address owner;          // slot 2 (160 bits) — owner's address
}
```

**Packing**: smaller types fit into a single slot. A `uint8` takes 1 byte, but reserves a full 256-bit slot unless paired with other small types:

```solidity
uint256 a;      // uses slot 0 (all 256 bits, 32 bytes)
uint8 b;        // uses slot 1 (1 byte, but reserves 32 bytes) — INEFFICIENT
uint8 c;        // uses slot 1 (packed with b) — shares the 32-byte slot
```

**Slot assignment order**: variables are assigned slots in declaration order. Reordering state variables changes storage layout, breaking upgradeable contracts.

### Mappings

Mappings are hash tables with keys hashing to slots:

```solidity
mapping(address => uint256) balances;
```

For a mapping at slot `S` and key `k`, the value is stored at slot `keccak256(k || S)`. This avoids sparse arrays — only used keys occupy storage.

```solidity
mapping(address => mapping(address => uint256)) allowances;
// For key1=alice, key2=bob, slot=2:
// allowances[alice][bob] stored at keccak256(bob || keccak256(alice || 2))
```

### Dynamic Arrays

Arrays are stored with length at slot `S` and elements starting at `keccak256(S)`:

```solidity
uint256[] values;  // slot 3
// length at slot 3
// values[0] at slot keccak256(3)
// values[1] at slot keccak256(3) + 1
```

Resizing arrays can be expensive — adding elements appends and costs storage.

## Gas Optimization

### Storage vs. Memory

Storage is expensive; memory is cheap:

- **Storage operation**: 5,000-20,000 gas (depends on initial vs. updated state)
- **Memory operation**: 3 gas per operation

Minimize storage writes:

```solidity
// EXPENSIVE
function sum(uint256[] calldata nums) public {
    for (uint256 i = 0; i < nums.length; i++) {
        total += nums[i];  // write to storage
    }
}

// OPTIMIZED
function sum(uint256[] calldata nums) public returns (uint256) {
    uint256 result = 0;  // memory
    for (uint256 i = 0; i < nums.length; i++) {
        result += nums[i];
    }
    return result;
}
```

### Packed Structs

Group small types to reduce slots:

```solidity
// 3 slots
struct User {
    uint256 id;
    uint8 role;
    address wallet;
}

// 2 slots
struct UserOptimized {
    address wallet;     // 160 bits
    uint8 role;         // 8 bits — packed with wallet
    uint88 padding;     // fills remaining 88 bits
    uint256 id;         // slot 2
}
```

Declare fields in order of size (largest first) to minimize wasted space.

### Storage Reading

Only read storage once:

```solidity
// INEFFICIENT
for (uint256 i = 0; i < owners.length; i++) {
    if (owners[i] == caller) { ... }
}

// OPTIMIZED
address[] memory cachedOwners = owners;
for (uint256 i = 0; i < cachedOwners.length; i++) {
    if (cachedOwners[i] == caller) { ... }
}
```

Copying to memory costs gas upfront but saves on repeated storage reads.

## Inheritance (C3 Linearization)

Solidity supports multiple inheritance with C3 linearization (Python's MRO):

```solidity
contract A { }
contract B is A { }
contract C is A { }
contract D is B, C { }

// Linearization: D, B, C, A
// Resolution order: functions resolved left-to-right
```

**Call order**: `super` calls the next contract in the linearization:

```solidity
contract A {
    function test() public virtual { }
}

contract B is A {
    function test() public override {
        super.test();  // calls A.test()
    }
}
```

Diamond inheritance can cause unexpected behavior:

```solidity
contract A { uint256 x; }
contract B is A { }
contract C is A { }
contract D is B, C { }
// x is stored once (not duplicated in B and C)
```

Solidity linearization prevents duplicate storage but can confuse developers expecting Java-like semantics.

## Events and Logs

Events are emitted to create tamper-proof logs:

```solidity
event Transfer(address indexed from, address indexed to, uint256 value);

function transfer(address to, uint256 amount) public {
    balances[msg.sender] -= amount;
    balances[to] += amount;
    emit Transfer(msg.sender, to, amount);
}
```

**Indexed parameters** create searchable topics (up to 3 per event). Non-indexed parameters are stored in the log data.

**Why logs?** Logs are not part of contract state but are permanently recorded in the blockchain. Dapps listen for events, enabling reactive interfaces. Events are cheaper than storage (8 gas vs. 20,000+ gas).

## Modifiers and Decorators

Modifiers extend function behavior:

```solidity
modifier onlyOwner() {
    require(msg.sender == owner, "not owner");
    _;  // placeholder for function body
}

function withdraw() public onlyOwner {
    // function body
}
```

Expands to:

```solidity
function withdraw() public {
    require(msg.sender == owner, "not owner");
    // function body
}
```

**Multiple modifiers**:

```solidity
function process() public onlyOwner nonReentrant {
    // both checks run before body
}
```

## Security Patterns

### Checks-Effects-Interactions

Order operations to prevent reentrancy:

```solidity
// VULNERABLE
function withdraw(uint256 amount) public {
    uint256 balance = balances[msg.sender];
    require(balance >= amount);
    
    // Interaction (attacker can call back here)
    (bool success, ) = msg.sender.call{value: amount}("");
    require(success);
    
    // Effects (after call, balances still unchanged)
    balances[msg.sender] -= amount;
}

// SAFE
function withdraw(uint256 amount) public {
    uint256 balance = balances[msg.sender];
    require(balance >= amount);
    
    // Effects (update state first)
    balances[msg.sender] -= amount;
    
    // Interaction (after state change)
    (bool success, ) = msg.sender.call{value: amount}("");
    require(success);
}
```

### Reentrancy Guard

Use a flag to prevent recursive calls:

```solidity
bool locked;

modifier nonReentrant() {
    require(!locked, "no reentrancy");
    locked = true;
    _;
    locked = false;
}

function withdraw() public nonReentrant {
    // safely withdraw
}
```

### Pull Over Push

Avoid calling external functions on unknown recipients:

```solidity
// RISKY: push pattern
function distribute() public {
    for (address user in users) {
        user.transfer(amount);  // can fail, blocking distribution
    }
}

// SAFE: pull pattern
function withdraw() public {
    payable(msg.sender).transfer(balances[msg.sender]);
}
```

Users pull their funds rather than the contract pushing. Failures don't affect others.

## Advanced Types

### Structs

Named data grouping:

```solidity
struct Order {
    address buyer;
    uint256 amount;
    uint256 timestamp;
}

Order order = Order(msg.sender, 100, block.timestamp);
```

### Enums

Compile-time constants:

```solidity
enum Status { Pending, Active, Closed }

Status status = Status.Pending;
```

### Fixed-Point Numbers

Represents decimals (limited support):

```solidity
fixed128x18 price = 1.5;  // 1.5 in 128-bit fixed-point
```

Limited adoption due to complexity; most use USDC (18-decimal integer representation).

## Compilation and Deployment

Solidity compiles to EVM bytecode:

```bash
solc contract.sol --bin    # outputs bytecode
```

Deployment costs gas (constructor execution + bytecode storage). Once deployed, the contract is immutable at that address (unless it uses proxy patterns for upgrades).

## see also

- [blockchain-smart-contracts.md](blockchain-smart-contracts.md) — EVM, Solana, cross-chain comparison
- [architecture-patterns.md](architecture-patterns.md) — proxy patterns, upgrade patterns
- [formal-verification.md](formal-verification.md) — verifying contract correctness