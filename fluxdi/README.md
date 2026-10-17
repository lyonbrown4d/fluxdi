# FluxDI - Semi-automatic Dependency Injector

[![Crates.io](https://img.shields.io/crates/v/fluxdi.svg)](https://crates.io/crates/fluxdi)
[![Documentation](https://docs.rs/fluxdi/badge.svg)](https://docs.rs/fluxdi)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://github.com/DaiYuANg/fluxdi/actions/workflows/CI.yml/badge.svg)](https://github.com/DaiYuANg/fluxdi/actions/workflows/CI.yml)

A lightweight, type-safe dependency injection container for Rust applications. FluxDI provides ergonomic service registration (including trait-object bindings), transient/root/calling-context/scoped lifetimes, semi-automatic dependency resolution, and circular dependency detection.

## ✨ Features

- 🔒 **Type-Safe**: Leverages Rust's type system for compile-time safety
- 🔄 **Transient Services**: Create new instances on each request
- 🔗 **Singleton Services**: Shared instances with reference counting via `Arc` / `Rc`
- 🧭 **Scoped Context**: Per-request/task instances via `create_scope()` + `Provider::scoped(...)`
- 🔍 **Circular Detection**: Prevents infinite loops in dependency graphs
- ❌ **Error Handling**: Comprehensive error types with detailed messages
- 📊 **Optional Logging**: Built-in logging initialization and tracing integration
- 🚀 **Zero-Cost Abstractions**: Feature gates enable compile-time optimization
- 🧵 **Optional Thread Safety**: Enable `thread-safe` for `Arc` and concurrent storage
- 📦 **Module System**: Organize services into reusable modules
- 🏗️ **Enterprise Ready**: Supports layered architecture, repository pattern, and use cases

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
fluxdi = { path = "../fluxdi" }  # For local development
```

Or from crates.io (when published):

```toml
[dependencies]
fluxdi = "1.2.1"
```

## 🚀 Quick Start

```rust
use fluxdi::{Injector, Provider, Shared, Module, Application};

// Define your services
struct DatabaseService {
    connection_string: String,
}

impl DatabaseService {
    fn new() -> Self {
        Self {
            connection_string: "postgresql://localhost:5432/myapp".to_string(),
        }
    }

    fn query(&self, sql: &str) -> String {
        format!("Executing '{}' on {}", sql, self.connection_string)
    }
}

struct UserService {
    db: Shared<DatabaseService>,
}

impl UserService {
    fn new(db: Shared<DatabaseService>) -> Self {
        Self { db }
    }

    fn create_user(&self, name: &str) -> String {
        self.db.query(&format!("INSERT INTO users (name) VALUES ('{}')", name))
    }
}

struct RootModule;

impl Module for RootModule {
    fn providers(&self, injector: &fluxdi::Injector) {
         // Register DatabaseService as singleton
        injector.provide::<DatabaseService>(Provider::root(|_| {
            Shared::new(DatabaseService::new())
        }));
        
        // Register UserService with DatabaseService dependency
        injector.provide::<UserService>(Provider::root(|inj| {
            let db = inj.resolve::<DatabaseService>();
            UserService::new(db).into()
        }));
    }
}

fn main() {
    // Create an application and register services
    let mut app = Application::new(RootModule);

    app.bootstrap_sync().expect("bootstrap failed");

    // Resolve and use services
    match app.injector().try_resolve::<UserService>() {
        Ok(user_service) => println!("{}", user_service.create_user("Alice")),
        Err(e) => eprintln!("Service resolution failed: {}", e),
    }

    // or just
    app.injector().resolve::<UserService>(); // This panics if not registered
}
```

## 📚 Documentation (mdBook)

FluxDI now includes an mdBook under `docs/`.

Build docs:

```bash
mdbook build docs
```

Serve locally:

```bash
mdbook serve docs --open
```

## Examples

FluxDI includes sixteen comprehensive examples showcasing different use cases and patterns:

### 1. Basic Example
**Location:** `examples/basic/`

A simple introduction to FluxDI fundamentals:
- Service registration with `Injector` and `Provider`
- Transient and singleton lifetimes
- Basic dependency resolution with `try_resolve()`
- Error handling with `Result` types

**Run:**
```bash
cargo run -p basic
```

### 2. Injectable Macro Example
**Location:** `examples/injectable-macro/`

Demonstrates `#[derive(Injectable)]` as an alternative to manual provider closures:
- Same behavior as manual `Provider::root(|inj| { ... })`
- Reduces boilerplate when all dependencies are `Shared<T>`
- Side-by-side parity with `examples/basic`

**Run:**
```bash
cargo run -p injectable-macro-example
```

### 3. Complex Example
**Location:** `examples/complex/`

Demonstrates enterprise-grade architecture with:
- **Domain Layer**: Clear entity definitions and repository interfaces
- **Application Layer**: Use case pattern for business logic
- **Infrastructure Layer**: SQLite persistence with concrete implementations
- **Dependency Injection**: Multi-level service composition
- **Module System**: Modular DI configuration with imported modules

Architecture:
```
core/
  ├── domain/       (User, Todo entities & repository traits)
  └── application/  (CreateUserUseCase, GetAllTodoUseCase, etc.)
infra/
  ├── di/           (Modules & dependency registration)
  └── persistence/  (SQLite repositories)
```

**Run:**
```bash
cd examples/complex
cargo run
```

**Run Tests:**
```bash
cd examples/complex
./test.sh
```

### 4. Actix-web Example
**Location:** `examples/actix/`

Minimal Actix-web integration with FluxDI extractors:
- App-level injector state via `injector_data(...)`
- Per-handler dependency resolution via `fluxdi::actix::Resolved<T>`
- Zero-boilerplate extraction for services from the container

**Run:**
```bash
cd examples/actix
cargo run
```

Then request:
```bash
curl http://127.0.0.1:8081/hello
```

### 5. Axum REST API Example
**Location:** `examples/axum/`

Real-world REST API integration with **Axum** web framework:
- HTTP handler functions with DI-resolved dependencies
- Structured JSON responses with error handling
- CRUD endpoints for Users and Todos
- Service state management via `InjectorState`
- Automatic per-handler DI resolution via `Resolved<T>` extractor
- Dependency resolution per-request
- Lifecycle-driven startup via `Module::on_start` + `Application::bootstrap().await`

**Features:**
- `POST /users` - Create user
- `GET /users` - List all users
- `GET /users/{id}` - Get user by ID
- `DELETE /users/{id}` - Delete user
- `POST /todos` - Create todo
- `GET /todos` - List all todos
- `PUT /todos/{id}/status` - Update todo status
- `DELETE /todos/{id}` - Delete todo

**Run:**
```bash
# Terminal 1: Start server
cd examples/axum
cargo run

# Terminal 2: Run comprehensive test suite
cd examples/axum
./test.sh
```

The test suite includes:
- Server health checks
- Sequential dependency extraction between requests
- HTTP status code validation
- JSON response parsing and assertion

### 6. Axum Lifecycle Example
**Location:** `examples/axum-lifecycle/`

Demonstrates module lifecycle startup with async hooks:
- Register `Router` as DI service
- Start `axum::serve` from `Module::on_start(...)`
- Bootstrap with `Application::bootstrap().await`

**Run:**
```bash
cd examples/axum-lifecycle
cargo run
```

### 7. Module Sync Bootstrap Example
**Location:** `examples/module-sync/`

Minimal module-centric synchronous startup:
- Uses `Module::configure(...)` as the single registration hook
- Starts with `Application::bootstrap_sync()`

**Run:**
```bash
cd examples/module-sync
cargo run
```

### 8. Module Async Bootstrap Example
**Location:** `examples/module-async/`

Async instance registration and lifecycle orchestration:
- Registers async instance via `Provider::root_async(...)`
- Uses `Application::bootstrap().await`
- Demonstrates lifecycle prewarm in `on_start(...)`

**Run:**
```bash
cd examples/module-async
cargo run
```

### 9. SeaORM SQLite Module Example
**Location:** `examples/seaorm-sqlite/`

SeaORM + SQLite integration through FluxDI module lifecycle:
- Registers `DatabaseConnection` via `Provider::root_async(...)`
- Uses `Module::on_start(...)` to run a startup connectivity check (`SELECT 1`)
- Boots with `Application::bootstrap().await`

**Run:**
```bash
cd examples/seaorm-sqlite
cargo run
```

### 10. Dual HTTP Random-Port Example
**Location:** `examples/dual-http-random-port/`

Runs two HTTP services in one module lifecycle:
- Randomly binds both services in `30000-65535`
- Starts both servers concurrently in `on_start(...)`
- Gracefully aborts both tasks in `on_stop(...)`

**Run:**
```bash
cd examples/dual-http-random-port
cargo run
```

### 11. Mixed Sync + Async Module Example
**Location:** `examples/module-mixed-sync-async/`

Demonstrates sync and async providers in the same module:
- Registers sync providers with `Provider::root(...)` and `Provider::transient(...)`
- Registers async providers with `Provider::root_async(...)`
- Uses sync `resolve(...)` and async `try_resolve_async(...).await` in the same startup flow

**Run:**
```bash
cd examples/module-mixed-sync-async
cargo run
```

### 12. Named Bindings Example
**Location:** `examples/named-bindings/`

Demonstrates multiple implementations for one trait via names:
- Registers two `dyn CacheBackend` providers (`primary`, `fallback`)
- Resolves each implementation with `resolve_named::<T>(name)`
- Shows strategy/backends selection without changing consumer code

**Run:**
```bash
cd examples/named-bindings
cargo run
```

### 13. Multi-binding Pipeline Example
**Location:** `examples/multi-binding-pipeline/`

Demonstrates plugin/middleware pipeline composition:
- Registers multiple `dyn Middleware` providers into one set
- Resolves all with `resolve_all::<T>()`
- Executes in deterministic registration order

**Run:**
```bash
cd examples/multi-binding-pipeline
cargo run
```

### 14. Graph Tooling Example
**Location:** `examples/graph-tooling/`

Demonstrates graph export and validation:
- Declares dependencies with `with_dependency/with_set_dependency`
- Runs `validate_graph` and prints issues (if any)
- Exports DOT and Mermaid via `dependency_graph`

**Run:**
```bash
cd examples/graph-tooling
cargo run
```

### 15. Decorator Example
**Location:** `examples/decorator/`

Demonstrates `Provider::with_decorator()` for cross-cutting composition:
- Wraps services with logging, caching, or retry without modifying business logic
- Multiple decorators chain with deterministic order
- Works with trait objects

**Run:**
```bash
cargo run -p decorator-example
```

### 16. Scoped Context Example
**Location:** `examples/scoped-context/`

Demonstrates request/task level scope isolation:
- Registers scoped services with `Provider::scoped(...)`
- Creates runtime scopes with `Injector::create_scope()`
- Shows per-scope cache isolation and root cache sharing

**Run:**
```bash
cd examples/scoped-context
cargo run
```

## Usage Guide

### Service Registration

#### Transient Services
Create new instances on each request:

```rust
use fluxdi::{Injector, Provider, Shared};
use uuid::Uuid;

struct LoggerService {
    session_id: String,
}

let injector = Injector::root();

// Transient: new instance each time (default behavior)
injector.provide::<LoggerService>(Provider::transient(|_| {
    Shared::new(LoggerService { 
        session_id: Uuid::new_v4().to_string() 
    })
}));

let logger1 = injector.resolve::<LoggerService>();
let logger2 = injector.resolve::<LoggerService>();
// logger1 and logger2 are different instances
```

#### Singleton Services
Create once and share across all dependents:

```rust
use fluxdi::{Injector, Provider, Shared};

struct ConfigService {
    app_name: String,
    debug: bool,
}

let injector = Injector::root();

// Singleton: same instance every time
injector.provide::<ConfigService>(Provider::root(|_| {
    Shared::new(ConfigService { 
        app_name: "MyApp".to_string(), 
        debug: true 
    })
}));

let config1 = injector.resolve::<ConfigService>();
let config2 = injector.resolve::<ConfigService>();
// config1 and config2 point to the same instance
```

#### Scoped Services
Create one instance per runtime scope:

```rust
use fluxdi::{Injector, Provider, Shared};

#[derive(Debug)]
struct RequestContext {
    request_id: u64,
}

let injector = Injector::root();
injector.provide::<RequestContext>(Provider::scoped(|_| {
    Shared::new(RequestContext { request_id: 1 })
}));

let scope = injector.create_scope();
let ctx1 = scope.resolve::<RequestContext>();
let ctx2 = scope.resolve::<RequestContext>();

assert!(Shared::ptr_eq(&ctx1, &ctx2));
```

### Error Handling

FluxDI provides both panicking and non-panicking variants:

```rust
use fluxdi::{Injector, Provider, Shared, Error};

let injector = Injector::root();
injector.provide::<String>(Provider::transient(|_| Shared::new("Hello".to_string())));

// Non-panicking (try_resolve returns Result)
match injector.try_resolve::<String>() {
    Ok(s) => println!("Got: {}", s),
    Err(e) => println!("Error: {}", e),
}

// Trying to resolve an unregistered type
match injector.try_resolve::<u32>() {
    Ok(_) => unreachable!(),
    Err(e) => println!("Expected error: {}", e),
}
```

### Provider Override (Testing)

You can replace existing providers for test and integration scenarios:

```rust
use fluxdi::{Injector, Provider, Shared};

let injector = Injector::root();
injector.provide::<String>(Provider::root(|_| Shared::new("real".to_string())));

injector
    .try_override_provider::<String>(Provider::root(|_| Shared::new("mock".to_string())))
    .expect("override should succeed");

assert_eq!(injector.resolve::<String>().as_str(), "mock");
```

### Named Bindings

Register and resolve multiple implementations for the same type/trait:

```rust
use fluxdi::{Injector, Provider, Shared};

trait Cache: Send + Sync {
    fn id(&self) -> &'static str;
}

struct Redis;
impl Cache for Redis {
    fn id(&self) -> &'static str { "redis" }
}

struct Memory;
impl Cache for Memory {
    fn id(&self) -> &'static str { "memory" }
}

let injector = Injector::root();

injector.provide_named::<dyn Cache>(
    "primary",
    Provider::root(|_| Shared::new(Redis) as Shared<dyn Cache>),
);
injector.provide_named::<dyn Cache>(
    "fallback",
    Provider::root(|_| Shared::new(Memory) as Shared<dyn Cache>),
);

let primary = injector.resolve_named::<dyn Cache>("primary");
let fallback = injector.resolve_named::<dyn Cache>("fallback");

assert_eq!(primary.id(), "redis");
assert_eq!(fallback.id(), "memory");
```

### Multi-binding

Register multiple implementations in one set and resolve them together:

```rust
use fluxdi::{Injector, Provider, Shared};

trait Middleware: Send + Sync {
    fn name(&self) -> &'static str;
}

struct Trim;
impl Middleware for Trim {
    fn name(&self) -> &'static str { "trim" }
}

struct Uppercase;
impl Middleware for Uppercase {
    fn name(&self) -> &'static str { "uppercase" }
}

let injector = Injector::root();
injector.provide_into_set::<dyn Middleware>(
    Provider::singleton(|_| Shared::new(Trim) as Shared<dyn Middleware>),
);
injector.provide_into_set::<dyn Middleware>(
    Provider::singleton(|_| Shared::new(Uppercase) as Shared<dyn Middleware>),
);

let middlewares = injector.resolve_all::<dyn Middleware>();
let order: Vec<&str> = middlewares.iter().map(|m| m.name()).collect();
assert_eq!(order, vec!["trim", "uppercase"]);
```

Ordering strategy:

- Parent injector bindings come first.
- Child injector bindings come after parent bindings.
- Within each injector, registration order is preserved.

### Graph Tooling

You can export dependency graphs and validate missing dependencies/cycles:

```rust
use fluxdi::{Injector, Provider, Shared};

struct A;
struct B;

let injector = Injector::root();
injector.provide::<A>(
    Provider::singleton(|_| Shared::new(A)).with_dependency::<B>(),
);
injector.provide::<B>(Provider::singleton(|_| Shared::new(B)));

let report = injector.validate_graph();
assert!(report.is_valid());

let graph = injector.dependency_graph();
let dot = graph.to_dot();
let mermaid = graph.to_mermaid();
assert!(dot.contains("digraph fluxdi"));
assert!(mermaid.contains("graph TD"));
```

CI-friendly guard:

```rust
injector.try_validate_graph()?;
```

### Derive Macro (Injectable)

Enable derive support:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["macros"] }
```

```rust
use fluxdi::{Injectable, Shared};

#[derive(Injectable)]
struct UserService {
    repo: Shared<MyRepo>,
}
```

### Dependency Injection

Services can depend on other services. Use module-based registration for clean organization:

```rust
use fluxdi::{Injector, Module, Provider, Shared};

struct DatabaseService { /* ... */ }
impl DatabaseService { fn new() -> Self { DatabaseService {} } }

struct CacheService { /* ... */ }
impl CacheService { fn new() -> Self { CacheService {} } }

struct UserRepository {
    db: Shared<DatabaseService>,
    cache: Shared<CacheService>,
}

impl UserRepository {
    fn new(db: Shared<DatabaseService>, cache: Shared<CacheService>) -> Self {
        Self { db, cache }
    }
}

// Define a module for persistence services
struct PersistenceModule;

impl Module for PersistenceModule {
    fn providers(&self, injector: &Injector) {
        injector.provide::<DatabaseService>(Provider::root(|_| {
            Shared::new(DatabaseService::new())
        }));
        
        injector.provide::<CacheService>(Provider::root(|_| {
            Shared::new(CacheService::new())
        }));
        
        injector.provide::<UserRepository>(Provider::root(|inj| {
            let db = inj.resolve::<DatabaseService>();
            let cache = inj.resolve::<CacheService>();
            UserRepository::new(db, cache).into()
        }));
    }
}

let injector = Injector::root();
let module = PersistenceModule;
module.providers(&injector);

let repo = injector.resolve::<UserRepository>();
```

## 🔍 Advanced Features

### Circular Dependency Detection

FluxDI automatically detects and prevents circular dependencies by tracking resolution paths:

```rust
use fluxdi::{Injector, Provider, Shared};

// Example: attempting to create circular dependencies will fail
struct ServiceA {
    b: Shared<ServiceB>,
}

struct ServiceB {
    a: Shared<ServiceA>,
}

let injector = Injector::root();

// These registrations will create a circular dependency
// Attempting to resolve either service will result in an error
// Error: "Circular dependency detected in resolution path"
```

### Resource Limits

You can constrain concurrent factory execution per provider:

```rust
use fluxdi::{Injector, Limits, Provider, Shared};

let injector = Injector::root();

injector.provide::<u32>(Provider::transient_with_limits(Limits::deny(2), |_| {
    Shared::new(42)
}));
```

Available policies:

- `Policy::Deny`: fail immediately with `ResourceLimitExceeded`.
- `Policy::Block`: wait for a free slot (supported when `thread-safe` is enabled).

Timeout support:

```rust
use std::time::Duration;
use fluxdi::{Limits, Provider, Shared};

injector.provide::<u32>(Provider::transient_with_limits(
    Limits::block_with_timeout(1, Duration::from_millis(50)),
    |_| Shared::new(42),
));
```

Async non-blocking wait (optional):

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["thread-safe", "async-factory", "resource-limit-async"] }
```

### Logging

Enable the `logging` feature for a default text logger:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["logging"] }
```

```rust
fn main() {
    fluxdi::try_init_logging().expect("failed to initialize logging");

    // FluxDI spans/events will be printed with default formatting.
}
```

`logging` reads `RUST_LOG` and falls back to `info` when unset.

For rich node-level diagnostics across registration, provider lookup, cache hit/miss,
and factory execution:

```bash
RUST_LOG=fluxdi=trace
```

### Tracing Integration

Enable the `tracing` feature for automatic logging:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["tracing"] }
```

```rust
use fluxdi::{Application, Module, Provider, Shared};
use tracing::info;

struct MyModule;

impl Module for MyModule {
    fn providers(&self, injector: &fluxdi::Injector) {
        injector.provide::<DatabaseService>(Provider::root(|_| {
            info!("Registering DatabaseService");
            Shared::new(DatabaseService::new())
        }));
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let mut app = Application::new(MyModule);
    app.bootstrap().await.expect("bootstrap failed");
    
    // Resolving services will be traced when tracing feature is enabled
    let _db = app.injector().try_resolve::<DatabaseService>();
}
```

### OpenTelemetry Bridge

Enable OpenTelemetry bridge support with the `opentelemetry` feature:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["opentelemetry"] }
opentelemetry_sdk = { version = "0.29", features = ["trace"] }
```

```rust
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::SdkTracerProvider;
use fluxdi::opentelemetry_layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    // Configure exporter/processors on this provider in real applications.
    let tracer_provider = SdkTracerProvider::builder().build();
    let tracer = tracer_provider.tracer("my-service");

    tracing_subscriber::registry()
        .with(opentelemetry_layer(tracer))
        .try_init()
        .expect("failed to install tracing subscriber");

    // Keep provider ownership in your app lifecycle and shutdown gracefully.
    let _ = tracer_provider.shutdown();
}
```

`fluxdi` only provides tracing-to-OTel wiring helpers (`opentelemetry_layer` and
`try_init_opentelemetry`) and does not auto-install exporters.

### Service Metrics and Prometheus Export

Enable metrics collection:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["metrics"] }
```

