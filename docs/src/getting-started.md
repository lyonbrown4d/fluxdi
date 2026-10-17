# Getting Started

## Install

This guide describes the unreleased checkout, requiring **Rust 1.99.1 or newer**.
`rust-toolchain.toml` pins that stable version. Use the local crate until the core and
macro changes receive a coordinated release:

```toml
[dependencies]
fluxdi = { path = "../fluxdi" }
```

Direct dependency versions are maintained in root `[workspace.dependencies]`.
The SeaORM SQLite example is an independent workspace because its native SQLite
binding range differs from the latest rusqlite used by the complex example.
Run it with `cargo run --manifest-path examples/seaorm-sqlite/Cargo.toml --locked`.

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
fluxdi = { path = "../fluxdi", features = ["macros"] }
```

Then:

```rust
use fluxdi::{Injectable, Shared};

#[derive(Injectable)]
struct AppService {
    dep: Shared<MyDependency>,
}
```
