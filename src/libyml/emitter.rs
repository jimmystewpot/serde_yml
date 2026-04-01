use crate::libyml::error::{Error, Result};
use std::fmt::{self, Debug};
use std::io;

/// Context for tracking nesting in the emitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(variant_size_differences)]
enum Context {
    /// Inside a sequence, tracks index.
    Sequence { index: usize },
    /// Inside a mapping, tracks whether next event is key or value.
    Mapping { is_key: bool },
}

/// A YAML emitter that writes events to a writer.
pub struct Emitter<'a, W> {
    writer: W,
    stack: Vec<Context>,
    need_newline: bool,
    first_item_inline: bool,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<W> Debug for Emitter<'_, W>
where
    W: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Emitter")
            .field("writer", &self.writer)
            .field("stack", &self.stack)
            .field("need_newline", &self.need_newline)
            .field("first_item_inline", &self.first_item_inline)
            .finish()
    }
}

/// Represents a YAML scalar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scalar<'a> {
    /// The optional tag of the scalar.
    pub tag: Option<String>,
    /// The value of the scalar.
    pub value: &'a str,
    /// The style of the scalar.
    pub style: ScalarStyle,
}

/// Represents the style of a YAML scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarStyle {
    /// Plain scalar style.
    Plain,
    /// Single-quoted scalar style.
    SingleQuoted,
    /// Double-quoted scalar style.
    DoubleQuoted,
    /// Literal scalar style.
    Literal,
    /// Folded scalar style.
    Folded,
}

/// Represents a YAML sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sequence {
    /// The optional tag of the sequence.
    pub tag: Option<String>,
}

/// Represents a YAML mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    /// The optional tag of the mapping.
    pub tag: Option<String>,
}

/// Represents a YAML event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<'a> {
    /// Indicates the start of a YAML stream.
    StreamStart,
    /// Indicates the end of a YAML stream.
    StreamEnd,
    /// Indicates the start of a YAML document.
    DocumentStart,
    /// Indicates the end of a YAML document.
    DocumentEnd,
    /// Indicates a YAML scalar.
    Scalar(Scalar<'a>),
    /// Indicates the start of a YAML sequence.
    SequenceStart(Sequence),
    /// Indicates the end of a YAML sequence.
    SequenceEnd,
    /// Indicates the start of a YAML mapping.
    MappingStart(Mapping),
    /// Indicates the end of a YAML mapping.
    MappingEnd,
}