Enable Prometheus text export (`prometheus` implies `metrics`):

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["prometheus"] }
```

```rust
use fluxdi::{Injector, Provider, Shared};

fn main() {
    let injector = Injector::root();
    injector.provide::<u32>(Provider::root(|_| Shared::new(42)));

    let _ = injector.try_resolve::<u32>();

    let snapshot = injector.metrics_snapshot();
    println!("resolve attempts = {}", snapshot.resolve_attempts_total);

    #[cfg(feature = "prometheus")]
    println!("{}", injector.prometheus_metrics());
}
```

### Async Factory Support

Enable async factories with the `async-factory` feature:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["async-factory"] }
```

```rust
use fluxdi::{Injector, Provider, Shared};

#[tokio::main]
async fn main() {
    let injector = Injector::root();

    injector.provide::<String>(Provider::root_async(|_| async {
        Shared::new("async-value".to_string())
    }));

    let value = injector.resolve_async::<String>().await;
    assert_eq!(value.as_str(), "async-value");
}
```

Resolution strategy:

- If a type is registered with `Provider::*_async`, resolve it with `try_resolve_async` / `resolve_async`.
- Sync providers (`Provider::root/singleton/transient`) work with both sync and async resolve APIs.
- Sync resolve against an async provider returns `AsyncFactoryRequiresAsyncResolve`.

