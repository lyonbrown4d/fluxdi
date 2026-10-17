use proc_macro2::TokenStream;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Ident, Path, Token, Type, Visibility, braced, parenthesized};

mod codegen;
mod graph;

#[cfg(test)]
mod tests;

mod keyword {
    syn::custom_keyword!(external);
}

pub(super) struct Module {
    docs: Vec<Attribute>,
    visibility: Visibility,
    name: Ident,
    bindings: Vec<Binding>,
}

struct Binding {
    docs: Vec<Attribute>,
    name: Ident,
    ty: Type,
    scope: Scope,
    factory: Option<Factory>,
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

impl Parse for Module {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let docs = Attribute::parse_outer(input)?;
        for attribute in &docs {
            require_doc(attribute, "static modules only support doc attributes")?;
        }
        let visibility = input.parse()?;
        input.parse::<Token![struct]>()?;
        let name = input.parse()?;
        if input.peek(Token![<]) || input.peek(Token![where]) {
            return Err(input.error("static modules do not support generics or where clauses"));
        }
        let body;
        braced!(body in input);
        let mut bindings = Vec::new();
        while !body.is_empty() {
            bindings.push(body.parse()?);
        }
        if !input.is_empty() {
            return Err(input.error("expected a single static module declaration"));
        }
        Ok(Self {
            docs,
            visibility,
            name,
            bindings,
        })
    }
}

impl Parse for Binding {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let attributes = Attribute::parse_outer(input)?;
        let mut docs = Vec::new();
        let mut scope = None;
        for attribute in attributes {
            if attribute.path().is_ident("scope") {
                if scope.is_some() {
                    return Err(syn::Error::new_spanned(
                        attribute,
                        "duplicate scope attribute",
                    ));
                }
                let value = attribute.parse_args::<Ident>()?;
                scope = Some(match value.to_string().as_str() {
                    "root" => Scope::Root,
                    "singleton" => Scope::Singleton,
                    "scoped" => Scope::Scoped,
                    "transient" => Scope::Transient,
                    _ => {
                        return Err(syn::Error::new_spanned(
                            value,
                            "expected scope(root), scope(singleton), scope(scoped), or scope(transient)",
                        ));
                    }
                });
            } else {
                require_doc(
                    &attribute,
                    "static bindings only support scope and doc attributes",
                )?;
                docs.push(attribute);
            }
        }
        let external = input.peek(keyword::external);
        if external {
            input.parse::<keyword::external>()?;
            if scope.is_some() {
                return Err(input.error(
                    "external bindings always have root scope; remove the scope attribute",
                ));
            }
        }
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;
        let factory = if external {
            None
        } else {
            input.parse::<Token![=]>()?;
            let path = input.parse::<Path>()?;
            let arguments;
            parenthesized!(arguments in input);
            let dependencies = arguments
                .parse_terminated(Ident::parse, Token![,])?
                .into_iter()
                .collect();
            Some(Factory { path, dependencies })
        };
        input.parse::<Token![;]>()?;
        Ok(Self {
            docs,
            name,
            ty,
            scope: if external {
                Scope::Root
            } else {
                scope.unwrap_or_default()
            },
            factory,
        })
    }
}

fn require_doc(attribute: &Attribute, message: &str) -> syn::Result<()> {
    if attribute.path().is_ident("doc") {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(attribute, message))
    }
}

fn key(name: &Ident) -> String {
    name.unraw().to_string()
}

impl Module {
    pub(super) fn expand(&self) -> syn::Result<TokenStream> {
        let graph = self.validate()?;
        Ok(self.generate(&graph))
    }
}
