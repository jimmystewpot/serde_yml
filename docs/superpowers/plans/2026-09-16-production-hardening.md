# Production Hardening & Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Address the 6 post-review refinement items in `serde_yml` to eliminate latent IO flush hazards, eradicate remaining heap allocations in emitter container commits and control escapes, remove dead container fields, and improve code clarity to achieve an unconditional Grade A+.

**Architecture:** Refactor `to_writer` to finalize and flush its inner serializer with `into_inner()`. Replace the intermediate `.collect()` in `commit_pending_parents` with zero-allocation `remove(0)` draining. Drop dead `tag` fields from committed `SequenceState` and `MappingState`. Replace `format!` in `write_scalar_content` with a stack-buffered zero-allocation hex encoder. Rename misleading `has_control` variable in `serialize_str`.

**Tech Stack:** Rust 2021 edition, `serde`, `itoa`, `ryu`, standard library IO traits (`io::Write`, `BufWriter`).

**Spec:** [`brainstorming_review.md`](file:///home/jalamb/.gemini/antigravity/brain/1b8790a9-3b50-4696-a297-3f9a867bbe5c/brainstorming_review.md)

## Global Constraints

- Zero `.unwrap()` calls in production code (`src/`); all errors must propagate via `Result`.
- Zero `.clone()` allocations on hot serialization pathways.
- Zero `unsafe` blocks across the crate.
- Zero compiler warnings under `cargo clippy --workspace --all-features --all-targets --no-deps -- -D warnings`.
- Strict YAML 1.2 specification compliance and 100% pass rate across existing 370+ test suite.

---

### Task 1: Fix `to_writer` Lifecycle and IO Flush Hazard

**Files:**
- Modify: `src/ser.rs:736-748`
- Test: `tests/test_ser.rs`

**Interfaces:**
- Consumes: `Serializer::new`, `Serializer::into_inner`
- Produces: `pub fn to_writer<'a, W, T>(writer: W, value: &T) -> Result<()>` guaranteeing `StreamEnd` and `flush()` on completion.

- [x] **Step 1: Write the failing test for `to_writer` flushing**

In `tests/test_ser.rs`, add a test verifying that `to_writer` flushes into a custom tracking writer that detects if `flush()` was invoked:

```rust
#[test]
fn test_to_writer_flushes_output() {
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};

    struct FlushTrackingWriter {
        buffer: Vec<u8>,
        flushed: Arc<Mutex<bool>>,
    }

    impl Write for FlushTrackingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buffer.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            *self.flushed.lock().unwrap() = true;
            self.buffer.flush()
        }
    }

    let flushed = Arc::new(Mutex::new(false));
    let writer = FlushTrackingWriter {
        buffer: Vec::new(),
        flushed: Arc::clone(&flushed),
    };

    serde_yml::to_writer(writer, &"test_val").unwrap();
    assert!(*flushed.lock().unwrap(), "to_writer must flush the underlying writer");
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_ser test_to_writer_flushes_output`
Expected: FAIL (assertion failed: `*flushed.lock().unwrap()`)

- [x] **Step 3: Implement `to_writer` fix via `into_inner()`**

In `src/ser.rs`:
```rust
pub fn to_writer<'a, W, T>(writer: W, value: &T) -> Result<()>
where
    W: io::Write + 'a,
    T: ?Sized + ser::Serialize,
{
    let mut serializer = Serializer::new(writer)?;
    value.serialize(&mut serializer)?;
    serializer.into_inner()?;
    Ok(())
}
```

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_ser test_to_writer_flushes_output`
Expected: PASS

- [x] **Step 5: Run full test suite to check for regressions**

Run: `cargo test --all-targets`
Expected: PASS (all tests pass)

- [x] **Step 6: Commit**

```bash
git add src/ser.rs tests/test_ser.rs
git commit -m "fix(ser): finalize and flush serializer in to_writer via into_inner"
```

---

### Task 2: Eliminate Intermediate Allocation in `commit_pending_parents`

**Files:**
- Modify: `src/libyml/emitter.rs:295-305`
- Test: `tests/test_ser.rs`, `tests/test_serde.rs`

**Interfaces:**
- Consumes: `self.pending: Vec<PendingContainer>`
- Produces: `fn commit_pending_parents(&mut self) -> Result<()>` with zero heap allocations.

- [x] **Step 1: Write a unit test exercising nested pending parents commit**

In `tests/test_ser.rs`, add:
```rust
#[test]
fn test_deeply_nested_empty_containers_commit() {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<&str, BTreeMap<&str, Vec<String>>> = BTreeMap::new();
    let mut inner = BTreeMap::new();
    inner.insert("items", Vec::new());
    map.insert("nested", inner);

    let yaml = serde_yml::to_string(&map).unwrap();
    assert_eq!(yaml, "nested:\n  items: []\n");
}
```

- [x] **Step 2: Run test to verify it runs and passes on current code**

Run: `cargo test --test test_ser test_deeply_nested_empty_containers_commit`
Expected: PASS

- [x] **Step 3: Refactor `commit_pending_parents` to remove `.collect::<Vec<_>>()`**

In `src/libyml/emitter.rs`:
```rust
    fn commit_pending_parents(&mut self) -> Result<()> {
        while self.pending.len() > 1 {
            let container = self.pending.remove(0);
            self.commit_container(container)?;
        }
        Ok(())
    }
```

- [x] **Step 4: Run test suite to verify correctness without heap allocation**

Run: `cargo test --all-targets`
Expected: PASS

- [x] **Step 5: Commit**

```bash
git add src/libyml/emitter.rs tests/test_ser.rs
git commit -m "perf(emitter): eliminate intermediate Vec allocation in commit_pending_parents"
```

---

### Task 3: Drop Dead `tag` Fields from Container Stack States

**Files:**
- Modify: `src/libyml/emitter.rs:12-29,338-346,379-389,400-408,429-437,469-478,488-497`
- Test: `tests/test_serde.rs`

**Interfaces:**
- Consumes: `PendingContainer::{Sequence, Mapping}` with `Option<String>` tag
- Produces: `SequenceState { indent: usize, count: usize, inlined_first: bool }`, `MappingState { indent: usize, count: usize, is_key: bool, inlined_first: bool }` without dead `tag` fields.

- [x] **Step 1: Verify dead code warning when `#[allow(dead_code)]` is removed**

Remove `#[allow(dead_code)]` from `tag` in `SequenceState` and `MappingState` in `src/libyml/emitter.rs`.
Run: `cargo check`
Expected: Warning: `fields 'tag' are never read`

- [x] **Step 2: Refactor `SequenceState` and `MappingState` to remove `tag`**

In `src/libyml/emitter.rs`:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct SequenceState {
    indent: usize,
    count: usize,
    inlined_first: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MappingState {
    indent: usize,
    count: usize,
    is_key: bool,
    inlined_first: bool,
}
```
Update all constructor call sites in `commit_container` where `SequenceState` and `MappingState` are pushed to omit `tag: ...`. Tags continue to be formatted and emitted directly to `self.writer` during `commit_container`, as they are never referenced again after commitment.

- [x] **Step 3: Run compiler checks to verify zero warnings**

Run: `cargo clippy --workspace --all-features --all-targets --no-deps -- -D warnings`
Expected: PASS with 0 warnings and 0 errors.

- [x] **Step 4: Run test suite**

Run: `cargo test --all-targets`
Expected: PASS

- [x] **Step 5: Commit**

```bash
git add src/libyml/emitter.rs
git commit -m "refactor(emitter): remove unused tag field from SequenceState and MappingState"
```

---

### Task 4: Zero-Allocation Escape Formatting for C0 and Unicode Control Characters

**Files:**
- Modify: `src/libyml/emitter.rs:817-832`
- Test: `tests/test_serde.rs:test_string_escapes`

**Interfaces:**
- Consumes: `c: char`
- Produces: `write_scalar_content` writing hex escapes directly from stack buffers (`[u8; 10]`) with zero `format!` allocations.

- [x] **Step 1: Inspect existing test coverage in `tests/test_serde.rs`**

Run: `cargo test --test test_serde test_string_escapes`
Expected: PASS

- [x] **Step 2: Implement stack-buffered hex writing in `src/libyml/emitter.rs`**

Add a helper function for stack-allocated hex writing:
```rust
fn write_hex_escape(writer: &mut dyn io::Write, prefix: u8, val: u32, digits: usize) -> io::Result<()> {
    const HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";
    let mut buf = [0u8; 10];
    buf[0] = b'\\';
    buf[1] = prefix;
    for i in 0..digits {
        let shift = (digits - 1 - i) * 4;
        let nibble = ((val >> shift) & 0xF) as usize;
        buf[2 + i] = HEX_CHARS[nibble];
    }
    writer.write_all(&buf[..2 + digits])
}
```

In `write_scalar_content`, replace lines 817-832:
```rust
c if (c as u32) < 0x20 => {
    write_hex_escape(&mut self.writer, b'x', c as u32, 2)
        .map_err(Self::io_err)?;
}
c if c.is_control() => {
    let code = c as u32;
    if code <= 0xFF {
        write_hex_escape(&mut self.writer, b'x', code, 2)
            .map_err(Self::io_err)?;
    } else if code <= 0xFFFF {
        write_hex_escape(&mut self.writer, b'u', code, 4)
            .map_err(Self::io_err)?;
    } else {
        write_hex_escape(&mut self.writer, b'U', code, 8)
            .map_err(Self::io_err)?;
    }
}
```

- [x] **Step 3: Run `test_string_escapes` to verify matching output**

Run: `cargo test --test test_serde test_string_escapes`
Expected: PASS

- [x] **Step 4: Run full test suite and clippy**

Run: `cargo test --all-targets && cargo clippy --workspace --all-features --all-targets --no-deps -- -D warnings`
Expected: PASS

- [x] **Step 5: Commit**

```bash
git add src/libyml/emitter.rs
git commit -m "perf(emitter): replace format! with stack-buffered hex escape writer"
```

---

### Task 5: Refactor String Quoting Variable Naming and Clarity

**Files:**
- Modify: `src/ser.rs:344-372`
- Test: `tests/test_serde.rs`

**Interfaces:**
- Consumes: `v: &str`
- Produces: `serialize_str` with clear, intent-revealing variable names.

- [x] **Step 1: Check `serialize_str` implementation in `src/ser.rs`**

Notice `has_control` currently matches `"` and `\`. Rename `has_control` to `needs_double_quotes` to reflect the actual semantic requirement of double quoting in YAML.

- [x] **Step 2: Update `src/ser.rs`**

```rust
    fn serialize_str(self, v: &str) -> Result<()> {
        let needs_double_quotes = v.chars().any(|c| {
            (c < ' ' && c != '\n')
                || c == '\x7f'
                || c == '"'
                || c == '\\'
                || c == '\u{85}'
                || c == '\u{2028}'
                || c == '\u{2029}'
                || c == '\u{feff}'
        });
        let style = if needs_double_quotes {
            ScalarStyle::DoubleQuoted
        } else if v.contains('\n') {
            ScalarStyle::Literal
        } else if crate::de::ambiguous_string(v)
            || v.starts_with(' ')
            || v.ends_with(' ')
            || v.is_empty()
            || v.starts_with([
                '@', '`', '&', '*', '!', '|', '>', '\'', '"', '%', '?',
                ':', '-', '[', '{', ']', '}', ',',
            ])
            || v.contains(": ")
            || v.contains(" #")
        {
            ScalarStyle::SingleQuoted
        } else {
            ScalarStyle::Plain
        };
        self.emit_scalar(Scalar {
            tag: None,
            value: v,
            style,
        })
    }
