use std::fmt;

/// A parse error from the `.mkml` DSL.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError(pub String);

impl ParseError {
    pub(crate) fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mkml parse error: {}", self.0)
    }
}

impl std::error::Error for ParseError {}
