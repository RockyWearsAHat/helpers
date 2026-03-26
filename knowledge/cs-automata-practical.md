# Automata in Practice — Regex Compilation, Lexing, State Machines & Workflow Engines

## Overview

Finite automata underpin many practical systems: regex engines compile patterns to state machines, lexers tokenize source code, protocol handlers manage state transitions, UI frameworks use statecharts, and workflow engines orchestrate multi-step processes. Understanding automata helps diagnose performance issues, reason about correctness, and choose between implementation strategies.

The key trade-off: **NFA vs DFA**. NFAs are compact and easy to generate; DFAs are faster to execute but larger. Most practical systems optimize this trade-off based on workload (compile-time cost vs matching speed).

## Regex Compilation: NFA to DFA

### NFA: Natural Representation

Regular expressions compile naturally to **nondeterministic finite automata (NFA)**. Thompson's construction builds an NFA directly from the regex syntax tree:
- **Literal `a`**: two states linked by an `a` transition
- **Concatenation `AB`**: connect A's accept state to B's start state
- **Alternation `A|B`**: new start state with epsilon transitions to both, merge accept states
- **Kleene star `A*`**: epsilon loop back from accept, epsilon bypass

Result: O(m) states for regex length m, O(m) transitions. Epsilon transitions require epsilon closure computation (subset construction).

### DFA: Fast Execution

**Deterministic finite automata** eliminate epsilon transitions: each state + symbol has exactly one target. Building a DFA requires the **subset construction** (powerset):
1. Start with the epsilon closure of the NFA start state
2. For each state set and input symbol, compute the epsilon closure of all reachable states
3. Repeat until no new states appear

Result: O(2^m) states in worst case (exponential blowup), but queries run in O(n) time with no backtracking.

### Engine Choice

- **Backtracking (NFA)**: Perl, Python, Java, JavaScript, Ruby, .NET. Supports backreferences and lookaround. Slower on pathological inputs (catastrophic backtracking). Examples: `(a+)+b` matching a string of a's with no b at the end tries exponentially many branches.
- **DFA (linear time)**: grep, awk, RE2 (Go). No backreferences. O(n) matching time guaranteed. Memory-intensive for complex patterns.

## Lexer Generation

### Tokenization as Automata

Lexers partition input into tokens by matching categories (identifiers, keywords, operators). The automaton's accept states are labeled with token types. A **longest-match** strategy accepts the longest-running sequence before returning control; lookahead (peek) handles ambiguity (e.g., `if` vs `identifier`).

Traditional lexer generators (lex, flex) produce C code from regex rules:
```
[a-zA-Z_]\w*    { return IDENTIFIER; }
[0-9]+          { return NUMBER; }
"+"             { return PLUS; }
\s+             { /* skip */ }
```

Generator converts each rule to an NFA, merges them, and produces a DFA table. Lexer execution is a state machine loop: read character, follow transition, check for token.

### Practical Issues

- **Keyword vs identifier**: Most lexers use a **reserved-word table**. Match identifiers, then check if they're keywords.
- **Longest-match ambiguity**: `<=` vs `<` and `=`. Standard: match greedily, backtrack if needed.
- **Context-sensitive tokens**: Some languages require lookahead or context. Example: Python indentation, C typedef parsing. Often handled outside the automaton with state flags.
- **Error recovery**: Real lexers emit a token on error and skip to a sync point, enabling error reporting without aborting.

## Protocol State Machines

### TCP Handshake & Connection States

TCP defines a finite state machine with 11 states:
```
CLOSED → SYN_SENT (active open)
       ↓
SYN_RCVD → ESTABLISHED (passive open, 3-way handshake)
       ↓
ESTABLISHED → FIN_WAIT_1 (active close)
           ↓
FIN_WAIT_2 → TIME_WAIT → CLOSED
           ↓
CLOSE_WAIT → LAST_ACK → CLOSED
```

Events: SYN, ACK, FIN, RST, timeout. Each (state, event) pair has a handler: update state, send segments, set timers.

**Key insight**: TCP's complexity comes not from the automaton (small) but from **timing**, **retransmission**, **window management**, and **concurrent connections**. The state machine is deterministic; the system is not.

### Protocol Implementation Pattern

```
protocol_state_machine {
  state_t current_state = INITIAL;
  
  on_event(event) {
    transition_t tx = transitions[current_state][event];
    if (tx.valid) {
      tx.action(*this);
      current_state = tx.next_state;
    } else {
      error("Unexpected event");
    }
  }
}
```

Transition table can be explicit (2D array), or implicit (switch statements). Explicit is faster; implicit is more readable.

