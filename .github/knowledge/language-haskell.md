# Haskell Best Practices

## Haskell Philosophy

Haskell is a purely functional, statically typed, lazy language. It enforces purity through the type system — side effects are tracked in types (IO monad).

## Types & Type Classes

```haskell
-- Algebraic Data Types
data Color = Red | Green | Blue
  deriving (Show, Eq, Ord, Enum, Bounded)

data Shape
  = Circle Double
  | Rectangle Double Double
  | Triangle Double Double
  deriving (Show)

-- Records
data User = User
  { userName  :: String
  , userEmail :: String
  , userAge   :: Int
  } deriving (Show, Eq)

-- Newtypes (zero-cost wrappers)
newtype UserId = UserId Int deriving (Show, Eq, Ord)
newtype Email = Email String deriving (Show, Eq)

-- Type classes (interfaces)
class Describable a where
  describe :: a -> String

instance Describable Shape where
  describe (Circle r)      = "Circle with radius " ++ show r
  describe (Rectangle w h) = "Rectangle " ++ show w ++ "x" ++ show h
  describe (Triangle b h)  = "Triangle with base " ++ show b

-- Type class constraints
printDescribable :: (Describable a, Show a) => a -> IO ()
printDescribable x = putStrLn (describe x)
```

## Pattern Matching & Guards

```haskell
-- Function definitions with patterns
area :: Shape -> Double
area (Circle r)      = pi * r * r
area (Rectangle w h) = w * h
area (Triangle b h)  = 0.5 * b * h

-- Guards
bmi :: Double -> String
bmi x
  | x < 18.5  = "underweight"
  | x < 25.0  = "normal"
  | x < 30.0  = "overweight"
  | otherwise  = "obese"

-- Case expressions
describe :: [a] -> String
describe xs = case xs of
  []      -> "empty"
  [_]     -> "singleton"
  [_, _]  -> "pair"
  _       -> "multiple elements"

-- Where clauses
cylinderArea :: Double -> Double -> Double
cylinderArea r h = 2 * pi * r * h + 2 * baseArea
  where baseArea = pi * r * r
```

## Monads & Do Notation

```haskell
-- Maybe monad (nullable values)
safeDivide :: Int -> Int -> Maybe Int
safeDivide _ 0 = Nothing
safeDivide a b = Just (a `div` b)

-- Chaining with >>= (bind)
lookupUser :: UserId -> Maybe User
lookupUser uid =
  findUser uid >>= \user ->
  verifyEmail (userEmail user) >>= \verified ->
  Just user { userEmail = verified }

-- Same thing with do notation (syntactic sugar for >>=)
lookupUser :: UserId -> Maybe User
lookupUser uid = do
  user     <- findUser uid
  verified <- verifyEmail (userEmail user)
  pure user { userEmail = verified }

-- IO monad (side effects)
main :: IO ()
main = do
  putStrLn "What is your name?"
  name <- getLine
  putStrLn ("Hello, " ++ name ++ "!")

-- Either monad (errors)
parseConfig :: String -> Either String Config
parseConfig input = do
  json   <- parseJSON input       -- Left "parse error" on failure
  host   <- lookupField "host" json
  port   <- lookupField "port" json >>= parsePort
  pure Config { configHost = host, configPort = port }
```

## Higher-Order Functions & Composition

```haskell
-- map, filter, fold
map (*2) [1, 2, 3]          -- [2, 4, 6]
filter even [1, 2, 3, 4, 5] -- [2, 4]
foldl (+) 0 [1, 2, 3, 4, 5] -- 15
foldr (:) [] [1, 2, 3]       -- [1, 2, 3]

-- Function composition
process :: String -> String
process = unwords . map capitalize . words . trim
-- Reads right to left: trim → words → capitalize each → rejoin

-- Point-free style
sumOfSquares :: [Int] -> Int
sumOfSquares = sum . map (^2)

-- $ operator (avoid parentheses)
putStrLn (show (length (filter even [1..100])))
putStrLn $ show $ length $ filter even [1..100]  -- Same thing, cleaner
```

## Lazy Evaluation

```haskell
-- Infinite lists
naturals :: [Int]
naturals = [1..]

fibs :: [Int]
fibs = 0 : 1 : zipWith (+) fibs (tail fibs)

-- Only compute what you need
take 10 fibs           -- [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]
head (filter (> 1000) fibs)  -- 1597

-- Lazy IO (careful — can cause resource leaks)
-- Use conduit, pipes, or streaming for production
```

## Concurrency

```haskell
import Control.Concurrent.Async

-- Parallel computation
(user, config) <- concurrently (getUser uid) getConfig

-- Race (first to finish wins)
result <- race (fetchPrimary url) (fetchBackup url)

-- STM (Software Transactional Memory)
import Control.Concurrent.STM

transfer :: TVar Int -> TVar Int -> Int -> STM ()
transfer from to amount = do
  balance <- readTVar from
  check (balance >= amount)  -- Retry if insufficient
  modifyTVar' from (subtract amount)
  modifyTVar' to (+ amount)

-- Run atomically
atomically $ transfer account1 account2 100
```

## Common Patterns

```haskell
-- Smart constructors
module User (User, mkUser, userName, userAge) where

data User = User { userName :: String, userAge :: Int }

mkUser :: String -> Int -> Maybe User
mkUser name age
  | null name = Nothing
  | age < 0   = Nothing
  | otherwise  = Just (User name age)

-- Lens (optics for nested updates)
-- Using the lens library
import Control.Lens

data Address = Address { _street :: String, _city :: String }
data Person = Person { _name :: String, _address :: Address }
makeLenses ''Address
makeLenses ''Person

-- Update nested field
updateCity :: Person -> Person
updateCity = address . city .~ "New York"

-- Monad transformers (stack effects)
type App a = ReaderT Config (ExceptT AppError IO) a

runApp :: Config -> App a -> IO (Either AppError a)
runApp config = runExceptT . flip runReaderT config
```

## Tooling

| Tool | Purpose |
|------|---------|
| **GHC** | Compiler |
| **cabal** / **stack** | Build systems |
| **HLS** | Language server (IDE support) |
| **hlint** | Linting suggestions |
| **fourmolu** / **ormolu** | Code formatting |
| **Hspec** / **QuickCheck** | Testing / property testing |
| **weeder** | Dead code detection |

---

*Sources: Haskell Programming from First Principles (Allen/Moronuki), Real World Haskell (O'Sullivan/Goerzen/Stewart), Learn You a Haskell (Lipovača), GHC documentation*
