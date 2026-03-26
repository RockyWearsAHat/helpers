# Formal Methods — Specification, Verification, and Model Checking

## Overview: The Formal Methods Spectrum

Formal methods apply mathematical techniques to prove (or disprove) that systems meet specifications. The spectrum ranges from light (static analysis) to heavy (interactive theorem proving), each trading cost against assurance.

| Technique             | Automation | Proof Completeness | Usage Cost | Assurance            |
| --------------------- | ---------- | ------------------ | ---------- | -------------------- |
| Model checking        | Full       | Complete (finite)  | Moderate   | Exhaustive (bounded) |
| SAT/SMT solving       | Full       | Complete           | Moderate   | Decidable formula    |
| Theorem proving       | Partial    | Incomplete         | Very high  | Anything provable    |
| Runtime monitoring    | Full       | None               | Low        | Empirical            |

The key tradeoff: automation is powerful but bounded; theorem proving is expressive but requires human guidance.

## TLA+ — Temporal Logic of Actions

Leslie Lamport's TLA+ is a language for specifying concurrent and distributed systems using mathematical logic. TLA+ separates **specification** (what the system should do) from **implementation** (how it does it), enabling refinement verification.

### Core Concepts

**States and actions:** A system is a set of states, and specification defines initial states and transitions (actions) that change state.

```
Naturals == {0, 1, 2, ...}
VARIABLE x, y

Init == x = 0 ∧ y = 0

Increment == x' = x + 1  ∧ UNCHANGED y
Reset    == x' = 0 ∧ y' = (y + 1) * x

Next == Increment ∨ Reset
Spec == Init ∧ □[Next]_x,y
```

The `□` (always) operator means Next or an unchanged-state stutter occurs at every step. The prime notation M' refers to the next-state value of variable M.

**Temporal properties:** TLA+ properties are assertions about entire execution traces, not individual states.

```
Safety property (bad thing never happens):
  □(¬(resourceA_locked ∧ resourceB_locked))

Liveness property (good thing eventually happens):
  □(request_pending → ◇ request_granted)
```

### Specification Levels and Refinement

TLA+ separates abstraction levels. A high-level spec might say "respond to requests"; a mid-level spec adds "using a queue"; a low-level spec is the actual algorithm. Each level refines the previous: if implementation satisfies low-level spec and low-level refines high-level, then implementation satisfies high-level.

**Weakening through refinement:** Implementations can be allowed to diverge from specs in controlled ways (hiding internal steps, data refinement). A spec might expose steps that an implementation never executes; that's usually fine as long as observable behavior matches.

### Model Checking TLA+

The TLA+ model checker, TLC, exhaustively explores the state graph to verify properties:

```
tlc spec.tla -config spec.cfg
```

TLC enumerates initial states, applies actions, discovers new states, and checks all safety and liveness properties. For finite state spaces (or bounded instantiations), it gives definitive yes/no answers. It also produces counterexample traces if properties fail.

**Limitation:** The state space can explode. A system with 10 variables each with 100 values has 10^20 states. TLA+ is best used for abstract specs where state spaces are manageable (thousands to millions of states, not billions).

**Success story:** AWS used TLA+ to verify core systems in DynamoDB, Elasticache, and auto-scaling controllers. Lamport reports that formal specifications often uncover design bugs before implementation, even without running TLC.

## Alloy — Relational Logic and Finite Model Finding

Alloy (Jackson, MIT) is a lightweight formal method using first-order relational logic. Instead of linear temporal logic, Alloy reasons about static relations and finite instances.

```
sig Person { friends: set Person }
sig Party { guests: set Person }
fact {
  all p: Person | p !in p.friends        -- no self-loops
  all p1, p2: Person | p1 in p2.friends  -- symmetry
    implies p2 in p1.friends
}

pred Invited[p: Person, party: Party] {
  p in party.guests
}

check { all p: Party, q: Party | (some p.guests & q.guests) }
  for 10 Person, 3 Party
```

**Relation-based reasoning:** Everything is a set or relation. The `guests` field is a relation mapping parties to persons. Logical constraints (facts) restrict valid models. Predicates define properties.

**Bounded model finding:** Instead of formal proof, Alloy searches for counterexamples within a bounded scope (e.g., 10 Person atoms, 3 Party atoms). If no counterexample exists in that scope, the property likely holds more generally (though not proven).

