use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use super::graph::Graph;
use super::{Input, Module, key};

impl Module {
    pub(super) fn generate(&self, graph: &Graph) -> TokenStream {
        let name = &self.name;
        let visibility = &self.visibility;
        let docs = &self.docs;
        let context = generated("context", 0);
        let providers = (0..self.bindings.len())
            .map(|index| generated("provider", index))
            .collect::<Vec<_>>();
        let fields = self
            .bindings
            .iter()
            .zip(&providers)
            .map(|(binding, field)| {
                let ty = &binding.ty;
                quote!(#field: ::fluxdi::Shared<::fluxdi::Provider<#ty>>)
            });
        let parameters = self.inputs.iter().map(|input| match input {
            Input::External(index) => {
                let argument = generated("external", *index);
                let ty = &self.bindings[*index].ty;
                quote!(#argument: ::fluxdi::Shared<#ty>)
            }
            Input::Import(index) => {
                let argument = generated("import", *index);
                let path = &self.imports[*index].path;
                quote!(#argument: #path)
            }
        });
        let constructors = self.provider_constructors(graph, &providers);
        let getters = self.bindings.iter().zip(&providers).enumerate().map(|(index, (binding, field))| {
            let name = &binding.name;
            let docs = &binding.docs;
            let ty = &binding.ty;
            if graph.is_async[index] {
                quote! {
                    #(#docs)*
                    pub async fn #name(&self) -> ::std::result::Result<::fluxdi::Shared<#ty>, ::fluxdi::Error> {
                        self.#context.try_resolve_provider_async(&self.#field).await
                    }
                }
            } else {
                quote! {
                    #(#docs)*
                    pub fn #name(&self) -> ::std::result::Result<::fluxdi::Shared<#ty>, ::fluxdi::Error> {
                        self.#context.try_resolve_provider(&self.#field)
                    }
                }
            }
        });
        let cloned_fields = providers
            .iter()
            .map(|field| quote!(#field: ::fluxdi::Shared::clone(&self.#field)))
            .collect::<Vec<_>>();
        let default = self.inputs.is_empty().then(|| {
            quote! {
                impl ::core::default::Default for #name {
                    fn default() -> Self { Self::new() }
                }
            }
        });
        let metadata = self.export_metadata(graph, &providers);
        let assertions = self.metadata_assertions();
        quote! {
            #(#docs)*
            #[derive(::core::clone::Clone)]
            #visibility struct #name {
                #context: ::fluxdi::Injector,
                #(#fields,)*
            }

            impl #name {
                /// Creates a module, taking imports and external bindings in declaration order.
                pub fn new(#(#parameters),*) -> Self {
                    #(#constructors)*
                    Self {
                        #context: ::fluxdi::Injector::root(),
                        #(#providers,)*
                    }
                }

                /// Creates a new scoped resolution context over the same graph.
                pub fn create_scope(&self) -> Self {
                    Self {
                        #context: self.#context.create_scope(),
                        #(#cloned_fields,)*
                    }
                }

                /// Creates a child resolution context over the same graph.
                pub fn create_child(&self) -> Self {
                    Self {
                        #context: ::fluxdi::Injector::child(::fluxdi::Shared::new(self.#context.clone())),
                        #(#cloned_fields,)*
                    }
                }

                /// Returns the injector context used to resolve this graph.
                pub fn injector(&self) -> &::fluxdi::Injector { &self.#context }

                /// Uses the same graph with an existing injector context.
                pub fn in_context(&self, injector: &::fluxdi::Injector) -> Self {
                    Self {
                        #context: injector.clone(),
                        #(#cloned_fields,)*
                    }
                }

                #(#getters)*
                #metadata
            }

            #assertions
            #default
        }
    }
}

pub(super) fn generated(kind: &str, index: usize) -> Ident {
    Ident::new(&format!("__fluxdi_{kind}_{index}"), Span::mixed_site())
}

pub(super) fn export(kind: &str, name: &Ident) -> Ident {
    Ident::new(
        &format!("__fluxdi_{kind}_{}", key(name)),
        Span::mixed_site(),
    )
}
