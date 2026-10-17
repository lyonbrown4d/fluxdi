#[cfg(feature = "tracing")]
use tracing::debug;

/// Defines the lifecycle scope of a service in the dependency injection container.
///
/// # Variants
///
/// * `Root` - Root-level singleton. The instance is created once and shared
///   across the entire application during its complete lifecycle.
///
/// * `Module` - Calling-context singleton. The instance is created once per resolving injector
///   context. Cloned handles share it; child injectors have their own caches.
///
/// * `Scoped` - Scope-level singleton. The instance is created once per runtime
///   scope (created with [`Injector::create_scope`](crate::Injector::create_scope))
///   and shared inside that scope only.
///
/// * `Transient` - Transient instance. A new instance is created each time
///   the service is requested from the dependency injection container.
///
/// # Examples
///
/// ```no_run
/// use fluxdi::Scope;
///
/// let root_scope = Scope::Root;
/// assert!(root_scope.is_singleton());
///
/// let transient_scope = Scope::Transient;
/// assert!(!transient_scope.is_singleton());
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    Root,
    Module,
    Scoped,
    Transient,
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scope::Root => write!(f, "Root"),
            Scope::Module => write!(f, "Module"),
            Scope::Scoped => write!(f, "Scoped"),
            Scope::Transient => write!(f, "Transient"),
        }
    }
}

impl Scope {
    /// Checks if this scope represents a singleton service.
    ///
    /// Returns `true` for `Root`, `Module`, and `Scoped` scopes, which create
    /// a single shared instance in their respective cache domain. Returns `false`
    /// for `Transient` scope, which creates a new instance on each request.
    ///
    /// # Returns
    ///
    /// * `true` - If the scope is `Root`, `Module`, or `Scoped` (singleton)
    /// * `false` - If the scope is `Transient` (not singleton)
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::Scope;
    ///
    /// assert!(Scope::Root.is_singleton());
    /// assert!(Scope::Module.is_singleton());
    /// assert!(Scope::Scoped.is_singleton());
    /// assert!(!Scope::Transient.is_singleton());
    /// ```
    pub fn is_singleton(self) -> bool {
        let result = matches!(self, Scope::Root | Scope::Module | Scope::Scoped);

        #[cfg(feature = "tracing")]
        debug!(
            "Checking scope lifecycle: scope={}, is_singleton={}",
            self, result
        );

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_is_singleton() {
        let scope = Scope::Root;
        assert!(scope.is_singleton(), "Scope::Root should be singleton");
    }

    #[test]
    fn test_module_is_singleton() {
        let scope = Scope::Module;
        assert!(scope.is_singleton(), "Scope::Module should be singleton");
    }

    #[test]
    fn test_transient_is_not_singleton() {
        let scope = Scope::Transient;
        assert!(
            !scope.is_singleton(),
            "Scope::Transient should not be singleton"
        );
    }

    #[test]
    fn test_scoped_is_singleton() {
        let scope = Scope::Scoped;
        assert!(scope.is_singleton(), "Scope::Scoped should be singleton");
    }

    #[test]
    fn test_scope_is_copy() {
        let scope1 = Scope::Root;
        let scope2 = scope1;
        // Both should be equal after copy
        assert!(scope1.is_singleton());
        assert!(scope2.is_singleton());
    }

    #[test]
    fn test_scope_is_clone() {
        let scope1 = Scope::Module;
        let scope2 = scope1;
        assert!(scope1.is_singleton());
        assert!(scope2.is_singleton());
    }

    #[test]
    fn test_all_scopes_are_covered() {
        // Test that all enum variants have been considered
        let scopes = [Scope::Root, Scope::Module, Scope::Scoped, Scope::Transient];

        let singleton_count = scopes.iter().filter(|s| s.is_singleton()).count();
        assert_eq!(
            singleton_count, 3,
            "There should be exactly 3 singleton scopes"
        );
    }
}
