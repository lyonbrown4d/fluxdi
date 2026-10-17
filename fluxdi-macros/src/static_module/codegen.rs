use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use super::graph::Graph;
use super::{Module, Scope};

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
        let parameters = self
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, binding)| binding.factory.is_none())
            .map(|(index, binding)| {
                let argument = generated("external", index);
                let ty = &binding.ty;
                quote!(#argument: ::fluxdi::Shared<#ty>)
            });
        let constructors = graph.order.iter().map(|index| {
            let binding = &self.bindings[*index];
            let provider = &providers[*index];
            let ty = &binding.ty;
            let injector = generated("injector", *index);
            if let Some(factory) = &binding.factory {
                let path = &factory.path;
                let constructor = match binding.scope {
                    Scope::Root => quote!(try_root),
                    Scope::Singleton => quote!(try_singleton),
                    Scope::Scoped => quote!(try_scoped),
                    Scope::Transient => quote!(try_transient),
                };
                let captures = graph.dependencies[*index].iter().enumerate().map(|(offset, target)| {
                    let captured = generated("dependency", offset);
                    let dependency = &providers[*target];
                    quote!(let #captured = ::fluxdi::Shared::clone(&#dependency);)
                });
                let arguments = graph.dependencies[*index].iter().enumerate().map(|(offset, _)| {
                    let captured = generated("dependency", offset);
                    let resolved = generated("resolved", offset);
                    quote!(let #resolved = #injector.try_resolve_provider(&#captured)?;)
                });
                let resolved = (0..graph.dependencies[*index].len()).map(|offset| generated("resolved", offset));
                quote! {
                    let #provider: ::fluxdi::Shared<::fluxdi::Provider<#ty>> = {
                        #(#captures)*
                        ::fluxdi::Shared::new(::fluxdi::Provider::<#ty>::#constructor(move |#injector| {
                            #(#arguments)*
                            #path(#(#resolved),*)
                        }))
                    };
                }
            } else {
                let argument = generated("external", *index);
                quote! {
                    let #provider: ::fluxdi::Shared<::fluxdi::Provider<#ty>> =
                        ::fluxdi::Shared::new(::fluxdi::Provider::<#ty>::try_root(move |_| {
                            ::std::result::Result::Ok(::fluxdi::Shared::clone(&#argument))
                        }));
                }
            }
        });
        let getters = self.bindings.iter().zip(&providers).map(|(binding, field)| {
            let name = &binding.name;
            let docs = &binding.docs;
            let ty = &binding.ty;
            quote! {
                #(#docs)*
                pub fn #name(&self) -> ::std::result::Result<::fluxdi::Shared<#ty>, ::fluxdi::Error> {
                    self.#context.try_resolve_provider(&self.#field)
                }
            }
        });
        let scoped_fields = providers
            .iter()
            .map(|field| quote!(#field: ::fluxdi::Shared::clone(&self.#field)));
        let child_fields = providers
            .iter()
            .map(|field| quote!(#field: ::fluxdi::Shared::clone(&self.#field)));
        let context_fields = providers
            .iter()
            .map(|field| quote!(#field: ::fluxdi::Shared::clone(&self.#field)));
        let default = self
            .bindings
            .iter()
            .all(|binding| binding.factory.is_some())
            .then(|| {
                quote! {
                    impl ::core::default::Default for #name {
                        fn default() -> Self {
                            Self::new()
                        }
                    }
                }
            });
        quote! {
            #(#docs)*
            #[derive(::core::clone::Clone)]
            #visibility struct #name {
                #context: ::fluxdi::Injector,
                #(#fields,)*
            }

            impl #name {
                /// Creates a module, taking external bindings in declaration order.
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
                        #(#scoped_fields,)*
                    }
                }

                /// Creates a child resolution context over the same graph.
                pub fn create_child(&self) -> Self {
                    Self {
                        #context: ::fluxdi::Injector::child(::fluxdi::Shared::new(self.#context.clone())),
                        #(#child_fields,)*
                    }
                }

                /// Returns the injector context used to resolve this graph.
                pub fn injector(&self) -> &::fluxdi::Injector {
                    &self.#context
                }

                /// Uses the same graph with an existing injector context.
                pub fn in_context(&self, injector: &::fluxdi::Injector) -> Self {
                    Self {
                        #context: injector.clone(),
                        #(#context_fields,)*
                    }
                }

                #(#getters)*
            }

            #default
        }
    }
}

fn generated(kind: &str, index: usize) -> Ident {
    Ident::new(&format!("__fluxdi_{kind}_{index}"), Span::mixed_site())
}
