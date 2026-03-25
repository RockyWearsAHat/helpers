# Prolog Best Practices

## Prolog Philosophy

Prolog is a logic programming language. You declare facts and rules about your domain, then ask queries — the engine finds solutions via unification and backtracking. It excels at symbolic AI, constraint solving, parsing, and expert systems.

- **Declarative**: Describe relationships, not procedures. Prolog finds the answers.
- **Unification**: Pattern matching that works in both directions — input and output are interchangeable.
- **Backtracking**: Automatic search through the solution space. Multiple solutions generated on demand.

## Facts and Rules

```prolog
% Facts (ground truths)
parent(tom, bob).
parent(tom, liz).
parent(bob, ann).
parent(bob, pat).

male(tom).
male(bob).
female(liz).
female(ann).
female(pat).

% Rules (derived knowledge)
father(X, Y) :- parent(X, Y), male(X).
mother(X, Y) :- parent(X, Y), female(X).

sibling(X, Y) :-
    parent(Z, X),
    parent(Z, Y),
    X \= Y.

ancestor(X, Y) :- parent(X, Y).
ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).

% Queries
% ?- father(tom, bob).     → true
% ?- father(X, bob).       → X = tom
% ?- ancestor(tom, ann).   → true
% ?- ancestor(tom, X).     → X = bob ; X = liz ; X = ann ; X = pat
```

## Lists

```prolog
% List notation: [Head | Tail]
% [1, 2, 3] = [1 | [2 | [3 | []]]]

% Length
length([], 0).
length([_ | T], N) :- length(T, N1), N is N1 + 1.

% Member
member(X, [X | _]).
member(X, [_ | T]) :- member(X, T).

% Append
append([], L, L).
append([H | T1], L2, [H | T3]) :- append(T1, L2, T3).

% Reverse
reverse([], []).
reverse([H | T], R) :- reverse(T, RT), append(RT, [H], R).

% Efficient reverse with accumulator
reverse(List, Reversed) :- reverse(List, [], Reversed).
reverse([], Acc, Acc).
reverse([H | T], Acc, R) :- reverse(T, [H | Acc], R).

% List operations (bidirectional!)
% ?- append([1,2], [3,4], X).     → X = [1,2,3,4]
% ?- append(X, [3,4], [1,2,3,4]). → X = [1,2]
% ?- append(X, Y, [1,2,3]).       → X=[], Y=[1,2,3] ; X=[1], Y=[2,3] ; ...
```

## Arithmetic

```prolog
% 'is' evaluates arithmetic expressions
factorial(0, 1) :- !.
factorial(N, F) :-
    N > 0,
    N1 is N - 1,
    factorial(N1, F1),
    F is N * F1.

% Fibonacci
fib(0, 0).
fib(1, 1).
fib(N, F) :-
    N > 1,
    N1 is N - 1,
    N2 is N - 2,
    fib(N1, F1),
    fib(N2, F2),
    F is F1 + F2.

% GCD
gcd(X, 0, X) :- X > 0.
gcd(X, Y, G) :- Y > 0, R is X mod Y, gcd(Y, R, G).
```

## Definite Clause Grammars (DCGs)

```prolog
% Parsing with DCGs — Prolog's built-in parsing notation
sentence --> noun_phrase, verb_phrase.
noun_phrase --> determiner, noun.
verb_phrase --> verb, noun_phrase.
verb_phrase --> verb.

determiner --> [the].
determiner --> [a].
noun --> [cat].
noun --> [dog].
noun --> [mouse].
verb --> [chases].
verb --> [sees].

% ?- sentence([the, cat, chases, a, mouse], []).
% true

% JSON-like parser
json_value --> json_string.
json_value --> json_number.
json_value --> json_array.
json_value --> json_object.

json_array --> "[", json_elements, "]".
json_elements --> json_value.
json_elements --> json_value, ",", json_elements.
```

## Constraint Logic Programming

```prolog
:- use_module(library(clpfd)).

% Sudoku solver (9x9)
sudoku(Rows) :-
    length(Rows, 9),
    maplist(same_length(Rows), Rows),
    append(Rows, Vs), Vs ins 1..9,
    maplist(all_distinct, Rows),
    transpose(Rows, Columns),
    maplist(all_distinct, Columns),
    Rows = [A,B,C,D,E,F,G,H,I],
    blocks(A, B, C), blocks(D, E, F), blocks(G, H, I),
    maplist(label, Rows).

blocks([], [], []).
blocks([A,B,C|T1], [D,E,F|T2], [G,H,I|T3]) :-
    all_distinct([A,B,C,D,E,F,G,H,I]),
    blocks(T1, T2, T3).

% N-Queens
n_queens(N, Qs) :-
    length(Qs, N),
    Qs ins 1..N,
    safe_queens(Qs),
    label(Qs).

safe_queens([]).
safe_queens([Q|Qs]) :-
    safe_queen(Q, Qs, 1),
    safe_queens(Qs).

safe_queen(_, [], _).
safe_queen(Q, [Q1|Qs], D) :-
    Q #\= Q1,
    Q #\= Q1 + D,
    Q #\= Q1 - D,
    D1 is D + 1,
    safe_queen(Q, Qs, D1).
```

## Meta-Programming

```prolog
% Assert/retract (dynamic knowledge base)
:- dynamic known/2.

remember(Key, Value) :-
    retractall(known(Key, _)),
    assert(known(Key, Value)).

forget(Key) :-
    retractall(known(Key, _)).

% findall (collect all solutions)
all_children(Parent, Children) :-
    findall(Child, parent(Parent, Child), Children).

% Clause inspection
describe_predicate(Name/Arity) :-
    functor(Head, Name, Arity),
    forall(clause(Head, Body),
           (write(Head), write(' :- '), writeln(Body))).
```

## Key Rules

1. **Think relationally, not procedurally.** Define relationships, not steps. Let Prolog search.
2. **Use cuts (`!`) sparingly.** Green cuts (optimization) are OK. Red cuts (change semantics) make code fragile.
3. **Use accumulator pattern** for tail-recursive predicates. Avoids stack overflow on large inputs.
4. **Use CLP(FD) for constraint problems.** Sudoku, scheduling, N-queens — don't write generate-and-test.
5. **Use DCGs for parsing.** They're cleaner and more maintainable than manual list difference-pair manipulation.
6. **Bidirectional predicates are powerful.** `append(X, Y, [1,2,3])` generates all splits — exploit this.

---

*Sources: The Art of Prolog (Sterling & Shapiro), Programming in Prolog (Clocksin & Mellish), SWI-Prolog documentation, Bratko's Prolog Programming for Artificial Intelligence*
