use fluxdi::{ErrorKind, Injector, Limits, Provider, Shared};

#[test]
fn explicit_handles_keep_identity_separate_from_the_registry() {
    let context = Injector::root().create_scope();
    context.provide::<u32>(Provider::root(|_| Shared::new(99)));
    let a = Shared::new(Provider::scoped(|_| Shared::new(1u32)));
    let b = Shared::new(Provider::scoped(|_| Shared::new(2u32)));
    let value = context.try_resolve_provider(&a).unwrap();
    assert!(Shared::ptr_eq(
        &value,
        &context.clone().try_resolve_provider(&a).unwrap()
    ));
    assert_eq!(*context.try_resolve_provider(&b).unwrap(), 2);
    assert_eq!(*context.try_resolve::<u32>().unwrap(), 99);
    assert!(!Shared::ptr_eq(
        &value,
        &context.create_scope().try_resolve_provider(&a).unwrap()
    ));
    // Explicit handles have no registration side effect.
    assert_eq!(context.dependency_graph().nodes.len(), 1);
}

#[test]
fn explicit_provider_failure_retries_and_decorates_success() {
    let context = Injector::root();
    let provider = Shared::new(
        Provider::try_singleton(|injector| {
            injector.try_resolve::<u64>()?;
            Ok(Shared::new(3u32))
        })
        .with_limits(Limits::deny(1))
        .with_decorator(|value| Shared::new(*value + 1)),
    );
    assert_eq!(
        context.try_resolve_provider(&provider).unwrap_err().kind,
        ErrorKind::ServiceNotProvided
    );
    context.provide::<u64>(Provider::root(|_| Shared::new(1)));
    let value = context.try_resolve_provider(&provider).unwrap();
    assert_eq!(*value, 4);
    assert!(Shared::ptr_eq(
        &value,
        &context.try_resolve_provider(&provider).unwrap()
    ));
}

#[cfg(feature = "async-factory")]
#[test]
fn explicit_async_provider_cancellation_releases_owner_and_creation_permit() {
    use std::{
        future::Future,
        sync::atomic::{AtomicUsize, Ordering},
        task::{Context, Poll},
    };
    let context = Injector::root();
    let calls = Shared::new(AtomicUsize::new(0));
    let provider = Shared::new(
        Provider::try_root_async({
            let calls = calls.clone();
            move |_| {
                let first = calls.fetch_add(1, Ordering::SeqCst) == 0;
                async move {
                    if first {
                        std::future::pending::<()>().await;
                    }
                    Ok(Shared::new(7u32))
                }
            }
        })
        .with_limits(Limits::deny(1)),
    );
    let waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut owner = Box::pin(context.try_resolve_provider_async(&provider));
    let mut waiter = Box::pin(context.try_resolve_provider_async(&provider));
    assert!(owner.as_mut().poll(&mut cx).is_pending());
    assert!(waiter.as_mut().poll(&mut cx).is_pending());
    drop(owner);
    let value = match waiter.as_mut().poll(&mut cx) {
        Poll::Ready(Ok(value)) => value,
        _ => panic!("waiter must retry canceled initialization"),
    };
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(Shared::ptr_eq(
        &value,
        &context.try_resolve_provider(&provider).unwrap()
    ));
}
