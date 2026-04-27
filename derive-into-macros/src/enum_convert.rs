use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DataEnum;

use crate::{
    attribute_parsing::{
        conversion_enum::{ConversionVariant, extract_enum_variants},
        conversion_meta::ConversionMeta,
    },
    derive_into::build_field_conversions,
};

pub(super) fn implement_all_enum_conversions(
    data_enum: &DataEnum,
    conversions: Vec<ConversionMeta>,
) -> syn::Result<TokenStream2> {
    let conversion_impls: Vec<_> = conversions
        .into_iter()
        .map(|conversion| {
            let variants =
                extract_enum_variants(data_enum, conversion.method, &conversion.other_type())?;
            implement_enum_conversion(conversion.clone(), &variants)
        })
        .collect::<Result<_, _>>()?;

    Ok(quote! {
        #(#conversion_impls)*
    })
}

fn implement_enum_conversion(
    meta: ConversionMeta,
    variants: &[ConversionVariant],
) -> syn::Result<TokenStream2> {
    let ConversionMeta {
        source_name,
        target_name,
        method,
        default_allowed,
        validate,
        wrap_unit: enum_wrap_unit,
    } = meta.clone();

    let default_fields = if default_allowed {
        quote! { ..Default::default() }
    } else {
        quote! {}
    };

    let is_from = method.is_from();
    let variant_conversions = variants.iter().map(|variant| {
        let ConversionVariant {
            source_name: source_variant_name,
            target_name: target_variant_name,
            named_variant,
            fields,
            wrap_unit,
        } = variant;

        let source_fields = fields.iter().map(|f| f.source_name.as_named());

        let field_conversions =
            build_field_conversions(&meta, *named_variant, false, fields).unwrap();

        if variant.fields.is_empty() {
            if *wrap_unit || enum_wrap_unit {
                // The deriving enum's variant is a unit variant, but the other
                // side encodes it as `Variant(())` (e.g. prost-generated proto
                // oneof enums). Emit the wrapping `()` on whichever side is
                // the proto type for this conversion direction.
                return if is_from {
                    quote! {
                        #source_name::#source_variant_name(_) => #target_name::#target_variant_name,
                    }
                } else {
                    quote! {
                        #source_name::#source_variant_name => #target_name::#target_variant_name(::core::default::Default::default()),
                    }
                };
            }
            return quote! {
                #source_name::#source_variant_name => #target_name::#target_variant_name,
            };
        }

        if variant.named_variant {
            quote! {
                #source_name::#source_variant_name{ #(#source_fields),* } => #target_name::#target_variant_name {
                    #(#field_conversions)*
                    #default_fields
                },
            }
        } else {
            quote! {
                #source_name::#source_variant_name(#(#source_fields),*) => {
                    #target_name::#target_variant_name(#(#field_conversions)*)
                },
            }
        }
    });

    let validate_call = validate.map(|func| quote! {
        #func(&source).map_err(|e| ::derive_into::ConvertError::Validation {
            from_type: stringify!(#source_name),
            to_type: stringify!(#target_name),
            details: format!("{}", e),
        })?;
    });

    Ok(if method.is_falliable() {
        quote! {
            impl TryFrom<#source_name> for #target_name {
                type Error = ::derive_into::ConvertError;
                fn try_from(source: #source_name) -> Result<#target_name, Self::Error> {
                    #validate_call
                    Ok(
                        match source {
                            #(#variant_conversions)*
                        }
                    )
                }
            }
        }
    } else {
        quote! {
            impl From<#source_name> for #target_name {
                fn from(source: #source_name) -> #target_name {
                    match source {
                        #(#variant_conversions)*
                    }
                }
            }
        }
    })
}
