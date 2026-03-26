# Android Development Patterns — Jetpack, Architecture & Composition

## View System vs Jetpack Compose

Android has two UI frameworks, each with its own paradigm.

### View System (Legacy Imperative)

The original Android framework (since API 1) uses **imperative, hierarchy-based views**:

```kotlin
class MainActivity : AppCompatActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    setContentView(R.layout.activity_main)
    
    val button = findViewById<Button>(R.id.button)
    button.setOnClickListener { 
      updateUI() 
    }
  }
  
  fun updateUI() {
    findViewById<TextView>(R.id.text).text = "Updated"
  }
}
```

Views live in an XML hierarchy; you reference them by ID and mutate state imperatively. Inflation of XML layouts is asynchronous; complex animations require manual choreography.

**Performance**: Native views are optimized for the Android platform. Direct access to OS rendering.

### Jetpack Compose (Modern Declarative)

Compose (API 21+) is **reactive and compositional**:

```kotlin
@Composable
fun MainScreen() {
  var count by remember { mutableStateOf(0) }
  
  Column {
    Text("Count: $count")
    Button(onClick = { count++ }) {
      Text("Increment")
    }
  }
}

// Use it:
setContent { MainScreen() }
```

State changes trigger recomposition (re-running the composable to generate a new UI tree).

### Lifecycle Differences

| Aspect | Views | Compose |
|--------|-------|---------|
| State management | In View/Activity | In @Composable via `remember` |
| Updates | Imperative mutations | Declarative recomposition |
| Layout inflation | XML + LayoutInflater | Functional composition |
| Performance | Native optimizations | Recomposition overhead (improving) |
| Testing | Requires Espresso | Intrinsic semantics layer |

**Current Status (2026)**: Compose is production-ready but still optimizing. Android recommends Compose for new projects. View system remains stable for maintenance.

## MVVM Architecture with Jetpack Components

MVVM (Model-View-ViewModel) is the standard Android architecture:

### ViewModel

ViewModel holds UI-related state and survives configuration changes (rotation):

```kotlin
class CounterViewModel : ViewModel() {
  private val _count = MutableLiveData(0)
  val count: LiveData<Int> = _count
  
  fun increment() {
    _count.value = _count.value!! + 1
  }
}

// In Activity/Fragment (survives rotation):
class MainActivity : AppCompatActivity() {
  private val viewModel: CounterViewModel by viewModels()
  
  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    setContentView(R.layout.main)
    
    viewModel.count.observe(this) { count ->
      findViewById<TextView>(R.id.text).text = "$count"
    }
    findViewById<Button>(R.id.button).setOnClickListener {
      viewModel.increment()
    }
  }
}
```

ViewModel's scope extends beyond a single Activity lifecycle. Android retains ViewModels across configuration changes; they're cleared only when the Activity is fully destroyed.

### LiveData

LiveData is a **lifecycle-aware observable**:

```kotlin
val liveData: LiveData<String> = ...
liveData.observe(this) { value ->
  // Called automatically when this (Activity) is in the foreground
  // Automatically stops observing when destroyed
}
```

LiveData respects lifecycle events; observers are only notified when the Activity/Fragment is active.

**Caveat**: LiveData was popular pre-2020. Modern apps prefer **StateFlow** (Kotlin Coroutines).

## StateFlow & Coroutines

StateFlow is the **modern reactive state container**:

```kotlin
class CounterViewModel : ViewModel() {
  private val _count = MutableStateFlow(0)
  val count: StateFlow<Int> = _count.asStateFlow()
  
  fun increment() {
    _count.update { it + 1 }
  }
}

// In Compose:
@Composable
fun Counter(viewModel: CounterViewModel) {
  val count by viewModel.count.collectAsState()
  Button(onClick = { viewModel.increment() }) {
    Text("$count")
  }
}

// In Views + Coroutines:
lifecycleScope.launch {
  viewModel.count.collect { count ->
    textView.text = "$count"
  }
}
```

StateFlow advantages over LiveData:
- Works in any scope (not just Activities)
- Integrates with Coroutines
- Type-safe with Flow operations (map, filter)

## Room Database

Room is an **SQLite wrapper** that adds type safety:

```kotlin
@Entity(tableName = "users")
data class User(
  @PrimaryKey val id: Int,
  val name: String,
  val email: String
)

@Dao
interface UserDao {
  @Query("SELECT * FROM users WHERE id = :userId")
  suspend fun getUser(userId: Int): User
  
  @Insert
  suspend fun insert(user: User)
}

@Database(entities = [User::class], version = 1)
abstract class AppDatabase : RoomDatabase() {
  abstract fun userDao(): UserDao
}

// Usage:
val user = userDao.getUser(1)  // Suspend function
```

Room provides:
- **Schema verification at compile time** (typos in queries are caught)
- **Automatic migrations** (define migration paths for schema changes)
- **Flow-based queries** for reactive updates

A Room query that returns `Flow<T>` reemits whenever the data changes in the database.