Runnable example:

```bash
cd examples/async-factory
cargo run
```

### Module Async Lifecycle Hooks

`Module` now supports async hooks for startup/shutdown orchestration:

- `configure(injector)` as the primary, unified provider registration hook
- `providers_async(injector)` as a legacy-compatible alias (new code should use `configure`)
- `on_start(injector)` to start runtime resources (for example, an Axum listener)
- `on_stop(injector)` to release resources in reverse module order

Use `Application::bootstrap().await` and `Application::shutdown().await` as the unified lifecycle APIs.

For production patterns (graceful shutdown, timeouts, background tasks), see the **Production Patterns** section in the docs (`mdbook serve docs`).

```rust,no_run
use fluxdi::{Application, Error, Injector, Module, Shared, module::ModuleLifecycleFuture};

struct WebModule;

impl Module for WebModule {
    fn on_start(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
        Box::pin(async move {
            let app = injector.resolve::<axum::Router>();
            let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
                .await
                .map_err(|e| Error::module_lifecycle_failed("WebModule", "on_start", &e.to_string()))?;

            // Start in background so bootstrap() can return.
            tokio::spawn(async move {
                let _ = axum::serve(listener, app.as_ref().clone()).await;
            });
            Ok(())
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut app = Application::new(WebModule);
    app.bootstrap().await
}
```

