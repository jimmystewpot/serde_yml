use crate::libyml::error::{Error, Result};
use std::fmt::{self, Debug};
use std::io;

/// Context for tracking nesting in the emitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    /// Inside a sequence, tracks how many items have been emitted.
    Sequence { items: usize },
    /// Inside a mapping, tracks how many scalars have been emitted (key/value pairs).
    Mapping { entries: usize },
}

/// A YAML emitter.
pub struct Emitter<'a, W> {
    writer: W,
    _phantom: std::marker::PhantomData<&'a ()>,
    stack: Vec<Context>,
    need_newline: bool,
    first_item_inline: bool,
}

impl<W> Debug for Emitter<'_, W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Emitter").finish()
    }
}

/// YAML event types.
#[derive(Debug, Clone)]
pub enum Event<'a> {
    /// Start of a YAML stream.
    StreamStart,
    /// End of a YAML stream.
    StreamEnd,
    /// Start of a YAML document.
    DocumentStart,
    /// End of a YAML document.
    DocumentEnd,
    /// Scalar value.
    Scalar(Scalar<'a>),
    /// Start of a sequence.
    SequenceStart(Sequence),
    /// End of a sequence.
    SequenceEnd,
    /// Start of a mapping.
    MappingStart(Mapping),
    /// End of a mapping.
    MappingEnd,
}

/// Represents a scalar value in YAML.
#[derive(Debug, Clone)]
pub struct Scalar<'a> {
    /// Optional tag for the scalar.
    pub tag: Option<String>,
    /// Value of the scalar.
    pub value: &'a str,
    /// Style of the scalar.
    pub style: ScalarStyle,
}

/// Styles for YAML scalars.
#[derive(Clone, Copy, Debug)]
pub enum ScalarStyle {
    /// Any scalar style.
    Any,
    /// Double quoted scalar style.
    DoubleQuoted,
    /// Folded scalar style.
    Folded,
    /// Plain scalar style.
    Plain,
    /// Single quoted scalar style.
    SingleQuoted,
    /// Literal scalar style.
    Literal,
}

/// Represents a YAML sequence.
#[derive(Debug, Clone)]
pub struct Sequence {
    /// Optional tag for the sequence.
    pub tag: Option<String>,
}

/// Represents a YAML mapping.
#[derive(Debug, Clone)]
pub struct Mapping {
    /// Optional tag for the mapping.
    pub tag: Option<String>,
}

