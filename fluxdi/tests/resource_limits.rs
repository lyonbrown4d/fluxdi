use fluxdi::{ErrorKind, Injector, Limits, Provider, Shared};

#[test]
fn deny_zero_limit_fails_immediately() {
    let injector = Injector::root();
    injector.provide::<u32>(Provider::transient_with_limits(Limits::deny(0), |_| {
        Shared::new(1)
    }));

    let error = injector.try_resolve::<u32>().unwrap_err();
    assert_eq!(error.kind, ErrorKind::ResourceLimitExceeded);
}

#[cfg(feature = "thread-safe")]
#[test]
fn deny_policy_rejects_when_concurrency_limit_is_reached() {
    use std::sync::{Arc, Barrier, Mutex};
    use std::thread;
    use std::time::Duration;

    let injector = Arc::new(Injector::root());
    injector.provide::<u64>(Provider::transient_with_limits(Limits::deny(1), |_| {
        thread::sleep(Duration::from_millis(20));
        Shared::new(7u64)
    }));

    let workers = 8;
    let barrier = Arc::new(Barrier::new(workers));
    let errors = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..workers)
        .map(|_| {
            let injector = Arc::clone(&injector);
            let barrier = Arc::clone(&barrier);
            let errors = Arc::clone(&errors);
            thread::spawn(move || {
                barrier.wait();
                match injector.try_resolve::<u64>() {
                    Ok(value) => Some(*value),
                    Err(error) => {
                        errors.lock().unwrap().push(error.kind);
                        None
                    }
                }
            })
        })
        .collect();

    let successes: Vec<u64> = handles
        .into_iter()
        .filter_map(|handle| handle.join().unwrap())
        .collect();

    let errors = errors.lock().unwrap();
    assert!(!successes.is_empty());
    assert!(errors.contains(&ErrorKind::ResourceLimitExceeded));
}

#[cfg(feature = "thread-safe")]
#[test]
fn block_policy_waits_until_capacity_is_available() {
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    let injector = Arc::new(Injector::root());
    injector.provide::<u32>(Provider::transient_with_limits(Limits::block(1), |_| {
        thread::sleep(Duration::from_millis(5));
        Shared::new(11u32)
    }));

    let workers = 6;
    let barrier = Arc::new(Barrier::new(workers));
    let handles: Vec<_> = (0..workers)
        .map(|_| {
            let injector = Arc::clone(&injector);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                injector.try_resolve::<u32>()
            })
        })
        .collect();

    for handle in handles {
        let value = handle.join().unwrap().unwrap();
        assert_eq!(*value, 11);
    }
}

#[cfg(feature = "thread-safe")]
#[test]
fn block_policy_with_timeout_returns_error_when_wait_expires() {
    use std::sync::{Arc, Condvar, Mutex};
    use std::thread;
    use std::time::Duration;

    let started = Arc::new((Mutex::new(false), Condvar::new()));
    let started_for_factory = Arc::clone(&started);
    let injector = Arc::new(Injector::root());
    injector.provide::<u64>(Provider::transient_with_limits(
        Limits::block_with_timeout(1, Duration::from_millis(5)),
        move |_| {
            let (lock, condvar) = &*started_for_factory;
            let mut has_started = lock.lock().unwrap();
            *has_started = true;
            condvar.notify_one();
            drop(has_started);
            thread::sleep(Duration::from_millis(30));
            Shared::new(9u64)
        },
    ));

    let first = {
        let injector = Arc::clone(&injector);
        thread::spawn(move || injector.try_resolve::<u64>())
    };

    let (lock, condvar) = &*started;
    let mut has_started = lock.lock().unwrap();
    while !*has_started {
        has_started = condvar.wait(has_started).unwrap();
    }
    drop(has_started);

    let second = injector.try_resolve::<u64>();
    let first_result = first.join().unwrap();

    assert!(first_result.is_ok());
    assert!(matches!(
        second,
        Err(error) if error.kind == ErrorKind::ResourceLimitExceeded
    ));
}

#[cfg(feature = "async-factory")]
#[test]
fn limits_apply_to_async_factories_via_with_limits() {
    use futures::executor::block_on;

    let injector = Injector::root();
    injector.provide::<String>(
        Provider::transient_async(|_| async { Shared::new("hello".to_string()) }).with_limits(
            Limits {
                max_concurrent_creations: Some(0),
                policy: fluxdi::Policy::Deny,
                timeout: None,
            },
        ),
    );

    let error = block_on(injector.try_resolve_async::<String>()).unwrap_err();
    assert_eq!(error.kind, ErrorKind::ResourceLimitExceeded);
}

#[cfg(all(
    feature = "thread-safe",
    feature = "async-factory",
    feature = "resource-limit-async"
))]
#[test]
fn async_block_policy_with_timeout_uses_non_blocking_wait() {
    use std::sync::{Arc, Condvar, Mutex};
    use std::thread;
    use std::time::Duration;

    let started = Arc::new((Mutex::new(false), Condvar::new()));
    let started_for_factory = Arc::clone(&started);

    let injector = Arc::new(Injector::root());
    injector.provide::<u64>(
        Provider::transient_async(move |_| {
            let started_for_factory = Arc::clone(&started_for_factory);
            async move {
                {
                    let (lock, condvar) = &*started_for_factory;
                    let mut has_started = lock.lock().unwrap();
                    *has_started = true;
                    condvar.notify_one();
                }

                tokio::time::sleep(Duration::from_millis(30)).await;
                Shared::new(5u64)
            }
        })
        .with_limits(Limits::block_with_timeout(1, Duration::from_millis(5))),
    );

    let first = {
        let injector = Arc::clone(&injector);
        thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .unwrap();
            runtime.block_on(async move { injector.try_resolve_async::<u64>().await })
        })
    };

    let (lock, condvar) = &*started;
    let mut has_started = lock.lock().unwrap();
    while !*has_started {
        has_started = condvar.wait(has_started).unwrap();
    }
    drop(has_started);

    let second = {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async { injector.try_resolve_async::<u64>().await })
    };

    let first_result = first.join().unwrap();
    assert!(first_result.is_ok(), "first result: {:?}", first_result);
    assert!(matches!(
        second,
        Err(error) if error.kind == ErrorKind::ResourceLimitExceeded
    ));
}

#[cfg(feature = "resource-limit-async")]
#[test]
fn sync_and_async_resolve_share_the_same_creation_limit() {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    };
    let injector = Arc::new(Injector::root());
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Arc::new(Mutex::new(release_rx));
    let calls = Arc::new(AtomicUsize::new(0));
    injector.provide::<u32>(Provider::transient_with_limits(
        Limits::deny(1),
        move |_| {
            if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                started_tx.send(()).unwrap();
                release_rx.lock().unwrap().recv().unwrap();
            }
            Shared::new(7)
        },
    ));
    let first = {
        let injector = injector.clone();
        std::thread::spawn(move || injector.try_resolve::<u32>())
    };
    started_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let second = futures::executor::block_on(injector.try_resolve_async::<u32>());
    release_tx.send(()).unwrap();
    first.join().unwrap().unwrap();
    assert!(matches!(second, Err(error) if error.kind == ErrorKind::ResourceLimitExceeded));
}
