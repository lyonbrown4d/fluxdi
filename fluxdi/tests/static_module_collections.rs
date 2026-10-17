#![cfg(feature = "macros")]

use fluxdi::{Error, ErrorKind, Shared};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
}

struct First;
struct Last;

impl Plugin for First {
    fn name(&self) -> &'static str {
        "first"
    }
}
impl Plugin for Last {
    fn name(&self) -> &'static str {
        "last"
    }
}

fn first() -> Result<Shared<First>, Error> {
    Ok(Shared::new(First))
}
fn last() -> Result<Shared<Last>, Error> {
    Ok(Shared::new(Last))
}
fn plugins(values: Vec<Shared<dyn Plugin>>) -> Result<Shared<Vec<Shared<dyn Plugin>>>, Error> {
    Ok(Shared::new(values))
}
fn maybe(value: Option<Shared<dyn Plugin>>) -> Result<Shared<Option<Shared<dyn Plugin>>>, Error> {
    Ok(Shared::new(value))
}

fluxdi::static_module! {
    struct Source {
        #[set(source_only)]
        external imported: dyn Plugin;
        external untagged: dyn Plugin;
    }
}

fluxdi::static_module! {
    struct App {
        #[set(plugins, secondary)]
        first: First = first();
        import source: Source {
            #[set(plugins)]
            middle: dyn Plugin = imported;
            ignored: dyn Plugin = untagged;
        };
        #[set(plugins)]
        external supplied: dyn Plugin;
        #[set(plugins)]
        last: Last = last();
        selected: Vec<Shared<dyn Plugin>> = plugins(all(plugins));
        secondary: Vec<Shared<dyn Plugin>> = plugins(all(secondary));
        empty: Vec<Shared<dyn Plugin>> = plugins(all(source_only));
        present: Option<Shared<dyn Plugin>> = maybe(optional(first));
        absent: Option<Shared<dyn Plugin>> = maybe(optional(never_declared));
    }
}

#[test]
fn collections_follow_declaration_order_and_use_only_explicit_import_memberships() {
    let middle: Shared<dyn Plugin> = Shared::new(First);
    let ignored: Shared<dyn Plugin> = Shared::new(Last);
    let supplied: Shared<dyn Plugin> = Shared::new(Last);
    let app = App::new(Source::new(middle.clone(), ignored), supplied.clone());
    let selected = app.selected().unwrap();
    assert_eq!(
        selected
            .iter()
            .map(|value| value.name())
            .collect::<Vec<_>>(),
        ["first", "first", "last", "last"]
    );
    let first: Shared<dyn Plugin> = app.first().unwrap();
    let last: Shared<dyn Plugin> = app.last().unwrap();
    assert!(Shared::ptr_eq(&selected[0], &first));
    assert!(Shared::ptr_eq(&selected[1], &middle));
    assert!(Shared::ptr_eq(&selected[2], &supplied));
    assert!(Shared::ptr_eq(&selected[3], &last));
    assert!(Shared::ptr_eq(&app.secondary().unwrap()[0], &first));
    assert!(Shared::ptr_eq(
        app.present().unwrap().as_ref().as_ref().unwrap(),
        &first
    ));
    assert!(app.absent().unwrap().is_none());
    assert!(app.empty().unwrap().is_empty());
}

fn identity(value: Shared<u32>) -> Result<Shared<u32>, Error> {
    Ok(value)
}
fn combine(first: Shared<u32>, second: Shared<u32>) -> Result<Shared<u32>, Error> {
    Ok(Shared::new(*first + *second))
}

fluxdi::static_module! {
    struct OrdinaryKeys {
        external optional: u32;
        external all: u32;
        result: u32 = combine(optional, all);
    }
}

#[test]
fn optional_and_all_remain_legal_required_binding_keys() {
    let app = OrdinaryKeys::new(Shared::new(2), Shared::new(3));
    assert_eq!(*app.result().unwrap(), 5);
}

