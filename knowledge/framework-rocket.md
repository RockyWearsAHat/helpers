# Rocket (Rust Web Framework)

## Core Architecture

Rocket is Rust's most ergonomic web framework, prioritizing type safety and developer experience. It uses procedural macros extensively — routes are defined with attribute macros, and request data is automatically parsed and validated through Rust's type system. If it compiles, it probably works.

### Minimal Application

```rust
#[macro_use] extern crate rocket;

#[get("/")]
fn index() -> &'static str {
    "Hello, Rocket!"
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
```

### Rocket.toml Configuration

```toml
[default]
address = "127.0.0.1"
port = 8000
workers = 16
log_level = "normal"
temp_dir = "/tmp"
limits = { form = "64 kB", json = "1 MiB" }

[debug]
log_level = "debug"

[release]
address = "0.0.0.0"
port = 443
tls = { certs = "certs/cert.pem", key = "certs/key.pem" }
secret_key = "generate-with-openssl-rand-base64-32"
```

Profiles: `debug`, `release`, `default` (applies to all). Environment variable overrides: `ROCKET_PORT=9000`, `ROCKET_LOG_LEVEL=debug`.

## Routing

```rust
#[get("/")]
fn index() -> &'static str { "Hello" }

#[post("/users", data = "<user>")]
fn create_user(user: Json<NewUser>) -> Json<User> { ... }

#[put("/users/<id>", data = "<user>")]
fn update_user(id: i32, user: Json<UpdateUser>) -> Json<User> { ... }

#[delete("/users/<id>")]
fn delete_user(id: i32) -> Status { Status::NoContent }

// Multiple segments
#[get("/files/<path..>")]
fn serve_file(path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(path)).ok()
}

// Optional parameters
#[get("/users?<page>&<limit>")]
fn list_users(page: Option<u32>, limit: Option<u32>) -> Json<Vec<User>> {
    let page = page.unwrap_or(1);
    let limit = limit.unwrap_or(20);
    ...
}

// Rank-based routing (lower rank = higher priority)
#[get("/user/<id>", rank = 1)]
fn user_by_id(id: u32) -> ... { }

#[get("/user/<name>", rank = 2)]
fn user_by_name(name: &str) -> ... { }

// Mounting
rocket::build()
    .mount("/", routes![index])
    .mount("/api/v1", routes![list_users, create_user, update_user, delete_user])
```

### Route Attribute Syntax

```rust
#[get("/path")]         // GET
#[post("/path")]        // POST
#[put("/path")]         // PUT
#[delete("/path")]      // DELETE
#[patch("/path")]       // PATCH
#[head("/path")]        // HEAD
#[options("/path")]     // OPTIONS
```

## Request Guards

Request guards are Rocket's central abstraction. Any type that implements `FromRequest` can appear as a handler parameter. Rocket calls `from_request()` before the handler runs — if any guard fails, the handler never executes.

```rust
// Custom request guard
#[derive(Debug)]
struct AuthUser {
    id: i32,
    role: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthUser {
    type Error = AuthError;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let token = req.headers().get_one("Authorization");

        match token {
            None => Outcome::Error((Status::Unauthorized, AuthError::MissingToken)),
            Some(token) => {
                match validate_token(token).await {
                    Ok(user) => Outcome::Success(user),
                    Err(_) => Outcome::Error((Status::Unauthorized, AuthError::InvalidToken)),
                }
            }
        }
    }
}

// Use as handler parameter — automatically validated
#[get("/profile")]
fn profile(user: AuthUser) -> Json<Profile> {
    Json(get_profile(user.id))
}

// Optional guard (doesn't fail the request)
#[get("/content")]
fn content(user: Option<AuthUser>) -> Template {
    if let Some(user) = user {
        Template::render("content_authed", context! { user })
    } else {
        Template::render("content_anon", context! {})
    }
}
```

### Built-in Guards

| Guard                      | Source                        | Type          |
| -------------------------- | ----------------------------- | ------------- |
| `&str`, `String`           | Dynamic path segment          | Path param    |
| `i32`, `u64`, `uuid::Uuid` | Dynamic path segment (parsed) | Path param    |
| `Json<T>`                  | Request body                  | Data guard    |
| `Form<T>`                  | Form body                     | Data guard    |
| `&ContentType`             | Content-Type header           | Request guard |
| `&Accept`                  | Accept header                 | Request guard |
| `SocketAddr`               | Client address                | Request guard |
| `&State<T>`                | Managed application state     | Request guard |
| `&CookieJar<'_>`           | Cookies                       | Request guard |
| `Option<T>`                | Makes any guard optional      | Wrapper       |
| `Result<T, E>`             | Catches guard failures        | Wrapper       |

