use crate::libyml::error::{Error, Result};
use std::fmt::{self, Debug};
use std::io;

/// Context for tracking containers on the emitter stack.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Container {
    Sequence(SequenceState),
    Mapping(MappingState),
}

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

/// A pending container start that has not yet been determined to be empty or non-empty.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingContainer {
    Sequence { tag: Option<String> },
    Mapping { tag: Option<String> },
}

enum ParentInfo {
    None,
    Sequence { indent: usize, inlined_first: bool },
    Mapping { indent: usize },
}

enum ScalarParentInfo {
    None,
    Sequence { indent: usize, inlined_first: bool },
    MappingKey { indent: usize, inlined_first: bool },
    MappingValue { indent: usize },
}

/// A YAML emitter that writes events to a writer.
pub struct Emitter<'a, W> {
    writer: W,
    stack: Vec<Container>,
    pending: Vec<PendingContainer>,
    need_separator: bool,
    has_emitted_content: bool,
    at_subsequent_document_start: bool,
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
            .field("pending", &self.pending)
            .field("need_separator", &self.need_separator)
            .field("has_emitted_content", &self.has_emitted_content)
            .field(
                "at_subsequent_document_start",
                &self.at_subsequent_document_start,
            )
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

fn format_tag(tag: &str) -> String {
    if tag.starts_with('!') {
        tag.to_string()
    } else {
        format!("!<{}>", tag)
    }
}

