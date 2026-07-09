<!-- COMPOSED FROM KNOWLEDGE, not found verbatim on disk. Searched for a prepared CS2420 principles
     doc across this machine (mdfind "CS 2420"/"CS2420"/"data structures", find ~ -iname variants,
     .github/knowledge, extraDocs, bin/knowledge, all local git repos+history) — no CS2420-branded
     principles document exists locally or on GitHub (the private RockyWearsAHat/cs-2420 repo holds
     only per-assignment Java source + a shared Maven pom.xml — build scaffolding, not principles;
     already captured, wrongly, as corpus/cs2420-setup.md and excluded there as language-specific).
     This document is composed, per owner direction, as proper language-agnostic data-structures-
     and-algorithms principles — the real content a CS2420 (Data Structures & Algorithms) course
     teaches — grounded in and partly quoting two authoritative knowledge sources already in this
     repo:
       - /Users/alexwaldmann/bin/extraDocs/data-structures-algorithms.md
         (byte-identical to /Users/alexwaldmann/bin/.github/knowledge/data-structures-algorithms.md)
     Composed 2026-07-09. -->

# CS2420 Principles — Data Structures & Algorithms (composed from knowledge)

Language-agnostic principles a Data Structures & Algorithms course (CS2420) grades against. Every
choice below is a design decision with a measurable cost — the discipline is naming that cost before
writing the code, not after it's slow.

## 1. Choose the data structure the *operations* demand, not the one that's familiar

The right structure is whichever gives every operation you actually perform the complexity you need
— not the one you reach for by habit. Per the complexity cheat sheet:

- Need O(1) membership test? Use a set/hash set, not a list scanned linearly.
- Need O(1) key lookup? Use a map/dict, not a list with linear search.
- Need ordered iteration *and* O(log n) insert? Use a balanced tree (Java `TreeMap`, C++ `map`, Rust
  `BTreeMap`), not a sorted array rebuilt on every insert.
- Need the running min/max fast? Use a heap/priority queue (O(log n) insert, O(1) peek), not a
  re-scan of the whole collection.
- Need to merge disjoint groups? Use Union-Find, not repeated set unions.
- Need LIFO vs FIFO vs both ends? Stack vs Queue vs Deque — pick the one whose native operations
  match the access pattern, don't bend a list to fake the discipline.

Maintaining two synchronized structures (e.g. a forward index and a reverse index) is a legitimate
choice when it's the only way to give *every* operation the complexity it needs — but it creates an
invariant ("both are always updated together") that must be enforced at every mutation site, not
assumed.

## 2. Big-O is a design tool, not a post-hoc grade

State the time and space complexity of every non-trivial operation *before* choosing the
implementation, the same way the cheat sheet pairs each structure with its cost table (array access
O(1) vs. search O(n); hash map average O(1) vs. worst-case O(n) under collision; balanced BST O(log
n) guaranteed vs. an unbalanced BST that degrades to O(n)). A structure chosen without stating its
complexity is a structure chosen blind — the choice must be defensible on the operations the code
actually performs, not on which container is most familiar.

Space complexity matters as much as time: an in-place algorithm costs O(1) extra space, merge sort
costs O(n), and every recursive algorithm costs O(depth) of stack space whether or not that's
budgeted for.

## 3. Amortized cost is real cost, but the average must be argued, not assumed

A dynamic array's `add()`/`append()` is O(1) *amortized* — most inserts are O(1), occasionally one is
O(n) when capacity doubles, and averaged over n operations each insert costs O(1). The same principle
governs hash-table resizing. Amortized analysis is a legitimate argument for calling an operation
"fast" — but it is an argument that has to be made (show the geometric growth, show the total cost
divided by n operations is O(1)), not a label applied because the common case felt quick in testing.
Worst-case cost does not vanish because it's rare; it has to be acceptable when it happens (e.g. no
resize allowed inside a latency-critical path).

## 4. Correctness before optimization — invariants are the correctness contract

A structure is correct only if its invariants hold after every operation, not merely on the happy
path. Before optimizing anything:

- Name the invariant a structure or algorithm depends on (a BST's ordering property, a heap's
  shape+order property, a hash table's load-factor bound, a Union-Find's parent-pointer acyclicity).
- Prove — or test — that every mutating operation preserves it, including the boundary cases: empty
  structure, single element, duplicate insert, delete of a non-existent element, delete of the only
  element.
- Only once correctness is established under those cases does a complexity number mean anything —
  a fast implementation of the wrong invariant is not a shortcut, it's a bug that scales.

"Premature optimization is the root of all evil — but so is choosing a linked list for random
access." Optimization without a stated invariant and a stated complexity target is guessing; pick the
structure for the operations required, verify it stays correct across the boundary cases, and only
then tune.

## 5. Match the algorithm family to the problem shape, not the reverse

- **Sorting**: the built-in sort (Timsort in Python/Java — O(n log n) worst case, stable, O(n) best
  case on nearly-sorted input) is correct by default; hand-roll a sort only for a stated constraint
  the built-in can't meet (fixed-width integer keys → radix/counting sort in O(n+k); guaranteed
  O(n log n) in O(1) space → heapsort).
- **Search**: binary search is O(log n) but *requires* a sorted invariant on the input — verify that
  invariant holds before reaching for it, and watch the classic correctness bug (`mid = lo + (hi -
  lo) // 2` avoids the overflow that `(lo + hi) // 2` does not).
- **Graph traversal**: BFS for shortest path on unweighted graphs and level-order needs (queue-driven,
  O(V+E)); DFS for cycle detection, topological sort, and connected components (stack/recursion-
  driven, O(V+E)); Dijkstra when edges are weighted and non-negative (heap-driven, O((V+E) log V)).
  The traversal choice is dictated by what question is being asked of the graph, not by which one is
  easier to code first.
- **Dynamic programming** applies exactly when a problem decomposes into smaller subproblems *that
  overlap* — state the recurrence (state, transition, base case, evaluation order) before writing the
  memo table; if subproblems don't overlap, DP buys nothing over plain recursion or divide-and-conquer.

## 6. Density and access pattern decide graph representation

Adjacency list costs O(V+E) space with O(degree) edge checks and neighbor iteration; adjacency matrix
costs O(V²) space with O(1) edge checks and O(V) neighbor iteration. Use the list for sparse graphs
(most real-world graphs); use the matrix only when dense or when O(1) edge-existence checks dominate
the workload. The representation is chosen from the graph's density and the operation mix, not by
default.

## 7. Test every structural boundary, not just the typical case

A data structure's correctness tests must cover, at minimum: the empty structure, a single element, a
duplicate insert (must be idempotent or explicitly defined), a no-op removal (removing what isn't
there must not corrupt state), and every error path an operation can raise. A structure that "works"
only on the multi-element steady-state case is unverified at exactly the boundaries where invariant
bugs live.

---

_Composed from: /Users/alexwaldmann/bin/extraDocs/data-structures-algorithms.md (complexity cheat
sheet, per-structure use/avoid guidance, sorting decision table, graph algorithms, DP pattern,
amortized-analysis section, decision tree, closing epigraph — all quoted or directly derived above).
No separate CS2420-branded source document was found on this machine._
