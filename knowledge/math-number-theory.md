# Number Theory for Software Engineers — Primes, Modular Arithmetic & Applications

## Why Number Theory Matters in Computing

Number theory — the study of integers and their properties — underpins cryptography, hashing, error detection, load distribution, and random number generation. Many algorithms that appear purely practical (hash tables, checksums, key exchange) rest on number-theoretic foundations. Understanding these foundations helps diagnose subtle bugs in distribution, hashing collisions, and cryptographic implementations.

## Prime Numbers — Building Blocks of Integers

A prime is an integer greater than 1 divisible only by 1 and itself. Primes serve as the "atoms" of the integers.

### The Fundamental Theorem of Arithmetic

Every integer greater than 1 has a unique factorization into primes (up to ordering). This uniqueness guarantee is what makes primes so useful:

- 60 = 2² × 3 × 5 — no other prime decomposition exists
- GCD and LCM computations reduce to comparing prime factorizations
- Many number-theoretic functions are defined in terms of prime factorizations

### Distribution of Primes

| Property                     | Description                                                                |
| ---------------------------- | -------------------------------------------------------------------------- |
| Prime counting function π(n) | Approximately n / ln(n) for large n                                        |
| Prime gaps                   | Irregular but grow slowly on average                                       |
| Twin primes                  | Pairs like (11, 13) — conjectured to be infinite                           |
| Density                      | Primes thin out logarithmically — about 1 in 23 numbers near 10⁸ are prime |

The irregularity of prime distribution is both a mathematical curiosity and a practical resource — it contributes to the unpredictability that cryptographic systems rely on.

## Primality Testing

Determining whether a large number is prime is a distinct problem from factoring it. Several approaches exist with different trade-off profiles.

### Deterministic Approaches

- **Trial division**: Test divisibility by all integers up to √n. Simple but O(√n) — impractical for large numbers.
- **AKS primality test**: The first proven polynomial-time deterministic test (2002). Theoretically important but slower in practice than probabilistic methods for typical input sizes.

### Probabilistic Approaches

- **Fermat test**: If $a^{n-1} \not\equiv 1 \pmod{n}$, then n is composite. Fast but fooled by Carmichael numbers.
- **Miller-Rabin test**: Refines Fermat by examining square roots of 1 mod n. With k rounds, the probability of a false positive is at most $4^{-k}$. Widely used in practice.
- **Solovay-Strassen test**: Based on the Jacobi symbol. Similar error bounds to Miller-Rabin but less commonly deployed.

| Test                    | Type          | Complexity    | False positives    |
| ----------------------- | ------------- | ------------- | ------------------ |
| Trial division          | Deterministic | O(√n)         | None               |
| AKS                     | Deterministic | O(log⁶ n)     | None               |
| Miller-Rabin (k rounds) | Probabilistic | O(k · log² n) | ≤ 4⁻ᵏ              |
| Fermat                  | Probabilistic | O(log² n)     | Carmichael numbers |

For cryptographic key generation, Miller-Rabin with sufficient rounds provides practical certainty.

### The Sieve of Eratosthenes

For generating all primes up to a bound N, the sieve marks multiples iteratively:

1. List integers from 2 to N
2. Starting from 2, mark all multiples of 2 (except 2 itself)
3. Move to the next unmarked number, mark its multiples
4. Continue until reaching √N — remaining unmarked numbers are prime

Time complexity: O(N log log N). Space: O(N). Segmented variants reduce memory for large ranges.

## Modular Arithmetic

Modular arithmetic restricts integers to a fixed range, wrapping around at a modulus m. The analogy of clock arithmetic captures the core idea — 14:00 and 2:00 are "the same" modulo 12.

### Congruence Relations

Two integers a and b are congruent modulo m (written $a \equiv b \pmod{m}$) if m divides (a − b).

Key properties:

- **Reflexive**: $a \equiv a \pmod{m}$
- **Symmetric**: $a \equiv b \implies b \equiv a$
- **Transitive**: $a \equiv b$ and $b \equiv c \implies a \equiv c$
- **Compatible with arithmetic**: Addition, subtraction, and multiplication preserve congruence
- **Division caveat**: Division is only valid when the divisor is coprime to m

### Residue Classes

The integers modulo m partition into m equivalence classes: {0, 1, 2, ..., m−1}. These classes form a ring under addition and multiplication mod m. When m is prime, every nonzero element has a multiplicative inverse — forming a field.

## The Euclidean Algorithm and GCD

The greatest common divisor (GCD) of two integers is efficiently computed via repeated division:

```
gcd(a, b):
    while b ≠ 0:
        a, b = b, a mod b
    return a
```

Time complexity: O(log(min(a, b))) — the number of steps is bounded by the Fibonacci sequence.

### The Extended Euclidean Algorithm

Extends GCD to find coefficients x, y such that $ax + by = \gcd(a, b)$. This has direct applications:

