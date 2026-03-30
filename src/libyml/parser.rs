use crate::libyml::{
    error::{Error, Mark, Result},
    tag::Tag,
};
use std::{
    borrow::Cow,
    collections::VecDeque,
    fmt::{self, Debug},
};
use yaml_rust2::parser::{
    Event as YamlEvent, MarkedEventReceiver, Parser as YamlParser,
};
use yaml_rust2::scanner::{Marker, TScalarStyle};

/// Represents a YAML parser.
#[derive(Debug)]
pub struct Parser<'input> {
    events: VecDeque<(Event<'input>, Mark)>,
    _input: Cow<'input, [u8]>,
}

/// Represents a YAML event encountered during parsing.
#[derive(Debug)]
pub enum Event<'input> {
    /// Indicates the start of a YAML stream.
    StreamStart,
    /// Indicates the end of a YAML stream.
    StreamEnd,
    /// Indicates the start of a YAML document.
    DocumentStart,
    /// Indicates the end of a YAML document.
    DocumentEnd,
    /// Indicates an alias to an anchor in a YAML document.
    Alias(Anchor),
    /// Represents a scalar value in a YAML document.
    Scalar(Scalar<'input>),
    /// Indicates the start of a sequence in a YAML document.
    SequenceStart(SequenceStart),
    /// Indicates the end of a sequence in a YAML document.
    SequenceEnd,
    /// Indicates the start of a mapping in a YAML document.
    MappingStart(MappingStart),
    /// Indicates the end of a mapping in a YAML document.
    MappingEnd,
}

/// Represents a scalar value in a YAML document.
pub struct Scalar<'input> {
    /// The anchor associated with the scalar value.
    pub anchor: Option<Anchor>,
    /// The tag associated with the scalar value.
    pub tag: Option<Tag>,
    /// The value of the scalar as a byte slice.
    pub value: Box<[u8]>,
    /// The style of the scalar value.
    pub style: ScalarStyle,
    /// The representation of the scalar value as a byte slice.
    pub repr: Option<&'input [u8]>,
}

/// Represents the start of a sequence in a YAML document.
#[derive(Debug)]
pub struct SequenceStart {
    /// The anchor associated with the sequence.
    pub anchor: Option<Anchor>,
    /// The tag associated with the sequence.
    pub tag: Option<Tag>,
}

/// Represents the start of a mapping in a YAML document.
#[derive(Debug)]
pub struct MappingStart {
    /// The anchor associated with the mapping.
    pub anchor: Option<Anchor>,
    /// The tag associated with the mapping.
    pub tag: Option<Tag>,
}

/// Represents an anchor in a YAML document.
#[derive(Ord, PartialOrd, Eq, PartialEq, Clone)]
pub struct Anchor(Box<[u8]>);

/// Represents the style of a scalar value in a YAML document.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ScalarStyle {
    /// Indicates a plain scalar value.
    Plain,
    /// Indicates a single-quoted scalar value.
    SingleQuoted,
    /// Indicates a double-quoted scalar value.
    DoubleQuoted,
    /// Indicates a literal scalar value.
    Literal,
    /// Indicates a folded scalar value.
    Folded,
}

impl<'input> Parser<'input> {
    /// Creates a new `Parser` instance with the given input data.
    ///
    /// Returns an error if the input is not valid UTF-8 or if the YAML
    /// parser encounters a syntax error.
    pub fn new(input: Cow<'input, [u8]>) -> Result<Parser<'input>> {
        let input_str =
            std::str::from_utf8(&input).map_err(|e| Error {
                problem: format!("input is not valid UTF-8: {}", e),
                problem_offset: e.valid_up_to() as u64,
                problem_mark: Mark {
                    index: e.valid_up_to() as u64,
                    line: 0,
                    column: 0,
                },
                context: None,
                context_mark: Mark::default(),
            })?;

        let mut events = VecDeque::new();
        let mut parser = YamlParser::new(input_str.chars());
        let mut collector = EventCollector {
            events: &mut events,
            anchors: Vec::new(),
        };

        if let Err(scan_error) = parser.load(&mut collector, true) {
            return Err(Error {
                problem: scan_error.to_string(),
                problem_offset: 0,
                problem_mark: Mark {
                    index: scan_error.marker().index() as u64,
                    line: scan_error.marker().line() as u64,
                    column: scan_error.marker().col() as u64,
                },
                context: None,
                context_mark: Mark::default(),
            });
        }

        Ok(Parser {
            events,
            _input: input,
        })
    }

    /// Parses the next YAML event from the input.
    pub fn parse_next_event(
        &mut self,
    ) -> Result<(Event<'input>, Mark)> {
        self.events.pop_front().ok_or_else(|| Error {
            problem: "Unexpected end of event stream".to_string(),
            problem_offset: 0,
            problem_mark: Mark::default(),
            context: None,
            context_mark: Mark::default(),
        })
    }

    /// Checks if the parser is initialized and ready to parse YAML.
    pub fn is_ok(&self) -> bool {
        true
    }
}

struct EventCollector<'a, 'input> {
    events: &'a mut VecDeque<(Event<'input>, Mark)>,
    anchors: Vec<Anchor>,
}

