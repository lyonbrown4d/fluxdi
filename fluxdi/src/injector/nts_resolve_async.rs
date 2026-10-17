use super::*;

impl Injector {
    #[cfg(feature = "async-factory")]
    pub async fn try_resolve_async<T>(&self) -> Result<Shared<T>, Error>
    where
        T: ?Sized + 'static,
    {
        let operation = async {
            let provider = self
                .resolve_provider::<T>()
                .inspect_err(|_| self.record_cache_miss())?;
            self.resolve_binding_async(provider).await
        };
        #[cfg(feature = "tracing")]
        let operation = {
            use tracing::Instrument;
            operation.instrument(info_span!(
                SPAN_RESOLVE,
                type_name = std::any::type_name::<T>(),
                op = "resolve"
            ))
        };
        self.track_resolve_async(operation).await
    }

    #[cfg(feature = "async-factory")]
    pub async fn try_resolve_named_async<T>(&self, name: &str) -> Result<Shared<T>, Error>
    where
        T: ?Sized + 'static,
    {
        let operation = async {
            let provider = self
                .resolve_provider_named::<T>(name)
                .inspect_err(|_| self.record_cache_miss())?;
            self.resolve_binding_async(provider).await
        };
        #[cfg(feature = "tracing")]
        let operation = {
            use tracing::Instrument;
            operation.instrument(info_span!(SPAN_RESOLVE, type_name = std::any::type_name::<T>(), op = "resolve_named", name = %name))
        };
        self.track_resolve_async(operation).await
    }

    #[cfg(feature = "async-factory")]
    pub async fn try_resolve_all_async<T>(&self) -> Result<Vec<Shared<T>>, Error>
    where
        T: ?Sized + 'static,
    {
        let operation = async {
            let provider = self
                .resolve_set_providers::<T>()
                .inspect_err(|_| self.record_cache_miss())?;
            let mut values = Vec::with_capacity(provider.len());
            for item in provider {
                values.push(self.resolve_binding_async(item).await?);
            }
            Ok(values)
        };
        #[cfg(feature = "tracing")]
        let operation = {
            use tracing::Instrument;
            operation.instrument(info_span!(
                SPAN_RESOLVE,
                type_name = std::any::type_name::<T>(),
                op = "resolve_all"
            ))
        };
        self.track_resolve_async(operation).await
    }

    #[cfg(feature = "async-factory")]
    pub async fn resolve_async<T>(&self) -> Shared<T>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_async::<T>().await.unwrap()
    }

    #[cfg(feature = "async-factory")]
    pub async fn optional_resolve_async<T>(&self) -> Option<Shared<T>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_async::<T>().await.ok()
    }

    #[cfg(feature = "async-factory")]
    pub async fn resolve_named_async<T>(&self, name: &str) -> Shared<T>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_named_async::<T>(name).await.unwrap()
    }

    #[cfg(feature = "async-factory")]
    pub async fn optional_resolve_named_async<T>(&self, name: &str) -> Option<Shared<T>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_named_async::<T>(name).await.ok()
    }

    #[cfg(feature = "async-factory")]
    pub async fn resolve_all_async<T>(&self) -> Vec<Shared<T>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_all_async::<T>().await.unwrap()
    }

    #[cfg(feature = "async-factory")]
    pub async fn optional_resolve_all_async<T>(&self) -> Option<Vec<Shared<T>>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_all_async::<T>().await.ok()
    }
}
