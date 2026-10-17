#![cfg(feature = "axum")]

use std::sync::{
    Mutex, Weak,
    atomic::{AtomicUsize, Ordering},
};

use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware,
    routing::get,
};
use fluxdi::{
    Injector, Provider, Shared,
    axum::{InjectorState, RequestScope, Resolved, request_scope},
};
use tower::ServiceExt;

struct Scoped(usize);
struct Module(usize);
struct Root(usize);

type Observed = Shared<Mutex<Vec<Weak<Scoped>>>>;

fn fixture() -> (InjectorState, Observed) {
    let injector = Shared::new(Injector::root());
    let sequence = Shared::new(AtomicUsize::new(1));
    let observed = Shared::new(Mutex::new(Vec::new()));
    let records = observed.clone();
    let ids = sequence.clone();
    injector.provide(Provider::scoped(move |_| {
        let service = Shared::new(Scoped(ids.fetch_add(1, Ordering::SeqCst)));
        records.lock().unwrap().push(Shared::downgrade(&service));
        service
    }));
    let ids = sequence.clone();
    injector.provide(Provider::singleton(move |_| {
        Shared::new(Module(ids.fetch_add(1, Ordering::SeqCst)))
    }));
    injector.provide(Provider::root(move |_| {
        Shared::new(Root(sequence.fetch_add(1, Ordering::SeqCst)))
    }));
    (InjectorState::new(injector), observed)
}

async fn inspect(
    Extension(scope): Extension<RequestScope>,
    Resolved(first): Resolved<Scoped>,
    Resolved(second): Resolved<Scoped>,
    Resolved(module): Resolved<Module>,
    Resolved(root): Resolved<Root>,
) -> String {
    assert!(Shared::ptr_eq(&first, &second));
    let direct = scope.injector().try_resolve::<Scoped>().unwrap();
    assert!(Shared::ptr_eq(&first, &direct));
    tokio::task::yield_now().await;
    format!("{},{},{}", first.0, module.0, root.0)
}

fn request() -> Request<Body> {
    Request::builder().uri("/").body(Body::empty()).unwrap()
}

async fn ids(response: axum::response::Response) -> Vec<usize> {
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    std::str::from_utf8(&body)
        .unwrap()
        .split(',')
        .map(|value| value.parse().unwrap())
        .collect()
}

#[tokio::test]
async fn middleware_shares_within_request_and_isolates_overlapping_requests() {
    let (state, observed) = fixture();
    let app = Router::new()
        .route("/", get(inspect))
        .layer(middleware::from_fn_with_state(state.clone(), request_scope))
        .with_state(state);
    let first = ids(app.clone().oneshot(request()).await.unwrap()).await;
    let (left, right) = futures::join!(
        app.clone().oneshot(request()),
        app.clone().oneshot(request())
    );
    let left = ids(left.unwrap()).await;
    let right = ids(right.unwrap()).await;
    assert_ne!(first[0], left[0]);
    assert_ne!(left[0], right[0]);
    assert_ne!(first[1], left[1]);
    assert_ne!(left[1], right[1]);
    assert_eq!(first[2], left[2]);
    assert_eq!(left[2], right[2]);
    assert!(
        observed
            .lock()
            .unwrap()
            .iter()
            .all(|item| item.upgrade().is_none())
    );
}

#[tokio::test]
async fn omitted_middleware_preserves_existing_injector_context() {
    let (state, _) = fixture();
    let app = Router::new()
        .route(
            "/",
            get(|Resolved(service): Resolved<Scoped>| async move { service.0.to_string() }),
        )
        .with_state(state);
    let first = ids(app.clone().oneshot(request()).await.unwrap()).await;
    let second = ids(app.oneshot(request()).await.unwrap()).await;
    assert_eq!(first, second);
}

