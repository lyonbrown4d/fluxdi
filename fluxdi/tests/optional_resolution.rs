use fluxdi::{Error, ErrorKind, Injector, Provider, Shared};

#[test]
fn optional_resolution_preserves_hierarchy_binding_identity_and_errors() {
    let root = Injector::root();
    assert!(root.try_resolve_optional::<u32>().unwrap().is_none());
    root.provide::<u32>(Provider::root(|_| Shared::new(7)));
    root.provide_named::<u32>("primary", Provider::root(|_| Shared::new(8)));
    let child = Injector::child(Shared::new(root));
    assert_eq!(*child.try_resolve_optional::<u32>().unwrap().unwrap(), 7);
    assert_eq!(
        *child
            .try_resolve_optional_named::<u32>("primary")
            .unwrap()
            .unwrap(),
        8
    );
    assert!(
        child
            .try_resolve_optional_named::<u32>("missing")
            .unwrap()
            .is_none()
    );
    child.provide::<u32>(Provider::try_singleton(|inj| {
        inj.try_resolve::<bool>()?;
        Ok(Shared::new(9))
    }));
    assert_eq!(
        child.try_resolve_optional::<u32>().unwrap_err().kind,
        ErrorKind::ServiceNotProvided
    );
    child.provide::<bool>(Provider::root(|_| Shared::new(true)));
    assert_eq!(*child.try_resolve_optional::<u32>().unwrap().unwrap(), 9);
    child.provide_named::<u32>(
        "primary",
        Provider::try_singleton(|_| Err(Error::factory_failed("u32", "offline"))),
    );
    assert_eq!(
        child
            .try_resolve_optional_named::<u32>("primary")
            .unwrap_err()
            .kind,
        ErrorKind::FactoryFailed
    );
}

#[cfg(feature = "async-factory")]
#[tokio::test]
async fn async_optional_resolution_propagates_errors_retries_and_reads_cached_values() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let injector = Injector::root();
    assert!(
        injector
            .try_resolve_optional_async::<u32>()
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        injector
            .try_resolve_optional_named_async::<u32>("main")
            .await
            .unwrap()
            .is_none()
    );
    injector.provide::<u32>(Provider::try_root_async(|inj| async move {
        inj.try_resolve::<bool>()?;
        Ok(Shared::new(7))
    }));
    assert_eq!(
        injector.try_resolve_optional::<u32>().unwrap_err().kind,
        ErrorKind::AsyncFactoryRequiresAsyncResolve
    );
    assert_eq!(
        injector
            .try_resolve_optional_async::<u32>()
            .await
            .unwrap_err()
            .kind,
        ErrorKind::ServiceNotProvided
    );
    injector.provide::<bool>(Provider::root(|_| Shared::new(true)));
    let value = injector
        .try_resolve_optional_async::<u32>()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(*value, 7);
    assert!(Shared::ptr_eq(
        &value,
        &injector.try_resolve_optional::<u32>().unwrap().unwrap()
    ));
    let first = Arc::new(AtomicBool::new(true));
    injector.provide_named::<u32>(
        "main",
        Provider::try_root_async(move |_| {
            let fail = first.swap(false, Ordering::SeqCst);
            async move {
                if fail {
                    Err(Error::factory_failed("u32", "offline"))
                } else {
                    Ok(Shared::new(8))
                }
            }
        }),
    );
    assert_eq!(
        injector
            .try_resolve_optional_named_async::<u32>("main")
            .await
            .unwrap_err()
            .kind,
        ErrorKind::FactoryFailed
    );
    assert_eq!(
        *injector
            .try_resolve_optional_named_async::<u32>("main")
            .await
            .unwrap()
            .unwrap(),
        8
    );
}
