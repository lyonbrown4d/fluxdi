//! Axum integration for FluxDI.
//!
//! State, optional request-scope middleware, and dependency extraction from an
//! `Injector`.

use std::ops::Deref;

use ::axum::{
    extract::{FromRef, FromRequestParts, Request, State},
    http::{StatusCode, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};

#[cfg(feature = "async-factory")]
use crate::ErrorKind;
use crate::{Error, Injector, runtime::Shared};

/// Axum state wrapper that stores a shared injector reference.
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
/// Installed by [`request_scope`], and accessible through `Extension<RequestScope>`.
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
/// Install with `axum::middleware::from_fn_with_state(state, request_scope)`.
/// Nested installations reuse the existing request scope. The middleware retains
/// the scope until the downstream response is produced, or processing is cancelled.
/// A streaming response must capture a scope or service handle itself if it needs
/// that dependency to remain alive while its body is consumed.
pub async fn request_scope(
    State(state): State<InjectorState>,
    mut request: Request,
    next: Next,
) -> Response {
    let scope = request
        .extensions()
        .get::<RequestScope>()
        .cloned()
        .unwrap_or_else(|| RequestScope {
            injector: Shared::new(state.injector.create_scope()),
        });
    request.extensions_mut().insert(scope.clone());
    let response = next.run(request).await;
    drop(scope);
    response
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

impl IntoResponse for ResolveRejection {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}

/// Resolves `T` from the request scope, or from `InjectorState` without middleware.
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

impl<S, T> FromRequestParts<S> for Resolved<T>
where
    S: Send + Sync,
    InjectorState: FromRef<S>,
    T: ?Sized + Send + Sync + 'static,
{
    type Rejection = ResolveRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let injector = parts
            .extensions
            .get::<RequestScope>()
            .map(RequestScope::injector)
            .unwrap_or_else(|| InjectorState::from_ref(state).injector());
        #[cfg(feature = "async-factory")]
        {
            match injector.try_resolve::<T>() {
                Ok(resolved) => Ok(Self(resolved)),
                Err(error) if error.kind == ErrorKind::AsyncFactoryRequiresAsyncResolve => {
                    let resolved = injector.try_resolve_async::<T>().await?;
                    Ok(Self(resolved))
                }
                Err(error) => Err(error.into()),
            }
        }

        #[cfg(not(feature = "async-factory"))]
        {
            let resolved = injector.try_resolve::<T>()?;
            Ok(Self(resolved))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use futures::executor::block_on;

    use crate::Provider;

    #[test]
    fn resolved_extractor_supports_sync_provider() {
        let injector = Shared::new(Injector::root());
        injector.provide::<u32>(Provider::root(|_| Shared::new(42)));
        let state = InjectorState::new(injector);

        let (mut parts, _) = Request::builder().uri("/").body(()).unwrap().into_parts();
        let resolved = block_on(Resolved::<u32>::from_request_parts(&mut parts, &state)).unwrap();
        assert_eq!(*resolved, 42);
    }

    #[cfg(feature = "async-factory")]
    #[test]
    fn resolved_extractor_supports_async_provider() {
        let injector = Shared::new(Injector::root());
        injector.provide::<u32>(Provider::root_async(|_| async { Shared::new(7) }));
        let state = InjectorState::new(injector);

        let (mut parts, _) = Request::builder().uri("/").body(()).unwrap().into_parts();
        let resolved = block_on(Resolved::<u32>::from_request_parts(&mut parts, &state)).unwrap();
        assert_eq!(*resolved, 7);
    }
}