fn write_hex_escape(
    writer: &mut dyn io::Write,
    prefix: u8,
    val: u32,
    digits: usize,
) -> io::Result<()> {
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

impl<W> Emitter<'_, W>
where
    W: io::Write,
{
    /// Creates a new YAML emitter.
    pub fn new(writer: W) -> Self {
        Emitter {
            writer,
            stack: Vec::new(),
            pending: Vec::new(),
            need_separator: false,
            has_emitted_content: false,
            at_subsequent_document_start: false,
            _marker: std::marker::PhantomData,
        }
    }

    /// Emits a YAML event.
    pub fn emit(&mut self, event: Event<'_>) -> Result<()> {
        match event {
            Event::StreamStart => {}
            Event::StreamEnd => {
                if self.at_subsequent_document_start {
                    self.write_all(b"---\n")?;
                    self.at_subsequent_document_start = false;
                }
                self.flush()?;
            }
            Event::DocumentStart => {
                self.commit_all_pending()?;
                if !self.has_emitted_content {
                    self.write_all(b"---\n")?;
                    self.need_separator = false;
                } else {
                    self.at_subsequent_document_start = true;
                    self.need_separator = false;
                }
            }
            Event::DocumentEnd => {
                self.commit_all_pending()?;
                if self.at_subsequent_document_start {
                    self.write_all(b"---\n")?;
                    self.at_subsequent_document_start = false;
                }
                self.stack.clear();
                self.need_separator = true;
            }
            Event::SequenceStart(seq) => {
                self.pending
                    .push(PendingContainer::Sequence { tag: seq.tag });
            }
            Event::SequenceEnd => {
                if matches!(
                    self.pending.last(),
                    Some(PendingContainer::Sequence { .. })
                ) {
                    self.commit_pending_parents()?;
                    if let Some(PendingContainer::Sequence { tag }) =
                        self.pending.pop()
                    {
                        self.emit_empty_sequence(tag)?;
                    }
                } else {
                    self.stack.pop();
                    self.on_child_container_end();
                }
            }
            Event::MappingStart(mapping) => {
                self.pending.push(PendingContainer::Mapping {
                    tag: mapping.tag,
                });
            }
            Event::MappingEnd => {
                if matches!(
                    self.pending.last(),
                    Some(PendingContainer::Mapping { .. })
                ) {
                    self.commit_pending_parents()?;
                    if let Some(PendingContainer::Mapping { tag }) =
                        self.pending.pop()
                    {
                        self.emit_empty_mapping(tag)?;
                    }
                } else {
                    self.stack.pop();
                    self.on_child_container_end();
                }
            }
            Event::Scalar(scalar) => {
                self.commit_all_pending()?;
                self.emit_scalar_internal(scalar)?;
            }
        }
        Ok(())
    }

    fn parent_info(&self) -> ParentInfo {
        match self.stack.last() {
            None => ParentInfo::None,
            Some(Container::Sequence(s)) => ParentInfo::Sequence {
                indent: s.indent,
                inlined_first: s.inlined_first,
            },
            Some(Container::Mapping(m)) => {
                ParentInfo::Mapping { indent: m.indent }
            }
        }
    }

    fn scalar_parent_info(&self) -> ScalarParentInfo {
        match self.stack.last() {
            None => ScalarParentInfo::None,
            Some(Container::Sequence(s)) => {
                ScalarParentInfo::Sequence {
                    indent: s.indent,
                    inlined_first: s.inlined_first,
                }
            }
            Some(Container::Mapping(m)) => {
                if m.is_key {
                    ScalarParentInfo::MappingKey {
                        indent: m.indent,
                        inlined_first: m.inlined_first,
                    }
                } else {
                    ScalarParentInfo::MappingValue { indent: m.indent }
                }
            }
        }
    }

    fn on_child_container_end(&mut self) {
        if let Some(parent) = self.stack.last_mut() {
            match parent {
                Container::Mapping(map) => {
                    map.is_key = true;
                    map.count += 1;
                }
                Container::Sequence(_) => {}
            }
        }
    }

    fn commit_pending_parents(&mut self) -> Result<()> {
        while self.pending.len() > 1 {
            let container = self.pending.remove(0);
            self.commit_container(container)?;
        }
        Ok(())
    }

    fn commit_all_pending(&mut self) -> Result<()> {
        let to_commit = std::mem::take(&mut self.pending);
        for container in to_commit {
            self.commit_container(container)?;
        }
        Ok(())
    }

    fn commit_container(
        &mut self,
        container: PendingContainer,
    ) -> Result<()> {
        let parent = self.parent_info();
        match container {
            PendingContainer::Sequence { tag } => {
                let formatted_tag = tag.as_deref().map(format_tag);
                match parent {
                    ParentInfo::None => {
                        self.has_emitted_content = true;
                        if self.at_subsequent_document_start {
                            self.at_subsequent_document_start = false;
                            if let Some(ref t) = formatted_tag {
                                self.write_all(b"--- ")?;
                                self.write_all(t.as_bytes())?;
                                self.write_all(b"\n")?;
                            } else {
                                self.write_all(b"---\n")?;
                            }
                        } else if let Some(ref t) = formatted_tag {
                            self.write_all(t.as_bytes())?;
                            self.write_all(b"\n")?;
                        }
                        self.stack.push(Container::Sequence(
                            SequenceState {
                                indent: 0,
                                count: 0,
                                inlined_first: false,
                            },
                        ));
                    }
                    ParentInfo::Sequence {
                        indent,
                        inlined_first,
                    } => {
                        let new_indent = indent + 2;
                        let inlined = if inlined_first {
                            if let Some(ref t) = formatted_tag {
                                self.write_all(t.as_bytes())?;
                                self.write_all(b"\n")?;
                                false
                            } else {
                                self.write_all(b"- ")?;
                                true
                            }
                        } else {
                            self.write_indent_spaces(indent)?;
                            self.write_all(b"- ")?;
                            if let Some(ref t) = formatted_tag {
                                self.write_all(t.as_bytes())?;
                                self.write_all(b"\n")?;
                                false
                            } else {
                                self.write_all(b"- ")?;
                                true
                            }
                        };
                        if let Some(Container::Sequence(s)) =
                            self.stack.last_mut()
                        {
                            s.inlined_first = false;
                            s.count += 1;
                        }
                        self.stack.push(Container::Sequence(
                            SequenceState {
                                indent: new_indent,
                                count: 0,
                                inlined_first: inlined,
                            },
                        ));
                    }
                    ParentInfo::Mapping { indent } => {
                        if let Some(ref t) = formatted_tag {
                            self.write_all(b": ")?;
                            self.write_all(t.as_bytes())?;
                            self.write_all(b"\n")?;
                        } else {
                            self.write_all(b":\n")?;
                        }
                        self.stack.push(Container::Sequence(
                            SequenceState {
                                indent,
                                count: 0,
                                inlined_first: false,
                            },
                        ));
                    }
                }
            }
            PendingContainer::Mapping { tag } => {
                let formatted_tag = tag.as_deref().map(format_tag);
                match parent {
                    ParentInfo::None => {
                        self.has_emitted_content = true;
                        if self.at_subsequent_document_start {
                            self.at_subsequent_document_start = false;
                            if let Some(ref t) = formatted_tag {
                                self.write_all(b"--- ")?;
                                self.write_all(t.as_bytes())?;
                                self.write_all(b"\n")?;
                            } else {
                                self.write_all(b"---\n")?;
                            }
                        } else if let Some(ref t) = formatted_tag {
                            self.write_all(t.as_bytes())?;
                            self.write_all(b"\n")?;
                        }
                        self.stack.push(Container::Mapping(
                            MappingState {
                                indent: 0,
                                count: 0,
                                is_key: true,
                                inlined_first: false,
                            },
                        ));
                    }
                    ParentInfo::Sequence {
                        indent,
                        inlined_first,
                    } => {
                        let new_indent = indent + 2;
                        let inlined = if inlined_first {
                            if let Some(ref t) = formatted_tag {
                                self.write_all(t.as_bytes())?;
                                self.write_all(b"\n")?;
                                false
                            } else {
                                true
                            }
                        } else {
                            self.write_indent_spaces(indent)?;
                            self.write_all(b"- ")?;
                            if let Some(ref t) = formatted_tag {
                                self.write_all(t.as_bytes())?;
                                self.write_all(b"\n")?;
                                false
                            } else {
                                true
                            }
                        };
                        if let Some(Container::Sequence(s)) =
                            self.stack.last_mut()
                        {
                            s.inlined_first = false;
                            s.count += 1;
                        }
                        self.stack.push(Container::Mapping(
                            MappingState {
                                indent: new_indent,
                                count: 0,
                                is_key: true,
                                inlined_first: inlined,
                            },
                        ));
                    }
                    ParentInfo::Mapping { indent } => {
                        if let Some(ref t) = formatted_tag {
                            self.write_all(b": ")?;
                            self.write_all(t.as_bytes())?;
                            self.write_all(b"\n")?;
                        } else {
                            self.write_all(b":\n")?;
                        }
                        let new_indent = indent + 2;
                        self.stack.push(Container::Mapping(
                            MappingState {
                                indent: new_indent,
                                count: 0,
                                is_key: true,
                                inlined_first: false,
                            },
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn emit_empty_sequence(
        &mut self,
        tag: Option<String>,
    ) -> Result<()> {
        let parent = self.parent_info();
        let formatted_tag = tag.as_deref().map(format_tag);
        match parent {
            ParentInfo::None => {
                self.has_emitted_content = true;
                if self.at_subsequent_document_start {
                    self.at_subsequent_document_start = false;
                    self.write_all(b"--- ")?;
                }
                if let Some(ref t) = formatted_tag {
                    self.write_all(t.as_bytes())?;
                    self.write_all(b" []\n")?;
                } else {
                    self.write_all(b"[]\n")?;
                }
            }
            ParentInfo::Sequence {
                indent,
                inlined_first,
            } => {
                if inlined_first {
                    if let Some(ref t) = formatted_tag {
                        self.write_all(t.as_bytes())?;
                        self.write_all(b" []\n")?;
                    } else {
                        self.write_all(b"[]\n")?;
                    }
                } else {
                    self.write_indent_spaces(indent)?;
                    self.write_all(b"- ")?;
                    if let Some(ref t) = formatted_tag {
                        self.write_all(t.as_bytes())?;
                        self.write_all(b" []\n")?;
                    } else {
                        self.write_all(b"[]\n")?;
                    }
                }
                if let Some(Container::Sequence(s)) =
                    self.stack.last_mut()
                {
                    s.inlined_first = false;
                    s.count += 1;
                }
            }
            ParentInfo::Mapping { .. } => {
                if let Some(ref t) = formatted_tag {
                    self.write_all(b": ")?;
                    self.write_all(t.as_bytes())?;
                    self.write_all(b" []\n")?;
                } else {
                    self.write_all(b": []\n")?;
                }
                if let Some(Container::Mapping(m)) =
                    self.stack.last_mut()
                {
                    m.is_key = true;
                    m.count += 1;
                }
            }
        }
        Ok(())
    }

    fn emit_empty_mapping(
        &mut self,
        tag: Option<String>,
    ) -> Result<()> {
        let parent = self.parent_info();
        let formatted_tag = tag.as_deref().map(format_tag);
        match parent {
            ParentInfo::None => {
                self.has_emitted_content = true;
                if self.at_subsequent_document_start {
                    self.at_subsequent_document_start = false;
                    self.write_all(b"--- ")?;
                }
                if let Some(ref t) = formatted_tag {
                    self.write_all(t.as_bytes())?;
                    self.write_all(b" {}\n")?;
                } else {
                    self.write_all(b"{}\n")?;
                }
            }
            ParentInfo::Sequence {
                indent,
                inlined_first,
            } => {
                if inlined_first {
                    if let Some(ref t) = formatted_tag {
                        self.write_all(t.as_bytes())?;
                        self.write_all(b" {}\n")?;
                    } else {
                        self.write_all(b"{}\n")?;
                    }
                } else {
                    self.write_indent_spaces(indent)?;
                    self.write_all(b"- ")?;
                    if let Some(ref t) = formatted_tag {
                        self.write_all(t.as_bytes())?;
                        self.write_all(b" {}\n")?;
                    } else {
                        self.write_all(b"{}\n")?;
                    }
                }
                if let Some(Container::Sequence(s)) =
                    self.stack.last_mut()
                {
                    s.inlined_first = false;
                    s.count += 1;
                }
            }
            ParentInfo::Mapping { .. } => {
                if let Some(ref t) = formatted_tag {
                    self.write_all(b": ")?;
                    self.write_all(t.as_bytes())?;
                    self.write_all(b" {}\n")?;
                } else {
                    self.write_all(b": {}\n")?;
                }
                if let Some(Container::Mapping(m)) =
                    self.stack.last_mut()
                {
                    m.is_key = true;
                    m.count += 1;
                }
            }
        }
        Ok(())
    }

    fn emit_scalar_internal(
        &mut self,
        scalar: Scalar<'_>,
    ) -> Result<()> {
        let formatted_tag = scalar.tag.as_deref().map(format_tag);
        let parent = self.scalar_parent_info();
        match parent {
            ScalarParentInfo::None => {
                self.has_emitted_content = true;
                if self.at_subsequent_document_start {
                    self.at_subsequent_document_start = false;
                    if matches!(
                        scalar.style,
                        ScalarStyle::Literal | ScalarStyle::Folded
                    ) && scalar.tag.is_none()
                    {
                        self.write_all(b"---\n")?;
                    } else {
                        self.write_all(b"--- ")?;
                    }
                }
                if let Some(ref t) = formatted_tag {
                    self.write_all(t.as_bytes())?;
                    if !scalar.value.is_empty() {
                        self.write_all(b" ")?;
                    }
                }
                if !scalar.value.is_empty() || scalar.tag.is_none() {
                    self.write_scalar_content(&scalar, 0)?;
                }
                if !matches!(
                    scalar.style,
                    ScalarStyle::Literal | ScalarStyle::Folded
                ) {
                    self.write_all(b"\n")?;
                }
            }
            ScalarParentInfo::Sequence {
                indent,
                inlined_first,
            } => {
                if inlined_first {
                    if let Some(ref t) = formatted_tag {
                        self.write_all(t.as_bytes())?;
                        if !scalar.value.is_empty() {
                            self.write_all(b" ")?;
                        }
                    }
                    if !scalar.value.is_empty() || scalar.tag.is_none()
                    {
                        self.write_scalar_content(&scalar, indent)?;
                    }
                    if !matches!(
                        scalar.style,
                        ScalarStyle::Literal | ScalarStyle::Folded
                    ) {
                        self.write_all(b"\n")?;
                    }
                } else {
                    self.write_indent_spaces(indent)?;
                    self.write_all(b"- ")?;
                    if let Some(ref t) = formatted_tag {
                        self.write_all(t.as_bytes())?;
                        if !scalar.value.is_empty() {
                            self.write_all(b" ")?;
                        }
                    }
                    if !scalar.value.is_empty() || scalar.tag.is_none()
                    {
                        self.write_scalar_content(&scalar, indent)?;
                    }
                    if !matches!(
                        scalar.style,
                        ScalarStyle::Literal | ScalarStyle::Folded
                    ) {
                        self.write_all(b"\n")?;
                    }
                }
                if let Some(Container::Sequence(s)) =
                    self.stack.last_mut()
                {
                    s.inlined_first = false;
                    s.count += 1;
                }
            }
            ScalarParentInfo::MappingKey {
                indent,
                inlined_first,
            } => {
                if !inlined_first {
                    self.write_indent_spaces(indent)?;
                }
                if let Some(ref t) = formatted_tag {
                    self.write_all(t.as_bytes())?;
                    if !scalar.value.is_empty() {
                        self.write_all(b" ")?;
                    }
                }
                if !scalar.value.is_empty() || scalar.tag.is_none() {
                    self.write_scalar_content(&scalar, indent)?;
                }
                if let Some(Container::Mapping(m)) =
                    self.stack.last_mut()
                {
                    m.inlined_first = false;
                    m.is_key = false;
                }
            }
            ScalarParentInfo::MappingValue { indent } => {
                self.write_all(b": ")?;
                if let Some(ref t) = formatted_tag {
                    self.write_all(t.as_bytes())?;
                    if !scalar.value.is_empty() {
                        self.write_all(b" ")?;
                    }
                }
                if !scalar.value.is_empty() || scalar.tag.is_none() {
                    self.write_scalar_content(&scalar, indent)?;
                }
                if !matches!(
                    scalar.style,
                    ScalarStyle::Literal | ScalarStyle::Folded
                ) {
                    self.write_all(b"\n")?;
                }
                if let Some(Container::Mapping(m)) =
                    self.stack.last_mut()
                {
                    m.is_key = true;
                    m.count += 1;
                }
            }
        }
        Ok(())
    }

    fn write_scalar_content(
        &mut self,
        scalar: &Scalar<'_>,
        indent: usize,
    ) -> Result<()> {
        match scalar.style {
            ScalarStyle::Plain => {
                self.write_all(scalar.value.as_bytes())?;
            }
            ScalarStyle::SingleQuoted => {
                self.write_all(b"'")?;
                for c in scalar.value.chars() {
                    if c == '\'' {
                        self.write_all(b"''")?;
                    } else {
                        let mut b = [0u8; 4];
                        self.write_all(
                            c.encode_utf8(&mut b).as_bytes(),
                        )?;
                    }
                }
                self.write_all(b"'")?;
            }
            ScalarStyle::DoubleQuoted => {
                self.write_all(b"\"")?;
                for c in scalar.value.chars() {
                    match c {
                        '"' => self.write_all(b"\\\"")?,
                        '\\' => self.write_all(b"\\\\")?,
                        '\0' => self.write_all(b"\\0")?,
                        '\x07' => self.write_all(b"\\a")?,
                        '\x08' => self.write_all(b"\\b")?,
                        '\t' => self.write_all(b"\\t")?,
                        '\n' => self.write_all(b"\\n")?,
                        '\x0b' => self.write_all(b"\\v")?,
                        '\x0c' => self.write_all(b"\\f")?,
                        '\r' => self.write_all(b"\\r")?,
                        '\x1b' => self.write_all(b"\\e")?,
                        '\u{0085}' => self.write_all(b"\\N")?,
                        '\u{00a0}' => self.write_all(b"\\_")?,
                        '\u{2028}' => self.write_all(b"\\L")?,
                        '\u{2029}' => self.write_all(b"\\P")?,
                        '\u{feff}' => self.write_all(b"\\uFEFF")?,
                        c if (c as u32) < 0x20 => {
                            write_hex_escape(
                                &mut self.writer,
                                b'x',
                                c as u32,
                                2,
                            )
                            .map_err(Self::io_err)?;
                        }
                        c if c.is_control() => {
                            let code = c as u32;
                            if code <= 0xFF {
                                write_hex_escape(
                                    &mut self.writer,
                                    b'x',
                                    code,
                                    2,
                                )
                                .map_err(Self::io_err)?;
                            } else if code <= 0xFFFF {
                                write_hex_escape(
                                    &mut self.writer,
                                    b'u',
                                    code,
                                    4,
                                )
                                .map_err(Self::io_err)?;
                            } else {
                                write_hex_escape(
                                    &mut self.writer,
                                    b'U',
                                    code,
                                    8,
                                )
                                .map_err(Self::io_err)?;
                            }
                        }
                        c => {
                            let mut b = [0u8; 4];
                            self.write_all(
                                c.encode_utf8(&mut b).as_bytes(),
                            )?;
                        }
                    }
                }
                self.write_all(b"\"")?;
            }
            ScalarStyle::Literal => {
                let header: &[u8] = if scalar.value.ends_with('\n') {
                    b"|\n"
                } else {
                    b"|-\n"
                };
                self.write_all(header)?;
                for line in scalar.value.lines() {
                    self.write_indent_spaces(indent + 2)?;
                    self.write_all(line.as_bytes())?;
                    self.write_all(b"\n")?;
                }
            }
            ScalarStyle::Folded => {
                let header: &[u8] = if scalar.value.ends_with('\n') {
                    b">\n"
                } else {
                    b">-\n"
                };
                self.write_all(header)?;
                let mut lines = scalar.value.lines().peekable();
                while let Some(line) = lines.next() {
                    if line.is_empty() {
                        self.write_all(b"\n")?;
                    } else {
                        self.write_indent_spaces(indent + 2)?;
                        self.write_all(line.as_bytes())?;
                        if lines.peek().is_some() {
                            self.write_all(b"\n\n")?;
                        } else {
                            self.write_all(b"\n")?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn write_indent_spaces(&mut self, count: usize) -> Result<()> {
        const SPACES: &[u8] = b"                                                                ";
        let mut remaining = count;
        while remaining > 0 {
            let chunk = remaining.min(SPACES.len());
            self.write_all(&SPACES[..chunk])?;
            remaining -= chunk;
        }
        Ok(())
    }

    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data).map_err(Self::io_err)
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
