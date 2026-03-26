# Logic Programming — Declarative Computation via Logical Inference

## Core Concept

Logic programming inverts the traditional evaluation model. Rather than imperative instructions ("do this, then that"), you declare facts and rules as logical predicates, then pose queries that the engine answers by searching through the logical space. The system automatically derives new facts from the database of rules using unification and backtracking.

A Prolog program is a set of logical clauses:
- **Facts**: ground truths (e.g., `parent(tom, bob).`)
- **Rules**: implications (e.g., `grandparent(X, Z) :- parent(X, Y), parent(Y, Z).`)
- **Queries**: questions to be answered (e.g., `?- grandparent(tom, X).`)

The key insight: computation becomes logical derivation. You specify the *what*, not the *how*.

## Execution Model: Unification and Search

### Unification

**Unification** is pattern matching with bidirectional binding. Two terms unify if they are structurally identical or variables can be bound to make them identical.

```
parent(tom, bob) unifies with parent(X, bob)  → X = tom
parent(tom, bob) unifies with parent(X, Y)    → X = tom, Y = bob
parent(tom, bob) unifies with parent(bob, X)  → fails
append([1,2], Y, [1,2,3,4])                   → Y = [3,4]
append(X, Y, [1,2,3,4])                       → generates multiple solutions
```

This bidirectionality is powerful — query and data are symmetric. You can query predicates forwards or backwards, and the same rule works for multiple use cases (e.g., `append` for concatenation, splitting, or membership checking).

### Backtracking

When a query fails at any point, the system **backtracks** to the most recent choice point and tries the next alternative. This automatic search is the engine's mechanism for exploring the solution space.

```prolog
% Three rules for number/1:
number(1).
number(2).
number(3).

% Query: ?- number(X).
% Attempt 1: unify with number(1) → X = 1 (succeed)
% On backtracking (press `;`), try next clause → X = 2
% On backtracking again → X = 3
% On backtracking again → fail (no more clauses)
```

Backtracking creates an implicit tree of choice points. The system performs depth-first search down this tree. If a subgoal fails, execution rolls back to the most recent choice point above it and tries the next branch.

### Resolution and SLD Resolution

Logic programming engines typically use **SLD resolution** (Selective Linear Definite clause resolution). Given a goal, the engine:
1. Selects a subgoal from the current goal list
2. Finds a clause whose head unifies with the subgoal
3. Replaces the subgoal with the clause body
4. Repeats until the goal list is empty (success) or no clause unifies (failure)

This is efficient compared to full logical inference because it's goal-driven and uses a single rule at a time.

## Cut (!) and Negation

### Cut: Pruning the Search Tree

The **cut** (written `!`) is a meta-predicate that commits to the current clause. It removes all choice points created since the clause was entered. This prevents backtracking.

```prolog
% Without cut: max/3 generates multiple solutions
max(X, Y, X) :- X >= Y.
max(X, Y, Y) :- Y > X.

% With cut: deterministic
max(X, Y, X) :- X >= Y, !.
max(X, Y, Y).  % only tried if first clause fails
```

Cut enables:
- **Determinism**: prevent spurious choice points, improving performance
- **Commitment**: once a clause is chosen, don't try others
- **Control**: implement if-then-else patterns

Controversy: Cut breaks declarativity. A clause with cut is no longer a tidy logical statement — it's a directive that says "don't reconsider this choice." Many consider cut a necessary performance optimization; others view it as a code smell indicating unclear logic.

### Negation-as-Failure

**Negation as failure** (`\+` or `not`) does not mean logical negation. Instead, `\+ Goal` succeeds if `Goal` fails.

```prolog
?- \+ number(4).  % succeeds (4 is not known to be a number)
?- \+ number(1).  % fails (1 is a number)
```

This is **not** logical negation (which would require completing the entire logical database). Instead, it assumes a "closed world" — if a fact can't be proven, it's false. This is pragmatic for databases and expert systems but violates classical logic semantics.

Negation-as-failure has subtle consequences:
- Order matters: `p(X) :- q(X), \+ r(X).` vs. `p(X) :- \+ r(X), q(X).` may give different results
- Negation in rule heads (negation-as-failure in the header) can create inconsistencies
- It's non-monotonic: adding new facts can make previously true queries false

## Constraint Logic Programming (CLP)

**CLP** extends logic programming with constraint domains. Rather than purely symbolic unification, it can reason about arithmetic, finite domains, linear inequalities, etc.

### CLP(FD) — Finite Domain Constraints

Most widely used. Constraints are over integers in a finite range.

```prolog
% Using SWI-Prolog's clpfd library
:- use_module(library(clpfd)).

% All different constraint: assign variables to different values
solve([X, Y, Z]) :-
    X #\= Y, Y #\= Z, X #\= Z,  % all different
    X #>= 0, X #=< 3,
    Y #>= 0, Y #=< 3,
    Z #>= 0, Z #=< 3,
    label([X, Y, Z]).          % search for assignment

% Query: ?- solve([X, Y, Z]).
% Generates all valid assignments: X=0, Y=1, Z=2, etc.
```

CLP(FD) is used for scheduling, sudoku solvers, configuration problems, and combinatorial search. The constraint solver prunes the search space early, much faster than raw Prolog backtracking.

### Other Constraint Domains

- **CLP(R)** — real numbers, linear constraints
- **CLP(B)** — Boolean satisfiability (SAT)
- **CLP(Q)** — rational numbers

