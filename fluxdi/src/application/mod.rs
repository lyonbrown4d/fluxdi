//! Application container for bootstrapping and managing the dependency injection system.
//!
//! This module provides the [`Application`] struct, which serves as the main entry point
//! for configuring and initializing a dependency injection container with a modular structure.
//!
//! # Overview
//!
//! The `Application` manages:
//! - Root module registration
//! - Bootstrap process for loading modules and their dependencies
//! - Access to the root injector
//! - Hierarchical module loading with proper isolation
//!
//! # Thread Safety
//!
//! When the `thread-safe` feature is enabled, the [`Application`] requires the root module
//! to implement `Send + Sync`, allowing the application to be safely shared across threads.
//!
//! # Examples
//!
//! ```
//! use fluxdi::application::Application;
//! use fluxdi::module::Module;
//! use fluxdi::injector::Injector;
//!
//! struct AppModule;
//!
//! impl Module for AppModule {
//!     fn providers(&self, injector: &Injector) {
//!         // Register providers
//!     }
//! }
//!
//! let mut app = Application::new(AppModule);
//! app.bootstrap_sync().unwrap();
//!
//! let injector = app.injector();
//! // Use injector to resolve dependencies
//! ```

use crate::Error;
use crate::injector::Injector;
use crate::module::Module;
use crate::runtime::Shared;

#[cfg(feature = "tracing")]
use tracing::{debug, info};

#[cfg(not(feature = "thread-safe"))]
type ModuleObject = Box<dyn Module>;
#[cfg(feature = "thread-safe")]
type ModuleObject = Box<dyn Module>;

struct LoadedModule {
    module: ModuleObject,
    injector: Shared<Injector>,
    started: bool,
}

/// The main application container for dependency injection.
///
/// `Application` manages the lifecycle of modules and provides access to the root
/// dependency injector. It handles the bootstrap process, which recursively loads
/// all modules and their imports, creating a hierarchical injector structure.
///
/// # Thread Safety
///
/// With the `thread-safe` feature enabled, the application requires modules to implement
/// `Send + Sync` to ensure they can be safely shared across threads. Without this feature,
/// modules have no additional thread-safety requirements.
///
/// # Lifecycle
///
/// 1. **Creation**: Create an application with a root module using [`new()`](Application::new)
/// 2. **Bootstrap**: Call [`bootstrap_sync()`](Application::bootstrap_sync) for sync-only modules
///    or [`bootstrap()`](Application::bootstrap) for async-capable lifecycle hooks
/// 3. **Usage**: Access the injector via [`injector()`](Application::injector) to resolve dependencies
/// 4. **Shutdown (optional)**: Call [`shutdown()`](Application::shutdown) to run `on_stop` hooks
///
/// # Examples
///
/// ```
/// use fluxdi::application::Application;
/// use fluxdi::module::Module;
/// use fluxdi::injector::Injector;
///
/// struct MyAppModule;
///
/// impl Module for MyAppModule {
///     fn providers(&self, injector: &Injector) {
///         // Configure your providers
///     }
/// }
///
/// let mut app = Application::new(MyAppModule);
/// assert!(!app.is_bootstrapped());
///
/// app.bootstrap_sync().unwrap();
/// assert!(app.is_bootstrapped());
///
/// let injector = app.injector();
/// // Use injector to get services
/// ```
pub struct Application {
    bootstrapped: bool,
    root: Option<ModuleObject>,
    injector: Shared<Injector>,
    started_modules: Vec<LoadedModule>,
}

#[cfg(feature = "debug")]
impl std::fmt::Debug for Application {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Application")
            .field("injector", &"...")
            .field("root", &"<dyn Module>")
            .field("started_modules", &self.started_modules.len())
            .finish()
    }
}

mod accessors;
mod constructors;
mod lifecycle;
mod module_loading;
pub mod options;

pub use options::{BootstrapOptions, ShutdownOptions};

#[cfg(test)]
mod tests;
