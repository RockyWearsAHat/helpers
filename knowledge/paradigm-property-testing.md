# Property-Based Testing in Practice — Generators, Strategies & Real-World Application

## Generator Design at Scale

**Generator composition.** Complex types are built from simpler generators. A generator for User objects combines generators for name, email, age:

```python
# Hypothesis
@composite
def user_gen(draw):
    return User(
        name=draw(text(min_size=1, max_size=100)),
        email=draw(emails()),
        age=draw(integers(min_value=0, max_value=150))
    )
```

Composability enables reuse: if you compose `user_gen` in list, you get `lists(user_gen())`, producing lists of users.

**Edge case generators.** General-purpose generators should include boundary cases. An integer generator should produce zero, negative numbers, max/min values, not just random mid-range values. Good frameworks do this automatically (distributing test cases across ranges).

**Constrained generators.** Domain-specific generators respect invariants:

```python
sorted_list = sorted(lists(integers()))  # ✗ Wrong: list might be unsorted
sorted_list = lists(integers()).map(sorted)  # ✓ Right: map ensures sorted
```

Unsorted lists violate the invariant "output is sorted", masking bugs in sort-dependent code.

**Filtering vs. mapping.** `.filter(lambda x: x > 0)` discards 50% of generated cases (inefficient). `.map(lambda x: abs(x))` transforms all cases and is faster. Prefer mapping where semantically valid.

## Shrinking and Debugging

**Shrinking rationale.** When a property fails on a large input (e.g., a list of 1000 items), the failure's root cause is rarely the size. Shrinking finds the **minimal failing case**:

```
Original failure: property(P) failed for [984, -122, 0, 3, 44, ..., 999] (1000 items)
Shrink 1: [984, -122, 0] (removed 997 items, still fails)
Shrink 2: [0] (removed 2 items, still fails)
Minimal case: [0] causes property to fail
```

Shrinking is automatic in modern frameworks (Hypothesis, fast-check, QuickCheck). It tries simplifications: smaller numbers, shorter lists, fewer elements, simpler values.

**Shrinking correctness.** Shrinking must preserve the failure. If `property([0])` passes but `property([0, 1])` fails, the framework won't shrink to `[0]` (correct because [0] doesn't reproduce the issue).

**When shrinking fails.** Shrinking assumes simplicity implies causality, but sometimes minimal examples are noisy. A property failing on `[0.0, NaN]` but passing on `[0.0, 1.0]` indicates NaN sensitivity, not list-size sensitivity. Examine shrunk examples carefully.

## Roundtrip and Invariant Properties

**Roundtrip properties.** Encode, then decode; result should equal original:

```python
def prop_json_roundtrip(obj):
    encoded = json.dumps(obj)
    decoded = json.loads(encoded)
    assert decoded == obj
```

Catches serialization bugs, encoding incompleteness. Works for any reversible transformation: compression, encryption, base64.

**Invariant properties.** State some fact that must hold after operation:

```python
def prop_sorted(lst):
    result = sort(lst)
    # Invariant 1: result is sorted
    for i in range(len(result) - 1):
        assert result[i] <= result[i+1]
    # Invariant 2: elements unchanged
    assert sorted(result) == sorted(lst)
```

Invariants catch partial failures: sort may be mostly correct but violate one constraint.

**Oracle properties.** Compare against trusted reference implementation:

```python
def prop_sort_matches_reference(lst):
    my_sort = my_sort_impl(lst)
    reference = list(sorted(lst))
    assert my_sort == reference
```