## 🧪 Testing

### Unit Tests

Run the crate test suite:

```bash
# Run all tests for the workspace
cargo test

# Run tests for the fluxdi crate only
cargo test -p fluxdi

# Run with tracing feature
cargo test --features tracing

# Run with async factory feature
cargo test --features async-factory

# Run documentation tests
cargo test --doc -p fluxdi
```

## 📁 Project Structure

```
fluxdi/
├── fluxdi/                 # FluxDI library crate
│   ├── src/              # core implementation (injector, provider, runtime)
│   └── README.md         # This file
├── examples/
│   ├── basic/            # Basic usage example with simple DI
│   ├── complex/          # Advanced DI patterns with SQLite, repositories, use cases
│   │   ├── src/
│   │   │   ├── core/     # Domain (entities, use cases)
│   │   │   └── infra/    # Infrastructure (persistence, DI configuration)
│   │   └── test.sh       # Test script for complex example
│   ├── actix/            # Actix-web integration example
│   │   └── src/
│   │       └── main.rs   # Minimal Actix extractor usage
│   ├── axum-lifecycle/   # Axum startup in Module::on_start lifecycle hook
│   │   └── src/
│   │       └── main.rs   # Lifecycle-driven server bootstrap
│   ├── module-sync/      # Sync module bootstrap with configure()
│   │   └── src/
│   │       └── main.rs   # bootstrap_sync usage
│   ├── module-async/     # Async module bootstrap with async providers
│   │   └── src/
│   │       └── main.rs   # bootstrap().await + root_async provider
│   ├── module-mixed-sync-async/ # Sync and async providers in one module
│   │   └── src/
│   │       └── main.rs   # mixed resolve + try_resolve_async usage
│   ├── multi-binding-pipeline/ # Ordered plugin/middleware pipeline with resolve_all
│   │   └── src/
│   │       └── main.rs   # provide_into_set + resolve_all usage
│   ├── graph-tooling/    # Dependency graph export + validation example
│   │   └── src/
│   │       └── main.rs   # dependency_graph + validate_graph
│   ├── named-bindings/   # Named/strategy bindings for one trait
│   │   └── src/
│   │       └── main.rs   # provide_named + resolve_named example
│   ├── seaorm-sqlite/    # SeaORM + SQLite module lifecycle example
│   │   └── src/
│   │       └── main.rs   # on_start connectivity check (SELECT 1)
│   ├── dual-http-random-port/ # Two HTTP servers with random 30000-65535 ports
│   │   └── src/
│   │       └── main.rs   # concurrent startup in on_start + abort in on_stop
│   ├── scoped-context/   # Runtime scoped injector example
│   │   └── src/
│   │       └── main.rs   # create_scope + Provider::scoped usage
│   └── axum/             # REST API with Axum web framework
│       ├── src/
│       │   └── main.rs   # HTTP handlers with DI integration
│       └── test.sh       # Comprehensive API test suite
└── README.md
```

