# Data Structures & Algorithms — When to Use What

## Complexity Cheat Sheet

### Time Complexity Intuition
| Complexity | Name | n=1M takes... | Example |
|-----------|------|---------------|---------|
| O(1) | Constant | Instant | Hash lookup, array index |
| O(log n) | Logarithmic | ~20 steps | Binary search |
| O(n) | Linear | 1M steps | Linear scan |
| O(n log n) | Linearithmic | ~20M steps | Good sorting |
| O(n²) | Quadratic | 1T steps (!) | Nested loops, bubble sort |
| O(2ⁿ) | Exponential | Heat death | Brute force subsets |
| O(n!) | Factorial | Beyond heat death | Brute force permutations |

### Space Complexity Matters Too
- In-place algorithm: O(1) extra space
- Merge sort: O(n) extra space
- Recursive algorithms: O(depth) stack space
- Hash tables: O(n) space for n entries

## Core Data Structures

### Arrays / Dynamic Arrays
```
Access: O(1)    Search: O(n)    Insert end: O(1) amortized    Insert middle: O(n)    Delete: O(n)
```
**Use when:** Random access needed, mostly appending, cache-friendly iteration.
**Avoid when:** Frequent insertions/deletions in the middle.
**Implementation:** Python `list`, Java `ArrayList`, C++ `vector`, JS `Array`.

### Linked Lists
```
Access: O(n)    Search: O(n)    Insert/Delete at known position: O(1)
```
**Use when:** Frequent insertions/deletions at arbitrary positions with an iterator; implementing queues/deques.
**Avoid when:** You need random access or cache-friendly iteration.
**Reality check:** In modern hardware, arrays beat linked lists for most workloads due to cache locality. Linked lists are mostly for interviews and specialized structures (LRU cache).

### Hash Tables (Hash Maps)
```
Average: Insert O(1)    Lookup O(1)    Delete O(1)
Worst:   Insert O(n)    Lookup O(n)    Delete O(n)    (hash collisions)
```
**Use when:** Fast key-value lookups, counting, deduplication, caching.
**Avoid when:** Need ordered traversal (use tree map), extreme memory constraints.
**Implementation:** Python `dict` (ordered since 3.7), Java `HashMap`, C++ `unordered_map`, JS `Map`.

### Trees

#### Binary Search Tree (BST)
```
Average: Insert O(log n)    Search O(log n)    Delete O(log n)
Worst (unbalanced): O(n) for everything
```

#### Balanced BSTs (Red-Black, AVL)
```
All operations: O(log n) guaranteed
```
**Use when:** Ordered data, range queries, finding successor/predecessor.
**Implementation:** Python `sortedcontainers.SortedDict`, Java `TreeMap`, C++ `map/set`, Rust `BTreeMap`.

#### Heap (Priority Queue)
```
Insert: O(log n)    Extract min/max: O(log n)    Peek min/max: O(1)
```
**Use when:** Need the minimum/maximum quickly, scheduling, top-k problems.
**Implementation:** Python `heapq` (min-heap), Java `PriorityQueue`, C++ `priority_queue`.

#### Trie (Prefix Tree)
```
Insert/Search: O(key_length)    Space: O(alphabet_size × total_chars)
```
**Use when:** Autocomplete, spell checking, IP routing, prefix matching.

### Graphs
```
Adjacency List: Space O(V+E), check edge O(degree), iterate neighbors O(degree)
Adjacency Matrix: Space O(V²), check edge O(1), iterate neighbors O(V)
```
**Use adjacency list** for sparse graphs (most real-world graphs).
**Use adjacency matrix** for dense graphs or when you need O(1) edge checks.

### Stack
```
Push: O(1)    Pop: O(1)    Peek: O(1)
```
**Use when:** LIFO ordering — expression evaluation, undo, DFS, bracket matching, function call simulation.

### Queue / Deque
```
Enqueue: O(1)    Dequeue: O(1)
```
**Use when:** FIFO ordering — BFS, task scheduling, buffering, sliding window.
**Implementation:** Python `collections.deque`, Java `ArrayDeque`.

## Essential Algorithms

### Sorting — Decision Guide
| Algorithm | Best | Average | Worst | Space | Stable? | When to use |
|-----------|------|---------|-------|-------|---------|-------------|
| Quicksort | O(n log n) | O(n log n) | O(n²) | O(log n) | No | General purpose, cache-friendly |
| Mergesort | O(n log n) | O(n log n) | O(n log n) | O(n) | Yes | Need stability, linked lists |
| Heapsort | O(n log n) | O(n log n) | O(n log n) | O(1) | No | Need guaranteed O(n log n) in-place |
| Timsort | O(n) | O(n log n) | O(n log n) | O(n) | Yes | **Default in Python, Java** — great for real data |
| Counting sort | O(n+k) | O(n+k) | O(n+k) | O(k) | Yes | Small integer range |
| Radix sort | O(d·n) | O(d·n) | O(d·n) | O(n+k) | Yes | Fixed-length integers/strings |

