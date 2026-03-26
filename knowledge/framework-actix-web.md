# Actix Web (Rust)

## Overview

Actix Web is Rust's most battle-tested web framework. It runs on Tokio, supports multi-threaded request handling, and achieves top-tier TechEmpower benchmarks. Despite the name, the actor system (actix) is optional — most apps use it as a straightforward async web framework.

## App & HttpServer

```rust
use actix_web::{web, App, HttpServer, HttpResponse};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .app_data(web::Data::new(AppState { db: pool.clone() }))
            .configure(api_config)
            .service(
                web::scope("/api/v1")
                    .wrap(auth_middleware::Auth)
                    .service(user_routes())
            )
            .default_service(web::to(|| HttpResponse::NotFound()))
    })
    .bind("0.0.0.0:8080")?
    .workers(num_cpus::get())  // defaults to physical CPUs
    .run()
    .await
}
```

The closure in `HttpServer::new` runs per worker thread — each worker gets its own `App` instance. `web::Data` (wrapped in Arc) is shared across workers.

## Extractors

Extractors pull typed data from requests. They're the primary API for accessing request data:

```rust
use actix_web::{web, HttpRequest};

// Path parameters
#[get("/users/{id}")]
async fn get_user(path: web::Path<i64>) -> impl Responder {
    let user_id = path.into_inner();
    HttpResponse::Ok().json(user)
}

// Multiple path params via tuple
#[get("/users/{user_id}/posts/{post_id}")]
async fn get_post(path: web::Path<(i64, i64)>) -> impl Responder {
    let (user_id, post_id) = path.into_inner();
    ...
}

// Query string
#[derive(Deserialize)]
struct Pagination {
    page: Option<u32>,
    per_page: Option<u32>,
}

#[get("/items")]
async fn list_items(query: web::Query<Pagination>) -> impl Responder {
    let page = query.page.unwrap_or(1);
    ...
}

// JSON body
#[derive(Deserialize, Validate)]
struct CreateUser {
    #[validate(length(min = 1, max = 100))]
    name: String,
    #[validate(email)]
    email: String,
}

#[post("/users")]
async fn create_user(body: web::Json<CreateUser>) -> impl Responder {
    let user = body.into_inner();
    ...
}

// Form data
#[post("/login")]
async fn login(form: web::Form<LoginForm>) -> impl Responder { ... }

// Application state
#[get("/health")]
async fn health(state: web::Data<AppState>) -> impl Responder {
    let db_ok = state.db.ping().await.is_ok();
    HttpResponse::Ok().json(serde_json::json!({"db": db_ok}))
}

// Raw request (escape hatch)
async fn raw(req: HttpRequest) -> impl Responder {
    let ua = req.headers().get("User-Agent");
    ...
}
```

### Extractor Ordering

Extractors execute left-to-right in the function signature. Body extractors (Json, Form, Payload) can only appear once — the body stream is consumed.

### Custom Extractor Error Handling

```rust
let json_cfg = web::JsonConfig::default()
    .limit(4096)
    .error_handler(|err, _req| {
        let detail = err.to_string();
        actix_web::error::InternalError::from_response(
            err,
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": "invalid_json",
                "detail": detail
            }))
        ).into()
    });

App::new().app_data(json_cfg)
```

## Routing

```rust
use actix_web::web;

// Attribute macros (most common)
#[get("/")]
#[post("/users")]
#[put("/users/{id}")]
#[delete("/users/{id}")]
#[patch("/users/{id}")]

// Manual registration
fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/users")
            .route(web::get().to(list_users))
            .route(web::post().to(create_user))
    );
    cfg.service(
        web::resource("/users/{id}")
            .route(web::get().to(get_user))
            .route(web::put().to(update_user))
            .route(web::delete().to(delete_user))
    );
}

// Scopes (route groups)
web::scope("/api")
    .guard(guard::Header("content-type", "application/json"))
    .service(users_scope())
```

## Guards

Guards control which requests match a route:

```rust
use actix_web::guard;

web::resource("/api")
    .guard(guard::Post())
    .guard(guard::Header("content-type", "application/json"))
    .guard(guard::fn_guard(|ctx| {
        ctx.head().headers().contains_key("x-api-key")
    }))
    .to(handler)
```

## Middleware

### Built-in Middleware

```rust
use actix_web::middleware;
use actix_cors::Cors;

App::new()
    .wrap(middleware::Logger::default())          // request logging
    .wrap(middleware::Compress::default())         // gzip/deflate/br
    .wrap(middleware::NormalizePath::trim())       // trailing slash handling
    .wrap(
        Cors::default()
            .allowed_origin("https://example.com")
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec!["Authorization", "Content-Type"])
            .max_age(3600)
    )
```

### Custom Middleware

```rust
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use futures::future::{ok, Ready, LocalBoxFuture};

pub struct Auth;

impl<S, B> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware { service })
    }
}

pub struct AuthMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, ctx: &mut core::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Check auth header
        let auth = req.headers().get("Authorization");
        if auth.is_none() {
            return Box::pin(async {
                Err(actix_web::error::ErrorUnauthorized("missing auth"))
            });
        }
        let fut = self.service.call(req);
        Box::pin(async move { fut.await })
    }
}
```

