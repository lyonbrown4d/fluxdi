#![cfg(feature = "macros")]

use fluxdi::{Injectable, Injector, Provider, Shared};

#[derive(Debug)]
struct Clock {
    tick: u64,
}

#[derive(Injectable)]
struct AppService {
    clock: Shared<Clock>,
}

#[test]
fn derive_injectable_resolves_shared_dependencies() {
    let injector = Injector::root();
    injector.provide::<Clock>(Provider::root(|_| Shared::new(Clock { tick: 42 })));
    injector.provide::<AppService>(Provider::root(AppService::from_injector));

    let service = injector.resolve::<AppService>();
    assert_eq!(service.clock.tick, 42);
}

#[test]
fn derive_fallible_constructor_propagates_missing_dependencies_and_recovers() {
    let injector = Injector::root();
    injector.provide::<AppService>(Provider::try_root(AppService::try_from_injector));
    let error = injector
        .try_resolve::<AppService>()
        .err()
        .expect("missing Clock must return an error");
    assert_eq!(error.kind, fluxdi::ErrorKind::ServiceNotProvided);
    assert!(error.message.contains("Clock"));
    injector.provide::<Clock>(Provider::root(|_| Shared::new(Clock { tick: 99 })));
    assert_eq!(injector.try_resolve::<AppService>().unwrap().clock.tick, 99);
}