impl<W> Emitter<'_, W>
where
    W: io::Write,
{
    /// Creates a new YAML emitter.
    pub fn new(writer: W) -> Self {
        Emitter {
            writer,
            stack: Vec::new(),
            need_newline: false,
            first_item_inline: false,
            _marker: std::marker::PhantomData,
        }
    }

    /// Emits a YAML event.
    pub fn emit(&mut self, event: Event<'_>) -> Result<()> {
        match event {
            Event::StreamStart | Event::StreamEnd => {}
            Event::DocumentStart => {
                if self.need_newline {
                    self.write_all(b"\n")?;
                }
                self.write_all(b"---\n")?;
                self.need_newline = false;
            }
            Event::DocumentEnd => {
                self.need_newline = true;
            }
            Event::Scalar(scalar) => {
                let in_seq = matches!(
                    self.stack.last(),
                    Some(Context::Sequence { .. })
                );
                let is_key = self.is_mapping_key();
                let is_value = self.is_mapping_value();

                if self.first_item_inline {
                    self.first_item_inline = false;
                    if in_seq {
                        self.write_all(b"- ")?;
                    }
                } else if in_seq {
                    self.write_indent()?;
                    self.write_all(b"- ")?;
                } else if is_key {
                    if self.need_newline {
                        self.write_all(b"\n")?;
                    }
                    self.write_indent()?;
                } else if is_value {
                    self.write_all(b": ")?;
                } else if self.need_newline {
                    self.write_all(b"\n")?;
                }

                if let Some(ref tag) = scalar.tag {
                    self.write_all(tag.as_bytes())?;
                    self.write_all(b" ")?;
                }

                match scalar.style {
                    ScalarStyle::SingleQuoted => {
                        self.write_all(b"'")?;
                        self.write_all(scalar.value.as_bytes())?;
                        self.write_all(b"'")?;
                    }
                    ScalarStyle::DoubleQuoted => {
                        self.write_all(b"\"")?;
                        for c in scalar.value.chars() {
                            match c {
                                '"' => self.write_all(b"\\\"")?,
                                '\\' => self.write_all(b"\\\\")?,
                                '\n' => self.write_all(b"\\n")?,
                                '\r' => self.write_all(b"\\r")?,
                                '\t' => self.write_all(b"\\t")?,
                                _ => {
                                    let mut b = [0u8; 4];
                                    self.write_all(c.encode_utf8(&mut b).as_bytes())?;
                                }
                            }
                        }
                        self.write_all(b"\"")?;
                    }
                    ScalarStyle::Literal => {
                        self.write_all(b"|-\n")?;
                        let depth = self.indent_depth() + 1;
                        let mut lines = scalar.value.lines().peekable();
                        while let Some(line) = lines.next() {
                            for _ in 0..depth {
                                self.write_all(b"  ")?;
                            }
                            self.write_all(line.as_bytes())?;
                            if lines.peek().is_some() || scalar.value.ends_with('\n') {
                                self.write_all(b"\n")?;
                            }
                        }
                    }
                    ScalarStyle::Folded => {
                        self.write_all(b">-\n")?;
                        let depth = self.indent_depth() + 1;
                        let mut lines = scalar.value.lines().peekable();
                        while let Some(line) = lines.next() {
                            if line.is_empty() {
                                self.write_all(b"\n")?;
                            } else {
                                for _ in 0..depth {
                                    self.write_all(b"  ")?;
                                }
                                self.write_all(line.as_bytes())?;
                                if lines.peek().is_some() || scalar.value.ends_with('\n') {
                                    self.write_all(b"\n")?;
                                }
                            }
                        }
                    }
                    _ => {
                        self.write_all(scalar.value.as_bytes())?;
                    }
                }

                if is_value {
                    self.need_newline = true;
                } else if is_key {
                    self.need_newline = false;
                } else if !matches!(scalar.style, ScalarStyle::Literal | ScalarStyle::Folded) {
                    self.write_all(b"\n")?;
                    self.need_newline = false;
                } else {
                    self.need_newline = true;
                }

                self.increment_parent();
            }
            Event::SequenceStart(seq) => {
                let in_seq = matches!(
                    self.stack.last(),
                    Some(Context::Sequence { .. })
                );
                let is_value = self.is_mapping_value();

                if self.first_item_inline {
                    self.first_item_inline = false;
                    if in_seq {
                        self.write_all(b"- ")?;
                    }
                } else if in_seq {
                    self.write_indent()?;
                    self.write_all(b"- ")?;
                } else if is_value {
                    self.write_all(b": ")?;
                } else if self.need_newline {
                    self.write_all(b"\n")?;
                }

                if let Some(ref tag) = seq.tag {
                    self.write_all(tag.as_bytes())?;
                    self.write_all(b"\n")?;
                    self.need_newline = false;
                }

                self.stack.push(Context::Sequence { index: 0 });
            }
            Event::SequenceEnd => {
                self.stack.pop();
                self.need_newline = true;
                self.increment_parent();
            }
            Event::MappingStart(mapping) => {
                let in_seq = matches!(
                    self.stack.last(),
                    Some(Context::Sequence { .. })
                );
                let is_value = self.is_mapping_value();

                if self.first_item_inline {
                    self.first_item_inline = false;
                    if in_seq {
                        self.write_all(b"- ")?;
                    }
                } else if in_seq {
                    self.write_indent()?;
                    self.write_all(b"- ")?;
                } else if is_value {
                    self.write_all(b": ")?;
                } else if self.need_newline {
                    self.write_all(b"\n")?;
                }

                if let Some(ref tag) = mapping.tag {
                    self.write_all(tag.as_bytes())?;
                    self.write_all(b"\n")?;
                    self.need_newline = false;
                } else if is_value {
                    self.write_all(b"\n")?;
                    self.need_newline = false;
                } else {
                    self.first_item_inline = true;
                }

                self.stack.push(Context::Mapping { is_key: true });
            }
            Event::MappingEnd => {
                self.stack.pop();
                self.need_newline = true;
                self.increment_parent();
            }
        }
        Ok(())
    }

    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data).map_err(Self::io_err)
    }

    fn write_indent(&mut self) -> Result<()> {
        let depth = self.indent_depth();
        for _ in 0..depth {
            self.write_all(b"  ")?;
        }
        Ok(())
    }

    fn indent_depth(&self) -> usize {
        self.stack.len()
    }

    fn is_mapping_key(&self) -> bool {
        matches!(self.stack.last(), Some(Context::Mapping { is_key: true }))
    }

    fn is_mapping_value(&self) -> bool {
        matches!(self.stack.last(), Some(Context::Mapping { is_key: false }))
    }

    fn increment_parent(&mut self) {
        if let Some(ctx) = self.stack.last_mut() {
            match ctx {
                Context::Sequence { index } => *index += 1,
                Context::Mapping { is_key } => *is_key = !*is_key,
            }
        }
    }

    fn io_err(err: io::Error) -> Error {
        Error::new(err.to_string())
    }

    /// Flushes the underlying writer of the YAML emitter.
    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush().map_err(Self::io_err)
    }

    /// Retrieves the inner writer from the YAML emitter.
    pub fn into_inner(self) -> W {
        self.writer
    }
}
