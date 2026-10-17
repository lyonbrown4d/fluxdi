#![cfg(feature = "macros")]

use fluxdi::{Error, ErrorKind, Provider, Shared};
use std::sync::atomic::{AtomicUsize, Ordering};

trait Repository: Send + Sync {
    fn name(&self) -> &str;
}

type RepositoryBinding = dyn Repository;

struct MemoryRepository(String);

impl Repository for MemoryRepository {
    fn name(&self) -> &str {
        &self.0
    }
}

struct Service {
    repository: Shared<dyn Repository>,
    selected: Shared<u32>,
}

fn repository(config: Shared<String>) -> Result<Shared<dyn Repository>, Error> {
    Ok(Shared::new(MemoryRepository((*config).clone())))
}

fn alias(repository: Shared<dyn Repository>) -> Result<Shared<dyn Repository>, Error> {
    Ok(repository)
}

fn service(
    repository: Shared<dyn Repository>,
    selected: Shared<u32>,
) -> Result<Shared<Service>, Error> {
    Ok(Shared::new(Service {
        repository,
        selected,
    }))
}

fn primary() -> Result<Shared<u32>, Error> {
    Ok(Shared::new(11))
}

fn secondary() -> Result<Shared<u32>, Error> {
    Ok(Shared::new(22))
}

fluxdi::static_module! {
    struct Assembly {
        service: Service = service(repo_alias, secondary);
        repo_alias: RepositoryBinding = alias(repo);
        secondary: u32 = secondary();
        #[scope(root)]
        repo: dyn Repository = repository(config);
        primary: u32 = primary();
        external config: String;
        external label: String;
    }
}

#[test]
fn assembles_explicit_keys_trait_aliases_and_externals_out_of_order() {
    let config = Shared::new("configured repository".to_owned());
    let label = Shared::new("application label".to_owned());
    let app = Assembly::new(config.clone(), label.clone());
    let result = app.service().unwrap();

    assert_eq!(result.repository.name(), "configured repository");
    assert_eq!(*result.selected, 22);
    assert_eq!(*app.primary().unwrap(), 11);
    assert!(Shared::ptr_eq(&app.config().unwrap(), &config));
    assert!(Shared::ptr_eq(&app.label().unwrap(), &label));
    assert!(Shared::ptr_eq(
        &app.repo().unwrap(),
        &app.repo_alias().unwrap()
    ));
}

#[test]
fn typed_getters_do_not_depend_on_dynamic_registry_bindings() {
    let app = Assembly::new(
        Shared::new("repository".into()),
        Shared::new("label".into()),
    );
    app.injector()
        .provide::<u32>(Provider::root(|_| Shared::new(99)));

    assert_eq!(*app.injector().resolve::<u32>(), 99);
    assert_eq!(*app.primary().unwrap(), 11);
    assert_eq!(*app.secondary().unwrap(), 22);
    assert_eq!(*app.service().unwrap().selected, 22);
}

