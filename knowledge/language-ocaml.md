# OCaml Conventions and Idioms

## OCaml Philosophy

OCaml is an ML-family language with a powerful type system, pattern matching, and algebraic data types. It compiles to fast native code and has a strong tradition in formal verification, compilers, and systems programming.

- **If it compiles, it usually works**: The type system catches most bugs at compile time.
- **Algebraic types + pattern matching**: The primary way to model data and control flow.
- **Performance**: Native code compiler produces efficient executables. No GC pauses for most workloads.

## Core Types and Pattern Matching

```ocaml
(* Variant types (sum types) *)
type shape =
  | Circle of float
  | Rectangle of float * float
  | Triangle of { a : float; b : float; c : float }

let area = function
  | Circle r -> Float.pi *. r *. r
  | Rectangle (w, h) -> w *. h
  | Triangle { a; b; c } ->
    let s = (a +. b +. c) /. 2.0 in
    Float.sqrt (s *. (s -. a) *. (s -. b) *. (s -. c))

(* Record types *)
type user = {
  name : string;
  age : int;
  email : string;
}

(* Functional record update *)
let birthday user = { user with age = user.age + 1 }

(* Option type (built-in, no nulls) *)
let find_user id =
  if id = 42 then Some { name = "Alice"; age = 30; email = "alice@test.com" }
  else None

(* Result type *)
type ('a, 'e) result = Ok of 'a | Error of 'e
```

## Functions and Composition

```ocaml
(* Functions are curried by default *)
let add x y = x + y
let add5 = add 5  (* partial application *)

(* Pipeline operator *)
let result =
  [1; 2; 3; 4; 5]
  |> List.filter (fun x -> x mod 2 = 0)
  |> List.map (fun x -> x * x)
  |> List.fold_left (+) 0

(* Function composition *)
let ( >> ) f g x = g (f x)

let process = String.trim >> String.lowercase_ascii

(* Recursive functions *)
let rec map f = function
  | [] -> []
  | x :: xs -> f x :: map f xs

(* Tail-recursive with accumulator *)
let sum lst =
  let rec aux acc = function
    | [] -> acc
    | x :: xs -> aux (acc + x) xs
  in
  aux 0 lst
```

## Modules and Functors

```ocaml
(* Module signature (interface) *)
module type COLLECTION = sig
  type 'a t
  val empty : 'a t
  val add : 'a -> 'a t -> 'a t
  val member : 'a -> 'a t -> bool
  val to_list : 'a t -> 'a list
end

(* Module implementation *)
module ListSet : COLLECTION = struct
  type 'a t = 'a list
  let empty = []
  let add x xs = if List.mem x xs then xs else x :: xs
  let member = List.mem
  let to_list xs = xs
end

(* Functors (modules parameterized by modules) *)
module Make_cache (Key : Map.OrderedType) = struct
  module M = Map.Make(Key)

  type 'a t = {
    data : 'a M.t;
    max_size : int;
  }

  let create max_size = { data = M.empty; max_size }

  let add key value cache =
    { cache with data = M.add key value cache.data }
end

module StringCache = Make_cache(String)
```

## Error Handling

```ocaml
(* Option for "might not exist" *)
let safe_div a b =
  if b = 0 then None
  else Some (a / b)

(* Result for "might fail with info" *)
let parse_int s =
  match int_of_string_opt s with
  | Some n -> Ok n
  | None -> Error (Printf.sprintf "Cannot parse '%s' as int" s)

(* Monadic binding with Result *)
let ( let* ) = Result.bind

let process input =
  let* x = parse_int input in
  let* y = validate_range x 1 100 in
  Ok (compute y)

(* Exceptions for truly exceptional cases *)
exception Config_error of string

let load_config path =
  match Sys.file_exists path with
  | false -> raise (Config_error (Printf.sprintf "File not found: %s" path))
  | true -> (* parse config *)
```

## Imperative Features (When Needed)

```ocaml
(* Mutable references *)
let counter = ref 0
let increment () = counter := !counter + 1

(* Arrays (mutable, O(1) access) *)
let arr = Array.make 10 0
let () = arr.(0) <- 42

(* Hash tables *)
let tbl = Hashtbl.create 16
let () = Hashtbl.replace tbl "key" "value"

(* Sequences / iterators (lazy) *)
let naturals = Seq.ints 0
let first_10_squares =
  naturals
  |> Seq.map (fun n -> n * n)
  |> Seq.take 10
  |> List.of_seq
```

## GADTs (Generalized Algebraic Data Types)

```ocaml
(* Type-safe expression evaluator *)
type _ expr =
  | Int : int -> int expr
  | Bool : bool -> bool expr
  | Add : int expr * int expr -> int expr
  | If : bool expr * 'a expr * 'a expr -> 'a expr
  | Eq : int expr * int expr -> bool expr

let rec eval : type a. a expr -> a = function
  | Int n -> n
  | Bool b -> b
  | Add (a, b) -> eval a + eval b
  | If (cond, t, f) -> if eval cond then eval t else eval f
  | Eq (a, b) -> eval a = eval b

(* The type system guarantees: Add only takes ints, If branches match types *)
```

## Conventions

1. **Exhaustive pattern matching.** Never use `_` as a catch-all unless you truly mean "any other case." The compiler warns about missing cases — listen to it.
2. **Use `Option` and `Result`, not exceptions**, for expected failure paths. Exceptions for bugs and truly exceptional conditions.
3. **Prefer immutable data.** Use refs, arrays, and mutable records only for performance-critical paths.
4. **Modules for abstraction.** Use module signatures to hide implementation details. Functors for parameterized modules.
5. **Tail recursion for loops.** Use accumulator pattern to make recursive functions tail-recursive.
6. **Let binding syntax `let*`** for monadic chaining (OCaml 4.08+). Cleaner than nested `match` expressions.

---

_Sources: Real World OCaml (Minsky, Madhavapeddy, Hickey), OCaml Manual (ocaml.org), Cornell CS 3110, OCaml Standard Library docs_
