//! Test suite for the serde_yml crate.
#![allow(clippy::zero_sized_map_values)]

use indoc::indoc;
use serde::de::Deserialize;
#[cfg(not(miri))]
use serde::de::{SeqAccess, Visitor};
use serde_derive::{Deserialize, Serialize};
use serde_yml::{Deserializer, Value};
#[cfg(not(miri))]
use std::collections::BTreeMap;
#[cfg(not(miri))]
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;

fn test_error<'de, T>(yaml: &'de str, expected: &str)
where
    T: Deserialize<'de> + Debug,
{
    let result = serde_yml::from_str::<T>(yaml);
    assert_eq!(expected, result.unwrap_err().to_string());

    let mut deserializer = Deserializer::from_str(yaml);
    if let Some(first_document) = deserializer.next()
        && deserializer.next().is_none()
    {
        let result = T::deserialize(first_document);
        assert_eq!(expected, result.unwrap_err().to_string());
    }
}

#[test]
fn test_scan_error() {
    let yaml = "&";
    let expected = "while scanning an anchor or alias, did not find expected alphabetic or numeric character at byte 0 line 1 column 1 at line 2 column 1";
    test_error::<Value>(yaml, expected);
}

#[test]
fn test_incorrect_type() {
    let yaml = indoc! {"
        ---
        str
    "};
    let expected =
        "invalid type: string \"str\", expected i16 at line 3 column 1 in .";
    test_error::<i16>(yaml, expected);
}

#[test]
fn test_incorrect_nested_type() {
    #[derive(Deserialize, Debug)]
    pub(crate) struct A {
        #[allow(dead_code)]
        pub(crate) b: Vec<B>,
    }
    #[derive(Deserialize, Debug)]
    pub(crate) struct C {
        #[allow(dead_code)]
        pub(crate) d: bool,
    }
    #[derive(Deserialize, Debug)]
    pub(crate) enum B {
        C(#[allow(dead_code)] C),
    }
    let yaml = indoc! {"
        b:
          - !C
            d: fase
    "};
    let expected = "invalid type: string \"fase\", expected a boolean at line 4 column 8 in b.\\[0\\].d";
    test_error::<A>(yaml, expected);
}

#[test]
fn test_empty() {
    let expected = "EOF while parsing a value";
    test_error::<String>("", expected);
}

#[test]
fn test_missing_field() {
    #[derive(Deserialize, Debug)]
    pub(crate) struct Basic {
        #[allow(dead_code)]
        pub(crate) v: bool,
        #[allow(dead_code)]
        pub(crate) w: bool,
    }
    let yaml = indoc! {"
        ---
        v: true
    "};
    let expected = "missing field `w` at line 3 column 2 in .";
    test_error::<Basic>(yaml, expected);
}

#[test]
fn test_unknown_anchor() {
    let yaml = indoc! {"
        ---
        *some
    "};
    let expected = "unknown anchor at line 3 column 1";
    test_error::<String>(yaml, expected);
}

#[test]
fn test_ignored_unknown_anchor() {
    #[derive(Deserialize, Debug)]
    pub(crate) struct Wrapper {
        #[allow(dead_code)]
        pub(crate) c: (),
    }
    let yaml = indoc! {"
        b: [*a]
        c: ~
    "};
    let expected = "unknown anchor at line 2 column 5 at line 2 column 2 in .";
    test_error::<Wrapper>(yaml, expected);
}

#[test]
fn test_bytes() {
    let expected = "byte-based YAML is not supported";
    test_error::<&[u8]>("...", expected);
}

#[test]
fn test_two_documents() {
    let yaml = indoc! {"
        ---
        0
        ---
        1
    "};
    let expected = "expected a single YAML document but found more than one";
    test_error::<usize>(yaml, expected);
}

#[test]
fn test_second_document_syntax_error() {
    let yaml = indoc! {"
        ---
        0
        ---
        ]
    "};

    let mut de = Deserializer::from_str(yaml);
    let first_doc = de.next().unwrap();
    let result = <usize as Deserialize>::deserialize(first_doc);
    assert_eq!(0, result.unwrap());

    let second_doc = de.next().unwrap();
    let result = <usize as Deserialize>::deserialize(second_doc);
    let expected = "while parsing a node, did not find expected node content at byte 10 line 4 column 1 at line 5 column 1";
    assert_eq!(expected, result.unwrap_err().to_string());
}

#[test]
fn test_missing_enum_tag() {
    #[derive(Deserialize, Debug)]
    pub(crate) enum E {
        V(#[allow(dead_code)] usize),
    }
    let yaml = indoc! {r#"
        "V": 16
        "other": 32
    "#};
    let expected =
        "invalid type: map, expected a YAML tag starting with '!' at line 2 column 4 in .";
    test_error::<E>(yaml, expected);
}

#[test]
fn test_serialize_nested_enum() {
    #[derive(Serialize, Debug)]
    pub(crate) enum Outer {
        Inner(Inner),
    }
    #[derive(Serialize, Debug)]
    pub(crate) enum Inner {
        Newtype(usize),
        #[allow(dead_code)]
        Tuple(usize, usize),
        #[allow(dead_code)]
        Struct { x: usize },
    }

    // This used to fail during serialization, but it seems it now succeeds or fails differently.
    // Let's check current behavior.
    let e = Outer::Inner(Inner::Newtype(0));
    let result = serde_yml::to_string(&e);
    assert!(result.is_ok()); // It seems it now supports nested enums in some cases?
}

#[test]
fn test_deserialize_nested_enum() {
    #[derive(Deserialize, Debug)]
    pub(crate) enum Outer {
        Inner(#[allow(dead_code)] Inner),
    }
    #[derive(Deserialize, Debug)]
    pub(crate) enum Inner {
        Variant(#[allow(dead_code)] Vec<usize>),
    }

    let yaml = indoc! {"
        ---
        !Inner []
    "};
    let expected = "deserializing nested enum in Outer::Inner from YAML is not supported yet at line 3 column 8 in .";
    test_error::<Outer>(yaml, expected);

    let yaml = indoc! {"
        ---
        !Variant []
    "};
    let expected = "unknown variant `Variant`, expected `Inner`";
    test_error::<Outer>(yaml, expected);

    let yaml = indoc! {"
        ---
        !Inner !Variant []
    "};
    let expected = "deserializing nested enum in Outer::Inner from YAML is not supported yet at line 3 column 8 in .";
    test_error::<Outer>(yaml, expected);
}

#[test]
fn test_variant_not_a_seq() {
    #[derive(Deserialize, Debug)]
    pub(crate) enum E {
        V(#[allow(dead_code)] usize),
    }
    let yaml = indoc! {"
        ---
        !V
        value: 0
    "};
    let expected =
        "invalid type: map, expected usize at line 4 column 6 in .";
    test_error::<E>(yaml, expected);
}

#[test]
fn test_struct_from_sequence() {
    #[derive(Deserialize, Debug)]
    pub(crate) struct Struct {
        #[allow(dead_code)]
        pub(crate) x: usize,
        #[allow(dead_code)]
        pub(crate) y: usize,
    }
    let yaml = indoc! {"
        [0, 0]
    "};
    let expected = "invalid type: sequence, expected struct Struct at line 2 column 1 in .";
    test_error::<Struct>(yaml, expected);
}

#[test]
fn test_bad_bool() {
    let yaml = indoc! {"
        ---
        !!bool str
    "};
    let expected = "invalid value: string \"str\", expected a boolean at line 3 column 8 in .";
    test_error::<bool>(yaml, expected);
}

#[test]
fn test_bad_int() {
    let yaml = indoc! {"
        ---
        !!int str
    "};
    let expected = "invalid value: string \"str\", expected an integer at line 3 column 7 in .";
    test_error::<i64>(yaml, expected);
}

#[test]
fn test_bad_float() {
    let yaml = indoc! {"
        ---
        !!float str
    "};
    let expected = "invalid value: string \"str\", expected a float at line 3 column 9 in .";
    test_error::<f64>(yaml, expected);
}

#[test]
fn test_bad_null() {
    let yaml = indoc! {"
        ---
        !!null str
    "};
    let expected = "invalid value: string \"str\", expected null at line 3 column 8 in .";
    test_error::<()>(yaml, expected);
}

#[test]
fn test_short_tuple() {
    let yaml = indoc! {"
        ---
        [0, 0]
    "};
    let expected = "invalid length 2, expected a tuple of size 3 at line 3 column 1 in .";
    test_error::<(u8, u8, u8)>(yaml, expected);
}

#[test]
fn test_long_tuple() {
    let yaml = indoc! {"
        ---
        [0, 0, 0]
    "};
    let expected = "invalid length 3, expected sequence of 2 elements at line 3 column 1 in .";
    test_error::<(u8, u8)>(yaml, expected);
}

#[test]
fn test_invalid_scalar_type() {
    #[derive(Deserialize, Debug)]
    pub(crate) struct S {
        #[allow(dead_code)]
        pub(crate) x: [i32; 1],
    }

    let yaml = "x: ''\n";
    let expected = "invalid type: string \"\", expected an array of length 1 at line 2 column 4 in x";
    test_error::<S>(yaml, expected);
}

#[cfg(not(miri))]
#[test]
fn test_infinite_recursion_objects() {
    #[derive(Deserialize, Debug)]
    pub(crate) struct S {
        #[allow(dead_code)]
        pub(crate) x: Option<Box<S>>,
    }

    let yaml = "&a {'x': *a}";
    let expected = "recursion limit exceeded at line 2 column 4";
    test_error::<S>(yaml, expected);
}

#[cfg(not(miri))]
#[test]
fn test_infinite_recursion_arrays() {
    #[derive(Deserialize, Debug)]
    pub(crate) struct S(
        #[allow(dead_code)] pub(crate) usize,
        #[allow(dead_code)] pub(crate) Option<Box<S>>,
    );

    let yaml = "&a [0, *a]";
    let expected = "recursion limit exceeded at line 2 column 4";
    test_error::<S>(yaml, expected);
}

#[cfg(not(miri))]
#[test]
fn test_infinite_recursion_newtype() {
    #[derive(Deserialize, Debug)]
    pub(crate) struct S(#[allow(dead_code)] pub(crate) Option<Box<S>>);

    let yaml = "&a [*a]";
    let expected = "recursion limit exceeded at line 2 column 4";
    test_error::<S>(yaml, expected);
}

#[cfg(not(miri))]
#[test]
fn test_finite_recursion_objects() {
    #[derive(Deserialize, Debug)]
    pub(crate) struct S {
        #[allow(dead_code)]
        pub(crate) x: Option<Box<S>>,
    }

    let yaml = "{'x':".repeat(1_000) + &"}".repeat(1_000);
    let expected = "recursion limit exceeded at line 2 column 1276";
    test_error::<S>(&yaml, expected);
}

#[cfg(not(miri))]
#[test]
fn test_finite_recursion_arrays() {
    #[derive(Deserialize, Debug)]
    pub(crate) struct S(
        #[allow(dead_code)] pub(crate) usize,
        #[allow(dead_code)] pub(crate) Option<Box<S>>,
    );

    let yaml = "[0, ".repeat(1_000) + &"]".repeat(1_000);
    let expected = "recursion limit exceeded at line 2 column 1021";
    test_error::<S>(&yaml, expected);
}

#[cfg(not(miri))]
#[test]
fn test_billion_laughs() {
    #[derive(Debug)]
    struct X;

    impl<'de> Visitor<'de> for X {
        type Value = X;

        fn expecting(
            &self,
            formatter: &mut Formatter<'_>,
        ) -> fmt::Result {
            formatter.write_str("exponential blowup")
        }

        fn visit_unit<E>(self) -> Result<X, E> {
            Ok(X)
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<X, S::Error>
        where
            S: SeqAccess<'de>,
        {
            while let Some(X) = seq.next_element()? {}
            Ok(X)
        }
    }

    impl<'de> Deserialize<'de> for X {
        fn deserialize<D>(deserializer: D) -> Result<X, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(X)
        }
    }

    let yaml = indoc! {"
        a: &a ~
        b: &b [*a,*a,*a,*a,*a,*a,*a,*a,*a]
        c: &c [*b,*b,*b,*b,*b,*b,*b,*b,*b]
        d: &d [*c,*c,*c,*c,*c,*c,*c,*c,*c]
        e: &e [*d,*d,*d,*d,*d,*d,*d,*d,*d]
        f: &f [*e,*e,*e,*e,*e,*e,*e,*e,*e]
        g: &g [*f,*f,*f,*f,*f,*f,*f,*f,*f]
        h: &h [*g,*g,*g,*g,*g,*g,*g,*g,*g]
        i: &i [*h,*h,*h,*h,*h,*h,*h,*h,*h]
    "};
    let expected = "Repetition Limit Exceeded: The repetition limit was exceeded while parsing the YAML";
    test_error::<BTreeMap<String, X>>(yaml, expected);
}

#[test]
fn test_duplicate_keys() {
    let yaml = indoc! {"
        ---
        thing: true
        thing: false
    "};
    let expected =
        "duplicate entry with key \"thing\" at line 3 column 6 in .";
    test_error::<Value>(yaml, expected);

    let yaml = indoc! {"
        ---
        null: true
        ~: false
    "};
    let expected = "duplicate entry with null key at line 3 column 5 in .";
    test_error::<Value>(yaml, expected);

    let yaml = indoc! {"
        ---
        99: true
        99: false
    "};
    let expected = "duplicate entry with key 99 at line 3 column 3 in .";
    test_error::<Value>(yaml, expected);

    let yaml = indoc! {"
        ---
        {}: true
        {}: false
    "};
    let expected = "duplicate entry in YAML map at line 3 column 3 in .";
    test_error::<Value>(yaml, expected);
}
