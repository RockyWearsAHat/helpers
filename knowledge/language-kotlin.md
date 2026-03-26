# Kotlin Conventions and Idioms

## Kotlin Philosophy

Kotlin is pragmatic, concise, safe, and interoperable (with Java). It fixes Java's pain points without inventing unnecessary complexity.

- **Null safety built in**: The billion-dollar mistake, fixed at the type system level.
- **Concise**: Less boilerplate than Java, but readable and explicit.
- **Coroutines**: First-class structured concurrency without callback hell.

## Null Safety

```kotlin
// Non-null by default
var name: String = "Alice"  // Cannot be null
// name = null  // ❌ Compile error

// Nullable types marked explicitly with ?
var nickname: String? = null  // Can be null

// Safe call
val length: Int? = nickname?.length

// Elvis operator (default value)
val displayName = nickname ?: "Anonymous"

// Smart cast — compiler tracks null checks
fun process(name: String?) {
    if (name != null) {
        // name is automatically String (not String?) here
        println(name.uppercase())
    }
}

// let — scope function for nullable
nickname?.let { nick ->
    println("Nickname: $nick")
}

// !! (force unwrap) — avoid unless you're certain
val len = nickname!!.length  // Throws NPE if null — use sparingly
```

## Data Classes

```kotlin
// Replace Java POJOs — auto-generates equals, hashCode, toString, copy, componentN
data class User(
    val id: String,
    val name: String,
    val email: String,
    val role: Role = Role.USER  // Default value
)

// Copy with modifications
val admin = user.copy(role = Role.ADMIN)

// Destructuring
val (id, name, email) = user
```

## Sealed Classes & Interfaces

```kotlin
// Restricted hierarchy — compiler knows all subtypes
sealed interface Result<out T> {
    data class Success<T>(val value: T) : Result<T>
    data class Failure(val error: Throwable) : Result<Nothing>
    data object Loading : Result<Nothing>
}

// Exhaustive when (no else needed)
fun <T> handle(result: Result<T>) {
    when (result) {
        is Result.Success -> println("Got: ${result.value}")
        is Result.Failure -> println("Error: ${result.error.message}")
        Result.Loading    -> println("Loading...")
    }
}

// Sealed class for state machines
sealed class UiState {
    data object Loading : UiState()
    data class Content(val items: List<Item>) : UiState()
    data class Error(val message: String, val retry: () -> Unit) : UiState()
}
```

## Coroutines

```kotlin
import kotlinx.coroutines.*

// Launch (fire and forget)
val job = scope.launch {
    val data = fetchData()
    updateUi(data)
}

// Async (returns a value)
val deferred = scope.async {
    fetchData()
}
val result = deferred.await()

// Parallel decomposition
coroutineScope {
    val user = async { fetchUser(id) }
    val posts = async { fetchPosts(id) }
    val config = async { fetchConfig() }

    display(user.await(), posts.await(), config.await())
}

// Structured concurrency — parent waits for children
suspend fun processAll(items: List<Item>) = coroutineScope {
    items.map { item ->
        async { process(item) }
    }.awaitAll()
}

// Cancellation is cooperative
suspend fun longTask() {
    repeat(1000) { i ->
        ensureActive()  // Check for cancellation
        doWork(i)
    }
}

// Dispatchers
withContext(Dispatchers.IO) {      // I/O operations
    readFile()
}
withContext(Dispatchers.Default) {  // CPU-intensive
    computeHash()
}
withContext(Dispatchers.Main) {     // UI thread (Android/Desktop)
    updateView()
}
```

## Flow (Reactive Streams)

