# State Machines in Software

## Finite State Machine Fundamentals

A state machine is a model with a finite number of states, transitions between them triggered by events, and optionally guards (conditions) and actions (side effects).

### Core Concepts

| Concept        | Definition                                   | Example                               |
| -------------- | -------------------------------------------- | ------------------------------------- |
| **State**      | A distinct mode the system can be in         | `idle`, `loading`, `error`, `success` |
| **Event**      | Something that happened (trigger)            | `FETCH`, `RESOLVE`, `REJECT`, `RETRY` |
| **Transition** | Moving from one state to another             | `idle` + `FETCH` → `loading`          |
| **Guard**      | Condition that must be true for a transition | `canRetry` (retries < 3)              |
| **Action**     | Side effect executed during a transition     | `sendRequest`, `logError`             |

### State Transition Table

| Current State | Event   | Guard     | Next State | Action      |
| ------------- | ------- | --------- | ---------- | ----------- |
| idle          | FETCH   | —         | loading    | sendRequest |
| loading       | RESOLVE | —         | success    | cacheResult |
| loading       | REJECT  | canRetry  | error      | logError    |
| loading       | REJECT  | !canRetry | failure    | notifyUser  |
| error         | RETRY   | —         | loading    | sendRequest |
| success       | RESET   | —         | idle       | clearCache  |

## Why State Machines Over Boolean Flags

### The Boolean Problem

```javascript
// Boolean flags approach — quickly becomes unmaintainable
let isLoading = false;
let isError = false;
let isSuccess = false;
let data = null;

// What does isLoading=true AND isError=true mean?
// How many possible states? 2^3 = 8, but only 4 are valid.
// Impossible states are representable and cause bugs.
```

### The State Machine Solution

```javascript
// State machine — impossible states are impossible
type State = "idle" | "loading" | "success" | "error";
let state: State = "idle";

// Transitions are explicit. No ambiguous combinations.
// "loading AND error" is unrepresentable.
```

**Key insight**: With N boolean flags, you have 2^N possible states, most of which are invalid. A state machine has exactly the valid states.

## Statecharts (Extended State Machines)

David Harel's statecharts add features to basic FSMs:

### Hierarchical (Nested) States

States can contain sub-states. The parent state encapsulates shared behavior:

```
┌─ Active ──────────────────┐
│  ┌─ Playing ─┐ ┌─ Paused ┐│
│  │           │ │         ││
│  └───────────┘ └─────────┘│
│      PAUSE →    ← RESUME  │
└───────────────────────────┘
         │ STOP → Idle
```

`Active` handles the `STOP` event for both `Playing` and `Paused`. No duplication.

### Parallel (Orthogonal) States

Multiple sub-states active simultaneously:

```
┌─ Player ─────────────────────────┐
│  ┌─ Playback ──┐ ┌─ Volume ───┐  │
│  │ playing     │ │ unmuted    │  │
│  │ paused      │ │ muted      │  │
│  └─────────────┘ └────────────┘  │
└──────────────────────────────────┘
```

Playback and Volume are independent sub-machines running in parallel.

### History States

Remember which sub-state was active when the parent state was exited. On re-entry, resume from the remembered sub-state instead of the initial default.

## XState

The standard state machine library for JavaScript/TypeScript.

### Creating a Machine

```typescript
import { createMachine, assign } from "xstate";

const fetchMachine = createMachine({
  id: "fetch",
  initial: "idle",
  context: {
    retries: 0,
    data: null,
    error: null,
  },
  states: {
    idle: {
      on: { FETCH: "loading" },
    },
    loading: {
      invoke: {
        src: "fetchData",
        onDone: {
          target: "success",
          actions: assign({ data: ({ event }) => event.output }),
        },
        onError: [
          {
            target: "error",
            guard: ({ context }) => context.retries < 3,
            actions: assign({
              retries: ({ context }) => context.retries + 1,
              error: ({ event }) => event.error,
            }),
          },
          {
            target: "failure",
          },
        ],
      },
    },
    success: {
      on: { RESET: "idle" },
    },
    error: {
      on: {
        RETRY: "loading",
      },
    },
    failure: {
      type: "final",
    },
  },
});
```

### XState Core Concepts

| Concept    | XState API            | Purpose                                          |
| ---------- | --------------------- | ------------------------------------------------ |
| Context    | `context`, `assign()` | Extended state (data alongside finite state)     |
| Guards     | `guard`               | Conditional transitions                          |
| Actions    | `actions`             | Side effects on transition (assign, send, raise) |
| Services   | `invoke` (actors)     | Async operations (promises, callbacks, machines) |
| Entry/Exit | `entry`, `exit`       | Actions run when entering/leaving a state        |

### TypeScript Types

```typescript
import { setup } from "xstate";

const machine = setup({
  types: {
    context: {} as { count: number; user: User | null },
    events: {} as
      | { type: "INCREMENT" }
      | { type: "DECREMENT" }
      | { type: "SET_USER"; user: User },
  },
  actions: {
    increment: assign({ count: ({ context }) => context.count + 1 }),
    setUser: assign({ user: ({ event }) => event.user }),
  },
  guards: {
    isPositive: ({ context }) => context.count > 0,
  },
}).createMachine({
  // ... fully typed machine definition
});
```

## Common State Machine Patterns

### Order Lifecycle

```
draft → submitted → [payment]
                      ├── paid → [fulfillment]
                      │            ├── shipped → delivered
                      │            └── cancelled (+ refund)
                      └── payment_failed → draft
```

