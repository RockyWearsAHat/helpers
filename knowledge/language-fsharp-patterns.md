# F# Patterns: Computation Expressions, Type Providers, Railway-Oriented Programming

## Computation Expressions: Custom Monadic Workflows

Computation expressions (CE) allow defining domain-specific workflows. They're F#'s answer to Haskell's `do` notation, but more flexible—they can represent any "builder" pattern.

```fsharp
// Define a computation expression builder
type MaybeBuilder() =
    member _.Bind(m, f) = Option.bind f m
    member _.Return(x) = Some x

let maybe = MaybeBuilder()

// Use computation expression
let result = maybe {
    let! x = Some 5
    let! y = Some 3
    return x + y
}  // Some 8

// Desugars to: Option.bind (fun x -> Option.bind (fun y -> Some(x + y)) (Some 3)) (Some 5)
```

**Common CEs in standard library:**
- `seq { }`: Lazy sequences
- `async { }`: Asynchronous workflows
- `task { }`: .NET Tasks

```fsharp
// Async workflow (CE)
let download (url: string) = async {
    use! response = Http.AsyncGetString(url)
    return response.Length
}

// task workflow (modern async)
let fetchData (id: int) = task {
    let! data = httpClient.GetAsync($"/api/{id}")
    return! data.Content.ReadAsStringAsync()
}

// seq workflow (lazy enumeration)
let numbers = seq {
    for i in 1..10 do
        if i % 2 = 0 then
            yield i
}  // Lazy; nothing computed until enumerated
```

**Builder method overloads control CE semantics:**
- `Bind`: Sequencing (let!), flatMap
- `Return`: Wrap pure value
- `Yield`: Produce value (esp. in seq)
- `Delay`: Defer computation
- `Combine`: Merge branches
- `Zero`: Empty value
- `Using`: Resource management (try-finally)

## Active Patterns: Pattern Matching Extensions

Active patterns let you define custom match cases, enabling domain-specific pattern syntax.

```fsharp
// Simple active pattern: partitions cases
let (|Even|Odd|) n = if n % 2 = 0 then Even else Odd

let classify n = 
    match n with
    | Even -> "even"
    | Odd -> "odd"

// Multi-case active pattern
let (|Zero|Positive|Negative|) n =
    if n = 0 then Zero
    elif n > 0 then Positive
    else Negative

let sign = function
    | Zero -> "zero"
    | Positive -> "positive"
    | Negative -> "negative"

// Partial active pattern: might not match (denoted |_|)
let (|Integer|_|) (str: string) =
    match System.Int32.TryParse(str) with
    | (true, i) -> Some i
    | _ -> None

let parseInt s =
    match s with
    | Integer i -> sprintf "Parsed: %i" i
    | _ -> sprintf "Not a number: %s" s

// Active pattern in function args
let processString = function
    | Integer 42 -> "Found the answer"
    | Integer n when n > 0 -> "Positive int"
    | _ -> "Not an int or not positive"
```

**Use cases:**
- Domain modeling: `(|ValidEmail|_|)`, `(|DateRange|_|)`
- Parsing: custom lexical patterns
- Conditional matching: wrap complex conditions

## Type Providers: Compile-Time Code Generation

Type providers generate types from external data sources (APIs, databases, CSV) at compile time.

```fsharp
// Type provider for CSV files
type CsvFile = FSharp.Data.CsvFile<"data.csv">

let data = CsvFile.Load("data.csv")
// Compiler sees columns as properties
for row in data.Rows do
    printfn "%s: %i" row.Name row.Age

// Type provider for JSON
type JsonProvider = FSharp.Data.JsonProvider<"""{"name": "Alice", "age": 30}""">

let person = JsonProvider.Load("person.json")
printfn "%s is %i" person.Name person.Age

// Type provider for databases
type SqlConnection = SqlCommandProvider<"SELECT * FROM Users WHERE ID = @id", ConnectionString>

let query = new SqlConnection()
let user = query.Execute(id = 1)
```

**Advantages:**
- No parsing boilerplate: types match external schema
- Compile-time validation: mismatched fields caught early
- Autocomplete in IDE: columns/fields show up

**Limitations:**
- Requires external resource at compile time
- Can slow incremental builds
- Debugging is harder (generated code)

## Discriminated Unions (Tagged Unions)

Discriminated unions are F#'s precise data modeling tool—more expressive than enums, safer than nullable references.

```fsharp
// Simple union
type Color = Red | Green | Blue

// Union with associated data
type Shape =
    | Circle of radius: float
    | Rectangle of width: float * height: float
    | Triangle of side1: float * side2: float * side3: float

let area = function
    | Circle r -> System.Math.PI * r * r
    | Rectangle (w, h) -> w * h
    | Triangle (a, b, c) ->
        // Heron's formula
        let s = (a + b + c) / 2.0
        System.Math.Sqrt(s * (s - a) * (s - b) * (s - c))

// Nested unions (common for ASTs)
type Expr =
    | Num of int
    | Var of string
    | Add of Expr * Expr
    | Mul of Expr * Expr

let rec eval env = function
    | Num n -> n
    | Var x -> env |> List.find (fun (k, _) -> k = x) |> snd
    | Add (e1, e2) -> eval env e1 + eval env e2
    | Mul (e1, e2) -> eval env e1 * eval env e2
```

