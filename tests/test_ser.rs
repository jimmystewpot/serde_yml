//! Test suite for the serde_yml crate.

use indoc::indoc;
use serde::ser::{Serialize, Serializer as _};
use serde_derive::Serialize;
use serde_yml::ser::{Serializer, SerializerConfig};

fn test_ser<T>(value: &T, expected: &str)
where
    T: Serialize + ?Sized,
{
    let actual = serde_yml::to_string(value).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_serialize_bool() {
    let mut buffer = Vec::new();
    {
        let mut serializer = Serializer::new(&mut buffer).unwrap();
        serializer.serialize_bool(true).unwrap();
        serializer.serialize_bool(false).unwrap();
    }
    let actual = String::from_utf8(buffer).unwrap();
    let expected = "true\n---\nfalse\n";
    assert_eq!(actual, expected);
}

#[test]
fn test_serialize_i8() {
    let mut buffer = Vec::new();
    {
        let mut serializer = Serializer::new(&mut buffer).unwrap();
        serializer.serialize_i8(42).unwrap();
        serializer.serialize_i8(-100).unwrap();
    }
    let actual = String::from_utf8(buffer).unwrap();
    let expected = "42\n---\n-100\n";
    assert_eq!(actual, expected);
}

#[test]
fn test_serialize_i16() {
    let mut buffer = Vec::new();
    {
        let mut serializer = Serializer::new(&mut buffer).unwrap();
        serializer.serialize_i16(42).unwrap();
        serializer.serialize_i16(-100).unwrap();
    }
    let actual = String::from_utf8(buffer).unwrap();
    let expected = "42\n---\n-100\n";
    assert_eq!(actual, expected);
}

#[test]
fn test_serialize_i32() {
    let mut buffer = Vec::new();
    {
        let mut serializer = Serializer::new(&mut buffer).unwrap();
        serializer.serialize_i32(42).unwrap();
        serializer.serialize_i32(-100).unwrap();
    }
    let actual = String::from_utf8(buffer).unwrap();
    let expected = "42\n---\n-100\n";
    assert_eq!(actual, expected);
}

#[test]
fn test_serialize_i64() {
    let mut buffer = Vec::new();
    {
        let mut serializer = Serializer::new(&mut buffer).unwrap();
        serializer.serialize_i64(42).unwrap();
        serializer.serialize_i64(-100).unwrap();
    }
    let actual = String::from_utf8(buffer).unwrap();
    let expected = "42\n---\n-100\n";
    assert_eq!(actual, expected);
}

#[test]
fn test_serialize_i128() {
    let mut buffer = Vec::new();
    {
        let mut serializer = Serializer::new(&mut buffer).unwrap();
        serializer.serialize_i128(42).unwrap();
        serializer.serialize_i128(-100).unwrap();
        serializer.serialize_u128(u64::MAX as u128).unwrap();
        serializer.serialize_u128(u64::MAX as u128 + 1).unwrap();
        serializer.serialize_i128(i64::MIN as i128).unwrap();
        serializer.serialize_i128(i64::MIN as i128 - 1).unwrap();
    }
    let actual = String::from_utf8(buffer).unwrap();
    let expected = "42\n---\n-100\n---\n18446744073709551615\n---\n18446744073709551616\n---\n-9223372036854775808\n---\n-9223372036854775809\n";
    assert_eq!(actual, expected);
}

#[test]
fn test_serialize_f64() {
    let mut buffer = Vec::new();
    {
        let mut serializer = Serializer::new(&mut buffer).unwrap();
        serializer.serialize_f64(3.141592653589793).unwrap();
        serializer.serialize_f64(f64::INFINITY).unwrap();
        serializer.serialize_f64(f64::NEG_INFINITY).unwrap();
        serializer.serialize_f64(f64::NAN).unwrap();
    }
    let actual = String::from_utf8(buffer).unwrap();
    let expected = "3.141592653589793\n---\n.inf\n---\n-.inf\n---\n.nan\n";
    assert_eq!(actual, expected);
}

#[test]
fn test_serialize_char() {
    let mut buffer = Vec::new();
    {
        let mut serializer = Serializer::new(&mut buffer).unwrap();
        serializer.serialize_char('a').unwrap();
        serializer.serialize_char('💻').unwrap();
    }
    let actual = String::from_utf8(buffer).unwrap();
    let expected = "a\n---\n💻\n";
    assert_eq!(actual, expected);
}

#[test]
fn test_serialize_str() {
    test_ser("hello", "hello\n");
}

#[test]
fn test_serialize_bytes() {
    let bytes = b"hello";
    let actual = serde_yml::to_string(&bytes).unwrap();
    // Byte serialization is now supported as a sequence
    assert!(actual.contains("104"));
}

#[test]
fn test_serialize_none() {
    test_ser::<Option<i32>>(&None, "null\n");
}

#[test]
fn test_serialize_some() {
    test_ser(&Some(42), "42\n");
}

#[test]
fn test_serialize_unit() {
    test_ser(&(), "null\n");
}

#[test]
fn test_serialize_unit_struct() {
    #[derive(Serialize)]
    struct Unit;
    test_ser(&Unit, "null\n");
}

#[test]
fn test_serialize_unit_variant() {
    #[derive(Serialize)]
    enum E {
        A,
    }
    test_ser(&E::A, "!A null\n");
}

#[test]
fn test_serialize_newtype_struct() {
    #[derive(Serialize)]
    struct Newtype(i32);
    test_ser(&Newtype(42), "42\n");
}

#[test]
fn test_serialize_newtype_variant() {
    #[derive(Serialize)]
    enum E {
        B(i32),
    }
    test_ser(&E::B(42), "!B 42\n");
}

#[test]
fn test_serialize_seq() {
    test_ser(&vec![1, 2, 3], "- 1\n- 2\n- 3\n");
}

#[test]
fn test_serialize_tuple() {
    test_ser(&(1, 2, 3), "- 1\n- 2\n- 3\n");
}

#[test]
fn test_serialize_map() {
    use std::collections::BTreeMap;
    let mut map = BTreeMap::new();
    map.insert("k", 107);
    test_ser(&map, "k: 107\n");
}

#[test]
fn test_serialize_enum() {
    #[derive(Serialize)]
    enum E {
        A,
        B(i32),
        C { x: i32, y: i32 },
    }
    let values = vec![E::A, E::B(42), E::C { x: 1, y: 2 }];
    let mut buffer = Vec::new();
    {
        let mut serializer = Serializer::new(&mut buffer).unwrap();
        for value in values {
            value.serialize(&mut serializer).unwrap();
        }
    }
    let actual = String::from_utf8(buffer).unwrap();
    let expected = "!A null\n---\n!B 42\n---\n!C\nx: 1\n'y': 2\n";
    assert_eq!(actual, expected);
}

#[test]
fn test_emit_scalar_with_literal_style() {
    use serde_yml::libyml::emitter::{Emitter, Event, Scalar, ScalarStyle};
    let mut buffer = Vec::new();
    {
        let mut emitter = Emitter::new(&mut buffer);
        emitter.emit(Event::Scalar(Scalar {
            tag: None,
            value: "test\nvalue",
            style: ScalarStyle::Literal,
        })).unwrap();
    }
    let actual = String::from_utf8(buffer).unwrap();
    let expected = "|-\n  test\n  value";
    assert_eq!(actual, expected);
}

#[test]
fn test_emit_scalar_with_folded_style() {
    use serde_yml::libyml::emitter::{Emitter, Event, Scalar, ScalarStyle};
    let mut buffer = Vec::new();
    {
        let mut emitter = Emitter::new(&mut buffer);
        emitter.emit(Event::Scalar(Scalar {
            tag: None,
            value: "test\nvalue",
            style: ScalarStyle::Folded,
        })).unwrap();
    }
    let actual = String::from_utf8(buffer).unwrap();
    let expected = ">-\n  test\n  value";
    assert_eq!(actual, expected);
}

#[test]
fn test_serialize_mixed_data_types() {
    #[derive(Serialize)]
    struct Data {
        name: String,
        age: u32,
        active: bool,
        scores: Vec<u32>,
        metadata: std::collections::BTreeMap<String, String>,
    }
    let mut metadata = std::collections::BTreeMap::new();
    metadata.insert("key1".to_owned(), "value1".to_owned());
    metadata.insert("key2".to_owned(), "value2".to_owned());
    let data = Data {
        name: "Alice".to_owned(),
        age: 30,
        active: true,
        scores: vec![80, 90, 95],
        metadata,
    };
    test_ser(&data, indoc! {"
        name: Alice
        age: 30
        active: true
        scores:
          - 80
          - 90
          - 95
        metadata:
          key1: value1
          key2: value2
    "});
}

#[test]
fn test_serialize_option() {
    let mut buffer = Vec::new();
    {
        let mut serializer = Serializer::new(&mut buffer).unwrap();
        serializer.serialize_some(&42).unwrap();
        serializer.serialize_none().unwrap();
    }
    let actual = String::from_utf8(buffer).unwrap();
    let expected = "42\n---\nnull\n";
    assert_eq!(actual, expected);
}

#[test]
fn test_serialize_special_characters() {
    test_ser(&"\"'\n\t", "\"\\\"'\\n\\t\"\n");
}

#[test]
fn test_tag_unit_variants() {
    #[derive(Serialize)]
    enum E {
        Unit,
    }
    let mut buffer = Vec::new();
    let config = SerializerConfig { tag_unit_variants: true };
    {
        let mut ser = Serializer::new_with_config(&mut buffer, config).unwrap();
        E::Unit.serialize(&mut ser).unwrap();
    }
    let actual = String::from_utf8(buffer).unwrap();
    assert_eq!(actual, "!Unit null\n");
}
