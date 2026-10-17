# fluxdi-macros

[English](README.md) · [简体中文](README.zh-CN.md)

Procedural macros for FluxDI's **unreleased workspace**, requiring **Rust 1.99.1 or
newer**. Coordinate the core and macro release before registry installation. Enable
macros through `fluxdi` during local development:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["macros"] }
```

## Runtime-container derive

```rust
use fluxdi::{Injectable, Scope, Shared};

#[derive(Injectable)]
struct AppService<T: ?Sized> {
    backend: Shared<T>,
    #[inject(name = "fallback")]
    fallback: Option<Shared<T>>,
    plugins: Vec<Shared<T>>,
}

// After registering all required dependencies:
// injector.provide::<AppService<MyBackend>>(AppService::provider(Scope::Root));
```

`Injectable` preserves type/lifetime/const generics and `where` clauses. It supports
named-field structs with `Shared<T>`, `Option<Shared<T>>`, or `Vec<Shared<T>>` fields.
Generated dependency types and the service must satisfy `fluxdi::Service`:
`'static`, plus `Send + Sync` with `thread-safe`.

It generates `from_injector`, `try_from_injector`, `provider(scope)`, and
`with_injected_dependencies(provider)`. The latter two attach dependency graph
hints; constructors alone do not. Constructors are synchronous, do not await async
factories, and do not catch user panics.

Optional fields return `None` only for absent bindings. Factory errors propagate.
Sets require at least one registered member and cannot be named. See the
[derive reference](../docs/src/derive-macros.md) for bounds and error contracts.

## Explicit static assembly

`static_module!` declares a synchronous graph with external inputs and typed factory
calls. Expansion checks missing binding identifiers, cycles, scope capture, and
method conflicts; Rust checks factory argument and return types. Root bindings
cannot capture declared contextual dependencies, even through transient bindings.

Providers initialize lazily with runtime caching and fallible initialization.
Compilation does not prove that network operations, configuration, or external
resources will initialize successfully. Static bindings stay outside the dynamic
registry; dynamic overrides and graph snapshots do not alter or describe them.

The current API has no module imports, automatic discovery, generated async
factories, or lifecycle ordering. See [static modules](../docs/src/static-modules.md)
for syntax and a runnable example, or the [core README](../fluxdi/README.md) for
both registration paths and all feature flags.

## Repository commands

Run from the repository root:

```bash
cargo run -p injectable-macro-example --locked
cargo run -p fluxdi --example static_module --features macros --locked
cargo test --workspace --locked
sh docs/build.sh
```

The documentation command builds the English and Simplified Chinese books.
