use std::{io, sync::Mutex};

use axum::{Router, routing::get};
use fluxdi::{
    Application, Error, Injector, Module, Provider, Shared, module::ModuleLifecycleFuture,
};
use rand::RngExt;
use tokio::{net::TcpListener, task::JoinHandle};

const MIN_PORT: u16 = 30000;
const MAX_PORT: u16 = 65535;
const BIND_ATTEMPTS: usize = 128;

#[derive(Default)]
struct ServerRuntime {
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

struct DualHttpModule;

impl Module for DualHttpModule {
    fn configure(&self, injector: &Injector) -> Result<(), Error> {
        injector
            .provide::<ServerRuntime>(Provider::root(|_| Shared::new(ServerRuntime::default())));
        Ok(())
    }

    fn on_start(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
        Box::pin(async move {
            let runtime = injector.resolve::<ServerRuntime>();

            let ((listener_a, port_a), (listener_b, port_b)) = tokio::try_join!(
                bind_random_listener(),
                bind_random_listener()
            )
            .map_err(|err| {
                Error::module_lifecycle_failed("DualHttpModule", "on_start", &err.to_string())
            })?;

            let app_a = Router::new()
                .route("/", get(|| async { "service-a" }))
                .route("/health", get(|| async { "ok" }));
            let app_b = Router::new()
                .route("/", get(|| async { "service-b" }))
                .route("/health", get(|| async { "ok" }));

            let task_a = tokio::spawn(async move {
                if let Err(err) = axum::serve(listener_a, app_a).await {
                    eprintln!("service-a exited: {err}");
                }
            });

            let task_b = tokio::spawn(async move {
                if let Err(err) = axum::serve(listener_b, app_b).await {
                    eprintln!("service-b exited: {err}");
                }
            });

            let mut tasks = runtime.tasks.lock().map_err(|err| {
                Error::module_lifecycle_failed("DualHttpModule", "on_start", &err.to_string())
            })?;
            tasks.push(task_a);
            tasks.push(task_b);

            println!("service-a listening on http://127.0.0.1:{port_a}");
            println!("service-b listening on http://127.0.0.1:{port_b}");
            println!("press Ctrl+C to shutdown");

            Ok(())
        })
    }

    fn on_stop(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
        Box::pin(async move {
            let runtime = injector.resolve::<ServerRuntime>();
            let mut tasks = runtime.tasks.lock().map_err(|err| {
                Error::module_lifecycle_failed("DualHttpModule", "on_stop", &err.to_string())
            })?;

            for task in tasks.drain(..) {
                task.abort();
            }

            Ok(())
        })
    }
}

async fn bind_random_listener() -> io::Result<(TcpListener, u16)> {
    for _ in 0..BIND_ATTEMPTS {
        let port = rand::rng().random_range(MIN_PORT..=MAX_PORT);
        match TcpListener::bind(("127.0.0.1", port)).await {
            Ok(listener) => return Ok((listener, port)),
            Err(err) if err.kind() == io::ErrorKind::AddrInUse => continue,
            Err(err) => return Err(err),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AddrNotAvailable,
        format!("unable to bind random port in range {MIN_PORT}-{MAX_PORT}"),
    ))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    fluxdi::init_logging();

    let mut app = Application::new(DualHttpModule);
    app.bootstrap().await?;

    tokio::signal::ctrl_c().await.map_err(|err| {
        Error::module_lifecycle_failed("DualHttpModule", "main", &err.to_string())
    })?;

    app.shutdown().await
}
