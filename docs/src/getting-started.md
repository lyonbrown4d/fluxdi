# Getting Started

## Install

FluxDI **2.0.0** requires **Rust 1.99.1 or newer**. The repository
`rust-toolchain.toml` pins that stable version. Install the core with its default
synchronous features:

```toml
[dependencies]
fluxdi = "2.0.0"
```

For local development, replace the registry version with `path = "../fluxdi"`.
The core and macro crates use the same 2.0.0 boundary; enabling `macros` selects
exactly `fluxdi-macros` 2.0.0.

Direct dependency versions are maintained in root `[workspace.dependencies]`.
The SeaORM SQLite example is an independent workspace because its native SQLite
binding range differs from the latest rusqlite used by the complex example.
Run it with `cargo run --manifest-path examples/seaorm-sqlite/Cargo.toml --locked`.

## Migrating from 1.x

- Upgrade the toolchain to Rust 1.99.1 or newer. Use `fluxdi` 2.0.0 and its matching `fluxdi-macros` 2.0.0; the core pins that macro version exactly.
- Add arms for `ErrorKind::FactoryFailed` and `DependencyCardinality::Optional` to exhaustive matches.
- Review `Provider::singleton` / `Scope::Module` assumptions: values are cached per calling injector context. Clones share the cache; children and new runtime scopes have their own singleton cache. Use `Provider::root` / `Scope::Root` for sharing throughout an injector tree.
- Reserve the derive-generated inherent method names `from_injector`, `try_from_injector`, `provider`, and `with_injected_dependencies`; rename conflicting application methods. Static modules also reserve `new`, `create_child`, `create_scope`, `in_context`, `injector`, `clone`, `default`, and generated hidden export method names.

## Minimal Example

```rust
use fluxdi::{Application, Injector, Module, Provider, Shared};

#[derive(Debug)]
struct Config(&'static str);

struct AppModule;

impl Module for AppModule {
    fn configure(&self, injector: &Injector) -> Result<(), fluxdi::Error> {
        injector.provide::<Config>(Provider::root(|_| Shared::new(Config("prod"))));
        Ok(())
    }
}

fn main() {
    fluxdi::init_logging();

    let mut app = Application::new(AppModule);
    app.bootstrap_sync().expect("bootstrap failed");

    let config = app.injector().resolve::<Config>();
    println!("mode={}", config.0);
}
```

## Build And Test

```bash
cargo check --workspace
cargo test --workspace
```

## Logging In Local Runs

Enable detailed DI node logs when running examples/apps:

```bash
RUST_LOG=fluxdi=trace cargo run -p basic
```

## Optional Derive Macros

```toml
[dependencies]
fluxdi = { version = "2.0.0", features = ["macros"] }
```

Then:

```rust
use fluxdi::{Injectable, Shared};

#[derive(Injectable)]
struct AppService {
    dep: Shared<MyDependency>,
}
```
