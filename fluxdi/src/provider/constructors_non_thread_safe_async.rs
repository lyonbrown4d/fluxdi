use super::*;

#[cfg(all(not(feature = "thread-safe"), feature = "async-factory"))]
impl<T: ?Sized + 'static> Provider<T> {
    /// Creates a singleton provider whose factory resolves asynchronously.
    #[cfg(feature = "async-factory")]
    pub fn singleton_async<F, Fut>(factory: F) -> Provider<T>
    where
        F: Fn(Injector) -> Fut + 'static,
        Fut: Future<Output = Shared<T>> + 'static,
    {
        #[cfg(feature = "tracing")]
        info!(
            type_name = std::any::type_name::<T>(),
            scope = %Scope::Module,
            threading = "single-threaded",
            factory_mode = "async",
            "Creating async singleton provider"
        );

        Provider::<T> {
            scope: Scope::Module,
            factory: Box::new(|_| {
                panic!(
                    "async provider cannot be used with try_resolve/resolve; use try_resolve_async/resolve_async"
                )
            }),
            async_factory: Some(Box::new(move |injector| {
                Box::pin({
                    let future = factory(injector);
                    async move {
                        #[cfg(feature = "tracing")]
                        debug!(
                            type_name = std::any::type_name::<T>(),
                            scope = %Scope::Module,
                            op = "provider_factory_call_async",
                            "Executing async singleton factory"
                        );
                        Instance::new(future.await)
                    }
                })
            })),
            try_factory: None,
            #[cfg(feature = "async-factory")]
            try_async_factory: None,
            limits: Limits::default(),
            dependency_hints: Vec::new(),
            limiter: None,
        }
    }

    /// Creates a transient provider whose factory resolves asynchronously.
    #[cfg(feature = "async-factory")]
    pub fn transient_async<F, Fut>(factory: F) -> Provider<T>
    where
        F: Fn(Injector) -> Fut + 'static,
        Fut: Future<Output = Shared<T>> + 'static,
    {
        #[cfg(feature = "tracing")]
        info!(
            type_name = std::any::type_name::<T>(),
            scope = %Scope::Transient,
            threading = "single-threaded",
            factory_mode = "async",
            "Creating async transient provider"
        );

        Provider::<T> {
            scope: Scope::Transient,
            factory: Box::new(|_| {
                panic!(
                    "async provider cannot be used with try_resolve/resolve; use try_resolve_async/resolve_async"
                )
            }),
            async_factory: Some(Box::new(move |injector| {
                Box::pin({
                    let future = factory(injector);
                    async move {
                        #[cfg(feature = "tracing")]
                        debug!(
                            type_name = std::any::type_name::<T>(),
                            scope = %Scope::Transient,
                            op = "provider_factory_call_async",
                            "Executing async transient factory"
                        );
                        Instance::new(future.await)
                    }
                })
            })),
            try_factory: None,
            #[cfg(feature = "async-factory")]
            try_async_factory: None,
            limits: Limits::default(),
            dependency_hints: Vec::new(),
            limiter: None,
        }
    }

    /// Creates a root-scoped provider whose factory resolves asynchronously.
    #[cfg(feature = "async-factory")]
    pub fn root_async<F, Fut>(factory: F) -> Provider<T>
    where
        F: Fn(Injector) -> Fut + 'static,
        Fut: Future<Output = Shared<T>> + 'static,
    {
        #[cfg(feature = "tracing")]
        info!(
            type_name = std::any::type_name::<T>(),
            scope = %Scope::Root,
            threading = "single-threaded",
            factory_mode = "async",
            "Creating async root provider"
        );

        Provider::<T> {
            scope: Scope::Root,
            factory: Box::new(|_| {
                panic!(
                    "async provider cannot be used with try_resolve/resolve; use try_resolve_async/resolve_async"
                )
            }),
            async_factory: Some(Box::new(move |injector| {
                Box::pin({
                    let future = factory(injector);
                    async move {
                        #[cfg(feature = "tracing")]
                        debug!(
                            type_name = std::any::type_name::<T>(),
                            scope = %Scope::Root,
                            op = "provider_factory_call_async",
                            "Executing async root factory"
                        );
                        Instance::new(future.await)
                    }
                })
            })),
            try_factory: None,
            #[cfg(feature = "async-factory")]
            try_async_factory: None,
            limits: Limits::default(),
            dependency_hints: Vec::new(),
            limiter: None,
        }
    }

    /// Creates a scope-scoped provider whose factory resolves asynchronously.
    #[cfg(feature = "async-factory")]
    pub fn scoped_async<F, Fut>(factory: F) -> Provider<T>
    where
        F: Fn(Injector) -> Fut + 'static,
        Fut: Future<Output = Shared<T>> + 'static,
    {
        #[cfg(feature = "tracing")]
        info!(
            type_name = std::any::type_name::<T>(),
            scope = %Scope::Scoped,
            threading = "single-threaded",
            factory_mode = "async",
            "Creating async scoped provider"
        );

        Provider::<T> {
            scope: Scope::Scoped,
            factory: Box::new(|_| {
                panic!(
                    "async provider cannot be used with try_resolve/resolve; use try_resolve_async/resolve_async"
                )
            }),
            async_factory: Some(Box::new(move |injector| {
                Box::pin({
                    let future = factory(injector);
                    async move {
                        #[cfg(feature = "tracing")]
                        debug!(
                            type_name = std::any::type_name::<T>(),
                            scope = %Scope::Scoped,
                            op = "provider_factory_call_async",
                            "Executing async scoped factory"
                        );
                        Instance::new(future.await)
                    }
                })
            })),
            try_factory: None,
            #[cfg(feature = "async-factory")]
            try_async_factory: None,
            limits: Limits::default(),
            dependency_hints: Vec::new(),
            limiter: None,
        }
    }
}
