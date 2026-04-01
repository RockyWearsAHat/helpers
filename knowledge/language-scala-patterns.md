# Scala Patterns: Case Classes, Pattern Matching, Type Classes

## Case Classes: Product Types with Boilerplate Elimination

Case classes are Scala's primary data container. They automatically generate `equals`, `hashCode`, `toString`, `copy`, and pattern matching support.

```scala
// Basic case class
case class Person(name: String, age: Int)

// Automatically generates:
// - Constructor
// - Accessors (name, age)
// - equals, hashCode, toString
// - copy method for updating fields

val p1 = Person("Alice", 30)
println(p1)  // Person(Alice,30)

// copy creates new instance with some fields changed
val p2 = p1.copy(age = 31)

// Pattern matching (covered below integrates with case classes)
p1 match {
  case Person(n, a) => println(s"$n is $a years old")
}

// Nested case classes
case class Address(street: String, city: String)
case class Employee(person: Person, address: Address)

val emp = Employee(Person("Bob", 25), Address("Main St", "NYC"))
```

**Case class rules:**
- Constructor params become public immutable fields
- Can accept default values: `case class Point(x: Int = 0, y: Int = 0)`
- Can be sealed: `sealed case class ...` (compiler ensures exhaustive matching)
- No-arg case class: `case class Marker()`—valid, often for markers/signals

## Pattern Matching: Exhaustive Destructuring

Pattern matching is Scala's workhorse for destructuring and control flow.

```scala
// Basic patterns
val result: Any = 42
result match {
  case 42 => println("The answer")
  case s: String => println(s"String: $s")
  case _ => println("something else")
}

// Destructuring case classes
case class Point(x: Int, y: Int)

Point(3, 4) match {
  case Point(0, 0) => println("origin")
  case Point(x, 0) => println(s"on x-axis at $x")
  case Point(x, y) => println(s"general: $x, $y")
}

// Nested patterns
case class Pair(a: Int, b: Int)
List(Pair(1, 2), Pair(3, 4)) match {
  case Pair(x, y) :: Pair(a, b) :: _ => 
    println(s"First pair: $x, $y; Second: $a, $b")
  case _ => println("doesn't match")
}

// Guards (conditions on patterns)
List(1, 2, 3) match {
  case x :: xs if x > 0 => println(s"positive: $x")
  case _ => println("non-positive or empty")
}

// Variable patterns and alternatives
val (a, b) = (10, 20)  // destructure tuple
val List(head, tail @ _*) = List(1, 2, 3)  // head = 1, tail = [2, 3]

// Constant patterns (not variables)
val X = 5
val Y = 10
Point(5, 10) match {
  case Point(X, Y) => println("matched constants 5 and 10")
  case _ => println("didn't match")
}
```

**Exhaustiveness checking**: Compiler warns if patterns don't cover all cases. Use `sealed` on case class hierarchies to enable this.

```scala
sealed trait Shape
case class Circle(radius: Double) extends Shape
case class Square(side: Double) extends Shape

def area(s: Shape) = s match {
  case Circle(r) => Math.PI * r * r
  case Square(s) => s * s
  // Compiler warns if you forget a case
}
```

## Implicit Conversions and Implicit Parameters

**Scala 2 idiom** (still common): Implicit conversions allow automatic type adaptation; implicit parameters provide type class-like behavior.

```scala
// Implicit conversion: Int → String (not recommended; error-prone)
implicit def intToString(n: Int): String = n.toString

val s: String = 42  // Implicitly converted

// Implicit parameter: provide context without explicit passing
case class Config(debug: Boolean)

def log(msg: String)(implicit config: Config) = {
  if (config.debug) println(msg)
}

implicit val config = Config(debug = true)
log("test")  // Config passed implicitly
```

**Scala 3 replaces this with `given` and `using`**:
```scala
// Scala 3: given (explicit type class instances)
given Config = Config(debug = true)

def log(msg: String)(using config: Config) = {
  if (config.debug) println(msg)
}

// Can pass explicitly
log("test")(using Config(debug = false))
```

**Common use: type classes**
```scala
// Scala 2
trait Show[T] {
  def show(t: T): String
}

implicit val intShow: Show[Int] = new Show[Int] {
  def show(n: Int) = n.toString
}

def printIt[T](t: T)(implicit s: Show[T]) = println(s.show(t))

// Scala 3
trait Show[T]:
  def show(t: T): String

given Show[Int] = n => n.toString

def printIt[T](t: T)(using s: Show[T]) = println(s.show(t))
```

## For Comprehensions: Monadic Sequencing

For comprehensions (for-yield) desugar to monadic chains. Any type with `map`, `flatMap`, `withFilter` works.

