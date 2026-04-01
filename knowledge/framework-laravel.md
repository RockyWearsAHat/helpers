# Laravel

## Eloquent ORM

### Models and Relationships

```php
class User extends Model
{
    protected $fillable = ['name', 'email', 'password'];
    protected $hidden = ['password', 'remember_token'];
    protected $casts = [
        'email_verified_at' => 'datetime',
        'settings' => 'array',
        'is_admin' => 'boolean',
    ];

    // Relationships
    public function posts(): HasMany
    {
        return $this->hasMany(Post::class);
    }

    public function profile(): HasOne
    {
        return $this->hasOne(Profile::class);
    }

    public function roles(): BelongsToMany
    {
        return $this->belongsToMany(Role::class)->withTimestamps()->withPivot('granted_by');
    }

    // Polymorphic
    public function images(): MorphMany
    {
        return $this->morphMany(Image::class, 'imageable');
    }
}

class Post extends Model
{
    protected $fillable = ['title', 'body', 'status'];

    public function user(): BelongsTo
    {
        return $this->belongsTo(User::class);
    }

    public function tags(): BelongsToMany
    {
        return $this->belongsToMany(Tag::class);
    }

    public function comments(): HasMany
    {
        return $this->hasMany(Comment::class)->latest();
    }
}
```

### Accessors and Mutators (Laravel 11 attribute casting)

```php
class User extends Model
{
    protected function name(): Attribute
    {
        return Attribute::make(
            get: fn (string $value) => ucfirst($value),
            set: fn (string $value) => strtolower($value),
        );
    }
}
```

### Scopes

```php
class Post extends Model
{
    // Local scope
    public function scopePublished(Builder $query): void
    {
        $query->where('status', 'published');
    }

    public function scopeByAuthor(Builder $query, User $user): void
    {
        $query->where('user_id', $user->id);
    }
}

// Usage: Post::published()->byAuthor($user)->latest()->paginate(20)
```

### Eloquent Query Builder

```php
// Basic queries
User::all();
User::find(1);
User::findOrFail(1);
User::where('active', true)->get();
User::where('age', '>', 18)->orderBy('name')->first();
User::where('email', 'like', '%@gmail.com')->count();
User::whereIn('role', ['admin', 'editor'])->get();
User::whereBetween('created_at', [now()->subMonth(), now()])->get();
User::whereNull('deleted_at')->get();

// Eager loading (prevent N+1)
$posts = Post::with(['user', 'tags', 'comments.user'])->get();
$posts = Post::with(['comments' => fn ($q) => $q->latest()->limit(5)])->get();

// Lazy eager loading
$user->load('posts');

// Aggregates
Post::where('user_id', 1)->avg('views');
Post::groupBy('status')->selectRaw('status, count(*) as count')->get();

// Chunking for large datasets
User::chunk(1000, function ($users) {
    foreach ($users as $user) { /* process */ }
});

// Insert/Update
Post::create(['title' => 'New', 'body' => 'Content', 'user_id' => 1]);
Post::updateOrCreate(['slug' => 'my-post'], ['title' => 'Updated']);
Post::upsert($records, uniqueBy: ['email'], update: ['name']); // bulk
```

## Routing

```php
// routes/web.php
Route::get('/', [HomeController::class, 'index'])->name('home');

Route::middleware('auth')->group(function () {
    Route::resource('posts', PostController::class);
    Route::post('posts/{post}/publish', [PostController::class, 'publish'])->name('posts.publish');
});

// Route model binding
Route::get('users/{user}', fn (User $user) => view('users.show', compact('user')));

// API routes
Route::prefix('api/v1')->middleware('api')->group(function () {
    Route::apiResource('users', UserApiController::class);
});
```

## Middleware

```php
// app/Http/Middleware/EnsureIsAdmin.php
class EnsureIsAdmin
{
    public function handle(Request $request, Closure $next): Response
    {
        if (! $request->user()?->is_admin) {
            abort(403, 'Unauthorized');
        }
        return $next($request);
    }
}

// Register in bootstrap/app.php (Laravel 11)
->withMiddleware(function (Middleware $middleware) {
    $middleware->alias(['admin' => EnsureIsAdmin::class]);
})
```

## Controllers

```php
class PostController extends Controller
{
    public function index()
    {
        $posts = Post::with('user')->published()->latest()->paginate(20);
        return view('posts.index', compact('posts'));
    }

    public function store(StorePostRequest $request)
    {
        $post = $request->user()->posts()->create($request->validated());
        return redirect()->route('posts.show', $post)->with('success', 'Post created!');
    }

    public function update(UpdatePostRequest $request, Post $post)
    {
        $this->authorize('update', $post);
        $post->update($request->validated());
        return redirect()->route('posts.show', $post);
    }

    public function destroy(Post $post)
    {
        $this->authorize('delete', $post);
        $post->delete();
        return redirect()->route('posts.index');
    }
}
```

### Form Requests

```php
class StorePostRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    public function rules(): array
    {
        return [
            'title' => ['required', 'string', 'max:300'],
            'body' => ['required', 'string'],
            'category_id' => ['required', 'exists:categories,id'],
            'tags' => ['array', 'max:10'],
            'tags.*' => ['exists:tags,id'],
        ];
    }
}
```

## Blade Templates

