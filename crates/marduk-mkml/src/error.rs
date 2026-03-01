use std::fmt;

/// A parse error from the `.mkml` DSL.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    /// 1-based source line number where the error occurred.
    pub line: usize,
    /// 1-based source column number where the error occurred.
    pub col: usize,
}

impl ParseError {
    pub(crate) fn new(msg: impl Into<String>, line: usize, col: usize) -> Self {
        Self { message: msg.into(), line, col }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mkml parse error at {}:{}: {}", self.line, self.col, self.message)
    }
}

impl std::error::Error for ParseError {}
