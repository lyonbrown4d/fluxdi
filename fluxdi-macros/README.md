# fluxdi-macros

Procedural macros for FluxDI.

The unreleased workspace requires Rust 1.99.1 or newer. The fallible constructor described below is part of the unreleased workspace.

This crate currently exposes one derive macro:

- `#[derive(Injectable)]`

## Recommended Usage

Use macros via `fluxdi`:

```toml
[dependencies]
fluxdi = { version = "1.2.1", features = ["macros"] }
```

Then import from `fluxdi`:

```rust
use fluxdi::Injectable;
```

## What `Injectable` Generates

For a struct:

```rust
#[derive(Injectable)]
struct AppService {
    dep: Shared<MyDep>,
}
```

the macro generates:

```rust
impl AppService {
    pub fn from_injector(injector: &fluxdi::Injector) -> fluxdi::Shared<Self> {
        Self::try_from_injector(injector).unwrap_or_else(|error| panic!("{error}"))
    }

    pub fn try_from_injector(
        injector: &fluxdi::Injector,
    ) -> Result<fluxdi::Shared<Self>, fluxdi::Error> {
        Ok(fluxdi::Shared::new(Self {
            dep: injector.try_resolve::<MyDep>()?,
        }))
    }
}
```

## End-to-End Example

```rust
use fluxdi::{Injectable, Injector, Provider, Shared};

#[derive(Debug)]
struct Clock {
    tick: u64,
}

#[derive(Injectable)]
struct AppService {
    clock: Shared<Clock>,
}

let injector = Injector::root();
injector.provide::<Clock>(Provider::root(|_| Shared::new(Clock { tick: 42 })));
injector.provide::<AppService>(Provider::root(|inj| AppService::from_injector(inj)));

let service = injector.resolve::<AppService>();
assert_eq!(service.clock.tick, 42);
```

## Derive Constraints

`Injectable` currently supports:

- Named-field structs only.
- Non-generic structs only.
- Fields typed exactly as `Shared<T>`.

Not supported:

- Tuple structs and unit structs.
- Enums and unions.
- Generic structs (`struct Service<T> { ... }`).
- Non-`Shared<T>` fields in derived injection.

## Error Behavior

`try_from_injector` propagates the first dependency resolution error. Register it with a fallible provider and resolve with a `try_` method:

```rust
injector.provide::<AppService>(Provider::try_root(AppService::try_from_injector));
let service = injector.try_resolve::<AppService>()?;
```

`from_injector` keeps the existing panic-on-error behavior. Neither method catches panics from user factories.
