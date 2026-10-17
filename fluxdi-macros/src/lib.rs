use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Field, Fields, GenericArgument, PathArguments, Type, TypePath,
    parse_macro_input,
};

#[proc_macro_derive(Injectable)]
pub fn derive_injectable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_injectable(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_injectable(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "Injectable derive currently does not support generic structs",
        ));
    }

    let ident = &input.ident;
    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &data_struct.fields,
                    "Injectable derive requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "Injectable derive only supports structs",
            ));
        }
    };

    let resolved_fields = fields
        .iter()
        .map(field_initializer)
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        impl #ident {
            pub fn from_injector(injector: &::fluxdi::Injector) -> ::fluxdi::Shared<Self> {
                Self::try_from_injector(injector).unwrap_or_else(|error| panic!("{}", error))
            }

            pub fn try_from_injector(injector: &::fluxdi::Injector) -> ::std::result::Result<::fluxdi::Shared<Self>, ::fluxdi::Error> {
                Ok(::fluxdi::Shared::new(Self {
                    #(#resolved_fields),*
                }))
            }
        }
    })
}

fn field_initializer(field: &Field) -> syn::Result<proc_macro2::TokenStream> {
    let field_ident = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new_spanned(field, "Injectable field must be named"))?;

    let dependency_ty = shared_inner_type(&field.ty).ok_or_else(|| {
        syn::Error::new_spanned(&field.ty, "Injectable fields must be typed as Shared<T>")
    })?;

    Ok(quote! {
        #field_ident: injector.try_resolve::<#dependency_ty>()?
    })
}

fn shared_inner_type(ty: &Type) -> Option<Type> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return None;
    };

    let segment = path.segments.last()?;
    if segment.ident != "Shared" {
        return None;
    }

    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };

    let GenericArgument::Type(inner_ty) = arguments.args.first()? else {
        return None;
    };

    Some(inner_ty.clone())
}