```blade
{{-- layouts/app.blade.php --}}
<!DOCTYPE html>
<html>
<body>
    @include('partials.nav')
    <main>
        @yield('content')
    </main>
    @stack('scripts')
</body>
</html>

{{-- posts/index.blade.php --}}
@extends('layouts.app')

@section('content')
    @forelse ($posts as $post)
        <x-post-card :post="$post" />
    @empty
        <p>No posts found.</p>
    @endforelse

    {{ $posts->links() }}
@endsection
```

### Blade Components

```php
// app/View/Components/PostCard.php or anonymous: resources/views/components/post-card.blade.php
<article {{ $attributes->merge(['class' => 'card']) }}>
    <h2>{{ $post->title }}</h2>
    <p>{{ $post->user->name }} · {{ $post->created_at->diffForHumans() }}</p>
    {{ $slot }}
</article>
```

## Authentication

| Package   | Use Case                                                   |
| --------- | ---------------------------------------------------------- |
| Breeze    | Minimal auth scaffolding (login, register, password reset) |
| Jetstream | Full featured (teams, 2FA, API tokens, profile management) |
| Sanctum   | SPA authentication + API tokens                            |
| Passport  | Full OAuth2 server                                         |

```php
// Sanctum API auth
Route::middleware('auth:sanctum')->get('/user', fn (Request $request) => $request->user());
```

## Authorization (Gates and Policies)

```php
// Policy
class PostPolicy
{
    public function update(User $user, Post $post): bool
    {
        return $user->id === $post->user_id;
    }

    public function delete(User $user, Post $post): bool
    {
        return $user->id === $post->user_id || $user->is_admin;
    }
}

// Gate (simple closure-based)
Gate::define('access-admin', fn (User $user) => $user->is_admin);

// Usage
$this->authorize('update', $post);       // in controller
@can('update', $post) ... @endcan         // in Blade
Gate::allows('access-admin')              // anywhere
```

## Queues and Jobs

```php
class ProcessPodcast implements ShouldQueue
{
    use Dispatchable, InteractsWithQueue, Queueable, SerializesModels;

    public int $tries = 3;
    public int $backoff = 60;

    public function __construct(private Podcast $podcast) {}

    public function handle(AudioProcessor $processor): void
    {
        $processor->process($this->podcast);
    }

    public function failed(Throwable $exception): void
    {
        Log::error("Podcast processing failed: {$exception->getMessage()}");
    }
}

// Dispatch
ProcessPodcast::dispatch($podcast);
ProcessPodcast::dispatch($podcast)->onQueue('audio')->delay(now()->addMinutes(5));
```

```bash
php artisan queue:work --queue=high,default --tries=3
```

## Events and Listeners

```php
// Event
class OrderShipped
{
    use Dispatchable, SerializesModels;
    public function __construct(public Order $order) {}
}

// Listener
class SendShipmentNotification implements ShouldQueue
{
    public function handle(OrderShipped $event): void
    {
        $event->order->user->notify(new OrderShippedNotification($event->order));
    }
}

// Dispatch
OrderShipped::dispatch($order);
```

## Collections

```php
$users = collect([['name' => 'Alice', 'score' => 90], ['name' => 'Bob', 'score' => 75]]);

$users->pluck('name');                    // ['Alice', 'Bob']
$users->sortByDesc('score');
$users->filter(fn ($u) => $u['score'] > 80);
$users->map(fn ($u) => strtoupper($u['name']));
$users->groupBy('role');
$users->sum('score');
$users->unique('email');
$users->chunk(100)->each(fn ($chunk) => process($chunk));

// Method chaining for complex transformations
$result = Order::all()
    ->groupBy('status')
    ->map(fn ($orders) => $orders->sum('total'))
    ->sortDesc();
```

## Artisan CLI

```bash
php artisan make:model Post -mfsc     # model + migration + factory + seeder + controller
php artisan make:request StorePostRequest
php artisan make:policy PostPolicy --model=Post
php artisan make:job ProcessPodcast
php artisan make:event OrderShipped
php artisan make:listener SendNotification --event=OrderShipped
php artisan migrate:fresh --seed       # drop all, re-migrate, seed
php artisan tinker                     # REPL with Laravel loaded
php artisan route:list                 # show all routes
php artisan optimize                   # cache config, routes, views
```

## Testing

```php
class PostTest extends TestCase
{
    use RefreshDatabase;

    public function test_user_can_create_post(): void
    {
        $user = User::factory()->create();
        $response = $this->actingAs($user)->post('/posts', [
            'title' => 'Test Post',
            'body' => 'Content here',
        ]);
        $response->assertRedirect();
        $this->assertDatabaseHas('posts', ['title' => 'Test Post', 'user_id' => $user->id]);
    }

    public function test_guest_cannot_create_post(): void
    {
        $this->post('/posts', ['title' => 'Test'])->assertRedirect('/login');
    }

    public function test_api_returns_paginated_posts(): void
    {
        Post::factory()->count(30)->create();
        $this->getJson('/api/posts')
            ->assertOk()
            ->assertJsonCount(20, 'data')
            ->assertJsonStructure(['data' => [['id', 'title', 'created_at']]]);
    }
}
```

## Scheduling

```php
// routes/console.php (Laravel 11)
Schedule::command('reports:generate')->dailyAt('02:00');
Schedule::job(new CleanupJob)->weekly();
Schedule::call(fn () => cache()->flush())->hourly();
```

```bash
# Cron entry (server)
* * * * * cd /path-to-project && php artisan schedule:run >> /dev/null 2>&1
```
