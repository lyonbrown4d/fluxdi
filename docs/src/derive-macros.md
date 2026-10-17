# Derive Macros

The unreleased workspace extends `#[derive(Injectable)]` with generics, named bindings, optional dependencies, set dependencies, and inferred graph hints. It requires Rust 1.99.1 or newer.

For local workspace usage:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["macros"] }
```

## Register a Derived Service

```rust
use fluxdi::{Injectable, Injector, Provider, Scope, Shared};

struct Clock(u64);

#[derive(Injectable)]
struct AppService {
    clock: Shared<Clock>,
}

fn main() -> Result<(), fluxdi::Error> {
    let injector = Injector::root();
    injector.provide::<Clock>(Provider::root(|_| Shared::new(Clock(42))));
    injector.provide::<AppService>(AppService::provider(Scope::Root));
    injector.try_validate_graph()?;
    assert_eq!(injector.try_resolve::<AppService>()?.clock.0, 42);
    Ok(())
}
```

`YourType::provider(scope)` creates a fallible provider and attaches graph hints inferred from the fields. Use `Scope::Module` for calling-context singleton behavior, or `Root`, `Scoped`, or `Transient` for their respective lifetimes. Decorators, limits, named registration, and set registration remain available on the resulting provider.

## Field Declarations

| Field | Resolution | Graph requirement |
| --- | --- | --- |
| `Shared<T>` | `try_resolve::<T>()` | One unnamed binding |
| `#[inject(name = "main")] Shared<T>` | `try_resolve_named::<T>("main")` | That exact named binding |
| `Option<Shared<T>>` | `try_resolve_optional::<T>()` | Binding may be absent |
| `#[inject(name = "main")] Option<Shared<T>>` | `try_resolve_optional_named::<T>("main")` | That named binding may be absent |
| `Vec<Shared<T>>` | `try_resolve_all::<T>()` | At least one set member |

Qualified paths such as `fluxdi::Shared<T>`, `std::option::Option<Shared<T>>`, and `std::vec::Vec<Shared<T>>` are accepted. Arbitrarily renamed type aliases are not inspected. Named set dependencies, unknown options, duplicate names, and unsupported field shapes are rejected.

Optional dependencies return `None` only when the binding itself is absent. Missing dependencies inside an existing factory, initialization failures, async-resolution requirements, and circular dependencies remain errors. The older `optional_resolve*` helpers discard all errors; derived constructors use the new fallible helpers instead.

Set dependencies retain the existing `resolve_all` contract: no members is an error, and members are resolved in parent-to-child registration order. A member initialization error propagates. Sets cannot be named.

## Generics

Type, lifetime, and const parameters and existing `where` clauses are preserved:

```rust
#[derive(Injectable)]
struct Pipeline<T: ?Sized, const N: usize>
where
    T: Backend,
{
    backend: Shared<T>,
    #[inject(name = "fallback")]
    fallback: Option<Shared<T>>,
    plugins: Vec<Shared<T>>,
    bytes: Shared<[u8; N]>,
}
```

The generated impl requires each dependency type and the service itself to implement `fluxdi::Service`. This blanket marker requires `'static`; with `thread-safe`, it also requires `Send + Sync`. A generic declaration does not have to duplicate those bounds. Local runtime builds still accept non-`Send` services. Borrowed dependencies must have a `'static` lifetime to participate in DI.

## Generated API

The derive generates four inherent methods:

- `from_injector(&Injector) -> Shared<Self>`: constructs synchronously, panicking on errors.
- `try_from_injector(&Injector) -> Result<Shared<Self>, Error>`: constructs synchronously and propagates dependency errors.
- `provider(Scope) -> Provider<Self>`: creates a fallible provider with inferred graph hints.
- `with_injected_dependencies(Provider<Self>) -> Provider<Self>`: attaches the same hints to a custom factory without replacing it.

If a type already defines a method with one of these names, rename that method before deriving. The original constructors remain available. Calls through `Provider::root(AppService::from_injector)` or `Provider::try_root(AppService::try_from_injector)` still work, but do not automatically attach hints: arbitrary provider closures remain opaque.

For a custom constructor or an async factory, attach the hints explicitly once:

```rust
let provider = AppService::with_injected_dependencies(
    Provider::try_root(|inj| AppService::try_from_injector(inj))
);
injector.provide::<AppService>(provider);
```

Constructors are synchronous; they do not await async dependency factories. Neither constructor catches user panics. See [Fallible Factories](./fallible-factories.md) for initialization errors and retry rules, and [Graph Tooling](./graph-tooling.md) for validation.

## Constraints and Example

Only named-field structs are supported. Tuple/unit structs, enums, unions, plain configuration fields, and nested wrappers beyond the shapes above are not supported. Use a manual factory for those constructions.

Run `cargo run -p injectable-macro-example` to see registration and graph validation together.
