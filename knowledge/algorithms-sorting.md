# Sorting Algorithms — Comparison vs. Non-Comparison, Stability, Adaptivity

Sorting is one of the most studied problems in computer science. Different algorithms excel in different contexts based on their runtime guarantees, space overhead, stability properties, and behavior on partially sorted data.

## Comparison-Based Sorts

All require at least $\Omega(n \log n)$ comparisons in the worst case (information-theoretic lower bound).

### Quicksort — O(n log n) average, O(n²) worst case, in-place

Quicksort recursively partitions around a pivot. It's **not stable** and **cache-friendly** (excellent locality of reference). Practical performance often exceeds mergesort despite worse worst-case complexity because of fewer data movements and CPU cache efficiency.

Performance highly depends on pivot selection:
- **Worst case**: already sorted data with naive pivot choice (first/last element)
- **Average case**: randomized or median-of-three pivot selection yields O(n log n)
- **Space**: O(log n) recursion depth

Quicksort dominates practice because modern systems reward cache locality. But it guarantees no upper bound on time.

### Mergesort — O(n log n) guaranteed, requires O(n) extra space, stable

Mergesort divides, recursively sorts, then merges. It's **stable** (equal elements retain original order) and **adaptive** to runs of already-sorted data (especially timsort variants).

- **Guaranteed O(n log n)** makes it suitable for real-time systems
- **Requires O(n) working space** — expensive in memory-constrained settings
- **Poor cache locality** compared to quicksort (many scattered memory accesses)
- Used internally by Python's `sorted()` function (via timsort)

### Heapsort — O(n log n) guaranteed, in-place, unstable

Heapsort maintains a max-heap and repeatedly extracts the largest element. Guarantees O(n log n) worst case with O(1) space, but:

- **Rarely faster than quicksort** in practice despite the same asymptotic bound
- **Poor cache behavior** (random heap access patterns)
- **Unstable** and not adaptive

Heapsort matters more in embedded systems or when space is critical than in typical applications.

### Timsort — O(n log n) adaptive, stable, mutable

Timsort (used in Python, Java, Android) combines mergesort with insertion sort:

1. Splits data into small runs (~32-64 elements)
2. Sorts each run with insertion sort
3. Merges runs efficiently

**Adaptive properties**: Detects existing sorted runs (natural runs) and uses them without re-sorting. On nearly sorted data, timsort approaches O(n).

- **Stable and guaranteed O(n log n)**
- **Minimal allocations** on sorted/semi-sorted inputs
- **Production quality** — battle-tested, robust

## Non-Comparison Sorts

Avoid the $\Omega(n \log n)$ lower bound by exploiting structure in the input.

### Radix Sort — O(nk) where k = number of digits, stable

Sorts by individual digits or bits, one pass per digit. Classic: **least significant digit (LSD) radix sort**.

- **Fast for fixed-size integers or strings** (k is typically small: 4-8 for 32-bit integers)
- **Classic assumption**: each element has $k$ digits in base $b$, so time is $O(nk)$ with $O(n+b)$ space
- **Stable** and **requires O(n + b) space** for buckets
- Poor performance if $k$ is large (sparse data or long strings)

### Counting Sort — O(n + k) where k = max element value

Counts occurrences of each value, then reconstructs sorted output. Stable variant preserves original order of equal elements.

- **Simple and optimal** when element range (k) is comparable to n
- **Impractical** if $k \gg n$ (e.g., sorting 10 numbers in range 0–1M)
- **Requires O(k) extra space**
- Used as a sub-routine in radix sort

### Bucket Sort — O(n + k) average, unstable without care

Distributes elements into k buckets, sorts each bucket, concatenates results.

- **Average case O(n + k)** assumes roughly uniform distribution
- **Unstable** unless the per-bucket sort is stable
- Real-world variant: distribute into ranges, use insertion sort within buckets

## Stability

An algorithm is **stable** if equal elements maintain their relative order. Example:

```
Input:  [(alice, 25), (bob, 25), (charlie, 30)]
Sort by age:
Stable:   [(alice, 25), (bob, 25), (charlie, 30)]  ← alice before bob preserved
Unstable: [(bob, 25), (alice, 25), (charlie, 30)]  ← order lost
```

**Why it matters**: Multi-stage sorts, preserving properties from earlier sorts, or database operations where original insertion order has meaning.

**Stable sorts**: Mergesort, timsort, counting sort, insertion sort  
**Unstable**: Quicksort, heapsort, selection sort, radix sort (LSD can be made stable with care; MSD is tricky)

## Adaptive Sorts

An **adaptive sort** exploits existing order in the input. Measures vary (subsequence length, inversions, runs) but the principle: nearly sorted data sorts faster.

| Type       | Measure | Intuition |
|------------|---------|-----------|
| Adaptivity to runs | Natural sorted sequences | Timsort, mergesort |
| Adaptivity to inversions | Number of pairs out of order | Insertion sort, bubble sort |
| Adaptivity to sequences | Length of longest increasing subsequence | Adaptive variants |

**Insertion sort** (O(n²) worst, O(n) on sorted) is adaptive but impractical for large datasets.

**Timsort** is the modern adaptive sort: O(n log n) worst case with O(n) average on nearly sorted data.

## External Sorting

When data exceeds RAM, external sorting reads from disk in passes:

1. Load chunks into RAM, sort with quicksort/mergesort
2. Write sorted runs to disk
3. Merge runs efficiently with minimal I/O

Mergesort's structure makes it ideal for external sorting. Number of merge passes: $\lceil \log_k(n/m) \rceil$ where m = RAM buffer size, k = number of merge pointers.

Database systems use variants of external mergesort for `ORDER BY`.

## Parallelization

- **Mergesort**: Natural divide-and-conquer; splits across cores, merges synchronized
- **Quicksort**: Parallel partitioning is complex; unbalanced partitions waste cores
- **Radix sort on GPUs**: Moving data to device memory often costs more than sorting on CPU for small n
- **Bitonic sort**: Designed for parallel hardware; good on massively parallel GPUs but not competitive on CPUs

## Choosing an Algorithm

| Scenario | Algorithm | Reason |
|----------|-----------|--------|
| Default choice (most data) | Timsort or quicksort | Adaptive, cache-friendly, proven |
| Guaranteed O(n log n) needed | Mergesort or heapsort | Predictable worst case |
| Fixed-size integers | Radix sort | Linear time, simpler than comparison |
| Embedded/memory-limited | Heapsort | O(1) space, O(n log n) time |
| Nearly sorted data | Timsort | O(n) on best case, stable |
| Streaming or online | Heap (priority queue) | Can select top-k without full sort |

---

**See also**: computational-thinking, performance-optimization, data-structures-algorithms, math-complexity-theory