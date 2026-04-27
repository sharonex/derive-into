use derive_into::Convert;

// Mimics a prost `Enumeration` enum: provides `From<MyEnum> for i32` and
// `TryFrom<i32> for MyEnum`, but NO `From<i32> for MyEnum`. Default = variant 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(i32)]
enum Codec {
    #[default]
    Unspecified = 0,
    H264 = 1,
    H265 = 2,
}

impl From<Codec> for i32 {
    fn from(c: Codec) -> i32 {
        c as i32
    }
}

impl TryFrom<i32> for Codec {
    type Error = String;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Codec::Unspecified),
            1 => Ok(Codec::H264),
            2 => Ok(Codec::H265),
            other => Err(format!("unknown codec: {}", other)),
        }
    }
}

// Mirrors a prost-generated message: enum fields are stored as `i32`
// (or `Option<i32>` / `Vec<i32>`).
#[derive(Debug, PartialEq)]
struct ProtoStream {
    id: u32,
    codec: i32,
    optional_codec: Option<i32>,
    fallback_codecs: Vec<i32>,
}

#[derive(Convert, Debug, PartialEq)]
#[convert(into(path = "ProtoStream"))]
#[convert(from(path = "ProtoStream"))]
struct ModelStream {
    id: u32,
    #[convert(enum_repr)]
    codec: Codec,
    #[convert(enum_repr)]
    optional_codec: Option<Codec>,
    #[convert(enum_repr)]
    fallback_codecs: Vec<Codec>,
}

// A separate model that exercises the fallible (TryFrom) path. Splitting it
// out avoids the `From + TryFrom` blanket-impl conflict on the same pair.
#[derive(Convert, Debug, PartialEq)]
#[convert(try_from(path = "ProtoStream"))]
struct ModelStreamStrict {
    id: u32,
    #[convert(enum_repr)]
    codec: Codec,
    #[convert(enum_repr)]
    optional_codec: Option<Codec>,
    #[convert(enum_repr)]
    fallback_codecs: Vec<Codec>,
}

fn main() {
    // Model -> Proto (Into): plain `.into()` works because `From<Codec> for i32` exists.
    let model = ModelStream {
        id: 1,
        codec: Codec::H264,
        optional_codec: Some(Codec::H265),
        fallback_codecs: vec![Codec::H264, Codec::H265],
    };
    let proto: ProtoStream = model.into();
    assert_eq!(proto.id, 1);
    assert_eq!(proto.codec, 1);
    assert_eq!(proto.optional_codec, Some(2));
    assert_eq!(proto.fallback_codecs, vec![1, 2]);

    // Proto -> Model (From, infallible): unknown tags fall back to Default.
    let proto_with_unknown = ProtoStream {
        id: 2,
        codec: 1,
        optional_codec: Some(99),
        fallback_codecs: vec![1, 88, 2],
    };
    let model: ModelStream = proto_with_unknown.into();
    assert_eq!(model.id, 2);
    assert_eq!(model.codec, Codec::H264);
    assert_eq!(model.optional_codec, Some(Codec::Unspecified));
    assert_eq!(
        model.fallback_codecs,
        vec![Codec::H264, Codec::Unspecified, Codec::H265]
    );

    // Proto -> Model (TryFrom): unknown tags propagate the error.
    let bad = ProtoStream {
        id: 3,
        codec: 99,
        optional_codec: None,
        fallback_codecs: vec![],
    };
    let result = ModelStreamStrict::try_from(bad);
    assert!(result.is_err());

    let good = ProtoStream {
        id: 4,
        codec: 2,
        optional_codec: Some(1),
        fallback_codecs: vec![1, 2],
    };
    let model = ModelStreamStrict::try_from(good).unwrap();
    assert_eq!(model.codec, Codec::H265);
    assert_eq!(model.optional_codec, Some(Codec::H264));
    assert_eq!(model.fallback_codecs, vec![Codec::H264, Codec::H265]);

    println!("enum_repr tests passed");
}