## Option and Result: Error Handling

`Option<'a>` (Some x | None) and `Result<'a, 'e>` (Ok x | Error e) encode success/failure at the type level.

```fsharp
// Option: presence/absence
let tryParse (s: string) : Option<int> =
    match System.Int32.TryParse(s) with
    | (true, n) -> Some n
    | _ -> None

let x = tryParse "42"  // Some 42
let y = tryParse "abc" // None

// Result: success with error context
type DataError = 
    | ParseError of string
    | NotFound of id: int

let fetchData (id: int) : Result<string, DataError> =
    if id < 0 then Error (NotFound id)
    else Ok ("data for " + id.ToString())

// Chaining with computation expressions
let workflow = result {
    let! a = Ok 5
    let! b = Ok 3
    return a + b
}  // Ok 8

let errWorkflow = result {
    let! a = Error (ParseError "invalid")
    let! b = Ok 3
    return a + b
}  // Error (ParseError "invalid")
```

## Railway-Oriented Programming (ROP)

ROP models computations as "happy path" (Ok) or "sad path" (Error), with explicit track switching.

```fsharp
type Result<'a, 'e> = Ok of 'a | Error of 'e

// Basic track operation
let bind f = function
    | Ok x -> f x
    | Error e -> Error e

// Pipeline operators
let (>>=) m f = bind f m
let (|>>) m f = Result.map f m

// Example workflow
let createUser name email : Result<User, string> =
    result {
        let! validName = validateName name
        let! validEmail = validateEmail email
        return { Name = validName; Email = validEmail }
    }

// Or explicitly chained
let createUserExplicit name email =
    validateName name
    |> Result.bind (fun n ->
        validateEmail email
        |> Result.bind (fun e ->
            Ok { Name = n; Email = e }
        )
    )

// Two-track function (takes regular function, lifts to Result track)
let lift f x = Ok (f x)

// Track switching: adapt Ok value or switch tracks
let recover onError = function
    | Ok x -> Ok x
    | Error e -> onError e

let withDefault defaultValue = function
    | Ok x -> x
    | Error _ -> defaultValue
```

## Piping: The F# Style

The pipe operator `|>` threads data through functions left-to-right.

```fsharp
// Without pipe (hard to read)
let result = Math.Sqrt(Math.Abs(-16.0))

// With pipe (data flow clear)
let result = -16.0 |> Math.Abs |> Math.Sqrt

// Piping through multiple operations
let numbers = [1; 2; 3; 4; 5]
let result =
    numbers
    |> List.filter (fun x -> x > 2)
    |> List.map (fun x -> x * 2)
    |> List.sum

// Backward pipe |< (less common) threads in reverse
let f = (+) |< 5
f 3  // 5 + 3 = 8
```

## Async Workflows: Asynchronous I/O

Async CEs enable clean concurrent code without explicit callback chains.

```fsharp
// Async workflow (cold task)
let fetchUrl (url: string) = async {
    use client = new System.Net.Http.HttpClient()
    let! response = client.GetStringAsync(url) |> Async.AwaitTask
    return response.Length
}

// Run multiple async operations concurrently
let fetchMultiple urls = async {
    let tasks = urls |> List.map fetchUrl
    let! results = Async.Parallel tasks
    return Array.sum results
}

// ExecuteAsync.RunSynchronously(fetchMultiple ["url1"; "url2"])
// or
// Async.Start(fetchMultiple [...])  // Fire and forget
```

## Fable: F# to JavaScript

Fable transpiles F# to JavaScript, allowing full-stack development in F#.

```fsharp
// Shared F# code
module Domain =
    type User = { Id: int; Name: string }
    
    let validateName (name: string) =
        if name.Length > 0 then Ok name else Error "Empty name"

// Backend (Giraffe web framework)
open Giraffe

let getUserHandler id = 
    fun (next: HttpFunc) (ctx: HttpContext) -> task {
        let user = { Id = id; Name = "Alice" }
        return! json user next ctx
    }

// Frontend (Fable/Browser)
[<JSImport("fetch", "")>]
let fetch (url: string) : JS.Promise<obj> = jsNative

let loadUser id = async {
    let! response = fetch $"/api/users/{id}" |> Async.AwaitPromise
    return response
}
```

## Giraffe Web Framework

Giraffe is a lightweight web framework for F# and ASP.NET.

```fsharp
open Giraffe

let hello = "Hello World!" |> text

let routes =
    choose [
        GET >=> route "/" >=> hello
        GET >=> routef "/user/%i" (fun id -> sprintf "User %i" id |> text)
        POST >=> route "/users" >=> handlePostUser
    ]

let notFound = "Not found" |> NOT_FOUND

let webApp = routes >=> notFound

[<EntryPoint>]
let main _ =
    WebHostBuilder()
        .UseKestrel()
        .ConfigureServices(fun services -> 
            services.AddGiraffe() |> ignore)
        .Configure(fun app ->
            app.UseGiraffe webApp)
        .Build()
        .Run()
    0
```

## See Also

- [paradigm-type-level-programming.md](paradigm-type-level-programming.md) — type providers and compile-time generation
- [pl-effect-systems.md](pl-effect-systems.md) — async and effect system theory
- [error-handling-patterns.md](error-handling-patterns.md) — error handling across languages