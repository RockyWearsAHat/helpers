# Bloom Filters — Hash Functions, False Positives, and Variants

## Overview

A **Bloom filter** is a space-efficient probabilistic data structure that answers membership queries: "Is element X in this set?" The distinctive property is that it always answers "possibly" or "definitely not" — false negatives never occur, but false positives do with a tunable probability.

The core idea: Use $k$ independent hash functions to set bits in an array. To check membership, hash the query and check if all corresponding bits are set. If any bit is unset, the element is definitely not in the set. If all bits are set, the element is probably in the set.

**Applications:** Database bloom indexes, caches, network flow analysis, spell checkers, duplicate detection, malware signature databases, distributed systems membership protocols.

---

## Mathematical Foundation

### False Positive Rate

Given:
- Array size: $m$ bits
- Number of elements inserted: $n$
- Number of hash functions: $k$
- Bits set during operation: approximately $kn$ (assuming independent hash functions and a large array)

The probability that a specific bit is **still 0** after $n$ insertions with $k$ hash functions:

$$P(\text{bit} = 0) = \left(1 - \frac{1}{m}\right)^{kn} \approx e^{-kn/m}$$

For a query, all $k$ bits must be set to return "possibly in set." The false positive rate:

$$P(\text{false positive}) = \left(1 - e^{-kn/m}\right)^k$$

This is minimized when $k = \frac{m}{n} \ln 2 \approx 0.693 \frac{m}{n}$.

### Optimal Parameters

To achieve a target false positive probability $p$:

$$m = -\frac{n \ln p}{(\ln 2)^2}$$

$$k = -\frac{\ln p}{\ln 2} = \log_2 \frac{1}{p}$$

Example: For $n = 1,000,000$ elements and $p = 0.01$:
- $m \approx 9.6 \text{ million bits} \approx 1.2 \text{ MB}$
- $k \approx 7$ hash functions

Doubling the array size reduces false positives by a factor of $1/4$.

---

## Hash Function Selection

### Requirements

1. **Independence:** Hash functions should produce uncorrelated outputs. Perfect independence is impossible with $k$ functions; the goal is near-independence for sufficiently large $m$.

2. **Uniformity:** Each hash function should produce outputs uniformly distributed over $[0, m-1]$.

3. **Efficiency:** Must be fast; Bloom filters are often used in hot paths.

### Practical Approaches

**Single hash with randomization:** Use one universal hash function and perturb the output:
- `hash(x, i) = (h(x) + i*d(x)) % m`, where $d(x)$ is a secondary hash
- Common in implementations; trades some independence for simplicity

**Two or three independent hash functions:** Pre-compute 2-3 hash values, then combine them:
- $h_i(x) = (h_1(x) + i \cdot h_2(x)) \bmod m$
- This is the "double hashing" approach; provides near-independence

**Universal hash families:** Arithmetic hash functions (e.g., $h(x) = (ax + b) \bmod p$) with random $a, b$ chosen from a family of pairwise-independent functions. Good theoretical properties but slower than simple concatenation.

**MurmurHash, xxHash, SipHash:** Fast, high-quality hash functions often used in practice. These individual functions produce excellent bit distribution; cycling through different subsets of output bits can simulate multiple hash functions.

---

## False Positive Analysis Under Collisions

The mathematical model assumes independence, but real hash functions have collisions. With weak hash functions or small $m$:

- Multiple insertions may hash to the same bit multiple times
- The false positive rate degrades gracefully but predictably
- Strong hash functions (e.g., MurmurHash-based) maintain theoretical bounds reasonably well

**Rule of thumb:** If hash quality is poor, increase $m$ by 20-30% to compensate.

---

## Standard Bloom Filter: Insertion & Querying

**Insert:**
```
for i = 1 to k:
    bit_index = hash_i(element) % m
    array[bit_index] = 1
```

**Lookup:**
```
for i = 1 to k:
    bit_index = hash_i(element) % m
    if array[bit_index] == 0:
        return DEFINITELY_NOT_IN_SET
return PROBABLY_IN_SET
```

**Space:** $m$ bits (independent of the number of elements after insertion, though $n$ affects false positive rate).

**Time:** $O(k)$ per operation; constant with respect to the data size, not the array size.

---

## Counting Bloom Filters

**Problem with standard Bloom filters:** Deletion is impossible. Once bits are set, they remain set; removing an element requires knowing which bits to reset, but multiple elements may have set the same bit.

**Solution:** Use counters instead of bits. Each counter tracks how many elements contributed to that bit.

**Insert:**
```
for i = 1 to k:
    counter_index = hash_i(element) % m
    counters[counter_index] += 1
```

**Delete:**
```
for i = 1 to k:
    counter_index = hash_i(element) % m
    counters[counter_index] -= 1
```

**Lookup:** Same as standard Bloom filter; check if all counters > 0.

**Trade-off:** Space increases from $m$ bits to $m \times \lceil \log_2(n_{\max}) \rceil$ bits, where $n_{\max}$ is the maximum number of identical hash collisions expected. With careful parameter selection, this overhead is 50-100%, acceptable for many applications.

**Counter overflow:** If the same element is inserted multiple times or hash collisions cause counter overflows, the structure degrades ungracefully. Practitioners often use 4-byte counters to prevent this.

---

## Modern Variants

### Cuckoo Filters

**Idea:** Instead of storing a bit, store a **fingerprint** (small hash) of the element. On insertion, if a slot is occupied, "kick out" the existing fingerprint and recursively place it elsewhere (cuckoo hashing).

