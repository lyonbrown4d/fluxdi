//! Statically assembled dependency graphs with explicit binding keys.
//!
//! [`crate::static_module!`] generates typed getters and a constructor taking
//! external values in declaration order. Each factory receives the listed
//! dependencies as [`crate::Shared`] values and returns `Result<Shared<T>, Error>`.
//! Bindings can be declared before their dependencies. Different keys may have
//! the same Rust type.
//!
//! ```
//! use fluxdi::{Error, Shared, static_module};
//!
//! fn size(text: Shared<String>) -> Result<Shared<usize>, Error> {
//!     Ok(Shared::new(text.len()))
//! }
//!
//! static_module! {
//!     pub struct App {
//!         size: usize = size(text);
//!         external text: String;
//!     }
//! }
//!
//! let app = App::new(Shared::new("configured".to_owned()));
//! assert_eq!(*app.size()?, 10);
//! assert!(Shared::ptr_eq(&app.size()?, &app.clone().size()?));
//! # Ok::<(), Error>(())
//! ```
//!
//! The default scope is `singleton`, which caches per resolving injector
//! context. `#[scope(root)]` shares across contexts, `#[scope(scoped)]` caches
//! at the nearest runtime scope boundary, and `#[scope(transient)]` creates a
//! fresh value per getter call. `create_child` and `create_scope` retain the
//! typed bindings and create corresponding injector contexts. `in_context`
//! binds those providers to an existing injector context, such as an HTTP
//! request scope.
//!
//! Graph errors are rejected during macro expansion. Unknown dependencies
//! cannot silently fall back to dynamic registry lookup:
//!
//! ```compile_fail
//! use fluxdi::{Error, Shared, static_module};
//! fn make(input: Shared<u32>) -> Result<Shared<u32>, Error> { Ok(input) }
//! static_module! {
//!     struct Missing {
//!         value: u32 = make(absent);
//!     }
//! }
//! ```
//!
//! Binding keys must be unique, even when their factories are identical:
//!
//! ```compile_fail
//! use fluxdi::{Error, Shared, static_module};
//! fn make() -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
//! static_module! {
//!     struct Duplicate {
//!         value: u32 = make();
//!         value: u32 = make();
//!     }
//! }
//! ```
//!
//! Cycles are rejected before any factory runs:
//!
//! ```compile_fail
//! use fluxdi::{Error, Shared, static_module};
//! fn identity(input: Shared<u32>) -> Result<Shared<u32>, Error> { Ok(input) }
//! static_module! {
//!     struct Cycle {
//!         first: u32 = identity(second);
//!         second: u32 = identity(third);
//!         third: u32 = identity(first);
//!     }
//! }
//! ```
//!
//! Rust checks the factory's argument types against the selected bindings:
//!
//! ```compile_fail,E0308
//! use fluxdi::{Error, Shared, static_module};
//! fn make(input: Shared<String>) -> Result<Shared<u32>, Error> {
//!     Ok(Shared::new(input.len() as u32))
//! }
//! static_module! {
//!     struct WrongArgument {
//!         external input: u32;
//!         value: u32 = make(input);
//!     }
//! }
//! ```
//!
//! Rust also checks the fallible shared return type:
//!
//! ```compile_fail,E0308
//! use fluxdi::{Error, Shared, static_module};
//! fn make() -> Result<Shared<String>, Error> { Ok(Shared::new("wrong".into())) }
//! static_module! {
//!     struct WrongReturn {
//!         value: u32 = make();
//!     }
//! }
//! ```
//!
//! Root bindings cannot capture scoped or calling-context singleton bindings,
//! including through transient dependencies:
//!
//! ```compile_fail
//! use fluxdi::{Error, Shared, static_module};
//! fn make() -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
//! fn identity(input: Shared<u32>) -> Result<Shared<u32>, Error> { Ok(input) }
//! static_module! {
//!     struct InvalidLifetime {
//!         #[scope(scoped)]
//!         request: u32 = make();
//!         #[scope(transient)]
//!         intermediate: u32 = identity(request);
//!         #[scope(root)]
//!         application: u32 = identity(intermediate);
//!     }
//! }
//! ```
//!
//! Binding keys cannot shadow generated context methods:
//!
//! ```compile_fail
//! use fluxdi::{Error, Shared, static_module};
//! fn make() -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
//! static_module! {
//!     struct ReservedName {
//!         create_scope: u32 = make();
//!     }
//! }
//! ```
