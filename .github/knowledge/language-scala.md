# Scala Best Practices

## Scala Philosophy

Scala fuses object-oriented and functional programming. Scala 3 (Dotty) simplifies the language significantly.

## Type System

```scala
// Union types (Scala 3)
def parse(input: String | Int): Double = input match
  case s: String => s.toDouble
  case n: Int    => n.toDouble

// Intersection types
trait Readable:
  def read(): String
trait Closeable:
  def close(): Unit

def process(resource: Readable & Closeable): String =
  try resource.read()
  finally resource.close()

// Opaque types (zero-cost wrappers)
opaque type UserId = String
object UserId:
  def apply(value: String): UserId = value
  extension (id: UserId) def value: String = id

// Enums (Scala 3)
enum Color:
  case Red, Green, Blue

enum Result[+T]:
  case Success(value: T)
  case Failure(error: String)
```

## Case Classes & Pattern Matching

```scala
// Case classes — immutable data with auto-generated equals, hashCode, toString, copy
case class User(name: String, email: String, age: Int)

val user = User("Alice", "alice@test.com", 30)
val admin = user.copy(name = "Admin")

// Pattern matching — exhaustive on sealed hierarchies
sealed trait Shape
case class Circle(radius: Double) extends Shape
case class Rectangle(width: Double, height: Double) extends Shape

def area(shape: Shape): Double = shape match
  case Circle(r)          => Math.PI * r * r
  case Rectangle(w, h)    => w * h

// Guard patterns
def classify(n: Int): String = n match
  case x if x > 0 => "positive"
  case 0           => "zero"
  case _           => "negative"

// Extractor patterns
object Email:
  def unapply(s: String): Option[(String, String)] =
    s.split("@") match
      case Array(user, domain) => Some((user, domain))
      case _                   => None

"alice@test.com" match
  case Email(user, domain) => println(s"User: $user, Domain: $domain")
```

## For Comprehensions (Monadic)

```scala
// For comprehension works with any monad (Option, List, Future, Either, IO)
// It desugars to flatMap/map/filter chains

// With Option
val result: Option[String] = for
  user  <- findUser(id)
  addr  <- user.address
  city  <- addr.city
yield city

// With Future
val dashboard: Future[(User, List[Post])] = for
  user  <- getUser(id)
  posts <- getPosts(user.id)
yield (user, posts)

// With Either
def validate(input: String): Either[String, Int] = for
  nonEmpty <- if input.nonEmpty then Right(input) else Left("empty")
  parsed   <- nonEmpty.toIntOption.toRight("not a number")
  valid    <- if parsed > 0 then Right(parsed) else Left("must be positive")
yield valid

// With List (like list comprehension)
val pairs = for
  x <- 1 to 10
  y <- 1 to 10
  if x + y == 10
yield (x, y)
```

## Implicits & Given/Using (Scala 3)

```scala
// Given instances (replace Scala 2 implicits)
trait Ordering[T]:
  def compare(a: T, b: T): Int

given Ordering[Int] with
  def compare(a: Int, b: Int): Int = a - b

// Using clauses (replace implicit parameters)
def sort[T](list: List[T])(using ord: Ordering[T]): List[T] =
  list.sortWith((a, b) => ord.compare(a, b) < 0)

// Extension methods (replace implicit classes)
extension (s: String)
  def words: List[String] = s.split("\\s+").toList
  def isBlank: Boolean = s.trim.isEmpty

"hello world".words  // List("hello", "world")

// Type classes
trait Show[T]:
  extension (t: T) def show: String

given Show[User] with
  extension (u: User) def show: String = s"${u.name} <${u.email}>"
```

## Concurrency

```scala
// Scala Future
import scala.concurrent.{Future, ExecutionContext}
import scala.concurrent.ExecutionContext.Implicits.global

val result: Future[String] = Future {
  heavyComputation()
}

// Parallel futures
val combined: Future[(User, Config)] = for
  user   <- Future(getUser(id))
  config <- Future(getConfig())
yield (user, config)

// Cats Effect IO (pure functional concurrency)
import cats.effect.IO

def fetchUser(id: String): IO[User] = IO.blocking {
  httpClient.get(s"/users/$id").as[User]
}

val program: IO[(User, Config)] = (fetchUser(id), getConfig()).parTupled

// ZIO
import zio.*

def fetchUser(id: String): ZIO[HttpClient, HttpError, User] =
  ZIO.serviceWithZIO[HttpClient](_.get(s"/users/$id"))
```

## Collections

```scala
// Immutable by default
val list = List(1, 2, 3, 4, 5)
val map = Map("a" -> 1, "b" -> 2)
val set = Set(1, 2, 3)

// Rich functional API
list.filter(_ > 2)                  // List(3, 4, 5)
list.map(_ * 2)                     // List(2, 4, 6, 8, 10)
list.flatMap(n => List(n, -n))      // List(1, -1, 2, -2, ...)
list.foldLeft(0)(_ + _)             // 15
list.groupBy(_ % 2 == 0)           // Map(false -> List(1,3,5), true -> List(2,4))
list.sliding(2).toList              // List(List(1,2), List(2,3), ...)
list.zip(list.tail)                 // List((1,2), (2,3), ...)
list.collect { case n if n > 3 => n * 10 }  // List(40, 50)

// Lazy collections
LazyList.from(1).filter(isPrime).take(10).toList
```

## Effect Systems

| Library | Paradigm |
|---------|----------|
| **Cats Effect** | Tagless final, IO monad |
| **ZIO** | ZIO effect type with built-in DI |
| **Monix** | Task-based, reactive |
| **Akka/Pekko** | Actor model |

## Build & Tooling

| Tool | Purpose |
|------|---------|
| **sbt** / **Mill** | Build system |
| **Metals** | Language server (VS Code, etc.) |
| **scalafmt** | Code formatting |
| **scalafix** | Linting & refactoring |
| **ScalaTest** / **MUnit** | Testing |
| **Wartremover** | Additional linting rules |

---

*Sources: Scala 3 documentation, Functional Programming in Scala (Chiusano/Bjarnason), Programming in Scala (Odersky), Essential Effects (Adam Rosien)*
