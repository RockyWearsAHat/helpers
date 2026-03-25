# F# Best Practices

## F# Philosophy

F# is a functional-first language on .NET. It combines ML-family type inference, algebraic data types, and pattern matching with full access to the .NET ecosystem. Railway-oriented programming and computation expressions are idiomatic.

- **Functional first, OOP when needed**: Default to immutable values, pure functions, and algebraic types.
- **Type inference**: Rarely annotate types manually — the compiler infers almost everything.
- **Computation expressions**: F#'s monadic abstraction — powers async, result, query, and custom workflows.

## Core Patterns

```fsharp
// Values are immutable by default
let name = "Alice"
let numbers = [1; 2; 3; 4; 5]

// Mutable only when needed (use sparingly)
let mutable counter = 0
counter <- counter + 1

// Functions
let add x y = x + y
let double = (*) 2

// Pipe operator for readable chains
[1..100]
|> List.filter (fun n -> n % 2 = 0)
|> List.map (fun n -> n * n)
|> List.sum

// Function composition
let processName = String.trim >> String.toLower >> String.replace " " "-"
```

## Algebraic Data Types

```fsharp
// Discriminated unions (sum types)
type Shape =
    | Circle of radius: float
    | Rectangle of width: float * height: float
    | Triangle of a: float * b: float * c: float

// Single-case unions for type safety (domain modeling)
type EmailAddress = EmailAddress of string
type OrderId = OrderId of int

// Record types (product types)
type User = {
    Name: string
    Age: int
    Email: EmailAddress
}

// Records are immutable; copy-and-update syntax
let older = { user with Age = user.Age + 1 }

// Option type (no nulls!)
type Result<'a> =
    | Ok of 'a
    | Error of string
```

## Pattern Matching

```fsharp
let area shape =
    match shape with
    | Circle r -> System.Math.PI * r * r
    | Rectangle (w, h) -> w * h
    | Triangle (a, b, c) ->
        let s = (a + b + c) / 2.0
        sqrt(s * (s - a) * (s - b) * (s - c))

// Active patterns (custom matching logic)
let (|Even|Odd|) n = if n % 2 = 0 then Even else Odd

let describe n =
    match n with
    | Even -> "even"
    | Odd -> "odd"

// Partial active patterns
let (|ParseInt|_|) (s: string) =
    match System.Int32.TryParse(s) with
    | true, n -> Some n
    | _ -> None

match input with
| ParseInt n -> printfn "Got number: %d" n
| _ -> printfn "Not a number"
```

## Railway-Oriented Programming

```fsharp
// Result type for error handling without exceptions
type Result<'T, 'E> =
    | Ok of 'T
    | Error of 'E

// Bind (>>=) chains operations that might fail
module Result =
    let bind f result =
        match result with
        | Ok value -> f value
        | Error e -> Error e

    let map f result =
        match result with
        | Ok value -> Ok (f value)
        | Error e -> Error e

// Pipeline of validations
let validateName name =
    if String.IsNullOrWhiteSpace name then Error "Name required"
    else Ok name

let validateAge age =
    if age < 0 || age > 150 then Error "Invalid age"
    else Ok age

let validateEmail email =
    if not (email |> String.contains "@") then Error "Invalid email"
    else Ok email

// Computation expression for Result
type ResultBuilder() =
    member _.Bind(result, f) = Result.bind f result
    member _.Return(value) = Ok value

let result = ResultBuilder()

let createUser name age email =
    result {
        let! validName = validateName name
        let! validAge = validateAge age
        let! validEmail = validateEmail email
        return { Name = validName; Age = validAge; Email = EmailAddress validEmail }
    }
```

## Async and Computation Expressions

```fsharp
// Async workflows
let fetchData url = async {
    use client = new System.Net.Http.HttpClient()
    let! response = client.GetStringAsync(url) |> Async.AwaitTask
    return response
}

// Parallel async
let fetchAll urls = async {
    let! results =
        urls
        |> List.map fetchData
        |> Async.Parallel
    return results
}

// Task computation expression (6.0+)
open System.Threading.Tasks

let fetchDataTask url = task {
    use client = new System.Net.Http.HttpClient()
    let! response = client.GetStringAsync(url)
    return response
}
```

## Collections

```fsharp
// List (immutable linked list)
let nums = [1; 2; 3]
let more = 0 :: nums          // prepend: [0; 1; 2; 3]
let combined = nums @ [4; 5]  // append: [1; 2; 3; 4; 5]

// Array (mutable, indexed, fast)
let arr = [|1; 2; 3|]
arr.[0] <- 10

// Seq (lazy)
let fibs = Seq.unfold (fun (a, b) -> Some(a, (b, a + b))) (0, 1)
fibs |> Seq.take 10 |> Seq.toList

// Map (immutable dictionary)
let lookup = Map.ofList [("a", 1); ("b", 2); ("c", 3)]
Map.find "b" lookup  // 2
Map.tryFind "d" lookup  // None
```

## Key Rules

1. **Use discriminated unions for domain modeling.** They make illegal states unrepresentable.
2. **Prefer `Result` over exceptions** for expected failure paths. Exceptions for unexpected/bug scenarios.
3. **Use the pipe operator `|>`** for data transformation chains. F# reads left-to-right, top-to-bottom.
4. **Avoid `null`.** Use `Option<'T>` instead. F# has `[<AllowNullLiteral>]` for .NET interop only.
5. **Keep functions small and composable.** Prefer `>>` (composition) and `|>` (application).
6. **Use computation expressions** for async, result, and custom monadic workflows.

---

_Sources: fsharp.org, F# for Fun and Profit (Scott Wlaschin), Domain Modeling Made Functional (Wlaschin), Microsoft F# Documentation_
