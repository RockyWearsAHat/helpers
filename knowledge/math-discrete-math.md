# Discrete Mathematics — Combinatorics, Graph Theory & Foundations

Discrete mathematics provides the combinatorial and algebraic foundations for computer science. It encompasses structures without continuity: finite sets, graphs, Boolean algebra, and recursive relations.

## Combinatorics: Enumeration & Counting

Combinatorics counts arrangements and selections of discrete objects.

### Fundamental Principles

**Addition principle**: If sets are disjoint, $|A \cup B| = |A| + |B|$.

**Multiplication principle**: If tasks are sequential, total arrangements = product of choices per task.

**Inclusion-exclusion**: $|A \cup B| = |A| + |B| - |A \cap B|$. Generalizes to $n$ sets via alternating sums.

### Permutations & Combinations

**$k$-permutations of $n$**: Ordered selections of $k$ objects from $n$.

$$P(n,k) = \frac{n!}{(n-k)!}$$

**Combinations (binomial coefficient)**:

$$\binom{n}{k} = \frac{n!}{k!(n-k)!}$$

Unordered selection; coefficient of $x^k$ in $(1+x)^n$ (binomial theorem):

$$(x+y)^n = \sum_{k=0}^n \binom{n}{k} x^k y^{n-k}$$

**Properties**: $\binom{n}{k} = \binom{n}{n-k}$, $\binom{n}{k} = \binom{n-1}{k-1} + \binom{n-1}{k}$ (Pascal's identity), $\sum_{k=0}^n \binom{n}{k} = 2^n$

**Multinomial coefficient**:

$$\binom{n}{k_1, k_2, \ldots, k_m} = \frac{n!}{k_1! k_2! \cdots k_m!}$$

Arrangements of $n$ objects into $m$ types.

### Stirling Numbers

**Stirling of the second kind** $S(n,k)$: Partitions of $n$ elements into $k$ non-empty subsets.

$$n^m = \sum_{k=0}^m S(m,k) n^{\underline{k}}$$

where $n^{\underline{k}} = n(n-1)\cdots(n-k+1)$ is the falling factorial.

**Stirling of the first kind** $c(n,k)$: Permutations of $n$ elements with $k$ cycles.

$$n! = \sum_{k=1}^n c(n,k)$$

**Approximation** (Stirling's approximation):

$$n! \approx \sqrt{2\pi n} \left(\frac{n}{e}\right)^n$$

Rate: $(1 + O(1/n))$; allows asymptotic analysis of combinatorial expressions.

## Recurrence Relations & Generating Functions

**Recurrence relation**: Express term in sequence via earlier terms.

Example: $a_n = 2a_{n-1} + 1$ with $a_0 = 0$ (Fibonacci-like).

### Solving Recurrences

**Characteristic equation method** (linear homogeneous):

$$a_n - 2a_{n-1} = 0 \Rightarrow r^n - 2r^{n-1} = 0 \Rightarrow r = 2$$

General solution $a_n = A \cdot 2^n$; initial conditions determine $A$.

**Non-homogeneous**: Particular + homogeneous solution. For constant forcing: try constant particular.

**Divide-and-conquer** (Master theorem):

$$T(n) = aT(n/b) + f(n)$$

Recurrences from recursive algorithms:
- If $f(n) = \Theta(n^d)$:
  - $a < b^d$: $T(n) = \Theta(n^d)$
  - $a = b^d$: $T(n) = \Theta(n^d \log n)$
  - $a > b^d$: $T(n) = \Theta(n^{\log_b a})$

### Generating Functions

**Ordinary generating function (OGF)**:

$$G(x) = \sum_{n=0}^\infty a_n x^n$$

Encodes sequence $(a_0, a_1, a_2, \ldots)$.

**Recurrence relation** → **algebraic equation in $G(x)$**:

$$a_n = 2a_{n-1} \Rightarrow G(x) = a_0 + 2x G(x) \Rightarrow G(x) = \frac{a_0}{1-2x}$$

Coefficients: $a_n = a_0 \cdot 2^n$ (extract $[x^n]$ coefficient).

**Examples**:
- $\frac{1}{1-x} = 1 + x + x^2 + \cdots \Rightarrow a_n = 1$
- $\frac{1}{(1-x)^2} = 1 + 2x + 3x^2 + \cdots \Rightarrow a_n = n+1$
- $\frac{x}{(1-x-x^2)} = x + x^2 + 2x^3 + \cdots \Rightarrow a_n =$ Fibonacci

**Convolution**: If $G(x) = A(x)B(x)$ then $c_n = \sum_{k=0}^n a_k b_{n-k}$ (Cauchy product).

**Exponential generating function (EGF)**:

$$E(x) = \sum_{n=0}^\infty a_n \frac{x^n}{n!}$$

Useful for labeled combinatorial structures; convolution w.r.t. EGF counts labeled unions.

## Graph Theory Essentials

**Graph** $G = (V, E)$: Set of vertices (nodes) and edges (connections). **Directed** (arcs) or **undirected**.

### Basic Concepts

**Degree**: Number of incident edges. Degree sum: $\sum_{v} \deg(v) = 2|E|$ (handshaking lemma).

**Path**: Sequence of edges connecting vertices, no repeats. **Cycle**: Path of length $\geq 3$ that closes.

**Connected**: Path exists between any two vertices (for undirected). **Strongly connected** (directed): Path in both directions.

**Tree**: Connected acyclic graph; $|E| = |V| - 1$. **Forest**: Disjoint union of trees.

**Degree sequence**: Sorted degrees. Realizable if sum is even and satisfies Erdős–Gallai condition.

### Graph Representations

**Adjacency matrix** $A$: $A_{ij} = 1$ if edge $(i,j)$ exists, 0 otherwise. Dense representation; $A^k$ counts walks of length $k$.

**Adjacency list**: For each vertex, list neighbors. Space-efficient for sparse graphs.

**Incidence matrix** $B$: Rows = vertices, columns = edges; $B_{ve} = 1$ if $v$ incident to edge $e$.

### Connectivity & Components

**Cut**: Minimal set of edges whose removal disconnects graph. Minimum cut equals maximum flow (Max-Flow Min-Cut Theorem).

**Bridge**: Edge whose removal increases connected components.

**Vertex connectivity**: Minimum vertices to remove for disconnection. $\kappa(G)$ ≤ minimum degree.

### Planar Graphs

**Planar**: Drawable on plane without edge crossings. **Euler's formula** for connected planar:

$$|V| - |E| + |F| = 2$$

where $|F|$ = faces (regions).

**Consequence**: $|E| \leq 3|V| - 6$ (linear bound). $K_5$ (complete on 5 vertices) and $K_{3,3}$ (complete bipartite) are non-planar.

**Coloring**: Minimum colors for vertex coloring = chromatic number $\chi(G)$. **Four Color Theorem**: Planar graphs have $\chi(G) \leq 4$ (proved 1976, computer-assisted).

### Important Graph Classes

**Bipartite**: Vertices split into two sets; edges only between sets. No odd cycles. **Matching**: Set of edges with no shared vertices. König's theorem: max matching = min vertex cover (bipartite).

**Directed acyclic graph (DAG)**: No cycles. Admits topological sort (order compatible with edges). Used in dependency analysis, build systems.

**Hypergraph**: Edges connect any number of vertices (not just 2). Set systems are hypergraphs.

## Modular Arithmetic & Number Theory

**Modular equivalence**: $a \equiv b \pmod{n}$ if $n | (a-b)$.

**Division algorithm**: $a = qn + r$ with $0 \leq r < n$; $r = a \bmod n$.

**Modular arithmetic**: $(a+b) \bmod n = ((a \bmod n) + (b \bmod n)) \bmod n$; similarly for $*$.

**Fermat's Little Theorem**: If $p$ prime, $\gcd(a,p)=1$, then $a^{p-1} \equiv 1 \pmod{p}$.

**Euler's theorem**: $a^{\phi(n)} \equiv 1 \pmod{n}$ if $\gcd(a,n)=1$, where $\phi(n)$ = Euler totient (count of integers $< n$ coprime to $n$).

**Chinese Remainder Theorem**: System $x \equiv a_i \pmod{n_i}$ (pairwise coprime moduli) has unique solution modulo $\prod n_i$.

**Application**: RSA cryptography uses $n = pq$ (product of large primes); encryption exploits difficulty of factorization.

## Set Theory & Boolean Algebra

**Set operations**: Union $\cup$, intersection $\cap$, complement $\overline{A}$, difference $A \setminus B$.

**De Morgan's laws**: $\overline{A \cup B} = \overline{A} \cap \overline{B}$, $\overline{A \cap B} = \overline{A} \cup \overline{B}$.

**Cardinality**: $|A|$ = size. For finite: $|A \cup B| = |A| + |B| - |A \cap B|$.

**Boolean algebra** $\mathbb{B} = \{0, 1\}$: Operations $\lor$ (OR), $\land$ (AND), $\neg$ (NOT).

- Commutative, associative, distributive laws hold
- Absorption: $a \lor (a \land b) = a$
- Involution: $\neg(\neg a) = a$
- Complements: $a \lor \neg a = 1$, $a \land \neg a = 0$

**Boolean function**: $f: \mathbb{B}^n \to \mathbb{B}$. Can be expressed as disjunctive normal form (OR of ANDs) or conjunctive normal form (AND of ORs).

**Satisfiability (SAT)**: Does there exist assignment making formula true? NP-complete; no known polynomial algorithm.

## Pigeonhole Principle & Probabilistic Method

**Pigeonhole principle**: If $n+1$ objects placed in $n$ bins, some bin has $\geq 2$.

**Generalized**: $kn+1$ objects, $n$ bins → some bin has $\geq k+1$.

**Application**: Among 367 people, two share a birthday (pigeons = people, pigeonholes = 366 days).

**Probabilistic method**: Prove existence without construction. Show that random object satisfies property with positive probability → such objects exist.

**Example**: Graphs with high chromatic number and high girth (no short cycles) exist via probabilistic argument (not constructive).