**Advantages:**
- Supports deletion without counters
- Slightly lower false positive rate than Bloom filters at similar space
- Fingerprint-based lookup can be faster (better cache locality)

**Disadvantages:**
- More complex implementation
- Performance degrades if the load factor exceeds ~50%; requires resizing
- Insertion time is not constant (though typically $O(1)$ amortized for low load factors)

**Space:** Similar to Bloom filters; typically 15-30% lower space for equivalent false positive rates.

### Quotient Filters

**Idea:** Store quotient and remainder of a hash value. Quotient determines the bucket; remainder stored in the bucket. Collisions are resolved by shifting entries.

**Advantages:**
- Faster lookup than Cuckoo filters in typical ranges (fewer hash function evaluations)
- Better cache locality
- Supports iteration over stored elements

**Disadvantages:**
- More complex than Bloom filters
- Performance sensitive to load factor and hash distribution
- Deletion can leave gaps that cause query efficiency to degrade

### Ribbon Filters

**Recent (2018) variant:** Compresses fingerprints using a linear system of equations over GF(2). Achieves near-theoretical minimum space for a target false positive rate.

**Advantages:**
- Smallest space overhead of any variant (~10% above theoretical minimum for the false positive target)
- Supports deletion
- Fast: typically 2-3 hash function evaluations

**Disadvantages:**
- Complex implementation (requires XOR-based bit manipulations and linear system solving at build time)
- Construction is offline (cannot stream inserts); requires batch preprocessing
- Limited library support; less common in practice

---

## Applications

### Databases & Indexes

**Use case:** Quickly reject queries that don't match any record without consulting disk.
- BigTable uses Bloom filters on SSTable indices to avoid disk seeks
- HBase, Cassandra, RocksDB use per-SSTable Bloom filters
- Reduces I/O on negative lookups by ~99%

### Caches

**Use case:** Avoid cache misses for items that were never cached (negative caching).
- CDNs use Bloom filters to identify "not worth requesting upstream"
- Web caches use them to avoid superfluous requests to a removed origin

### Network Flows

**Use case:** Detect duplicate or malformed packets; identify botnet C2 traffic.
- Firewalls use distributed Bloom filters to track known-good sources
- Flow-based anomaly detection relies on approximate membership tests

### Spell Checkers

**Use case:** Minimize storage for a large dictionary while supporting fast negative lookups.
- Bloom filters store valid words; misspellings typically fail the filter
- False positives (invalid word passes filter) accepted because a secondary check catches them

### Blockchain & P2P Networks

**Use case:** Simplified Payment Verification (SPV) in Bitcoin; peer discovery.
- SPV clients request Bloom filters from full nodes; filter specifies addresses to monitor
- P2P networks use Bloom filters to avoid re-requesting known-bad blocks

---

## Comparison with Alternatives

| Structure | Space | Lookup | Insert | Delete | False Positives |
|-----------|-------|--------|--------|--------|---|
| Bloom Filter | Best | $O(k)$ | $O(k)$ | ✗ | Yes |
| Counting BF | +50-100% | $O(k)$ | $O(k)$ | ✓ | Yes |
| Hash Set | O(n) | O(1) avg | O(1) avg | ✓ | No |
| Cuckoo Filter | 15-30% better | $O(1)$ | $O(1)$ amz | ✓ | Yes, lower |
| Ribbon Filter | 10% overhead | $O(k)$ | Batch only | ✓ | Yes, minimal |

**When to use Bloom filters:**
- Space is critical and false positives are acceptable
- Data is append-only (no deletions)
- Simplicity and proven library support matter

**When to use Cuckoo filters:**
- Need deletion support without space penalty of counters
- Can tolerate slightly more complex implementation

**When to use hash tables:**
- False positives are unacceptable
- Insert/delete/lookup performance matters more than space

---

## Distributed Bloom Filters

**Challenge:** A single filter may not fit on one machine; data stream is too large for offline processing.

**Approach 1 — Partitioned:** Divide element space among $M$ machines; each maintains a Bloom filter for its partition. Query all partitions. False positive rate remains local to each partition; union of the filters has the same rate.

**Approach 2 — Distributed Consensus:** Replicate the filter on multiple machines; synchronize insertions. Trade bandwidth for availability.

**Approach 3 — Scalable Bloom Filters:** When capacity is unknown, start with a small filter and add new filters as insertions exceed a threshold. Query checks all filters. Slightly higher false positive rate; total rate is sum of individual rates.

---

## Implementation Considerations

1. **Bit Array Access:** Use a concrete bit-packing library rather than boolean arrays. A boolean array is typically 1 byte per bit; bit-packing reduces space by 8×.

2. **Hash Randomization:** Initialize hash functions with random seeds to avoid correlation over multiple Bloom filters in the same system.

3. **Serialization:** Bloom filters are easily serialized (just the bit array + metadata: $m$, $k$). Useful for caching, replication.

4. **Memory Layout:** Store the bit array in contiguous memory to maximize cache efficiency during lookups.

5. **Counter Overflow Handling:** For counting Bloom filters, use appropriately sized counters (4-8 bytes) and document overflow behavior — typically accept the overflow or resize.

---

## Theoretical Limits & Open Questions

- The false positive rate $\left(1 - e^{-kn/m}\right)^k$ is asymptotically optimal for any data structure that answers membership queries with no false negatives and bounded false positives.
- Variants (Cuckoo, Ribbon) approach these limits more closely via different encoding schemes.
- Dynamic resizing of Bloom filters (growing $m$ online) is achievable but not standardized; most systems replace entire filters.

**See also:** algorithms-hash-tables.md, distributed-partitioning.md, algorithms-randomized.md, math-information-theory.md