## Datalog: Subset of Prolog

**Datalog** is a subset of Prolog without the cut, without negation-as-failure (or with careful negation), and without function symbols in data. It's designed for deductive databases and is simpler to reason about.

```datalog
% All lowercase, no cut, no arithmetic
parent(tom, bob).
parent(bob, ann).

ancestor(X, Y) :- parent(X, Y).
ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).

% Query: ?- ancestor(tom, ann).
```

Datalog is used in:
- Deductive databases (querying derived facts)
- Program analysis (computing reachability, data flow)
- Knowledge graphs (Wikidata uses Datalog-like systems)
- Datomic (immutable database with Datalog query language)
- Souffle (analysis framework for security and program analysis)

Datalog is **stratifiable** — computation can be organized into layers where negation only appears after earlier strata complete. This enables efficient evaluation and guarantees pointwise-fixpoint semantics.

## Answer Set Programming (ASP)

**ASP** shifts the semantics again. Rather than computing a single answer through search, ASP computes all **stable models** (answer sets) that satisfy the rules. It uses **negation-as-failure** but interprets it differently than Prolog.

```
node(1..3).
edge(1,2). edge(2,3).

% Color each node, either red or blue
{color(N, red) ; color(N, blue)} :- node(N).

% Constraint: no two adjacent nodes have same color
:- color(X, C), color(Y, C), edge(X, Y).
```

ASP is used for:
- Constraint satisfaction (find all solutions, not just one)
- Combinatorial optimization
- AI planning
- Knowledge representation

The semantics are different from Prolog: rather than depth-first search with a single answer, ASP finds all stable models. This is computationally harder but more declarative for certain problem classes.

## Datomic: Modern Logic Databases

**Datomic** brings logic programming principles to production databases. It uses Datalog as its query language with an immutable, accumulate-only architecture:

```clojure
; Transact facts (immutable append)
[{:db/id -1
  :person/name "Alice"
  :person/age 30}]

; Query using Datalog
[:find ?name ?age
 :where
 [?e :person/name ?name]
 [?e :person/age ?age]
 [?e :person/age ?a]
 [(> ?a 25)]]
```

Datomic's key innovations:
- **Accumulate-only writes**: data is never overwritten, enabling perfect audit trails
- **Time-aware queries**: query at any point in history
- **Logic-based joins**: declarative queries eliminate loop-based iteration

## Souffle: Program Analysis

**Souffle** is a Datalog-like language for static program analysis. It extended Datalog with aggregation and modern performance optimizations, used in security, type systems, and compiler research.

```souffle
% Three-address code analysis
Statement(stmt, op, arg1, arg2).

% Dataflow: which variables reach which points
Reaching(var, point) :-
   Definition(var, point) ;
   (Reaching(var, pred), Next(pred, point), !Kill(var, point)).
```

Souffle enables efficient analysis of codebases at scale. Its semantics are closer to traditional Datalog, with clear termination properties.

## Logic Programming in Modern Systems

### Integration with Traditional Languages

Most modern systems don't use logic programming exclusively. Instead, they integrate logical query/inference as a library:

- **Python/JavaScript**: Constraint solvers (scipy, constraint.js)
- **Functional languages**: Logic combinators (e.g., `miniKanren` in Scheme/Clojure)
- **Databases**: Datalog/logic query layers (Datomic, Jena RDF stores, some Prolog libraries in Go)
- **ML training**: Logical constraints as loss functions or proof search as training data

### Performance and Scalability

Traditional Prolog (chronological backtracking) is slow on large solution spaces. Modern optimizations:

- **Tabling** (memoization) — SWI-Prolog's `table/1` directive: cache results for subgoals to avoid recomputation
- **Constraint propagation** (CLP) — prune search space before it explodes
- **Parallel search** — execute independent branches concurrently
- **JIT compilation** — WAM (Warren Abstract Machine) and modern variants compile to native code

Despite these, logic programming remains primarily for symbolic domains (AI planning, scheduling, parsing) rather than general computation.

## Mental Models and Trade-offs

### When Logic Programming Shines

- **Search problems**: scheduling, N-queens, constraint satisfaction
- **Symbolic AI**: expert systems, knowledge representation, reasoning
- **Parsing and NLP**: DCGs (Definite Clause Grammars) elegantly express syntax
- **Bi-directional transformations**: same rule for encode/decode or forward/backward querying

### When It Struggles

- **Imperative tasks**: sequencing reads/writes, stateful updates
- **Numerical computation**: arithmetic is cumbersome; outsource to CLP or external code
- **Performance-critical code**: backtracking search can be slow without tight constraints
- **Large datasets**: unindexed search over millions of facts is naive; real systems need better data structures

### Controversy: Is It Dead or Hibernating?

Some say logic programming failed because it doesn't map well to imperative architectures and user intuitions. Others argue it's not dead, just specialized — constraint solvers, Datalog systems, and ASP are quietly used in industry for specific classes of problems. Modern functional languages integrate logic via libraries (`miniKanren`, `constraint` packages), rather than building entire languages around it.

## See Also

- [Functional Programming](functional-programming.md) — orthogonal paradigm, sometimes combined with logic
- [Type Systems — Theory & Practical Application](type-systems-theory.md) — logic type checking (dependent types, refinement types)
- [Constraint Satisfaction and Optimization](api-design.md) — applications of CLP
- [Program Analysis](code-archaeology.md) — Datalog and Souffle in practice