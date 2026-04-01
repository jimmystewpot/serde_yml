//! Test suite for the serde_yml crate.
#![allow(
    clippy::decimal_literal_representation,
    clippy::derive_partial_eq_without_eq,
    clippy::unreadable_literal,
    clippy::shadow_unrelated
)]

use indoc::indoc;
use serde_derive::{Deserialize, Serialize};
use serde_yml::{Mapping, Value};
use std::collections::BTreeMap;
use std::fmt::Debug;

fn test_serde<T>(thing: &T, yaml: &str)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + Debug,
{
    let serialized = serde_yml::to_string(&thing).unwrap();
    assert_eq!(yaml, serialized);

    let deserialized: T =
        serde_yml::from_str(yaml).expect("failed to deserialize from_str");
    assert_eq!(*thing, deserialized);

    let value: Value =
        serde_yml::from_str(yaml).expect("failed to deserialize from_str to value");
    let deserialized: T =
        T::deserialize(&value).expect("failed to deserialize from value");
    assert_eq!(*thing, deserialized);

    let deserialized: T =
        serde_yml::from_value(value).expect("failed to deserialize from_value");
    assert_eq!(*thing, deserialized);

    let _ = serde_yml::from_str::<serde::de::IgnoredAny>(yaml)
        .expect("failed to deserialize to IgnoredAny");
}

#[test]
fn test_default() {
    assert_eq!(Value::default(), Value::Null);
}

