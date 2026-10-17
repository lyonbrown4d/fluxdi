# Web Integrations

FluxDI provides optional Axum (`axum`) and Actix-web (`actix`) adapters. Both enable
`thread-safe`; the core injector and static modules do not require either framework.

## Request middleware

Install `request_scope` once around the routes that need request isolation. It creates
one `Injector::create_scope()` context per request. `Resolved<T>` uses that context,
so multiple extractors and handler resolutions share scoped instances. Concurrent
requests have separate contexts. Root bindings remain shared across the injector tree;
calling-context singletons have a cache per request context. Nested installations
reuse the existing scope.

Without this middleware, `Resolved<T>` continues to resolve from framework state.
It does not implicitly create a scope per extractor.

### Axum

```rust
use axum::{Router, middleware, routing::get};
use fluxdi::{Injector, Provider, Shared};
use fluxdi::axum::{InjectorState, Resolved, request_scope};

struct Context;

async fn handler(Resolved(a): Resolved<Context>, Resolved(b): Resolved<Context>) -> String {
    Shared::ptr_eq(&a, &b).to_string()
}

let injector = Shared::new(Injector::root());
injector.provide(Provider::scoped(|_| Shared::new(Context)));
let state = InjectorState::new(injector);
let app: Router = Router::new()
    .route("/", get(handler))
    .layer(middleware::from_fn_with_state(state.clone(), request_scope))
    .with_state(state);
```

Use `Extension<fluxdi::axum::RequestScope>` to obtain the context. Its `injector()`
returns a cloned injector handle for named bindings, sets, or request-local values.
Place middleware that needs this handle downstream of `request_scope`, accounting
for Axum's layer execution order.

### Actix-web

```rust
use actix_web::{App, HttpResponse, middleware, web};
use fluxdi::{Injector, Provider, Shared};
use fluxdi::actix::{Resolved, injector_data, request_scope};

struct Context;

async fn handler(Resolved(a): Resolved<Context>, Resolved(b): Resolved<Context>) -> HttpResponse {
    HttpResponse::Ok().body(Shared::ptr_eq(&a, &b).to_string())
}

let injector = Shared::new(Injector::root());
injector.provide(Provider::scoped(|_| Shared::new(Context)));
let app = App::new()
    .app_data(injector_data(injector))
    .wrap(middleware::from_fn(request_scope))
    .route("/", web::get().to(handler));
```

The middleware requires `InjectorState` in `app_data`; missing configuration is an
internal-server error. Access a live context with
`request.extensions().get::<fluxdi::actix::RequestScope>()` after importing
`actix_web::HttpMessage`. Clone the scope before awaiting. Middleware that needs it
must execute after scope installation.

Both adapters support asynchronous factories when `async-factory` is enabled.
Provider errors remain errors; extractors retry via async resolution only for
`AsyncFactoryRequiresAsyncResolve`.

## Resource lifetime

The middleware holds its scope while the downstream request future runs. Completion,
error, or cancellation releases that handle. Actix removes its installed extension
from successful responses and clears request extensions when returning requests to
its pool. An explicitly retained request, scope, service, or injector handle can
keep resources alive longer.

The end of request processing means producing the response, not consuming its entire
streaming body. A streaming response or spawned task must capture the service or
scope handle it needs. Dropping a scope releases its caches through normal `Arc`
ownership; there is no automatic async shutdown hook for each request. External
resources must implement their own cancellation and cleanup behavior.

Avoid root factories that dynamically capture request-scoped services: a root cache
could retain the first request's value. The static graph rejects declared contextual
dependencies of root bindings; arbitrary dynamic factories remain the caller's
responsibility.

## Static modules in request scopes

Static modules keep typed providers outside the dynamic registry. Use the request
context explicitly rather than asking `Resolved<T>` to locate a static binding:

```rust,ignore
// The middleware state must use this module's injector tree.
let di = InjectorState::new(Shared::new(app.injector().clone()));

// In an Axum handler with State(app) and Extension(scope):
let request_app = app.in_context(scope.injector().as_ref());
let service = request_app.service()?;
```

The same `in_context` call works with the Actix request scope. See
[Static Modules](./static-modules.md) for the assembly API. The integration test
`fluxdi/tests/static_http.rs` exercises this bridge through the Axum pipeline.

## Relevant examples

- `examples/axum`: middleware and two extractors at `GET /scoped/debug`
- `examples/actix`: middleware and two extractors at `GET /hello/scoped`
- `examples/axum-lifecycle`: application lifecycle integration
- `examples/scoped-context`: framework-independent contexts
- `examples/dual-http-random-port`: server lifecycle
