use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{Data, DeriveInput, Fields, GenericParam, parse_macro_input, parse_quote};

mod dependency;
mod static_module;
use dependency::Dependency;

/// Defines an explicit synchronous dependency graph checked at compile time.
///
/// Bindings name their factory dependencies directly. Factories return
/// `Result<Shared<T>, Error>`; external bindings are supplied to `new`.
#[proc_macro]
pub fn static_module(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as static_module::Module);
    match input.expand() {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Derives synchronous constructors and a provider carrying dependency graph hints.
///
/// Supports `Shared<T>`, `Option<Shared<T>>`, and `Vec<Shared<T>>` fields.
/// Use `#[inject(name = "primary")]` on required or optional named bindings.
#[proc_macro_derive(Injectable, attributes(inject))]
pub fn derive_injectable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_injectable(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_injectable(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    if let Some(attribute) = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("inject"))
    {
        return Err(syn::Error::new_spanned(
            attribute,
            "inject is a field attribute",
        ));
    }
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "Injectable derive requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "Injectable derive only supports structs",
            ));
        }
    };
    let dependencies = fields
        .iter()
        .map(Dependency::parse)
        .collect::<syn::Result<Vec<_>>>()?;
    let mut generics = input.generics.clone();
    for dependency in &dependencies {
        let ty = &dependency.ty;
        generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(#ty: ::fluxdi::Service));
    }
    generics
        .make_where_clause()
        .predicates
        .push(parse_quote!(Self: ::fluxdi::Service));
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let ident = &input.ident;
    let injector = binding_ident(input, "__fluxdi_injector");
    let provider = binding_ident(input, "__fluxdi_provider");
    let scope = binding_ident(input, "__fluxdi_scope");
    let initializers = dependencies.iter().map(|dep| dep.initializer(&injector));
    let hints = dependencies.iter().map(Dependency::hint);
    Ok(quote! {
        impl #impl_generics #ident #type_generics #where_clause {
            /// Constructs the service, panicking on dependency resolution errors.
            pub fn from_injector(#injector: &::fluxdi::Injector) -> ::fluxdi::Shared<Self> {
                Self::try_from_injector(#injector).unwrap_or_else(|error| panic!("{}", error))
            }

            /// Constructs the service while preserving dependency resolution errors.
            pub fn try_from_injector(#injector: &::fluxdi::Injector) -> ::std::result::Result<::fluxdi::Shared<Self>, ::fluxdi::Error> {
                ::std::result::Result::Ok(::fluxdi::Shared::new(Self {
                    #(#initializers),*
                }))
            }

            /// Builds a fallible provider with graph hints inferred from the fields.
            pub fn provider(#scope: ::fluxdi::Scope) -> ::fluxdi::Provider<Self> {
                let #provider = match #scope {
                    ::fluxdi::Scope::Root => ::fluxdi::Provider::try_root(Self::try_from_injector),
                    ::fluxdi::Scope::Module => ::fluxdi::Provider::try_singleton(Self::try_from_injector),
                    ::fluxdi::Scope::Scoped => ::fluxdi::Provider::try_scoped(Self::try_from_injector),
                    ::fluxdi::Scope::Transient => ::fluxdi::Provider::try_transient(Self::try_from_injector),
                };
                Self::with_injected_dependencies(#provider)
            }

            /// Attaches inferred graph hints to a custom provider, without changing its factory.
            pub fn with_injected_dependencies(#provider: ::fluxdi::Provider<Self>) -> ::fluxdi::Provider<Self> {
                #provider #(#hints)*
            }
        }
    })
}

fn binding_ident(input: &DeriveInput, base: &str) -> Ident {
    let mut candidate = base.to_string();
    while input.generics.params.iter().any(|param| {
        let ident = match param {
            GenericParam::Const(param) => &param.ident,
            GenericParam::Type(param) => &param.ident,
            GenericParam::Lifetime(param) => &param.lifetime.ident,
        };
        ident.to_string().trim_start_matches("r#") == candidate.as_str()
    }) {
        candidate.push('_');
    }
    Ident::new(&candidate, Span::mixed_site())
}

#[cfg(test)]
mod tests;
