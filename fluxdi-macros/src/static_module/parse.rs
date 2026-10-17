use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Ident, Path, PathArguments, Token, braced, parenthesized};

use super::{Argument, Binding, Factory, Import, Imported, Input, Module, Scope, key};

mod keyword {
    syn::custom_keyword!(external);
    syn::custom_keyword!(import);
}

impl Parse for Module {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let docs = Attribute::parse_outer(input)?;
        require_docs(&docs, "static modules only support doc attributes")?;
        let visibility = input.parse()?;
        input.parse::<Token![struct]>()?;
        let name = input.parse()?;
        if input.peek(Token![<]) || input.peek(Token![where]) {
            return Err(input.error("static modules do not support generics or where clauses"));
        }
        let body;
        braced!(body in input);
        let mut module = Self {
            docs,
            visibility,
            name,
            bindings: Vec::new(),
            imports: Vec::new(),
            inputs: Vec::new(),
        };
        while !body.is_empty() {
            let attributes = Attribute::parse_outer(&body)?;
            if body.peek(keyword::import) && !body.peek2(Token![:]) {
                require_docs(&attributes, "static imports only support doc attributes")?;
                module.parse_import(&body)?;
            } else {
                let binding = parse_binding(&body, attributes)?;
                if binding.factory.is_none() {
                    module.inputs.push(Input::External(module.bindings.len()));
                }
                module.bindings.push(binding);
            }
        }
        if !input.is_empty() {
            return Err(input.error("expected a single static module declaration"));
        }
        Ok(module)
    }
}

impl Module {
    fn parse_import(&mut self, input: ParseStream<'_>) -> syn::Result<()> {
        input.parse::<keyword::import>()?;
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let path = input.parse::<Path>()?;
        if path
            .segments
            .iter()
            .any(|segment| !matches!(segment.arguments, PathArguments::None))
        {
            return Err(syn::Error::new_spanned(
                path,
                "static module imports do not support generics",
            ));
        }
        let module = self.imports.len();
        let body;
        braced!(body in input);
        while !body.is_empty() {
            let attributes = Attribute::parse_outer(&body)?;
            let (docs, groups) = parse_member_attributes(attributes)?;
            let declared_async = body.parse::<Option<Token![async]>>()?.is_some();
            let name: Ident = body.parse()?;
            body.parse::<Token![:]>()?;
            let ty = body.parse()?;
            let source = if body.peek(Token![=]) {
                body.parse::<Token![=]>()?;
                body.parse()?
            } else {
                name.clone()
            };
            body.parse::<Token![;]>()?;
            self.bindings.push(Binding {
                docs,
                name,
                ty,
                scope: Scope::Root,
                factory: None,
                imported: Some(Imported { module, source }),
                declared_async,
                groups,
            });
        }
        input.parse::<Token![;]>()?;
        self.inputs.push(Input::Import(module));
        self.imports.push(Import { name, path });
        Ok(())
    }
}

fn parse_binding(input: ParseStream<'_>, attributes: Vec<Attribute>) -> syn::Result<Binding> {
    let mut docs = Vec::new();
    let mut scope = None;
    let mut groups = Vec::new();
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
        } else if attribute.path().is_ident("set") {
            parse_groups(&attribute, &mut groups)?;
        } else {
            require_doc(
                &attribute,
                "static bindings only support scope, set, and doc attributes",
            )?;
            docs.push(attribute);
        }
    }
    let external = input.peek(keyword::external);
    if external {
        input.parse::<keyword::external>()?;
        if scope.is_some() {
            return Err(
                input.error("external bindings always have root scope; remove the scope attribute")
            );
        }
        if input.peek(Token![async]) {
            return Err(input.error("external bindings cannot be async"));
        }
    }
    let declared_async = input.parse::<Option<Token![async]>>()?.is_some();
    if declared_async && input.peek(keyword::external) {
        return Err(input.error("external bindings cannot be async"));
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
            .parse_terminated(Argument::parse, Token![,])?
            .into_iter()
            .collect();
        Some(Factory { path, dependencies })
    };
    input.parse::<Token![;]>()?;
    Ok(Binding {
        docs,
        name,
        ty,
        scope: if external {
            Scope::Root
        } else {
            scope.unwrap_or_default()
        },
        factory,
        imported: None,
        declared_async,
        groups,
    })
}

fn require_docs(attributes: &[Attribute], message: &str) -> syn::Result<()> {
    for attribute in attributes {
        require_doc(attribute, message)?;
    }
    Ok(())
}

fn require_doc(attribute: &Attribute, message: &str) -> syn::Result<()> {
    if attribute.path().is_ident("doc") {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(attribute, message))
    }
}

impl Parse for Argument {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        if !input.peek(syn::token::Paren) {
            return Ok(Self::Required(name));
        }
        let arguments;
        parenthesized!(arguments in input);
        let target: Ident = arguments.parse()?;
        if !arguments.is_empty() {
            return Err(arguments.error("expected a single binding or group key"));
        }
        match key(&name).as_str() {
            "optional" => Ok(Self::Optional(target)),
            "all" => Ok(Self::All(target)),
            _ => Err(syn::Error::new_spanned(
                name,
                "expected optional(key) or all(group)",
            )),
        }
    }
}

fn parse_member_attributes(
    attributes: Vec<Attribute>,
) -> syn::Result<(Vec<Attribute>, Vec<Ident>)> {
    let mut docs = Vec::new();
    let mut groups = Vec::new();
    for attribute in attributes {
        if attribute.path().is_ident("set") {
            parse_groups(&attribute, &mut groups)?;
        } else {
            require_doc(
                &attribute,
                "imported bindings only support doc attributes and set attributes",
            )?;
            docs.push(attribute);
        }
    }
    Ok((docs, groups))
}

fn parse_groups(attribute: &Attribute, groups: &mut Vec<Ident>) -> syn::Result<()> {
    let values = attribute
        .parse_args_with(syn::punctuated::Punctuated::<Ident, Token![,]>::parse_terminated)?;
    if values.is_empty() {
        return Err(syn::Error::new_spanned(
            attribute,
            "set requires at least one group key",
        ));
    }
    for group in values {
        if groups.iter().any(|existing| key(existing) == key(&group)) {
            return Err(syn::Error::new_spanned(
                &group,
                format!("duplicate static set membership `{}`", key(&group)),
            ));
        }
        groups.push(group);
    }
    Ok(())
}
