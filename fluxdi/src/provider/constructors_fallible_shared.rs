use super::*;

impl<T: ?Sized + 'static> Provider<T> {
    /// Creates a root provider whose initialization can return an error.
    ///
    /// Only successful values are cached. An error is propagated unchanged and
    /// allows a later caller or waiting caller to retry initialization.
    ///
    /// ```
    /// use fluxdi::{Error, Injector, Provider, Shared};
    /// let injector = Injector::root();
    /// injector.provide::<u32>(Provider::try_root(|_| Ok(Shared::new(42))));
    /// assert_eq!(*injector.try_resolve::<u32>()?, 42);
    /// # Ok::<(), Error>(())
    /// ```
    pub fn try_root<F>(factory: F) -> Self
    where
        F: Fn(&Injector) -> Result<Shared<T>, Error> + Send + Sync + 'static,
    {
        Self::try_with_scope(Scope::Root, factory)
    }

    /// Creates a singleton provider whose initialization can return an error.
    pub fn try_singleton<F>(factory: F) -> Self
    where
        F: Fn(&Injector) -> Result<Shared<T>, Error> + Send + Sync + 'static,
    {
        Self::try_with_scope(Scope::Module, factory)
    }

    /// Creates a scoped provider whose initialization can return an error.
    pub fn try_scoped<F>(factory: F) -> Self
    where
        F: Fn(&Injector) -> Result<Shared<T>, Error> + Send + Sync + 'static,
    {
        Self::try_with_scope(Scope::Scoped, factory)
    }

    /// Creates a transient provider whose initialization can return an error.
    pub fn try_transient<F>(factory: F) -> Self
    where
        F: Fn(&Injector) -> Result<Shared<T>, Error> + Send + Sync + 'static,
    {
        Self::try_with_scope(Scope::Transient, factory)
    }

    fn try_with_scope<F>(scope: Scope, factory: F) -> Self
    where
        F: Fn(&Injector) -> Result<Shared<T>, Error> + Send + Sync + 'static,
    {
        let factory = Shared::new(factory);
        let legacy = factory.clone();
        let legacy = move |inj: &Injector| legacy(inj).unwrap_or_else(|error| panic!("{error}"));
        let mut provider = match scope {
            Scope::Root => Self::root(legacy),
            Scope::Module => Self::singleton(legacy),
            Scope::Scoped => Self::scoped(legacy),
            Scope::Transient => Self::transient(legacy),
        };
        provider.try_factory = Some(Box::new(move |inj| factory(inj).map(Instance::new)));
        provider
    }

    /// Creates an async root provider whose initialization can return an error.
    #[cfg(feature = "async-factory")]
    pub fn try_root_async<F, Fut>(factory: F) -> Self
    where
        F: Fn(Injector) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Shared<T>, Error>> + Send + 'static,
    {
        Self::try_with_scope_async(Scope::Root, factory)
    }

    /// Creates an async singleton provider whose initialization can return an error.
    #[cfg(feature = "async-factory")]
    pub fn try_singleton_async<F, Fut>(factory: F) -> Self
    where
        F: Fn(Injector) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Shared<T>, Error>> + Send + 'static,
    {
        Self::try_with_scope_async(Scope::Module, factory)
    }

    /// Creates an async scoped provider whose initialization can return an error.
    #[cfg(feature = "async-factory")]
    pub fn try_scoped_async<F, Fut>(factory: F) -> Self
    where
        F: Fn(Injector) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Shared<T>, Error>> + Send + 'static,
    {
        Self::try_with_scope_async(Scope::Scoped, factory)
    }

    /// Creates an async transient provider whose initialization can return an error.
    #[cfg(feature = "async-factory")]
    pub fn try_transient_async<F, Fut>(factory: F) -> Self
    where
        F: Fn(Injector) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Shared<T>, Error>> + Send + 'static,
    {
        Self::try_with_scope_async(Scope::Transient, factory)
    }

    #[cfg(feature = "async-factory")]
    fn try_with_scope_async<F, Fut>(scope: Scope, factory: F) -> Self
    where
        F: Fn(Injector) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Shared<T>, Error>> + Send + 'static,
    {
        let factory = Shared::new(factory);
        let legacy = factory.clone();
        let legacy = move |inj| {
            let future = legacy(inj);
            async move { future.await.unwrap_or_else(|error| panic!("{error}")) }
        };
        let mut provider = match scope {
            Scope::Root => Self::root_async(legacy),
            Scope::Module => Self::singleton_async(legacy),
            Scope::Scoped => Self::scoped_async(legacy),
            Scope::Transient => Self::transient_async(legacy),
        };
        provider.try_async_factory = Some(Box::new(move |inj| {
            let future = factory(inj);
            Box::pin(async move { future.await.map(Instance::new) })
        }));
        provider
    }
}
