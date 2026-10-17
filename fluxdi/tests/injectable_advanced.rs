#![cfg(feature = "macros")]

use fluxdi::{
    DependencyCardinality, Error, ErrorKind, Injectable, Injector, Provider, Scope, Shared,
};

trait Backend: Send + Sync {
    fn id(&self) -> u32;
}
struct Memory(u32);
impl Backend for Memory {
    fn id(&self) -> u32 {
        self.0
    }
}

#[derive(Injectable)]
struct Pipeline<'a, T: ?Sized, const N: usize>
where
    T: Backend,
{
    default: Shared<T>,
    #[inject(name = "primary")]
    named: fluxdi::Shared<T>,
    #[inject(name = "optional")]
    optional: std::option::Option<Shared<T>>,
    plugins: std::vec::Vec<Shared<T>>,
    label: Shared<&'a str>,
    bytes: Shared<[u8; N]>,
}

#[test]
fn generics_named_optional_and_set_fields_match_runtime_and_graph_bindings() {
    type App = Pipeline<'static, dyn Backend, 2>;
    let root = Injector::root();
    root.provide::<App>(App::provider(Scope::Module));
    assert!(!root.validate_graph().is_valid());
    root.provide::<dyn Backend>(Provider::root(|_| {
        Shared::new(Memory(1)) as Shared<dyn Backend>
    }));
    root.provide_named::<dyn Backend>(
        "primary",
        Provider::root(|_| Shared::new(Memory(2)) as Shared<dyn Backend>),
    );
    root.provide_into_set::<dyn Backend>(Provider::root(|_| {
        Shared::new(Memory(3)) as Shared<dyn Backend>
    }));
    root.provide::<&'static str>(Provider::root(|_| Shared::new("pipeline")));
    root.provide::<[u8; 2]>(Provider::root(|_| Shared::new([7, 8])));
    let child = Injector::child(Shared::new(root));
    child.provide_into_set::<dyn Backend>(Provider::singleton(|_| {
        Shared::new(Memory(4)) as Shared<dyn Backend>
    }));
    assert!(
        child.validate_graph().is_valid(),
        "absent optional target must be valid"
    );
    let app = child.try_resolve::<App>().unwrap();
    assert_eq!(app.default.id(), 1);
    assert_eq!(app.named.id(), 2);
    assert!(app.optional.is_none());
    assert_eq!(
        app.plugins.iter().map(|p| p.id()).collect::<Vec<_>>(),
        [3, 4]
    );
    assert_eq!(*app.label, "pipeline");
    assert_eq!(*app.bytes, [7, 8]);
    child.provide_named::<dyn Backend>(
        "optional",
        Provider::singleton(|_| Shared::new(Memory(5)) as Shared<dyn Backend>),
    );
    // Construct directly, since the cached instance correctly retains its original fields.
    assert_eq!(
        App::try_from_injector(&child)
            .unwrap()
            .optional
            .as_ref()
            .unwrap()
            .id(),
        5
    );
    let graph = child.dependency_graph();
    let node = graph
        .nodes
        .iter()
        .find(|n| n.type_name.contains("Pipeline<"))
        .unwrap();
    assert_eq!(node.dependencies.len(), 6);
    assert!(
        node.dependencies
            .iter()
            .any(|d| d.cardinality == DependencyCardinality::Optional
                && d.name.as_deref() == Some("optional"))
    );
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.from == node.id && e.label.as_deref() == Some("one:primary"))
    );
    assert_eq!(
        graph
            .edges
            .iter()
            .filter(|e| e.from == node.id && e.label.as_deref() == Some("all"))
            .count(),
        2
    );
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.from == node.id && e.label.as_deref() == Some("optional:optional"))
    );
}

#[derive(Injectable)]
struct Maybe {
    value: Option<Shared<u32>>,
    #[inject(name = "secondary")]
    named: Option<Shared<u64>>,
}

#[test]
fn optional_injection_distinguishes_absence_from_nested_and_factory_errors() {
    let injector = Injector::root();
    let empty = Maybe::try_from_injector(&injector).unwrap();
    assert!(empty.value.is_none() && empty.named.is_none());
    injector.provide::<u32>(Provider::try_root(|inj| {
        inj.try_resolve::<bool>()?;
        Ok(Shared::new(7))
    }));
    assert_eq!(
        Maybe::try_from_injector(&injector).err().unwrap().kind,
        ErrorKind::ServiceNotProvided
    );
    injector.provide::<bool>(Provider::root(|_| Shared::new(true)));
    injector.provide_named::<u64>(
        "secondary",
        Provider::try_root(|_| Err(Error::factory_failed("u64", "offline"))),
    );
    let error = Maybe::try_from_injector(&injector).err().unwrap();
    assert_eq!(error.kind, ErrorKind::FactoryFailed);
    assert!(error.message.contains("offline"));
}

#[derive(Injectable)]
struct Plugins {
    values: Vec<Shared<u32>>,
}

