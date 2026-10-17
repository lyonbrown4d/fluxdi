# Modules And Lifecycle

`Module::configure(...)` is the preferred provider registration entry. Async
bootstrap invokes `providers_async(...)`, whose default implementation calls
`configure(...)`; existing overrides of the async hook continue to run.

For async startup/shutdown orchestration, use:

- `Module::on_start(...)`
- `Module::on_stop(...)`
- `Application::bootstrap().await`
- `Application::shutdown().await`

Sync-only bootstrapping is still available through `bootstrap_sync()`.

## Lifecycle Options (timeout)

Enable the `lifecycle` feature for bootstrap and shutdown timeouts. Parallel startup
options are also available without this feature:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["lifecycle"] }
```

```rust
use std::time::Duration;
use fluxdi::{Application, BootstrapOptions, ShutdownOptions, Injector, Module, Provider, Shared};
use fluxdi::module::ModuleLifecycleFuture;

struct AppModule;

impl Module for AppModule {
    fn configure(&self, injector: &Injector) -> Result<(), fluxdi::Error> {
        injector.provide::<String>(Provider::root(|_| Shared::new("ok".to_string())));
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), fluxdi::Error> {
    let mut app = Application::new(AppModule);

    // Bootstrap with a 30s timeout
    let opts = BootstrapOptions::default().with_timeout(Duration::from_secs(30));
    app.bootstrap_with_options(opts).await?;

    // Shutdown with a 10s timeout
    let opts = ShutdownOptions::default().with_timeout(Duration::from_secs(10));
    app.shutdown_with_options(opts).await?;

    Ok(())
}
```

When a timeout is exceeded, bootstrap returns an error and stops modules that
finished startup successfully, in reverse module order. Configuration and startup
errors use the same rollback path. Cleanup receives a separate timeout budget equal
to the configured startup timeout, so the full operation can exceed that timeout.
If the caller cancels bootstrap directly, call `shutdown()` to clean up modules
that already started. A failed or canceled `on_start` must manage its own partially
created resources; only hooks that returned success enter the shutdown list.
`is_bootstrapped()` becomes true only after successful startup and false after
shutdown. A failed or canceled application cannot bootstrap again; create a new
`Application` for a fresh attempt.

## Shutdown Error Aggregation

If multiple modules fail during `on_stop()`, FluxDI attempts to stop all modules
and returns a single aggregated error listing every failure:

```
Shutdown failed: 2 module(s) reported errors:
  1) Module lifecycle failed: module=WebModule, phase=on_stop, details=...
  2) Module lifecycle failed: module=DatabaseModule, phase=on_stop, details=...
```

This makes it easier to diagnose and fix shutdown issues in production.

For recommended patterns (graceful shutdown, timeouts, background tasks), see
[Production Patterns](./production-patterns.md).

## Basic Example

```rust
use fluxdi::{Application, Error, Injector, Module, Provider, Shared, module::ModuleLifecycleFuture};

struct AppModule;

impl Module for AppModule {
    fn configure(&self, injector: &Injector) -> Result<(), Error> {
        injector.provide::<String>(Provider::root(|_| Shared::new("ok".to_string())));
        Ok(())
    }

    fn on_start(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
        Box::pin(async move {
            let value = injector.resolve::<String>();
            println!("started with {}", value);
            Ok(())
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut app = Application::new(AppModule);
    app.bootstrap().await?;
    app.shutdown().await
}
```
