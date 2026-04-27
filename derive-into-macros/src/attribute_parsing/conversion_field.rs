use darling::{FromField, FromMeta};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, format_ident, quote};
use syn::{Field, Ident, Path, spanned::Spanned};

use crate::util::{extract_hashmap_inner_types, extract_inner_type, is_surrounding_type};

use super::conversion_meta::ConversionMethod;

// Field level attributes using darling
#[derive(FromMeta, Debug)]
struct ConvertFieldAttr {
    path: Option<Path>,

    #[darling(default)]
    skip: bool,

    #[darling(default)]
    unwrap: bool,

    #[darling(default)]
    unwrap_or_default: bool,

    #[darling(default)]
    default: bool,

    #[darling(default)]
    enum_repr: bool,

    // Add any other field attributes you need
    #[darling(default)]
    rename: Option<String>,

    #[darling(default)]
    with_func: Option<syn::Path>,
}

#[derive(FromField, Debug)]
#[darling(attributes(convert))]
struct ConvertField {
    ident: Option<Ident>,

    #[darling(default)]
    skip: bool,

    #[darling(default)]
    rename: Option<String>,

    #[darling(default)]
    default: bool,

    #[darling(default)]
    unwrap: bool,

    #[darling(default)]
    unwrap_or_default: bool,

    #[darling(default)]
    enum_repr: bool,

    #[darling(default)]
    with_func: Option<syn::Path>,

    // Different conversion types
    #[darling(default, multiple)]
    from: Vec<ConvertFieldAttr>,

    #[darling(default, multiple)]
    try_from: Vec<ConvertFieldAttr>,

    #[darling(default, multiple)]
    into: Vec<ConvertFieldAttr>,

    #[darling(default, multiple)]
    try_into: Vec<ConvertFieldAttr>,
}

#[derive(Clone)]
pub(crate) enum FieldConversionMethod {
    Plain,
    UnwrapOption(Box<FieldConversionMethod>),
    UnwrapOrDefault(Box<FieldConversionMethod>),
    SomeOption(Box<FieldConversionMethod>),
    Option(Box<FieldConversionMethod>),
    Iterator(Box<FieldConversionMethod>),
    HashMap(Box<FieldConversionMethod>, Box<FieldConversionMethod>),
    /// Represents a prost-style enum field that is stored as `i32` on the
    /// other side. Lifts through `Option`/`Vec` wrappers; the leaf `Plain`
    /// position holds an enum value.
    ///
    /// Only used when `is_from` is true (proto -> model direction); for the
    /// reverse direction the enum's `From<Enum> for i32` impl makes the
    /// regular `Plain` (`.into()`) path work.
    EnumRepr {
        enum_path: Path,
        inner: Box<FieldConversionMethod>,
    },
}

#[derive(Clone)]
pub(crate) enum FieldIdentifier {
    Named(Ident),
    Unnamed(usize),
}

#[derive(Clone)]
pub(crate) struct ConvertibleField {
    pub(crate) source_name: FieldIdentifier,
    pub(crate) span: Span,
    pub(crate) skip: bool,
    pub(crate) default: bool,
    pub(crate) method: FieldConversionMethod,
    pub(crate) target_name: FieldIdentifier,
    pub(crate) conversion_func: Option<syn::Path>,
}

