use super::*;

impl Application {
    /// Creates a new application with the given root module.
    ///
    /// The application is created in an un-bootstrapped state. You must call
    /// either [`bootstrap_sync()`](Application::bootstrap_sync) or
    /// [`bootstrap()`](Application::bootstrap) to load the module and its dependencies.
    ///
    /// # Parameters
    ///
    /// - `root`: The root module that defines the application's dependency graph
    ///
    /// # Returns
    ///
    /// A new `Application` instance ready to be bootstrapped.
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::application::Application;
    /// use fluxdi::module::Module;
    /// use fluxdi::injector::Injector;
    ///
    /// struct RootModule;
    ///
    /// impl Module for RootModule {
    ///     fn providers(&self, injector: &Injector) {}
    /// }
    ///
    /// let app = Application::new(RootModule);
    /// assert!(!app.is_bootstrapped());
    /// ```
    pub fn new(root: impl Module + 'static) -> Self {
        #[cfg(feature = "tracing")]
        info!("Creating new Application instance with root module");

        Self {
            bootstrapped: false,
            root: Some(Box::new(root)),
            injector: Shared::new(Injector::root()),
            started_modules: Vec::new(),
        }
    }
}
