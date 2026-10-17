# Providers And Resolution

FluxDI supports four scopes:

- `Provider::transient(...)` creates a new instance for each resolve.
- `Provider::root(...)` caches globally at root injector scope.
- `Provider::singleton(...)` caches once per calling injector context (`Scope::Module`). Cloned injector handles share that context; child injectors and new runtime scopes have their own caches.
- `Provider::scoped(...)` caches once per runtime scope created via `create_scope()`.

A provider inherited from a parent is resolved using the calling context. A local
binding takes precedence over a parent's cached instance. A nested `create_scope()`
starts a new scoped cache; ordinary child injectors inside that scope share the
nearest scoped cache. Root bindings remain shared across the entire hierarchy.

Concurrent first resolves of a cached binding share one initialization. Async
waiters can retry after an initializer is canceled or panics. If synchronous
resolution encounters an initializer already running asynchronously, it returns
`AsyncFactoryRequiresAsyncResolve` rather than blocking the executor; use async resolution
to wait. Resolving an uncached async factory synchronously still returns
`AsyncFactoryRequiresAsyncResolve`.

Example:

```rust
use fluxdi::{Injector, Provider, Shared};

#[derive(Debug)]
struct Logger;

let injector = Injector::root();
injector.provide::<Logger>(Provider::root(|_| Shared::new(Logger)));

let logger = injector.resolve::<Logger>();
```

Scoped context example:

```rust
use fluxdi::{Injector, Provider, Shared};

#[derive(Debug)]
struct RequestContext(usize);

let injector = Injector::root();
injector.provide::<RequestContext>(Provider::scoped(|_| Shared::new(RequestContext(1))));

let scope = injector.create_scope();
let first = scope.resolve::<RequestContext>();
let second = scope.resolve::<RequestContext>();

assert!(Shared::ptr_eq(&first, &second));
```

Prefer `try_resolve::<T>()` when you need recoverable errors:

```rust
let logger = injector.try_resolve::<Logger>()?;
```

## Named Bindings

You can register multiple implementations for the same type/trait by name:

```rust
use fluxdi::{Injector, Provider, Shared};

trait Cache: Send + Sync {
    fn id(&self) -> &'static str;
}

struct Redis;
impl Cache for Redis {
    fn id(&self) -> &'static str { "redis" }
}

struct Memory;
impl Cache for Memory {
    fn id(&self) -> &'static str { "memory" }
}

let injector = Injector::root();

injector.provide_named::<dyn Cache>(
    "primary",
    Provider::root(|_| Shared::new(Redis) as Shared<dyn Cache>),
);
injector.provide_named::<dyn Cache>(
    "fallback",
    Provider::root(|_| Shared::new(Memory) as Shared<dyn Cache>),
);

let primary = injector.resolve_named::<dyn Cache>("primary");
let fallback = injector.resolve_named::<dyn Cache>("fallback");

assert_eq!(primary.id(), "redis");
assert_eq!(fallback.id(), "memory");
```

## Multi-binding

You can register multiple providers into a set and resolve all of them:

```rust
use fluxdi::{Injector, Provider, Shared};

trait Middleware: Send + Sync {
    fn name(&self) -> &'static str;
}

struct Trim;
impl Middleware for Trim {
    fn name(&self) -> &'static str { "trim" }
}

struct Uppercase;
impl Middleware for Uppercase {
    fn name(&self) -> &'static str { "uppercase" }
}

let injector = Injector::root();
injector.provide_into_set::<dyn Middleware>(
    Provider::singleton(|_| Shared::new(Trim) as Shared<dyn Middleware>),
);
injector.provide_into_set::<dyn Middleware>(
    Provider::singleton(|_| Shared::new(Uppercase) as Shared<dyn Middleware>),
);

let pipeline = injector.resolve_all::<dyn Middleware>();
let names: Vec<&str> = pipeline.iter().map(|m| m.name()).collect();
assert_eq!(names, vec!["trim", "uppercase"]);
```

Ordering is deterministic:

- Parent injector set bindings are resolved first.
- Child injector set bindings are resolved after parent bindings.
- Within one injector, registration order is preserved.

## Provider Override

For tests and integration environments, you can replace a registered provider:

```rust
use fluxdi::{Injector, Provider, Shared};

let injector = Injector::root();
injector.provide::<String>(Provider::root(|_| Shared::new("real".to_string())));

injector
    .try_override_provider::<String>(Provider::root(|_| Shared::new("mock".to_string())))
    .unwrap();
```

Notes:

- Override requires an existing provider.
- Root-scoped overrides affect root resolution for child injectors.
- Cached instances for the overridden type are invalidated.

## Error Suggestions

FluxDI errors include actionable hints. Common examples:

- Missing service: suggests registering inside `Module::configure(...)`.
- Async provider resolved via sync path: suggests `try_resolve_async/resolve_async`.
- Duplicate provider: suggests `override_provider/try_override_provider`.
