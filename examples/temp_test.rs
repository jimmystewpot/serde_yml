use serde_yml::{Mapping, Value};
use std::collections::BTreeMap;
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct S {
    s: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, PartialOrd, Ord)]
struct SKey {
    s: String,
}

fn main() {
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
    println!("test_value: {:?}", serde_yml::to_string(&thing).unwrap());

    let mut thing = BTreeMap::new();
    thing.insert(SKey { s: "a".to_owned() }, SKey { s: "b".to_owned() });
    println!("test_map_key_value: {:?}", serde_yml::to_string(&thing).unwrap());

    let thing = S { s: " ".to_owned() };
    println!("test_moar_strings_needing_quote: {:?}", serde_yml::to_string(&thing).unwrap());

    let thing = "\0\x07\x08\t\n\x0b\x0c\r\x1b\"\\\u{0085}\u{2028}\u{2029}";
    println!("test_string_escapes: {:?}", serde_yml::to_string(&thing).unwrap());
}
