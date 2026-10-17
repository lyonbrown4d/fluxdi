# Derive Macros

FluxDI provides `#[derive(Injectable)]` through the `macros` feature to reduce
boilerplate when registering services with `Shared<T>` dependencies.

Enable:

```toml
[dependencies]
fluxdi = { version = "1.2.1", features = ["macros"] }
```

## Basic Example

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

## Manual vs Macro Parity

`#[derive(Injectable)]` generates a `from_injector` constructor that resolves
each `Shared<T>` field from the injector. The following are equivalent:

**Manual provider:**

```rust
injector.provide::<UserService>(Provider::root(|inj| {
    let repo = inj.resolve::<UserRepository>();
    Shared::new(UserService { repo })
}));
```

**Macro-based provider (same behavior):**

```rust
#[derive(Injectable)]
struct UserService {
    repo: Shared<UserRepository>,
}

injector.provide::<UserService>(Provider::root(|inj| UserService::from_injector(inj)));
```

Use the macro when your service struct has only `Shared<T>` dependencies and
you want to avoid writing the resolution logic by hand. Use manual providers
when you need custom construction (e.g., config parameters, fallible init, or
non-`Shared` fields).

## Generated API

The unreleased workspace generates both constructors:

- `from_injector(&Injector) -> Shared<Self>` keeps the existing panic-on-error behavior.
- `try_from_injector(&Injector) -> Result<Shared<Self>, fluxdi::Error>` propagates dependency errors using `try_resolve::<T>()?` for each `Shared<T>` field.

Use the fallible constructor with a fallible provider:

```rust
injector.provide::<AppService>(Provider::try_root(AppService::try_from_injector));
let service = injector.try_resolve::<AppService>()?;
```

The methods do not catch user panics. See [Fallible Factories](./fallible-factories.md) for failure caching and retry behavior.

## Constraints

- Only named-field structs are supported.
- Generic structs are not supported.
- Every injected field must be typed exactly as `Shared<T>`.

## Runnable Example

See `examples/injectable-macro` for a full demo that mirrors `examples/basic`
using the derive macro.
