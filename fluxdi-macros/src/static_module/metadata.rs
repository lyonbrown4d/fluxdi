use proc_macro2::{Ident, TokenStream};
use quote::quote;

use super::codegen::export;
use super::graph::Graph;
use super::{Module, Scope, key};

impl Module {
    pub(super) fn export_metadata(&self, graph: &Graph, providers: &[Ident]) -> TokenStream {
        let metadata = self.bindings.iter().enumerate().map(|(index, binding)| {
            let provider = export("provider", &binding.name);
            let root_safe = export("root_safe", &binding.name);
            let is_async = export("is_async", &binding.name);
            let field = &providers[index];
            let ty = &binding.ty;
            let effective_async = graph.is_async[index];
            let safe = if let Some(imported) = &binding.imported {
                let path = &self.imports[imported.module].path;
                let source = export("root_safe", &imported.source);
                quote!(<#path>::#source())
            } else if matches!(binding.scope, Scope::Singleton | Scope::Scoped) {
                quote!(false)
            } else {
                let dependencies = graph.dependencies[index].iter().map(|target| {
                    let name = export("root_safe", &self.bindings[*target].name);
                    quote!(Self::#name())
                });
                quote!(true #(&& #dependencies)*)
            };
            quote! {
                #[doc(hidden)]
                pub fn #provider(&self) -> ::fluxdi::Shared<::fluxdi::Provider<#ty>> {
                    ::fluxdi::Shared::clone(&self.#field)
                }
                #[doc(hidden)]
                pub const fn #root_safe() -> bool { #safe }
                #[doc(hidden)]
                pub const fn #is_async() -> bool { #effective_async }
            }
        });
        quote!(#(#metadata)*)
    }

    pub(super) fn metadata_assertions(&self) -> TokenStream {
        let module = &self.name;
        let assertions = self.bindings.iter().filter_map(|binding| {
            if let Some(imported) = &binding.imported {
                let path = &self.imports[imported.module].path;
                let source = export("is_async", &imported.source);
                let declared_async = binding.declared_async;
                let message = format!(
                    "static import `{}` async declaration does not match source export `{}`",
                    key(&binding.name),
                    key(&imported.source)
                );
                Some(quote!(assert!(<#path>::#source() == #declared_async, #message);))
            } else if binding.factory.is_some() && binding.scope == Scope::Root {
                let safe = export("root_safe", &binding.name);
                let message = format!(
                    "root binding `{}` cannot capture an imported contextual binding",
                    key(&binding.name)
                );
                Some(quote!(assert!(#module::#safe(), #message);))
            } else {
                None
            }
        });
        quote! { const _: () = { #(#assertions)* }; }
    }
}