```kotlin
import kotlinx.coroutines.flow.*

// Cold stream — emits values on collection
fun numberFlow(): Flow<Int> = flow {
    for (i in 1..10) {
        delay(100)
        emit(i)
    }
}

// Operators (like Rx but simpler)
numberFlow()
    .filter { it % 2 == 0 }
    .map { it * it }
    .take(3)
    .collect { println(it) }  // 4, 16, 36

// StateFlow (observable state)
class ViewModel {
    private val _uiState = MutableStateFlow<UiState>(UiState.Loading)
    val uiState: StateFlow<UiState> = _uiState.asStateFlow()

    fun load() {
        viewModelScope.launch {
            _uiState.value = UiState.Loading
            try {
                val data = repository.fetch()
                _uiState.value = UiState.Content(data)
            } catch (e: Exception) {
                _uiState.value = UiState.Error(e.message ?: "Unknown error")
            }
        }
    }
}

// SharedFlow (event bus, one-to-many)
private val _events = MutableSharedFlow<Event>()
val events: SharedFlow<Event> = _events.asSharedFlow()
```

## Extension Functions

```kotlin
// Add methods to existing types without inheritance
fun String.isEmail(): Boolean =
    matches(Regex("^[\\w.-]+@[\\w.-]+\\.[a-zA-Z]{2,}$"))

"user@example.com".isEmail()  // true

// Extension on nullable types
fun String?.orEmpty(): String = this ?: ""

// Extension properties
val String.wordCount: Int
    get() = split("\\s+".toRegex()).size

// Scope functions (stdlib extensions)
// let — transform, handle nullable
val length = name?.let { it.trim().length }

// apply — configure an object (returns the object)
val user = User().apply {
    name = "Alice"
    email = "alice@test.com"
}

// also — side effects (returns the object)
val user = createUser().also { logger.info("Created: $it") }

// run — compute with receiver (returns the result)
val result = service.run {
    connect()
    query("SELECT 1")
}

// with — like run but not an extension
val csv = with(StringBuilder()) {
    appendLine("name,email")
    users.forEach { appendLine("${it.name},${it.email}") }
    toString()
}
```

## Idiomatic Kotlin

```kotlin
// String templates
"Hello, $name! You are ${age + 1} next year."

// when expression (replaces switch)
val label = when {
    score >= 90 -> "A"
    score >= 80 -> "B"
    score >= 70 -> "C"
    else        -> "F"
}

// Single-expression functions
fun Double.toCelsius() = (this - 32) * 5 / 9

// Collection operations
val adults = people.filter { it.age >= 18 }
val names = people.map { it.name }
val byCity = people.groupBy { it.city }
val oldest = people.maxByOrNull { it.age }
val (minors, adults) = people.partition { it.age < 18 }

// require/check for preconditions
fun setAge(age: Int) {
    require(age >= 0) { "Age must be non-negative: $age" }
    // ...
}

fun process() {
    val state = checkNotNull(currentState) { "State not initialized" }
    check(state.isReady) { "State must be ready, was: $state" }
}

// use — auto-close resources (like try-with-resources)
File("data.txt").bufferedReader().use { reader ->
    reader.lineSequence().forEach { println(it) }
}

// Delegation
class Preferences(private val map: Map<String, Any>) {
    val name: String by map
    val age: Int by map
}

// Lazy initialization
val expensiveValue: String by lazy {
    computeExpensiveValue()
}
```

## Error Handling

```kotlin
// Kotlin uses unchecked exceptions (no checked exceptions)
// Use sealed classes for expected failure modes instead of exceptions

// Result type (stdlib)
fun parse(input: String): Result<Int> = runCatching {
    input.toInt()
}

parse("42").getOrNull()      // 42
parse("abc").getOrDefault(0) // 0
parse("abc").getOrElse { e -> log(e); -1 }

// Or use a custom sealed type (preferred for domain errors)
sealed interface ParseResult {
    data class Success(val value: Int) : ParseResult
    data class Invalid(val input: String) : ParseResult
}
```

## Tooling

| Tool                      | Purpose                     |
| ------------------------- | --------------------------- |
| **ktlint**                | Linting + formatting        |
| **detekt**                | Static analysis             |
| **Gradle** (Kotlin DSL)   | Build system                |
| **JUnit 5** / **kotest**  | Testing                     |
| **MockK**                 | Mocking (Kotlin-native)     |
| **kotlinx.serialization** | Multiplatform serialization |

---

_Sources: Kotlin documentation, Kotlin in Action (Jemerov/Isakova), Effective Kotlin (Marcin Moskala), Android developer guidelines, KotlinConf talks_