```typescript
const orderMachine = createMachine({
  id: "order",
  initial: "draft",
  states: {
    draft: {
      on: { SUBMIT: "submitted" },
    },
    submitted: {
      on: {
        PAYMENT_SUCCESS: "paid",
        PAYMENT_FAILED: "draft",
      },
    },
    paid: {
      on: {
        SHIP: "shipped",
        CANCEL: "cancelled",
      },
    },
    shipped: {
      on: { DELIVER: "delivered" },
    },
    delivered: { type: "final" },
    cancelled: { type: "final" },
  },
});
```

### Authentication Flow

```
unauthenticated → authenticating → [result]
                                     ├── authenticated → [session]
                                     │                    ├── refreshing → authenticated
                                     │                    └── LOGOUT → unauthenticated
                                     └── error → unauthenticated (RETRY)
```

### Traffic Light

```typescript
const trafficLightMachine = createMachine({
  id: "trafficLight",
  initial: "green",
  states: {
    green: {
      after: { 30000: "yellow" },
    },
    yellow: {
      after: { 5000: "red" },
    },
    red: {
      after: { 30000: "green" },
    },
  },
});
```

### Payment Processing

```
idle → initiating → processing → [result]
                                   ├── authorized → capturing → [result]
                                   │                              ├── captured (final)
                                   │                              └── capture_failed → authorized
                                   ├── declined → idle
                                   └── requires_3ds → awaiting_3ds → processing
```

### UI Form Flow

```
editing → validating → [result]
                         ├── valid → submitting → [result]
                         │                          ├── submitted (final)
                         │                          └── submission_error → editing
                         └── invalid → editing (show errors)
```

## Testing State Machines

State machines are highly testable because states and transitions are explicit:

```typescript
import { createActor } from "xstate";

describe("Order Machine", () => {
  it("transitions from draft to submitted on SUBMIT", () => {
    const actor = createActor(orderMachine).start();
    actor.send({ type: "SUBMIT" });
    expect(actor.getSnapshot().value).toBe("submitted");
  });

  it("returns to draft on payment failure", () => {
    const actor = createActor(orderMachine).start();
    actor.send({ type: "SUBMIT" });
    actor.send({ type: "PAYMENT_FAILED" });
    expect(actor.getSnapshot().value).toBe("draft");
  });

  it("does not allow shipping a draft order", () => {
    const actor = createActor(orderMachine).start();
    actor.send({ type: "SHIP" }); // Invalid event in draft state
    expect(actor.getSnapshot().value).toBe("draft"); // Unchanged
  });
});
```

### Model-Based Testing

XState can generate test paths automatically — covering all states and transitions:

```typescript
import { createTestModel } from "@xstate/test";

const testModel = createTestModel(orderMachine);
const testPaths = testModel.getSimplePaths();

testPaths.forEach((path) => {
  it(path.description, async () => {
    await path.test({
      /* test context with assertions per state */
    });
  });
});
```

## Workflow Engines

For long-running, distributed workflows that survive process restarts:

| Engine                 | Language                       | Model                    | Key Feature                                           |
| ---------------------- | ------------------------------ | ------------------------ | ----------------------------------------------------- |
| **Temporal**           | Any (Go SDK, Java, TS, Python) | Code-first workflows     | Durable execution, automatic retries, versioning      |
| **AWS Step Functions** | JSON/YAML (ASL)                | State machine definition | Serverless, visual editor, AWS integration            |
| **Camunda**            | Java, REST                     | BPMN 2.0                 | Industry standard, visual modeling, DMN for decisions |
| **Inngest**            | TypeScript                     | Event-driven functions   | Developer-friendly, step functions, serverless        |

### Temporal Example

```typescript
// Workflow definition
async function orderWorkflow(orderId: string): Promise<void> {
  // Each activity is retried automatically on failure
  await activities.validateOrder(orderId);
  await activities.processPayment(orderId);

  // Durable timer — survives process restarts
  await sleep('3 days');

  // Saga compensation on failure
  try {
    await activities.shipOrder(orderId);
  } catch {
    await activities.refundPayment(orderId);
    throw;
  }
}
```

### Step Functions Example

```json
{
  "StartAt": "ValidateOrder",
  "States": {
    "ValidateOrder": {
      "Type": "Task",
      "Resource": "arn:aws:lambda:...:validate",
      "Next": "ProcessPayment",
      "Catch": [{ "ErrorEquals": ["ValidationError"], "Next": "OrderFailed" }]
    },
    "ProcessPayment": {
      "Type": "Task",
      "Resource": "arn:aws:lambda:...:payment",
      "Next": "ShipOrder"
    },
    "ShipOrder": {
      "Type": "Task",
      "Resource": "arn:aws:lambda:...:ship",
      "End": true
    },
    "OrderFailed": { "Type": "Fail" }
  }
}
```

## When to Use State Machines

### Good Fit

- Complex UI flows (multi-step forms, wizards, editors)
- Business processes with defined lifecycle (orders, payments, subscriptions)
- Protocol implementations (WebSocket state, TCP handshake)
- Game logic (player states, NPC behavior)
- Workflow orchestration (approval chains, deployment pipelines)
- Anything where you draw a state diagram on a whiteboard

### Poor Fit

- Simple boolean toggles (on/off)
- CRUD without lifecycle (basic data entry)
- Stateless request/response handlers
- When the number of states is truly dynamic or unbounded

**Rule of thumb**: If you have more than 3 boolean flags interacting, or if-else chains checking multiple conditions to determine behavior, consider a state machine.
