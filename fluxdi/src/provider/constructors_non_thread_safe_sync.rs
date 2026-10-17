use super::*;

#[cfg(not(feature = "thread-safe"))]
impl<T: ?Sized + 'static> Provider<T> {
    /// Creates a singleton provider with calling-context scope (single-threaded).
    ///
    /// A singleton provider creates **one instance per calling injector context**.
    /// Once created, the same instance is returned on subsequent resolutions
    /// within that same context. Child injectors use their own caches.
    ///
    /// # Type Parameters
    ///
    /// - `F`: Factory function type that takes an [`Injector`] reference and returns `Shared<T>`
    ///
    /// # Arguments
    ///
    /// - `factory`: A closure that creates the instance when first requested
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::{Provider, Shared};
    ///
    /// struct Config {
    ///     debug: bool,
    /// }
    ///
    /// let provider = Provider::singleton(|_injector| {
    ///     Shared::new(Config { debug: true })
    /// });
    /// ```
    ///
    /// # Note
    ///
    /// This is the single-threaded version. The factory does not need to be `Send + Sync`.
    pub fn singleton<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        #[cfg(feature = "tracing")]
        info!(
            type_name = std::any::type_name::<T>(),
            scope = %Scope::Module,
            threading = "single-threaded",
            factory_mode = "sync",
            "Creating singleton provider"
        );

        Provider::<T> {
            scope: Scope::Module,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!(
                    type_name = std::any::type_name::<T>(),
                    scope = %Scope::Module,
                    op = "provider_factory_call",
                    "Executing singleton factory"
                );

                Instance::new(factory(injector))
            }),
            #[cfg(feature = "async-factory")]
            async_factory: None,
            try_factory: None,
            #[cfg(feature = "async-factory")]
            try_async_factory: None,
            limits: Limits::default(),
            dependency_hints: Vec::new(),
            limiter: None,
        }
    }

    /// Creates a scope-scoped provider (single-threaded).
    ///
    /// A scoped provider creates **one instance per runtime scope**
    /// (see `Injector::create_scope()`).
    pub fn scoped<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        #[cfg(feature = "tracing")]
        info!(
            type_name = std::any::type_name::<T>(),
            scope = %Scope::Scoped,
            threading = "single-threaded",
            factory_mode = "sync",
            "Creating scoped provider"
        );

        Provider::<T> {
            scope: Scope::Scoped,
            factory: Box::new(move |injector| Instance::new(factory(injector))),
            #[cfg(feature = "async-factory")]
            async_factory: None,
            try_factory: None,
            #[cfg(feature = "async-factory")]
            try_async_factory: None,
            limits: Limits::default(),
            dependency_hints: Vec::new(),
            limiter: None,
        }
    }

    /// Creates a transient provider (single-threaded).
    ///
    /// A transient provider creates a **new instance on every resolution**.
    /// No caching or instance reuse occurs.
    ///
    /// # Type Parameters
    ///
    /// - `F`: Factory function type that takes an [`Injector`] reference and returns `Shared<T>`
    ///
    /// # Arguments
    ///
    /// - `factory`: A closure that creates a new instance on each invocation
    ///
    /// # Use Cases
    ///
    /// - Request handlers
    /// - Short-lived operations
    /// - Stateful services that should not be shared
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::{Provider, Shared};
    ///
    /// struct RequestHandler {
    ///     id: u64,
    /// }
    ///
    /// let provider = Provider::transient(|_injector| {
    ///     Shared::new(RequestHandler {
    ///         id: std::time::SystemTime::now()
    ///             .duration_since(std::time::UNIX_EPOCH)
    ///             .unwrap()
    ///             .as_nanos() as u64,
    ///     })
    /// });
    /// ```
    ///
    /// # Note
    ///
    /// This is the single-threaded version. The factory does not need to be `Send + Sync`.
    pub fn transient<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        #[cfg(feature = "tracing")]
        info!(
            type_name = std::any::type_name::<T>(),
            scope = %Scope::Transient,
            threading = "single-threaded",
            factory_mode = "sync",
            "Creating transient provider"
        );

        Provider::<T> {
            scope: Scope::Transient,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!(
                    type_name = std::any::type_name::<T>(),
                    scope = %Scope::Transient,
                    op = "provider_factory_call",
                    "Executing transient factory"
                );

                Instance::new(factory(injector))
            }),
            #[cfg(feature = "async-factory")]
            async_factory: None,
            try_factory: None,
            #[cfg(feature = "async-factory")]
            try_async_factory: None,
            limits: Limits::default(),
            dependency_hints: Vec::new(),
            limiter: None,
        }
    }

    /// Creates a root-scoped provider (single-threaded).
    ///
    /// A root provider creates **one instance per root injector** (application-wide).
    /// This is the highest level of singleton, shared across all child injectors.
    ///
    /// # Type Parameters
    ///
    /// - `F`: Factory function type that takes an [`Injector`] reference and returns `Shared<T>`
    ///
    /// # Arguments
    ///
    /// - `factory`: A closure that creates the instance when first requested
    ///
    /// # Use Cases
    ///
    /// - Application configuration
    /// - Logging infrastructure
    /// - Connection pools
    /// - Global caches
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::{Provider, Shared};
    ///
    /// struct AppConfig {
    ///     version: String,
    /// }
    ///
    /// let provider = Provider::root(|_injector| {
    ///     Shared::new(AppConfig {
    ///         version: "1.0.0".to_string(),
    ///     })
    /// });
    /// ```
    ///
    /// # Note
    ///
    /// This is the single-threaded version. The factory does not need to be `Send + Sync`.
    pub fn root<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        #[cfg(feature = "tracing")]
        info!(
            type_name = std::any::type_name::<T>(),
            scope = %Scope::Root,
            threading = "single-threaded",
            factory_mode = "sync",
            "Creating root provider"
        );

        Provider::<T> {
            scope: Scope::Root,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!(
                    type_name = std::any::type_name::<T>(),
                    scope = %Scope::Root,
                    op = "provider_factory_call",
                    "Executing root factory"
                );

                Instance::new(factory(injector))
            }),
            #[cfg(feature = "async-factory")]
            async_factory: None,
            try_factory: None,
            #[cfg(feature = "async-factory")]
            try_async_factory: None,
            limits: Limits::default(),
            dependency_hints: Vec::new(),
            limiter: None,
        }
    }

    /// Creates a singleton provider with resource limits.
    pub fn singleton_with_limits<F>(limits: Limits, factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        Self::singleton(factory).with_limits(limits)
    }

    /// Creates a transient provider with resource limits.
    pub fn transient_with_limits<F>(limits: Limits, factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        Self::transient(factory).with_limits(limits)
    }

    /// Creates a root provider with resource limits.
    pub fn root_with_limits<F>(limits: Limits, factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        Self::root(factory).with_limits(limits)
    }

    /// Creates a scoped provider with resource limits.
    pub fn scoped_with_limits<F>(limits: Limits, factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        Self::scoped(factory).with_limits(limits)
    }
}