impl<'a, W> Emitter<'a, W>
where
    W: io::Write,
{
    /// Creates a new YAML emitter.
    pub fn new(write: W) -> Emitter<'a, W> {
        Emitter {
            writer: write,
            _phantom: std::marker::PhantomData,
            stack: Vec::new(),
            need_newline: false,
            first_item_inline: false,
        }
    }

    fn io_err(e: io::Error) -> Error {
        Error {
            problem: format!("IO error: {}", e),
            problem_offset: 0,
            problem_mark: Default::default(),
            context: None,
            context_mark: Default::default(),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.writer.write_all(buf).map_err(Self::io_err)
    }

    fn indent_depth(&self) -> usize {
        let mut depth = 0;
        for ctx in &self.stack {
            match ctx {
                Context::Mapping { .. } => depth += 1,
                Context::Sequence { .. } => depth += 1,
            }
        }
        if depth > 0 { depth - 1 } else { 0 }
    }

    fn write_indent(&mut self) -> Result<()> {
        let depth = self.indent_depth();
        for _ in 0..depth {
            self.write_all(b"  ")?;
        }
        Ok(())
    }

    fn is_mapping_key(&self) -> bool {
        matches!(self.stack.last(), Some(Context::Mapping { entries }) if entries % 2 == 0)
    }

    fn is_mapping_value(&self) -> bool {
        matches!(self.stack.last(), Some(Context::Mapping { entries }) if entries % 2 == 1)
    }

    fn increment_parent(&mut self) {
        match self.stack.last_mut() {
            Some(Context::Sequence { items }) => *items += 1,
            Some(Context::Mapping { entries }) => *entries += 1,
            None => {}
        }
    }

    /// Emits a YAML event.
    pub fn emit(&mut self, event: Event<'_>) -> Result<()> {
        match event {
            Event::StreamStart | Event::StreamEnd => {}
            Event::DocumentStart | Event::DocumentEnd => {}
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
                        self.write_all(scalar.value.as_bytes())?;
                        self.write_all(b"\"")?;
                    }
                    _ => {
                        self.write_all(scalar.value.as_bytes())?;
                    }
                }

                if !is_value && !is_key {
                    self.write_all(b"\n")?;
                    self.need_newline = false;
                } else if is_value {
                    self.need_newline = true;
                } else if is_key {
                    self.need_newline = false;
                }

                self.increment_parent();
            }
            Event::SequenceStart(seq) => {
                let in_seq = matches!(
                    self.stack.last(),
                    Some(Context::Sequence { .. })
                );
                let is_value = self.is_mapping_value();
                let inline = self.first_item_inline;

                if self.first_item_inline {
                    self.first_item_inline = false;
                    if in_seq {
                        self.write_all(b"- ")?;
                    }
                } else if in_seq {
                    self.write_indent()?;
                    self.write_all(b"- ")?;
                } else if is_value {
                    self.write_all(b":")?;
                }

                if let Some(ref tag) = seq.tag {
                    if !is_value && !in_seq && !inline {
                        if self.need_newline {
                            self.write_all(b"\n")?;
                        }
                        self.write_indent()?;
                    }
                    self.write_all(tag.as_bytes())?;
                    self.write_all(b"\n")?;
                    self.need_newline = false;
                    self.first_item_inline = false;
                } else if is_value {
                    self.write_all(b"\n")?;
                    self.need_newline = false;
                    self.first_item_inline = false;
                } else {
                    self.first_item_inline = true;
                }

                self.increment_parent();
                self.stack.push(Context::Sequence { items: 0 });
            }
            Event::SequenceEnd => {
                let was_empty = matches!(
                    self.stack.last(),
                    Some(Context::Sequence { items: 0 })
                );
                self.stack.pop();

                if was_empty {
                    let is_key = self.is_mapping_key();
                    let is_value = self.is_mapping_value();
                    let in_seq = matches!(
                        self.stack.last(),
                        Some(Context::Sequence { .. })
                    );

                    if is_value {
                        self.write_all(b": ")?;
                    } else if is_key || in_seq {
                        self.write_indent()?;
                        if in_seq {
                            self.write_all(b"- ")?;
                        }
                    }
                    if !is_value && !is_key && !in_seq {
                        self.write_indent()?;
                    }
                    self.write_all(b"[]\n")?;
                    self.need_newline = false;
                    self.increment_parent();
                } else if self.need_newline {
                    self.write_all(b"\n")?;
                    self.need_newline = false;
                }
            }
            Event::MappingStart(mapping) => {
                let in_seq = matches!(
                    self.stack.last(),
                    Some(Context::Sequence { .. })
                );
                let is_value = self.is_mapping_value();
                let inline = self.first_item_inline;

                if self.first_item_inline {
                    self.first_item_inline = false;
                } else if in_seq {
                    self.write_indent()?;
                    self.write_all(b"- ")?;
                } else if is_value {
                    self.write_all(b":")?;
                }

                if let Some(ref tag) = mapping.tag {
                    if !is_value && !in_seq && !inline {
                        if self.need_newline {
                            self.write_all(b"\n")?;
                        }
                        self.write_indent()?;
                    }
                    self.write_all(tag.as_bytes())?;
                    self.write_all(b"\n")?;
                    self.need_newline = false;
                    self.first_item_inline = false;
                } else if is_value {
                    self.write_all(b"\n")?;
                    self.need_newline = false;
                    self.first_item_inline = false;
                } else {
                    self.first_item_inline = true;
                }

                self.increment_parent();
                self.stack.push(Context::Mapping { entries: 0 });
            }
            Event::MappingEnd => {
                let was_empty = matches!(
                    self.stack.last(),
                    Some(Context::Mapping { entries: 0 })
                );
                self.stack.pop();

                if was_empty {
                    let is_key = self.is_mapping_key();
                    let is_value = self.is_mapping_value();
                    let in_seq = matches!(
                        self.stack.last(),
                        Some(Context::Sequence { .. })
                    );

                    if is_value {
                        self.write_all(b": ")?;
                    } else if is_key || in_seq {
                        self.write_indent()?;
                        if in_seq {
                            self.write_all(b"- ")?;
                        }
                    }
                    if !is_value && !is_key && !in_seq {
                        self.write_indent()?;
                    }
                    self.write_all(b"{}\n")?;
                    self.need_newline = false;
                    self.increment_parent();
                } else if self.need_newline {
                    self.write_all(b"\n")?;
                    self.need_newline = false;
                }
            }
        }

        Ok(())
    }

    /// Flushes the YAML emitter.
    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush().map_err(Self::io_err)
    }

    /// Retrieves the inner writer from the YAML emitter.
    pub fn into_inner(self) -> W {
        self.writer
    }
}