### Simpler: wrap_fn

```rust
App::new().wrap_fn(|req, srv| {
    let start = Instant::now();
    let fut = srv.call(req);
    async move {
        let res = fut.await?;
        let elapsed = start.elapsed();
        log::info!("Request took: {:?}", elapsed);
        Ok(res)
    }
})
```

## Error Handling

```rust
use actix_web::{error, HttpResponse};
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
enum AppError {
    #[display("not found: {}", _0)]
    NotFound(String),
    #[display("validation error: {}", _0)]
    Validation(String),
    #[display("internal error")]
    Internal,
}

impl error::ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::NotFound(msg) =>
                HttpResponse::NotFound().json(serde_json::json!({"error": msg})),
            AppError::Validation(msg) =>
                HttpResponse::BadRequest().json(serde_json::json!({"error": msg})),
            AppError::Internal =>
                HttpResponse::InternalServerError().finish(),
        }
    }
}

// Use in handlers — the ? operator works naturally
async fn get_user(id: web::Path<i64>, db: web::Data<Pool>) -> Result<HttpResponse, AppError> {
    let user = db.find_user(*id).await
        .map_err(|_| AppError::Internal)?
        .ok_or_else(|| AppError::NotFound(format!("user {}", id)))?;
    Ok(HttpResponse::Ok().json(user))
}
```

## Application State

```rust
struct AppState {
    db: PgPool,
    redis: redis::Client,
    config: AppConfig,
}

// Shared across workers (Arc internally)
App::new()
    .app_data(web::Data::new(AppState {
        db: PgPool::connect(&database_url).await?,
        redis: redis::Client::open(redis_url)?,
        config: AppConfig::from_env(),
    }))
```

## Database Integration

### SQLx (async, compile-time checked)

```rust
use sqlx::PgPool;

async fn list_users(pool: web::Data<PgPool>) -> Result<HttpResponse, Error> {
    let users = sqlx::query_as!(User, "SELECT id, name, email FROM users")
        .fetch_all(pool.get_ref())
        .await?;
    Ok(HttpResponse::Ok().json(users))
}
```

### Connection Pooling

| Crate               | Type                                   | Backends                |
| ------------------- | -------------------------------------- | ----------------------- |
| `sqlx`              | Built-in pool                          | Postgres, MySQL, SQLite |
| `deadpool-postgres` | tokio-postgres pool                    | Postgres                |
| `bb8`               | Generic async pool                     | Any                     |
| `r2d2`              | Sync pool (with actix_web::web::block) | Any                     |

## WebSocket

```rust
use actix_ws::Message;

#[get("/ws")]
async fn ws(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Text(text) => {
                    session.text(format!("echo: {}", text)).await.unwrap();
                }
                Message::Ping(bytes) => {
                    session.pong(&bytes).await.unwrap();
                }
                Message::Close(reason) => {
                    let _ = session.close(reason).await;
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(response)
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use actix_web::{test, App};
    use super::*;

    #[actix_web::test]
    async fn test_get_user() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_pool().await))
                .service(get_user)
        ).await;

        let req = test::TestRequest::get()
            .uri("/users/1")
            .insert_header(("Authorization", "Bearer test"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: User = test::read_body_json(resp).await;
        assert_eq!(body.name, "Alice");
    }

    #[actix_web::test]
    async fn test_create_user() {
        let app = test::init_service(App::new().service(create_user)).await;

        let req = test::TestRequest::post()
            .uri("/users")
            .set_json(&serde_json::json!({"name": "Bob", "email": "bob@test.com"}))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
    }
}
```

## Static Files & TLS

```rust
use actix_files as fs;

App::new()
    .service(fs::Files::new("/static", "./static").show_files_listing())
    .service(fs::Files::new("/", "./public").index_file("index.html"))

// TLS
let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
builder.set_private_key_file("key.pem", SslFiletype::PEM)?;
builder.set_certificate_chain_file("cert.pem")?;

HttpServer::new(|| App::new())
    .bind_openssl("0.0.0.0:443", builder)?
```

## Multi-Threading Model

Actix Web spawns N worker threads (default = physical CPU count). Each worker runs its own Tokio runtime and its own copy of the `App`. Shared state (`web::Data`) wraps an `Arc`, so it's shared safely across workers. This model means no single-threaded bottleneck — each worker handles requests independently.

## Comparison to Other Rust Frameworks

| Feature              | Actix Web               | Axum             | Rocket            |
| -------------------- | ----------------------- | ---------------- | ----------------- |
| Maturity             | Most mature             | Growing fast     | Established       |
| Approach             | Extractors + middleware | Tower-based      | Guards + fairings |
| Async runtime        | Tokio                   | Tokio            | Tokio             |
| Compile-time routing | No                      | Some (via tower) | Attribute macros  |
| Ecosystem            | Large                   | Tower ecosystem  | Self-contained    |
| Performance          | Top-tier                | Top-tier         | Slightly lower    |
| Learning curve       | Moderate                | Moderate (Tower) | Lower             |
