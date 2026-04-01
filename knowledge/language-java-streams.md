# Java Streams API — Pipelines, Lazy Evaluation, and Functional Data Processing

## Overview

The Streams API (Java 8+) provides a declarative, functional-style approach to processing collections. A stream is a sequence of elements that flows through a pipeline of operations. Unlike imperative loops, streams defer computation (lazy evaluation), enable parallelization, and abstract data source details.

## Stream Pipelines

A stream pipeline consists of:

1. **Source**: Collection, array, generator, or I/O
2. **Intermediate operations**: Transform elements (filter, map, flat­map)
3. **Terminal operation**: Produce a result (collect, reduce, forEach)

```java
List<String> names = List.of("Alice", "Bob", "Charlie");

// Pipeline:
// Source: names.stream()
// Intermediate: .filter(n -> n.length() > 3)
// Intermediate: .map(String::toUpperCase)
// Terminal: .collect(toList())
List<String> result = names.stream()
    .filter(n -> n.length() > 3)
    .map(String::toUpperCase)
    .collect(Collectors.toList());

// Result: ["ALICE", "CHARLIE"]
```

A stream is **consumed** by a terminal operation; reusing a stream throws `IllegalStateException`:

```java
Stream<String> stream = names.stream();
stream.forEach(System.out::println);
stream.forEach(System.out::println);  // IllegalStateException: stream consumed
```

## Lazy Evaluation

Intermediate operations do not execute until a terminal operation is called. Evaluation is **lazy and short-circuiting**:

```java
Stream<Integer> nums = Stream.of(1, 2, 3, 4, 5);

nums
    .peek(x -> System.out.println("Processing: " + x))
    .filter(x -> x > 2)
    .limit(2)
    .forEach(System.out::println);

// Output:
// Processing: 1
// Processing: 2
// Processing: 3
// (stops; limit(2) satisfied)
```

Lazy evaluation enables:
- **Short-circuiting**: `limit(n)` stops after n elements
- **Avoiding unnecessary work**: Intermediate operations don't execute until needed
- **Infinite streams**: `Stream.iterate()` works because operations are deferred

```java
// Infinite stream is safe; limit(5) stops after 5 elements
Stream.iterate(1, x -> x + 1)
    .limit(5)
    .forEach(System.out::println);  // 1, 2, 3, 4, 5
```

## Common Intermediate Operations

### filter

Retain elements matching a predicate:

```java
Stream<Integer> nums = Stream.of(1, 2, 3, 4, 5);
nums.filter(x -> x % 2 == 0)
    .forEach(System.out::println);  // 2, 4
```

### map

Transform each element:

```java
List<String> words = List.of("a", "bb", "ccc");
words.stream()
    .map(String::length)
    .forEach(System.out::println);  // 1, 2, 3
```

### flatMap

Transform each element into a stream, then flatten:

```java
List<List<Integer>> lists = List.of(
    List.of(1, 2),
    List.of(3, 4),
    List.of(5)
);

lists.stream()
    .flatMap(List::stream)
    .forEach(System.out::println);  // 1, 2, 3, 4, 5

// Split strings into characters
Stream.of("hello", "world")
    .flatMap(s -> s.chars().boxed().map(c -> (char) c))
    .forEach(System.out::println);  // h, e, l, l, o, w, o, r, l, d
```

### distinct

Remove duplicates (uses `equals` and `hashCode`):

```java
Stream.of(1, 2, 2, 3, 1)
    .distinct()
    .forEach(System.out::println);  // 1, 2, 3
```

### sorted

Sort elements (natural order or custom comparator):

```java
Stream.of(3, 1, 4, 1, 5)
    .sorted()
    .forEach(System.out::println);  // 1, 1, 3, 4, 5

Stream.of("apple", "pie", "a")
    .sorted(Comparator.comparingInt(String::length))
    .forEach(System.out::println);  // a, pie, apple
```

### peek

Inspect elements without modifying them (debugging):

```java
Stream.of(1, 2, 3)
    .peek(x -> System.out.println("Before filter: " + x))
    .filter(x -> x > 1)
    .peek(x -> System.out.println("After filter: " + x))
    .collect(Collectors.toList());
```

## Terminal Operations

### forEach

Consume all elements:

```java
List<String> names = List.of("Alice", "Bob");
names.stream().forEach(System.out::println);
```

### collect

Gather elements into a collection using collectors:

```java
List<Integer> list = Stream.of(1, 2, 3).collect(Collectors.toList());
Set<Integer> set = Stream.of(1, 2, 2, 3).collect(Collectors.toSet());
```

### reduce

Combine elements into a single value:

```java
int sum = Stream.of(1, 2, 3, 4)
    .reduce(0, (acc, x) -> acc + x);  // 10

Optional<Integer> max = Stream.of(3, 1, 4)
    .reduce((a, b) -> a > b ? a : b);  // Optional[4]

// String concatenation
String result = Stream.of("a", "b", "c")
    .reduce("", (acc, s) -> acc + s);  // "abc"
```

### count, min, max, anyMatch, allMatch

```java
long count = Stream.of(1, 2, 3).count();  // 3

Optional<Integer> min = Stream.of(3, 1, 4).min(Integer::compareTo);  // Optional[1]

boolean hasEven = Stream.of(1, 3, 5, 7).anyMatch(x -> x % 2 == 0);  // false

boolean allPositive = Stream.of(1, 2, 3).allMatch(x -> x > 0);  // true
```

## Collectors

`Collectors` provides reusable reduction strategies:

### toList, toSet, toCollection

```java
List<String> list = stream.collect(Collectors.toList());
Set<String> set = stream.collect(Collectors.toSet());
TreeSet<String> tree = stream.collect(Collectors.toCollection(TreeSet::new));
```

