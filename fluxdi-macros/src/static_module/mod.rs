use proc_macro2::TokenStream;
use syn::ext::IdentExt;
use syn::{Attribute, Ident, Path, Type, Visibility};

mod codegen;
mod conditions;
mod graph;
mod metadata;
mod parse;
mod providers;

#[cfg(test)]
mod argument_tests;
#[cfg(test)]
mod condition_tests;
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
    groups: Vec<Ident>,
}

struct Imported {
    module: usize,
    source: Ident,
}

struct Factory {
    path: Path,
    dependencies: Vec<Argument>,
}

enum Argument {
    Required(Ident),
    Optional(Ident),
    All(Ident),
}

impl Argument {
    fn name(&self) -> &Ident {
        match self {
            Self::Required(name) | Self::Optional(name) | Self::All(name) => name,
        }
    }
}

pub(super) fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    if let Some(output) = conditions::preprocess(input.clone())? {
        return Ok(output);
    }
    syn::parse2::<Module>(input)?.expand()
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
