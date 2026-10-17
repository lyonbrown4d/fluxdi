#![cfg(feature = "async-factory")]
use fluxdi::{ErrorKind, Injector, Provider, Shared};
use std::{
    future::Future,
    task::{Context, Poll},
};

#[tokio::test]
async fn concurrent_async_resolves_are_independent_and_share_cached_initialization() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let injector = Injector::root();
    let created = Arc::new(AtomicUsize::new(0));
    injector.provide::<u32>(Provider::root_async({
        let created = created.clone();
        move |_| {
            created.fetch_add(1, Ordering::SeqCst);
            async {
                tokio::task::yield_now().await;
                Shared::new(7)
            }
        }
    }));
    let (a, b) = tokio::join!(
        injector.try_resolve_async::<u32>(),
        injector.try_resolve_async::<u32>()
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert!(Shared::ptr_eq(&a, &b));
    assert_eq!(created.load(Ordering::SeqCst), 1);
    injector.provide::<u64>(Provider::transient_async(|_| async {
        tokio::task::yield_now().await;
        Shared::new(7)
    }));
    let (a, b) = tokio::join!(
        injector.try_resolve_async::<u64>(),
        injector.try_resolve_async::<u64>()
    );
    assert!(!Shared::ptr_eq(&a.unwrap(), &b.unwrap()));
}

#[test]
fn canceling_initializer_wakes_waiter_and_allows_retry() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let injector = Injector::root();
    let calls = Arc::new(AtomicUsize::new(0));
    injector.provide::<u32>(
        Provider::root_async(move |_| {
            let call = calls.fetch_add(1, Ordering::SeqCst);
            async move {
                if call == 0 {
                    std::future::pending::<()>().await;
                }
                Shared::new(7)
            }
        })
        .with_limits(fluxdi::Limits::deny(1)),
    );
    let waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut first = Box::pin(injector.try_resolve_async::<u32>());
    let mut waiter = Box::pin(injector.try_resolve_async::<u32>());
    assert!(first.as_mut().poll(&mut cx).is_pending());
    assert!(waiter.as_mut().poll(&mut cx).is_pending());
    drop(first);
    let value = match waiter.as_mut().poll(&mut cx) {
        Poll::Ready(Ok(value)) => value,
        _ => panic!("waiter should retry canceled initialization"),
    };
    assert!(Shared::ptr_eq(
        &value,
        &futures::executor::block_on(injector.resolve_async::<u32>())
    ));
}

#[tokio::test]
async fn async_decorators_execute_in_order_and_result_is_cached() {
    let injector = Injector::root();
    injector.provide::<u32>(
        Provider::root_async(|_| async { Shared::new(5) })
            .with_decorator(|v| Shared::new(*v + 1))
            .with_decorator(|v| Shared::new(*v * 2)),
    );
    let a = injector.resolve_async::<u32>().await;
    assert_eq!(*a, 12);
    assert!(Shared::ptr_eq(&a, &injector.resolve::<u32>()));
}

#[tokio::test]
async fn recursive_async_resolution_still_reports_a_cycle_after_suspension() {
    let injector = Injector::root();
    injector.provide::<u32>(Provider::root_async(|inj| async move {
        tokio::task::yield_now().await;
        let error = inj.try_resolve_async::<u32>().await.unwrap_err();
        assert_eq!(error.kind, ErrorKind::CircularDependency);
        Shared::new(7)
    }));
    assert_eq!(*injector.resolve_async::<u32>().await, 7);
}

#[cfg(feature = "thread-safe")]
#[test]
fn resolution_future_can_move_between_threads() {
    let injector = Injector::root();
    injector.provide::<u32>(Provider::transient_async(|_| async {
        let mut yielded = false;
        futures::future::poll_fn(move |cx| {
            if yielded {
                Poll::Ready(())
            } else {
                yielded = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await;
        Shared::new(7)
    }));
    let mut future = Box::pin(async move { injector.try_resolve_async::<u32>().await });
    let waker = futures::task::noop_waker();
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    let value = std::thread::spawn(move || futures::executor::block_on(future))
        .join()
        .unwrap()
        .unwrap();
    assert_eq!(*value, 7);
}

#[tokio::test]
async fn concurrent_initializers_detect_a_wait_cycle_instead_of_deadlocking() {
    #[derive(Debug)]
    struct A(bool);
    #[derive(Debug)]
    struct B(bool);
    let injector = Injector::root();
    injector.provide::<A>(Provider::root_async(|inj| async move {
        tokio::task::yield_now().await;
        let detected = match inj.try_resolve_async::<B>().await {
            Ok(b) => b.0,
            Err(error) => error.kind == ErrorKind::CircularDependency,
        };
        Shared::new(A(detected))
    }));
    injector.provide::<B>(Provider::root_async(|inj| async move {
        tokio::task::yield_now().await;
        let detected = match inj.try_resolve_async::<A>().await {
            Ok(a) => a.0,
            Err(error) => error.kind == ErrorKind::CircularDependency,
        };
        Shared::new(B(detected))
    }));
    let (a, b) = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        tokio::join!(injector.resolve_async::<A>(), injector.resolve_async::<B>())
    })
    .await
    .expect("cycle must not deadlock");
    assert!(a.0 && b.0);
}

#[cfg(feature = "thread-safe")]
#[test]
fn mixed_sync_and_async_resolve_share_initialization_without_blocking_the_executor() {
    use std::sync::{Arc, Mutex, mpsc};
    for async_first in [false, true] {
        let injector = Arc::new(Injector::root());
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let release_rx = Mutex::new(release_rx);
        injector.provide::<u32>(Provider::root(move |_| {
            started_tx.send(()).unwrap();
            release_rx.lock().unwrap().recv().unwrap();
            Shared::new(7)
        }));
        let first = {
            let injector = injector.clone();
            std::thread::spawn(move || {
                if async_first {
                    futures::executor::block_on(injector.try_resolve_async::<u32>())
                } else {
                    injector.try_resolve::<u32>()
                }
            })
        };
        started_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        if async_first {
            let error = injector.try_resolve::<u32>().unwrap_err();
            release_tx.send(()).unwrap();
            let value = first.join().unwrap().unwrap();
            assert_eq!(error.kind, ErrorKind::AsyncFactoryRequiresAsyncResolve);
            assert!(Shared::ptr_eq(&value, &injector.resolve::<u32>()));
        } else {
            let mut waiter = Box::pin(injector.try_resolve_async::<u32>());
            let waker = futures::task::noop_waker();
            assert!(
                waiter
                    .as_mut()
                    .poll(&mut Context::from_waker(&waker))
                    .is_pending()
            );
            release_tx.send(()).unwrap();
            let value = first.join().unwrap().unwrap();
            assert!(Shared::ptr_eq(
                &value,
                &futures::executor::block_on(waiter).unwrap()
            ));
        }
    }
}