```scala
// For-yield with List
val result = for {
  x <- List(1, 2, 3)
  y <- List(10, 20) if x > 1
  z = x + y
} yield (x, y, z)
// result = List((2,10,12), (2,20,22), (3,10,13), (3,20,23))

// Desugars to:
// List(1, 2, 3).flatMap(x =>
//   List(10, 20).withFilter(_ => x > 1).flatMap(y => ...))

// Works with Option
val opt = for {
  a <- Some(5)
  b <- Some(3)
} yield a + b  // Some(8)

// Works with Future
val fut = for {
  a <- Future { 1 }
  b <- Future { 2 }
} yield a + b
```

## Traits and Mixins: Flexible Composition

Traits are interfaces with implementation. Scala's linearization allows multiple trait mixing.

```scala
// Trait: interface + default implementation
trait Animal {
  def name: String
  def sound: String
}

trait Warm extends Animal {
  def hasWarmBlood = true
}

trait Furry extends Animal {
  def isFurry = true
}

// Mix traits into class
class Dog(val name: String) extends Animal with Warm with Furry {
  def sound = "Woof"
}

val dog = new Dog("Rex")
println(dog.sound)        // Woof
println(dog.hasWarmBlood) // true
println(dog.isFurry)      // true

// Trait with state
trait Counter {
  private var count = 0
  def increment() = count += 1
  def getCount = count
}

class Button extends Counter
```

**Linearization**: Method resolution order (left-to-right, depth-first). Can be complex with deep hierarchies; prefer shallow trait stacks.

## Type Classes (the Right Way)

Unlike implicit conversions, type classes are type-safe and testable.

```scala
// Define contract
trait Serializable[T] {
  def serialize(t: T): String
  def deserialize(s: String): T
}

// Provide instances for types
given Serializable[Int] = new Serializable[Int] {
  def serialize(n: Int) = n.toString
  def deserialize(s: String) = s.toInt
}

given Serializable[String] = new Serializable[String] {
  def serialize(s: String) = s
  def deserialize(s: String) = s
}

// Use with context bounds (Scala 3)
def save[T: Serializable](t: T): String = {
  implicitly[Serializable[T]].serialize(t)
}

// Or with explicit using (Scala 3)
def save[T](t: T)(using s: Serializable[T]): String = s.serialize(t)
```

## Akka / Pekko Actors: Message-Driven Concurrency

Actors are lightweight concurrent entities communicating via messages. Akka is the standard library; Pekko is the community fork (after Akka's license change).

```scala
// Define actor
class PingActor extends Actor {
  def receive: Receive = {
    case "ping" => 
      println("Pong!")
      sender() ! "pong"
    case _ => println("Unknown message")
  }
}

// Create and send
import akka.actor.ActorSystem
val system = ActorSystem("MySystem")
val actor = system.actorOf(Props[PingActor], "ping")
actor ! "ping"
```

**Key concepts:**
- **Location transparency**: Same code works locally or distributed
- **Fault tolerance**: Supervisor strategies restart failed actors
- **Back-pressure**: Message queuing prevents overwhelming receivers
- **No shared mutable state**: Messages are the only communication

## Effect Systems: ZIO and Cats Effect

Modern Scala uses functional effect systems for controlled side effects.

```scala
// ZIO
import zio.*

val effect: ZIO[Any, Nothing, String] = 
  for {
    _ <- Console.printLine("What's your name?")
    name <- Console.readLine
    _ <- Console.printLine(s"Hello, $name")
  } yield name

ZIOAppDefault.run {
  effect
}

// Cats Effect (IO monad)
import cats.effect.IO

val program: IO[Unit] = 
  for {
    line <- IO.readLine
    _ <- IO.println(s"You entered: $line")
  } yield ()

program.unsafeRunSync()
```

**Advantages over raw threading:**
- Referential transparency: Effects are values, testable
- Resource safety: Automatic cleanup via `Resource`
- Composability: Combine effects declaratively

## sbt Build Tool

Standard build tool for Scala projects.

```scala
// build.sbt
name := "MyProject"
version := "0.1.0"
scalaVersion := "3.3.0"

libraryDependencies ++= Seq(
  "org.scalactic" %% "scalactic" % "3.2.16",
  "org.scalatest" %% "scalatest" % "3.2.16" % Test,
  "com.typesafe.akka" %% "akka-actor" % "2.8.0"
)

Compile / scalacOptions ++= Seq(
  "-deprecation",
  "-feature",
  "-Wunused:imports"
)
```

**Key commands:**
- `sbt compile`: Compile
- `sbt test`: Run tests
- `sbt run`: Run main
- `sbt package`: Create JAR
- `sbt console`: REPL with project dependencies

## See Also

- [language-scala.md](language-scala.md) — core conventions
- [paradigm-type-level-programming.md](paradigm-type-level-programming.md) — advanced types
- [paradigm-concurrent-models.md](paradigm-concurrent-models.md) — actor model deep dive