# Randomized Algorithms — Monte Carlo, Las Vegas, Sampling, Sketching, Load Balancing

Randomized algorithms use random choices during execution to improve expected performance, enable simpler implementations, or provide probabilistic guarantees. Distinct from deterministic algorithms, they trade worst-case guarantees for better average (or expected) behavior.

## Monte Carlo vs Las Vegas

**Monte Carlo**: always terminates in bounded time; output is **correct with high probability** (or within bounded error). May produce wrong answer.

Example: randomized primality test (Miller-Rabin). Runs in $O(\log n)` time. Output "probably prime" or "definitely composite" with error probability $< 2^{-40}` (configurable).

**Las Vegas**: always produces correct output; **runtime** is probabilistic. Might not terminate quickly.

Example: randomized quicksort. Always outputs sorted array. Expected time $O(n \log n)`, but worst-case time $O(n^2)` (occurs with probability ~0 on random inputs).

**Trade-off**: Monte Carlo trades correctness for speed. Las Vegas trades worst-case runtime for deterministic correctness. Most applications prefer Las Vegas (correctness is non-negotiable); Monte Carlo used when error tolerance and computational budget are tight.

## Randomized Quicksort

Traditional quicksort: deterministic pivot selection (first, last, median-of-three) can trigger $O(n^2)` on adversarial input (e.g., already sorted with last-element pivot).

**Randomization**: choose pivot uniformly at random from the array.

**Analysis**: expected runtime $E[T(n)] = O(n \log n)` regardless of input. Intuition: each random partition divides the array into random-sized subarrays; expected size of each recursive call is $O(n/c)` for constant $c$, leading to $O(\log n)` depth in expectation.

**Benefit**: eliminates pathological inputs. Adversary cannot force worst-case without knowing random choices.

**Limitation**: worst-case $O(n^2)` still possible (e.g., all pivots chosen as boundary elements), but occurs with exponentially small probability.

**Practical relevance**: randomized quicksort is **competitive with deterministic sorts** while eliminating adversarial worst-cases. Used in practice (C++ standard library implementations often use randomization or dual-pivot variants to strengthen guarantees).

## Reservoir Sampling

Problem: select $k$ items uniformly at random from a stream of unknown length $n \gg k$. Must use only $O(k)` memory.

**Algorithm** (Vitter's Algorithm R):
1. Fill reservoir with first $k` items
2. For each subsequent item $i` (position $> k`), generate random $j \in [1, i]`
3. If $j \leq k`, replace reservoir[j] with item $i`; else discard

**Result**: each of the $n` items has probability $k/n` of being in final reservoir.

**Complexity**: $O(n)` time, $O(k)` space. Single pass.

**Proof of correctness**: by induction. After seeing $i` items, each has probability $k/i` of being in reservoir (can be verified by analyzing probability of inclusion/exclusion across iterations).

**Variants**: weighted reservoir sampling (sample with probability proportional to weight), distributed reservoir sampling (merge reservoirs from multiple streams).

**Use cases**: sampling from database query results, log files, network traffic — anywhere data arrives online and storage is constrained.

## Count-Min Sketch

Probabilistic data structure for **frequency estimation** in data streams. Estimates how many times an element has appeared.

**Structure**: array of size $m \times d` (typically small, e.g., $m = 1000, d = 10`). Additionally, $d` independent hash functions $h_1, \ldots, h_d`.

**Update frequency of item $x$**:
- For $j = 1 \ldots d`: increment counter[j][h_j(x)]

**Query frequency of $x$**:
- Return $\min(counter[1][h_1(x)], \ldots, counter[d][h_d(x)])$

**Error guarantee**: if true frequency of  $x$ is $f_x$, estimated frequency satisfies:
$$f_x \leq \hat{f}_x \leq f_x + (\epsilon \cdot N)$$
with probability $1 - \delta$, where $\epsilon = e / m$ (constant $e \approx 2.718$) and $\delta = 2^{ -d}$.

**Memory**: $O(md)$ space; no dependence on number of distinct items.

**Applications**: network traffic analysis (count packet types), cache hit rate estimation, heavy-hitter detection (items with frequency $> N / k$).

## HyperLogLog

Data structure for **cardinality estimation**: approximately count the number of distinct elements in a stream.

**Principle**: elements with rare bit patterns (low leading zeros in hash) are unlikely; if we observe such an element, cardinality is likely high.

**Structure**: register array of size $m$ (registers store leading-zero counts). Hash function maps items to $m$ registers.

**Update step**: for each item $x$:
1. Hash $x$ to get bit string
2. Find position of first 1-bit (call this $\rho$)
3. Update register[hash(x) % m] = max(register[hash(x) % m], $\rho$)

**Query**: cardinality estimate is derived from harmonic mean of registers (formula involves constants depending on $m$).

**Accuracy**: standard error $1.04 / \sqrt{m}`. With $m = 1024`, error is ~3%.

**Memory**: $O(\log \log n)` per register, $m$ registers total. Extremely efficient (16KB storage estimates cardinality of billions).

**Improvement**: HyperLogLog++ adds preprocessing to handle small cardinalities more accurately.

**Applications**: database query optimization (estimate output size), data warehouse systems (distinct count aggregations), approximating set sizes without storing all elements.

## MinHash / Locality-Sensitive Hashing

Techniques for **approximate set similarity** and similarity search.

**MinHash** (for Jaccard similarity): 
1. Apply $k$ independent hash functions to set elements
2. For each function $h_i`, store minimum hash value: $m_i = \min\{h_i(x) : x \in S\}`
3. Estimate Jaccard similarity(A, B) as: fraction of hash functions where $m_A = m_B$

**Property**: $\Pr[\text{min hash equal}] = Jaccard(A, B)`. Increasing $k` improves estimation.

**Memory**: $O(k)` per set (all elements compressed into $k` values).

