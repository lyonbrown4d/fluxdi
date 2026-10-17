# Static Modules

The unreleased `static_module!` macro provides explicit typed assembly, selected
module imports, and synchronous or asynchronous factories. Enable `macros`; async
graphs also require `async-factory`. The core remains independent of HTTP frameworks.

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

Each synchronous factory is a function path taking the declared `Shared<T>` arguments
and returning `Result<Shared<T>, Error>`. Factories run lazily on first resolution.
External values have root scope. Constructor inputs include import instances and
external values, in their combined textual declaration order.
They form an explicit boundary: the macro cannot inspect how those values were built.

## Import selected bindings

Imports explicitly select source bindings; they do not import a whole registry:

```rust
static_module! {
    struct Data {
        #[scope(root)]
        repository: Repository = repository(config);
        external config: Config;
    }
}

static_module! {
    struct Frontend {
        import data: crate::Data {
            primary_repository: Repository = repository;
        };
        service: Service = service(primary_repository);
        external label: String;
    }
}

let data = Data::new(Shared::new(Config("memory://demo".into())));
let app = Frontend::new(data.clone(), Shared::new(String::from("web")));
let shared_tree = app.in_context(data.injector());
```

The import alias `data` names a constructor input passed by value. Pass `data.clone()`
when the caller needs to keep the source module. Within its block,
`local_key: Type = source_key;` renames a selected binding; omitting `= source_key`
uses the same key in both modules. The block ends with `;`. Module paths can be
qualified Rust paths, including paths to another crate. Same-type bindings remain
separate keys. Local and imported keys must be unique, as must import aliases.

Imports clone typed provider handles, preserving source provider identity and scope.
They do not call source getters, mutate the source, or retain the whole source module
and its context. `Frontend::new` creates a fresh root context; it does not adopt
`data`'s caches, even if `data` was already resolved. Use `in_context(data.injector())`
explicitly to reuse its tree, as `shared_tree` does above. Multiple imports of the
same source instance share provider identities within the importing context.

## Asynchronous factories

```toml
fluxdi = { path = "../fluxdi", features = ["macros", "async-factory"] }
```

Declare `async` when the factory itself returns a future:

```rust
async fn open_repository(config: Shared<Config>) -> Result<Shared<Repository>, Error> {
    Ok(Shared::new(Repository(config)))
}

static_module! {
    struct AsyncData {
        #[scope(root)]
        async repository: Repository = open_repository(config);
        external config: Config;
    }
}

static_module! {
    struct AsyncApp {
        import data: crate::AsyncData {
            async primary_repository: Repository = repository;
        };
        service: Service = service(primary_repository);
    }
}

let data = AsyncData::new(Shared::new(Config("memory://demo".into())));
let app = AsyncApp::new(data);
let service = app.service().await?;
```

An async factory produces `Future<Output = Result<Shared<T>, Error>>`. A binding
with any async dependency also gets an async getter, even when its own factory is
synchronous: `service` above awaits its dependency and then calls `service(...)`
without awaiting that function. This inference follows transitive local dependencies.
Import entries must explicitly mark the source binding's effective async state;
a missing or extra `async` fails compilation, including for reexports and source
bindings promoted by their dependencies.

`new` stays synchronous and lazy. Resolution awaits dependency arguments sequentially
in the order written in the factory call; it does not start them in parallel or add
an executor. Without `thread-safe`, factories can return non-`Send` futures. With
`thread-safe`, the existing factory `Send + Sync` and future `Send` bounds apply.
Only successful initialization is cached; errors allow later resolution to retry.
Canceled initialization releases the initialization state so a later caller can retry.
These are the same rules as [fallible factories](./fallible-factories.md).

## Compilation checks

Macro expansion rejects unknown binding identifiers, duplicate identifiers, constructor
cycles, invalid scopes, and generated-method name conflicts. Cycle diagnostics include
the dependency chain. Factories with incompatible argument or return types fail Rust
type checking.

A root binding cannot transitively capture `singleton` or `scoped` bindings, including
through transient factories. Without scope proxies, such a dependency would retain
the first caller's contextual service in a global cache. Root bindings may depend on
root or external bindings, and may retain values produced by transient factories
whose dependency graph has no contextual binding. Import and reexport metadata
preserve this check across module boundaries, including nested reexports and
transient bridges; importing a contextual source does not make it root-safe.

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

- `new(import_instances_and_external_values...)` creates a fresh root context and typed providers.
- Each binding name is a getter returning `Result<Shared<T>, Error>`, or a future with that output for an effective async binding.
- `clone()` shares the context and provider identities.
- `create_child()` creates an ordinary child context.
- `create_scope()` creates an isolated scoped context.
- `in_context(&Injector)` reuses the providers with the supplied context.
- `injector()` returns the module's current context.

Modules without imports or external inputs also implement `Default`.

Use contexts from the same injector tree when root instances must remain shared.
`in_context` uses the supplied tree's root cache; it does not verify ancestry.
Nested scopes isolate scoped instances. Ordinary children inside a scope reuse its
scoped instances while keeping their own calling-context singleton cache.

Generated wiring holds typed provider handles and calls `try_resolve_provider` directly.
Async wiring uses `try_resolve_provider_async`. Both share the runtime's initialization,
concurrency, error propagation and retry rules.
It does not register those bindings in the dynamic type/name registry. Dynamic overrides
and `dependency_graph()` therefore do not change or describe the static graph. The
remaining cache and cycle-guard machinery still runs at runtime; this API does not
claim zero lookup or zero allocation cost.

## Current boundaries

Imported modules form a closed dependency DAG: a source module cannot depend back
on its importer. Automatic discovery, conditional bindings, implicit optional/set
selection, cyclic module imports, and lifecycle startup/shutdown ordering remain
unsupported. Supply optional values and collections explicitly as external inputs
when appropriate. `Injectable` remains the separate runtime-container derive API;
static assembly is not a complete CDI implementation.

`new`, `create_child`, `create_scope`, `in_context`, `injector`, `clone`, and `default`
are reserved binding names. Collisions with generated hidden export methods are also rejected.
Generated hidden provider and metadata helpers are implementation glue, not an
application API. Module generics are rejected. Module and import attributes support
only documentation; local bindings also accept `#[scope(...)]`.
Publish the core and macro crates together when releasing this API.

See [Web Integrations](./web-integrations.md) for bridging a static module into a
request scope. Run the executable example with:

```bash
cargo run -p fluxdi --example static_module --features macros
cargo run -p fluxdi --example static_composition_async --features macros,async-factory
```
