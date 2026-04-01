use crate::libyml::error as libyml_error;
use serde::{de, ser};
use std::{
    error::Error as StdError,
    fmt::{self, Debug, Display},
    io, result, string,
    sync::Arc,
};

/// Represents a position in the YAML input.
#[derive(Debug)]
pub struct Pos {
    /// The mark representing the position.
    pub mark: libyml_error::Mark,
    /// The path to the position.
    pub path: String,
}

/// The input location where an error occurred.
#[derive(Clone, Copy, Debug)]
pub struct Location {
    /// The byte index of the error.
    index: usize,
    /// The line of the error.
    line: usize,
    /// The column of the error.
    column: usize,
}

impl Location {
    /// Returns the byte index where the error occurred.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns the line number where the error occurred.
    pub fn line(&self) -> usize {
        self.line
    }

    /// Returns the column number where the error occurred.
    pub fn column(&self) -> usize {
        self.column
    }

    // This function is intended for internal use only to maintain decoupling with the yaml crate.
    #[doc(hidden)]
    pub fn from_mark(mark: libyml_error::Mark) -> Self {
        Location {
            index: mark.index() as usize,
            // `line` and `column` returned from libyml are 0-indexed but all error messages add +1 to this value.
            line: mark.line() as usize + 1,
            column: mark.column() as usize + 1,
        }
    }
}

/// An error that occurred during YAML serialization or deserialization.
pub struct Error(Box<ErrorImpl>);

/// Alias for a `Result` with the error type `serde_yml::Error`.
pub type Result<T> = result::Result<T, Error>;

/// The internal representation of an error.
#[derive(Debug)]
pub enum ErrorImpl {
    /// A generic error message with an optional position.
    Message(String, Option<Pos>),
    /// An error originating from the `libyml` library.
    Libyml(libyml_error::Error),
    /// An I/O error.
    IoError(io::Error),
    /// An error encountered while converting a byte slice to a string using UTF-8 encoding.
    FromUtf8(string::FromUtf8Error),
    /// An error indicating that the end of the YAML stream was reached unexpectedly.
    EndOfStream,
    /// An error indicating that more than one YAML document was encountered.
    MoreThanOneDocument,
    /// An error indicating that the recursion limit was exceeded.
    RecursionLimitExceeded(libyml_error::Mark),
    /// An error indicating that the repetition limit was exceeded.
    RepetitionLimitExceeded,
    /// An error indicating that byte-based YAML is unsupported.
    BytesUnsupported,
    /// An error indicating that an unknown anchor was encountered.
    UnknownAnchor(libyml_error::Mark),
    /// An error indicating that serializing a nested enum is not supported.
    SerializeNestedEnum,
    /// An error indicating that a scalar value was encountered in a merge operation.
    ScalarInMerge,
    /// An error indicating that a tagged value was encountered in a merge operation.
    TaggedInMerge,
    /// An error indicating that a scalar value was encountered in a merge operation.
    ScalarInMergeElement,
    /// An error indicating that a sequence value was encountered in a merge operation.
    SequenceInMergeElement,
    /// An error indicating that a mapping value was expected in a merge operation.
    ExpectedMapInMerge,
    /// Failed to parse number
    FailedToParseNumber,
    /// Empty tag
    EmptyTag,
}

pub(crate) fn new(error: ErrorImpl) -> Error {
    Error(Box::new(error))
}

pub(crate) fn shared(error: Arc<ErrorImpl>) -> Error {
    Error(Box::new(ErrorImpl::Message(format!("{}", error), None)))
}

pub(crate) fn fix_mark(
    mut err: Error,
    mark: libyml_error::Mark,
    path: &crate::modules::path::Path<'_>,
) -> Error {
    if let ErrorImpl::Message(_, pos) = err.0.as_mut()
        && pos.is_none()
    {
        *pos = Some(Pos {
            mark,
            path: path.to_string(),
        });
    }
    err
}

impl Error {
    pub(crate) fn new(error: ErrorImpl) -> Self {
        Error(Box::new(error))
    }

    /// Returns the location of the error, if available.
    pub fn location(&self) -> Option<Location> {
        match self.0.as_ref() {
            ErrorImpl::Message(_, Some(pos)) => {
                Some(Location::from_mark(pos.mark))
            }
            ErrorImpl::Libyml(err) => {
                Some(Location::from_mark(err.mark()))
            }
            ErrorImpl::RecursionLimitExceeded(mark) => {
                Some(Location::from_mark(*mark))
            }
            ErrorImpl::UnknownAnchor(mark) => {
                Some(Location::from_mark(*mark))
            }
            _ => None,
        }
    }