## Data Guards

Data guards parse the request body. Any type implementing `FromData` can be used with the `data = "<param>"` attribute.

```rust
use rocket::serde::{Deserialize, json::Json};
use rocket::form::Form;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct NewUser {
    name: String,
    email: String,
}

// JSON body
#[post("/users", data = "<user>")]
fn create_user(user: Json<NewUser>) -> Json<User> { ... }

// Form data
#[derive(FromForm)]
struct LoginForm<'r> {
    email: &'r str,
    password: &'r str,
    #[field(name = "remember")]
    remember_me: bool,
}

#[post("/login", data = "<form>")]
fn login(form: Form<LoginForm<'_>>) -> Redirect { ... }

// Form with validation
#[derive(FromForm)]
struct Registration<'r> {
    #[field(validate = len(2..100))]
    name: &'r str,
    #[field(validate = contains('@'))]
    email: &'r str,
    #[field(validate = len(8..))]
    password: &'r str,
    #[field(validate = eq(self.password))]
    confirm: &'r str,
}
```

## Responders

Any handler return type must implement `Responder`. Rocket includes many built-in responders.

```rust
use rocket::response::{status, content, Redirect, Flash};
use rocket::fs::NamedFile;

// String types
#[get("/")] fn text() -> &'static str { "plain text" }
#[get("/")] fn html() -> content::RawHtml<&'static str> { content::RawHtml("<h1>HTML</h1>") }

// JSON
#[get("/data")] fn data() -> Json<Vec<User>> { Json(get_users()) }

// Status codes
#[post("/users")] fn create() -> status::Created<Json<User>> {
    status::Created::new("/users/1").body(Json(user))
}
#[get("/missing")] fn missing() -> status::NotFound<&'static str> {
    status::NotFound("not here")
}

// Files
#[get("/file")] async fn file() -> Option<NamedFile> {
    NamedFile::open("static/file.txt").await.ok() // None → 404
}

// Redirect
#[get("/old")] fn old() -> Redirect { Redirect::to(uri!(new_page)) }

// Flash messages (stored in cookies)
#[post("/login")] fn login() -> Flash<Redirect> {
    Flash::success(Redirect::to(uri!(dashboard)), "Logged in!")
}
#[get("/dashboard")] fn dashboard(flash: Option<FlashMessage<'_>>) -> Template { ... }

// Custom responder
#[derive(Responder)]
#[response(status = 201, content_type = "json")]
struct CreatedResponse {
    body: String,
    #[response(ignore)]
    id: i32,
}
```

## Fairings (Middleware)

Fairings are Rocket's hook system — they don't wrap handlers (like traditional middleware) but respond to lifecycle events.

```rust
use rocket::fairing::{Fairing, Info, Kind};

struct RequestTimer;

#[rocket::async_trait]
impl Fairing for RequestTimer {
    fn info(&self) -> Info {
        Info { name: "Request Timer", kind: Kind::Request | Kind::Response }
    }

    async fn on_request(&self, req: &mut Request<'_>, _: &mut Data<'_>) {
        req.local_cache(|| Instant::now());
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        let start = req.local_cache(|| Instant::now());
        let duration = start.elapsed();
        res.set_header(Header::new("X-Response-Time", format!("{}ms", duration.as_millis())));
    }
}

// Attach
rocket::build()
    .attach(RequestTimer)
    .attach(AdHoc::on_liftoff("Liftoff", |_| Box::pin(async {
        println!("🚀 Rocket has launched!");
    })))
```

### Fairing Callbacks

| Callback      | When                        | Can Modify              |
| ------------- | --------------------------- | ----------------------- |
| `on_ignite`   | During build, before launch | Rocket instance         |
| `on_liftoff`  | After server starts         | Nothing (informational) |
| `on_request`  | Before routing              | Request + Data          |
| `on_response` | Before sending response     | Response                |
| `on_shutdown` | Server shutting down        | Nothing (cleanup)       |

## Managed State

