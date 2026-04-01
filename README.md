<!-- markdownlint-disable MD033 MD041 -->

<img src="https://kura.pro/serde_yml/images/logos/serde_yml.svg"
alt="Serde YML logo" width="66" align="right" />

<!-- markdownlint-enable MD033 MD041 -->

# Serde YML (a fork of Serde YAML)

[![Made With Love][made-with-rust]][11] [![Crates.io][crates-badge]][07] [![lib.rs][libs-badge]][12] [![Docs.rs][docs-badge]][08] [![Codecov][codecov-badge]][09] [![Build Status][build-badge]][10] [![GitHub][github-badge]][06]

[Serde YML][00] is a robust Rust library for using the [Serde][01] serialization framework with data in [YAML][05] file format. It focuses on reliability, performance, and compliance with the YAML 1.2 standard.

## Features

- **Full Serde Support:** Serialization and deserialization of any Rust data structure implementing Serde's traits.
- **YAML 1.2 Compliant:** Improved support for YAML 1.2, including correct float formatting (`.inf`, `.nan`) and tag handling.
- **Enum Tagging:** Robust support for YAML's `!tag` syntax for idiomatic representation of Rust enum variants.
- **Support for Large Integers:** Full support for `i128` and `u128` types.
- **Rich Scalar Styles:** Support for Literal (`|`) and Folded (`>`) block scalars for multiline strings.
- **Direct Value Access:** Flexible access to YAML documents through the `Value` type, `Mapping`, and `Sequence`.
- **Informative Errors:** Clear, descriptive error messages with precise location information (line and column).
- **BOM Handling:** Automatic handling and stripping of the UTF-8 Byte Order Mark (BOM).
- **Complex Keys:** Support for complex mapping keys using the YAML `?` syntax.
- **Advanced Enum Modules:** Specialized modules like `singleton_map` and `singleton_map_recursive` for alternative enum serialization strategies.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
serde = "1.0"
serde_yml = "1.0"
```

## Usage

Here's a quick example on how to use Serde YML to serialize and deserialize a struct to and from YAML:

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Point {
    x: f64,
    y: f64,
}

fn main() -> Result<(), serde_yml::Error> {
    let point = Point { x: 1.0, y: 2.0 };

    // Serialize to YAML
    let yaml = serde_yml::to_string(&point)?;
    assert_eq!(yaml, "x: 1.0\ny: 2.0\n");

    // Deserialize from YAML
    let deserialized_point: Point = serde_yml::from_str(&yaml)?;
    assert_eq!(point, deserialized_point);

    Ok(())
}
```

## Enum Serialization

By default, Serde YML uses YAML tags to represent Rust enum variants, providing a clean and idiomatic YAML representation.

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
}

fn main() -> Result<(), serde_yml::Error> {
    let messages = vec![
        Message::Quit,
        Message::Move { x: 10, y: 20 },
        Message::Write("Hello".to_string()),
    ];

    let yaml = serde_yml::to_string(&messages)?;
    // Output:
    // - !Quit null
    // - !Move
    //   x: 10
    //   y: 20
    // - !Write Hello

    let deserialized: Vec<Message> = serde_yml::from_str(&yaml)?;
    assert_eq!(messages, deserialized);
    Ok(())
}
```

## Documentation

For full API documentation, please visit [https://docs.rs/serde-yml][08].

## Rust Version Compatibility

Compiler support: requires rustc 1.70.0+

## Examples

Serde YML provides a set of comprehensive examples. You can find them in the
`examples` directory of the project. To run the examples, clone the repository
and execute the following command in your terminal from the project:

```shell
cargo run --example example
```

## Contributing

Contributions are welcome! Please submit a Pull Request on [GitHub][06].

## Credits and Acknowledgements

Serde YML is a continuation of the excellent work done by [David Tolnay][03] and the maintainers of the [serde-yaml][02] library. While Serde YML has evolved into a separate library with its own emitter and parser integration, we express our sincere gratitude to them for their foundational contributions to the Rust community.

## License

Licensed under either of the [Apache License](LICENSE-APACHE) or the
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

[00]: https://serdeyml.com
[01]: https://github.com/serde-rs/serde
[02]: https://github.com/dtolnay/serde-yaml
[03]: https://github.com/dtolnay
[05]: https://yaml.org/
[06]: https://github.com/sebastienrousseau/serde_yml
[07]: https://crates.io/crates/serde_yml
[08]: https://docs.rs/serde_yml
[09]: https://codecov.io/gh/sebastienrousseau/serde_yml
[10]: https://github.com/sebastienrousseau/serde-yml/actions?query=branch%3Amaster
[11]: https://www.rust-lang.org/
[12]: https://lib.rs/crates/serde_yml
[build-badge]: https://img.shields.io/github/actions/workflow/status/sebastienrousseau/serde_yml/release.yml?branch=master&style=for-the-badge&logo=github "Build Status"
[codecov-badge]: https://img.shields.io/codecov/c/github/sebastienrousseau/serde_yml?style=for-the-badge&token=Q9KJ6XXL67&logo=codecov "Codecov"
[crates-badge]: https://img.shields.io/crates/v/serde_yml.svg?style=for-the-badge&color=fc8d62&logo=rust "Crates.io"
[libs-badge]: https://img.shields.io/badge/lib.rs-v1.0.0-orange.svg?style=for-the-badge "View on lib.rs"
[docs-badge]: https://img.shields.io/badge/docs.rs-serde__yml-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs "Docs.rs"
[github-badge]: https://img.shields.io/badge/github-sebastienrousseau/serde--yml-8da0cb?style=for-the-badge&labelColor=555555&logo=github "GitHub"
[made-with-rust]: https://img.shields.io/badge/rust-f04041?style=for-the-badge&labelColor=c0282d&logo=rust 'Made With Rust'
