use proc_macro2::TokenStream;

use super::{Module, conditions, key};

fn branches(source: TokenStream) -> Vec<TokenStream> {
    let output = conditions::preprocess(source).unwrap().unwrap();
    syn::parse2::<syn::File>(output)
        .unwrap()
        .items
        .into_iter()
        .map(|item| {
            let syn::Item::Macro(item) = item else {
                panic!("conditions must be emitted on item macro invocations");
            };
            assert_eq!(item.attrs.len(), 1);
            item.mac.tokens
        })
        .collect()
}

#[test]
fn false_branch_removes_declarations_before_semantic_parsing() {
    let choices = branches(quote::quote! {
        struct App {
            #[cfg(false)] #[scope(invalid)] async duplicate: Missing = missing(unknown);
            #[cfg(false)] external duplicate: Missing;
            #[cfg(false)] import unavailable: Missing<Invalid> {
                #[scope(invalid)] value: Missing;
            };
            #[cfg(false)] external async invalid: Missing;
            external kept: Value;
        }
    });
    let module = syn::parse2::<Module>(choices[1].clone()).unwrap();
    assert_eq!(module.bindings.len(), 1);
    assert_eq!(key(&module.bindings[0].name), "kept");
    assert_eq!(module.imports.len(), 0);
    assert_eq!(module.inputs.len(), 1);
    assert_eq!(module.validate().unwrap().is_async, [false]);
    assert_eq!(
        branches(quote::quote!(
            #[cfg(false)]
            struct Omitted {}
        ))
        .len(),
        1
    );
}

#[test]
fn repeated_predicates_are_selected_together_across_import_entries() {
    let declarations = (0..256).map(|index| {
        let name = quote::format_ident!("member_{index}");
        quote::quote!(#[cfg(feature = "debug")] external #name: Value;)
    });
    let branches = branches(quote::quote! {
        struct App {
            #(#declarations)*
            import source: Source {
                #[cfg(feature = "debug")] selected: Value;
                remaining: Value;
            };
        }
    });
    for branch in &branches {
        assert!(conditions::preprocess(branch.clone()).unwrap().is_none());
    }
    let enabled = syn::parse2::<Module>(branches[0].clone()).unwrap();
    let disabled = syn::parse2::<Module>(branches[1].clone()).unwrap();
    assert_eq!(enabled.bindings.len(), 258);
    assert_eq!(disabled.bindings.len(), 1);
    assert_eq!(key(&disabled.bindings[0].name), "remaining");
}

#[test]
fn cfg_attr_reenters_for_nested_conditions_and_preserves_native_predicates() {
    for predicate in ["true", "false"] {
        let missing_comma = format!("#[cfg_attr({predicate})] struct App {{}}");
        let error = conditions::preprocess(missing_comma.parse().unwrap()).unwrap_err();
        assert!(error.to_string().contains("expected `,`"));
        let empty_attributes = format!("#[cfg_attr({predicate},)] struct App {{}}");
        assert_eq!(branches(empty_attributes.parse().unwrap()).len(), 2);
    }
    let first = branches(quote::quote! {
        #[cfg_attr(true, doc = "module")]
        struct App {
            #[cfg_attr(true, cfg_attr(false, set(group)), scope(root))]
            value: Value = make();
        }
    });
    let second = branches(first[0].clone());
    let selected = syn::parse2::<Module>(second[1].clone()).unwrap();
    assert_eq!(selected.docs.len(), 1);
    assert!(selected.bindings[0].groups.is_empty());
    assert!(selected.bindings[0].scope == super::Scope::Root);
    selected.validate().unwrap();
}
