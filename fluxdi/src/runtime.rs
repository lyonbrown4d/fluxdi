//! Runtime type definitions for shared ownership and interior mutability.
//!
//! This module provides conditional type aliases based on the `thread-safe` feature flag:
//!
//! - When `thread-safe` is enabled: Uses thread-safe types (`Arc` and `RwLock`)
//! - When `thread-safe` is disabled: Uses single-threaded types (`Rc` and `RefCell`)
//!
//! # Type Aliases
//!
//! - [`Shared<T>`]: Smart pointer for shared ownership
//! - [`Store<T>`]: Container providing interior mutability
//!
//! # Examples
//!
//! ```
//! use fluxdi::runtime::{Shared, Store};
//!
//! // Create a shared reference to a store
//! let value = Store::new(42);
//! let shared = Shared::new(value);
//! ```

#[cfg(feature = "thread-safe")]
use std::sync::{Arc, RwLock};

#[cfg(not(feature = "thread-safe"))]
use std::{cell::RefCell, rc::Rc};

/// Type alias for shared ownership of data.
///
/// When the `thread-safe` feature is enabled, this is [`Arc<T>`] (thread-safe reference counting).
/// When disabled, this is [`std::rc::Rc<T>`] (single-threaded reference counting).
///
/// # Thread Safety
///
/// - **With `thread-safe` feature**: `Arc<T>` allows sharing data across threads safely.
/// - **Without `thread-safe` feature**: `std::rc::Rc<T>` is more performant but not thread-safe.
///
/// # Examples
///
/// ```
/// use fluxdi::runtime::Shared;
///
/// let data = Shared::new(vec![1, 2, 3]);
/// let clone = Shared::clone(&data);
/// ```
#[cfg(feature = "thread-safe")]
pub type Shared<T> = Arc<T>;
#[cfg(not(feature = "thread-safe"))]
pub type Shared<T> = Rc<T>;

/// Type alias for interior mutability with runtime borrow checking.
///
/// When the `thread-safe` feature is enabled, this is [`RwLock<T>`] (thread-safe read-write lock).
/// When disabled, this is [`std::cell::RefCell<T>`] (single-threaded interior mutability).
///
/// # Thread Safety
///
/// - **With `thread-safe` feature**: `RwLock<T>` provides safe concurrent access with blocking.
/// - **Without `thread-safe` feature**: `std::cell::RefCell<T>` uses runtime borrow checking without locks.
///
/// # Examples
///
/// ```
/// use fluxdi::runtime::Store;
///
/// let store = Store::new(42);
/// #[cfg(feature = "thread-safe")]
/// {
///     let value = store.read().unwrap();
///     assert_eq!(*value, 42);
/// }
/// #[cfg(not(feature = "thread-safe"))]
/// {
///     let value = store.borrow();
///     assert_eq!(*value, 42);
/// }
/// ```
#[cfg(feature = "thread-safe")]
pub type Store<T> = RwLock<T>;
#[cfg(not(feature = "thread-safe"))]
pub type Store<T> = RefCell<T>;

/// A value that can be resolved by the current FluxDI runtime.
///
/// All `'static` types implement this trait. With `thread-safe`, values must
/// additionally implement `Send + Sync`. Unsized trait objects are supported.
#[cfg(feature = "thread-safe")]
pub trait Service: Send + Sync + 'static {}
#[cfg(feature = "thread-safe")]
impl<T: ?Sized + Send + Sync + 'static> Service for T {}
#[cfg(not(feature = "thread-safe"))]
pub trait Service: 'static {}
#[cfg(not(feature = "thread-safe"))]
impl<T: ?Sized + 'static> Service for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_can_be_cloned() {
        let data = Shared::new(100);
        let clone = Shared::clone(&data);

        #[cfg(feature = "thread-safe")]
        assert_eq!(Arc::strong_count(&data), 2);

        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(Rc::strong_count(&data), 2);

        drop(clone);

        #[cfg(feature = "thread-safe")]
        assert_eq!(Arc::strong_count(&data), 1);

        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(Rc::strong_count(&data), 1);
    }

    #[test]
    fn test_store_allows_mutation() {
        let store = Store::new(42);

        #[cfg(feature = "thread-safe")]
        {
            {
                let value = store.read().unwrap();
                assert_eq!(*value, 42);
            }
            {
                let mut value = store.write().unwrap();
                *value = 100;
            }
            {
                let value = store.read().unwrap();
                assert_eq!(*value, 100);
            }
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            {
                let value = store.borrow();
                assert_eq!(*value, 42);
            }
            {
                let mut value = store.borrow_mut();
                *value = 100;
            }
            {
                let value = store.borrow();
                assert_eq!(*value, 100);
            }
        }
    }

    #[test]
    fn test_shared_with_store() {
        let store = Store::new(vec![1, 2, 3]);
        let shared = Shared::new(store);
        let clone = Shared::clone(&shared);

        #[cfg(feature = "thread-safe")]
        {
            let data = shared.read().unwrap();
            assert_eq!(data.len(), 3);
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            let data = shared.borrow();
            assert_eq!(data.len(), 3);
        }

        drop(clone);
    }

    #[test]
    fn test_store_with_string() {
        let store = Store::new(String::from("Hello"));

        #[cfg(feature = "thread-safe")]
        {
            let mut value = store.write().unwrap();
            value.push_str(", World!");
            assert_eq!(*value, "Hello, World!");
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            let mut value = store.borrow_mut();
            value.push_str(", World!");
            assert_eq!(*value, "Hello, World!");
        }
    }

    #[test]
    fn test_multiple_shared_references() {
        let data = Shared::new(42);
        let refs: Vec<_> = (0..5).map(|_| Shared::clone(&data)).collect();

        #[cfg(feature = "thread-safe")]
        assert_eq!(Arc::strong_count(&data), 6); // original + 5 clones

        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(Rc::strong_count(&data), 6); // original + 5 clones

        drop(refs);

        #[cfg(feature = "thread-safe")]
        assert_eq!(Arc::strong_count(&data), 1);

        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(Rc::strong_count(&data), 1);
    }
}
