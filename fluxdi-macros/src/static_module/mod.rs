use proc_macro2::TokenStream;
use syn::ext::IdentExt;
use syn::{Attribute, Ident, Path, Type, Visibility};

mod codegen;
mod graph;
mod metadata;
mod parse;
mod providers;

#[cfg(test)]
mod tests;

pub(super) struct Module {
    docs: Vec<Attribute>,
    visibility: Visibility,
    name: Ident,
    bindings: Vec<Binding>,
    imports: Vec<Import>,
    inputs: Vec<Input>,
}

struct Import {
    name: Ident,
    path: Path,
}

enum Input {
    External(usize),
    Import(usize),
}

struct Binding {
    docs: Vec<Attribute>,
    name: Ident,
    ty: Type,
    scope: Scope,
    factory: Option<Factory>,
    imported: Option<Imported>,
    declared_async: bool,
}

struct Imported {
    module: usize,
    source: Ident,
}

struct Factory {
    path: Path,
    dependencies: Vec<Ident>,
}

#[derive(Clone, Copy, Default, PartialEq)]
enum Scope {
    Root,
    #[default]
    Singleton,
    Scoped,
    Transient,
}

fn key(name: &Ident) -> String {
    name.unraw().to_string()
}

impl Module {
    pub(super) fn expand(&self) -> syn::Result<TokenStream> {
        let graph = self.validate()?;
        let output = self.generate(&graph);
        Ok(if graph.is_async.iter().any(|value| *value) {
            quote::quote!(::fluxdi::__static_module_async! { #output })
        } else {
            output
        })
    }
}
