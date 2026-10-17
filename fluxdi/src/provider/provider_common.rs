use super::*;

impl<T: ?Sized + 'static> Provider<T> {
    pub(crate) fn create(&self, injector: &Injector) -> Result<Instance<T>, Error> {
        match &self.try_factory {
            Some(factory) => factory(injector),
            None => Ok((self.factory)(injector)),
        }
    }

    #[cfg(feature = "async-factory")]
    pub(crate) async fn create_async(&self, injector: Injector) -> Result<Instance<T>, Error> {
        if let Some(factory) = &self.try_async_factory {
            factory(injector).await
        } else if let Some(factory) = &self.async_factory {
            Ok(factory(injector).await)
        } else {
            self.create(&injector)
        }
    }

    /// Applies resource limits to this provider.
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self.limiter = Limiter::from_limits(limits);
        self
    }

    pub(crate) fn acquire_creation_permit(
        &self,
        type_name: &str,
    ) -> Result<Option<CreationPermit>, Error> {
        if let Some(limiter) = &self.limiter {
            return Limiter::try_acquire(limiter, type_name).map(Some);
        }

        Ok(None)
    }

    #[cfg(feature = "async-factory")]
    pub(crate) async fn acquire_creation_permit_async(
        &self,
        type_name: &str,
    ) -> Result<Option<CreationPermit>, Error> {
        if let Some(limiter) = &self.limiter {
            #[cfg(all(feature = "thread-safe", feature = "resource-limit-async"))]
            {
                return Limiter::try_acquire_async(limiter, type_name)
                    .await
                    .map(Some);
            }

            #[cfg(not(all(feature = "thread-safe", feature = "resource-limit-async")))]
            {
                return Limiter::try_acquire(limiter, type_name).map(Some);
            }
        }

        Ok(None)
    }

    /// Declares that this provider depends on a single service `D`.
    ///
    /// This hint is used by graph tooling (`dependency_graph`, `validate_graph`)
    /// and does not change runtime resolution behavior.
    pub fn with_dependency<D>(mut self) -> Self
    where
        D: ?Sized + 'static,
    {
        self.dependency_hints.push(DependencyHint::one::<D>());
        self
    }

    /// Declares that this provider depends on a named service `D`.
    ///
    /// This hint is used by graph tooling (`dependency_graph`, `validate_graph`)
    /// and does not change runtime resolution behavior.
    pub fn with_named_dependency<D>(mut self, name: impl Into<String>) -> Self
    where
        D: ?Sized + 'static,
    {
        self.dependency_hints
            .push(DependencyHint::named::<D>(name.into()));
        self
    }

    /// Declares that this provider depends on all set bindings of service `D`.
    ///
    /// This hint is used by graph tooling (`dependency_graph`, `validate_graph`)
    /// and does not change runtime resolution behavior.
    pub fn with_set_dependency<D>(mut self) -> Self
    where
        D: ?Sized + 'static,
    {
        self.dependency_hints.push(DependencyHint::all::<D>());
        self
    }

    /// Wraps resolved instances with a decorator (e.g. logging, caching).
    ///
    /// The decorator receives the base instance and returns a wrapped instance.
    /// Order is deterministic: base factory runs first, then decorator.
    ///
    /// For multiple decorators, chain: `provider.with_decorator(d1).with_decorator(d2)`.
    #[cfg(not(feature = "thread-safe"))]
    pub fn with_decorator<F>(self, decorator: F) -> Self
    where
        F: Fn(Shared<T>) -> Shared<T> + 'static,
    {
        let Self {
            scope,
            factory,
            try_factory,
            #[cfg(feature = "async-factory")]
            try_async_factory,
            #[cfg(feature = "async-factory")]
            async_factory,
            limits,
            dependency_hints,
            limiter,
        } = self;
        let decorator = Shared::new(decorator);
        let try_factory = try_factory.map(|factory| {
            let decorator = decorator.clone();
            Box::new(move |injector: &Injector| {
                factory(injector).map(|instance| Instance::new(decorator(instance.value())))
            }) as TryFactory<T>
        });
        #[cfg(feature = "async-factory")]
        let try_async_factory = try_async_factory.map(|factory| {
            let decorator = decorator.clone();
            Box::new(move |injector| {
                let future = factory(injector);
                let decorator = decorator.clone();
                Box::pin(async move {
                    future
                        .await
                        .map(|instance| Instance::new(decorator(instance.value())))
                }) as Pin<Box<dyn Future<Output = Result<Instance<T>, Error>>>>
            }) as TryAsyncFactory<T>
        });
        #[cfg(feature = "async-factory")]
        let async_factory = async_factory.map(|factory| {
            let decorator = decorator.clone();
            Box::new(move |injector| {
                let future = factory(injector);
                let decorator = decorator.clone();
                Box::pin(async move { Instance::new(decorator(future.await.value())) })
                    as Pin<Box<dyn Future<Output = Instance<T>>>>
            }) as AsyncFactory<T>
        });
        Self {
            scope,
            factory: Box::new(move |inj| {
                let instance = factory(inj);
                Instance::new(decorator(instance.value()))
            }),
            try_factory,
            #[cfg(feature = "async-factory")]
            try_async_factory,
            #[cfg(feature = "async-factory")]
            async_factory,
            limits,
            dependency_hints,
            limiter,
        }
    }

    /// Wraps resolved instances with a decorator (thread-safe).
    #[cfg(feature = "thread-safe")]
    pub fn with_decorator<F>(self, decorator: F) -> Self
    where
        F: Fn(Shared<T>) -> Shared<T> + Send + Sync + 'static,
    {
        let Self {
            scope,
            factory,
            try_factory,
            #[cfg(feature = "async-factory")]
            try_async_factory,
            #[cfg(feature = "async-factory")]
            async_factory,
            limits,
            dependency_hints,
            limiter,
        } = self;
        let decorator = Shared::new(decorator);
        let try_factory = try_factory.map(|factory| {
            let decorator = decorator.clone();
            Box::new(move |injector: &Injector| {
                factory(injector).map(|instance| Instance::new(decorator(instance.value())))
            }) as TryFactory<T>
        });
        #[cfg(feature = "async-factory")]
        let try_async_factory = try_async_factory.map(|factory| {
            let decorator = decorator.clone();
            Box::new(move |injector| {
                let future = factory(injector);
                let decorator = decorator.clone();
                Box::pin(async move {
                    future
                        .await
                        .map(|instance| Instance::new(decorator(instance.value())))
                })
                    as Pin<Box<dyn Future<Output = Result<Instance<T>, Error>> + Send>>
            }) as TryAsyncFactory<T>
        });
        #[cfg(feature = "async-factory")]
        let async_factory = async_factory.map(|factory| {
            let decorator = decorator.clone();
            Box::new(move |injector| {
                let future = factory(injector);
                let decorator = decorator.clone();
                Box::pin(async move { Instance::new(decorator(future.await.value())) })
                    as Pin<Box<dyn Future<Output = Instance<T>> + Send>>
            }) as AsyncFactory<T>
        });
        Self {
            scope,
            factory: Box::new(move |inj| {
                let instance = factory(inj);
                Instance::new(decorator(instance.value()))
            }),
            try_factory,
            #[cfg(feature = "async-factory")]
            try_async_factory,
            #[cfg(feature = "async-factory")]
            async_factory,
            limits,
            dependency_hints,
            limiter,
        }
    }
}