Alloy excels at discovering subtle bugs in data structures and API contracts. It's easier to learn than TLA+ and runs faster on small systems.

## Model Checkers: SPIN and nuSMV

Professional model checkers automate exhaustive verification of concurrent systems.

### SPIN

SPIN (Holzmann, Bell Labs) verifies systems specified in Promela (Process and Communication Language). It finds deadlocks, liveness violations, and user-defined properties.

```
proctype sender(chan out) {
  do
    :: out!msg -> skip
  od
}

proctype receiver(chan in) {
  do
    :: in?msg -> printf("received\n")
  od
}

init {
  chan c = [2] of { byte };
  run sender(c);
  run receiver(c);
}
```

SPIN builds a product automaton combining all processes and a property automaton (what you're checking for), then explores reachable states. It's been widely used in protocol verification (telecom, avionics) and catches race conditions and deadlocks that testing misses.

### nuSMV

nuSMV (Cimatti et al., FBK) verifies finite state systems using symbolic model checking. It reads SMV input (state machine notation) and checks CTL properties via BDD-based algorithms.

```
VAR state : {idle, active, error};
VAR counter : 0..10;

ASSIGN
  init(state) := idle;
  next(state) := case
    state = idle & counter = 5 : active;
    state = active : error;
    TRUE : state;
  esac;

CTLSPEC AG(state != error);
```

nuSMV scales better than SPIN for large state spaces by using symbolic representations (BDDs) instead of explicit enumeration. It's been applied to hardware verification and safety-critical systems.

## Theorem Provers: Coq, Lean, Isabelle/HOL

When state space is infinite or verification needs inductive argument, theorem provers handle it via interactive proof construction.

### Coq

Coq uses constructive type theory (covered in math-type-theory.md). Specifications and proofs are both code. The Curry-Howard correspondence means correct proofs generate correct programs.

```
Definition double (n : nat) : nat := n + n.

Lemma double_correct : forall n : nat, double (n + n) = n + n + n + n.
Proof.
  intro n.
  unfold double.
  omega.
Qed.
```

Coq has been used to verify: CompCert (a C compiler), cryptographic protocols, and mathematical theorems (Four Color Theorem, Odd Order Theorem).

### Lean 4

Lean is designed for modern interactive theorem proving with better ergonomics than Coq. It supports tactic-based proof development and has an active community of mathematicians.

```
def double (n : Nat) : Nat := n + n

theorem double_eq_add (n : Nat) : double n = n + n := by rfl

theorem even_sum : ∀ n, Even (n + n) := by
  intro n
  use n
  ring
```

### Isabelle/HOL

Isabelle is a generic proof assistant; Isabelle/HOL instantiates it with higher-order logic. It's particularly strong for hardware and software verification with a pragmatic orientation.

The theorem provers share a philosophy: proofs are objects. Verifying a system means constructing a proof object showing the system meets its spec. The prover checks proof validity mechanically.

## Specification vs. Verification

A common confusion: specification and verification are different activities.

**Specification:** Formally describe what the system should do. A spec might say "respond to every request within 100ms" or "never deadlock." The spec itself might be buggy (wrong requirements), but it's unambiguous.

**Verification:** Check that an implementation satisfies the spec. A verified implementation is proven correct relative to the spec, not relative to reality. If the spec is wrong, the implementation is correctly wrong.

**The rub:** Getting the spec right is often harder than writing the code. Formal methods force clarity, which often reveals design flaws before implementation. But they don't eliminate the need for domain expertise and simulation.

## Industrial Adoption

**AWS:** Uses TLA+ for designing distributed services. DynamoDB and other core services are formally specified and checked. Cost: high upfront, paid back in confidence and reduced bugs. TLA+ specs are maintained as living documents.

**Microsoft:** Used formal methods for Windows kernel drivers and Xbox security. Terminator (automated termination verification) proved liveness in interrupt handlers.

**Avionics & Automotive:** Airbus (A380 flight control), Boeing, and major automotive suppliers apply model checking and theorem proving to safety-critical systems. Industry standards (DO-178C) acknowledge formal methods as a credible verification technique.

**Challenges in industry:** Formal methods require expertise, slow time-to-market initially, and require disciplined specification. Most teams lack trained personnel. Successful adoption requires commitment from leadership, not just tooling.

## Cross-References

See also: formal-verification.md, distributed-consensus.md, architecture-state-machines.md, math-type-theory.md.