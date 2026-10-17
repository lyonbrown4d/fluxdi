//! Defer conditions to rustc before parsing or validating the selected graph.
use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use quote::quote;
use syn::Attribute;
use syn::parse::{Parse, ParseStream};

#[derive(Clone)]
struct Declaration {
    attributes: Vec<Attribute>,
    tokens: Vec<TokenTree>,
    children: Option<(usize, Vec<Declaration>)>,
}

impl Parse for Declaration {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Ok(Self {
            attributes: Attribute::parse_outer(input)?,
            tokens: input.parse::<TokenStream>()?.into_iter().collect(),
            children: None,
        })
    }
}

struct Condition {
    predicate: TokenStream,
    // A cfg removes a false declaration; cfg_attr only changes attributes.
    replacement: Option<Vec<Attribute>>,
}

fn condition(attribute: &Attribute) -> syn::Result<Option<Condition>> {
    if attribute.path().is_ident("cfg") {
        return Ok(Some(Condition {
            predicate: attribute.parse_args::<TokenStream>()?,
            replacement: None,
        }));
    }
    if attribute.path().is_ident("cfg_attr") {
        let mut arguments = attribute.parse_args::<TokenStream>()?.into_iter();
        let mut comma = false;
        let predicate: TokenStream = arguments
            .by_ref()
            .take_while(|token| {
                comma = matches!(token, TokenTree::Punct(punct) if punct.as_char() == ',');
                !comma
            })
            .collect();
        if !comma {
            return Err(syn::Error::new_spanned(
                attribute,
                "expected `,` after cfg_attr predicate",
            ));
        }
        let nested: TokenStream = arguments.collect();
        let replacements = syn::parse::Parser::parse2(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            nested,
        )?;
        return Ok(Some(Condition {
            predicate,
            replacement: Some(
                replacements
                    .into_iter()
                    .map(|meta| syn::parse_quote!(#[#meta]))
                    .collect(),
            ),
        }));
    }
    Ok(None)
}

impl Declaration {
    fn discover_children(&mut self, module: bool) -> syn::Result<()> {
        let import = matches!(self.tokens.first(), Some(TokenTree::Ident(ident)) if ident == "import")
            && !matches!(self.tokens.get(1), Some(TokenTree::Punct(punct)) if punct.as_char() == ':');
        if !module && !import {
            return Ok(());
        }
        let Some((index, group)) =
            self.tokens
                .iter()
                .enumerate()
                .find_map(|(index, token)| match token {
                    TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                        Some((index, group))
                    }
                    _ => None,
                })
        else {
            return Ok(());
        };
        let mut declarations = Vec::new();
        let mut pending = TokenStream::new();
        for token in group.stream() {
            let end = matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ';');
            pending.extend([token]);
            if end {
                let mut declaration = syn::parse2::<Self>(pending)?;
                declaration.discover_children(false)?;
                declarations.push(declaration);
                pending = TokenStream::new();
            }
        }
        if !pending.is_empty() {
            let mut declaration = syn::parse2::<Self>(pending)?;
            declaration.discover_children(false)?;
            declarations.push(declaration);
        }
        self.children = Some((index, declarations));
        Ok(())
    }

    fn first_predicate(&self) -> syn::Result<Option<TokenStream>> {
        for attribute in &self.attributes {
            if let Some(condition) = condition(attribute)? {
                return Ok(Some(condition.predicate));
            }
        }
        if let Some((_, children)) = &self.children {
            for child in children {
                if let Some(predicate) = child.first_predicate()? {
                    return Ok(Some(predicate));
                }
            }
        }
        Ok(None)
    }

    fn select(mut self, predicate: &str, enabled: bool) -> syn::Result<Option<Self>> {
        let mut attributes = Vec::new();
        for attribute in self.attributes {
            if let Some(condition) = condition(&attribute)?
                && condition.predicate.to_string() == predicate
            {
                match (condition.replacement, enabled) {
                    (None, false) => return Ok(None),
                    (None, true) | (Some(_), false) => {}
                    (Some(replacements), true) => attributes.extend(replacements),
                }
            } else {
                attributes.push(attribute);
            }
        }
        self.attributes = attributes;
        if let Some((_, children)) = &mut self.children {
            let mut selected = Vec::new();
            for child in std::mem::take(children) {
                if let Some(child) = child.select(predicate, enabled)? {
                    selected.push(child);
                }
            }
            *children = selected;
        }
        Ok(Some(self))
    }

    fn tokens(&self) -> TokenStream {
        let mut tokens = self.tokens.clone();
        if let Some((index, children)) = &self.children {
            let TokenTree::Group(original) = &tokens[*index] else {
                unreachable!()
            };
            let mut group = Group::new(
                Delimiter::Brace,
                children.iter().map(Self::tokens).collect(),
            );
            group.set_span(original.span());
            tokens[*index] = TokenTree::Group(group);
        }
        let attributes = &self.attributes;
        quote!(#(#attributes)* #(#tokens)*)
    }
}

pub(super) fn preprocess(input: TokenStream) -> syn::Result<Option<TokenStream>> {
    let mut declaration = syn::parse2::<Declaration>(input)?;
    declaration.discover_children(true)?;
    let Some(predicate) = declaration.first_predicate()? else {
        return Ok(None);
    };
    let selected = declaration
        .clone()
        .select(&predicate.to_string(), true)?
        .map(|item| item.tokens());
    let rejected = declaration
        .select(&predicate.to_string(), false)?
        .map(|item| item.tokens());
    let yes = selected.map(|tokens| {
        quote! {
            #[cfg(#predicate)]
            ::fluxdi::static_module! { #tokens }
        }
    });
    let no = rejected.map(|tokens| {
        quote! {
            #[cfg(not(#predicate))]
            ::fluxdi::static_module! { #tokens }
        }
    });
    Ok(Some(quote!(#yes #no)))
}
