#[derive(thiserror::Error, Debug)]
pub enum ConvertError {
    #[error("Missing required field '{field}' when converting {from_type} to {to_type}")]
    MissingField {
        field: &'static str,
        from_type: &'static str,
        to_type: &'static str,
    },

    #[error("Failed to convert field '{field}' from {from_type} to {to_type}: {details}")]
    FieldConversion {
        field: &'static str,
        from_type: &'static str,
        to_type: &'static str,
        details: String,
    },

    #[error("Validation failed converting {from_type} to {to_type}: {details}")]
    Validation {
        from_type: &'static str,
        to_type: &'static str,
        details: String,
    },

    #[error("Custom function failed for '{field}' from {from_type} to {to_type}: {details}")]
    CustomFunction {
        field: &'static str,
        from_type: &'static str,
        to_type: &'static str,
        details: String,
    },
}
