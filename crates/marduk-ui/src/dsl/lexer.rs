use marduk_engine::paint::Color;

use crate::dsl::error::ParseError;

// ── Token ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Ident(String),
    Str(String),
    Number(f32),
    Color(Color),
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

// ── Lexer ─────────────────────────────────────────────────────────────────

pub struct Lexer<'s> {
    src: &'s str,
    pos: usize,
}

impl<'s> Lexer<'s> {
    pub fn new(src: &'s str) -> Self {
        Self { src, pos: 0 }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let eof = tok == Token::Eof;
            tokens.push(tok);
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
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, ParseError> {
        self.skip_whitespace_and_comments();

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
            other => Err(ParseError::new(format!("unexpected character {:?}", other))),
        }
    }

    fn lex_string(&mut self) -> Result<Token, ParseError> {
        self.advance(); // consume opening `"`
        let mut s = String::new();
        loop {
            match self.advance() {
                None => return Err(ParseError::new("unterminated string literal")),
                Some('"') => break,
                Some('\\') => {
                    match self.advance() {
                        Some('n')  => s.push('\n'),
                        Some('t')  => s.push('\t'),
                        Some('"')  => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some(c)    => s.push(c),
                        None => return Err(ParseError::new("unterminated escape sequence")),
                    }
                }
                Some(c) => s.push(c),
            }
        }
        Ok(Token::Str(s))
    }

    fn lex_color(&mut self) -> Result<Token, ParseError> {
        self.advance(); // consume `#`
        let start = self.pos;
        let mut count = 0;
        while matches!(self.peek(), Some(c) if c.is_ascii_hexdigit()) {
            self.advance();
            count += 1;
        }
        if count != 8 {
            return Err(ParseError::new(format!(
                "color literal must be #rrggbbaa (8 hex digits), got {} digits",
                count
            )));
        }
        let hex = &self.src[start..self.pos];
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
        let a = u8::from_str_radix(&hex[6..8], 16).unwrap();
        let color = Color::from_straight(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        );
        Ok(Token::Color(color))
    }

    fn lex_number(&mut self) -> Result<Token, ParseError> {
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
            .map_err(|_| ParseError::new(format!("invalid number {:?}", s)))
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
