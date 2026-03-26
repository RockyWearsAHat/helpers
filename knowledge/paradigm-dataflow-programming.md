# Dataflow Programming — Computation as Flow of Data

## Overview

**Dataflow programming** models computation as data moving through a network of operators. Rather than imperative instruction sequences, you define how data transforms as it flows.

```
Input Data → [Transform A] → [Transform B] ⊕ [Transform C] → Output Data
                   ↓               ↓                  ↓
                 internal state tracking through the pipeline
```

This is fundamentally different from imperative programming (do X, then Y, then Z) or functional programming (functions, composition). Dataflow is orthogonal to both; it's about **topology** — how components connect and how data moves.

## Flow-Based Programming (FBP)

**Flow-Based Programming** was formalized by J. Paul Morrison starting in the 1970s. It's a conceptual framework with specific properties:

### Core Components

1. **Components/Processes**: black boxes that transform input to output
2. **Connections/Edges**: conduits between components; **typed** and **bounded**
3. **Information Packets (IPs)**: discrete units of data flowing through connections
4. **Ports**: named inputs/outputs on a component

```
    Component A
   ─────────────
   │ in  │ out │
   └─────────────┘
      ↓          ↓
   [data packet] → (transformation) → [result packet]
      ↑                                   ↓
   Connection (bounded buffer)    [out port]
```

### Key Properties (Morrison's Definition)

- **Asynchronous**: Components run independently; connections provide implicit buffering and flow control
- **Bounded**: Each connection has a finite buffer size (prevents unbounded memory use)
- **Deterministic**: If all inputs are the same and components are deterministic, output is the same (no races)
- **Black-box semantics**: Components don't know who sends/receives data; only read/write ports
- **Named ports**: Multiple inputs/outputs on a single component; routed by name
- **IP lifetimes**: Data packets have defined creation, processing, and termination

### Advantages

- **Composability**: Build complex systems from simple pieces
- **Visibility**: Data flow is explicit; easier to trace state changes
- **Reusability**: Components work in any composition (if contracts are met)
- **Parallelism**: Pipeline stages can run concurrently; scheduling is automatic
- **Debugging**: Visual representation of data flow aids understanding

### Disadvantages

- **Overhead**: Buffering, routing, IP allocation adds cost
- **Latency**: Bounded buffers can cause backpressure blocking
- **Complexity for tight loops**: Not efficient for very tight, synchronous inner loops
- **Testing**: Isolated components are easy; testing integration requires coordination

## Visual Dataflow Systems

### LabVIEW

**LabVIEW** (Laboratory Virtual Instrument Engineering Workbench) is the industrial standard for visual dataflow. Scientists and engineers define logic visually:

```
Input → [Function] → [Function] → Conditional → Output
          ↓            ↓              ↓
       (wires connect data flow)
```

- **No text code** (though integration with text exists)
- **Nodes** represent operations (math, I/O, control flow)
- **Wires** represent data dependencies and flow
- **Data types** are color-coded on wires
- **Execution** follows the graph: a node runs when all inputs are ready

LabVIEW dominates instrumentation, test automation, and instrument control.

### Unreal Engine Blueprints

**Blueprints** in Unreal Engine use visual dataflow for game logic:

```
Event → [Branch] ⟳ [Get Actor Location] → [Print String] → Output
         ↓
      (conditional flow)
```

- **Nodes** represent game logic (events, actors, properties, functions)
- **Wires** represent both data flow and execution order
- **Execution pins** (different from data pins): control which branches execute
- **Event-driven**: nodes often react to game events (collision, input, timer)

### Node-RED

**Node-RED** is a JavaScript-based visual interface for wiring IoT and automation:

```
[MQTT In] → [JSON] → [Condition] → [MQTT Out]
               ↓         ↓
            (data flow)
```

- **Runs on Node.js**: components are JavaScript functions
- **Palette**: library of pre-built nodes (HTTP, database, logic, etc.)
- **Web UI**: design pipelines in a browser
- **Flow storage**: flows are JSON, can be version-controlled

Widely used in IoT, home automation (Home Assistant), and cloud integrations.

## Reactive Streams as Dataflow