Trades implementation complexity for confidence. If reference is slow but correct (e.g., Python's built-in sort), oracle is cheaper than writing exhaustive invariants.

## Stateful and Model-Based Testing

**Stateful testing.** Properties that evolve state:

```python
# Hypothesis stateful testing
class QueueStateMachine(RuleBasedStateMachine):
    def __init__(self):
        self.queue = []
    
    @rule(item=just(1))
    def enqueue(self, item):
        self.queue.append(item)
    
    @rule()
    def dequeue(self):
        if self.queue:
            self.queue.pop(0)
    
    @invariant()
    def queue_consistent(self):
        assert all(isinstance(x, int) for x in self.queue)
```

Hypothesis generates random sequences of operations and verifies invariants hold throughout. Catches race conditions, state ordering bugs, and invalid state transitions.

**Model-based testing.** Compare implementation against simplified model:

```python
class DictModel:
    def __init__(self):
        self.model = {}
    
    def put(self, k, v):
        self.model[k] = v
    
    def get(self, k):
        return self.model.get(k)

class DictImplementation:
    # Actual implementation being tested
    pass

def test_dict(command_list):
    model = DictModel()
    impl = DictImplementation()
    
    for cmd in command_list:
        if cmd.type == 'put':
            model.put(cmd.k, cmd.v)
            impl.put(cmd.k, cmd.v)
        elif cmd.type == 'get':
            assert model.get(cmd.k) == impl.get(cmd.k)
```

Model is usually a simplified in-memory simulation; implementation is production code. Test verifies they produce identical results.

## Real-World Application Strategies

**Start properties you know will pass.** Roundtrip properties are usually safe: "encode then decode should equal original". Get confidence, then add harder properties.

**Use properties for regression testing.** Every bug discovered becomes a property: "this sequence of operations must not produce this error again". Regression properties catch resurfaces.

**Property-based shrinking for debugging.** When a unit test fails mysteriously, convert to property-based test. Shrinking clarifies the minimal failing case.

**Combine with example-based tests.** Properties aren't a replacement—example-based tests document intended behavior and domain expertise. Properties verify those behaviors hold broadly.

**Performance testing with properties.** Generate random large inputs and measure performance. Benchmark against property tests to catch algorithmic regressions:

```python
@given(lists(integers(), min_size=100, max_size=10000))
def test_search_performance(data):
    start = time.time()
    my_search(data, data[0])
    elapsed = time.time() - start
    assert elapsed < 0.1  # Must complete in under 100ms
```

## Tooling Across Languages

**Hypothesis (Python).** Mature, excellent shrinking, good error messages. Large community, many plugins (NumPy arrays, Pandas DataFrames).

**fast-check (JavaScript/TypeScript).** Modern, TypeScript-first, strong type support. Growing ecosystem. Good shrinkage and async support.

**QuickCheck (Haskell).** The original (2000). Elegant, lazy evaluation enables sophisticated shrinking. Limited in imperative languages.

**Proptest (Rust)** and **jqwik (Java).** Good frameworks in their languages. Proptest uses regression-file caching; jqwik has Spring integration.

**PBT frameworks across languages:** Most support generators, shrinking, and statistics (how often did this case occur?). Few make stateful testing ergonomic beyond Hypothesis.

## Coverage and Value Distribution

**Shrinking hides edge cases.** Shrinking produces minimal examples, but minimal doesn't mean typical. A property failing on `[0.0]` doesn't tell you how often you're testing with `[0.0]` vs. `[1000000]`. Most frameworks track this internally; check statistics.

**Custom distributions.** If random distribution skews away from failures, bias generators toward interesting regions:

```python
@given(integers().filter(lambda x: x > 100))  # Only > 100
@given(just(0) | integers())  # Bias toward 0
```

Biasing increases probability of finding bugs in that region.

**Statistical properties.** Some properties are probabilistic: "10% of runs should cache-hit". Property testing handles these awkwardly; use benchmark or statistical frameworks instead.

## Common Pitfalls

**Non-deterministic properties.** If property can pass or fail randomly for the same input, framework can't shrink. Ensure deterministic implementation and randomness is only in generator.

**Properties too strong.** A property claiming all permutations of a subarray are equal will fail. Start with weak properties and strengthen incrementally.

**Assuming coverage.** Property testing doesn't guarantee line coverage. A property might consistently avoid calling a branch. Combine with code coverage tools.

**Generator that doesn't generate.** A generator with heavy filtering (`filter(lambda x: x > 1000000)` on small integers) wastes cycles. Framework will give up after too many rejections.

**Ignoring floating-point quirks.** `assert x == sqrt(x**2)` fails due to rounding. Use approximate equality: `assert abs(x - sqrt(x**2)) < epsilon`.