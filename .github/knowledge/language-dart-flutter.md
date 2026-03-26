# Dart & Flutter Best Practices

## Dart Language

### Type System (Sound Null Safety)

```dart
// Non-nullable by default
String name = 'Alice';     // Cannot be null
String? nickname;           // Nullable

// Null-aware operators
int length = nickname?.length ?? 0;
String display = nickname ?? 'Anonymous';
nickname ??= 'Default';    // Assign if null

// Late initialization
late final String config;   // Initialized before first use

// Type inference
var count = 42;             // Inferred as int
final name = 'Alice';      // Immutable reference
const pi = 3.14159;        // Compile-time constant
```

### Classes & Records

```dart
// Records (Dart 3.0+) — immutable, structural types
(String, int) userInfo = ('Alice', 30);
var (name, age) = userInfo;  // Destructuring

// Named fields
({String name, int age}) user = (name: 'Alice', age: 30);

// Classes with modern syntax
class User {
  final String name;
  final String email;
  final int age;

  const User({required this.name, required this.email, this.age = 0});

  @override
  String toString() => 'User($name, $email)';
}

// Factory constructors
class Logger {
  static final Logger _instance = Logger._internal();
  factory Logger() => _instance;
  Logger._internal();
}

// Extension methods
extension StringExtensions on String {
  bool get isEmail => contains('@') && contains('.');
  String get capitalized => '${this[0].toUpperCase()}${substring(1)}';
}

'hello'.capitalized;  // 'Hello'
```

### Sealed Classes & Pattern Matching (Dart 3.0+)

```dart
sealed class Result<T> {}
class Success<T> extends Result<T> { final T value; Success(this.value); }
class Failure<T> extends Result<T> { final String error; Failure(this.error); }
class Loading<T> extends Result<T> {}

// Exhaustive switch (compiler checks all cases)
String describe<T>(Result<T> result) => switch (result) {
  Success(value: var v) => 'Got: $v',
  Failure(error: var e) => 'Error: $e',
  Loading()             => 'Loading...',
};

// if-case
if (result case Success(value: var user)) {
  print('User: $user');
}

// Guard clauses in patterns
switch (value) {
  case int n when n > 0: print('positive');
  case int n when n < 0: print('negative');
  case int(): print('zero');
}
```

### Async/Await

```dart
Future<User> fetchUser(String id) async {
  final response = await http.get(Uri.parse('/users/$id'));
  if (response.statusCode != 200) {
    throw HttpException('Failed: ${response.statusCode}');
  }
  return User.fromJson(jsonDecode(response.body));
}

// Parallel
final results = await Future.wait([
  fetchUser('1'),
  fetchUser('2'),
  fetchUser('3'),
]);

// Streams
Stream<int> countUp(int max) async* {
  for (var i = 0; i < max; i++) {
    await Future.delayed(Duration(seconds: 1));
    yield i;
  }
}

await for (var n in countUp(10)) {
  print(n);
}
```

## Flutter

### Widget Design

```dart
// StatelessWidget — pure function of props
class Greeting extends StatelessWidget {
  final String name;
  const Greeting({super.key, required this.name});

  @override
  Widget build(BuildContext context) {
    return Text('Hello, $name!', style: Theme.of(context).textTheme.headlineMedium);
  }
}

// StatefulWidget — manages mutable state
class Counter extends StatefulWidget {
  const Counter({super.key});

  @override
  State<Counter> createState() => _CounterState();
}

class _CounterState extends State<Counter> {
  int _count = 0;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Text('Count: $_count'),
        ElevatedButton(
          onPressed: () => setState(() => _count++),
          child: const Text('Increment'),
        ),
      ],
    );
  }
}
```

### State Management

| Approach     | Complexity | Use Case                                     |
| ------------ | ---------- | -------------------------------------------- |
| **setState** | Simple     | Local widget state                           |
| **Provider** | Medium     | App-wide state (recommended by Flutter team) |
| **Riverpod** | Medium     | Type-safe Provider alternative               |
| **BLoC**     | High       | Complex business logic, event-driven         |
| **GetX**     | Low        | Rapid prototyping (controversial)            |

### Flutter Best Practices

1. **const constructors** — Use `const` wherever possible for widget reuse.
2. **Small widgets** — Extract widgets into separate classes, not methods.
3. **Keys** — Use keys in lists, animations, and form fields.
4. **Avoid deeply nested build methods.** Break into smaller widgets.
5. **Use `BuildContext` wisely** — Don't store it or use it across async gaps.

## Tooling

| Tool                   | Purpose                                  |
| ---------------------- | ---------------------------------------- |
| **dart analyze**       | Static analysis                          |
| **dart format**        | Code formatting                          |
| **dart fix**           | Auto-apply lint fixes                    |
| **flutter test**       | Testing                                  |
| **DevTools**           | Performance profiling, widget inspection |
| **very_good_analysis** | Strict lint rules                        |

---

_Sources: Dart documentation, Effective Dart, Flutter documentation, Flutter architectural overview_
