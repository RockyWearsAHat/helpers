# Haskell Practical: Type Classes, Monads, and I/O

## Type Classes: The Abstraction Mechanism

Haskell's type classes are its primary abstraction mechanism, replacing inheritance found in OOP. A type class defines a set of operations that a type must implement; instances provide specific implementations.

```haskell
-- Core hierarchy: Functor → Applicative → Monad
class Functor f where
  fmap :: (a -> b) -> f a -> f b  -- map a function over wrapped values

class Functor f => Applicative f where
  pure :: a -> f a                -- wrap a pure value
  (<*>) :: f (a -> b) -> f a -> f b  -- apply wrapped function to wrapped value

class Applicative m => Monad m where
  (>>=) :: m a -> (a -> m b) -> m b  -- sequencing with binding
  (>>) :: m a -> m b -> m b       -- sequence, ignore first result
  return :: a -> m a              -- same as pure
```

**Why this hierarchy?** Each level adds structure. Functors map functions. Applicatives add function wrapping. Monads add sequencing and binding — critical for modeling effects.

**Common instances:**
- `Maybe a`: computation that might fail (no exception)
- `Either e a`: computation that might fail with error of type `e`
- `[a]`: non-deterministic computation (0+ results)
- `IO a`: side effect producing `a`

```haskell
-- Maybe: absence or failure
instance Functor Maybe where
  fmap f (Just x) = Just (f x)
  fmap _ Nothing = Nothing

instance Monad Maybe where
  Nothing >>= _ = Nothing
  Just x >>= f = f x
  return = Just

-- Either: failure with context
instance Monad (Either e) where
  Right x >>= f = f x
  Left e >>= _ = Left e
  return = Right
```

## Do Notation: Syntactic Sugar for Monads

`do` notation desugars into nested `>>=` calls. It makes monadic code read sequentially without deep nesting.

```haskell
-- Without do notation
result = 
  readLn >>= \x ->
  readLn >>= \y ->
  return (x + y)

-- With do notation (equivalent)
result = do
  x <- readLn
  y <- readLn
  return (x + y)

-- Desugaring: x <- m automatically becomes m >>= \x -> ...
-- Expression statements (no <-) desugar to (>>), ignoring result
doExample = do
  putStrLn "Start"        -- putStrLn "Start" >> ...
  x <- readLn
  putStrLn "Got input"    -- ... >> putStrLn "Got input" >> ...
  return x
```

**Key insight**: `do` is syntactic sugar, not magic. Understanding the desugarings clarifies when code works and when it doesn't.

## IO and Side Effects

The `IO` monad encapsulates all side effects. Haskell's type system enforces that side effects stay in `IO`, preserving purity everywhere else.

```haskell
-- Main must be IO ()
main :: IO ()
main = do
  putStrLn "Enter your name:"
  name <- getLine
  putStrLn ("Hello, " ++ name)

-- Mixing pure and IO
processFile :: FilePath -> IO String
processFile path = do
  content <- readFile path
  let processed = map toUpper content  -- pure function
  return processed

-- Return values from IO
getName :: IO String
getName = do
  putStrLn "Name?"
  getLine  -- last expression in do block is the result
```

**Common pitfall**: `return` does NOT exit early. In `do` blocks, `return x` wraps `x` in the monad; execution continues. Use `pure` in applicative contexts for clarity.

```haskell
-- This does NOT print only 1; it prints both
example :: IO ()
example = do
  putStrLn "1"
  return ()
  putStrLn "2"  -- still executes
```

## Maybe and Either: Error Handling Without Exceptions

`Maybe` represents optional values; `Either` carries error information.

```haskell
-- Maybe: no value (Nothing) or has value (Just x)
safeDiv :: Double -> Double -> Maybe Double
safeDiv _ 0 = Nothing
safeDiv x y = Just (x / y)

-- Chaining with do notation
computation :: Maybe Int
computation = do
  x <- Just 5
  y <- safeDiv 10 2
  return (x + floor y)  -- Short-circuits if any step is Nothing

-- Either: Left e for error, Right a for success
divide :: Double -> Double -> Either String Double
divide _ 0 = Left "Division by zero"
divide x y = Right (x / y)

-- Either chains similarly; Left propagates, short-circuiting
calc :: Either String Int
calc = do
  x <- Right 5
  y <- divide 10 2
  Right (x + floor y)
```

**When to use:**
- `Maybe`: Simple absence OR presence
- `Either`: Need to communicate WHY computation failed
- Exceptions (`throw`/`catch`): Truly exceptional, unrecoverable errors (rare in pure code)

## Typeclasses vs. Interfaces

Haskell type classes are NOT the same as OOP interfaces.

**Type class advantages:**
- **Ad hoc polymorphism**: Implement `Eq` for an existing type retroactively (no source modification required)
- **Multiple inheritance**: A type can instantiate multiple classes without conflicts
- **Operator overloading**: `+`, `==`, etc. are just class methods

