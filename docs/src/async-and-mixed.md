# Async And Mixed Usage

Enable async provider factories:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["async-factory"] }
```

Register async providers with `Provider::*_async` and resolve using:

- `try_resolve_async::<T>().await`
- `resolve_async::<T>().await`

Sync and async providers can coexist in the same module.

See:

- `examples/module-async`
- `examples/module-mixed-sync-async`