    /// Returns a shared reference to the internal error implementation.
    pub fn shared(&self) -> Arc<ErrorImpl> {
        Arc::new(match self.0.as_ref() {
            ErrorImpl::Message(msg, pos) => ErrorImpl::Message(
                msg.clone(),
                pos.as_ref().map(|p| Pos {
                    mark: p.mark,
                    path: p.path.clone(),
                }),
            ),
            ErrorImpl::Libyml(err) => ErrorImpl::Libyml(err.clone()),
            ErrorImpl::IoError(e) => {
                ErrorImpl::Message(e.to_string(), None)
            }
            ErrorImpl::FromUtf8(err) => {
                ErrorImpl::FromUtf8(err.clone())
            }
            ErrorImpl::EndOfStream => ErrorImpl::EndOfStream,
            ErrorImpl::MoreThanOneDocument => {
                ErrorImpl::MoreThanOneDocument
            }
            ErrorImpl::RecursionLimitExceeded(mark) => {
                ErrorImpl::RecursionLimitExceeded(*mark)
            }
            ErrorImpl::RepetitionLimitExceeded => {
                ErrorImpl::RepetitionLimitExceeded
            }
            ErrorImpl::BytesUnsupported => ErrorImpl::BytesUnsupported,
            ErrorImpl::UnknownAnchor(mark) => {
                ErrorImpl::UnknownAnchor(*mark)
            }
            ErrorImpl::SerializeNestedEnum => {
                ErrorImpl::SerializeNestedEnum
            }
            ErrorImpl::ScalarInMerge => ErrorImpl::ScalarInMerge,
            ErrorImpl::TaggedInMerge => ErrorImpl::TaggedInMerge,
            ErrorImpl::ScalarInMergeElement => {
                ErrorImpl::ScalarInMergeElement
            }
            ErrorImpl::SequenceInMergeElement => {
                ErrorImpl::SequenceInMergeElement
            }
            ErrorImpl::ExpectedMapInMerge => {
                ErrorImpl::ExpectedMapInMerge
            }
            ErrorImpl::FailedToParseNumber => {
                ErrorImpl::FailedToParseNumber
            }
            ErrorImpl::EmptyTag => ErrorImpl::EmptyTag,
        })
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.0.source()
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error(Box::new(ErrorImpl::Message(msg.to_string(), None)))
    }
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error(Box::new(ErrorImpl::Message(msg.to_string(), None)))
    }
}

impl From<libyml_error::Error> for Error {
    fn from(err: libyml_error::Error) -> Self {
        if err.problem.contains("found unknown anchor") {
            Error(Box::new(ErrorImpl::UnknownAnchor(err.problem_mark)))
        } else if err.problem.contains("recursion limit exceeded") {
            Error(Box::new(ErrorImpl::RecursionLimitExceeded(err.problem_mark)))
        } else {
            Error(Box::new(ErrorImpl::Libyml(err)))
        }
    }
}

impl StdError for ErrorImpl {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            ErrorImpl::IoError(err) => Some(err),
            ErrorImpl::FromUtf8(err) => Some(err),
            _ => None,
        }
    }
}

impl Display for ErrorImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorImpl::Message(msg, None) => Display::fmt(msg, f),
            ErrorImpl::Message(msg, Some(pos)) => {
                write!(f, "{} at {} in {}", msg, pos.mark, pos.path)
            }
            ErrorImpl::Libyml(err) => Display::fmt(err, f),
            ErrorImpl::IoError(err) => Display::fmt(err, f),
            ErrorImpl::FromUtf8(err) => Display::fmt(err, f),
            ErrorImpl::EndOfStream => f.write_str("EOF while parsing a value"),
            ErrorImpl::MoreThanOneDocument => {
                f.write_str("expected a single YAML document but found more than one")
            }
            ErrorImpl::RecursionLimitExceeded(mark) => {
                write!(f, "recursion limit exceeded at {}", mark)
            }
            ErrorImpl::RepetitionLimitExceeded => f.write_str(
                "Repetition Limit Exceeded: The repetition limit was exceeded while parsing the YAML",
            ),
            ErrorImpl::BytesUnsupported => {
                f.write_str("byte-based YAML is not supported")
            }
            ErrorImpl::UnknownAnchor(mark) => {
                write!(f, "unknown anchor at {}", mark)
            }
            ErrorImpl::SerializeNestedEnum => {
                f.write_str("serializing nested enums is not supported")
            }
            ErrorImpl::ScalarInMerge => {
                f.write_str("unexpected scalar in merge")
            }
            ErrorImpl::TaggedInMerge => {
                f.write_str("unexpected tagged value in merge")
            }
            ErrorImpl::ScalarInMergeElement => {
                f.write_str("unexpected scalar in merge element")
            }
            ErrorImpl::SequenceInMergeElement => {
                f.write_str("unexpected sequence in merge element")
            }
            ErrorImpl::ExpectedMapInMerge => {
                f.write_str("expected a mapping in merge element")
            }
            ErrorImpl::FailedToParseNumber => {
                f.write_str("failed to parse YAML number")
            }
            ErrorImpl::EmptyTag => {
                f.write_str("empty tag")
            }
        }
    }
}