#[tokio::test]
async fn scope_is_released_on_response_error_and_cancellation() {
    let (state, observed) = fixture();
    let app = Router::new()
        .route(
            "/",
            get(|_: Resolved<Scoped>| async { StatusCode::BAD_REQUEST }),
        )
        .route(
            "/pending",
            get(|service: Resolved<Scoped>| async move {
                futures::future::pending::<()>().await;
                service.0.0.to_string()
            }),
        )
        .layer(middleware::from_fn_with_state(state.clone(), request_scope))
        .with_state(state);
    let response = app.clone().oneshot(request()).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(observed.lock().unwrap()[0].upgrade().is_none());
    let pending = Request::builder()
        .uri("/pending")
        .body(Body::empty())
        .unwrap();
    let mut future = Box::pin(app.oneshot(pending));
    assert!(futures::poll!(future.as_mut()).is_pending());
    assert!(observed.lock().unwrap()[1].upgrade().is_some());
    drop(future);
    assert!(observed.lock().unwrap()[1].upgrade().is_none());
}

#[tokio::test]
async fn nested_middleware_reuses_scope_and_cloned_handle_can_outlive_response() {
    async fn observe(
        mut request: Request<Body>,
        next: middleware::Next,
    ) -> axum::response::Response {
        let scope = request.extensions().get::<RequestScope>().unwrap();
        let service = scope.injector().try_resolve::<Scoped>().unwrap();
        request.extensions_mut().insert(service);
        next.run(request).await
    }
    let (state, observed) = fixture();
    let app = Router::new()
        .route(
            "/",
            get(
                |Extension(scope): Extension<RequestScope>,
                 Extension(expected): Extension<Shared<Scoped>>,
                 Resolved(service): Resolved<Scoped>| async move {
                    assert!(Shared::ptr_eq(&expected, &service));
                    let mut response = axum::response::Response::new(Body::empty());
                    response.extensions_mut().insert(scope);
                    response
                },
            ),
        )
        .layer(middleware::from_fn_with_state(state.clone(), request_scope))
        .layer(middleware::from_fn(observe))
        .layer(middleware::from_fn_with_state(state.clone(), request_scope))
        .with_state(state);
    let response = app.oneshot(request()).await.unwrap();
    let scope = response.extensions().get::<RequestScope>().unwrap().clone();
    let service = scope.injector().try_resolve::<Scoped>().unwrap();
    assert_eq!(observed.lock().unwrap().len(), 1);
    assert!(Shared::ptr_eq(
        &service,
        &observed.lock().unwrap()[0].upgrade().unwrap()
    ));
    drop(service);
    drop(response);
    assert!(observed.lock().unwrap()[0].upgrade().is_some());
    drop(scope);
    assert!(observed.lock().unwrap()[0].upgrade().is_none());
}

#[tokio::test]
async fn unresolved_dependency_returns_controlled_internal_error() {
    let state = InjectorState::new(Shared::new(Injector::root()));
    let app = Router::new()
        .route("/", get(|_: Resolved<Scoped>| async { "unreachable" }))
        .layer(middleware::from_fn_with_state(state.clone(), request_scope))
        .with_state(state);
    let response = app.oneshot(request()).await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[cfg(feature = "async-factory")]
#[tokio::test]
async fn asynchronous_provider_is_shared_by_request_extractors() {
    let injector = Shared::new(Injector::root());
    injector.provide(Provider::scoped_async(|_| async {
        tokio::task::yield_now().await;
        Shared::new(Scoped(42))
    }));
    let state = InjectorState::new(injector);
    let app = Router::new()
        .route(
            "/",
            get(
                |Resolved(first): Resolved<Scoped>, Resolved(second): Resolved<Scoped>| async move {
                    assert!(Shared::ptr_eq(&first, &second));
                    first.0.to_string()
                },
            ),
        )
        .layer(middleware::from_fn_with_state(state.clone(), request_scope))
        .with_state(state);
    assert_eq!(ids(app.oneshot(request()).await.unwrap()).await, vec![42]);
}
