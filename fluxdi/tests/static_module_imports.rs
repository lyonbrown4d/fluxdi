#![cfg(feature = "macros")]

use fluxdi::{Error, Shared};
use std::sync::atomic::{AtomicUsize, Ordering};

fn counted(calls: Shared<AtomicUsize>) -> Result<Shared<usize>, Error> {
    Ok(Shared::new(calls.fetch_add(1, Ordering::SeqCst)))
}

fn identity(value: Shared<usize>) -> Result<Shared<usize>, Error> {
    Ok(value)
}

mod source {
    use super::*;

    fluxdi::static_module! {
        pub struct Data {
            external calls: AtomicUsize;
            external unused: String;
            value: usize = counted(calls);
            #[scope(root)]
            root: usize = counted(calls);
            #[scope(transient)]
            root_bridge: usize = identity(root);
            #[scope(scoped)]
            scoped: usize = counted(calls);
            #[scope(transient)]
            transient: usize = counted(calls);
        }
    }
}

fn combine(first: Shared<usize>, second: Shared<usize>) -> Result<Shared<usize>, Error> {
    Ok(Shared::new(*first + *second))
}

fluxdi::static_module! {
    struct App {
        import first: crate::source::Data {
            selected: usize = value;
            root: usize;
            root_bridge: usize;
            scoped: usize;
            transient: usize;
        };
        external label: String;
        import second: crate::source::Data {
            other: usize = value;
        };
        sum: usize = combine(selected, other);
        #[scope(root)]
        root_sum: usize = combine(root_bridge, root);
    }
}

fluxdi::static_module! {
    struct Reexport {
        import app: App { forwarded: usize = selected; };
    }
}

fn source() -> source::Data {
    source::Data::new(
        Shared::new(AtomicUsize::new(0)),
        Shared::new("unused".into()),
    )
}

#[test]
fn selected_aliases_reexports_and_multiple_imports_preserve_provider_identity() {
    let source = source();
    let label = Shared::new("between imports".to_owned());
    let app = App::new(source.clone(), label.clone(), source);
    let selected = app.selected().unwrap();
    assert!(Shared::ptr_eq(&selected, &app.other().unwrap()));
    assert_eq!(*app.sum().unwrap(), *selected * 2);
    assert!(Shared::ptr_eq(&label, &app.label().unwrap()));

    let reexport = Reexport::new(app.clone()).in_context(app.injector());
    assert!(Shared::ptr_eq(&selected, &reexport.forwarded().unwrap()));
}

#[test]
fn imports_use_the_app_context_and_can_explicitly_rejoin_the_source_tree() {
    let source = source();
    let original = source.value().unwrap();
    let source_root = source.root().unwrap();
    let app = App::new(source.clone(), Shared::new(String::new()), source.clone());
    let fresh = app.selected().unwrap();
    assert!(!Shared::ptr_eq(&original, &fresh));
    assert!(!Shared::ptr_eq(&source_root, &app.root().unwrap()));

    let bridged = app.in_context(source.injector());
    assert!(Shared::ptr_eq(&original, &bridged.selected().unwrap()));
    assert!(Shared::ptr_eq(&source_root, &bridged.root().unwrap()));

    let child = app.create_child();
    assert!(!Shared::ptr_eq(&fresh, &child.selected().unwrap()));
    assert!(Shared::ptr_eq(&app.root().unwrap(), &child.root().unwrap()));
    let root_sum = app.root_sum().unwrap();
    assert_eq!(*root_sum, *app.root().unwrap() * 2);
    assert!(Shared::ptr_eq(&root_sum, &child.root_sum().unwrap()));
    let scope = app.create_scope();
    let scoped = scope.scoped().unwrap();
    assert!(Shared::ptr_eq(
        &scoped,
        &scope.create_child().scoped().unwrap()
    ));
    assert!(!Shared::ptr_eq(
        &scoped,
        &scope.create_scope().scoped().unwrap()
    ));
    assert!(!Shared::ptr_eq(
        &app.transient().unwrap(),
        &app.transient().unwrap()
    ));
}

#[test]
fn composing_keeps_selected_providers_without_retaining_the_source_context() {
    let unused = Shared::new("source-only input".to_owned());
    let weak_unused = Shared::downgrade(&unused);
    let source = source::Data::new(Shared::new(AtomicUsize::new(0)), unused);
    let source_value = source.value().unwrap();
    let weak_cached = Shared::downgrade(&source_value);
    drop(source_value);
    let app = App::new(source.clone(), Shared::new(String::new()), source);

    assert!(weak_unused.upgrade().is_none());
    assert!(weak_cached.upgrade().is_none());
    assert_eq!(*app.selected().unwrap(), 1);
}
