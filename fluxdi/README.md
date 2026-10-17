# FluxDI

[English](README.md) · [简体中文](README.zh-CN.md)

FluxDI is a type-safe dependency injection container for Rust. The core is
independent of HTTP frameworks; Axum and Actix-web adapters are optional.

This README describes the **unreleased workspace**, requiring **Rust 1.99.1 or
newer**. The core and macro changes need a coordinated release before registry installation.
The [English book](../docs/src/introduction.md) is the canonical reference.

## Local dependency

```toml
[dependencies]
fluxdi = { path = "../fluxdi" }
```

Add `features = ["macros"]` for `Injectable` and `static_module!`, or select the
features below for async factories, thread safety, adapters, and instrumentation.

## Manual registration

```rust
use fluxdi::{Error, Injector, Provider, Shared};

struct Config(&'static str);
struct Service(Shared<Config>);

fn main() -> Result<(), Error> {
    let injector = Injector::root();
    injector.provide::<Config>(Provider::root(|_| Shared::new(Config("prod"))));
    injector.provide::<Service>(Provider::try_singleton(|inj| {
        let config = inj.try_resolve::<Config>()?;
        Ok(Shared::new(Service(config)))
    }));

    let service = injector.try_resolve::<Service>()?;
    assert_eq!(service.0.0, "prod");
    Ok(())
}
```

`Shared<T>` uses `Rc<T>` by default and `Arc<T>` with `thread-safe`.
Thread-safe services must satisfy `Send + Sync + 'static`; local services can be
non-`Send`. Named bindings, ordered sets, decorators, and overrides are available
through the runtime injector.

## Scope contracts

| Provider | Cache lifetime |
| --- | --- |
| `Provider::root` / `Scope::Root` | Shared globally within the injector tree |
| `Provider::singleton` / `Scope::Module` | Once per calling injector context |
| `Provider::scoped` / `Scope::Scoped` | Nearest `create_scope()` boundary |
| `Provider::transient` / `Scope::Transient` | A new instance on every resolution |

Cloned handles share a context. Ordinary children have their own singleton cache;
inside a runtime scope, they share its scoped cache. Nested scopes isolate scoped
instances. Local bindings take precedence over inherited cached values.

Concurrent first resolutions of a cached binding share initialization. Fallible
`try_*` factories cache successful results only; errors propagate, release creation
permits, and allow subsequent attempts. Each caller attempts initialization at most
once. User panics are not converted into `Error`.

Async factories require `async-factory` and async resolution when uncached.
Successful cached values can be read synchronously. A synchronous caller encountering
initialization already running asynchronously gets `AsyncFactoryRequiresAsyncResolve`.
See [providers](../docs/src/providers-and-resolution.md) and
[fallible factories](../docs/src/fallible-factories.md) for full concurrency rules.

## Two macro paths

`#[derive(Injectable)]` generates constructors for runtime-container resolution.
It supports generic named-field structs with `Shared<T>`, `Option<Shared<T>>`, and
`Vec<Shared<T>>` fields, plus named dependencies. Register `YourType::provider(scope)`
to attach inferred graph hints. Arbitrary factory closures still need explicit hints.
See [derive macros](../docs/src/derive-macros.md).

`static_module!` declares an explicit synchronous graph and typed factory calls:

```rust
use fluxdi::{Error, Shared, static_module};

struct Config(&'static str);
struct Service(Shared<Config>);

fn service(config: Shared<Config>) -> Result<Shared<Service>, Error> {
    Ok(Shared::new(Service(config)))
}

static_module! {
    struct App {
        #[scope(scoped)]
        service: Service = service(config);
        external config: Config;
    }
}

fn main() -> Result<(), Error> {
    let app = App::new(Shared::new(Config("prod")));
    let scope = app.create_scope();
    let service = scope.service()?;
    assert_eq!(service.0.0, "prod");
    Ok(())
}
```

The macro checks declared missing bindings, cycles, scopes, and factory types at
compile time. Root bindings cannot capture declared contextual dependencies.
Initialization can still fail at runtime; network operations, configuration, and
external resource construction are outside those checks.

Static bindings are separate from the dynamic registry: dynamic overrides and graph
snapshots do not describe or alter them. This API currently has no module imports,
automatic discovery, generated async factories, or lifecycle ordering. It is not a
full CDI implementation. See [static modules](../docs/src/static-modules.md).

## Application and HTTP lifetime

`Module::configure` registers providers; `Application::bootstrap` and `shutdown`
run lifecycle hooks. Startup options support parallel execution; the `lifecycle` feature adds timeouts.
Startup failure rolls back successfully started modules. A failed or canceled start
hook must clean up its own partially created resources.

HTTP adapters require explicit `request_scope` middleware to create one runtime
scope per request. Without it, `Resolved<T>` resolves from framework state.
Request scope retention ends when the response is produced, before a streaming body
has necessarily finished. Streaming responses and spawned tasks must retain their
needed service or scope handles. There is no automatic per-request async shutdown.
See [lifecycle](../docs/src/modules-and-lifecycle.md),
[production patterns](../docs/src/production-patterns.md), and
[Web integrations](../docs/src/web-integrations.md).

## Feature catalog

| Feature | Purpose |
| --- | --- |
| `default` | Enables `debug` |
| `debug` | Debug implementations for container/provider diagnostics |
| `thread-safe` | Thread-safe container and `Arc` shared services |
| `lock-free` | DashMap storage; enables `thread-safe`; uses sharded locking |
| `async-factory` | Asynchronous provider factories |
| `resource-limit-async` | Async resource limits; enables `thread-safe`, `async-factory`, and Tokio |
| `macros` | Re-exports `Injectable` and `static_module!` |
| `lifecycle` | Tokio-backed lifecycle timeouts |
| `axum` | Optional Axum adapter; enables `thread-safe` |
| `actix` | Optional Actix-web adapter; enables `thread-safe` |
| `tracing` | Tracing instrumentation |
| `logging` | Tracing subscriber and `RUST_LOG` support; enables `tracing` |
| `metrics` | Injector metric snapshots |
| `prometheus` | Prometheus text export; enables `metrics` |
| `opentelemetry` | OpenTelemetry tracing integration; enables `tracing` |

See [observability](../docs/src/observability.md) and
[performance](../docs/src/performance.md) for instrumentation and benchmark limits.

## Repository commands

Run these commands from the repository root with its pinned toolchain:

```bash
cargo check --workspace --locked
cargo test --workspace --locked
cargo run -p basic --locked
cargo run -p fluxdi --example static_module --features macros --locked
sh docs/build.sh
```

The documentation command builds both language editions. Workspace dependency
versions are centralized in the root manifest. SeaORM's SQLite example has an
isolated workspace and lockfile to avoid a native SQLite ABI dependency conflict:

```bash
cargo run --manifest-path examples/seaorm-sqlite/Cargo.toml --locked
```

Other examples use `cargo run -p <package>`. See the
[example catalog](../docs/src/examples.md) and
[dependency audit](../docs/src/dependency-audit.md) for details.
