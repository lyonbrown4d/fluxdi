use fluxdi::{Injector, Provider, Shared};

#[test]
fn child_binding_shadows_parent_cache_for_single_and_named_services() {
    let parent = Shared::new(Injector::root());
    parent.provide::<u32>(Provider::root(|_| Shared::new(1)));
    parent.provide_named::<u32>("main", Provider::root(|_| Shared::new(1)));
    parent.resolve::<u32>();
    parent.resolve_named::<u32>("main");
    let child = Injector::child(parent);
    child.provide::<u32>(Provider::singleton(|_| Shared::new(2)));
    child.provide_named::<u32>("main", Provider::singleton(|_| Shared::new(2)));
    assert_eq!(*child.resolve::<u32>(), 2);
    assert_eq!(*child.resolve_named::<u32>("main"), 2);
}

#[test]
fn new_scopes_do_not_inherit_scoped_cache_for_single_named_or_set() {
    let root = Injector::root();
    root.provide::<u32>(Provider::scoped(|_| Shared::new(1)));
    root.provide_named::<u32>("main", Provider::scoped(|_| Shared::new(1)));
    root.provide_into_set::<u32>(Provider::scoped(|_| Shared::new(1)));
    let outer = root.create_scope();
    let a = outer.resolve::<u32>();
    let named_a = outer.resolve_named::<u32>("main");
    let set_a = outer.resolve_all::<u32>();
    let inner = outer.create_scope();
    assert!(!Shared::ptr_eq(&a, &inner.resolve::<u32>()));
    assert!(!Shared::ptr_eq(
        &named_a,
        &inner.resolve_named::<u32>("main")
    ));
    assert!(!Shared::ptr_eq(&set_a[0], &inner.resolve_all::<u32>()[0]));
    let child = Injector::child(Shared::new(outer.clone()));
    assert!(Shared::ptr_eq(&a, &child.resolve::<u32>()));
}

#[test]
fn singleton_cache_belongs_to_calling_context() {
    let parent = Shared::new(Injector::root());
    parent.provide::<u32>(Provider::singleton(|_| Shared::new(1)));
    let a = parent.resolve::<u32>();
    assert!(Shared::ptr_eq(&a, &parent.clone().resolve::<u32>()));
    let child = Injector::child(parent);
    let b = child.resolve::<u32>();
    assert!(!Shared::ptr_eq(&a, &b));
    assert!(Shared::ptr_eq(&b, &child.resolve::<u32>()));
}

#[test]
fn named_binding_can_depend_on_another_binding_of_the_same_type() {
    let injector = Injector::root();
    injector.provide_named::<u32>("base", Provider::transient(|_| Shared::new(7)));
    injector.provide_named::<u32>(
        "decorated",
        Provider::transient(|inj| Shared::new(*inj.resolve_named::<u32>("base") + 1)),
    );
    assert_eq!(*injector.resolve_named::<u32>("decorated"), 8);
}

#[cfg(feature = "thread-safe")]
#[test]
fn concurrent_first_resolves_share_one_initialization() {
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    };
    for scope in [
        fluxdi::Scope::Root,
        fluxdi::Scope::Module,
        fluxdi::Scope::Scoped,
    ] {
        let injector = Arc::new(Injector::root().create_scope());
        let created = Arc::new(AtomicUsize::new(0));
        let mut provider = Provider::singleton({
            let created = created.clone();
            move |_| {
                created.fetch_add(1, Ordering::SeqCst);
                Arc::new(7u32)
            }
        });
        provider.scope = scope;
        injector.provide::<u32>(provider);
        let start = Arc::new(Barrier::new(16));
        let handles: Vec<_> = (0..16)
            .map(|_| {
                let injector = injector.clone();
                let start = start.clone();
                std::thread::spawn(move || {
                    start.wait();
                    injector.resolve::<u32>()
                })
            })
            .collect();
        let values: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(created.load(Ordering::SeqCst), 1, "scope={scope}");
        assert!(values.iter().all(|value| Arc::ptr_eq(&values[0], value)));
    }
}

#[test]
fn panicking_initializer_can_be_retried() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let injector = Injector::root();
    let first = Arc::new(AtomicBool::new(true));
    injector.provide::<u32>(Provider::root_with_limits(
        fluxdi::Limits::deny(1),
        move |_| {
            assert!(!first.swap(false, Ordering::SeqCst), "first attempt fails");
            Shared::new(7)
        },
    ));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| injector.resolve::<u32>()))
            .is_err()
    );
    assert_eq!(*injector.resolve::<u32>(), 7);
}

#[cfg(feature = "thread-safe")]
#[test]
fn initializers_on_different_threads_detect_a_wait_cycle() {
    use std::sync::{Arc, Barrier, mpsc};
    #[derive(Debug)]
    struct A(bool);
    #[derive(Debug)]
    struct B(bool);
    let injector = Arc::new(Injector::root());
    let started = Arc::new(Barrier::new(2));
    injector.provide::<A>(Provider::root({
        let started = started.clone();
        move |inj| {
            started.wait();
            let detected = match inj.try_resolve::<B>() {
                Ok(b) => b.0,
                Err(error) => error.kind == fluxdi::ErrorKind::CircularDependency,
            };
            Shared::new(A(detected))
        }
    }));
    injector.provide::<B>(Provider::root(move |inj| {
        started.wait();
        let detected = match inj.try_resolve::<A>() {
            Ok(a) => a.0,
            Err(error) => error.kind == fluxdi::ErrorKind::CircularDependency,
        };
        Shared::new(B(detected))
    }));
    let (tx, rx) = mpsc::channel();
    let a = {
        let injector = injector.clone();
        let tx = tx.clone();
        std::thread::spawn(move || tx.send(injector.resolve::<A>().0).unwrap())
    };
    let b = std::thread::spawn(move || tx.send(injector.resolve::<B>().0).unwrap());
    for _ in 0..2 {
        assert!(
            rx.recv_timeout(std::time::Duration::from_secs(2))
                .expect("cycle must not deadlock")
        );
    }
    a.join().unwrap();
    b.join().unwrap();
}
