# Getting Started

## Install

The unreleased workspace requires **Rust 1.99.1 or newer**. `rust-toolchain.toml` pins the development toolchain to 1.99.1; all workspace packages declare the same minimum version. The published dependency examples below refer to earlier releases.

```toml
[dependencies]
fluxdi = "1.2.1"
```

For local workspace usage:

```toml
[dependencies]
fluxdi = { path = "../fluxdi" }
```

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
fluxdi = { version = "1.2.1", features = ["macros"] }
```

Then:

```rust
use fluxdi::{Injectable, Shared};

#[derive(Injectable)]
struct AppService {
    dep: Shared<MyDependency>,
}
```
