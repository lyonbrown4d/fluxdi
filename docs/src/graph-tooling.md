# Graph Tooling

FluxDI can export a dependency graph from provider metadata and validate it:

- `injector.dependency_graph()` returns a graph model.
- `injector.validate_graph()` returns missing dependency / cycle issues.
- `injector.try_validate_graph()` returns `Err` when graph is invalid.

## Declaring Dependencies

Graph validation relies on provider dependency hints:

```rust
use fluxdi::{Injector, Provider, Shared};

struct A;
struct B;

let injector = Injector::root();
injector.provide::<A>(
    Provider::singleton(|_| Shared::new(A)).with_dependency::<B>(),
);
injector.provide::<B>(Provider::singleton(|_| Shared::new(B)));

let report = injector.validate_graph();
assert!(report.is_valid());
```

Additional hints:

- `with_named_dependency::<T>("name")`
- `with_set_dependency::<T>()` (requires at least one set member)
- `with_optional_dependency::<T>()`
- `with_optional_named_dependency::<T>("name")`

With `#[derive(Injectable)]`, register `YourType::provider(scope)` to infer these hints from field declarations. For a custom factory, wrap it in `YourType::with_injected_dependencies(provider)`. See [Derive Macros](./derive-macros.md).

Optional hints use `DependencyCardinality::Optional`. An absent optional target is valid. A registered target creates an `optional` or `optional:name` edge and participates in cycle validation. Existing exhaustive matches on `DependencyCardinality` must handle the new variant.

Graph validation only inspects metadata; it does not initialize services or verify that factories will succeed. Dependencies inside arbitrary closures still require explicit hints. Graph snapshots are not a freeze operation and do not prevent later registration changes.

## DOT and Mermaid Output

```rust
let graph = injector.dependency_graph();
let dot = graph.to_dot();
let mermaid = graph.to_mermaid();
```

## CI-friendly Check

You can fail fast in tests or startup:

```rust
injector.try_validate_graph()?;
```

Runnable example:

```bash
cargo run -p graph-tooling-example
```
