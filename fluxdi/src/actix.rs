//! Actix-web integration for FluxDI.
//!
//! State, service configuration, optional request-scope middleware, and
//! dependency extraction from an `Injector`.

use std::ops::Deref;

#[cfg(not(feature = "async-factory"))]
use std::future::{Ready, ready};

#[cfg(feature = "async-factory")]
use futures::future::LocalBoxFuture;

use ::actix_web::{
    Error as ActixError, FromRequest, HttpMessage, HttpRequest, HttpResponse, ResponseError,
    body::MessageBody,
    dev::{Payload, ServiceRequest, ServiceResponse},
    http::StatusCode,
    middleware::Next,
    web::{self, ServiceConfig},
};

#[cfg(feature = "async-factory")]
use crate::ErrorKind;
use crate::{Error, Injector, runtime::Shared};

/// Actix state wrapper that stores a shared injector reference.
#[derive(Clone)]
pub struct InjectorState {
    injector: Shared<Injector>,
}

impl InjectorState {
    /// Builds state from a shared injector.
    pub fn new(injector: Shared<Injector>) -> Self {
        Self { injector }
    }

    /// Returns a cloned shared injector handle.
    pub fn injector(&self) -> Shared<Injector> {
        self.injector.clone()
    }
}

/// Injector context shared by all dependency resolutions in one HTTP request.
///
/// Installed by [`request_scope`], and available from `HttpMessage::extensions`.
/// Cloning this handle keeps its scope alive beyond request processing.
#[derive(Clone)]
pub struct RequestScope {
    injector: Shared<Injector>,
}

impl RequestScope {
    /// Returns a cloned handle for request-local resolutions and registrations.
    pub fn injector(&self) -> Shared<Injector> {
        self.injector.clone()
    }
}

/// Establishes one dependency scope for the downstream request pipeline.
///
/// Install with `actix_web::middleware::from_fn(request_scope)` and configure
/// [`injector_data`]. Nested installations reuse the existing request scope.
/// The scope extension is removed from the produced response's request. Errors
/// and cancellation release it when the request is dropped. Explicit request,
/// scope, or service clones may extend its lifetime. Streaming bodies must
/// capture a scope or service handle if they require it while being consumed.
pub async fn request_scope<B: MessageBody>(
    request: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<B>, ActixError> {
    let owns_scope = !request.extensions().contains::<RequestScope>();
    let scope = if owns_scope {
        let state = request
            .app_data::<web::Data<InjectorState>>()
            .ok_or_else(missing_state)?;
        let scope = RequestScope {
            injector: Shared::new(state.injector.create_scope()),
        };
        request.extensions_mut().insert(scope.clone());
        Some(scope)
    } else {
        None
    };
    // Do not clone HttpRequest before calling the router: Actix requires its
    // request allocation to remain unique while mutating routing information.
    let result = next.call(request).await;
    if owns_scope && let Ok(response) = &result {
        response.request().extensions_mut().remove::<RequestScope>();
    }
    drop(scope);
    result
}

fn missing_state() -> ResolveRejection {
    ResolveRejection::internal("InjectorState is not configured in app_data".to_string())
}

/// Builds typed app data for `actix_web::App::app_data(...)`.
pub fn injector_data(injector: Shared<Injector>) -> web::Data<InjectorState> {
    web::Data::new(InjectorState::new(injector))
}

/// Extension trait that wires injector state into `ServiceConfig`.
pub trait ServiceConfigExt {
    fn with_injector(&mut self, injector: Shared<Injector>) -> &mut Self;
}

impl ServiceConfigExt for ServiceConfig {
    fn with_injector(&mut self, injector: Shared<Injector>) -> &mut Self {
        self.app_data(injector_data(injector))
    }
}

/// Rejection type used by [`Resolved`].
#[derive(Debug)]
pub struct ResolveRejection {
    pub status: StatusCode,
    pub message: String,
}

impl ResolveRejection {
    fn internal(message: String) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message,
        }
    }
}

impl From<Error> for ResolveRejection {
    fn from(value: Error) -> Self {
        Self::internal(value.to_string())
    }
}

impl std::fmt::Display for ResolveRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl ResponseError for ResolveRejection {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status).body(self.message.clone())
    }
}

/// Resolves `T` from the request scope, or from [`InjectorState`] without middleware.
pub struct Resolved<T: ?Sized + Send + Sync + 'static>(pub Shared<T>);

impl<T: ?Sized + Send + Sync + 'static> Resolved<T> {
    pub fn into_shared(self) -> Shared<T> {
        self.0
    }
}

impl<T: ?Sized + Send + Sync + 'static> Deref for Resolved<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl<T: ?Sized + Send + Sync + 'static> From<Resolved<T>> for Shared<T> {
    fn from(value: Resolved<T>) -> Self {
        value.0
    }
}

impl<T> FromRequest for Resolved<T>
where
    T: ?Sized + Send + Sync + 'static,
{
    type Error = ActixError;
    #[cfg(feature = "async-factory")]
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;
    #[cfg(not(feature = "async-factory"))]
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        // Clone before returning the future: never hold an extensions RefCell
        // borrow across asynchronous resolution or concurrent extractors.
        let injector = req
            .extensions()
            .get::<RequestScope>()
            .map(RequestScope::injector)
            .or_else(|| {
                req.app_data::<web::Data<InjectorState>>()
                    .map(|state| state.injector())
            })
            .ok_or_else(missing_state);

        #[cfg(feature = "async-factory")]
        {
            Box::pin(async move {
                let injector = injector?;
                let resolved = match injector.try_resolve::<T>() {
                    Ok(resolved) => resolved,
                    Err(error) if error.kind == ErrorKind::AsyncFactoryRequiresAsyncResolve => {
                        injector
                            .try_resolve_async::<T>()
                            .await
                            .map_err(ResolveRejection::from)?
                    }
                    Err(error) => return Err(ResolveRejection::from(error).into()),
                };
                Ok(Self(resolved))
            })
        }
        #[cfg(not(feature = "async-factory"))]
        {
            ready(
                injector
                    .and_then(|injector| {
                        injector
                            .try_resolve::<T>()
                            .map(Self)
                            .map_err(ResolveRejection::from)
                    })
                    .map_err(ActixError::from),
            )
        }
    }
}
