#![cfg(feature = "macros")]

use fluxdi::{Error, Shared};

fn number() -> Result<Shared<u32>, Error> {
    Ok(Shared::new(7))
}

fn summarize(
    config: Option<Shared<String>>,
    values: Vec<Shared<u32>>,
) -> Result<Shared<String>, Error> {
    Ok(Shared::new(format!(
        "{}:{}",
        config.as_deref().map(String::as_str).unwrap_or("none"),
        values.iter().map(|value| **value).sum::<u32>()
    )))
}

fluxdi::static_module! {
    #[cfg(any())]
    struct RemovedModule {
        #[scope(invalid)]
        bad: UnresolvedType = unresolved(absent);
    }
}

fluxdi::static_module! {
    #[cfg_attr(all(), doc = "Compiler-selected module")]
    struct Conditional {
        #[cfg(false)]
        external unused: UnknownExternal;
        #[cfg(any())]
        import absent: nonexistent::Module { missing: UnknownImport; };
        #[cfg(any())]
        #[scope(invalid)]
        duplicate: UnknownType = nonexistent(absent);
        #[cfg(any())]
        duplicate: u32 = number();
        #[cfg_attr(all(), cfg(any()))]
        async discarded_async: Missing = absent(discarded_async);
        #[cfg(any())]
        first: u32 = absent(second);
        #[cfg(any())]
        second: u32 = absent(first);
        #[cfg(feature = "debug")]
        external config: String;
        #[cfg(true)]
        #[cfg_attr(true, scope(root))]
        #[cfg_attr(any(), scope(invalid))]
        #[cfg_attr(feature = "debug", set(values))]
        value: u32 = number();
        #[cfg_attr(feature = "debug", scope(root))]
        changed_scope: u32 = number();
        #[cfg(feature = "debug")]
        alternative: u32 = number();
        #[cfg(not(feature = "debug"))]
        alternative: u32 = number();
        summary: String = summarize(optional(config), all(values));
    }
}

#[test]
fn compiler_filters_before_validation_and_changes_the_actual_constructor_and_group() {
    #[cfg(feature = "debug")]
    let app = Conditional::new(Shared::new("debug".into()));
    #[cfg(not(feature = "debug"))]
    let app = Conditional::new();
    #[cfg(feature = "debug")]
    assert_eq!(&*app.summary().unwrap(), "debug:7");
    #[cfg(not(feature = "debug"))]
    assert_eq!(&*app.summary().unwrap(), "none:0");
    assert_eq!(*app.alternative().unwrap(), 7);
    assert_eq!(
        Shared::ptr_eq(
            &app.changed_scope().unwrap(),
            &app.create_child().changed_scope().unwrap()
        ),
        cfg!(feature = "debug")
    );
    assert!(Shared::ptr_eq(
        &app.value().unwrap(),
        &app.create_child().value().unwrap()
    ));
}

fluxdi::static_module! {
    struct Source { external value: u32; }
}

fluxdi::static_module! {
    struct SelectedImports {
        #[cfg_attr(any(), cfg(any()))]
        import source: Source {
            #[cfg(any())]
            #[set(values)]
            #[scope(invalid)]
            missing: UnresolvedType;
            #[cfg_attr(all(), cfg(all()), set(values))]
            selected: u32 = value;
        };
        summary: String = summarize(optional(never_declared), all(values));
    }
}

#[test]
fn selected_import_conditions_filter_types_and_preserve_selected_membership() {
    let app = SelectedImports::new(Source::new(Shared::new(9)));
    assert_eq!(&*app.summary().unwrap(), "none:9");
}

#[cfg(feature = "async-factory")]
mod asynchronous {
    use super::*;

    #[cfg(feature = "debug")]
    async fn load() -> Result<Shared<u32>, Error> {
        Ok(Shared::new(11))
    }

    fn optional_value(value: Option<Shared<u32>>) -> Result<Shared<u32>, Error> {
        Ok(value.unwrap_or_else(|| Shared::new(0)))
    }

    fn sum(values: Vec<Shared<u32>>) -> Result<Shared<u32>, Error> {
        Ok(Shared::new(values.iter().map(|value| **value).sum()))
    }

    fluxdi::static_module! {
        struct ConditionalAsync {
            #[cfg(feature = "debug")]
            #[set(values)]
            async value: u32 = load();
            optional_result: u32 = optional_value(optional(value));
            group_result: u32 = sum(all(values));
        }
    }

    #[test]
    fn selected_optional_and_group_edges_change_inferred_getter_asyncness() {
        let app = ConditionalAsync::new();
        #[cfg(feature = "debug")]
        let (optional, group) = (
            futures::executor::block_on(app.optional_result()).unwrap(),
            futures::executor::block_on(app.group_result()).unwrap(),
        );
        #[cfg(not(feature = "debug"))]
        let (optional, group) = (app.optional_result().unwrap(), app.group_result().unwrap());
        assert_eq!(*optional, if cfg!(feature = "debug") { 11 } else { 0 });
        assert_eq!(*group, *optional);
    }
}