#[test]
fn generated_identifiers_do_not_shadow_user_factory_paths_or_raw_keys() {
    fn __fluxdi_provider_0(input: Shared<u32>) -> Result<Shared<u32>, Error> {
        Ok(Shared::new(*input + 1))
    }
    fn __fluxdi_dependency_0(input: Shared<u32>) -> Result<Shared<u32>, Error> {
        Ok(Shared::new(*input + 1))
    }
    fluxdi::static_module! {
        struct Hygienic {
            external __fluxdi_context_0: u32;
            r#type: u32 = __fluxdi_provider_0(__fluxdi_context_0);
            result: u32 = __fluxdi_dependency_0(r#type);
        }
    }

    let app = Hygienic::new(Shared::new(40));
    assert_eq!(*app.r#type().unwrap(), 41);
    assert_eq!(*app.result().unwrap(), 42);
}

fn value() -> Result<Shared<u32>, Error> {
    Ok(Shared::new(7))
}

fluxdi::static_module! {
    struct Lifetimes {
        #[scope(root)]
        root: u32 = value();
        module: u32 = value();
        #[scope(scoped)]
        scoped: u32 = value();
        #[scope(transient)]
        transient: u32 = value();
    }
}

#[test]
fn root_module_and_transient_keep_their_cache_domains() {
    let app = Lifetimes::new();
    let clone = app.clone();
    let child = app.create_child();
    let scope = child.create_scope();
    let root = app.root().unwrap();
    let module = app.module().unwrap();

    assert!(Shared::ptr_eq(&root, &child.root().unwrap()));
    assert!(Shared::ptr_eq(&root, &scope.root().unwrap()));
    assert!(Shared::ptr_eq(&module, &clone.module().unwrap()));
    assert!(!Shared::ptr_eq(&module, &child.module().unwrap()));
    assert!(!Shared::ptr_eq(
        &child.module().unwrap(),
        &scope.module().unwrap()
    ));
    assert!(Shared::ptr_eq(
        &child.module().unwrap(),
        &child.module().unwrap()
    ));
    assert!(!Shared::ptr_eq(
        &app.transient().unwrap(),
        &app.transient().unwrap()
    ));
}

#[test]
fn scoped_values_follow_the_nearest_boundary_including_bridged_contexts() {
    let app = Lifetimes::new();
    let first = app.create_scope();
    let child = first.create_child();
    let nested = child.create_scope();
    let sibling = app.create_scope();
    let first_value = first.scoped().unwrap();

    assert!(Shared::ptr_eq(&first_value, &child.scoped().unwrap()));
    assert!(!Shared::ptr_eq(&first_value, &nested.scoped().unwrap()));
    assert!(!Shared::ptr_eq(&first_value, &sibling.scoped().unwrap()));
    let bridged = app.in_context(child.injector());
    assert!(Shared::ptr_eq(&first_value, &bridged.scoped().unwrap()));
    assert!(Shared::ptr_eq(
        &app.root().unwrap(),
        &bridged.root().unwrap()
    ));
}

struct Attempts {
    dependency: AtomicUsize,
    dependent: AtomicUsize,
}

fn retry(attempts: Shared<Attempts>) -> Result<Shared<u32>, Error> {
    if attempts.dependency.fetch_add(1, Ordering::SeqCst) == 0 {
        return Err(Error::factory_failed("retry", "temporary outage"));
    }
    Ok(Shared::new(42))
}

fn dependent(value: Shared<u32>, attempts: Shared<Attempts>) -> Result<Shared<u64>, Error> {
    attempts.dependent.fetch_add(1, Ordering::SeqCst);
    Ok(Shared::new(u64::from(*value)))
}

fluxdi::static_module! {
    struct Retry {
        result: u64 = dependent(value, attempts);
        value: u32 = retry(attempts);
        external attempts: Attempts;
    }
}

#[test]
fn dependency_failure_propagates_and_success_is_cached_after_retry() {
    let attempts = Shared::new(Attempts {
        dependency: AtomicUsize::new(0),
        dependent: AtomicUsize::new(0),
    });
    let app = Retry::new(attempts.clone());

    let error = app.result().unwrap_err();
    assert_eq!(error.kind, ErrorKind::FactoryFailed);
    assert!(error.message.contains("temporary outage"));
    assert_eq!(attempts.dependent.load(Ordering::SeqCst), 0);
    let result = app.result().unwrap();
    assert_eq!(*result, 42);
    assert!(Shared::ptr_eq(&result, &app.result().unwrap()));
    assert_eq!(*app.value().unwrap(), 42);
    assert_eq!(attempts.dependency.load(Ordering::SeqCst), 2);
    assert_eq!(attempts.dependent.load(Ordering::SeqCst), 1);
}

#[test]
fn dropping_all_module_contexts_releases_factory_inputs_and_cached_values() {
    let config = Shared::new("repository".to_owned());
    let weak_config = Shared::downgrade(&config);
    let app = Assembly::new(config, Shared::new("label".into()));
    let service = app.service().unwrap();
    let weak_service = Shared::downgrade(&service);
    let weak_repository = Shared::downgrade(&service.repository);
    let child = app.create_child();
    drop(service);
    drop(app);
    assert!(weak_config.upgrade().is_some());

    drop(child);
    assert!(weak_config.upgrade().is_none());
    assert!(weak_service.upgrade().is_none());
    assert!(weak_repository.upgrade().is_none());
}

#[cfg(feature = "thread-safe")]
#[test]
fn concurrent_cloned_modules_construct_a_single_cached_value() {
    use std::sync::Barrier;

    fn counted(calls: Shared<AtomicUsize>) -> Result<Shared<u32>, Error> {
        Ok(Shared::new(calls.fetch_add(1, Ordering::SeqCst) as u32))
    }

    fluxdi::static_module! {
        struct Concurrent {
            value: u32 = counted(calls);
            external calls: AtomicUsize;
        }
    }

    let calls = Shared::new(AtomicUsize::new(0));
    let app = Concurrent::new(calls.clone());
    let start = Shared::new(Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let app = app.clone();
            let start = start.clone();
            std::thread::spawn(move || {
                start.wait();
                app.value().unwrap()
            })
        })
        .collect();
    let values: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(values.iter().all(|value| Shared::ptr_eq(value, &values[0])));
    assert_eq!(*values[0], 0);
}
