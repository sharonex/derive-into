use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::{DeriveInput, Path};

use crate::{
    attribute_parsing::{
        conversion_field::{ConvertibleField, FieldConversionMethod},
        conversion_meta::{ConversionMeta, extract_conversions},
    },
    enum_convert::implement_all_enum_conversions,
    struct_convert::implement_all_struct_conversions,
};

/// Generate an infallible conversion expression for a value according to the
/// recursive `FieldConversionMethod`. Returns a `TokenStream` that evaluates
/// to the converted value.
fn infallible_expr(value: TokenStream2, method: &FieldConversionMethod) -> TokenStream2 {
    match method {
        FieldConversionMethod::Plain => quote!(#value.into()),
        FieldConversionMethod::Option(inner) => {
            let inner_expr = infallible_expr(quote!(v), inner);
            quote!(#value.map(|v| #inner_expr))
        }
        FieldConversionMethod::Iterator(inner) => {
            let inner_expr = infallible_expr(quote!(v), inner);
            quote!(#value.into_iter().map(|v| #inner_expr).collect())
        }
        FieldConversionMethod::HashMap(key_method, val_method) => {
            let key_expr = infallible_expr(quote!(k), key_method);
            let val_expr = infallible_expr(quote!(v), val_method);
            quote!(#value.into_iter().map(|(k, v)| (#key_expr, #val_expr)).collect())
        }
        FieldConversionMethod::UnwrapOption(inner) => {
            let inner_expr = infallible_expr(quote!(__unwrapped), inner);
            quote!({
                let __unwrapped = #value.expect(
                    format!("Expected value to exist when converting").as_str()
                );
                #inner_expr
            })
        }
        FieldConversionMethod::UnwrapOrDefault(inner) => {
            let inner_expr = infallible_expr(quote!(__unwrapped), inner);
            quote!({
                let __unwrapped = #value.unwrap_or_default();
                #inner_expr
            })
        }
        FieldConversionMethod::SomeOption(inner) => {
            let inner_expr = infallible_expr(value, inner);
            quote!(Some(#inner_expr))
        }
    }
}

fn fallible_expr(value: TokenStream2, method: &FieldConversionMethod) -> TokenStream2 {
    match method {
        FieldConversionMethod::Plain => {
            quote!(#value.try_into().map_err(|e| format!("{:?}", e)))
        }
        FieldConversionMethod::Option(inner) => {
            let inner_expr = fallible_expr(quote!(v), inner);
            quote!(#value.map(|v| #inner_expr).transpose())
        }
        FieldConversionMethod::Iterator(inner) => {
            let inner_expr = fallible_expr(quote!(v), inner);
            quote!(#value.into_iter().map(|v| #inner_expr).collect::<Result<_, _>>())
        }
        FieldConversionMethod::HashMap(key_method, val_method) => {
            let key_expr = fallible_expr(quote!(k), key_method);
            let val_expr = fallible_expr(quote!(v), val_method);
            quote!((|| -> Result<_, String> {
                let mut result = ::std::collections::HashMap::new();
                for (k, v) in #value {
                    result.insert(#key_expr?, #val_expr?);
                }
                Ok(result)
            })())
        }
        FieldConversionMethod::UnwrapOption(inner) => {
            let inner_expr = fallible_expr(quote!(__unwrapped), inner);
            quote!(#value
                .ok_or_else(|| String::from("Expected value to exist"))
                .and_then(|__unwrapped| #inner_expr))
        }
        FieldConversionMethod::UnwrapOrDefault(inner) => {
            let inner_expr = fallible_expr(quote!(__unwrapped), inner);
            quote!({
                let __unwrapped = #value.unwrap_or_default();
                #inner_expr
            })
        }
        FieldConversionMethod::SomeOption(inner) => {
            let inner_expr = fallible_expr(value, inner);
            quote!(#inner_expr.map(Some))
        }
    }
}

pub(super) fn field_falliable_conversion(
    ConvertibleField {
        source_name,
        target_name,
        skip,
        method,
        span,
        default,
        conversion_func,
    }: ConvertibleField,
    target_type: &Path,
    named: bool,
    source_prefix: bool,
) -> TokenStream2 {
    if skip {
        return quote! {};
    }

    let named_start = if named {
        quote! { #target_name: }
    } else {
        quote! {}
    };

    let source_name = if source_prefix {
        quote!(source.#source_name)
    } else {
        let source_name = source_name.as_named();
        quote!(#source_name)
    };

    if default {
        return quote_spanned! { span =>
            #named_start Default::default(),
        };
    }

    let error_creator = if cfg!(feature = "anyhow") {
        quote!(anyhow::anyhow!)
    } else {
        quote!(format!)
    };

    if let Some(func) = conversion_func {
        return quote_spanned! { span =>
            #named_start #func(&source).map_err(|e|
                    #error_creator("Failed trying to convert {} to {}: {:?}",
                        stringify!(#source_name),
                        stringify!(#target_type),
                        e,
                    )
                )?,
        };
    }

    let map_err = quote! {
        map_err(|e|
            #error_creator("Failed trying to convert {} to {}: {}",
                stringify!(#source_name),
                stringify!(#target_type),
                e,
            )
        )
    };

    let expr = fallible_expr(source_name, &method);

    quote_spanned! { span =>
        #named_start #expr.#map_err?,
    }
}

pub(super) fn field_infalliable_conversion(
    ConvertibleField {
        source_name,
        target_name,
        skip,
        method,
        span,
        default,
        conversion_func,
    }: ConvertibleField,
    named: bool,
    source_prefix: bool,
) -> TokenStream2 {
    if skip {
        return quote! {};
    }
    let named_start = if named {
        quote! { #target_name: }
    } else {
        quote! {}
    };

    let source_name = if source_prefix {
        quote!(source.#source_name)
    } else {
        let source_name = source_name.as_named();
        quote!(#source_name)
    };

    if default {
        return quote_spanned! { span =>
            #named_start Default::default(),
        };
    }

    if let Some(func) = conversion_func {
        return quote_spanned! { span =>
            #named_start #func(&source),
        };
    }

    let expr = infallible_expr(source_name, &method);

    quote_spanned! { span =>
        #named_start #expr,
    }
}

pub(super) fn build_field_conversions(
    meta: &ConversionMeta,
    named: bool,
    source_prefix: bool,
    fields: &[ConvertibleField],
) -> syn::Result<Vec<TokenStream2>> {
    Ok(fields
        .iter()
        .map(|field| {
            if meta.method.is_falliable() {
                field_falliable_conversion(field.clone(), &meta.target_name, named, source_prefix)
            } else {
                field_infalliable_conversion(field.clone(), named, source_prefix)
            }
        })
        .collect())
}

pub(super) fn try_convert_derive(ast: &DeriveInput) -> syn::Result<TokenStream2> {
    let conversions = extract_conversions(ast);

    match &ast.data {
        syn::Data::Struct(data_struct) => {
            implement_all_struct_conversions(data_struct, conversions)
        }
        syn::Data::Enum(data_enum) => implement_all_enum_conversions(data_enum, conversions),
        syn::Data::Union(_) => Err(syn::Error::new_spanned(
            ast.ident.clone(),
            "Unions are not supported".to_string(),
        ))?,
    }
}
