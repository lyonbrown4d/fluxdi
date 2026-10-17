use super::*;

fn expand(source: &str) -> syn::Result<TokenStream> {
    syn::parse_str::<Module>(source)?.expand()
}

#[test]
fn explicit_graph_orders_forward_references_and_preserves_repeated_arguments() {
    let module = syn::parse_str::<Module>(
        "pub struct App {
            consumer: Consumer = make_consumer(second, first, second);
            external first: dyn Service;
            external second: Alias;
        }",
    )
    .unwrap();
    let graph = module.validate().unwrap();
    assert_eq!(graph.order, vec![2, 1, 0]);
    assert_eq!(graph.dependencies[0], vec![2, 1, 2]);
    syn::parse2::<syn::File>(module.generate(&graph)).unwrap();
}

#[test]
fn graph_errors_identify_binding_names_and_dependency_chains() {
    for (source, expected) in [
        (
            "struct App { value: Value = make(missing); }",
            "unknown static binding `missing` required by `value`",
        ),
        (
            "struct App { external value: Value; external value: Value; }",
            "duplicate static binding `value`",
        ),
        (
            "struct App { external value: Value; external r#value: Value; }",
            "duplicate static binding `value`",
        ),
        (
            "struct App { first: First = make(second); second: Second = make(third); third: Third = make(first); }",
            "static dependency cycle: first -> second -> third -> first",
        ),
        (
            "struct App { recursive: Value = make(recursive); }",
            "static dependency cycle: recursive -> recursive",
        ),
    ] {
        let error = expand(source).unwrap_err().to_string();
        assert!(error.contains(expected), "{source}: got {error}");
    }
    for name in [
        "new",
        "create_scope",
        "create_child",
        "injector",
        "in_context",
        "clone",
        "default",
    ] {
        let source = format!("struct App {{ external {name}: Value; }}");
        assert!(
            expand(&source)
                .unwrap_err()
                .to_string()
                .contains("conflicts with a generated module method")
        );
    }
}

#[test]
fn root_bindings_reject_context_capture_even_through_transient_edges() {
    for (source, expected) in [
        (
            "struct App { #[scope(root)] global: Global = make(local); local: Local = make(); }",
            "root binding `global` cannot capture contextual binding `local`: global -> local",
        ),
        (
            "struct App { #[scope(root)] global: Global = make(bridge); #[scope(transient)] bridge: Bridge = make(local); #[scope(scoped)] local: Local = make(); }",
            "global -> bridge -> local",
        ),
    ] {
        let error = expand(source).unwrap_err().to_string();
        assert!(error.contains(expected), "{source}: got {error}");
    }
    expand("struct App { #[scope(root)] global: Global = make(bridge); #[scope(transient)] bridge: Bridge = make(external_value); external external_value: Value; }").unwrap();
}

#[test]
fn scope_and_syntax_errors_reject_unsupported_declarations() {
    for (source, expected) in [
        ("struct App<T> {}", "do not support generics"),
        ("struct App where A: B {}", "do not support generics"),
        ("struct App {} struct Other {}", "single static module"),
        (
            "#[derive(Clone)] struct App {}",
            "only support doc attributes",
        ),
        (
            "struct App { #[cfg(test)] value: Value = make(); }",
            "only support scope and doc attributes",
        ),
        (
            "struct App { #[scope(other)] value: Value = make(); }",
            "expected scope(root)",
        ),
        (
            "struct App { #[scope(root)] #[scope(root)] value: Value = make(); }",
            "duplicate scope",
        ),
        (
            "struct App { #[scope(root)] external value: Value; }",
            "external bindings always have root scope",
        ),
        (
            "struct App { value: Value = make(42); }",
            "expected identifier",
        ),
        (
            "struct App { value: Value = |injector| make(injector); }",
            "expected identifier",
        ),
        (
            "struct App { value: Value = async_make().await; }",
            "expected `;`",
        ),
    ] {
        let error = expand(source).unwrap_err().to_string();
        assert!(
            error.contains(expected),
            "{source}: expected {expected:?}, got {error:?}"
        );
    }
}

#[test]
fn valid_empty_and_annotated_modules_expand_as_rust_items() {
    for source in [
        "pub struct Empty {}",
        "/// Application docs\npub(crate) struct App { /// Binding docs\n #[scope(root)] root: dyn Service = factories::root(); #[scope(singleton)] single: Alias = factories::single(root); #[scope(scoped)] local: Value = factories::local(single); #[scope(transient)] transient: Value = factories::generic::<Value>(local); }",
        "struct App { external __fluxdi_context_0: Value; __fluxdi_provider_0: Value = make(__fluxdi_context_0); }",
    ] {
        syn::parse2::<syn::File>(expand(source).unwrap()).unwrap();
    }
}

#[test]
fn generic_factory_paths_keep_explicit_rust_type_arguments() {
    let module =
        syn::parse_str::<Module>("struct App { value: u32 = factories::make::<u32>(); }").unwrap();
    let factory = module.bindings[0].factory.as_ref().unwrap();
    let segment = factory.path.segments.last().unwrap();
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        panic!("factory turbofish arguments were lost");
    };
    assert!(arguments.colon2_token.is_some());
    assert_eq!(arguments.args.len(), 1);
    syn::parse2::<syn::File>(module.expand().unwrap()).unwrap();
}
