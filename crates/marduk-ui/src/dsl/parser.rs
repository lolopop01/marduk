use crate::dsl::ast::{Attr, DslDocument, Import, Node, Value};
use crate::dsl::error::ParseError;
use crate::dsl::lexer::{Lexer, Token};

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

    fn advance(&mut self) -> &Token {
        let tok = self.tokens.get(self.pos).unwrap_or(&Token::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.advance().clone() {
            Token::Ident(s) => Ok(s),
            tok => Err(ParseError::new(format!("expected identifier, got {:?}", tok))),
        }
    }

    fn expect(&mut self, expected: &Token) -> Result<(), ParseError> {
        let got = self.advance().clone();
        if &got == expected {
            Ok(())
        } else {
            Err(ParseError::new(format!("expected {:?}, got {:?}", expected, got)))
        }
    }

    // ── Document ──────────────────────────────────────────────────────────

    pub fn parse_document(&mut self) -> Result<DslDocument, ParseError> {
        let mut imports = Vec::new();

        // Consume all leading `use` declarations.
        while self.peek() == &Token::Use {
            imports.push(self.parse_import()?);
        }

        let root = self.parse_node()?;

        // Optional trailing semicolon or Eof.
        if self.peek() == &Token::Semicolon {
            self.advance();
        }

        Ok(DslDocument { imports, root })
    }

    // ── Import ────────────────────────────────────────────────────────────

    fn parse_import(&mut self) -> Result<Import, ParseError> {
        self.advance(); // consume `use`
        let path = match self.advance().clone() {
            Token::Str(s) => s,
            tok => return Err(ParseError::new(format!("expected string path after `use`, got {:?}", tok))),
        };
        self.expect(&Token::As)?;
        let alias = self.expect_ident()?;
        self.expect(&Token::Semicolon)?;
        Ok(Import { path, alias })
    }

    // ── Node ──────────────────────────────────────────────────────────────

    fn parse_node(&mut self) -> Result<Node, ParseError> {
        // widget name
        let widget = match self.peek().clone() {
            Token::Ident(s) => { self.advance(); s }
            tok => return Err(ParseError::new(format!("expected widget name, got {:?}", tok))),
        };

        // optional inline string content
        let content = if let Token::Str(_) = self.peek() {
            if let Token::Str(s) = self.advance().clone() { Some(s) } else { None }
        } else {
            None
        };

        // optional attribute list [...]
        let attrs = if self.peek() == &Token::LBracket {
            self.parse_attrs()?
        } else {
            Vec::new()
        };

        // optional children block {...}
        let children = if self.peek() == &Token::LBrace {
            self.parse_children()?
        } else {
            Vec::new()
        };

        // optional trailing semicolon
        if self.peek() == &Token::Semicolon {
            self.advance();
        }

        Ok(Node { widget, content, attrs, children })
    }

    fn parse_attrs(&mut self) -> Result<Vec<Attr>, ParseError> {
        self.advance(); // consume `[`
        let mut attrs = Vec::new();

        loop {
            if self.peek() == &Token::RBracket {
                self.advance();
                break;
            }
            attrs.push(self.parse_attr()?);
            match self.peek() {
                Token::Comma => { self.advance(); }
                Token::RBracket => {}
                tok => return Err(ParseError::new(format!("expected `,` or `]` in attrs, got {:?}", tok))),
            }
        }

        Ok(attrs)
    }

    fn parse_attr(&mut self) -> Result<Attr, ParseError> {
        let key = self.expect_ident()?;
        self.expect(&Token::Eq)?;
        let value = self.parse_value()?;
        Ok(Attr { key, value })
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        match self.advance().clone() {
            Token::Str(s)    => Ok(Value::Str(s)),
            Token::Number(n) => Ok(Value::Number(n)),
            Token::Color(c)  => Ok(Value::Color(c)),
            Token::Ident(s)  => Ok(Value::Ident(s)),
            tok => Err(ParseError::new(format!("expected value, got {:?}", tok))),
        }
    }

    fn parse_children(&mut self) -> Result<Vec<Node>, ParseError> {
        self.advance(); // consume `{`
        let mut children = Vec::new();

        loop {
            if self.peek() == &Token::RBrace {
                self.advance();
                break;
            }
            if self.peek() == &Token::Eof {
                return Err(ParseError::new("unclosed `{` block"));
            }
            children.push(self.parse_node()?);
        }

        Ok(children)
    }
}

// ── Public parse entry point ──────────────────────────────────────────────

/// Parse a `.mkml` source string into a [`DslDocument`].
pub fn parse_str(src: &str) -> Result<DslDocument, ParseError> {
    let tokens = Lexer::new(src).tokenize()?;
    Parser::new(tokens).parse_document()
}
