use syn::{DeriveInput, parse_macro_input};

use crate::derive_into::try_convert_derive;

mod attribute_parsing;
mod derive_into;
mod enum_convert;
mod struct_convert;
mod util;

#[proc_macro_derive(Convert, attributes(convert))]
pub fn convert_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    try_convert_derive(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