impl MarkedEventReceiver for EventCollector<'_, '_> {
    fn on_event(&mut self, ev: YamlEvent, mark: Marker) {
        let my_mark = Mark {
            index: mark.index() as u64,
            line: mark.line() as u64,
            column: mark.col() as u64,
        };

        let event = match ev {
            YamlEvent::StreamStart => Event::StreamStart,
            YamlEvent::StreamEnd => Event::StreamEnd,
            YamlEvent::DocumentStart => Event::DocumentStart,
            YamlEvent::DocumentEnd => Event::DocumentEnd,
            YamlEvent::Alias(id) => {
                if id > 0 && id <= self.anchors.len() {
                    Event::Alias(self.anchors[id - 1].clone())
                } else {
                    Event::Alias(Anchor(Box::from(
                        b"unknown".as_slice(),
                    )))
                }
            }
            YamlEvent::Scalar(val, style, id, tag) => {
                let anchor = if id > 0 {
                    let a = Anchor(Box::from(
                        format!("&{}", id).into_bytes(),
                    ));
                    if id > self.anchors.len() {
                        self.anchors.resize(id, a.clone());
                    }
                    self.anchors[id - 1] = a.clone();
                    Some(a)
                } else {
                    None
                };
                Event::Scalar(Scalar {
                    anchor,
                    tag: tag.map(|t| Tag::new(&format!("{:?}", t))),
                    value: Box::from(val.as_bytes()),
                    style: match style {
                        TScalarStyle::Plain => ScalarStyle::Plain,
                        TScalarStyle::SingleQuoted => {
                            ScalarStyle::SingleQuoted
                        }
                        TScalarStyle::DoubleQuoted => {
                            ScalarStyle::DoubleQuoted
                        }
                        TScalarStyle::Literal => ScalarStyle::Literal,
                        TScalarStyle::Folded => ScalarStyle::Folded,
                    },
                    repr: None,
                })
            }
            YamlEvent::SequenceStart(id, tag) => {
                let anchor = if id > 0 {
                    let a = Anchor(Box::from(
                        format!("&{}", id).into_bytes(),
                    ));
                    if id > self.anchors.len() {
                        self.anchors.resize(id, a.clone());
                    }
                    self.anchors[id - 1] = a.clone();
                    Some(a)
                } else {
                    None
                };
                Event::SequenceStart(SequenceStart {
                    anchor,
                    tag: tag.map(|t| Tag::new(&format!("{:?}", t))),
                })
            }
            YamlEvent::SequenceEnd => Event::SequenceEnd,
            YamlEvent::MappingStart(id, tag) => {
                let anchor = if id > 0 {
                    let a = Anchor(Box::from(
                        format!("&{}", id).into_bytes(),
                    ));
                    if id > self.anchors.len() {
                        self.anchors.resize(id, a.clone());
                    }
                    self.anchors[id - 1] = a.clone();
                    Some(a)
                } else {
                    None
                };
                Event::MappingStart(MappingStart {
                    anchor,
                    tag: tag.map(|t| Tag::new(&format!("{:?}", t))),
                })
            }
            YamlEvent::MappingEnd => Event::MappingEnd,
            YamlEvent::Nothing => return,
        };

        self.events.push_back((event, my_mark));
    }
}

impl Debug for Scalar<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Scalar")
            .field("anchor", &self.anchor)
            .field("tag", &self.tag)
            .field("value", &String::from_utf8_lossy(&self.value))
            .field("style", &self.style)
            .finish()
    }
}

impl Debug for Anchor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", String::from_utf8_lossy(&self.0))
    }
}

#[cfg(test)]
mod tests {
    use crate::{de::Progress, loader::Loader};

    #[test]
    fn can_load_document_with_16_spaces_value() {
        let hardcoded = "t: a                abc";
        let progress = Progress::Str(hardcoded);
        let mut loader = Loader::new(progress).unwrap();
        let _document = loader.next_document().unwrap();
    }
}
