# fluxdi-macros

Procedural macros for FluxDI. The unreleased workspace requires Rust 1.99.1 or newer.

Enable the macro through `fluxdi` during local development:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["macros"] }
```

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

`Injectable` preserves type/lifetime/const generics and `where` clauses. It generates `from_injector`, `try_from_injector`, `provider(scope)`, and `with_injected_dependencies(provider)`. The latter two attach dependency graph hints; the original constructors alone do not.

Optional fields return `None` only for absent bindings. Factory errors propagate. Sets require at least one registered member and cannot be named. Only named-field structs with `Shared<T>`, `Option<Shared<T>>`, or `Vec<Shared<T>>` fields are supported.

See the [derive reference](../docs/src/derive-macros.md) for bounds, error semantics, generated method names, and custom provider integration.
