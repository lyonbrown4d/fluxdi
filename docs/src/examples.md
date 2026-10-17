# Examples

The main workspace contains the following examples:

- `fluxdi/examples/static_module.rs` (compile-time graph and typed assembly; requires `macros`)

- `examples/basic`
- `examples/injectable-macro` (macro vs manual parity)
- `examples/decorator` (Provider::with_decorator for logging/caching)
- `examples/complex`
- `examples/async-factory`
- `examples/module-sync`
- `examples/module-async`
- `examples/module-mixed-sync-async`
- `examples/scoped-context`
- `examples/named-bindings`
- `examples/multi-binding-pipeline`
- `examples/graph-tooling`
- `examples/dual-http-random-port`
- `examples/axum`
- `examples/axum-lifecycle`
- `examples/actix`

`examples/seaorm-sqlite` is an independent workspace. The latest SeaORM/SQLx and
rusqlite require incompatible `libsqlite3-sys` versions; separate lockfiles allow
both integrations to use their latest stable releases.

Scoped web request demos:

- `examples/axum` -> `GET /scoped/debug`
- `examples/actix` -> `GET /hello/scoped`

Run one example:

```bash
cargo run -p basic
cargo run -p graph-tooling-example
cargo run -p injectable-macro-example
cargo run -p decorator-example
cargo run -p fluxdi --example static_module --features macros
cargo run --manifest-path examples/seaorm-sqlite/Cargo.toml --locked
```

Examples initialize FluxDI logging by default. For verbose DI-node diagnostics:

```bash
RUST_LOG=fluxdi=trace cargo run -p basic
```
