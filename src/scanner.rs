use std::{iter::Peekable, str::Chars};

use crate::value::{Type, Value};

#[derive(Debug, PartialEq)]
pub enum TokenKind {
    Print,
    Println,
    Const,
    If,
    Else,
    While,
    For,
    Foreach,
    In,
    Import,
    Use,
    ColonColon,
    Dot,
    Type(Type),
    Semicolon,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Equal,
    EqualEqual,
    Bang,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    AndAnd,
    OrOr,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusPlus,
    PlusEqual,
    MinusMinus,
    MinusEqual,
    Literal(Value),
    // Conservamos los dígitos para que el parser pueda aceptar el mínimo de i64
    // junto con su signo: su magnitud positiva no cabe en un i64.
    Number(String),
    Identifier(String),
    Eof,
}

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
}

pub struct Scanner<'a> {
    chars: Peekable<Chars<'a>>,
    line: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().peekable(),
            line: 1,
        }
    }

    pub fn scan_tokens(mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        while let Some(character) = self.chars.next() {
            let line = self.line;
            let kind = match character {
                '(' => TokenKind::LeftParen,
                ')' => TokenKind::RightParen,
                '{' => TokenKind::LeftBrace,
                '}' => TokenKind::RightBrace,
                '[' => TokenKind::LeftBracket,
                ']' => TokenKind::RightBracket,
                ',' => TokenKind::Comma,
                '.' => TokenKind::Dot,
                ':' => {
                    if self.chars.next_if_eq(&':').is_none() {
                        return Err(format!(
                            "Línea {line}: se esperaba '::' en la ruta de biblioteca."
                        ));
                    }
                    TokenKind::ColonColon
                }
                ';' => TokenKind::Semicolon,
                '=' => self.paired('=', TokenKind::EqualEqual, TokenKind::Equal),
                '!' => self.paired('=', TokenKind::BangEqual, TokenKind::Bang),
                '<' => self.paired('=', TokenKind::LessEqual, TokenKind::Less),
                '>' => self.paired('=', TokenKind::GreaterEqual, TokenKind::Greater),
                '&' | '|' => {
                    if self.chars.next_if_eq(&character).is_none() {
                        return Err(format!(
                            "Línea {line}: se esperaba '{character}{character}'; '{character}' aislado no es un operador."
                        ));
                    }
                    if character == '&' {
                        TokenKind::AndAnd
                    } else {
                        TokenKind::OrOr
                    }
                }
                '+' => self.compound_or_single(
                    '+',
                    TokenKind::PlusPlus,
                    TokenKind::PlusEqual,
                    TokenKind::Plus,
                ),
                '-' => self.compound_or_single(
                    '-',
                    TokenKind::MinusMinus,
                    TokenKind::MinusEqual,
                    TokenKind::Minus,
                ),
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '%' => TokenKind::Percent,
                '"' => TokenKind::Literal(Value::String(self.quoted('"')?)),
                '\'' => {
                    let text = self.quoted('\'')?;
                    let mut chars = text.chars();
                    match (chars.next(), chars.next()) {
                        (Some(value), None) => TokenKind::Literal(Value::Char(value)),
                        _ => {
                            return Err(format!(
                                "Línea {line}: un char debe contener exactamente un carácter Unicode."
                            ));
                        }
                    }
                }
                '\n' => {
                    self.line += 1;
                    continue;
                }
                ' ' | '\r' | '\t' => continue,
                c if c.is_ascii_digit() => self.number(c)?,
                c if c.is_ascii_alphabetic() || c == '_' => self.identifier(c),
                c => return Err(format!("Línea {line}: carácter inesperado '{c}'.")),
            };
            tokens.push(Token { kind, line });
        }
        tokens.push(Token {
            kind: TokenKind::Eof,
            line: self.line,
        });
        Ok(tokens)
    }

    fn paired(&mut self, next: char, paired: TokenKind, single: TokenKind) -> TokenKind {
        if self.chars.next_if_eq(&next).is_some() {
            paired
        } else {
            single
        }
    }

    // Distingue el operador doble ('++' o '--'), la variante con '=' ('+=' o '-=')
    // y el signo aislado ('+' o '-'). Se mira un solo carácter por delante.
    fn compound_or_single(
        &mut self,
        doubled: char,
        doubled_kind: TokenKind,
        equal_kind: TokenKind,
        single: TokenKind,
    ) -> TokenKind {
        match self.chars.peek() {
            Some(&c) if c == doubled => {
                self.chars.next();
                doubled_kind
            }
            Some(&'=') => {
                self.chars.next();
                equal_kind
            }
            _ => single,
        }
    }

    fn quoted(&mut self, delimiter: char) -> Result<String, String> {
        let start_line = self.line;
        let mut value = String::new();
        for character in self.chars.by_ref() {
            if character == delimiter {
                return Ok(value);
            }
            if character == '\n' {
                self.line += 1;
            }
            value.push(character);
        }
        let kind = if delimiter == '"' { "cadena" } else { "char" };
        Err(format!(
            "Línea {start_line}: {kind} sin comillas de cierre."
        ))
    }

    fn digits(&mut self, text: &mut String) {
        while let Some(c) = self.chars.next_if(|c| c.is_ascii_digit()) {
            text.push(c);
        }
    }

    fn number(&mut self, first: char) -> Result<TokenKind, String> {
        let mut text = String::from(first);
        self.digits(&mut text);
        if self.chars.next_if_eq(&'.').is_some() {
            text.push('.');
            if !self.chars.peek().is_some_and(char::is_ascii_digit) {
                return Err(format!(
                    "Línea {}: se esperaba un dígito después del punto decimal.",
                    self.line
                ));
            }
            self.digits(&mut text);
        }
        if let Some(c) = self.chars.next_if(|c| matches!(c, 'e' | 'E')) {
            text.push(c);
            if let Some(sign) = self.chars.next_if(|c| matches!(c, '+' | '-')) {
                text.push(sign);
            }
            if !self.chars.peek().is_some_and(char::is_ascii_digit) {
                return Err(format!(
                    "Línea {}: se esperaban dígitos en el exponente.",
                    self.line
                ));
            }
            self.digits(&mut text);
        }
        Ok(TokenKind::Number(text))
    }

    fn identifier(&mut self, first: char) -> TokenKind {
        let mut name = String::from(first);
        while let Some(c) = self
            .chars
            .next_if(|c| c.is_ascii_alphanumeric() || *c == '_')
        {
            name.push(c);
        }
        match name.as_str() {
            "print" => TokenKind::Print,
            "println" => TokenKind::Println,
            "const" => TokenKind::Const,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "foreach" => TokenKind::Foreach,
            "in" => TokenKind::In,
            "import" => TokenKind::Import,
            "use" => TokenKind::Use,
            "int" => TokenKind::Type(Type::Int),
            "float" => TokenKind::Type(Type::Float),
            "bool" => TokenKind::Type(Type::Bool),
            "char" => TokenKind::Type(Type::Char),
            "string" => TokenKind::Type(Type::String),
            "true" => TokenKind::Literal(Value::Bool(true)),
            "false" => TokenKind::Literal(Value::Bool(false)),
            _ => TokenKind::Identifier(name),
        }
    }
}
