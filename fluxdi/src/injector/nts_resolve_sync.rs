use super::*;

impl Injector {
    pub fn try_resolve<T>(&self) -> Result<Shared<T>, Error>
    where
        T: ?Sized + 'static,
    {
        #[cfg(feature = "tracing")]
        let _span = info_span!(
            SPAN_RESOLVE,
            type_name = std::any::type_name::<T>(),
            op = "resolve"
        )
        .entered();
        self.track_resolve(|| {
            let provider = self
                .resolve_provider::<T>()
                .inspect_err(|_| self.record_cache_miss())?;
            self.resolve_binding(provider)
        })
    }

    pub fn try_resolve_named<T>(&self, name: &str) -> Result<Shared<T>, Error>
    where
        T: ?Sized + 'static,
    {
        #[cfg(feature = "tracing")]
        let _span = info_span!(SPAN_RESOLVE, type_name = std::any::type_name::<T>(), op = "resolve_named", name = %name).entered();
        self.track_resolve(|| {
            let provider = self
                .resolve_provider_named::<T>(name)
                .inspect_err(|_| self.record_cache_miss())?;
            self.resolve_binding(provider)
        })
    }

    pub fn try_resolve_all<T>(&self) -> Result<Vec<Shared<T>>, Error>
    where
        T: ?Sized + 'static,
    {
        #[cfg(feature = "tracing")]
        let _span = info_span!(
            SPAN_RESOLVE,
            type_name = std::any::type_name::<T>(),
            op = "resolve_all"
        )
        .entered();
        self.track_resolve(|| {
            let provider = self
                .resolve_set_providers::<T>()
                .inspect_err(|_| self.record_cache_miss())?;
            let mut values = Vec::with_capacity(provider.len());
            for item in provider {
                values.push(self.resolve_binding(item)?);
            }
            Ok(values)
        })
    }

    pub fn resolve<T>(&self) -> Shared<T>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve::<T>().unwrap()
    }

    pub fn optional_resolve<T>(&self) -> Option<Shared<T>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve::<T>().ok()
    }

    pub fn resolve_named<T>(&self, name: &str) -> Shared<T>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_named::<T>(name).unwrap()
    }

    pub fn optional_resolve_named<T>(&self, name: &str) -> Option<Shared<T>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_named::<T>(name).ok()
    }

    pub fn resolve_all<T>(&self) -> Vec<Shared<T>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_all::<T>().unwrap()
    }

    pub fn optional_resolve_all<T>(&self) -> Option<Vec<Shared<T>>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_all::<T>().ok()
    }
}
