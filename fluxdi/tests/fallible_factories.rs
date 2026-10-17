use fluxdi::{Error, ErrorKind, Injector, Limits, Provider, Scope, Shared};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn failed_initialization_releases_limits_and_only_successes_are_cached_in_each_scope() {
    for scope in [Scope::Root, Scope::Module, Scope::Scoped, Scope::Transient] {
        let injector = Injector::root().create_scope();
        let calls = Arc::new(AtomicUsize::new(0));
        let decorated = Arc::new(AtomicUsize::new(0));
        let factory = {
            let calls = calls.clone();
            move |_: &Injector| {
                if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err(Error::factory_failed("u32", "temporarily unavailable"))
                } else {
                    Ok(Shared::new(7u32))
                }
            }
        };
        let provider = match scope {
            Scope::Root => Provider::try_root(factory),
            Scope::Module => Provider::try_singleton(factory),
            Scope::Scoped => Provider::try_scoped(factory),
            Scope::Transient => Provider::try_transient(factory),
        }
        .with_limits(Limits::deny(1))
        .with_decorator({
            let decorated = decorated.clone();
            move |value| {
                decorated.fetch_add(1, Ordering::SeqCst);
                Shared::new(*value + 1)
            }
        });
        injector.provide::<u32>(provider);
        let error = injector.try_resolve::<u32>().unwrap_err();
        assert_eq!(error.kind, ErrorKind::FactoryFailed);
        assert!(error.message.contains("temporarily unavailable"));
        assert_eq!(decorated.load(Ordering::SeqCst), 0);
        let a = injector.try_resolve::<u32>().unwrap();
        let b = injector.try_resolve::<u32>().unwrap();
        assert_eq!(*a, 8);
        assert_eq!(*b, 8);
        let cached = scope != Scope::Transient;
        assert_eq!(Shared::ptr_eq(&a, &b), cached);
        assert_eq!(calls.load(Ordering::SeqCst), if cached { 2 } else { 3 });
        assert_eq!(decorated.load(Ordering::SeqCst), if cached { 1 } else { 2 });
        #[cfg(feature = "metrics")]
        {
            let snapshot = injector.metrics_snapshot();
            assert_eq!(snapshot.resolve_failures_total, 1);
            assert_eq!(snapshot.resolve_success_total, 2);
        }
    }
}

#[test]
fn dependency_errors_propagate_without_being_reclassified() {
    let injector = Injector::root();
    injector.provide::<u32>(Provider::try_root(|inj| {
        inj.try_resolve::<u64>()?;
        Ok(Shared::new(7))
    }));
    let error = injector.try_resolve::<u32>().unwrap_err();
    assert_eq!(error.kind, ErrorKind::ServiceNotProvided);
    assert!(error.message.contains("u64"));
    injector.provide::<u64>(Provider::root(|_| Shared::new(1)));
    assert_eq!(*injector.try_resolve::<u32>().unwrap(), 7);
}

#[test]
fn named_and_set_trait_bindings_keep_fallible_factories_and_decorators() {
    trait Backend: std::fmt::Debug + Send + Sync {
        fn id(&self) -> u32;
    }
    #[derive(Debug)]
    struct Memory(u32);
    impl Backend for Memory {
        fn id(&self) -> u32 {
            self.0
        }
    }
    let injector = Injector::root();
    injector.provide_named::<dyn Backend>(
        "main",
        Provider::try_root(|_| Ok(Shared::new(Memory(7)) as Shared<dyn Backend>))
            .with_decorator(|value| Shared::new(Memory(value.id() + 1)) as Shared<dyn Backend>),
    );
    injector.provide_into_set::<dyn Backend>(Provider::try_singleton(|_| {
        Ok(Shared::new(Memory(9)) as Shared<dyn Backend>)
    }));
    assert_eq!(
        injector
            .try_resolve_named::<dyn Backend>("main")
            .unwrap()
            .id(),
        8
    );
    assert_eq!(
        injector.try_resolve_all::<dyn Backend>().unwrap()[0].id(),
        9
    );
    let expected = Error::factory_failed("Backend", "set initialization failed");
    injector.provide_into_set::<dyn Backend>(Provider::try_transient({
        let expected = expected.clone();
        move |_| Err(expected.clone())
    }));
    let actual = injector.try_resolve_all::<dyn Backend>().unwrap_err();
    assert_eq!(actual.kind, expected.kind);
    assert_eq!(actual.message, expected.message);
    injector.provide_named::<u32>(
        "failure",
        Provider::try_root(move |_| Err(expected.clone())),
    );
    assert_eq!(
        injector
            .try_resolve_named::<u32>("failure")
            .unwrap_err()
            .kind,
        ErrorKind::FactoryFailed
    );
}

#[test]
#[should_panic(expected = "explicit failure")]
fn panicking_resolution_remains_available_for_fallible_providers() {
    let injector = Injector::root();
    injector.provide::<u32>(Provider::try_root(|_| {
        Err(Error::factory_failed("u32", "explicit failure"))
    }));
    injector.resolve::<u32>();
}