#[test]
fn set_injection_requires_a_binding_and_preserves_initialization_errors() {
    let injector = Injector::root();
    injector.provide::<Plugins>(Plugins::provider(Scope::Transient));
    assert!(!injector.validate_graph().is_valid());
    assert_eq!(
        injector.try_resolve::<Plugins>().err().unwrap().kind,
        ErrorKind::ServiceNotProvided
    );
    injector.provide_into_set::<u32>(Provider::root(|_| Shared::new(7)));
    assert_eq!(*injector.resolve::<Plugins>().values[0], 7);
    injector.provide_into_set::<u32>(Provider::try_root(|_| {
        Err(Error::factory_failed("plugin", "failed"))
    }));
    assert!(injector.validate_graph().is_valid());
    assert_eq!(
        injector.try_resolve::<Plugins>().err().unwrap().kind,
        ErrorKind::FactoryFailed
    );
}

#[test]
fn optional_graph_edges_detect_cycles_when_the_target_is_registered() {
    #[allow(
        dead_code,
        reason = "the deliberately cyclic services cannot be constructed"
    )]
    #[derive(Injectable)]
    struct A {
        b: Option<Shared<B>>,
    }
    #[allow(
        dead_code,
        reason = "the deliberately cyclic services cannot be constructed"
    )]
    #[derive(Injectable)]
    struct B {
        a: Shared<A>,
    }
    let injector = Injector::root();
    injector.provide::<A>(A::provider(Scope::Root));
    assert!(injector.validate_graph().is_valid());
    injector.provide::<B>(B::provider(Scope::Root));
    assert!(
        injector
            .validate_graph()
            .issues
            .iter()
            .any(|i| i.kind == fluxdi::GraphValidationIssueKind::CircularDependency)
    );
    assert_eq!(
        injector.try_resolve::<A>().err().unwrap().kind,
        ErrorKind::CircularDependency
    );
}

#[derive(Injectable)]
struct RequiredNamed {
    #[inject(name = "main")]
    value: Shared<u32>,
}

#[test]
fn named_hints_do_not_fall_back_to_an_unnamed_binding() {
    let injector = Injector::root();
    injector.provide::<RequiredNamed>(RequiredNamed::provider(Scope::Root));
    injector.provide::<u32>(Provider::root(|_| Shared::new(1)));
    assert!(!injector.validate_graph().is_valid());
    assert_eq!(
        injector.try_resolve::<RequiredNamed>().err().unwrap().kind,
        ErrorKind::ServiceNotProvided
    );
    injector.provide_named::<u32>("main", Provider::root(|_| Shared::new(2)));
    assert!(injector.validate_graph().is_valid());
    assert_eq!(*injector.resolve::<RequiredNamed>().value, 2);
}

#[test]
fn inferred_hints_can_be_attached_to_a_custom_factory_in_any_scope() {
    for scope in [Scope::Root, Scope::Module, Scope::Scoped, Scope::Transient] {
        assert_eq!(RequiredNamed::provider(scope).scope, scope);
    }
    let injector = Injector::root();
    injector.provide::<RequiredNamed>(RequiredNamed::with_injected_dependencies(
        Provider::try_transient(|_| {
            Ok(Shared::new(RequiredNamed {
                value: Shared::new(9),
            }))
        }),
    ));
    assert!(!injector.validate_graph().is_valid());
    injector.provide_named::<u32>("main", Provider::root(|_| Shared::new(2)));
    assert!(injector.validate_graph().is_valid());
    assert_eq!(*injector.resolve::<RequiredNamed>().value, 9);
}

#[cfg(not(feature = "thread-safe"))]
#[test]
fn generic_injection_keeps_non_send_services_in_local_runtime() {
    #[derive(Injectable)]
    struct Local<T> {
        value: Shared<T>,
    }
    let injector = Injector::root();
    injector
        .provide::<std::cell::Cell<u32>>(Provider::root(|_| Shared::new(std::cell::Cell::new(7))));
    injector.provide::<Local<std::cell::Cell<u32>>>(Local::provider(Scope::Root));
    assert_eq!(
        injector
            .resolve::<Local<std::cell::Cell<u32>>>()
            .value
            .get(),
        7
    );
}

#[test]
#[allow(
    non_upper_case_globals,
    reason = "exercise const parameter collisions with generated bindings"
)]
fn const_parameters_do_not_shadow_generated_constructor_or_provider_bindings() {
    #[derive(Injectable)]
    struct Names<
        const injector: usize,
        const provider: usize,
        const scope: usize,
        const r#__fluxdi_injector: usize,
    > {
        a: Shared<[u8; injector]>,
        b: Shared<[u16; provider]>,
        c: Shared<[u32; scope]>,
        d: Shared<[u64; __fluxdi_injector]>,
    }
    let injector = Injector::root();
    injector.provide::<[u8; 1]>(Provider::root(|_| Shared::new([1])));
    injector.provide::<[u16; 2]>(Provider::root(|_| Shared::new([2; 2])));
    injector.provide::<[u32; 3]>(Provider::root(|_| Shared::new([3; 3])));
    injector.provide::<[u64; 4]>(Provider::root(|_| Shared::new([4; 4])));
    injector.provide::<Names<1, 2, 3, 4>>(Names::provider(Scope::Root));
    assert!(injector.validate_graph().is_valid());
    let value = injector.resolve::<Names<1, 2, 3, 4>>();
    assert_eq!(*value.a, [1]);
    assert_eq!(*value.b, [2; 2]);
    assert_eq!(*value.c, [3; 3]);
    assert_eq!(*value.d, [4; 4]);
}