## 🔧 Configuration

### Singleton Context

`Provider::singleton(...)` caches in the injector context that calls resolve.
Cloning an injector shares its context; child injectors and new runtime scopes get
independent singleton caches. Use `Provider::root(...)` for hierarchy-wide sharing,
and `Provider::scoped(...)` for the nearest runtime scope. See
[Providers And Resolution](../docs/src/providers-and-resolution.md) for concurrency
and initialization error behavior.

### Feature Flags

FluxDI exposes a small set of feature flags. See `fluxdi/Cargo.toml` for the authoritative list:

- `debug` (enabled by default) — enables extra debug formatting in selected types and errors.
- `thread-safe` (optional) — switches internal shared pointer and synchronization primitives to `Arc` + `RwLock` for concurrent access.
- `lock-free` (optional) — uses `DashMap` for provider/instance stores in `thread-safe` mode.
- `logging` (optional) — enables `try_init_logging()` / `init_logging()` helpers and `RUST_LOG` filter support.
- `tracing` (optional) — enables tracing logs and spans during registration and resolution.
- `opentelemetry` (optional) — adds OpenTelemetry bridge helpers in `fluxdi::observability`.
- `metrics` (optional) — enables internal injector counters and `Injector::metrics_snapshot()`.
- `prometheus` (optional) — enables `Injector::prometheus_metrics()` text export (includes `metrics`).
- `actix` (optional) — enables Actix-web integration helpers and extractors (and enables `thread-safe`).
- `axum` (optional) — enables Axum integration helpers and extractors (and enables `thread-safe`).
- `async-factory` (optional) — enables `Provider::*_async` and `Injector::*_resolve_async` APIs.
- `resource-limit-async` (optional) — enables non-blocking async resource-limit waits sharing the synchronous creation budget, with Tokio timeouts.
- `macros` (optional) — enables `#[derive(Injectable)]` via `fluxdi-macros`.
- `lifecycle` (optional) — enables `BootstrapOptions`/`ShutdownOptions` timeout support for production environments.

