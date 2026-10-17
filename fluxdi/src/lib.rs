#[cfg(feature = "actix")]
pub mod actix;
pub mod application;
#[cfg(feature = "axum")]
pub mod axum;
pub mod error;
pub mod graph;
pub mod injector;
pub mod instance;
pub mod module;
pub mod observability;
pub mod provider;
pub mod resolve_guard;
pub mod runtime;
pub mod scope;
#[cfg(feature = "macros")]
pub mod static_modules;

pub use application::*;
#[cfg(feature = "axum")]
pub use axum::*;
pub use error::*;
#[cfg(feature = "macros")]
pub use fluxdi_macros::{Injectable, static_module};
pub use graph::*;
pub use injector::*;
pub use instance::*;
pub use module::*;
pub use observability::*;
pub use provider::*;
pub use runtime::*;
pub use scope::*;

// The feature belongs to FluxDI, not the crate invoking the procedural macro.
// Dispatch here so downstream applications need no identically named feature.
#[doc(hidden)]
#[cfg(all(feature = "macros", feature = "async-factory"))]
#[macro_export]
macro_rules! __static_module_async {
    ($($tokens:tt)*) => { $($tokens)* };
}

#[doc(hidden)]
#[cfg(all(feature = "macros", not(feature = "async-factory")))]
#[macro_export]
macro_rules! __static_module_async {
    ($($tokens:tt)*) => {
        compile_error!("async static modules require FluxDI's `async-factory` feature");
    };
}