#[test]
fn test_int() {
    let thing = 256;
    let yaml = indoc! {"
        256
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_int_max_u64() {
    let thing = u64::MAX;
    let yaml = indoc! {"
        18446744073709551615
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_int_min_i64() {
    let thing = i64::MIN;
    let yaml = indoc! {"
        -9223372036854775808
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_int_max_i64() {
    let thing = i64::MAX;
    let yaml = indoc! {"
        9223372036854775807
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_i128_small() {
    let thing: i128 = -256;
    let yaml = indoc! {"
        -256
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_u128_small() {
    let thing: u128 = 256;
    let yaml = indoc! {"
        256
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_float() {
    let thing = f64::INFINITY;
    let yaml = indoc! {"
        .inf
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_float32() {
    let thing = f32::INFINITY;
    let yaml = indoc! {"
        .inf
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_vec() {
    let thing = vec![1, 2, 3];
    let yaml = indoc! {"
        - 1
        - 2
        - 3
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_map() {
    let mut thing = BTreeMap::new();
    thing.insert("a".to_owned(), 1);
    thing.insert("b".to_owned(), 2);
    let yaml = indoc! {"
        a: 1
        b: 2
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_basic_struct() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Basic {
        a: usize,
        b: String,
    }
    let thing = Basic {
        a: 1,
        b: "two".to_owned(),
    };
    let yaml = indoc! {"
        a: 1
        b: two
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_nested_struct() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Outer {
        a: Inner,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Inner {
        b: usize,
    }
    let thing = Outer { a: Inner { b: 1 } };
    let yaml = indoc! {"
        a:
          b: 1
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_unit_struct() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Unit;
    let thing = Unit;
    let yaml = indoc! {"
        null
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_unit_variant() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum E {
        V,
    }
    let thing = E::V;
    let yaml = "!V null\n";
    test_serde(&thing, yaml);
}

#[test]
fn test_newtype_struct() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Newtype(usize);
    let thing = Newtype(1);
    let yaml = indoc! {"
        1
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_newtype_variant() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum E {
        Size(usize),
    }
    let thing = E::Size(127);
    let yaml = "!Size 127\n";
    test_serde(&thing, yaml);
}

#[test]
fn test_tuple_variant() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum E {
        Rgb(u8, u8, u8),
    }
    let thing = E::Rgb(32, 64, 96);
    let yaml = "!Rgb\n- 32\n- 64\n- 96\n";
    test_serde(&thing, yaml);
}

#[test]
fn test_struct_variant() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum E {
        Color { r: u8, g: u8, b: u8 },
    }
    let thing = E::Color {
        r: 32,
        g: 64,
        b: 96,
    };
    let yaml = "!Color\nr: 32\ng: 64\nb: 96\n";
    test_serde(&thing, yaml);
}

#[test]
fn test_option() {
    let thing = Some(1);
    let yaml = indoc! {"
        1
    "};
    test_serde(&thing, yaml);

    let thing: Option<usize> = None;
    let yaml = indoc! {"
        null
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_char() {
    let thing = '#';
    let yaml = "'#'\n";
    test_serde(&thing, yaml);
}

#[test]
fn test_nested_vec() {
    let thing = vec![vec![1, 2], vec![3]];
    let yaml = indoc! {"
        - - 1
          - 2
        - - 3
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_mapping() {
    let mut thing = Mapping::new();
    thing.insert(Value::String("a".to_owned()), Value::Number(1.into()));
    thing.insert(Value::String("b".to_owned()), Value::Number(2.into()));
    let yaml = indoc! {"
        a: 1
        b: 2
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_value() {
    let mut thing = Mapping::new();
    thing.insert(
        Value::String("type".to_owned()),
        Value::String("primary".to_owned()),
    );
    thing.insert(
        Value::String("config".to_owned()),
        Value::Sequence(vec![
            Value::Null,
            Value::Bool(true),
            Value::Number(65535.into()),
            Value::Number(0.54321.into()),
            Value::String("s".to_owned()),
            Value::Mapping(Mapping::new()),
        ]),
    );
    let yaml = "type: primary\nconfig:\n  - null\n  - true\n  - 65535\n  - 0.54321\n  - s\n  -   - {}\n";
    test_serde(&thing, yaml);
}

#[test]
fn test_long_string() {
    let thing = "a".repeat(80);
    let yaml = format!("{}\n", thing);
    test_serde(&thing.to_owned(), &yaml);
}

#[test]
fn test_multiline_string() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct S {
        trailing_newline: String,
        no_trailing_newline: String,
    }
    let thing = S {
        trailing_newline: "aaa\nbbb\n".to_owned(),
        no_trailing_newline: "aaa\nbbb".to_owned(),
    };
    let yaml = "trailing_newline: \"aaa\\nbbb\\n\"\nno_trailing_newline: \"aaa\\nbbb\"\n";
    test_serde(&thing, yaml);
}

#[test]
fn test_string_escapes() {
    let thing = "\0\x07\x08\t\n\x0b\x0c\r\x1b\"\\\u{0085}\u{2028}\u{2029}";
    let yaml = "\"\0\u{7}\u{8}\\t\\n\u{b}\u{c}\\r\u{1b}\\\"\\\\\u{85}\u{2028}\u{2029}\"\n";
    test_serde(&thing.to_owned(), yaml);
}

#[test]
fn test_nested_enum() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum Outer {
        Inner(Inner),
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum Inner {
        Unit,
    }
    let thing = Outer::Inner(Inner::Unit);
    let yaml = "!Inner null\n";
    test_serde(&thing, yaml);
}

#[test]
fn test_tagged_map_value() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Container {
        profile: Config,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum Config {
        ClassValidator { class_name: String },
    }
    let thing = Container {
        profile: Config::ClassValidator {
            class_name: "ApplicationConfig".to_owned(),
        },
    };
    let yaml = "profile: !ClassValidator\n  class_name: ApplicationConfig\n";
    test_serde(&thing, yaml);
}

#[test]
fn test_boolish_serialization() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct S {
        s: String,
    }
    let thing = S { s: "true".to_owned() };
    let yaml = indoc! {"
        s: 'true'
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_strings_needing_quote() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct S {
        s: String,
    }
    let thing = S { s: "true".to_owned() };
    let yaml = indoc! {"
        s: 'true'
    "};
    test_serde(&thing, yaml);
}

#[test]
fn test_moar_strings_needing_quote() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct S {
        s: String,
    }
    let thing = S { s: " ".to_owned() };
    let yaml = "s:  \n";
    test_serde(&thing, yaml);
}

#[test]
fn test_map_key_value() {
    #[derive(Serialize, Deserialize, PartialEq, Debug, Eq, PartialOrd, Ord)]
    struct S {
        s: String,
    }
    let mut thing = BTreeMap::new();
    thing.insert(S { s: "a".to_owned() }, S { s: "b".to_owned() });
    let yaml = "s: a\n:\n  s: b\n";
    test_serde(&thing, yaml);
}

#[test]
fn test_unit() {
    let thing = ();
    let yaml = indoc! {"
        null
    "};
    test_serde(&thing, yaml);
}
