use crate::libyml::error::{Error, Result};
use std::fmt::{self, Debug};
use std::io;
use yaml_rust2::Yaml;
use hashlink::LinkedHashMap;

/// A YAML emitter.
pub struct Emitter<'a, W> {
    writer: W,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<W> Debug for Emitter<'_, W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Emitter")
            .finish()
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
        }
    }

    /// Emits a YAML event.
    pub fn emit(&mut self, event: Event<'_>) -> Result<()> {
        match event {
            Event::Scalar(scalar) => {
                self.writer.write_all(scalar.value.as_bytes()).map_err(|e| Error {
                    problem: format!("IO error: {}", e),
                    problem_offset: 0,
                    problem_mark: Default::default(),
                    context: None,
                    context_mark: Default::default(),
                })?;
                self.writer.write_all(b"\n").map_err(|e| Error {
                    problem: format!("IO error: {}", e),
                    problem_offset: 0,
                    problem_mark: Default::default(),
                    context: None,
                    context_mark: Default::default(),
                })?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Flushes the YAML emitter.
    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush().map_err(|e| Error {
            problem: format!("IO error: {}", e),
            problem_offset: 0,
            problem_mark: Default::default(),
            context: None,
            context_mark: Default::default(),
        })
    }

    /// Retrieves the inner writer from the YAML emitter.
    pub fn into_inner(self) -> W {
        self.writer
    }
}

#[allow(dead_code)]
fn events_to_yaml(events: &[Event<'_>], start: usize) -> Option<(Yaml, usize)> {
    if start >= events.len() {
        return None;
    }

    match &events[start] {
        Event::Scalar(scalar) => {
            let yaml = if scalar.value == "null" {
                Yaml::Null
            } else if scalar.value == "true" {
                Yaml::Boolean(true)
            } else if scalar.value == "false" {
                Yaml::Boolean(false)
            } else if let Ok(i) = scalar.value.parse::<i64>() {
                Yaml::Integer(i)
            } else if let Ok(f) = scalar.value.parse::<f64>() {
                Yaml::Real(f.to_string())
            } else {
                Yaml::String(scalar.value.to_string())
            };
            Some((yaml, start + 1))
        }
        Event::SequenceStart(_) => {
            let mut items = Vec::new();
            let mut pos = start + 1;
            while pos < events.len() {
                if matches!(events[pos], Event::SequenceEnd) {
                    return Some((Yaml::Array(items), pos + 1));
                }
                if let Some((yaml, next_pos)) = events_to_yaml(events, pos) {
                    items.push(yaml);
                    pos = next_pos;
                } else {
                    pos += 1;
                }
            }
            Some((Yaml::Array(items), pos))
        }
        Event::MappingStart(_) => {
            let mut map = LinkedHashMap::new();
            let mut pos = start + 1;
            while pos < events.len() {
                if matches!(events[pos], Event::MappingEnd) {
                    return Some((Yaml::Hash(map), pos + 1));
                }
                if let Some((key_yaml, next_pos)) = events_to_yaml(events, pos) {
                    if let Some((val_yaml, final_pos)) = events_to_yaml(events, next_pos) {
                        map.insert(key_yaml, val_yaml);
                        pos = final_pos;
                    } else {
                        pos = next_pos;
                    }
                } else {
                    pos += 1;
                }
            }
            Some((Yaml::Hash(map), pos))
        }
        _ => None,
    }
}