**Type class limitations:**
- Cannot store in data structures (e.g., `[SomeClass]` is not expressible; must use wrapper types or existential quantification)
- Requires explicit instance declarations (no implicit subtyping hierarchy)
- Resolution is compile-time; no dynamic dispatch

```haskell
-- Multiple instances for same type (overlapping requires extension)
instance Eq Int where (==) = intEq
instance Ord Int where compare = intCompare

-- Common type class: Show (convert to string)
class Show a where
  show :: a -> String

instance Show Bool where
  show True = "True"
  show False = "False"

-- Type class constraints in function signatures
printIfEq :: (Show a, Eq a) => a -> a -> IO ()
printIfEq x y = 
  if x == y 
    then putStrLn ("Equal: " ++ show x)
    else putStrLn "Not equal"
```

## Algebraic Data Types and Pattern Matching

ADTs model data precisely. Pattern matching destructures them.

```haskell
-- Sum type (one of many options)
data Result a = Success a | Failure String

-- Product type (tuple of fields)
data Person = Person { name :: String, age :: Int }

-- Recursive
data Tree a = Leaf a | Node (Tree a) (Tree a)

-- Pattern matching exhaustively destructures
depth :: Tree a -> Int
depth (Leaf _) = 0
depth (Node left right) = 1 + max (depth left) (depth right)

-- Guards and patterns combined
classify :: Int -> String
classify n
  | n < 0 = "negative"
  | n == 0 = "zero"
  | n > 0 = "positive"

-- Patterns in function parameters
isEmpty :: [a] -> Bool
isEmpty [] = True
isEmpty _ = False

-- Pattern guards
safeHead :: [a] -> Maybe a
safeHead (x:_) = Just x
safeHead [] = Nothing
```

## GHC Extensions and Modern Haskell

The Haskell standard (Haskell 2010) is minimal. Most practical code uses GHC extensions.

```haskell
{-# LANGUAGE OverloadedStrings #-}      -- String literals → many types
{-# LANGUAGE RecordWildCards #-}        -- "Person {..}" in patterns
{-# LANGUAGE MultiParamTypeClasses #-}  -- Classes with 2+ params
{-# LANGUAGE FlexibleInstances #-}      -- Complex instance constraints
{-# LANGUAGE ConstraintKinds #-}        -- Treat constraints as types
{-# LANGUAGE GADTs #-}                  -- Generalized Algebraic Data Types
{-# LANGUAGE RankNTypes #-}             -- Forall in argument types
```

**Most impactful:**
- `OverloadedStrings`: Makes string literals polymorphic, reducing friction with `Text`
- `RecordWildCards`: Unpacks all record fields in pattern; reduces boilerplate
- `GADTs`: Encode type information in constructors; powerful for DSLs
- `FlexibleInstances`: Allow instances on partially applied types

## Lazy Evaluation: Model and Pitfalls

Haskell is lazy—expressions are not evaluated until needed. This enables elegant infinite structures but introduces complexity.

```haskell
-- Infinite list (evaluated lazily)
ones = 1 : ones

-- Take only first 5
take 5 ones  -- [1,1,1,1,1]

-- Thunks: unevaluated expressions in memory
expensive = foldl (+) 0 [1..1000000]
-- The list and composition are all thunks until result is needed
```

**Pitfall: Lazy accumulation (space leaks)**
```haskell
-- BAD: accumulator stays unevaluated, building a thunk chain
sumBad :: [Int] -> Int
sumBad = foldl (+) 0

-- GOOD: use strict application to force evaluation
sumGood :: [Int] -> Int
sumGood = foldl' (+) 0  -- foldl' is strict
```

**Pitfall: Unexpected behavior from laziness**
```haskell
-- This returns immediately (lazy!)
result = filter (> 5) [1..1000000]

-- But printing forces evaluation of entire list
print result  -- Now it's slow
```

**Controlling evaluation:**
- `seq :: a -> b -> b`: Evaluate first arg to WHNF (weak head normal form)
- `$!`: Strict application operator
- `-O2` compiler flag: Enables strictness analysis
- `BangPatterns` extension: `f !x = ...` forces arg

## Build Tools: Cabal and Stack

**Cabal**: Lower-level, dependency resolution, build system. More flexibility, steeper learning curve.
**Stack**: Higher-level wrapper around Cabal, reproducible builds, lockfiles. Recommended for projects.

```yaml
-- stack.yaml (reproduces exact environment)
resolver: lts-21.0  -- locked snapshot of packages

packages:
  - .

ghc-options:
  - -Wall
  - -O2
```

```cabal
-- myproject.cabal
executable myproject
  main-is: Main.hs
  build-depends:
    base >=4.16 && <5,
    text >=1.2 && <2.1,
    bytestring,
    containers
```

**Key difference**: Stack locks versions; Cabal allows flexibility. Stack is often preferred in teams for reproducibility.

## See Also

- [language-haskell.md](language-haskell.md) — conventions and core idioms
- [pl-effect-systems.md](pl-effect-systems.md) — monad and effect system theory
- [paradigm-functional-programming.md](paradigm-functional-programming.md) — FP concepts