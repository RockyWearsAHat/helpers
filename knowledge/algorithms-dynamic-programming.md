# Dynamic Programming — Overlapping Subproblems, Memoization, Tabulation, Patterns

Dynamic programming is a design technique for optimization problems: if a problem exhibits optimal substructure and overlapping subproblems, DP can reduce exponential brute force to polynomial time by caching results.

## Prerequisite Properties

### Optimal Substructure

The optimal solution is built from optimal solutions to subproblems. Formally: if $f(n)$ is optimal, then $f(n) = g(f(n-1), f(n-2), \ldots)$ where $g$ combines optimally-solved subproblems.

Example: shortest path. If the shortest path from A to C goes through B, then the path A→B must be the shortest path from A to B.

Counter-example (why it fails): longest path in a graph with cycles. The longest path to vertex v might not use optimal longest paths to its predecessors.

### Overlapping Subproblems

The same subproblem is solved multiple times in the recursion tree. Without caching, the same computation repeats exponentially.

Example: Fibonacci. $f(5) = f(4) + f(3)$, and $f(4) = f(3) + f(2)$ — the recursive tree recomputes $f(3)$ twice.

No overlap example: Merge sort. Each subproblem (merge range [i, j]) appears once. Recursively, it's solved only once per size. **No benefit from memoization.**

## Memoization (Top-Down)

Cache subproblem results in a dictionary/map. Solves recursively, storing computed values.

```python
memo = {}
def fib(n):
    if n in memo: return memo[n]
    if n <= 1: return n
    memo[n] = fib(n-1) + fib(n-2)
    return memo[n]
```

**Characteristics**:
- Write: Natural recursive form, easier to reason about
- Time: O(# unique subproblems × time per subproblem)
- Space: O(# unique subproblems) for cache + O(recursion depth) for call stack
- Cache misses have overhead but avoid redundant computation

**When to use**: Problem size is unknown, only a subset of subproblems are actually needed, recursive structure is clearer.

## Tabulation (Bottom-Up)

Explicitly iterate through all subproblems in dependency order, filling a table.

```python
def fib(n):
    dp = [0] * (n+1)
    dp[1] = 1
    for i in range(2, n+1):
        dp[i] = dp[i-1] + dp[i-2]
    return dp[n]
```

**Characteristics**:
- Time: O(# subproblems × time per subproblem)
- Space: O(# subproblems) for the table
- No call stack overhead
- Requires knowing all subproblems upfront

**When to use**: All subproblems will be solved, iterative order is clear, avoid recursion depth limits.

## Classical Patterns

### Knapsack Problem — O(nW) where n = items, W = capacity

**0/1 Knapsack** (each item taken 0 or 1 times):
```
dp[i][w] = max value using first i items with capacity w
dp[i][w] = max(
    dp[i-1][w],               // don't take item i
    dp[i-1][w-weight[i]] + value[i]  // take item i
)
```

**Unbounded Knapsack** (items reuse allowed):
```
dp[w] = max(dp[w], dp[w-weight[i]] + value[i]) for all items i
```

**Bounded Knapsack** (item i used at most count[i] times): Convert to 0/1 via binary representation.

**Applications**: resource allocation, investment portfolio, packing problems.

### Longest Common Subsequence (LCS) — O(mn)

```
dp[i][j] = LCS length of string1[0..i-1] and string2[0..j-1]
dp[i][j] = {
    0 if i=0 or j=0
    dp[i-1][j-1] + 1 if s1[i-1] == s2[j-1]
    max(dp[i-1][j], dp[i][j-1]) otherwise
}
```

**Reconstruction**: Backtrack through DP table to recover actual subsequence.

**Variants**: 
- Longest common substring (contiguous): Run on all subranges or track maximum
- Edit distance (Levenshtein): LCS-like but allows insertions/deletions/substitutions

### Edit Distance (Levenshtein) — O(mn)

Minimum operations (insert, delete, replace) to transform string1 to string2.

```
dp[i][j] = edit distance between string1[0..i-1] and string2[0..j-1]
dp[i][j] = {
    i if j=0, j if i=0
    dp[i-1][j-1] if s1[i-1] == s2[j-1]
    1 + min(dp[i-1][j], dp[i][j-1], dp[i-1][j-1])  // del, ins, sub
}
```

**Applications**: spell checking, DNA sequence alignment, fuzzy matching.

### Coin Change — O(n × amount) where n = number of coins

**Minimum coins** to make amount:
```
dp[i] = minimum coins for amount i
dp[i] = min(dp[i - coin] + 1) for all coins ≤ i
```

**Counting ways** to make amount:
```
dp[i] = number of ways for amount i
dp[i] += dp[i - coin] for all coins ≤ i
```

**Applications**: making change, subset sum variants, currency exchange.

### Matrix Chain Multiplication — O(n³)

Minimize scalar multiplications when computing product of n matrices (associativity changes cost, not result).

```
dp[i][j] = minimum cost to multiply matrices i through j
dp[i][j] = min(dp[i][k] + dp[k+1][j] + cost(i,k) × cost(k+1,j))
           for all k in [i, j-1]
```

**Applications**: compiler optimization, computational geometry, expression evaluation order.

## Space Optimization

Many DP problems use $O(1)$ or $O(n)$ space instead of $O(n^2)$ when the recurrence depends only on recent rows/columns.

**Fibonacci O(1) space**: Track only last two values instead of array of size n.

**LCS O(n) space**: Current and previous row; discard older rows.

**2D reduction to 1D**: If $dp[i][j]$ depends only on $dp[i-1][*]$, use rolling arrays.

## Advanced Patterns

### Bitmask DP — O(2^n × n)

Use bitmask to represent subset of items. State: $dp[mask]$ = answer for subset.

**Application**: Traveling salesman problem (TSP) on ≤ 20 cities. $dp[mask][city]$ = minimum cost to visit cities in mask, ending at city.

### Tree DP

Root tree at arbitrary node, solve for subtrees bottom-up.

$dp[node]$ = answer for subtree rooted at node, combining answers from children.

**Applications**: tree diameter, maximum weighted independent set, rerooting problems.

### Digit DP — O(d × state)

Count numbers up to N with certain digit properties. State: $(position, constraint, property)$.

$dp[pos][tight]$ = count when filling digits from position pos onward, tight = whether we're still bounded by N's digits.

**Applications**: count numbers with no repeated digits, counting numbers divisible by k, etc.

### Interval DP — O(n³)

$dp[i][j]$ = answer for range [i, j]. Often preceded by a middle element: $dp[i][j][k]$ = answer for range [i, j] with specific state k.

**Applications**: burst balloons, remove boxes, paint fence.

## Transition Tuning

- **Divide-and-conquer optimization**: If DP recurrence satisfies quadrangle inequality, monotone queue can reduce state transitions from O(n) to O(log n)
- **Convex hull trick**: For linear recurrences, maintain lower envelope of lines
- **CHT with Li Chao tree**: Dynamic lower envelope queries

## When DP May Not Help

- No overlapping subproblems → no cache benefit (e.g., merge sort)
- State space too large → memory/time infeasible
- Recurrence not easily invertible → can't build bottom-up
- Greedy or other technique simpler and sufficient

---

**See also**: computational-thinking, algorithms-graph, data-structures-algorithms, math-complexity-theory