fn good(events: Shared<Mutex<Vec<&'static str>>>) -> Result<Shared<u32>, Error> {
    events.lock().unwrap().push("good");
    Ok(Shared::new(1))
}
fn fail(events: Shared<Mutex<Vec<&'static str>>>) -> Result<Shared<u32>, Error> {
    events.lock().unwrap().push("fail");
    Err(Error::factory_failed("member", "original failure"))
}
fn after(events: Shared<Mutex<Vec<&'static str>>>) -> Result<Shared<u32>, Error> {
    events.lock().unwrap().push("after");
    Ok(Shared::new(3))
}
fn consume(
    calls: Shared<AtomicUsize>,
    _optional: Option<Shared<u32>>,
    _members: Vec<Shared<u32>>,
) -> Result<Shared<u32>, Error> {
    calls.fetch_add(1, Ordering::SeqCst);
    Ok(Shared::new(0))
}

fluxdi::static_module! {
    struct Failures {
        external events: Mutex<Vec<&'static str>>;
        external calls: AtomicUsize;
        #[set(members)] good: u32 = good(events);
        #[set(members)] failure: u32 = fail(events);
        #[set(members)] after: u32 = after(events);
        optional_result: u32 = consume(calls, optional(failure), all(unknown));
        collection_result: u32 = consume(calls, optional(good), all(members));
        required_result: u32 = identity(failure);
    }
}

#[test]
fn present_optional_errors_propagate_and_collection_stops_at_first_failure() {
    let events = Shared::new(Mutex::new(Vec::new()));
    let calls = Shared::new(AtomicUsize::new(0));
    let app = Failures::new(events.clone(), calls.clone());
    let optional_error = app.optional_result().unwrap_err();
    assert_eq!(optional_error.kind, ErrorKind::FactoryFailed);
    assert!(optional_error.message.contains("original failure"));
    assert_eq!(*events.lock().unwrap(), ["fail"]);
    events.lock().unwrap().clear();
    let collection_error = app.collection_result().unwrap_err();
    assert_eq!(collection_error.kind, optional_error.kind);
    assert_eq!(collection_error.message, optional_error.message);
    assert_eq!(*events.lock().unwrap(), ["good", "fail"]);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        app.required_result().unwrap_err().message,
        optional_error.message
    );
}

fn counted(calls: Shared<AtomicUsize>) -> Result<Shared<usize>, Error> {
    Ok(Shared::new(calls.fetch_add(1, Ordering::SeqCst)))
}
fn retain(values: Vec<Shared<usize>>) -> Result<Shared<Vec<Shared<usize>>>, Error> {
    Ok(Shared::new(values))
}

fn empty_root(
    optional: Option<Shared<usize>>,
    members: Vec<Shared<usize>>,
) -> Result<Shared<usize>, Error> {
    assert!(optional.is_none());
    assert!(members.is_empty());
    Ok(Shared::new(0))
}

fluxdi::static_module! {
    struct Scopes {
        external calls: AtomicUsize;
        #[scope(root)] empty: usize = empty_root(optional(absent), all(unknown));
        #[set(values)] #[scope(root)] root: usize = counted(calls);
        #[set(values)] #[scope(transient)] fresh: usize = counted(calls);
        #[scope(transient)] values: Vec<Shared<usize>> = retain(all(values));
    }
}

#[test]
fn each_consumer_gets_a_vector_while_member_scope_and_cache_identity_are_preserved() {
    let calls = Shared::new(AtomicUsize::new(0));
    let app = Scopes::new(calls.clone());
    assert_eq!(*app.empty().unwrap(), 0);
    let first = app.values().unwrap();
    let second = app.create_child().values().unwrap();
    assert!(!Shared::ptr_eq(&first, &second));
    assert!(Shared::ptr_eq(&first[0], &second[0]));
    assert!(!Shared::ptr_eq(&first[1], &second[1]));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}