## Hilt Dependency Injection

Hilt is **compile-time DI** for Android, built on Dagger:

```kotlin
@HiltViewModel
class CounterViewModel @Inject constructor(
  private val repository: UserRepository
) : ViewModel() {
  // Inject dependencies
}

@AndroidEntryPoint
class MainActivity : AppCompatActivity() {
  private val viewModel: CounterViewModel by viewModels()
}

// Module (provides bindings):
@Module
@InstallIn(SingletonComponent::class)
object RepositoryModule {
  @Provides
  fun provideUserRepository(api: UserApi): UserRepository {
    return UserRepositoryImpl(api)
  }
}
```

Hilt generates all boilerplate at compile time. Scopes (Singleton, ViewModel, Activity, Fragment) manage lifetime automatically.

## DataStore (Preferences)

DataStore replaces **SharedPreferences** for key-value storage:

```kotlin
val dataStore: DataStore<Preferences> = context.createDataStore("settings")

// Write:
dataStore.edit { preferences ->
  preferences[KEY_USER_ID] = 42
}

// Read (reactive):
dataStore.data.collect { preferences ->
  val userId = preferences[KEY_USER_ID] ?: 0
}
```

DataStore advantages:
- Asynchronous (Coroutines-based, never blocks UI)
- Type-safe via `Preferences.Key<T>`
- Atomic writes and migrations

## Navigation Component

Navigation Component manages **fragment back stacking** and deep links:

```kotlin
// In XML:
<navigation xmlns:android="http://schemas.android.com/apk/res/android">
  <fragment android:id="@+id/home" android:name="HomeFragment" />
  <fragment android:id="@+id/details" android:name="DetailsFragment" />
  <action android:id="@+id/to_details" app:destination="@id/details" />
</navigation>

// In Fragment:
findNavController().navigate(R.id.to_details)

// With arguments:
val bundle = bundleOf("userId" to 42)
findNavController().navigate(R.id.to_details, bundle)
```

Or in **Compose**:

```kotlin
NavHost(navController, startDestination = "home") {
  composable("home") { HomeScreen() }
  composable("details/{userId}") { backStackEntry ->
    DetailsScreen(userId = backStackEntry.arguments?.getString("userId"))
  }
}
```

Navigation Component handles back button, deep linking, and state preservation automatically.

## WorkManager (Background Tasks)

WorkManager executes **background work that must complete reliably**, even across app restarts:

```kotlin
class UploadWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
  override suspend fun doWork(): Result {
    return try {
      val result = uploadData()
      Result.success()
    } catch (e: Exception) {
      Result.retry()
    }
  }
}

// Enqueue:
val uploadWork = OneTimeWorkRequestBuilder<UploadWorker>().build()
WorkManager.getInstance(context).enqueueUniqueWork("upload", REPLACE, uploadWork)
```

WorkManager chooses the best execution strategy:
- **Foreground Service** (if charging and connected, immediate)
- **JobScheduler** (batched, system decides timing)
- **Broadcast receivers** (periodic)

Guarantees delivery even if app is killed.

## Gradle Build System

Android uses **Gradle** (Kotlin or Groovy DSL):

```kotlin
// build.gradle.kts
plugins {
  id("com.android.application")
  kotlin("android")
}

android {
  namespace = "com.example.app"
  compileSdk = 35
  
  defaultConfig {
    applicationId = "com.example.app"
    minSdk = 24
    targetSdk = 35
  }
  
  buildFeatures {
    compose = true
    buildConfig = true
  }
}

dependencies {
  implementation("androidx.compose.ui:ui:1.6.0")
  testImplementation("junit:junit:4.13.2")
}
```

Key concepts:
- **Flavors**: Different build variants (free, paid, dev)
- **Build Types**: debug, release (with ProGuard/R8)
- **Source Sets**: app/src/main, app/src/test, app/src/androidTest

## ProGuard & R8 Obfuscation

ProGuard/R8 **minifies, optimizes, and obfuscates** code for release builds:

```pro
# proguard-rules.pro
# Keep model classes for serialization
-keep class com.example.models.** { *; }

# Keep callback methods
-keepclassmembers class * {
  public <methods>;
}
```

R8 replaces ProGuard in modern Android. It:
- Removes unused code
- Renames classes/methods to single letters
- Optimizes bytecode

**Caveat**: Reflection, serialization, and annotation-based frameworks require ProGuard rules to prevent over-obfuscation.

## App Bundles & Play Store

Modern Android apps ship as **app bundles** (.aab), not APKs:

```bash
# Build bundle:
./gradlew bundleRelease

# Play Store generates per-device APKs (fewer unused resources)
# Users download only what their device needs
```

Bundles reduce app size by 30-40% compared to monolithic APKs.

## See Also

- **language-kotlin** — Kotlin idioms and concurrency patterns
- **architecture-clean-hexagonal** — General clean architecture principles
- **devops-cicd-patterns** — CI/CD for Android builds