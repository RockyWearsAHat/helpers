# Groovy Conventions and Idioms

## Groovy Philosophy

Groovy is a dynamic language for the JVM that extends Java syntax with closures, builders, and metaprogramming. It's the language behind Gradle build scripts, Jenkins pipelines, and Grails web framework.

- **Java-compatible**: Almost all Java code is valid Groovy. Groovy adds convenience, not complexity.
- **Closures everywhere**: First-class closures with concise syntax drive Groovy's DSL capability.
- **Optional typing**: Mix static and dynamic typing as needed. `@CompileStatic` for performance-critical code.

## Core Syntax

```groovy
// Semicolons optional, types optional
def name = "Alice"
String typed = "Bob"

// GStrings (interpolation)
def greeting = "Hello, ${name}! You are ${name.length()} chars long."
def multiline = """
    SELECT *
    FROM users
    WHERE name = '${name}'
""".stripIndent().trim()

// Lists and maps
def list = [1, 2, 3, 4, 5]
def map = [name: "Alice", age: 30, active: true]

// Safe navigation
def length = user?.address?.city?.length()  // null if any is null

// Elvis operator
def port = config.port ?: 8080  // default if null/false

// Spread operator
def names = users*.name  // collect .name from each element
```

## Closures

```groovy
// Closure syntax
def square = { int n -> n * n }
square(5)  // 25

// Implicit `it` parameter
def double = { it * 2 }
[1, 2, 3].collect(double)  // [2, 4, 6]

// Collection methods (like Ruby)
def numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

numbers.findAll { it % 2 == 0 }       // [2, 4, 6, 8, 10]
numbers.collect { it * it }            // [1, 4, 9, ...]
numbers.inject(0) { sum, n -> sum + n } // 55
numbers.groupBy { it % 3 }            // [0:[3,6,9], 1:[1,4,7,10], 2:[2,5,8]]
numbers.any { it > 5 }                // true
numbers.every { it > 0 }              // true

// Closure delegation (powers DSLs)
def configure(Closure cl) {
    def config = new Config()
    cl.delegate = config
    cl.resolveStrategy = Closure.DELEGATE_FIRST
    cl()
    return config
}
```

## Builders and DSLs

```groovy
// MarkupBuilder (XML/HTML generation)
def writer = new StringWriter()
def xml = new groovy.xml.MarkupBuilder(writer)
xml.users {
    user(id: 1) {
        name("Alice")
        email("alice@test.com")
    }
    user(id: 2) {
        name("Bob")
        email("bob@test.com")
    }
}

// JsonBuilder
def json = new groovy.json.JsonBuilder()
json {
    name "Alice"
    age 30
    hobbies "reading", "coding"
}
println json.toPrettyString()

// JsonSlurper (parsing)
def data = new groovy.json.JsonSlurper().parseText(jsonString)
println data.users[0].name
```

## Gradle Build Scripts

```groovy
// build.gradle (Groovy DSL — the most common Groovy use case)
plugins {
    id 'java'
    id 'application'
}

repositories {
    mavenCentral()
}

dependencies {
    implementation 'com.google.guava:guava:32.1.3-jre'
    testImplementation 'org.junit.jupiter:junit-jupiter:5.10.1'
}

application {
    mainClass = 'com.example.Main'
}

tasks.register('generateDocs') {
    doLast {
        // Groovy code runs here
        def docs = file('docs')
        docs.mkdirs()
        new File(docs, 'index.html').text = '<html>Generated</html>'
    }
}

test {
    useJUnitPlatform()
}
```

## Metaprogramming

```groovy
// Add methods to existing classes at runtime
String.metaClass.isPalindrome = {
    delegate == delegate.reverse()
}
"racecar".isPalindrome()  // true

// ExpandoMetaClass for dynamic behavior
class User {
    String name
}

User.metaClass.greet = { "Hello, I'm ${delegate.name}" }
new User(name: "Alice").greet()  // "Hello, I'm Alice"

// @CompileStatic for type safety and performance
@groovy.transform.CompileStatic
class Calculator {
    int add(int a, int b) { a + b }
}

// AST Transformations (compile-time metaprogramming)
@groovy.transform.ToString
@groovy.transform.EqualsAndHashCode
@groovy.transform.TupleConstructor
class Point {
    int x, y
}
```

## Conventions

1. **Use `@CompileStatic`** for performance-critical code. Dynamic dispatch is 10-100x slower than static.
2. **Use `def` for local variables** when the type is obvious. Use explicit types for method signatures and public APIs.
3. **GString injection risk**: Never use `"${userInput}"` in SQL or shell commands. Use parameterized queries.
4. **Prefer Groovy collection methods** (`findAll`, `collect`, `inject`) over Java streams in Groovy code.
5. **Use `?.` (safe navigation) and `?:` (Elvis)** instead of explicit null checks.
6. **In Gradle, prefer the Kotlin DSL** for new projects — better IDE support and type safety.

---

_Sources: Groovy Documentation (groovy-lang.org), Groovy in Action (König et al.), Gradle User Guide, Making Java Groovy (Kousen)_
