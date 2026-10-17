#![cfg(feature = "actix")]

use actix_web::{App, HttpResponse, http::StatusCode, test, web};
use fluxdi::{
    Injector, Provider, Shared,
    actix::{Resolved, injector_data},
};

#[derive(Debug)]
struct Greeter {
    message: String,
}

async fn greet(Resolved(greeter): Resolved<Greeter>) -> HttpResponse {
    HttpResponse::Ok().body(greeter.message.clone())
}

#[actix_web::test]
async fn resolved_extractor_resolves_dependency_from_app_data() {
    let injector = Shared::new(Injector::root());
    injector.provide::<Greeter>(Provider::root(|_| {
        Shared::new(Greeter {
            message: "hello".to_string(),
        })
    }));

    let app = test::init_service(
        App::new()
            .app_data(injector_data(injector.clone()))
            .route("/", web::get().to(greet)),
    )
    .await;

    let request = test::TestRequest::get().uri("/").to_request();
    let response = test::call_service(&app, request).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = test::read_body(response).await;
    assert_eq!(body.as_ref(), b"hello");
}

#[actix_web::test]
async fn resolved_extractor_returns_internal_error_when_state_missing() {
    let app = test::init_service(App::new().route("/", web::get().to(greet))).await;

    let request = test::TestRequest::get().uri("/").to_request();
    let response = test::call_service(&app, request).await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

use actix_web::{HttpMessage, middleware};
use fluxdi::actix::{RequestScope, request_scope};
use std::sync::{
    Mutex, Weak,
    atomic::{AtomicUsize, Ordering},
};

struct Scoped(usize);
struct Module(usize);
struct Root(usize);
type Observed = Shared<Mutex<Vec<Weak<Scoped>>>>;

fn fixture() -> (Shared<Injector>, Observed) {
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
    (injector, observed)
}

async fn inspect(
    request: actix_web::HttpRequest,
    Resolved(first): Resolved<Scoped>,
    Resolved(second): Resolved<Scoped>,
    Resolved(module): Resolved<Module>,
    Resolved(root): Resolved<Root>,
) -> String {
    assert!(Shared::ptr_eq(&first, &second));
    let scope = request.extensions().get::<RequestScope>().unwrap().clone();
    let direct = scope.injector().try_resolve::<Scoped>().unwrap();
    assert!(Shared::ptr_eq(&first, &direct));
    tokio::task::yield_now().await;
    format!("{},{},{}", first.0, module.0, root.0)
}

fn parse_ids(body: &[u8]) -> Vec<usize> {
    std::str::from_utf8(body)
        .unwrap()
        .split(',')
        .map(|value| value.parse().unwrap())
        .collect()
}

#[actix_web::test]
async fn middleware_shares_within_request_and_isolates_overlapping_requests() {
    let (injector, observed) = fixture();
    let app = test::init_service(
        App::new()
            .app_data(injector_data(injector))
            .wrap(middleware::from_fn(request_scope))
            .route("/", web::get().to(inspect)),
    )
    .await;
    let response = test::call_service(&app, test::TestRequest::get().to_request()).await;
    assert_eq!(response.status(), StatusCode::OK);
    // A still-live ServiceResponse must not retain the completed request scope.
    assert!(
        response
            .request()
            .extensions()
            .get::<RequestScope>()
            .is_none()
    );
    assert!(observed.lock().unwrap()[0].upgrade().is_none());
    let first = parse_ids(&test::read_body(response).await);
    let (left, right) = futures::join!(
        test::call_service(&app, test::TestRequest::get().to_request()),
        test::call_service(&app, test::TestRequest::get().to_request())
    );
    let left = parse_ids(&test::read_body(left).await);
    let right = parse_ids(&test::read_body(right).await);
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

#[actix_web::test]
async fn omitted_middleware_preserves_existing_injector_context() {
    let (injector, _) = fixture();
    let app = test::init_service(App::new().app_data(injector_data(injector)).route(
        "/",
        web::get().to(|Resolved(service): Resolved<Scoped>| async move { service.0.to_string() }),
    ))
    .await;
    let first = test::call_service(&app, test::TestRequest::get().to_request()).await;
    let first = test::read_body(first).await;
    let second = test::call_service(&app, test::TestRequest::get().to_request()).await;
    assert_eq!(first, test::read_body(second).await);
}

#[actix_web::test]
async fn middleware_returns_controlled_error_when_state_missing() {
    let app = test::init_service(
        App::new()
            .wrap(middleware::from_fn(request_scope))
            .route("/", web::get().to(|| async { "unreachable" })),
    )
    .await;
    let error = test::try_call_service(&app, test::TestRequest::get().to_request())
        .await
        .unwrap_err();
    assert_eq!(
        error.as_response_error().status_code(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[actix_web::test]
async fn scope_is_released_on_response_error_and_cancellation() {
    async fn reject(
        request: actix_web::dev::ServiceRequest,
        _: middleware::Next<actix_web::body::BoxBody>,
    ) -> Result<actix_web::dev::ServiceResponse, actix_web::Error> {
        let scope = request.extensions().get::<RequestScope>().unwrap().clone();
        let _service = scope.injector().try_resolve::<Scoped>().unwrap();
        Err(actix_web::error::ErrorBadRequest("middleware rejected"))
    }
    let (injector, observed) = fixture();
    let app = test::init_service(
        App::new()
            .app_data(injector_data(injector.clone()))
            .wrap(middleware::from_fn(request_scope))
            .route(
                "/",
                web::get().to(|_: Resolved<Scoped>| async {
                    Err::<HttpResponse, _>(actix_web::error::ErrorBadRequest("rejected"))
                }),
            )
            .route(
                "/pending",
                web::get().to(|service: Resolved<Scoped>| async move {
                    futures::future::pending::<()>().await;
                    service.0.0.to_string()
                }),
            ),
    )
    .await;
    let response = test::call_service(&app, test::TestRequest::get().to_request()).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(observed.lock().unwrap()[0].upgrade().is_none());
    let mut future = Box::pin(test::call_service(
        &app,
        test::TestRequest::get().uri("/pending").to_request(),
    ));
    assert!(futures::poll!(future.as_mut()).is_pending());
    assert!(observed.lock().unwrap()[1].upgrade().is_some());
    drop(future);
    assert!(observed.lock().unwrap()[1].upgrade().is_none());
    let rejecting = test::init_service(
        App::new()
            .app_data(injector_data(injector))
            .wrap(middleware::from_fn(reject))
            .wrap(middleware::from_fn(request_scope)),
    )
    .await;
    let error = test::try_call_service(&rejecting, test::TestRequest::get().to_request())
        .await
        .unwrap_err();
    assert_eq!(
        error.as_response_error().status_code(),
        StatusCode::BAD_REQUEST
    );
    assert!(observed.lock().unwrap()[2].upgrade().is_none());
}

#[actix_web::test]
async fn nested_middleware_reuses_scope_and_cloned_handle_can_outlive_response() {
    async fn observe(
        request: actix_web::dev::ServiceRequest,
        next: middleware::Next<impl actix_web::body::MessageBody>,
    ) -> Result<actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>, actix_web::Error>
    {
        let scope = request.extensions().get::<RequestScope>().unwrap().clone();
        request
            .extensions_mut()
            .insert(scope.injector().try_resolve::<Scoped>().unwrap());
        next.call(request).await
    }
    let (injector, observed) = fixture();
    let saved = Shared::new(Mutex::new(None::<RequestScope>));
    let captured = saved.clone();
    let app = test::init_service(
        App::new()
            .app_data(injector_data(injector))
            .wrap(middleware::from_fn(request_scope))
            .wrap(middleware::from_fn(observe))
            .wrap(middleware::from_fn(request_scope))
            .route(
                "/",
                web::get().to(
                    move |request: actix_web::HttpRequest, Resolved(service): Resolved<Scoped>| {
                        assert!(Shared::ptr_eq(
                            request.extensions().get::<Shared<Scoped>>().unwrap(),
                            &service
                        ));
                        *captured.lock().unwrap() =
                            request.extensions().get::<RequestScope>().cloned();
                        async { HttpResponse::Ok().finish() }
                    },
                ),
            ),
    )
    .await;
    let response = test::call_service(&app, test::TestRequest::get().to_request()).await;
    assert!(
        response
            .request()
            .extensions()
            .get::<RequestScope>()
            .is_none()
    );
    let scope = saved.lock().unwrap().take().unwrap();
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

#[cfg(feature = "async-factory")]
#[actix_web::test]
async fn asynchronous_provider_is_shared_by_concurrent_request_extractors() {
    let injector = Shared::new(Injector::root());
    injector.provide(Provider::scoped_async(|_| async {
        tokio::task::yield_now().await;
        Shared::new(Scoped(42))
    }));
    let app = test::init_service(App::new()
        .app_data(injector_data(injector))
        .wrap(middleware::from_fn(request_scope))
        .route("/", web::get().to(|Resolved(first): Resolved<Scoped>, Resolved(second): Resolved<Scoped>| async move {
            assert!(Shared::ptr_eq(&first, &second));
            first.0.to_string()
        }))).await;
    let response = test::call_service(&app, test::TestRequest::get().to_request()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(test::read_body(response).await.as_ref(), b"42");
}