**Reactive streams** (RxJS, Akka Streams, Reactor, etc.) implement dataflow concepts in code:

```javascript
// RxJS: dataflow composition
source$
    .pipe(
        filter(x => x > 10),
        map(x => x * 2),
        tap(x => console.log(x)),
        take(5)
    )
    .subscribe(onNext, onError, onComplete)
```

Key properties:
- **Streams are IPs**: data packets flowing through the pipeline
- **Operators are components**: `filter`, `map`, `reduce` transform data
- **Composition**: `pipe` chains operators (similar to Unix pipes)
- **Backpressure**: subscribers signal demand; producers throttle if needed
- **Time and async**: operators like `debounce`, `throttle`, `retry`, `timeout` handle async concerns

Reactive streams bridge imperative/functional and dataflow worlds. Code looks functional (chainable operators), but semantics are dataflow (explicit data flow topology).

## TensorFlow and PyTorch: Computation Graphs

**Deep learning frameworks** use dataflow for machine learning:

### TensorFlow Graph

```python
# Build a computation graph (TF 1.x style)
import tensorflow as tf

x = tf.placeholder(tf.float32, shape=[None, 784])
W = tf.Variable(tf.random.normal([784, 10]))
b = tf.Variable(tf.zeros([10]))

logits = tf.matmul(x, W) + b
loss = tf.nn.softmax_cross_entropy_with_logits(labels=y, logits=logits)

# Graph is now defined (dataflow topology)
# Execution requires a session:
with tf.Session() as sess:
    sess.run(loss)  # evaluate the graph
```

Nodes in the graph are operations (matrix multiply, add, etc.). Edges are tensors (multi-dimensional arrays). The graph is a DAG (directed acyclic graph) of computations.

### PyTorch Dynamic Graphs

**PyTorch** builds graphs dynamically at runtime (eager execution):

```python
import torch

x = torch.zeros(10, 784)
W = torch.randn(784, 10, requires_grad=True)
b = torch.zeros(10, requires_grad=True)

# Graph is built as you compute (no separate compilation)
logits = x @ W + b
loss = F.softmax(logits, dim=1)

# Backward pass traces the graph (automatic differentiation)
loss.backward()
```

The graph isn't predefined; it emerges from execution. This is more Pythonic but loses static optimization opportunities.

## Kahn Process Networks

**Kahn Process Networks (KPN)** are a formal model of dataflow:

1. **Processes communicate via FIFO queues**
2. **Reads block until data is available** (unbuffered reads)
3. **System is deterministic**: output depends only on inputs, not timing
4. **Completeness condition**: if processes are well-formed, network always terminates

```
Process P1: reads x, writes x*2 to y
Process P2: reads y, writes y+1 to z

KPN ensures: order of writes doesn't affect final result
```

KPN properties:
- **Determinate**: same input always yields same output
- **Order-independent**: data can be buffered, reordered; result is unchanged if computation is well-formed
- **Composite**: KPNs can be nested

### Use

KPN is more of a **verification tool** than a programming model. Use it to **reason** about dataflow systems, prove correctness, detect deadlocks.

## Synchronous Dataflow

**Synchronous dataflow (SDF)** adds a scheduling constraint: each operation **consumes and produces a fixed number of tokens** per execution.

```
Component A: consumes 1 token, produces 2 tokens
Component B: consumes 2 tokens, produces 1 token

A → B → A (cycle)

Schedule:
  A fires once: 1 input, 2 outputs
  B fires once: 2 inputs (from A), 1 output
  Network is balanced (no deadlock)
```

### Advantages

- **Schedulable**: compiler can determine firing order statically
- **Memory bounded**: if acyclic or deadlock-free, buffer sizes are finite and computable
- **Declarative**: focus on topology, not implementation

### Limitations

- **Rigid**: fixed production/consumption rules are restrictive
- **Dynamic rates**: if rates vary per invocation, SDF fails (need **dynamic dataflow**)

### Tools

- **Ptolemy** (UC Berkeley): framework for experimenting with dataflow models
- **SDFG-based compilers**: some HPC frameworks use SDF for performance prediction

## Apache Beam: Modern Dataflow Programming

**Apache Beam** brings dataflow to distributed data processing:

