use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, GenericArgument, Ident, LitStr, PathArguments, Type, TypePath};

pub(super) enum Cardinality {
    One,
    Optional,
    All,
}

pub(super) struct Dependency {
    field: Ident,
    pub(super) ty: Type,
    cardinality: Cardinality,
    name: Option<LitStr>,
}

impl Dependency {
    pub(super) fn parse(field: &Field) -> syn::Result<Self> {
        let field_ident = field
            .ident
            .clone()
            .ok_or_else(|| syn::Error::new_spanned(field, "Injectable field must be named"))?;
        let (cardinality, ty) = if let Some(ty) = inner_type(&field.ty, "Shared") {
            (Cardinality::One, ty)
        } else if let Some(ty) =
            inner_type(&field.ty, "Option").and_then(|ty| inner_type(ty, "Shared"))
        {
            (Cardinality::Optional, ty)
        } else if let Some(ty) =
            inner_type(&field.ty, "Vec").and_then(|ty| inner_type(ty, "Shared"))
        {
            (Cardinality::All, ty)
        } else {
            return Err(syn::Error::new_spanned(
                &field.ty,
                "Injectable fields must be Shared<T>, Option<Shared<T>>, or Vec<Shared<T>>",
            ));
        };
        let mut name = None;
        for attribute in field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("inject"))
        {
            let mut parsed = false;
            attribute.parse_nested_meta(|meta| {
                if !meta.path.is_ident("name") {
                    return Err(meta.error("expected name = \"binding\""));
                }
                if name.is_some() {
                    return Err(meta.error("duplicate binding name"));
                }
                name = Some(meta.value()?.parse::<LitStr>()?);
                parsed = true;
                Ok(())
            })?;
            if !parsed {
                return Err(syn::Error::new_spanned(
                    attribute,
                    "expected inject(name = \"binding\")",
                ));
            }
        }
        if matches!(cardinality, Cardinality::All) && name.is_some() {
            return Err(syn::Error::new_spanned(
                field,
                "set dependencies cannot have a binding name",
            ));
        }
        Ok(Self {
            field: field_ident,
            ty: ty.clone(),
            cardinality,
            name,
        })
    }

    pub(super) fn initializer(&self, injector: &Ident) -> TokenStream {
        let field = &self.field;
        let ty = &self.ty;
        let resolution = match (&self.cardinality, &self.name) {
            (Cardinality::One, None) => quote!(#injector.try_resolve::<#ty>()?),
            (Cardinality::One, Some(name)) => quote!(#injector.try_resolve_named::<#ty>(#name)?),
            (Cardinality::Optional, None) => quote!(#injector.try_resolve_optional::<#ty>()?),
            (Cardinality::Optional, Some(name)) => {
                quote!(#injector.try_resolve_optional_named::<#ty>(#name)?)
            }
            (Cardinality::All, _) => quote!(#injector.try_resolve_all::<#ty>()?),
        };
        quote!(#field: #resolution)
    }

    pub(super) fn hint(&self) -> TokenStream {
        let ty = &self.ty;
        match (&self.cardinality, &self.name) {
            (Cardinality::One, None) => quote!(.with_dependency::<#ty>()),
            (Cardinality::One, Some(name)) => quote!(.with_named_dependency::<#ty>(#name)),
            (Cardinality::Optional, None) => quote!(.with_optional_dependency::<#ty>()),
            (Cardinality::Optional, Some(name)) => {
                quote!(.with_optional_named_dependency::<#ty>(#name))
            }
            (Cardinality::All, _) => quote!(.with_set_dependency::<#ty>()),
        }
    }
}

fn inner_type<'a>(ty: &'a Type, wrapper: &str) -> Option<&'a Type> {
    let Type::Path(TypePath { qself: None, path }) = ty else {
        return None;
    };
    let segment = path.segments.last()?;
    if segment.ident != wrapper {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    if arguments.args.len() != 1 {
        return None;
    }
    match arguments.args.first()? {
        GenericArgument::Type(inner_ty) => Some(inner_ty),
        _ => None,
    }
}
