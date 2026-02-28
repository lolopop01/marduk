use crate::ast::{DslDocument, Import, Node, Prop, Value};
use crate::error::ParseError;
use crate::lexer::{Lexer, Token};

// ── Parser ────────────────────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    /// Look at the token `offset` positions ahead of current without consuming.
    fn peek_ahead(&self, offset: usize) -> &Token {
        self.tokens.get(self.pos + offset).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.advance() {
            Token::Ident(s) => Ok(s),
            tok => Err(ParseError::new(format!("expected identifier, got {:?}", tok))),
        }
    }

    fn expect_str(&mut self) -> Result<String, ParseError> {
        match self.advance() {
            Token::Str(s) => Ok(s),
            tok => Err(ParseError::new(format!("expected string, got {:?}", tok))),
        }
    }

    fn expect_token(&mut self, expected: &Token) -> Result<(), ParseError> {
        let got = self.advance();
        if &got == expected {
            Ok(())
        } else {
            Err(ParseError::new(format!("expected {:?}, got {:?}", expected, got)))
        }
    }

    // ── Document ──────────────────────────────────────────────────────────

    pub fn parse_document(&mut self) -> Result<DslDocument, ParseError> {
        let mut imports = Vec::new();

        // Consume all leading `import` declarations.
        while self.peek() == &Token::Import {
            imports.push(self.parse_import()?);
        }

        let root = self.parse_node()?;

        Ok(DslDocument { imports, root })
    }

    // ── Import ────────────────────────────────────────────────────────────

    fn parse_import(&mut self) -> Result<Import, ParseError> {
        self.advance(); // consume `import`
        let path = self.expect_str()?;
        self.expect_token(&Token::As)?;
        let alias = self.expect_ident()?;
        Ok(Import { path, alias })
    }

    // ── Node ──────────────────────────────────────────────────────────────

    fn parse_node(&mut self) -> Result<Node, ParseError> {
        let widget = self.expect_ident()?;

        // Optional inline string content: `Text "Hello"` or `Button "OK"`
        let content = if let Token::Str(_) = self.peek() {
            if let Token::Str(s) = self.advance() { Some(s) } else { None }
        } else {
            None
        };

        // Optional block `{ ... }` with properties and/or children mixed freely.
        let (props, children) = if self.peek() == &Token::LBrace {
            self.parse_block()?
        } else {
            (Vec::new(), Vec::new())
        };

        Ok(Node { widget, content, props, children })
    }

    // ── Block ─────────────────────────────────────────────────────────────

    /// Parse `{ item* }` where each item is either a `key: value` property
    /// or a child widget node.
    ///
    /// Disambiguation: when we see `Ident`, we look one token ahead:
    /// - `Ident ":"` → property
    /// - `Ident <anything else>` → child widget node
    fn parse_block(&mut self) -> Result<(Vec<Prop>, Vec<Node>), ParseError> {
        self.advance(); // consume `{`
        let mut props = Vec::new();
        let mut children = Vec::new();

        loop {
            match self.peek() {
                Token::RBrace => { self.advance(); break; }
                Token::Eof    => return Err(ParseError::new("unclosed '{' block")),
                Token::Ident(_) => {
                    if self.peek_ahead(1) == &Token::Colon {
                        props.push(self.parse_prop()?);
                    } else {
                        children.push(self.parse_node()?);
                    }
                }
                tok => {
                    return Err(ParseError::new(format!(
                        "unexpected {:?} inside block — expected a property (key: value) or a widget name",
                        tok
                    )));
                }
            }
        }

        Ok((props, children))
    }

    // ── Prop ──────────────────────────────────────────────────────────────

    fn parse_prop(&mut self) -> Result<Prop, ParseError> {
        let key = self.expect_ident()?;
        self.advance(); // consume `:`
        let value = self.parse_value()?;
        Ok(Prop { key, value })
    }

    // ── Value ─────────────────────────────────────────────────────────────

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        match self.advance() {
            Token::Str(s)    => Ok(Value::Str(s)),
            Token::Number(n) => Ok(Value::Number(n)),
            Token::Color(c)  => Ok(Value::Color(c)),
            Token::Ident(s)  => Ok(Value::Ident(s)),
            tok => Err(ParseError::new(format!("expected a value, got {:?}", tok))),
        }
    }
}

// ── Public parse entry point ──────────────────────────────────────────────

/// Parse a `.mkml` source string into a [`DslDocument`].
pub fn parse_str(src: &str) -> Result<DslDocument, ParseError> {
    let tokens = Lexer::new(src).tokenize()?;
    Parser::new(tokens).parse_document()
}
