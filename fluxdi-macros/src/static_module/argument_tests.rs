use super::{Argument, Module};

#[test]
fn arguments_preserve_collection_order_and_bare_wrapper_names() {
    let module = syn::parse_str::<Module>(
        "struct App {
            consumer: Value = consume(all(group), optional(absent), optional(second), optional, all);
            #[set(group, other)] external first: Value;
            external optional: Value;
            import source: Source { #[set(group)] second: Value; };
            #[set(group)] external all: Value;
        }",
    )
    .unwrap();
    let graph = module.validate().unwrap();
    assert_eq!(graph.dependencies[0], [1, 3, 4, 3, 2, 4]);
    assert_eq!(graph.arguments[0], [0..3, 3..3, 3..4, 4..5, 5..6]);
    let factory = module.bindings[0].factory.as_ref().unwrap();
    assert!(matches!(factory.dependencies[3], Argument::Required(_)));
    assert!(matches!(factory.dependencies[4], Argument::Required(_)));
    syn::parse2::<syn::File>(module.expand().unwrap()).unwrap();
}

#[test]
fn expanded_collection_edges_keep_argument_diagnostics_and_async_propagation() {
    for (source, expected) in [
        (
            "struct App { #[scope(root)] consumer: Value = consume(all(group)); #[set(group)] external first: Value; #[set(group)] external second: Value; #[set(group)] contextual: Value = make(); }",
            "consumer -> contextual",
        ),
        (
            "struct App { consumer: Value = consume(optional(absent), all(group)); #[set(group)] external first: Value; #[set(group)] external second: Value; #[set(group)] recursive: Value = make(consumer); }",
            "consumer -> recursive -> consumer",
        ),
    ] {
        let error = syn::parse_str::<Module>(source)
            .unwrap()
            .validate()
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains(expected), "{error}");
    }
    let module = syn::parse_str::<Module>(
        "struct App {
            empty: Value = make(optional(absent), all(absent));
            consumer: Value = consume(optional(source), all(group));
            #[set(group)] async source: Value = make_async();
        }",
    )
    .unwrap();
    assert_eq!(module.validate().unwrap().is_async, [false, true, true]);
}

#[test]
fn membership_and_argument_parser_reject_ambiguous_forms() {
    for (source, expected) in [
        (
            "struct App { #[set()] external value: Value; }",
            "at least one group",
        ),
        (
            "struct App { #[set(group, r#group)] external value: Value; }",
            "duplicate static set membership",
        ),
        (
            "struct App { import source: Source { #[set(group)] #[set(group)] value: Value; }; }",
            "duplicate static set membership",
        ),
        (
            "struct App { value: Value = make(optional(first, second)); }",
            "single binding or group key",
        ),
        (
            "struct App { value: Value = make(all(optional(first))); }",
            "single binding or group key",
        ),
        (
            "struct App { value: Value = make(other(first)); }",
            "expected optional(key) or all(group)",
        ),
        ("#[set(group)] struct App {}", "only support doc attributes"),
        (
            "struct App { #[set(group)] import source: Source {}; }",
            "only support doc attributes",
        ),
    ] {
        let error = match syn::parse_str::<Module>(source) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("unexpectedly accepted {source}"),
        };
        assert!(error.contains(expected), "{source}: {error}");
    }
}