Decorators (`Provider::with_decorator()`) work with any feature set; no extra flag required.

The crate default is currently `default = ["debug"]`. If you need multithreaded use, enable `thread-safe`.

### Environment Variables

When using the `logging` (or `tracing`) feature, you can control log levels:

```bash
# Set log level
RUST_LOG=debug cargo run --example basic

# Enable only FluxDI logs
RUST_LOG=fluxdi=info cargo run --example basic
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development Setup

1. Clone the repository:
```bash
git clone https://github.com/DaiYuANg/fluxdi.git
cd fluxdi
```

2. Run tests:
```bash
cargo test --all-features
```

3. Check formatting:
```bash
cargo fmt --check
```

4. Run clippy:
```bash
cargo clippy -- -D warnings
```

## 📋 Roadmap & TODO

### 🧵 Thread Safety
- [x] **Arc-based Runtime**: Thread-safe version of FluxDI using `Arc` instead of `Rc` (implemented behind the `thread-safe` feature)
- [x] **Send + Sync Services**: Support for `Send + Sync` services in thread-safe mode (enforced by API bounds)
- [x] **Concurrent Access**: Concurrent reads/writes supported via `RwLock` in thread-safe mode
- [x] **Sharded Concurrent Storage**: Optional `lock-free` feature uses `DashMap` (the feature name is retained for compatibility)

### 🔧 Advanced Features
- [x] **Lazy Initialization**: Singleton instances are created on first `resolve`
- [x] **Service Metrics**: Internal counters + timing totals via `metrics_snapshot`
- [x] **Resource Limits**: Per-provider concurrent creation limits with `Limits` + `Policy`
- [x] **Named Bindings**: Resolve multiple implementations by key with `provide_named/resolve_named`
- [x] **Multi-binding**: Resolve ordered plugin sets with `provide_into_set/resolve_all`
- [x] **Graph Tooling**: Export dependency graph and validate missing/cyclic dependencies
- [x] **Scoped Context**: `create_scope` + `Provider::scoped` runtime scope isolation

### 📦 Ecosystem Integration
- [x] **Async Factory Support**: Async/await provider factories via `Provider::*_async` + `Injector::*_resolve_async`
- [x] **Actix-web Integration**: `fluxdi::actix::{Resolved<T>, ServiceConfigExt, injector_data}`
- [x] **Axum Integration**: Demonstrated with REST API example and state management
- [x] **Axum Auto-Resolve Plugin**: `fluxdi::axum::Resolved<T>` extractor for per-handler resolution
- [ ] **Rocket Integration**: Layer and extractor support for Rocket web framework

### Architectural Patterns
- [x] **Repository Pattern**: Demonstrated in complex example with SQLite repositories
- [x] **Layered Architecture**: Clean separation of domain, application, and infrastructure layers
- [x] **Use Case Pattern**: Business logic encapsulated in use cases with DI
- [x] **Web Framework Integration**: Explored with Axum + Actix-web

### Developer Experience
- [x] **Derive Macros**: Auto-generate factory functions from service structs (`#[derive(Injectable)]`)
- [x] **Error Suggestions**: Better error messages with fix suggestions

### 📊 Observability
- [x] **OpenTelemetry**: Optional tracing bridge helpers via `opentelemetry_layer` / `try_init_opentelemetry`
- [x] **Prometheus Metrics**: `prometheus_metrics()` text export for scraping

### 🎯 Performance
- [ ] **Memory Optimization**: Reduced memory footprint for large containers

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/DaiYuANg/fluxdi/blob/main/LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by dependency injection patterns from other languages and frameworks
- Built with ❤️ using Rust's powerful type system
- Thanks to the Rust community for excellent crates and documentation

---

**FluxDI** - A semi-automatic dependency injection container for Rust  
**Repository:** [DaiYuANg/fluxdi](https://github.com/DaiYuANg/fluxdi)  
**Made with ❤️ by the Binary Sea Team**


