use proc_macro2::{Ident, TokenStream};
use quote::quote;

use super::codegen::{export, generated};
use super::graph::Graph;
use super::{Argument, Module, Scope};

impl Module {
    pub(super) fn provider_constructors(
        &self,
        graph: &Graph,
        providers: &[Ident],
    ) -> Vec<TokenStream> {
        graph.order.iter().map(|index| {
            let binding = &self.bindings[*index];
            let provider = &providers[*index];
            let ty = &binding.ty;
            if let Some(imported) = &binding.imported {
                let argument = generated("import", imported.module);
                let source = export("provider", &imported.source);
                return quote! {
                    let #provider: ::fluxdi::Shared<::fluxdi::Provider<#ty>> = #argument.#source();
                };
            }
            let Some(factory) = &binding.factory else {
                let argument = generated("external", *index);
                return quote! {
                    let #provider: ::fluxdi::Shared<::fluxdi::Provider<#ty>> =
                        ::fluxdi::Shared::new(::fluxdi::Provider::<#ty>::try_root(move |_| {
                            ::std::result::Result::Ok(::fluxdi::Shared::clone(&#argument))
                        }));
                };
            };
            let path = &factory.path;
            let injector = generated("injector", *index);
            let is_async = graph.is_async[*index];
            let constructor = constructor(binding.scope, is_async);
            let captures = graph.dependencies[*index].iter().enumerate().map(|(offset, target)| {
                let captured = generated("dependency", offset);
                let dependency = &providers[*target];
                quote!(let #captured = ::fluxdi::Shared::clone(&#dependency);)
            });
            let arguments = graph.dependencies[*index].iter().enumerate().map(|(offset, target)| {
                let captured = generated("dependency", offset);
                let resolved = generated("resolved", offset);
                if graph.is_async[*target] {
                    quote!(let #resolved = #injector.try_resolve_provider_async(&#captured).await?;)
                } else {
                    quote!(let #resolved = #injector.try_resolve_provider(&#captured)?;)
                }
            });
            let resolved = factory.dependencies.iter().zip(&graph.arguments[*index]).map(|(argument, edges)| {
                let values = edges.clone().map(|offset| generated("resolved", offset)).collect::<Vec<_>>();
                match argument {
                    Argument::Required(_) => quote!(#(#values)*),
                    Argument::Optional(_) if values.is_empty() => quote!(::std::option::Option::None),
                    Argument::Optional(_) => quote!(::std::option::Option::Some(#(#values)*)),
                    Argument::All(_) => quote!(::std::vec![#(#values),*]),
                }
            });
            let invocation = if binding.declared_async {
                quote!(#path(#(#resolved),*).await)
            } else {
                quote!(#path(#(#resolved),*))
            };
            let body = if is_async {
                // Providers hold Fn closures; each returned future owns fresh handle clones.
                let future_captures = (0..graph.dependencies[*index].len()).map(|offset| {
                    let captured = generated("dependency", offset);
                    quote!(let #captured = ::fluxdi::Shared::clone(&#captured);)
                });
                quote! {
                    #(#future_captures)*
                    async move {
                        #(#arguments)*
                        #invocation
                    }
                }
            } else {
                quote! { #(#arguments)* #invocation }
            };
            quote! {
                let #provider: ::fluxdi::Shared<::fluxdi::Provider<#ty>> = {
                    #(#captures)*
                    ::fluxdi::Shared::new(::fluxdi::Provider::<#ty>::#constructor(move |#injector| { #body }))
                };
            }
        }).collect()
    }
}

fn constructor(scope: Scope, is_async: bool) -> Ident {
    let base = match scope {
        Scope::Root => "try_root",
        Scope::Singleton => "try_singleton",
        Scope::Scoped => "try_scoped",
        Scope::Transient => "try_transient",
    };
    Ident::new(
        &format!("{base}{}", if is_async { "_async" } else { "" }),
        proc_macro2::Span::mixed_site(),
    )
}