```java
// Pipeline: define topology
Pipeline pipeline = Pipeline.create();

PCollection<String> lines = pipeline.apply(
    TextIO.read().from("input.txt")
);

PCollection<String> output = lines
    .apply(Filter.by(s -> s.length() > 5))
    .apply(MapElements.into(strings()).via(String::toUpperCase));

output.apply(TextIO.write().to("output.txt"));

pipeline.run();  // Execute on Spark, Flink, or Dataflow
```

- **Unified API**: batch and streaming share the same API
- **Windowing**: temporal semantics for streaming (tumble, slide, session)
- **Distributed execution**: runners (Spark, Flink, Cloud Dataflow) handle parallelism
- **Declarative**: pipeline is a DAG of transforms

Beam is practical for ETL, data processing at scale.

## Spreadsheets as Dataflow

**Spreadsheets** are dataflow systems:

```
A1: 10
B1: =A1 * 2
C1: =B1 + 5
```

- **Cells are nodes** (components)
- **Formulas are edges** (data dependencies)
- **Automatic evaluation**: when A1 changes, B1 and C1 recompute
- **Lazy evaluation** (rows with no change don't recompute)

Spreadsheets are:
- **Accessible**: non-programmers can compose logic
- **Visual**: data flow is visible to users
- **Incremental**: only affected cells recompute
- **Problematic**: formulas can be cryptic, versions/dependencies are implicit

Spreadsheet dataflow inspired modern reactive systems (reactive programming libraries often promise "spreadsheet-like" reactivity).

## Comparison: Dataflow Models

| Model | Domain | Execution | Determinism | Use | Adoption |
|-------|--------|-----------|-------------|-----|----------|
| **FBP** | Integration, general | Async, buffered | Yes | Middleware, microservices | Niche (commercial tools) |
| **Visual (LabVIEW)** | Instrumentation | Synchronous + async | Depends on nodes | Test, measurement, control | High (industrial) |
| **Reactive Streams** | Application | Async, backpressured | Operator-dependent | Web services, real-time | High (modern frameworks) |
| **TensorFlow Graphs** | ML | Batch or streaming | Data-dependent | Training, inference | Very high |
| **KPN** | Verification | Async | Yes | Formal analysis | Academic |
| **SDF** | Signal processing, HPC | Static schedule | Yes | Embedded, DSP | Moderate (research + some industry) |
| **Apache Beam** | Data processing | Distributed | Operator-dependent | ETL, analytics | Growing (cloud platforms) |
| **Spreadsheets** | Lightweight automation | Lazy, incremental | Formula-dependent | Non-programmers, rapid prototyping | Ubiquitous |

## Mental Model: Dataflow vs. Imperative

### Imperative (Procedural)

```python
result = []
for item in input_list:
    processed = process(item)
    if valid(processed):
        result.append(transformed(processed))
return result
```

Think: **sequence of operations on the call stack**.

### Dataflow

```
Input → [Process] → [Filter] → [Transform] → Output
          (stateful,      (stateful,          (stateful,
           sequential)     sequential)         sequential)
```

Think: **topology of independent components; data flows through**.

### Functional

```python
return [transformed(p) for p in (filter(valid, map(process, input_list)))]
```

Think: **function composition, nested transformations**.

## When to Use Dataflow

### Good Fit

- **Data pipelines**: ETL, streaming, data science
- **Signal processing**: audio, video, sensor data
- **Instrumentation**: lab instruments, test automation
- **Integration**: routing data between systems
- **Machine learning**: computation graphs
- **Reactive systems**: event-driven, real-time applications

### Poor Fit

- **Tight loops**: numeric computation (e.g., inner loops in matrix math)
- **Complex control flow**: deeply nested conditionals, complex branching
- **Highly stateful systems**: state machines with many interdependencies
- **Procedural tasks**: if the algorithm is inherently sequential, imperative is clearer

## See Also

- [Reactive Programming — Streams, Observables & Dataflow](paradigm-reactive-programming.md)
- [Architecture Event-Driven Systems](architecture-event-driven.md)
- [Data Engineering — Streaming Pipelines](data-engineering-streaming.md)
- [Concurrency & Parallelism Patterns](concurrency-patterns.md)