**In practice**: Use your language's built-in sort (Timsort). Only implement custom sorting for very specific performance needs.

### Binary Search
```python
def binary_search(arr, target):
    lo, hi = 0, len(arr) - 1
    while lo <= hi:
        mid = lo + (hi - lo) // 2  # Avoids overflow
        if arr[mid] == target:
            return mid
        elif arr[mid] < target:
            lo = mid + 1
        else:
            hi = mid - 1
    return -1  # Not found; lo = insertion point
```
**Variations:** Find first/last occurrence, find insertion point (bisect_left/right), binary search on answer (minimize/maximize a function).

### Graph Algorithms

#### BFS (Breadth-First Search)
```python
from collections import deque

def bfs(graph, start):
    visited = {start}
    queue = deque([start])
    while queue:
        node = queue.popleft()
        for neighbor in graph[node]:
            if neighbor not in visited:
                visited.add(neighbor)
                queue.append(neighbor)
```
**Use for:** Shortest path (unweighted), level-order traversal, connected components.

#### DFS (Depth-First Search)
```python
def dfs(graph, node, visited=None):
    if visited is None:
        visited = set()
    visited.add(node)
    for neighbor in graph[node]:
        if neighbor not in visited:
            dfs(graph, neighbor, visited)
```
**Use for:** Cycle detection, topological sort, connected components, path finding.

#### Dijkstra's Algorithm (Shortest Path, Non-Negative Weights)
```python
import heapq

def dijkstra(graph, start):
    dist = {start: 0}
    pq = [(0, start)]
    while pq:
        d, u = heapq.heappop(pq)
        if d > dist.get(u, float('inf')):
            continue
        for v, w in graph[u]:
            if d + w < dist.get(v, float('inf')):
                dist[v] = d + w
                heapq.heappush(pq, (dist[v], v))
    return dist
```

### Dynamic Programming Patterns

**Key insight:** If you can define the solution in terms of smaller subproblems, and those subproblems overlap, use DP.

1. **Fibonacci (trivial DP)**
   - Naive recursion: O(2ⁿ)
   - Memoized: O(n)
   - Bottom-up: O(n) time, O(1) space

2. **Common Subproblems:**
   - Knapsack (0/1 and unbounded)
   - Longest Common Subsequence / Substring
   - Edit Distance (Levenshtein)
   - Coin Change
   - Matrix Chain Multiplication

3. **Approach:**
   ```
   1. Define state: What changes between subproblems?
   2. Define transition: How do I build from smaller states?
   3. Define base case: What's the smallest subproblem?
   4. Determine order: Bottom-up or top-down?
   5. Optimize space: Can I reduce from 2D to 1D?
   ```

## Choosing the Right Structure — Quick Decision Tree

```
Need to look up by key?
  → Hash Map (average O(1))
  Need ordered iteration too? → Tree Map (O(log n))

Need to maintain order of insertion?
  → Array / Linked List / OrderedDict

Need min/max quickly?
  → Heap / Priority Queue

Need to check membership?
  → Hash Set (O(1) average)
  Need range queries? → Sorted Set / Tree Set

Need LIFO? → Stack
Need FIFO? → Queue
Need both ends? → Deque

Working with hierarchical data?
  → Tree (binary, n-ary, trie)

Working with relationships/connections?
  → Graph (adjacency list)

Need to merge disjoint groups?
  → Union-Find / Disjoint Set
```

## Amortized Analysis — Why ArrayList.add() is O(1)

Dynamic arrays double their capacity when full. Most inserts are O(1). Occasionally one is O(n) (copy everything). But averaged over n operations: each insert costs O(1) amortized.

Same principle applies to hash table resizing.

## Common Interview Patterns → Data Structure

| Pattern | Data Structure | Example Problems |
|---------|---------------|-----------------|
| Two pointers | Array / String | Container with most water, palindrome |
| Sliding window | Array + Deque/HashMap | Max sum subarray, longest substring without repeat |
| Fast & slow pointer | Linked List | Cycle detection, find middle |
| Stack for parsing | Stack | Valid parentheses, evaluate expression |
| Monotonic stack | Stack | Next greater element, largest rectangle |
| Top-K | Heap | K most frequent, K closest points |
| Prefix sum | Array | Range sum queries, subarray sum equals K |
| Union-Find | Disjoint Set | Connected components, redundant connection |
| Trie | Trie | Word search, autocomplete |
| BFS/DFS | Queue/Stack + Graph | Shortest path, flood fill, islands |

---

*"Premature optimization is the root of all evil — but so is choosing a linked list for random access." — Every data structures TA*
