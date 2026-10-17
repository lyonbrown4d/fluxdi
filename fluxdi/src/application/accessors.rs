use super::*;

impl Application {
    /// Returns a shared reference to the root injector.
    ///
    /// The injector can be used to resolve dependencies after the application
    /// has been bootstrapped. The returned reference can be cloned to share
    /// access to the injector.
    ///
    /// # Returns
    ///
    /// A shared reference to the root [`Injector`].
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::application::Application;
    /// use fluxdi::module::Module;
    /// use fluxdi::injector::Injector;
    ///
    /// struct AppModule;
    /// impl Module for AppModule {
    ///     fn providers(&self, injector: &Injector) {}
    /// }
    ///
    /// let mut app = Application::new(AppModule);
    /// app.bootstrap_sync().unwrap();
    ///
    /// let injector = app.injector();
    /// let another_ref = app.injector();
    /// // Both references point to the same injector
    /// ```
    pub fn injector(&self) -> Shared<Injector> {
        #[cfg(feature = "tracing")]
        debug!("Accessing root injector");

        #[cfg(feature = "tracing")]
        {
            if self.is_bootstrapped() {
                debug!("Injector is available and application is bootstrapped");
            } else {
                debug!("Injector is available but application is not bootstrapped yet");
            }
        }

        self.injector.clone()
    }

    /// Checks whether the application has been bootstrapped.
    ///
    /// Returns `true` after successful startup through either
    /// [`bootstrap_sync()`](Application::bootstrap_sync) or [`bootstrap()`](Application::bootstrap),
    /// `false` otherwise.
    ///
    /// # Returns
    ///
    /// - `true` if the application is bootstrapped
    /// - `false` if the application has not been bootstrapped yet
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::application::Application;
    /// use fluxdi::module::Module;
    /// use fluxdi::injector::Injector;
    ///
    /// struct AppModule;
    /// impl Module for AppModule {
    ///     fn providers(&self, injector: &Injector) {}
    /// }
    ///
    /// let mut app = Application::new(AppModule);
    /// assert!(!app.is_bootstrapped());
    ///
    /// app.bootstrap_sync().unwrap();
    /// assert!(app.is_bootstrapped());
    /// ```
    pub fn is_bootstrapped(&self) -> bool {
        let bootstrapped = self.bootstrapped;

        #[cfg(feature = "tracing")]
        debug!("Checking application bootstrap state: {}", bootstrapped);

        bootstrapped
    }
}