- **Modular inverse**: If gcd(a, m) = 1, then x from $ax + my = 1$ gives $a^{-1} \equiv x \pmod{m}$
- **Solving linear Diophantine equations**: $ax + by = c$ has solutions iff gcd(a, b) divides c
- **Cryptographic key computation**: RSA private keys are computed via extended GCD

## Euler's Totient Function

$\phi(n)$ counts integers from 1 to n that are coprime to n.

| n   | φ(n) | Coprime integers   |
| --- | ---- | ------------------ |
| 1   | 1    | {1}                |
| 6   | 2    | {1, 5}             |
| 7   | 6    | {1, 2, 3, 4, 5, 6} |
| 12  | 4    | {1, 5, 7, 11}      |

Computation from prime factorization: $\phi(n) = n \prod_{p \mid n} \left(1 - \frac{1}{p}\right)$

For a prime p: $\phi(p) = p - 1$. For a prime power: $\phi(p^k) = p^k - p^{k-1}$.

The totient function appears in:

- Euler's theorem: $a^{\phi(n)} \equiv 1 \pmod{n}$ when gcd(a, n) = 1
- RSA key generation: The private exponent satisfies $ed \equiv 1 \pmod{\phi(n)}$ (or more precisely, $\lambda(n)$)
- Counting generators of cyclic groups

## Fermat's Little Theorem

For prime p and integer a not divisible by p:

$$a^{p-1} \equiv 1 \pmod{p}$$

Equivalently: $a^p \equiv a \pmod{p}$ for all integers a.

Applications include:

- **Primality testing**: The Fermat test checks this condition — failure proves compositeness
- **Efficient modular inverse**: $a^{-1} \equiv a^{p-2} \pmod{p}$
- **Simplifying exponentiation**: Reduce exponents modulo (p − 1) before computing

## Modular Exponentiation

Computing $a^b \mod m$ efficiently is critical for cryptography. Naive computation (multiply b times) is impractical for large exponents. The square-and-multiply method:

```
mod_exp(base, exp, mod):
    result = 1
    base = base mod mod
    while exp > 0:
        if exp is odd:
            result = (result × base) mod mod
        exp = exp >> 1
        base = (base × base) mod mod
    return result
```

Time complexity: O(log b) multiplications, each involving numbers up to m. This is what makes public-key cryptography computationally feasible — even with 2048-bit exponents.

## The Chinese Remainder Theorem (CRT)

If $m_1, m_2, \ldots, m_k$ are pairwise coprime, then the system of congruences:

$$x \equiv a_1 \pmod{m_1}, \quad x \equiv a_2 \pmod{m_2}, \quad \ldots, \quad x \equiv a_k \pmod{m_k}$$

has a unique solution modulo $M = m_1 m_2 \cdots m_k$.

### Applications

- **Parallel computation**: Split a large modular computation into smaller independent subproblems
- **RSA speedup**: Decryption via CRT is ~4× faster than direct computation
- **Representing large numbers**: Store a number via its residues modulo several small primes
- **Secret sharing**: Distribute fragments that individually reveal nothing

## The Discrete Logarithm Problem

Given g, h, and p, find x such that $g^x \equiv h \pmod{p}$.

While modular exponentiation is efficient, its inverse — the discrete logarithm — appears to be computationally hard for well-chosen parameters. This asymmetry is the foundation of several cryptographic constructions.

