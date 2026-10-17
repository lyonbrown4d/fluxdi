# Fallible Factories

These APIs are available in the unreleased workspace, which requires Rust 1.99.1 or newer.

## Synchronous Initialization

Use `Provider::try_root`, `try_singleton`, `try_scoped`, or `try_transient` when initialization can fail. Each accepts a factory returning `Result<Shared<T>, fluxdi::Error>` and follows the same lifetime as its infallible counterpart.

```rust
use fluxdi::{Error, Injector, Provider, Shared};

struct Port(u16);

fn main() -> Result<(), Error> {
    let injector = Injector::root();
    injector.provide::<Port>(Provider::try_root(|_| {
        let raw = std::env::var("PORT")
            .map_err(|error| Error::factory_failed("Port", &error.to_string()))?;
        let port = raw.parse::<u16>()
            .map_err(|error| Error::factory_failed("Port", &error.to_string()))?;
        Ok(Shared::new(Port(port)))
    }));

    let port = injector.try_resolve::<Port>()?;
    println!("port={}", port.0);
    Ok(())
}
```

Propagate dependency failures with `inj.try_resolve::<Dependency>()?` inside a synchronous factory. `#[derive(Injectable)]` also provides [a fallible constructor](./derive-macros.md#generated-api).

## Asynchronous Initialization

With `async-factory`, use `try_root_async`, `try_singleton_async`, `try_scoped_async`, or `try_transient_async`. The factory takes an owned injector and returns a future producing `Result<Shared<T>, Error>`.

```rust
injector.provide::<Service>(Provider::try_singleton_async(|inj| async move {
    let dependency = inj.try_resolve_async::<Dependency>().await?;
    let service = Service::connect(dependency).await
        .map_err(|error| Error::factory_failed("Service", &error.to_string()))?;
    Ok(Shared::new(service))
}));
let service = injector.try_resolve_async::<Service>().await?;
```

An uncached async provider requires async resolution. A successfully cached instance can also be read synchronously. With `thread-safe`, factories must be `Send + Sync` and their futures must be `Send`.

## Errors, Caching, and Retry

- The `try_resolve*` methods propagate factory errors unchanged, including errors from named and set bindings. `Error::factory_failed` is a helper for application errors; dependency failures keep their original category.
- Root, calling-context singleton, and scoped providers cache only successful instances. Transient providers initialize on every resolution.
- After an initialization error, the caller receives `Err`. Creation permits are released, and subsequent callers can attempt initialization again. Concurrent callers waiting for that instance are awakened and may perform the next attempt; they need not receive the same error as the first caller. Each caller performs at most one initialization attempt.
- Decorators run only after successful initialization, in registration order. Resource limits are configured with `.with_limits(...)`.
- `resolve*` and `from_injector` retain panic-on-error behavior. Fallible APIs propagate `Result` errors; they do not convert user panics into `Error`.

`ErrorKind` has a new `FactoryFailed` variant. Downstream code that matches all variants exhaustively must add an arm for it when upgrading.
