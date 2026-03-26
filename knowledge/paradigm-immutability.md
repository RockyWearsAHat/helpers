# Immutability Patterns — Persistent Data Structures, Value Objects & Performance

## Immutability and Identity

**Mutability vs. immutability.** Mutable objects change in place:

```java
Person p = new Person("Alice", 30);
p.setAge(31);  // Same object, mutated
```

Immutable objects don't change; operations produce new objects:

```kotlin
data class Person(val name: String, val age: Int)
val p = Person("Alice", 30)
val p2 = p.copy(age = 31)  // New object; p unchanged
```

**Trade-offs.** Immutability reduces bugs (state can't surprise you later), simplifies concurrency (no locks), enables reasoning (value semantics). Cost: object allocation overhead, memory pressure, and learning curve.

**When immutability matters:** Shared state (threads, distributed systems), long-lived computations (event sourcing, audit trails), functional pipelines (map/filter/reduce).

## Persistent Data Structures

**The core idea.** Persistent data structures reuse structure between versions, sharing unmodified parts. Modifying a list returns a new list that shares nodes with the original.

**Tries (prefix trees) for strings and small-key data.** Each node represents a prefix; children branch by character. Modifying a trie path:

```
Original: "cat", "car" → 'c' root with 'a' child with 't' and 'r' children
Insert "cup": Reuse 'c' root, add new branch for 'u'
Structure: Original subtree ('c'→'a'→'t','r') shared; new 'c'→'u'→'p' branch added
```

Cost: O(k) where k = key length. Memory savings come from branch sharing; family(s) of tries with common prefixes are compact.

**Hash array mapped tries (HAMTs).** Generalization of tries using hash functions instead of prefixes. Efficient for arbitrary keys, enable persistent hash maps and sets. Clojure and Scala use HAMTs internally.

**Structural sharing.** The principle: two versions of a data structure share unmodified nodes, differing only in paths affected by changes.

```
Persistent List Insert:

Original:  [A] → [B] → [C] → nil
Version 1: [X] → [A] → [B] → [C] → nil
           └────────────────────────────┘
                  Shared nodes
```

Inserting X at the front creates one new node ([X]); the rest is shared.

## Copy-on-Write (CoW)

**CoW semantics.** Multiple references initially point to the same backing storage. On modification, create a new copy and modify that, leaving original intact.

```c
// POSIX fork uses CoW for memory pages
pid_t pid = fork();
if (pid == 0) {
    // Child process
    shared_array[0] = 999;  // OS CoWs the page; child gets its own copy
}
// Parent still sees original array[0]
```

**Cost model.** CoW is cheap until modification: read-only access is free (shared memory), write triggers copy (O(n) for full copy). Effective when most operations are reads or few pages are dirty.

**Application in user-space.** Languages and frameworks apply CoW manually:

```python
# Python's multiprocessing with fork()
# Parent and child share memory until child writes
data = [1, 2, 3]
process = multiprocessing.Process(target=modify, args=(data,))
```

JavaScript Immer library simulates CoW for immutable updates:

```javascript
const nextState = produce(state, draft => {
    draft.user.name = "Carol";  // Modify draft (copy)
});
// nextState is new object; state unchanged
```

Immer tracks changes, copies only modified branches.

## Value Objects and Equality

**Value semantics.** Objects compared by content, not identity. Two `Point(3, 4)` objects are equal regardless of creation location.

**Structural equality.** Languages support this differently:

```java
// Java: override equals() and hashCode()
class Point {
    int x, y;
    @Override public boolean equals(Object o) {
        return o instanceof Point p && p.x == x && p.y == y;
    }
}

// Kotlin: data class automatically generates equals
data class Point(val x: Int, val y: Int)

// Python: dataclass or __eq__
@dataclass
class Point:
    x: int
    y: int
```

**Immutability requirement for hashable value objects.** If a value object changes, its hash code changes, breaking hash table lookups. Immutable value objects can be safely used as dict keys and set members.

## Frozen Objects (JavaScript)

**Object.freeze.** JavaScript's Object.freeze() prevents property modification:

```javascript
const user = { name: "Alice", age: 30 };
Object.freeze(user);
user.age = 31;  // Silent failure in non-strict; error in strict
```

**Shallow freezing.** freeze() is shallow; nested objects remain mutable:

```javascript
const user = { name: "Alice", address: { city: "NYC" } };
Object.freeze(user);
user.address.city = "LA";  // ✓ Works! nested obj not frozen
```

Deep freezing requires recursive application. Libraries (immer, immutable.js) provide alternatives or helpers.

**Performance cost.** Freezing carries small runtime overhead (checks before property writes in engines that optimize for it).

## Records and Readonly Types (Java, C#)

**Java records (14+).** Records automatically generate immutable value semantics:

```java
record Point(int x, int y) { }
// Auto-generates: equals, hashCode, toString
```

Simpler than manual data classes. Fields are implicitly `final`.

**C# readonly.** C# supports `readonly` on fields and structs:

```csharp
readonly struct Point {
    public readonly int X;
    public readonly int Y;
}
```

Stack-allocated (if struct) and immutable. More lightweight than reference types.

**Functional alternatives.** Languages like Rust and Haskell default to immutability; mutability is explicit. Fewer patterns needed (no mutable by default problem).

## Immer.js for JavaScript Immutability

**Immer rationale.** JavaScript developers expect mutable syntax. Immer lets you write imperative-style updates while maintaining immutability:

```javascript
const nextState = produce(state, draft => {
    draft.user.name = "Carol";
    draft.todos.push({ title: "New task", done: false });
});
// nextState is new object with mutations applied; state unchanged
```

**How it works.** Immer wraps the state in a `Proxy`, intercepts mutations, records changes, and applies them to a fresh copy.

**Performance.** Immer has overhead (proxy interception, change tracking). For large states with few mutations, it's fast. For frequent fine-grained updates, hand-optimized immutable code might be faster.

**Trade-off.** Developer ergonomics (write imperative code) vs. performance (proxy overhead). Most applications don't hit the performance ceiling.

## Performance Implications

**Allocation cost.** Creating new objects for every change costs memory and GC pressure. Small data structures (tuples, short lists) amortize this. Large structures that change frequently may regress.

**Structure sharing benefit.** Persistent data structures help. If 90% of a tree is unchanged, sharing saves memory. If all nodes change, sharing is wasted overhead.

**Cache locality.** Mutable in-place modifications improve cache hits (sequential memory access). Persistent structures with scattered nodes may incur cache misses. Profile before assuming immutability is slower.

**GC implications.** More objects → more GC work. Generational GC handles short-lived immutable objects efficiently. Long-lived objects may fragment the heap.

**Optimization techniques:**
- **Batching.** Accumulate changes; apply in bulk rather than one-at-a-time.
- **Lazy copying.** Delay structural sharing until needed (copy-on-write).
- **Structural pruning.** If only a small branch changes, copy only that branch, not the whole tree.

## Concurrency and Thread Safety

**Immutability as default.** If state is immutable, concurrent reads are safe (no synchronization needed). Shared mutable state requires locks.

```java
// Immutable: no locks needed
public class User {
    private final String name;
    private final int age;
}

// Mutable: needs synchronization
public synchronized void setAge(int newAge) { ... }
```

**Copy-and-swap.** Update immutable state atomically:

```java
private volatile User user;
void updateUser(String newName) {
    User updated = new User(newName, user.age);
    user = updated;  // Atomic assignment
}
```

Single `volatile` write replaces complex locking.

**Downside:** Frequent updates create many intermediate objects, increasing GC burden.

## Partial Immutability Patterns

**Defensive copying.** Constructor receives mutable argument; copy it:

```java
public ImmutableList(List<?> source) {
    this.items = new ArrayList<>(source);  // Copy
}
```

Prevents caller from modifying backing store. Costs CPU/memory but guarantees safety.

**Sealed types.** Restrict subclasses to known set, enabling optimization:

```kotlin
sealed class Result {
    data class Success(val value: Int) : Result()
    data class Failure(val err: String) : Result()
}
```

Compiler can check exhaustiveness in when expressions. Hints optimizer about type hierarchy.

## When NOT to Use Immutability

**Hot loops with frequent allocations.** If code allocates millions of small objects in tight loops, immutability's allocation overhead may dominate.

**Large structures with rare access patterns.** If you own the entire object and never share it, mutation is simpler and faster.

**Mutable by semantics.** Mutable heaps, stacks, and priority queues describe their domain (access order matters). Immutable variants exist but feel unnatural.

**Legacy codebases.** Retrofitting immutability is expensive and disruptive. Consider on new code.