**Use**: detecting near-duplicate documents, finding similar users in recommendation systems.

**Locality-Sensitive Hashing (LSH)**: family of hash functions where similar items hash to same bucket with high probability; dissimilar items rarely collide.

**Example**: random hyperplane hashing for approximate nearest neighbor search. Items hashed based on which side of random hyperplanes they fall on.

## Random Sampling in Algorithms

**Randomized load balancing** (Power of Two Choices):
- Instead of assigning tasks to random server, query 2 random servers and assign to less-loaded one
- Reduces maximum load from $\Theta(\log n)` to $\Theta(\log \log n)` with high probability
- $O(1)` additional cost (two random queries)

**Applications**: work-stealing schedulers, load-balanced job queues.

**Randomized graph algorithms**:
- Karger's min-cut algorithm: randomly contract edges to find minimum cut in undirected graphs
- Expected time $O(n^2 \log n)` (slower than deterministic algorithms but elegant, provides insight)

## Skip Lists as Randomized Data Structure

Skip lists are fundamentally randomized: each node gets random height from geometric distribution.

**Expected complexity**: $O(\log n)` search, insertion, deletion (height is random, but distributed such that longest chains are rare).

**Advantages over balanced trees**: no rotation logic (simpler code), concurrent access friendlier, comparable complexity guarantees.

**Trade-off**: non-deterministic worst-case (single operation may traverse full list if heights are unlucky), but probability is vanishingly small.

## Randomized vs Deterministic

| Aspect | Randomized | Deterministic |
|--------|-----------|---------------|
| Worst case | often higher (or unbounded) | usually better |
| Average case / Expected | often better | may be worse |
| Adversarial input | resistant (randomness defeats adversary) | vulnerable |
| Simplicity | often simpler (no complex balancing) | complex invariants |
| Reproducibility | non-deterministic | repeatable |
| Hidden constants | varies with random seed | fixed |

**When to use randomized**:
- Adversarial input likely
- Average case dominates (common in practice)
- Simplicity matters (fewer bugs)
- Approximation acceptable (sampling, sketching)

**When to use deterministic**:
- Worst-case matters (real-time systems, SLAs)
- Reproducibility required (debugging, auditing)
- Theoretical lower bounds matter
- Adversarial input negligible

## Derandomization Techniques

Some randomized algorithms can be **derandomized**: replace random choices with deterministic ones (e.g., via universal hash families, algebraic methods).

**Example**: random quicksort derandomized by using deterministic median-of-medians pivot selection (yields deterministic $O(n \log n)` sort, but higher constant factors).

Derandomization often complex; random versions preferred in practice.

## Tail Bounds & Probability

Core analytical tool: **Chernoff bounds**, **Markov inequality**, **Chebyshev's inequality** quantify probability of deviation from expectation.

Example (Chernoff): if $X$ is sum of independent Bernoulli random variables, deviation probability decays exponentially:
$$\Pr[X > (1 + \delta) \mu] \leq e^{-\frac{\delta^2 \mu}{2 + \delta}}$$

Enables fine-tuning of error probabilities (add more hash functions, more iterations) to achieve desired guarantees.

## See Also

algorithms-hash-tables (universal hashing), algorithms-trees (skip lists, treaps), data-structures-algorithms, math-probability-theory