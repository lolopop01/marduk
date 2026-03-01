use crate::error::ParseError;

// ── Token ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Ident(String),
    Str(String),
    Number(f32),
    /// Color literal: `[r, g, b, a]` straight-alpha bytes as parsed from `#rrggbbaa`.
    Color([u8; 4]),
    // Punctuation
    Colon,
    LBrace,
    RBrace,
    // Keywords
    Import,
    As,
    // Sentinel
    Eof,
}

// ── TokenWithPos ───────────────────────────────────────────────────────────

/// A token annotated with its source position.
#[derive(Debug, Clone)]
pub struct TokenWithPos {
    pub token: Token,
    /// 1-based line number of the first character of this token.
    pub line: usize,
    /// 1-based column number of the first character of this token.
    pub col: usize,
}

// ── Lexer ─────────────────────────────────────────────────────────────────

pub struct Lexer<'s> {
    src: &'s str,
    pos: usize,
    /// 1-based current line number.
    line: usize,
    /// 1-based current column number.
    col: usize,
}

impl<'s> Lexer<'s> {
    pub fn new(src: &'s str) -> Self {
        Self { src, pos: 0, line: 1, col: 1 }
    }

    /// Returns the current `(line, col)` position (1-based).
    pub fn current_pos(&self) -> (usize, usize) {
        (self.line, self.col)
    }

    pub fn tokenize(mut self) -> Result<Vec<TokenWithPos>, ParseError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            let (line, col) = self.current_pos();
            let tok = self.next_token()?;
            let eof = tok == Token::Eof;
            tokens.push(TokenWithPos { token: tok, line, col });
            if eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.src[self.pos..].chars().next()?;
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(c) if c.is_whitespace()) {
                self.advance();
            }
            // skip `//` line comments
            if self.src[self.pos..].starts_with("//") {
                while !matches!(self.peek(), None | Some('\n')) {
                    self.advance();
                }
            // skip `/* */` block comments
            } else if self.src[self.pos..].starts_with("/*") {
                self.advance(); self.advance(); // consume `/*`
                loop {
                    if self.src[self.pos..].starts_with("*/") {
                        self.advance(); self.advance(); // consume `*/`
                        break;
                    }
                    if self.advance().is_none() {
                        break; // unterminated — EOF will surface on next token
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Called after `skip_whitespace_and_comments()`. Does NOT skip whitespace itself.
    fn next_token(&mut self) -> Result<Token, ParseError> {
        let ch = match self.peek() {
            None => return Ok(Token::Eof),
            Some(c) => c,
        };

        match ch {
            ':' => { self.advance(); Ok(Token::Colon) }
            '{' => { self.advance(); Ok(Token::LBrace) }
            '}' => { self.advance(); Ok(Token::RBrace) }
            '"' => self.lex_string(),
            '#' => self.lex_color(),
            c if c.is_ascii_digit() || c == '-' => self.lex_number(),
            c if c.is_alphabetic() || c == '_' => self.lex_ident_or_keyword(),
            other => {
                let (line, col) = self.current_pos();
                Err(ParseError::new(format!("unexpected character {:?}", other), line, col))
            }
        }
    }

    fn lex_string(&mut self) -> Result<Token, ParseError> {
        self.advance(); // consume opening `"`
        let mut s = String::new();
        loop {
            let (line, col) = self.current_pos();
            match self.advance() {
                None => return Err(ParseError::new("unterminated string literal", line, col)),
                Some('"') => break,
                Some('\\') => {
                    let (el, ec) = self.current_pos();
                    match self.advance() {
                        Some('n')  => s.push('\n'),
                        Some('t')  => s.push('\t'),
                        Some('"')  => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some(c)    => s.push(c),
                        None => return Err(ParseError::new("unterminated escape sequence", el, ec)),
                    }
                }
                Some(c) => s.push(c),
            }
        }
        Ok(Token::Str(s))
    }

    fn lex_color(&mut self) -> Result<Token, ParseError> {
        let (line, col) = self.current_pos();
        self.advance(); // consume `#`
        let start = self.pos;
        let mut count = 0;
        while matches!(self.peek(), Some(c) if c.is_ascii_hexdigit()) {
            self.advance();
            count += 1;
        }
        if count != 6 && count != 8 {
            return Err(ParseError::new(
                format!("color literal must be #rrggbb or #rrggbbaa, got {} digits", count),
                line, col,
            ));
        }
        let hex = &self.src[start..self.pos];
        // All characters were validated as ascii_hexdigit above, and 2 hex
        // digits fit in u8 (max 0xFF = 255), so these conversions never fail.
        let r = u8::from_str_radix(&hex[0..2], 16).expect("validated hex digits");
        let g = u8::from_str_radix(&hex[2..4], 16).expect("validated hex digits");
        let b = u8::from_str_radix(&hex[4..6], 16).expect("validated hex digits");
        let a = if count == 8 {
            u8::from_str_radix(&hex[6..8], 16).expect("validated hex digits")
        } else {
            255
        };
        Ok(Token::Color([r, g, b, a]))
    }

    fn lex_number(&mut self) -> Result<Token, ParseError> {
        let (line, col) = self.current_pos();
        let start = self.pos;
        if self.peek() == Some('-') {
            self.advance();
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.advance();
        }
        if self.peek() == Some('.') {
            self.advance();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.advance();
            }
        }
        let s = &self.src[start..self.pos];
        s.parse::<f32>()
            .map(Token::Number)
            .map_err(|_| ParseError::new(format!("invalid number {:?}", s), line, col))
    }

    fn lex_ident_or_keyword(&mut self) -> Result<Token, ParseError> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
            self.advance();
        }
        let word = &self.src[start..self.pos];
        Ok(match word {
            "import" => Token::Import,
            "as"     => Token::As,
            _        => Token::Ident(word.to_string()),
        })
    }
}
