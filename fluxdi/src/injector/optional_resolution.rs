use super::*;
use crate::runtime::Service;

impl Injector {
    fn optional_provider<T: ?Sized + Service>(
        &self,
        name: Option<&str>,
    ) -> Result<Option<Shared<Provider<T>>>, Error> {
        // Inspect the binding itself: a missing dependency inside an existing
        // factory must remain an error rather than becoming None.
        let provider = match name {
            Some(name) => self.get_provider_named::<T>(name),
            None => self.get_provider::<T>(),
        };
        match provider {
            Some(provider) => provider
                .downcast::<Provider<T>>()
                .map(Some)
                .map_err(|_| Error::type_mismatch(std::any::type_name::<T>())),
            None => {
                self.record_cache_miss();
                Ok(None)
            }
        }
    }

    /// Returns `None` only when the binding is absent. Factory errors propagate.
    /// Unlike `optional_resolve`, this does not discard initialization errors.
    pub fn try_resolve_optional<T: ?Sized + Service>(&self) -> Result<Option<Shared<T>>, Error> {
        self.track_resolve(|| {
            self.optional_provider::<T>(None)?
                .map(|provider| self.resolve_binding(provider))
                .transpose()
        })
    }

    /// Resolves an optional named binding without discarding factory errors.
    pub fn try_resolve_optional_named<T: ?Sized + Service>(
        &self,
        name: &str,
    ) -> Result<Option<Shared<T>>, Error> {
        self.track_resolve(|| {
            self.optional_provider::<T>(Some(name))?
                .map(|provider| self.resolve_binding(provider))
                .transpose()
        })
    }

    /// Async counterpart of `try_resolve_optional`.
    #[cfg(feature = "async-factory")]
    pub async fn try_resolve_optional_async<T: ?Sized + Service>(
        &self,
    ) -> Result<Option<Shared<T>>, Error> {
        self.track_resolve_async(async {
            match self.optional_provider::<T>(None)? {
                Some(provider) => self.resolve_binding_async(provider).await.map(Some),
                None => Ok(None),
            }
        })
        .await
    }

    /// Async counterpart of `try_resolve_optional_named`.
    #[cfg(feature = "async-factory")]
    pub async fn try_resolve_optional_named_async<T: ?Sized + Service>(
        &self,
        name: &str,
    ) -> Result<Option<Shared<T>>, Error> {
        self.track_resolve_async(async {
            match self.optional_provider::<T>(Some(name))? {
                Some(provider) => self.resolve_binding_async(provider).await.map(Some),
                None => Ok(None),
            }
        })
        .await
    }
}
