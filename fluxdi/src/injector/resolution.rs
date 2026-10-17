use super::*;
use crate::runtime::Service;

impl Injector {
    /// Resolves an explicit typed provider in this calling context.
    ///
    /// This uses the same scope caches, fallible initialization, decorators and
    /// resource limits as registry resolution, without looking up a binding by
    /// type or name. Keep the shared provider handle to preserve cache identity.
    /// The provider is not registered in this injector's dynamic registry or
    /// dependency graph. Generated static modules use this entry point.
    pub fn try_resolve_provider<T: ?Sized + Service>(
        &self,
        provider: &Shared<Provider<T>>,
    ) -> Result<Shared<T>, Error> {
        self.track_resolve(|| self.resolve_binding(provider.clone()))
    }

    /// Resolves an explicit provider asynchronously in this calling context.
    ///
    /// Like [`Self::try_resolve_provider`], this does not register the provider.
    /// Cancellation and failed initialization release the initialization owner
    /// so a later caller can retry.
    #[cfg(feature = "async-factory")]
    pub async fn try_resolve_provider_async<T: ?Sized + Service>(
        &self,
        provider: &Shared<Provider<T>>,
    ) -> Result<Shared<T>, Error> {
        self.track_resolve_async(self.resolve_binding_async(provider.clone()))
            .await
    }

    fn resolution_key<T: ?Sized + Service>(&self, provider: &Shared<Provider<T>>) -> ResolutionKey {
        let context = self
            .cache_target_for_scope(provider.scope)
            .unwrap_or_else(|| self.clone());
        ResolutionKey {
            type_id: TypeId::of::<T>(),
            context: Shared::as_ptr(&context.inner) as usize,
            provider: Shared::as_ptr(provider) as usize,
            cached: provider.scope != Scope::Transient,
        }
    }

    pub(super) fn record_cache_miss(&self) {
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_resolve_cache_miss();
    }

    fn record_cache_hit(&self) {
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_resolve_cache_hit();
    }

    pub(super) fn resolve_binding<T: ?Sized + Service>(
        &self,
        provider: Shared<Provider<T>>,
    ) -> Result<Shared<T>, Error> {
        let _guard = ResolveGuard::push_binding(self.resolution_key(&provider))?;
        let cell = self
            .cache_target_for_scope(provider.scope)
            .map(|target| target.initialization(&provider));
        if let Some(value) = cell.as_ref().and_then(|cell| cell.get()) {
            #[cfg(feature = "tracing")]
            trace!(type_name = std::any::type_name::<T>(), scope = %provider.scope, cache = "hit", "Resolved binding from its context cache");
            self.record_cache_hit();
            return Ok(value.value());
        }
        #[cfg(feature = "tracing")]
        trace!(type_name = std::any::type_name::<T>(), scope = %provider.scope, cache = "miss", "Resolving binding in its calling context");
        self.record_cache_miss();
        #[cfg(feature = "async-factory")]
        if provider.async_factory.is_some() {
            return Err(Error::async_factory_requires_async_resolve(
                std::any::type_name::<T>(),
            ));
        }
        let instance = match cell {
            Some(cell) => cell.get_or_try_init(|| self.execute_factory(&provider))?,
            None => self.execute_factory(&provider)?,
        };
        Ok(instance.value())
    }

    #[cfg(feature = "async-factory")]
    pub(super) async fn resolve_binding_async<T: ?Sized + Service>(
        &self,
        provider: Shared<Provider<T>>,
    ) -> Result<Shared<T>, Error> {
        crate::resolve_guard::guard_async(self.resolution_key(&provider), async {
            let cell = self
                .cache_target_for_scope(provider.scope)
                .map(|target| target.initialization(&provider));
            if let Some(value) = cell.as_ref().and_then(|cell| cell.get()) {
                #[cfg(feature = "tracing")]
                trace!(type_name = std::any::type_name::<T>(), scope = %provider.scope, cache = "hit", "Resolved binding from its context cache");
                self.record_cache_hit();
                return Ok(value.value());
            }
            #[cfg(feature = "tracing")]
            trace!(type_name = std::any::type_name::<T>(), scope = %provider.scope, cache = "miss", "Resolving binding in its calling context");
            self.record_cache_miss();
            let instance = match cell {
                Some(cell) => {
                    cell.get_or_try_init_async(|| self.execute_factory_async(&provider))
                        .await?
                }
                None => self.execute_factory_async(&provider).await?,
            };
            Ok(instance.value())
        })
        .await
    }

    fn execute_factory<T: ?Sized + Service>(
        &self,
        provider: &Provider<T>,
    ) -> Result<Shared<Instance<T>>, Error> {
        #[cfg(feature = "tracing")]
        let _span =
            info_span!(SPAN_FACTORY_EXECUTE, type_name = std::any::type_name::<T>()).entered();
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_factory_execution();
        let _permit = provider.acquire_creation_permit(std::any::type_name::<T>())?;
        provider.create(self).map(Shared::new)
    }

    #[cfg(feature = "async-factory")]
    async fn execute_factory_async<T: ?Sized + Service>(
        &self,
        provider: &Provider<T>,
    ) -> Result<Shared<Instance<T>>, Error> {
        let operation = async {
            #[cfg(feature = "metrics")]
            self.inner.metrics.record_factory_execution();
            let _permit = provider
                .acquire_creation_permit_async(std::any::type_name::<T>())
                .await?;
            let instance = provider.create_async(self.clone()).await?;
            Ok(Shared::new(instance))
        };
        #[cfg(feature = "tracing")]
        {
            use tracing::Instrument;
            operation
                .instrument(info_span!(
                    SPAN_FACTORY_EXECUTE,
                    type_name = std::any::type_name::<T>()
                ))
                .await
        }
        #[cfg(not(feature = "tracing"))]
        operation.await
    }

    pub(super) fn track_resolve<T>(
        &self,
        operation: impl FnOnce() -> Result<T, Error>,
    ) -> Result<T, Error> {
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_resolve_attempt();
        #[cfg(feature = "metrics")]
        let started = std::time::Instant::now();
        let result = operation();
        #[cfg(feature = "metrics")]
        match &result {
            Ok(_) => self.inner.metrics.record_resolve_success(started.elapsed()),
            Err(_) => self.inner.metrics.record_resolve_failure(started.elapsed()),
        }
        result
    }

    #[cfg(feature = "async-factory")]
    pub(super) async fn track_resolve_async<T>(
        &self,
        operation: impl std::future::Future<Output = Result<T, Error>>,
    ) -> Result<T, Error> {
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_resolve_attempt();
        #[cfg(feature = "metrics")]
        let started = std::time::Instant::now();
        let result = operation.await;
        #[cfg(feature = "metrics")]
        match &result {
            Ok(_) => self.inner.metrics.record_resolve_success(started.elapsed()),
            Err(_) => self.inner.metrics.record_resolve_failure(started.elapsed()),
        }
        result
    }
}