pub(crate) fn extract_convertible_fields(
    fields: &syn::Fields,
    conversion_type: ConversionMethod,
    other_type: &Path,
) -> syn::Result<Vec<ConvertibleField>> {
    let mut result = Vec::new();

    // Determine which nested field we should check based on conversion type
    let is_from = matches!(
        conversion_type,
        ConversionMethod::From | ConversionMethod::TryFrom
    );

    for (i, field) in fields.iter().enumerate() {
        // Use darling to parse field attributes
        let convert_field = match ConvertField::from_field(field) {
            Ok(cf) => cf,
            Err(e) => {
                return Err(syn::Error::new(
                    field.span(),
                    format!("Failed to parse field attributes: {}", e),
                ));
            }
        };

        // Determine source field identifier
        let source_name = match &convert_field.ident {
            Some(ident) => FieldIdentifier::Named(ident.clone()),
            None => FieldIdentifier::Unnamed(i),
        };

        // Get the specific conversion attributes based on conversion type
        let field_conv_attrs: Vec<_> = match conversion_type {
            ConversionMethod::From => convert_field.from,
            ConversionMethod::TryFrom => convert_field.try_from,
            ConversionMethod::Into => convert_field.into,
            ConversionMethod::TryInto => convert_field.try_into,
        }
        .into_iter()
        .filter(|attrs| !attrs.path.as_ref().is_some_and(|path| path != other_type))
        .collect();

        let field_conv_attrs = match field_conv_attrs.len() {
            0 | 1 => field_conv_attrs.first(),
            _ => {
                return Err(syn::Error::new(
                    field.span(),
                    format!(
                        "Expected exactly one conversion attribute for field {:?}",
                        field_conv_attrs
                    ),
                ));
            }
        };

        let unwrap = field_conv_attrs
            .as_ref()
            .map_or(convert_field.unwrap, |attrs| attrs.unwrap);

        let unwrap_or_default = field_conv_attrs
            .as_ref()
            .map_or(convert_field.unwrap_or_default, |attrs| {
                attrs.unwrap_or_default
            });

        let enum_repr = field_conv_attrs
            .as_ref()
            .map_or(convert_field.enum_repr, |attrs| attrs.enum_repr);

        let default = field_conv_attrs
            .as_ref()
            .map_or(convert_field.default, |attrs| attrs.default);

        // Skip applies if either top-level or field-specific skip is true
        let skip = convert_field.skip || field_conv_attrs.as_ref().is_some_and(|attrs| attrs.skip);

        // Skip if marked with skip
        if skip {
            continue;
        }

        // Determine target field identifier with priority:
        // 1. Field-specific rename
        // 2. Top-level rename
        // 3. Original field name
        let target_name = field_conv_attrs
            .as_ref()
            .and_then(|attrs| attrs.rename.as_ref())
            .or(convert_field.rename.as_ref())
            .map(|rename| FieldIdentifier::Named(Ident::new(rename, field.span())))
            .unwrap_or_else(|| source_name.clone());

        // Determine field conversion method
        let method = decide_field_method(field, is_from, unwrap, unwrap_or_default, enum_repr)?;

        let conversion_func = field_conv_attrs
            .as_ref()
            .and_then(|attrs| attrs.with_func.as_ref())
            .or(convert_field.with_func.as_ref())
            .cloned();

        let (source_name, target_name) = if is_from {
            (target_name.clone(), source_name.clone())
        } else {
            (source_name.clone(), target_name.clone())
        };

        result.push(ConvertibleField {
            source_name,
            span: field.span(),
            skip: false, // We've already filtered out skipped fields
            method,
            target_name,
            default,
            conversion_func,
        });
    }

    // sort so that fields with conversion functions are first
    result.sort_by(|a, b| {
        if a.conversion_func.is_some() && b.conversion_func.is_none() {
            std::cmp::Ordering::Less
        } else if a.conversion_func.is_none() && b.conversion_func.is_some() {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });

    Ok(result)
}

/// Recursively determines the conversion method for a type by inspecting
/// nested container types (Option, Vec, HashMap).
fn decide_field_method_for_type(ty: &syn::Type) -> FieldConversionMethod {
    if let Some(inner_ty) = extract_inner_type(ty, "Option") {
        let inner = decide_field_method_for_type(inner_ty);
        return FieldConversionMethod::Option(Box::new(inner));
    }
    if let Some(inner_ty) = extract_inner_type(ty, "Vec") {
        let inner = decide_field_method_for_type(inner_ty);
        return FieldConversionMethod::Iterator(Box::new(inner));
    }
    if let Some((key_ty, val_ty)) = extract_hashmap_inner_types(ty) {
        let key_inner = decide_field_method_for_type(key_ty);
        let val_inner = decide_field_method_for_type(val_ty);
        return FieldConversionMethod::HashMap(Box::new(key_inner), Box::new(val_inner));
    }
    FieldConversionMethod::Plain
}

/// For `enum_repr` fields, walks `Option<...>` / `Vec<...>` wrappers to find
/// the leaf path (the enum type). Returns the leaf path together with a
/// `FieldConversionMethod` mirroring the wrapper structure (`Plain` at the
/// leaf where the enum lives).
fn build_enum_repr_method(ty: &syn::Type) -> Option<(Path, FieldConversionMethod)> {
    if let Some(inner_ty) = extract_inner_type(ty, "Option") {
        let (path, inner) = build_enum_repr_method(inner_ty)?;
        return Some((path, FieldConversionMethod::Option(Box::new(inner))));
    }
    if let Some(inner_ty) = extract_inner_type(ty, "Vec") {
        let (path, inner) = build_enum_repr_method(inner_ty)?;
        return Some((path, FieldConversionMethod::Iterator(Box::new(inner))));
    }
    if let syn::Type::Path(type_path) = ty {
        return Some((type_path.path.clone(), FieldConversionMethod::Plain));
    }
    None
}

pub(crate) fn decide_field_method(
    field: &Field,
    is_from: bool,
    unwrap: bool,
    unwrap_or_default: bool,
    enum_repr: bool,
) -> syn::Result<FieldConversionMethod> {
    let is_option = is_surrounding_type(&field.ty, "Option");

    if unwrap && unwrap_or_default {
        return Err(syn::Error::new_spanned(
            &field.ty,
            "Cannot use both unwrap and unwrap_or_default",
        ));
    }

    if enum_repr && (unwrap || unwrap_or_default) {
        return Err(syn::Error::new_spanned(
            &field.ty,
            "enum_repr cannot be combined with unwrap / unwrap_or_default",
        ));
    }

    // `enum_repr` only changes behavior in the From / TryFrom direction
    // (i32 -> Enum, where prost only emits TryFrom). For Into / TryInto the
    // regular `Plain` path works because prost emits `From<Enum> for i32`.
    if enum_repr && is_from {
        let (enum_path, inner) = build_enum_repr_method(&field.ty).ok_or_else(|| {
            syn::Error::new_spanned(
                &field.ty,
                "enum_repr field must be a path type, optionally wrapped in Option<..> or Vec<..>",
            )
        })?;
        return Ok(FieldConversionMethod::EnumRepr {
            enum_path,
            inner: Box::new(inner),
        });
    }

    if unwrap || unwrap_or_default {
        match (is_option, is_from) {
            (true, false) => {
                // Option<T> -> T: unwrap, then recursively convert inner
                let inner_ty = extract_inner_type(&field.ty, "Option").unwrap();
                let inner_method = decide_field_method_for_type(inner_ty);
                return if unwrap {
                    Ok(FieldConversionMethod::UnwrapOption(Box::new(inner_method)))
                } else {
                    Ok(FieldConversionMethod::UnwrapOrDefault(Box::new(
                        inner_method,
                    )))
                };
            }
            (true, true) => {
                // From direction: T -> Option<T>, wrap in Some
                let inner_ty = extract_inner_type(&field.ty, "Option").unwrap();
                let inner_method = decide_field_method_for_type(inner_ty);
                return Ok(FieldConversionMethod::SomeOption(Box::new(inner_method)));
            }
            (false, true) => {
                // From direction: other side has Option<T>, self has T
                let inner_method = decide_field_method_for_type(&field.ty);
                return if unwrap {
                    Ok(FieldConversionMethod::UnwrapOption(Box::new(inner_method)))
                } else {
                    Ok(FieldConversionMethod::UnwrapOrDefault(Box::new(
                        inner_method,
                    )))
                };
            }
            (false, false) => {
                return Err(syn::Error::new_spanned(
                    &field.ty,
                    "Cannot unwrap non-Option field",
                ));
            }
        }
    }

    // No unwrap attributes — determine method recursively from the type
    Ok(decide_field_method_for_type(&field.ty))
}

impl ToTokens for FieldIdentifier {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            FieldIdentifier::Named(ident) => {
                tokens.extend(quote! { #ident });
            }
            FieldIdentifier::Unnamed(index) => {
                let index = syn::Index::from(*index);
                tokens.extend(quote! { #index });
            }
        }
    }
}

impl FieldIdentifier {
    pub(crate) fn as_named(&self) -> TokenStream2 {
        match self {
            FieldIdentifier::Named(ident) => quote! { #ident },
            FieldIdentifier::Unnamed(index) => {
                let field_name = format_ident!("field{}", index);
                quote! { #field_name }
            }
        }
    }
}