```

- [x] **Step 3: Run test suite**

Run: `cargo test --all-targets`
Expected: PASS

- [x] **Step 4: Commit**

```bash
git add src/ser.rs
git commit -m "refactor(ser): rename has_control to needs_double_quotes for semantic clarity"
```

---

### Task 6: Final Verification, Linting, and Documentation Audit

**Files:**
- Modify: `token_usage.md`
- Test: Full crate suite

**Interfaces:**
- Verification: All test binaries, doc tests, format, and clippy gates.

- [x] **Step 1: Format check**

Run: `cargo fmt --check`
Expected: PASS (clean exit)

- [x] **Step 2: Clippy with deny warnings**

Run: `cargo clippy --workspace --all-features --all-targets --no-deps -- -D warnings`
Expected: PASS (0 warnings, 0 errors)

- [x] **Step 3: All targets test execution**

Run: `cargo test --all-targets`
Expected: PASS (370+ tests pass)

- [x] **Step 4: Documentation test execution**

Run: `cargo test --doc`
Expected: PASS (86 doctests pass)

- [x] **Step 5: Audit unwrap/clone/unsafe counters**

Run:
```bash
grep -rn "unwrap()" src/ --include="*.rs"
grep -rn "unsafe" src/ --include="*.rs"
```
Expected: 0 occurrences in production code.

- [x] **Step 6: Update token usage dashboard and walkthrough**

Update `token_usage.md` and `walkthrough.md` with final metrics.
