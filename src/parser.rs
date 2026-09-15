use crate::{
    scanner::{Token, TokenKind},
    value::{Type, Value},
};

#[derive(Debug)]
pub struct Name {
    pub text: String,
    pub line: usize,
}

impl Name {
    pub fn error(&self, message: &str) -> String {
        format!("Línea {}: {message}", self.line)
    }
}

// AST: las expresiones producen valores; las instrucciones realizan acciones.
#[derive(Debug)]
pub enum Expr {
    LibraryCall {
        path: Vec<Name>,
        receiver: Option<Box<Expr>>,
        arguments: Vec<Expr>,
    },
    Literal(Value),
    Variable(Name),
    Array {
        elements: Vec<Expr>,
        line: usize,
    },
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
        line: usize,
    },
    Unary {
        operator: UnaryOp,
        operand: Box<Expr>,
        line: usize,
    },
    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
        line: usize,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
}

#[derive(Clone, Copy, Debug)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

// Operador de una asignación compuesta: `x += v` equivale a `x = x + v`.
#[derive(Clone, Copy, Debug)]
pub enum AssignOp {
    Add,
    Subtract,
}

impl AssignOp {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+=",
            Self::Subtract => "-=",
        }
    }

    pub fn binary(self) -> BinaryOp {
        match self {
            Self::Add => BinaryOp::Add,
            Self::Subtract => BinaryOp::Subtract,
        }
    }
}

// Incremento o decremento en una unidad: `x++` equivale a `x = x + 1`.
#[derive(Clone, Copy, Debug)]
pub enum IncrementOp {
    Increment,
    Decrement,
}

impl IncrementOp {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Increment => "++",
            Self::Decrement => "--",
        }
    }

    pub fn binary(self) -> BinaryOp {
        match self {
            Self::Increment => BinaryOp::Add,
            Self::Decrement => BinaryOp::Subtract,
        }
    }
}

impl UnaryOp {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Not => "!",
        }
    }
}

impl BinaryOp {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Remainder => "%",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::And => "&&",
            Self::Or => "||",
        }
    }
}

#[derive(Debug)]
pub enum Stmt {
    Import {
        path: Vec<Name>,
        is_use: bool,
        line: usize,
    },
    Declare {
        declared_type: Type,
        is_constant: bool,
        name: Name,
        initializer: Expr,
    },
    Assign {
        name: Name,
        indices: Vec<(Expr, usize)>,
        value: Expr,
    },
    CompoundAssign {
        name: Name,
        indices: Vec<(Expr, usize)>,
        operator: AssignOp,
        value: Expr,
        line: usize,
    },
    Increment {
        name: Name,
        indices: Vec<(Expr, usize)>,
        operator: IncrementOp,
        line: usize,
    },
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
        line: usize,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
        line: usize,
    },
    For {
        initializer: Box<Stmt>,
        condition: Expr,
        update: Box<Stmt>,
        body: Vec<Stmt>,
        line: usize,
    },
    Foreach {
        declared_type: Type,
        name: Name,
        iterable: Expr,
        body: Vec<Stmt>,
        line: usize,
    },
    Print(Expr),
    Println(Expr),
}

