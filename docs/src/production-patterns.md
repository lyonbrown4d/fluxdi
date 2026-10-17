# Production Patterns for Long-Running Servers

This guide covers recommended patterns when running FluxDI applications as
long-lived services (HTTP servers, workers, etc.).

## Lifecycle Feature

Enable the `lifecycle` feature for timeout support:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["lifecycle"] }
```

## Parallel Module Startup

Run module `on_start` hooks in parallel to reduce bootstrap time:

```rust
let opts = BootstrapOptions::default().with_parallel_start(true);
app.bootstrap_with_options(opts).await?;
```

When multiple modules fail during parallel startup, all failures are aggregated
into a single error (similar to shutdown). Use `Error::bootstrap_aggregate` for
programmatic access.

If any module fails during bootstrap (sequential or parallel), modules that
already started are rolled back: `on_stop` is called on them in reverse order
before the error is returned.

## Bootstrap Timeout

Prevent startup from hanging indefinitely if a module blocks:

```rust
use std::time::Duration;
use fluxdi::{Application, BootstrapOptions};

let opts = BootstrapOptions::default().with_timeout(Duration::from_secs(30));
app.bootstrap_with_options(opts).await?;
```

## Graceful Shutdown with Ctrl+C

Wait for a shutdown signal before stopping:

```rust
#[tokio::main]
async fn main() -> Result<(), fluxdi::Error> {
    let mut app = Application::new(MyModule);
    app.bootstrap().await?;

    tokio::signal::ctrl_c().await.expect("failed to listen for signal");
    app.shutdown().await
}
```

See `examples/dual-http-random-port` for a full implementation.

## Shutdown Timeout (Graceful)

Ensure shutdown does not hang when modules are slow to clean up. With a timeout,
the shutdown is **graceful**: each module's `on_stop` is attempted within the
remaining time budget. All modules are always attempted (no partial abort);
timeouts and failures are aggregated into a single error.

```rust
let opts = ShutdownOptions::default().with_timeout(Duration::from_secs(10));
app.shutdown_with_options(opts).await?;
```

## Background Server Tasks

`on_start` runs synchronously; start servers in the background so `bootstrap()`
can return:

```rust
fn on_start(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
    Box::pin(async move {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
        let app = injector.resolve::<Router>().as_ref().clone();

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Ok(())
    })
}
```

## Aborting Background Tasks in on_stop

When you spawn tasks in `on_start`, store their handles and abort them in `on_stop`:

```rust
#[derive(Default)]
struct ServerRuntime {
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

fn on_start(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
    Box::pin(async move {
        let runtime = injector.resolve::<ServerRuntime>();
        let task = tokio::spawn(server_loop());
        runtime.tasks.lock().unwrap().push(task);
        Ok(())
    })
}

fn on_stop(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
    Box::pin(async move {
        let runtime = injector.resolve::<ServerRuntime>();
        for task in runtime.tasks.lock().unwrap().drain(..) {
            task.abort();
        }
        Ok(())
    })
}
```

See `examples/dual-http-random-port` for the full pattern.

## Error Handling

- **Bootstrap failures**: Return early; the application has not started.
- **Shutdown failures**: FluxDI aggregates all module failures. Check `err.message`
  for the full list. Log the error and exit with a non-zero code.
- **Shutdown aggregation**: All modules are attempted; you get a single error
  listing every failure.

## Module Structure

Organize by responsibility:

1. **Config/Infra modules** (DB, cache, config) – no HTTP
2. **Web module** – depends on infra, starts HTTP listener in `on_start`
3. **Root module** – imports infra + web, optionally handles signals

Example layout:

```
AppModule
├── DatabaseModule   (on_start: connect, on_stop: disconnect)
├── CacheModule      (on_start: connect)
└── WebModule        (on_start: bind + spawn server, on_stop: abort tasks)
```

## Relevant Examples

- `examples/axum-lifecycle` – minimal HTTP server with lifecycle
- `examples/dual-http-random-port` – multiple servers, ctrl_c, task abort
- `examples/axum` – full REST API with DI
