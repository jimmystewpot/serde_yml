use std::{
    fmt::{self, Debug, Display},
    ops::Deref,
};

/// Custom error type for Tag operations.
#[derive(Clone, Copy, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct TagFormatError;

impl Display for TagFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error occurred while formatting tag")
    }
}

impl std::error::Error for TagFormatError {}

/// Represents a tag in a YAML document.
/// A tag specifies the data type or semantic meaning of a value.
#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Hash)]
pub struct Tag(pub(crate) Box<[u8]>);

impl Tag {
    /// The null tag, representing a null value.
    pub const NULL: &'static str = "tag:yaml.org,2002:null";

    /// The bool tag, representing a boolean value.
    pub const BOOL: &'static str = "tag:yaml.org,2002:bool";

    /// The int tag, representing an integer value.
    pub const INT: &'static str = "tag:yaml.org,2002:int";

    /// The float tag, representing a floating-point value.
    pub const FLOAT: &'static str = "tag:yaml.org,2002:float";

    /// Checks if the tag starts with the given prefix.
    pub fn starts_with(
        &self,
        prefix: &str,
    ) -> Result<bool, TagFormatError> {
        if prefix.len() > self.0.len() {
            Err(TagFormatError)
        } else {
            let prefix_bytes = prefix.as_bytes();
            let tag_bytes = &self.0[..prefix_bytes.len()];
            Ok(tag_bytes == prefix_bytes)
        }
    }

    /// Creates a new `Tag` instance from a `&str` input.
    pub fn new(tag_str: &str) -> Tag {
        Tag(Box::from(tag_str.as_bytes()))
    }
}

impl PartialEq<str> for Tag {
    fn eq(&self, other: &str) -> bool {
        self.0.as_ref() == other.as_bytes()
    }
}

impl PartialEq<&str> for Tag {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_ref() == other.as_bytes()
    }
}

impl Deref for Tag {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for Tag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = String::from_utf8_lossy(&self.0);
        Debug::fmt(&s, formatter)
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = String::from_utf8_lossy(&self.0);
        Display::fmt(&s, f)
    }
}

impl From<String> for Tag {
    fn from(s: String) -> Self {
        Tag(Box::from(s.into_bytes()))
    }
}

impl From<&str> for Tag {
    fn from(s: &str) -> Self {
        Tag(Box::from(s.as_bytes()))
    }
}