```rust
struct AppConfig {
    api_key: String,
    max_uploads: usize,
}

struct DbPool(Pool<Postgres>);

#[launch]
fn rocket() -> _ {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(&database_url)
        .unwrap();

    rocket::build()
        .manage(AppConfig {
            api_key: env::var("API_KEY").unwrap(),
            max_uploads: 10,
        })
        .manage(DbPool(pool))
        .mount("/", routes![handler])
}

#[get("/data")]
fn handler(config: &State<AppConfig>, db: &State<DbPool>) -> Json<Data> {
    // config and db are automatically extracted
    ...
}
```

## Database Integration (rocket_db_pools)

```rust
use rocket_db_pools::{sqlx, Database, Connection};

#[derive(Database)]
#[database("mydb")]
struct MyDb(sqlx::PgPool);

// Rocket.toml
// [default.databases.mydb]
// url = "postgres://user:pass@localhost/db"
// max_connections = 5

#[get("/users/<id>")]
async fn get_user(mut db: Connection<MyDb>, id: i32) -> Option<Json<User>> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
        .fetch_optional(&mut **db)
        .await
        .ok()
        .flatten()
        .map(Json)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(MyDb::init())
        .mount("/", routes![get_user])
}
```

## Templates

```rust
use rocket_dyn_templates::{Template, context};

#[get("/")]
fn index() -> Template {
    Template::render("index", context! {
        title: "Home",
        users: get_users(),
    })
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Template::fairing())
        .mount("/", routes![index])
}
```

Templates directory: `templates/`. Supports Tera (`.html.tera`) and Handlebars (`.html.hbs`).

## Catchers (Error Pages)

```rust
#[catch(404)]
fn not_found(req: &Request<'_>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "error": "Not Found",
        "path": req.uri().path().as_str(),
    }))
}

#[catch(500)]
fn internal_error() -> &'static str {
    "Internal Server Error"
}

#[catch(default)]
fn default_catcher(status: Status, req: &Request<'_>) -> String {
    format!("{} ({})", status, req.uri())
}

rocket::build()
    .register("/", catchers![not_found, internal_error, default_catcher])
    .register("/api", catchers![api_not_found]) // scoped catchers
```

## Sentinels

Sentinels verify at launch time that the application is correctly configured. If a type used in a route requires managed state or a database that wasn't attached, Rocket will abort at startup instead of failing at runtime.

```rust
// This will fail at launch if MyDb isn't attached
#[get("/users")]
async fn list_users(db: Connection<MyDb>) -> Json<Vec<User>> { ... }

// Rocket checks:
// - Is MyDb managed? ✓ or abort
// - Is Template fairing attached? ✓ or abort
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rocket::local::asynchronous::Client;
    use rocket::http::{ContentType, Status};

    async fn client() -> Client {
        Client::tracked(rocket::build()
            .mount("/", routes![index, create_user, get_user]))
            .await
            .unwrap()
    }

    #[rocket::async_test]
    async fn test_index() {
        let client = client().await;
        let response = client.get("/").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().await.unwrap(), "Hello, Rocket!");
    }

    #[rocket::async_test]
    async fn test_create_user() {
        let client = client().await;
        let response = client.post("/users")
            .header(ContentType::JSON)
            .body(r#"{"name": "Test", "email": "test@example.com"}"#)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);
    }

    #[rocket::async_test]
    async fn test_not_found() {
        let client = client().await;
        let response = client.get("/nonexistent").dispatch().await;
        assert_eq!(response.status(), Status::NotFound);
    }
}
```

## Comparison: Rocket vs Actix Web

| Feature             | Rocket                       | Actix Web                 |
| ------------------- | ---------------------------- | ------------------------- |
| Ergonomics          | Best-in-class (macros)       | Good (explicit)           |
| Request parsing     | Request guards (type-driven) | Extractors (trait-based)  |
| Middleware          | Fairings (lifecycle hooks)   | Transform trait / from_fn |
| Config              | Rocket.toml + env vars       | Code-based                |
| Compile-time checks | Sentinels                    | None                      |
| Performance         | Very good                    | Slightly faster           |
| Async runtime       | Tokio (built-in)             | Tokio (built-in)          |
| Maturity            | Stable, v0.5                 | Stable, v4                |
| Learning curve      | Lower (more magic)           | Higher (more explicit)    |
| Flash messages      | Built-in                     | Manual                    |
| Forms               | #[derive(FromForm)]          | serde                     |
| Startup validation  | Yes (sentinels)              | No (runtime errors)       |

Rocket trades a bit of performance for significantly better developer experience. Choose Rocket when ergonomics matter; choose Actix Web when you need maximum throughput or more control over the HTTP layer.
