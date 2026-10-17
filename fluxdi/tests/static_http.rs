#![cfg(all(feature = "macros", feature = "axum"))]

use std::sync::atomic::{AtomicUsize, Ordering};

use axum::{
    Extension, Router, body::Body, extract::State, http::Request, middleware, routing::get,
};
use fluxdi::{
    Error, Shared,
    axum::{InjectorState, RequestScope, request_scope},
    static_module,
};
use tower::ServiceExt;

fn root() -> Result<Shared<u32>, Error> {
    Ok(Shared::new(42))
}

fn request_id(sequence: Shared<AtomicUsize>) -> Result<Shared<usize>, Error> {
    Ok(Shared::new(sequence.fetch_add(1, Ordering::SeqCst)))
}

static_module! {
    struct App {
        #[scope(root)]
        root: u32 = root();
        #[scope(scoped)]
        request_id: usize = request_id(sequence);
        external sequence: AtomicUsize;
    }
}

#[derive(Clone)]
struct Snapshot {
    id: Shared<usize>,
    root: Shared<u32>,
}

async fn handler(
    State(app): State<App>,
    Extension(scope): Extension<RequestScope>,
) -> Extension<Snapshot> {
    let request_app = app.in_context(scope.injector().as_ref());
    let id = request_app.request_id().unwrap();
    assert!(Shared::ptr_eq(&id, &request_app.request_id().unwrap()));
    Extension(Snapshot {
        id,
        root: request_app.root().unwrap(),
    })
}

#[tokio::test]
async fn static_wiring_uses_http_scope_without_dynamic_registration() {
    let app = App::new(Shared::new(AtomicUsize::new(1)));
    let root = app.root().unwrap();
    assert!(app.injector().dependency_graph().nodes.is_empty());
    let state = InjectorState::new(Shared::new(app.injector().clone()));
    let router = Router::new()
        .route("/", get(handler))
        .layer(middleware::from_fn_with_state(state, request_scope))
        .with_state(app);
    let request = || Request::new(Body::empty());
    let (first, second) =
        futures::join!(router.clone().oneshot(request()), router.oneshot(request()));
    let first = first.unwrap();
    let second = second.unwrap();
    let first = first.extensions().get::<Snapshot>().unwrap();
    let second = second.extensions().get::<Snapshot>().unwrap();
    assert_ne!(*first.id, *second.id);
    assert!(Shared::ptr_eq(&first.root, &root));
    assert!(Shared::ptr_eq(&second.root, &root));
}
