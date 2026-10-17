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

For generated static wiring, enable `macros` as well. Declare async factories with
`async key: Type = factory(dependencies);`; dependent getters become async automatically.
See [Static Modules](./static-modules.md) for imports and async qualifier checks.