### joining

```java
String csv = Stream.of("a", "b", "c")
    .collect(Collectors.joining(", "));  // "a, b, c"

String quoted = Stream.of("x", "y")
    .collect(Collectors.joining("/*", "*", "*/"));  // "/*x*y*/"
```

### groupingBy

Group elements by a function:

```java
Map<Integer, List<String>> byLength = Stream.of("a", "bb", "ccc", "dd")
    .collect(Collectors.groupingBy(String::length));
// {1=["a"], 2=["bb", "dd"], 3=["ccc"]}

// With custom downstream collector
Map<Integer, Long> countByLength = Stream.of("a", "bb", "ccc")
    .collect(Collectors.groupingBy(String::length, Collectors.counting()));
// {1=1, 2=1, 3=1}

// Nested grouping
Map<Integer, Map<Character, List<String>>> nested = Stream.of("apple", "apricot", "banana")
    .collect(Collectors.groupingBy(
        String::length,
        Collectors.groupingBy(s -> s.charAt(0))
    ));
```

### partitioningBy

Split into two groups (true/false):

```java
Map<Boolean, List<Integer>> evenOdd = Stream.of(1, 2, 3, 4, 5)
    .collect(Collectors.partitioningBy(x -> x % 2 == 0));
// {false=[1, 3, 5], true=[2, 4]}

// With custom downstream
Map<Boolean, Long> countEvenOdd = Stream.of(1, 2, 3, 4, 5)
    .collect(Collectors.partitioningBy(x -> x % 2 == 0, Collectors.counting()));
```

### toMap

```java
Map<String, Integer> lengths = Stream.of("a", "bb", "ccc")
    .collect(Collectors.toMap(s -> s, String::length));
// {"a"=1, "bb"=2, "ccc"=3}

// With duplicate key merge function
Map<Integer, String> byLengthLatest = Stream.of("a", "aa", "bb", "ccc")
    .collect(Collectors.toMap(
        String::length,
        s -> s,
        (old, new_) -> new_  // Keep latest on conflict
    ));
```

### Custom Collectors

```java
// Sum of integers directly
Collector<Integer, ?, Integer> summing = Collectors.summingInt(x -> x);

// Average
Collector<Integer, ?, Double> averaging = Collectors.averagingInt(x -> x);

// Statistics
Collector<Integer, ?, IntSummaryStatistics> stats = Collectors.summarizingInt(x -> x);
IntSummaryStatistics s = Stream.of(1, 2, 3).collect(stats);
System.out.println("Average: " + s.getAverage() + ", Sum: " + s.getSum());
```

## Optional

`Stream` commonly produces `Optional<T>` from terminal operations (`findFirst`, `reduce`):

```java
Optional<String> first = Stream.of("a", "b", "c")
    .filter(s -> s.length() > 5)
    .findFirst();  // Optional.empty()

// Chain operations on Optional
first
    .map(String::toUpperCase)
    .ifPresent(System.out::println);

// Or use orElse
String result = first.orElse("default");
String orThrow = first.orElseThrow(() -> new Exception("Not found"));
```

## Parallel Streams

Parallel streams distribute work across multiple threads using `ForkJoinPool.commonPool()`:

```java
List<Integer> nums = List.of(1, 2, 3, 4, 5);

// Sequential
int sum = nums.stream()
    .filter(x -> x > 2)
    .map(x -> x * 2)
    .reduce(0, Integer::sum);

// Parallel
int parallelSum = nums.parallelStream()
    .filter(x -> x > 2)
    .map(x -> x * 2)
    .reduce(0, Integer::sum);
```

### Parallelization Overhead

Parallel streams introduce overhead (thread forking, joining). Benefit only if:
- Data size is large (threshold ~10,000 elements)
- Operations are expensive (not trivial like `x + 1`)
- Data source is efficient to split (arrays, lists; not linked lists)

```java
// Inefficient parallelization
Stream.iterate(1, x -> x + 1)
    .limit(1000)
    .parallel()  // Bad: iterate() is hard to split
    .map(x -> x * 2);

// Better
IntStream.range(1, 1001)
    .parallel()  // Good: range is easily split
    .map(x -> x * 2);
```

### Ordering in Parallel Streams

`parallel()` may reorder elements. Use `forEachOrdered` to preserve order:

```java
nums.parallelStream()
    .map(x -> x * 2)
    .forEachOrdered(System.out::println);  // Preserves encounter order
```

Collectors like `toList()` preserve encounter order even in parallel streams (at a cost).

## Primitive Streams

`IntStream`, `LongStream`, `DoubleStream` avoid boxing overhead:

```java
// Stream<Integer> boxes each element
Stream.of(1, 2, 3)
    .map(x -> x * 2)
    // Each element is Integer object

// IntStream avoids boxing
IntStream.of(1, 2, 3)
    .map(x -> x * 2)
    .sum();  // Primitive reduction

// Range
IntStream.rangeClosed(1, 100).sum();  // Sum 1 to 100

// Unboxing with boxed()
var list = IntStream.of(1, 2, 3)
    .boxed()
    .collect(Collectors.toList());  // Stream<Integer>
```

## Stream.of vs Collections

```java
// Stream.of (varargs, fixed source)
Stream.of(1, 2, 3);

// Collection.stream() (most common)
List.of(1, 2, 3).stream();

// Custom source
Stream.generate(() -> Math.random()).limit(5);

// File I/O
Files.lines(Paths.get("file.txt"));

// Infinite streams
Stream.iterate(1, x -> x + 1).limit(10);
```

## See Also

- Lazy evaluation and functional composition
- Parallel algorithms and ForkJoinPool
- Collectors and reduction strategies
- Optional and null handling
- Streams in Java NIO (Files.lines)