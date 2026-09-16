# Security Hardening & Zero-Panic Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate unhandled `panic!()` and `unwrap()` points across value deserializers and serializers, replace defensive `unreachable!()` with structured errors, and verify with automated regression tests.

**Architecture:** Replace panicking branches in `MapDeserializer`, `MapRefDeserializer`, and `SerializeMap` with proper `serde::de::Error::custom` and `serde::ser::Error::custom` errors. In `de.rs`, defensively guard anchor retrieval and fallback type reporting.

**Tech Stack:** Rust 2024 edition, `serde`, `serde_yml`.

## Global Constraints

- 100% Safe Rust: No `unsafe` blocks allowed in `src/`.
- Zero Panics: Functions returning `Result<T, E>` must never invoke `panic!()` or `.unwrap()` on visitor or caller input.
- Clippy & Rustfmt: Must compile with `-D warnings` under `cargo clippy` and zero diffs under `cargo fmt --check`.
- Full Test Suite Passing: All 370+ unit/integration tests and 86 doctests must pass without regressions.

---

### Task 1: Eliminate Panics in `MapDeserializer` and `MapRefDeserializer`

**Files:**
- Modify: `src/value/de.rs:755-762`, `src/value/de.rs:1310-1320`
- Test: `tests/test_error.rs`

**Interfaces:**
- Consumes: `serde::de::{MapAccess, Error as DeError}`
- Produces: `Result<T::Value, Error>` returning `Err(Error::custom("visit_value called before visit_key"))` on out-of-order calls.

- [ ] **Step 1: Write the failing test in `tests/test_error.rs`**

```rust
#[test]
fn test_map_deserializer_out_of_order_calls_do_not_panic() {
    use serde::de::{DeserializeSeed, Deserializer, MapAccess, Visitor};
    use std::fmt;

    struct PrematureValueVisitor;
    impl<'de> Visitor<'de> for PrematureValueVisitor {
        type Value = ();
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("premature value test")
        }
        fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
            struct UnitSeed;
            impl<'de> DeserializeSeed<'de> for UnitSeed {
                type Value = ();
                fn deserialize<D: Deserializer<'de>>(self, _d: D) -> Result<(), D::Error> {
                    Ok(())
                }
            }
            // Calling next_value_seed before next_key_seed should return Err, not panic
            assert!(access.next_value_seed(UnitSeed).is_err());
            Ok(())
        }
    }

    let map = serde_yml::Mapping::new();
    let val = serde_yml::Value::Mapping(map.clone());

    // Test owned Value visit_map
    let _ = serde_yml::Value::deserialize_any(val, PrematureValueVisitor);

    // Test borrowed &Mapping visit_map
    let mut map_ref_de = serde_yml::value::de::MapRefDeserializer::new(&map);
    assert!(PrematureValueVisitor.visit_map(&mut map_ref_de).is_ok());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_error test_map_deserializer_out_of_order_calls_do_not_panic`
Expected: FAIL with `panicked at src/value/de.rs:759: 'visit_value called before visit_key'`

- [ ] **Step 3: Implement structured error handling in `src/value/de.rs`**

In `src/value/de.rs`:
```rust
    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(de::Error::custom("visit_value called before visit_key")),
        }
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_error test_map_deserializer_out_of_order_calls_do_not_panic`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/value/de.rs tests/test_error.rs
git commit -m "fix(value): return structured error instead of panicking on premature visit_value"
```

---

### Task 2: Eliminate Panic in `SerializeMap::serialize_value`

**Files:**
- Modify: `src/value/ser.rs:430-440`
- Test: `tests/test_error.rs`

**Interfaces:**
- Consumes: `serde::ser::{SerializeMap, Error as SerError}`
- Produces: `Result<()>` returning `Err(ser::Error::custom("serialize_value called before serialize_key"))`

- [ ] **Step 1: Write the failing test in `tests/test_error.rs`**

```rust
#[test]
fn test_serialize_value_without_key_returns_error() {
    use serde::ser::{SerializeMap, Serializer};

    let mut serializer = serde_yml::value::Serializer;
    let mut map_serializer = serializer.serialize_map(None).unwrap();
    let result = map_serializer.serialize_value(&42);
    assert!(result.is_err(), "Expected error when serialize_value called before serialize_key");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_error test_serialize_value_without_key_returns_error`
Expected: FAIL with `panicked at src/value/ser.rs:435: 'serialize_value called before serialize_key'`

- [ ] **Step 3: Implement graceful error return in `src/value/ser.rs`**

In `src/value/ser.rs`:
```rust
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let (mapping, key) = match self {
            SerializeMap::CheckForTag | SerializeMap::Tagged(_) => {
                unreachable!()
            }
            SerializeMap::Untagged { mapping, next_key } => {
                (mapping, next_key)
            }
        };
        match key.take() {
            Some(key) => {
                mapping.insert(key, to_value(value)?);
                Ok(())
            }
            None => Err(ser::Error::custom(
                "serialize_value called before serialize_key",
            )),
        }
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_error test_serialize_value_without_key_returns_error`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/value/ser.rs tests/test_error.rs
git commit -m "fix(serializer): return error instead of panicking on serialize_value without key"
```

---

### Task 3: Guard `unwrap()` and `unreachable!()` in `src/de.rs`

**Files:**
- Modify: `src/de.rs:295-315`, `src/de.rs:1629-1646`
- Test: `tests/test_de.rs`

**Interfaces:**
- Consumes: `document.anchor_names: BTreeMap<usize, String>`, `event: &Event<'_>`
- Produces: Safe anchor extraction and fallback error creation without panic or abort.

- [ ] **Step 1: Write test in `tests/test_de.rs`**

```rust
#[test]
fn test_anchors_extraction_safe_and_consistent() {
    let yaml = indoc! {"
        item: &my_anchor
          name: target
        ref: *my_anchor
    "};
    let de = serde_yml::Deserializer::from_str(yaml);
    let anchors = de.anchors();
    assert!(anchors.is_some());
    let anchors = anchors.unwrap();
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].anchor_name, "my_anchor");
}
```

- [ ] **Step 2: Replace unwrap and unreachable in `src/de.rs`**

In `src/de.rs`:
Replace `document.anchor_names.get(alias_id).unwrap()` with `if let Some(anchor_name) = document.anchor_names.get(alias_id)`.
Replace `Event::Alias(_) => unreachable!()` with `Event::Alias(_) => de::Error::invalid_type(Unexpected::Other("alias"), exp)`.

- [ ] **Step 3: Verify all tests, clippy, and fmt pass**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-features --all-targets -- -D warnings && cargo test --workspace --all-features`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/de.rs tests/test_de.rs
git commit -m "fix(de): eliminate unwrap in anchors extraction and unreachable in invalid_type"
```

---

### Task 4: Complete Verification & PR Submission

- [ ] **Step 1: Push branch and create PR**
- [ ] **Step 2: Monitor GitHub Actions CI on PR to ensure all checks pass**
