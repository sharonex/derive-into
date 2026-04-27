#[test]
fn test_derive_macro() {
    let t = trybuild::TestCases::new();
    t.pass("tests/cases/basic.rs");
    t.pass("tests/cases/test_complex_conversions.rs");
    t.pass("tests/cases/test_enum_conversions.rs");
    t.pass("tests/cases/test_struct_conversions.rs");
    t.pass("tests/cases/test_field_attributes.rs");
    t.pass("tests/cases/test_nested_containers.rs");
    t.pass("tests/cases/test_enum_repr.rs");
    t.pass("tests/cases/test_validate.rs");
}