| Algorithm            | Complexity             | Notes                                     |
| -------------------- | ---------------------- | ----------------------------------------- |
| Brute force          | O(p)                   | Try all x                                 |
| Baby-step giant-step | O(√p) time and space   | Meet-in-the-middle approach               |
| Pollard's rho        | O(√p) time, O(1) space | Probabilistic cycle detection             |
| Index calculus       | Sub-exponential        | Most effective for prime fields           |
| Quantum (Shor's)     | Polynomial             | Requires fault-tolerant quantum computers |

The hardness of discrete logarithms underpins Diffie-Hellman key exchange, ElGamal encryption, and digital signature algorithms. Elliptic curve variants offer equivalent security with smaller key sizes.

## Hashing and Number-Theoretic Properties

Hash functions map arbitrary data to fixed-size integers. Number theory informs their design and analysis:

- **Modular hashing**: h(k) = k mod m. Choosing m as a prime distant from powers of 2 reduces clustering from patterned keys.
- **Multiplicative hashing**: h(k) = ⌊m · frac(k · A)⌋ where A ≈ (√5 − 1)/2 (the golden ratio). Distributes keys more uniformly than simple modular hashing for structured inputs.
- **Universal hashing**: Families where $h_{a,b}(k) = ((ak + b) \mod p) \mod m$ with random a, b provide provable collision bounds.
- **Polynomial hashing**: For strings, $h(s) = \sum s_i \cdot r^i \mod p$. The choice of prime p and base r affects collision probability.

### Why Prime Table Sizes Help

When a hash table uses size m and keys exhibit patterns (arithmetic progressions, common factors), a prime m ensures that different keys map to different slots more often. Non-prime sizes can create clustering when key values share factors with m.

## Random Number Generation

### Pseudo-Random Number Generators (PRNGs)

PRNGs are deterministic algorithms that produce sequences appearing random. Key properties:

| Property         | Description                                        |
| ---------------- | -------------------------------------------------- |
| Period           | Length before the sequence repeats — must be large |
| Uniformity       | Output should be evenly distributed                |
| Independence     | Consecutive outputs should appear uncorrelated     |
| Seed sensitivity | Different seeds should produce unrelated sequences |

Linear congruential generators use the recurrence $X_{n+1} = (aX_n + c) \mod m$. The choice of a, c, and m (typically a power of 2 or a large prime) determines the period and statistical quality. The maximum period is m, achieved only with specific parameter relationships.

### Cryptographically Secure PRNGs

Standard PRNGs are predictable given sufficient output. Cryptographic PRNGs add the requirement that predicting the next output is computationally infeasible — often built on number-theoretic hardness assumptions (e.g., the Blum Blum Shub generator relies on the difficulty of factoring).

### Truly Random Sources

Hardware random number generators exploit physical phenomena (thermal noise, radioactive decay, photon arrival times). These lack the periodic behavior of algorithmic generators but are slower to produce.

## Cryptographic Applications — Conceptual Overview

### RSA — Factoring Hardness

The RSA construction relies on the difficulty of factoring the product of two large primes:

1. Choose large primes p, q; compute n = pq
2. Compute $\lambda(n) = \text{lcm}(p-1, q-1)$
3. Choose public exponent e coprime to λ(n)
4. Compute private exponent $d \equiv e^{-1} \pmod{\lambda(n)}$
5. Encryption: $c = m^e \mod n$; Decryption: $m = c^d \mod n$

Security rests on the assumption that factoring n into p and q is infeasible for sufficiently large primes.

### Diffie-Hellman — Discrete Logarithm Hardness

Two parties agree on a shared secret over an insecure channel:

1. Public parameters: prime p and generator g
2. Each party chooses a private exponent and publishes $g^{\text{private}} \mod p$
3. The shared secret is $g^{ab} \mod p$ — computable by both parties but not by an eavesdropper (assuming discrete log is hard)

### Checksums and Error Detection

Modular arithmetic underlies error detection codes:

- **Parity bits**: Sum mod 2
- **ISBN check digits**: Weighted sum mod 11 (using all digits 0-9 plus X for 10)
- **Luhn algorithm**: Weighted digit sum mod 10 for credit card validation
- **CRC**: Polynomial division over GF(2) — a finite field construction

## Load Distribution and Consistent Hashing

Distributing requests across n servers often uses modular hashing: server = hash(key) mod n. This works but has a critical weakness — changing n redistributes nearly all keys.

Consistent hashing addresses this by mapping both keys and servers onto a ring (integers mod a large prime or 2³²). Each key is assigned to the next server clockwise on the ring. Adding or removing a server redistributes only ~1/n of keys. The ring structure relies on modular arithmetic's wraparound property.

Strategies for improving balance on the ring include virtual nodes (mapping each physical server to multiple ring positions) and bounded-load algorithms that cap the maximum load per server.

## Debugging Hashing and Distribution Problems

Number-theoretic understanding helps diagnose common issues:

- **Clustering in hash tables**: Often caused by non-prime table sizes or hash functions that don't mix bits well. Check if key values share common factors with the table size.
- **Uneven load distribution**: If server count shares factors with hash output patterns, some servers receive disproportionate traffic. Prime server counts or better hash functions can improve uniformity.
- **PRNG period exhaustion**: If a simulation reuses values unexpectedly, the generator's period may be too short or the seed space too small.
- **Birthday paradox collisions**: In a space of size N, collisions become likely after ~√N random insertions. For 32-bit hashes, expect collisions around 2¹⁶ ≈ 65,536 items. For 64-bit, around 2³² ≈ 4 billion.

| Space size     | 50% collision probability after |
| -------------- | ------------------------------- |
| 2³² (32-bit)   | ~77,000 items                   |
| 2⁶⁴ (64-bit)   | ~5 × 10⁹ items                  |
| 2¹²⁸ (128-bit) | ~2 × 10¹⁹ items                 |
| 2²⁵⁶ (256-bit) | ~4 × 10³⁸ items                 |

## Connections Between Topics

```
Primes ──→ Factorization ──→ RSA
  │              │
  ▼              ▼
Modular Arith ──→ Euler's Totient ──→ Key Generation
  │              │
  ▼              ▼
Mod Exponentiation ──→ Discrete Log ──→ Diffie-Hellman
  │
  ▼
Hashing ──→ Distribution ──→ Consistent Hashing
  │
  ▼
PRNGs ──→ Simulation, Sampling, Cryptographic Protocols
```

Number theory provides the mathematical bedrock for problems that appear throughout computing — from the mundane (hash table sizing) to the critical (secure communication). The recurring theme is that operations easy in one direction (multiplication, exponentiation) but hard to reverse (factoring, discrete logarithm) create useful asymmetries that software systems exploit.