// Parser descendente: cada método corresponde a una regla de la gramática.
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Result<Vec<Stmt>, String> {
        let mut statements = Vec::new();
        while self.peek().kind != TokenKind::Eof {
            statements.push(self.statement()?);
        }
        Ok(statements)
    }

    fn statement(&mut self) -> Result<Stmt, String> {
        // Los bucles y el if terminan en '}' y no llevan ';'. El resto sí.
        match self.peek().kind {
            TokenKind::If => return self.if_statement(),
            TokenKind::While => return self.while_statement(),
            TokenKind::For => return self.for_statement(),
            TokenKind::Foreach => return self.foreach_statement(),
            _ => {}
        }
        let statement = match self.peek().kind {
            TokenKind::Import | TokenKind::Use => self.import_statement()?,
            TokenKind::Const | TokenKind::Type(_) => self.declaration()?,
            TokenKind::Identifier(_) => self.assignment()?,
            TokenKind::Print | TokenKind::Println => {
                let newline = self.peek().kind == TokenKind::Println;
                self.current += 1;
                self.print_statement(newline)?
            }
            _ => {
                return Err(self.error(
                    "Se esperaba una declaración con tipo, una asignación, 'import', 'use', 'if', 'while', 'for', 'foreach', 'print' o 'println'.",
                ));
            }
        };
        self.consume(
            TokenKind::Semicolon,
            "Se esperaba ';' al final de la instrucción.",
        )?;
        Ok(statement)
    }

    fn import_statement(&mut self) -> Result<Stmt, String> {
        let line = self.peek().line;
        let is_use = self.peek().kind == TokenKind::Use;
        self.current += 1;
        Ok(Stmt::Import {
            path: self.path()?,
            is_use,
            line,
        })
    }

    fn path(&mut self) -> Result<Vec<Name>, String> {
        let mut path = vec![self.name()?];
        while self.peek().kind == TokenKind::ColonColon {
            self.current += 1;
            path.push(self.name()?);
        }
        Ok(path)
    }

    fn arguments(&mut self) -> Result<Vec<Expr>, String> {
        self.consume(
            TokenKind::LeftParen,
            "Se esperaba '(' para llamar al método de biblioteca.",
        )?;
        let mut arguments = Vec::new();
        if self.peek().kind != TokenKind::RightParen {
            loop {
                arguments.push(self.expression()?);
                if self.peek().kind != TokenKind::Comma {
                    break;
                }
                self.current += 1;
            }
        }
        self.consume(
            TokenKind::RightParen,
            "Se esperaba ')' después de los argumentos.",
        )?;
        Ok(arguments)
    }

    // Modificación de una variable ya declarada: asignación simple, asignación
    // compuesta o incremento/decremento. Sin el ';' final, para reutilizarla
    // dentro de un 'for'.
    fn assignment(&mut self) -> Result<Stmt, String> {
        let name = self.name()?;
        let mut indices = Vec::new();
        while self.peek().kind == TokenKind::LeftBracket {
            indices.push(self.index()?);
        }
        let line = self.peek().line;
        match self.peek().kind {
            TokenKind::Equal => {
                self.current += 1;
                Ok(Stmt::Assign {
                    name,
                    indices,
                    value: self.expression()?,
                })
            }
            TokenKind::PlusEqual | TokenKind::MinusEqual => {
                let operator = if self.peek().kind == TokenKind::PlusEqual {
                    AssignOp::Add
                } else {
                    AssignOp::Subtract
                };
                self.current += 1;
                Ok(Stmt::CompoundAssign {
                    name,
                    indices,
                    operator,
                    value: self.expression()?,
                    line,
                })
            }
            TokenKind::PlusPlus | TokenKind::MinusMinus => {
                let operator = if self.peek().kind == TokenKind::PlusPlus {
                    IncrementOp::Increment
                } else {
                    IncrementOp::Decrement
                };
                self.current += 1;
                Ok(Stmt::Increment {
                    name,
                    indices,
                    operator,
                    line,
                })
            }
            _ => Err(self.error(
                "Se esperaba '=', '+=', '-=', '++' o '--' después del nombre para modificar una variable ya declarada. Para declararla hay que indicar su tipo.",
            )),
        }
    }

    fn declaration(&mut self) -> Result<Stmt, String> {
        let is_constant = self.peek().kind == TokenKind::Const;
        if is_constant {
            self.current += 1;
        }
        let declared_type = self.array_type()?;
        let name = self.name()?;
        self.consume(
            TokenKind::Equal,
            "Se esperaba '=' y un valor inicial después del nombre.",
        )?;
        Ok(Stmt::Declare {
            declared_type,
            is_constant,
            name,
            initializer: self.expression()?,
        })
    }

    // Un tipo básico seguido de los '[]' que indican niveles de array.
    fn array_type(&mut self) -> Result<Type, String> {
        let TokenKind::Type(basic) = &self.peek().kind else {
            return Err(self.error("Se esperaba un tipo (int, float, bool, char o string)."));
        };
        let mut declared_type = basic.clone();
        self.current += 1;
        while self.peek().kind == TokenKind::LeftBracket {
            self.current += 1;
            self.consume(
                TokenKind::RightBracket,
                "Se esperaba ']' en el tipo del array; no se declara su longitud.",
            )?;
            declared_type = Type::Array(Box::new(declared_type));
        }
        Ok(declared_type)
    }

    fn if_statement(&mut self) -> Result<Stmt, String> {
        let line = self.peek().line;
        self.current += 1;
        self.consume(TokenKind::LeftParen, "Se esperaba '(' después de 'if'.")?;
        let condition = self.expression()?;
        self.consume(
            TokenKind::RightParen,
            "Se esperaba ')' después de la condición.",
        )?;
        let then_branch = self.block()?;
        let else_branch = if self.peek().kind == TokenKind::Else {
            self.current += 1;
            // 'else if' encadena otro if; 'else' va seguido de un bloque.
            if self.peek().kind == TokenKind::If {
                Some(vec![self.if_statement()?])
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };
        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
            line,
        })
    }

    fn while_statement(&mut self) -> Result<Stmt, String> {
        let line = self.peek().line;
        self.current += 1;
        self.consume(TokenKind::LeftParen, "Se esperaba '(' después de 'while'.")?;
        let condition = self.expression()?;
        self.consume(
            TokenKind::RightParen,
            "Se esperaba ')' después de la condición de 'while'.",
        )?;
        let body = self.block()?;
        Ok(Stmt::While {
            condition,
            body,
            line,
        })
    }

    fn for_statement(&mut self) -> Result<Stmt, String> {
        let line = self.peek().line;
        self.current += 1;
        self.consume(TokenKind::LeftParen, "Se esperaba '(' después de 'for'.")?;
        // La inicialización puede declarar la variable del contador o reasignar
        // una ya existente. Condición y actualización son obligatorias.
        let initializer = match self.peek().kind {
            TokenKind::Const | TokenKind::Type(_) => self.declaration()?,
            TokenKind::Identifier(_) => self.assignment()?,
            _ => {
                return Err(
                    self.error("Se esperaba una declaración o una asignación al inicio de 'for'.")
                );
            }
        };
        self.consume(
            TokenKind::Semicolon,
            "Se esperaba ';' después de la inicialización de 'for'.",
        )?;
        let condition = self.expression()?;
        self.consume(
            TokenKind::Semicolon,
            "Se esperaba ';' después de la condición de 'for'.",
        )?;
        let update = self.assignment()?;
        self.consume(
            TokenKind::RightParen,
            "Se esperaba ')' después de la actualización de 'for'.",
        )?;
        let body = self.block()?;
        Ok(Stmt::For {
            initializer: Box::new(initializer),
            condition,
            update: Box::new(update),
            body,
            line,
        })
    }

    fn foreach_statement(&mut self) -> Result<Stmt, String> {
        let line = self.peek().line;
        self.current += 1;
        self.consume(
            TokenKind::LeftParen,
            "Se esperaba '(' después de 'foreach'.",
        )?;
        let declared_type = self.array_type()?;
        let name = self.name()?;
        self.consume(
            TokenKind::In,
            "Se esperaba 'in' después del nombre del bucle.",
        )?;
        let iterable = self.expression()?;
        self.consume(
            TokenKind::RightParen,
            "Se esperaba ')' después del array de 'foreach'.",
        )?;
        let body = self.block()?;
        Ok(Stmt::Foreach {
            declared_type,
            name,
            iterable,
            body,
            line,
        })
    }

    fn block(&mut self) -> Result<Vec<Stmt>, String> {
        self.consume(
            TokenKind::LeftBrace,
            "Se esperaba '{' para abrir el bloque.",
        )?;
        let mut statements = Vec::new();
        while self.peek().kind != TokenKind::RightBrace {
            if self.peek().kind == TokenKind::Eof {
                return Err(self.error("Se esperaba '}' para cerrar el bloque."));
            }
            statements.push(self.statement()?);
        }
        self.current += 1;
        Ok(statements)
    }

    fn name(&mut self) -> Result<Name, String> {
        if let TokenKind::Identifier(text) = &self.peek().kind {
            let name = Name {
                text: text.clone(),
                line: self.peek().line,
            };
            self.current += 1;
            Ok(name)
        } else {
            Err(self.error("Se esperaba un nombre de variable que no sea una palabra reservada."))
        }
    }

    fn print_statement(&mut self, newline: bool) -> Result<Stmt, String> {
        self.consume(
            TokenKind::LeftParen,
            "Se esperaba '(' después de 'print' o 'println'.",
        )?;
        let expression = self.expression()?;
        self.consume(
            TokenKind::RightParen,
            "Se esperaba ')' después de la expresión.",
        )?;
        Ok(if newline {
            Stmt::Println(expression)
        } else {
            Stmt::Print(expression)
        })
    }

    fn expression(&mut self) -> Result<Expr, String> {
        self.or()
    }

    fn or(&mut self) -> Result<Expr, String> {
        self.binary(Self::and, &[(TokenKind::OrOr, BinaryOp::Or)])
    }

    fn and(&mut self) -> Result<Expr, String> {
        self.binary(Self::equality, &[(TokenKind::AndAnd, BinaryOp::And)])
    }

    fn equality(&mut self) -> Result<Expr, String> {
        self.binary(
            Self::comparison,
            &[
                (TokenKind::EqualEqual, BinaryOp::Equal),
                (TokenKind::BangEqual, BinaryOp::NotEqual),
            ],
        )
    }

    fn comparison(&mut self) -> Result<Expr, String> {
        self.binary(
            Self::term,
            &[
                (TokenKind::Less, BinaryOp::Less),
                (TokenKind::LessEqual, BinaryOp::LessEqual),
                (TokenKind::Greater, BinaryOp::Greater),
                (TokenKind::GreaterEqual, BinaryOp::GreaterEqual),
            ],
        )
    }

    fn term(&mut self) -> Result<Expr, String> {
        self.binary(
            Self::factor,
            &[
                (TokenKind::Plus, BinaryOp::Add),
                (TokenKind::Minus, BinaryOp::Subtract),
            ],
        )
    }

    fn factor(&mut self) -> Result<Expr, String> {
        self.binary(
            Self::unary,
            &[
                (TokenKind::Star, BinaryOp::Multiply),
                (TokenKind::Slash, BinaryOp::Divide),
                (TokenKind::Percent, BinaryOp::Remainder),
            ],
        )
    }

    // Cada nivel consume sus operadores de izquierda a derecha y delega los
    // operandos al nivel de mayor precedencia.
    fn binary(
        &mut self,
        operand: fn(&mut Self) -> Result<Expr, String>,
        operators: &[(TokenKind, BinaryOp)],
    ) -> Result<Expr, String> {
        let mut expression = operand(self)?;
        while let Some((_, operator)) = operators.iter().find(|(kind, _)| *kind == self.peek().kind)
        {
            let line = self.peek().line;
            let operator = *operator;
            self.current += 1;
            expression = Expr::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(operand(self)?),
                line,
            };
        }
        Ok(expression)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        let operator = match self.peek().kind {
            TokenKind::Plus => UnaryOp::Plus,
            TokenKind::Minus => UnaryOp::Minus,
            TokenKind::Bang => UnaryOp::Not,
            _ => return self.postfix(),
        };
        let line = self.peek().line;
        self.current += 1;
        // Un signo menos seguido de NUMBER se convierte junto con los dígitos
        // para conservar el literal mínimo de i64, cuya magnitud no cabe sola.
        if matches!(operator, UnaryOp::Minus) && matches!(self.peek().kind, TokenKind::Number(_)) {
            let number = self.number(true, line)?;
            return self.finish_postfix(number);
        }
        Ok(Expr::Unary {
            operator,
            operand: Box::new(self.unary()?),
            line,
        })
    }

    fn index(&mut self) -> Result<(Expr, usize), String> {
        let line = self.peek().line;
        self.current += 1;
        let index = self.expression()?;
        self.consume(
            TokenKind::RightBracket,
            "Se esperaba ']' después del índice.",
        )?;
        Ok((index, line))
    }

    fn postfix(&mut self) -> Result<Expr, String> {
        let expression = self.primary()?;
        self.finish_postfix(expression)
    }

    fn finish_postfix(&mut self, mut expression: Expr) -> Result<Expr, String> {
        loop {
            match self.peek().kind {
                TokenKind::LeftBracket => {
                    let (index, line) = self.index()?;
                    expression = Expr::Index {
                        array: Box::new(expression),
                        index: Box::new(index),
                        line,
                    };
                }
                TokenKind::Dot => {
                    self.current += 1;
                    let path = vec![self.name()?];
                    expression = Expr::LibraryCall {
                        path,
                        receiver: Some(Box::new(expression)),
                        arguments: self.arguments()?,
                    };
                }
                _ => break,
            }
        }
        Ok(expression)
    }

    fn number(&mut self, negative: bool, line: usize) -> Result<Expr, String> {
        if let TokenKind::Number(digits) = &self.peek().kind {
            let text = if negative {
                format!("-{digits}")
            } else {
                digits.clone()
            };
            let value = if text.contains(['.', 'e', 'E']) {
                let value = text
                    .parse::<f64>()
                    .map_err(|_| format!("Línea {line}: literal float inválido."))?;
                if !value.is_finite() {
                    return Err(format!(
                        "Línea {line}: literal fuera del rango de float finito."
                    ));
                }
                Value::Float(value)
            } else {
                Value::Int(text.parse::<i64>().map_err(|_| {
                    format!("Línea {line}: literal fuera del rango de int (64 bits con signo).")
                })?)
            };
            self.current += 1;
            return Ok(Expr::Literal(value));
        }
        Err(self.error("Se esperaba un literal numérico."))
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match &self.peek().kind {
            TokenKind::LeftBracket => {
                let line = self.peek().line;
                self.current += 1;
                let mut elements = Vec::new();
                if self.peek().kind != TokenKind::RightBracket {
                    loop {
                        elements.push(self.expression()?);
                        if self.peek().kind != TokenKind::Comma {
                            break;
                        }
                        self.current += 1;
                    }
                }
                self.consume(TokenKind::RightBracket, "Se esperaba ']' después de los elementos del array.")?;
                Ok(Expr::Array { elements, line })
            }
            TokenKind::Number(_) => self.number(false, self.peek().line),
            TokenKind::LeftParen => {
                self.current += 1;
                let expression = self.expression()?;
                self.consume(TokenKind::RightParen, "Se esperaba ')' después de la expresión agrupada.")?;
                Ok(expression)
            }
            TokenKind::Literal(value) => {
                let expression = Expr::Literal(value.clone());
                self.current += 1;
                Ok(expression)
            }
            TokenKind::Identifier(_) => {
                let mut path = self.path()?;
                if path.len() == 1 && self.peek().kind != TokenKind::LeftParen {
                    Ok(Expr::Variable(path.remove(0)))
                } else {
                    Ok(Expr::LibraryCall { path, receiver: None, arguments: self.arguments()? })
                }
            }
            _ => Err(self
                .error("Se esperaba un literal (int, float, bool, char, string o array), una variable o una expresión entre paréntesis.")),
        }
    }

    fn consume(&mut self, expected: TokenKind, message: &str) -> Result<(), String> {
        if self.peek().kind != expected {
            return Err(self.error(message));
        }
        self.current += 1;
        Ok(())
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn error(&self, message: &str) -> String {
        format!("Línea {}: {message}", self.peek().line)
    }
}
