#![cfg(all(feature = "macros", feature = "async-factory"))]

use fluxdi::{Error, ErrorKind, Shared};
use std::{
    future::Future,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll},
};

async fn load(calls: Shared<AtomicUsize>) -> Result<Shared<u32>, Error> {
    calls.fetch_add(1, Ordering::SeqCst);
    tokio::task::yield_now().await;
    Ok(Shared::new(40))
}

fn increment(value: Shared<u32>) -> Result<Shared<u32>, Error> {
    Ok(Shared::new(*value + 1))
}

fluxdi::static_module! {
    struct Data {
        external calls: AtomicUsize;
        #[scope(root)]
        async value: u32 = load(calls);
        promoted: u32 = increment(value);
        #[scope(transient)]
        async fresh: u32 = load(calls);
    }
}

fluxdi::static_module! {
    struct App {
        import data: Data {
            async selected: u32 = promoted;
            async root: u32 = value;
            async fresh: u32;
        };
        result: u32 = increment(selected);
    }
}

fluxdi::static_module! {
    struct Reexport {
        import app: App { async value: u32 = result; };
        result: u32 = increment(value);
    }
}

#[tokio::test]
async fn mixed_factories_are_lazy_and_promote_transitively_across_imports() {
    let calls = Shared::new(AtomicUsize::new(0));
    let data = Data::new(calls.clone());
    let app = App::new(data);
    let reexport = Reexport::new(app.clone()).in_context(app.injector());
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let cloned = app.clone();
    let (first, second) = tokio::join!(app.result(), cloned.result());
    let first = first.unwrap();
    assert_eq!(*first, 42);
    assert!(Shared::ptr_eq(&first, &second.unwrap()));
    assert_eq!(*reexport.result().await.unwrap(), 43);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(Shared::ptr_eq(
        &app.root().await.unwrap(),
        &app.create_child().root().await.unwrap()
    ));

    let (first, second) = tokio::join!(app.fresh(), app.fresh());
    assert!(!Shared::ptr_eq(&first.unwrap(), &second.unwrap()));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

async fn retry(calls: Shared<AtomicUsize>) -> Result<Shared<u32>, Error> {
    match calls.fetch_add(1, Ordering::SeqCst) {
        0 => std::future::pending().await,
        1 => Err(Error::factory_failed("retry", "temporary outage")),
        _ => Ok(Shared::new(41)),
    }
}

fluxdi::static_module! {
    struct Retry {
        external calls: AtomicUsize;
        async value: u32 = retry(calls);
        result: u32 = increment(value);
    }
}

#[test]
fn canceled_initialization_and_factory_errors_retry_without_poisoning_the_cache() {
    let calls = Shared::new(AtomicUsize::new(0));
    let app = Retry::new(calls.clone());
    let waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut owner = Box::pin(app.result());
    let mut waiter = Box::pin(app.result());
    assert!(owner.as_mut().poll(&mut cx).is_pending());
    assert!(waiter.as_mut().poll(&mut cx).is_pending());
    drop(owner);
    let error = match waiter.as_mut().poll(&mut cx) {
        Poll::Ready(Err(error)) => error,
        _ => panic!("waiter should retry cancellation and receive the factory error"),
    };
    assert_eq!(error.kind, ErrorKind::FactoryFailed);
    assert!(error.message.contains("temporary outage"));
    let value = futures::executor::block_on(app.result()).unwrap();
    assert_eq!(*value, 42);
    assert!(Shared::ptr_eq(
        &value,
        &futures::executor::block_on(app.result()).unwrap()
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[cfg(not(feature = "thread-safe"))]
#[test]
fn local_modules_accept_values_and_futures_that_are_not_send() {
    use std::rc::Rc;

    async fn local(input: Shared<Rc<u32>>) -> Result<Shared<Rc<u32>>, Error> {
        let retained = input.clone();
        futures::future::ready(()).await;
        Ok(retained)
    }
    fluxdi::static_module! {
        struct Local {
            external input: Rc<u32>;
            async value: Rc<u32> = local(input);
        }
    }
    let input = Shared::new(Rc::new(7));
    let app = Local::new(input.clone());
    assert!(Shared::ptr_eq(
        &input,
        &futures::executor::block_on(app.value()).unwrap()
    ));
}

#[cfg(feature = "thread-safe")]
#[test]
fn module_resolution_futures_can_move_between_threads() {
    fn assert_send_sync<T: Send + Sync>(_: &T) {}
    let calls = Shared::new(AtomicUsize::new(0));
    let app = App::new(Data::new(calls));
    assert_send_sync(&app);
    let mut future = Box::pin(async move { app.result().await });
    let waker = futures::task::noop_waker();
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    let result = std::thread::spawn(move || futures::executor::block_on(future))
        .join()
        .unwrap()
        .unwrap();
    assert_eq!(*result, 42);
}
