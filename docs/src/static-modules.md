# Static Modules

The unreleased `static_module!` macro adds an explicit, synchronous assembly path.
Enable `macros`; the core remains independent of HTTP frameworks.

```toml
fluxdi = { path = "../fluxdi", features = ["macros"] }
```

## Declare the graph

```rust
use fluxdi::{Error, Shared, static_module};

struct Config(String);
struct Repository(Shared<Config>);
struct Service(Shared<Repository>);

fn repository(config: Shared<Config>) -> Result<Shared<Repository>, Error> {
    Ok(Shared::new(Repository(config)))
}

fn service(repository: Shared<Repository>) -> Result<Shared<Service>, Error> {
    Ok(Shared::new(Service(repository)))
}

static_module! {
    struct App {
        #[scope(scoped)]
        service: Service = service(repository);
        #[scope(root)]
        repository: Repository = repository(config);
        external config: Config;
    }
}

let app = App::new(Shared::new(Config("memory://demo".into())));
let request = app.create_scope();
let a = request.service()?;
let b = request.service()?;
assert!(Shared::ptr_eq(&a, &b));
# Ok::<(), Error>(())
```

Binding identifiers select dependencies. Multiple bindings may have the same type;
`service(primary_repository)` explicitly selects one. Trait objects and type aliases
are supported, and Rust checks factory argument and return types. Declaration order
does not determine construction order.

Every factory is a function path taking the declared `Shared<T>` arguments and
returning `Result<Shared<T>, Error>`. Factories run lazily on first resolution.
External values are passed to `new` in their declaration order and have root scope.
They form an explicit boundary: the macro cannot inspect how those values were built.

## Compilation checks

Macro expansion rejects unknown binding identifiers, duplicate identifiers, constructor
cycles, invalid scopes, and generated-method name conflicts. Cycle diagnostics include
the dependency chain. Factories with incompatible argument or return types fail Rust
type checking.

A root binding cannot transitively capture `singleton` or `scoped` bindings, including
through transient factories. Without scope proxies, such a dependency would retain
the first caller's contextual service in a global cache. Root bindings may depend on
root or external bindings, and may retain values produced by transient factories
whose dependency graph has no contextual binding.

These checks cover the declared graph. A factory's internal network calls, configuration,
global state, or calls to a separately captured dynamic container remain runtime concerns.
Compilation does not prove that initialization will succeed.

## Contexts and caching

| Declaration | Cache location |
| --- | --- |
| No scope attribute, or `#[scope(singleton)]` | Calling injector context |
| `#[scope(root)]` | Root of the injector tree |
| `#[scope(scoped)]` | Nearest `create_scope()` boundary |
| `#[scope(transient)]` | No cache |

Generated methods:

- `new(external_values...)` creates a root context and typed providers.
- Each binding name is a getter returning `Result<Shared<T>, Error>`.
- `clone()` shares the context and provider identities.
- `create_child()` creates an ordinary child context.
- `create_scope()` creates an isolated scoped context.
- `in_context(&Injector)` reuses the providers with the supplied context.
- `injector()` returns the module's current context.

Modules without external inputs also implement `Default`.

Use contexts from the same injector tree when root instances must remain shared.
`in_context` uses the supplied tree's root cache; it does not verify ancestry.
Nested scopes isolate scoped instances. Ordinary children inside a scope reuse its
scoped instances while keeping their own calling-context singleton cache.

Generated wiring holds typed provider handles and calls `try_resolve_provider` directly.
It shares the runtime's initialization, concurrency, error propagation and retry rules.
It does not register those bindings in the dynamic type/name registry. Dynamic overrides
and `dependency_graph()` therefore do not change or describe the static graph. The
remaining cache and cycle-guard machinery still runs at runtime; this API does not
claim zero lookup or zero allocation cost.

## Current boundaries

This first static assembly API supports one closed synchronous graph with external
inputs. It does not yet provide module imports, automatic discovery, conditional
bindings, implicit optional/set selection, generated async factories, or lifecycle
startup/shutdown ordering. Supply optional values, collections, or already initialized
async resources explicitly as external inputs when appropriate. `Injectable` remains
the separate runtime-container derive API.

`new`, `create_child`, `create_scope`, `in_context`, `injector`, `clone`, and `default` are reserved
binding names. Module generics and non-documentation attributes are rejected. Publish
the core and macro crates together when releasing this API.

See [Web Integrations](./web-integrations.md) for bridging a static module into a
request scope. Run the executable example with:

```bash
cargo run -p fluxdi --example static_module --features macros
```