## UI State Machines: XState & Statecharts

### Beyond Finite Automata: Hierarchical State Machines

**Statecharts** (Harel, 1987) extend finite automata with:
- **Hierarchy**: states contain substates (nesting)
- **Parallelism**: concurrent regions within a state
- **History**: shortcuts to previous substates
- **Guards**: conditional transitions

This models complex UI flows without combinatorial explosion of states.

### XState: Practical Statechart Library

XState (JavaScript) implements statecharts for React, Vue, and vanilla JavaScript:

```js
const toggleMachine = createMachine({
  initial: 'off',
  states: {
    off: {
      on: { TOGGLE: 'on' }
    },
    on: {
      on: { TOGGLE: 'off' }
    }
  }
});
```

**Hierarchical example** (traffic light with blink mode):
```js
states: {
  light: {
    initial: 'red',
    states: {
      red: { on: { TIMER: 'green' } },
      green: { on: { TIMER: 'yellow' } },
      yellow: { on: { TIMER: 'red' } }
    }
  },
  maintenance: {
    on: { REPAIR: 'light' }
  }
}
```

**Entry/exit actions**: Run code on state entry/exit:
```js
red: {
  entry: 'startTimer',
  exit: 'stopTimer',
  on: { TIMER: 'green' }
}
```

**Guarded transitions** (conditional):
```js
on: {
  SUBMIT: {
    target: 'success',
    cond: (ctx) => ctx.data.isValid
  }
}
```

### Benefits for UI

- **Visualization**: machines are visualizable, helping team understanding
- **Testing**: enumerate all (state, input) pairs; ensure coverage
- **Type safety**: TypeScript support (XState v5+) prevents invalid transitions
- **Explicitness**: impossible states become apparent; bugs surface during design

## Workflow Engines: Automata + Persistence

### Orchestration Layers

Workflow engines layer automata with:
- **Persistence**: save state across restarts
- **Concurrency**: spawn parallel tasks, wait for joins
- **Retry**: transient failures + exponential backoff
- **Visibility**: logs, dashboards, audit trails

Examples: Apache Airflow (DAGs), Temporal (durable workflows), AWS Step Functions.

### State Representation

A workflow instance is a tuple:
```
(workflow_id, version, current_state, context, execution_log)
```

On event (task completion, timeout), the engine:
1. Load the instance from persistent storage
2. Look up transitions for (state, event)
3. Execute actions (side effects, spawn tasks)
4. Update state and persist
5. Emit events (webhooks, logs)

**Critical property**: **idempotency**. If step N fails and retries, repeated execution must not double-execute non-idempotent side effects. Workflows use **deduplication keys** and **exactly-once semantics**.

## Statechart Notation: Harel Diagrams

### UML State Machine Diagram Conventions

- **States**: rounded rectangles
- **Transitions**: arrows labeled `event [guard] / action`
- **Initial state**: filled circle with arrow
- **Final state**: concentric circles
- **Composite states**: rectangles with nested diagram
- **Parallel regions**: dashed vertical line separating concurrent branches
- **History**: circle with H (shallow) or H* (deep)

Example: order workflow
```
[Initial] → CheckInventory
              ↓ [in stock] → WaitForPayment
                              ↓ [paid] → Ship
                                          ↓ → [Delivered]
              ↓ [out of stock] → Backorder
                                   ↓ [restocked] → WaitForPayment
              ↓ [error] → [Failed]
```

Notation enables non-programmers to collaborate on behavior design.

## Performance & Trade-offs

### Regex: Catastrophic Backtracking

Backtracking engines risk exponential time on adversarial inputs:
```
(a+)+b    matching "aaaaaaaaaaaaaaaaab"  → 2^20 paths
```

- **DFA avoids this** but requires exponential space upfront
- **Hybrid approach**: NFA with memoization (implicit DFA table) or timeout backoff

### Lexer: Table Size vs Complexity

- **Large automated merge** of many rules → large DFA table (slow startup, fast matching)
- **Hand-written cascading lexer** → smaller code, more maintainable, sometimes faster in practice

### Statechart: Exponential Blow-up

Concurrent regions multiply state count. A machine with 3 independent 4-state regions has 64 reachable states. Management strategies:
- Decompose into smaller machines, communicate via events
- Use orthogonal regions only where needed
- Limit nesting depth

## See Also

[math-automata-computability](math-automata-computability.md), [regex-patterns](regex-patterns.md), [compiler-design-frontend](compiler-design-frontend.md), [architecture-state-machines](architecture-state-machines.md)