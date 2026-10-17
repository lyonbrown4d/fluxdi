use std::sync::atomic::{AtomicUsize, Ordering};

use actix_web::{App, HttpResponse, HttpServer, web};
use fluxdi::{
    Injector, Provider, Shared,
    actix::{Resolved, injector_data, request_scope},
};

#[derive(Debug)]
struct GreetingService {
    message: String,
}

#[derive(Debug)]
struct RequestContext {
    request_scope_id: usize,
}

static NEXT_REQUEST_SCOPE_ID: AtomicUsize = AtomicUsize::new(1);

async fn hello(Resolved(service): Resolved<GreetingService>) -> HttpResponse {
    HttpResponse::Ok().body(service.message.clone())
}

async fn hello_scoped(
    Resolved(service): Resolved<GreetingService>,
    Resolved(context_a): Resolved<RequestContext>,
    Resolved(context_b): Resolved<RequestContext>,
) -> HttpResponse {
    HttpResponse::Ok().body(format!(
        "{} | request_scope_id={} | same_instance_within_scope={}",
        service.message,
        context_a.request_scope_id,
        Shared::ptr_eq(&context_a, &context_b)
    ))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    fluxdi::init_logging();

    let injector = Shared::new(Injector::root());
    injector.provide::<GreetingService>(Provider::root(|_| {
        Shared::new(GreetingService {
            message: "hello from fluxdi + actix".to_string(),
        })
    }));
    injector.provide::<RequestContext>(Provider::scoped(|_| {
        Shared::new(RequestContext {
            request_scope_id: NEXT_REQUEST_SCOPE_ID.fetch_add(1, Ordering::SeqCst),
        })
    }));

    HttpServer::new(move || {
        App::new()
            .app_data(injector_data(injector.clone()))
            .wrap(actix_web::middleware::from_fn(request_scope))
            .route("/hello", web::get().to(hello))
            .route("/hello/scoped", web::get().to(hello_scoped))
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}
