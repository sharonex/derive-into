use derive_into::Convert;

mod structs {
    use super::*;

    struct Source {
        name: String,
    }

    fn validate_source(source: &Source) -> Result<(), String> {
        if source.name.is_empty() {
            return Err("name must not be empty".into());
        }
        Ok(())
    }

    #[derive(Convert, Debug, PartialEq)]
    #[convert(try_from(path = "Source", validate = "validate_source"))]
    struct Target {
        name: String,
    }

    pub fn validate_passes() {
        let source = Source {
            name: "hello".into(),
        };
        let target: Target = source.try_into().unwrap();
        assert_eq!(target, Target { name: "hello".into() });
    }

    pub fn validate_fails() {
        let source = Source { name: "".into() };
        let result: Result<Target, _> = source.try_into();
        assert_eq!(
            result.unwrap_err().to_string(),
            "Validation failed converting Source to Target: name must not be empty"
        );
    }
}

mod enums {
    use super::*;

    #[derive(Clone)]
    enum SourceEnum {
        A,
        B(String),
    }

    fn validate_enum(source: &SourceEnum) -> Result<(), String> {
        if let SourceEnum::B(s) = source {
            if s.is_empty() {
                return Err("B value must not be empty".into());
            }
        }
        Ok(())
    }

    #[derive(Convert, Debug, PartialEq)]
    #[convert(try_from(path = "SourceEnum", validate = "validate_enum"))]
    enum TargetEnum {
        A,
        B(String),
    }

    pub fn enum_validate_passes() {
        let source = SourceEnum::B("hello".into());
        let target: TargetEnum = source.try_into().unwrap();
        assert_eq!(target, TargetEnum::B("hello".into()));
    }

    pub fn enum_validate_fails() {
        let source = SourceEnum::B("".into());
        let result: Result<TargetEnum, _> = source.try_into();
        assert_eq!(
            result.unwrap_err().to_string(),
            "Validation failed converting SourceEnum to TargetEnum: B value must not be empty"
        );
    }
}

fn main() {
    structs::validate_passes();
    structs::validate_fails();
    enums::enum_validate_passes();
    enums::enum_validate_fails();
    println!("validate tests passed");
}
