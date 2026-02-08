pub(super) fn is_surrounding_type(ty: &syn::Type, surrounding_type: &'static str) -> bool {
    extract_inner_type(ty, surrounding_type).is_some()
}

pub(crate) fn extract_inner_type<'a>(
    ty: &'a syn::Type,
    surrounding_type: &str,
) -> Option<&'a syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 {
            let segment = &type_path.path.segments[0];
            if segment.ident == surrounding_type {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

pub(crate) fn extract_hashmap_inner_types(ty: &syn::Type) -> Option<(&syn::Type, &syn::Type)> {
    if let syn::Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 {
            let segment = &type_path.path.segments[0];
            if segment.ident == "HashMap" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    let mut types = args.args.iter().filter_map(|arg| {
                        if let syn::GenericArgument::Type(ty) = arg {
                            Some(ty)
                        } else {
                            None
                        }
                    });
                    if let (Some(key_ty), Some(val_ty)) = (types.next(), types.next()) {
                        return Some((key_ty, val_ty));
                    }
                }
            }
        }
    }
    None
}
