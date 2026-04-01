use std::fmt::{self, Debug, Display};

/// A type alias for a `Result` with an `Error` as the error type.
pub type Result<T> = std::result::Result<T, Error>;

/// Represents an error that occurred during YAML processing.
#[derive(Clone)]
pub struct Error {
    /// A string describing the problem that caused the error.
    pub problem: String,

    /// The offset of the problem that caused the error.
    pub problem_offset: u64,

    /// The mark indicating the position of the problem that caused the error.
    pub problem_mark: Mark,

    /// An optional string providing additional context for the error.
    pub context: Option<String>,

    /// The mark indicating the position of the context related to the error.
    pub context_mark: Mark,
}

impl Error {
    /// Creates a new `Error` with the specified problem description.
    pub fn new(problem: String) -> Self {
        Error {
            problem,
            problem_offset: 0,
            problem_mark: Mark::default(),
            context: None,
            context_mark: Mark::default(),
        }
    }

    /// Returns the mark indicating the position of the problem that caused the error.
    pub fn mark(&self) -> Mark {
        self.problem_mark
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.problem)?;
        if self.problem_mark.line != 0 || self.problem_mark.column != 0
        {
            write!(formatter, " at {}", self.problem_mark)?;
        } else if self.problem_offset != 0 {
            write!(formatter, " at position {}", self.problem_offset)?;
        }
        if let Some(context) = &self.context {
            write!(formatter, ", {}", context)?;
            if (self.context_mark.line != 0
                || self.context_mark.column != 0)
                && (self.context_mark.line != self.problem_mark.line
                    || self.context_mark.column
                        != self.problem_mark.column)
            {
                write!(formatter, " at {}", self.context_mark)?;
            }
        }
        Ok(())
    }
}

impl Debug for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = formatter.debug_struct("Error");
        f.field("problem", &self.problem);
        if self.problem_mark.line != 0 || self.problem_mark.column != 0
        {
            f.field("problem_mark", &self.problem_mark);
        } else if self.problem_offset != 0 {
            f.field("problem_offset", &self.problem_offset);
        }
        if let Some(context) = &self.context {
            f.field("context", context);
            if self.context_mark.line != 0
                || self.context_mark.column != 0
            {
                f.field("context_mark", &self.context_mark);
            }
        }
        f.finish()
    }
}

/// Represents a mark in a YAML document.
/// A mark indicates a specific position or location within the document.
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Mark {
    /// The index of the mark.
    pub index: u64,
    /// The line number of the mark.
    pub line: u64,
    /// The column number of the mark.
    pub column: u64,
}

impl Mark {
    /// Retrieves the index of the mark.
    pub fn index(&self) -> u64 {
        self.index
    }

    /// Retrieves the line number of the mark.
    pub fn line(&self) -> u64 {
        self.line
    }

    /// Retrieves the column number of the mark.
    pub fn column(&self) -> u64 {
        self.column
    }
}

impl Display for Mark {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line != 0 || self.column != 0 {
            write!(
                formatter,
                "line {} column {}",
                self.line + 1,
                self.column + 1,
            )
        } else {
            write!(formatter, "position {}", self.index)
        }
    }
}

impl Debug for Mark {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = formatter.debug_struct("Mark");
        if self.line != 0 || self.column != 0 {
            f.field("line", &(self.line + 1));
            f.field("column", &(self.column + 1));
        } else {
            f.field("index", &self.index);
        }
        f.finish()
    }
}
