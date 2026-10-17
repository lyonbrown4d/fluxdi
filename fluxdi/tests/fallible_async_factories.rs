#![cfg(feature = "async-factory")]
use fluxdi::{Error, ErrorKind, Injector, Limits, Provider, Scope, Shared};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[tokio::test]
async fn async_failures_retry_and_decorate_successes_in_each_scope() {
    for scope in [Scope::Root, Scope::Module, Scope::Scoped, Scope::Transient] {
        let injector = Injector::root().create_scope();
        let calls = Arc::new(AtomicUsize::new(0));
        let factory = {
            let calls = calls.clone();
            move |_: Injector| {
                let first = calls.fetch_add(1, Ordering::SeqCst) == 0;
                async move {
                    if first {
                        Err(Error::factory_failed("u32", "async initialization failed"))
                    } else {
                        Ok(Shared::new(5u32))
                    }
                }
            }
        };
        let provider = match scope {
            Scope::Root => Provider::try_root_async(factory),
            Scope::Module => Provider::try_singleton_async(factory),
            Scope::Scoped => Provider::try_scoped_async(factory),
            Scope::Transient => Provider::try_transient_async(factory),
        }
        .with_limits(Limits::deny(1))
        .with_decorator(|value| Shared::new(*value + 1))
        .with_decorator(|value| Shared::new(*value * 2));
        injector.provide::<u32>(provider);
        assert_eq!(
            injector.try_resolve::<u32>().unwrap_err().kind,
            ErrorKind::AsyncFactoryRequiresAsyncResolve
        );
        assert_eq!(
            injector.try_resolve_async::<u32>().await.unwrap_err().kind,
            ErrorKind::FactoryFailed
        );
        let a = injector.try_resolve_async::<u32>().await.unwrap();
        let b = injector.try_resolve_async::<u32>().await.unwrap();
        assert_eq!(*a, 12);
        assert_eq!(*b, 12);
        let cached = scope != Scope::Transient;
        assert_eq!(Shared::ptr_eq(&a, &b), cached);
        assert_eq!(calls.load(Ordering::SeqCst), if cached { 2 } else { 3 });
        if cached {
            assert!(Shared::ptr_eq(&a, &injector.resolve::<u32>()));
        }
    }
}

#[tokio::test]
async fn failed_initializer_wakes_waiter_to_retry_and_releases_creation_permit() {
    let injector = Injector::root();
    let calls = Arc::new(AtomicUsize::new(0));
    injector.provide::<u32>(
        Provider::try_root_async({
            let calls = calls.clone();
            move |_| {
                let first = calls.fetch_add(1, Ordering::SeqCst) == 0;
                async move {
                    tokio::task::yield_now().await;
                    if first {
                        Err(Error::factory_failed("u32", "first attempt failed"))
                    } else {
                        Ok(Shared::new(7))
                    }
                }
            }
        })
        .with_limits(Limits::deny(1)),
    );
    let (a, b) = tokio::join!(
        injector.try_resolve_async::<u32>(),
        injector.try_resolve_async::<u32>()
    );
    let (error, value) = match (a, b) {
        (Err(error), Ok(value)) | (Ok(value), Err(error)) => (error, value),
        _ => panic!("one caller must fail and the waiter must retry"),
    };
    assert_eq!(error.kind, ErrorKind::FactoryFailed);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(Shared::ptr_eq(
        &value,
        &injector.resolve_async::<u32>().await
    ));
}

#[tokio::test]
async fn async_resolution_propagates_sync_factory_errors_and_can_retry() {
    let injector = Injector::root();
    injector.provide::<u32>(Provider::try_root(|inj| {
        inj.try_resolve::<u64>()?;
        Ok(Shared::new(7))
    }));
    assert_eq!(
        injector.try_resolve_async::<u32>().await.unwrap_err().kind,
        ErrorKind::ServiceNotProvided
    );
    injector.provide::<u64>(Provider::root(|_| Shared::new(1)));
    assert_eq!(*injector.resolve_async::<u32>().await, 7);
}

#[tokio::test]
async fn named_and_set_async_bindings_propagate_errors() {
    let injector = Injector::root();
    injector.provide_named::<u32>(
        "failure",
        Provider::try_root_async(|_| async { Err(Error::factory_failed("u32", "named failure")) }),
    );
    injector.provide_into_set::<u32>(Provider::try_scoped_async(|_| async {
        Err(Error::factory_failed("u32", "set failure"))
    }));
    assert!(
        injector
            .try_resolve_named_async::<u32>("failure")
            .await
            .unwrap_err()
            .message
            .contains("named failure")
    );
    assert!(
        injector
            .try_resolve_all_async::<u32>()
            .await
            .unwrap_err()
            .message
            .contains("set failure")
    );
}
