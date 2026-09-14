use std::{env, error::Error, fs, io, io::Write, iter::Peekable, process::ExitCode, str::Chars};

// Scanner: convierte caracteres en tokens, sin ejecutar el programa.
#[derive(Debug, PartialEq)]
enum TokenKind {
    Print,
    LeftParen,
    RightParen,
    String(String),
    Identifier(String),
    Eof,
}

#[derive(Debug)]
struct Token {
    kind: TokenKind,
    line: usize,
}

struct Scanner<'a> {
    chars: Peekable<Chars<'a>>,
    line: usize,
}

impl<'a> Scanner<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().peekable(),
            line: 1,
        }
    }

    fn scan_tokens(mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        while let Some(character) = self.chars.next() {
            let line = self.line;
            let kind = match character {
                '(' => TokenKind::LeftParen,
                ')' => TokenKind::RightParen,
                '"' => self.string()?,
                '\n' => {
                    self.line += 1;
                    continue;
                }
                ' ' | '\r' | '\t' => continue,
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

    fn string(&mut self) -> Result<TokenKind, String> {
        let start_line = self.line;
        let mut value = String::new();
        for character in self.chars.by_ref() {
            match character {
                '"' => return Ok(TokenKind::String(value)),
                '\n' => self.line += 1,
                _ => {}
            }
            value.push(character);
        }
        Err(format!(
            "Línea {start_line}: cadena sin comillas de cierre."
        ))
    }

    fn identifier(&mut self, first: char) -> TokenKind {
        let mut name = String::from(first);
        while let Some(&c) = self.chars.peek() {
            if !c.is_ascii_alphanumeric() && c != '_' {
                break;
            }
            name.push(c);
            self.chars.next();
        }
        match name.as_str() {
            "print" => TokenKind::Print,
            _ => TokenKind::Identifier(name),
        }
    }
}

// AST: separa las expresiones (valores) de las instrucciones (acciones).
#[derive(Debug)]
enum Expr {
    String(String),
}

#[derive(Debug)]
enum Stmt {
    Print(Expr),
}

// Parser descendente: cada método corresponde a una regla de la gramática.
// program    -> statement* EOF
// statement  -> "print" "(" expression ")"
// expression -> STRING
struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    fn parse(&mut self) -> Result<Vec<Stmt>, String> {
        let mut statements = Vec::new();
        while self.peek().kind != TokenKind::Eof {
            statements.push(self.statement()?);
        }
        Ok(statements)
    }

    fn statement(&mut self) -> Result<Stmt, String> {
        self.consume(TokenKind::Print, "Se esperaba 'print'.")?;
        self.print_statement()
    }

    fn print_statement(&mut self) -> Result<Stmt, String> {
        self.consume(TokenKind::LeftParen, "Se esperaba '(' después de 'print'.")?;
        let expression = self.expression()?;
        self.consume(TokenKind::RightParen, "Se esperaba ')' después del texto.")?;
        Ok(Stmt::Print(expression))
    }

    fn expression(&mut self) -> Result<Expr, String> {
        match &self.peek().kind {
            TokenKind::String(value) => {
                let expression = Expr::String(value.clone());
                self.current += 1;
                Ok(expression)
            }
            _ => Err(self.error("Se esperaba una cadena entre comillas dobles.")),
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

// Intérprete de árbol: ejecuta el AST sin conocer el texto ni los tokens.
struct Interpreter<W: Write> {
    output: W,
}

impl<W: Write> Interpreter<W> {
    fn interpret(&mut self, statements: &[Stmt]) -> io::Result<()> {
        for statement in statements {
            self.execute(statement)?;
        }
        Ok(())
    }

    fn execute(&mut self, statement: &Stmt) -> io::Result<()> {
        match statement {
            Stmt::Print(expression) => writeln!(self.output, "{}", Self::evaluate(expression)),
        }
    }

    fn evaluate(expression: &Expr) -> &str {
        match expression {
            Expr::String(value) => value,
        }
    }
}

fn run(source: &str, output: impl Write) -> Result<(), Box<dyn Error>> {
    let tokens = Scanner::new(source).scan_tokens()?;
    let statements = Parser::new(tokens).parse()?;
    Interpreter { output }.interpret(&statements)?;
    Ok(())
}

fn run_file() -> Result<(), Box<dyn Error>> {
    let path = env::args_os()
        .nth(1)
        .ok_or("Uso: cargo run -- archivo.oki")?;
    let source = fs::read_to_string(path)?;
    run(&source, io::stdout().lock())
}

fn main() -> ExitCode {
    match run_file() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(source: &str) -> String {
        let mut bytes = Vec::new();
        run(source, &mut bytes).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn executes_hello_file() {
        assert_eq!(output(include_str!("../hello.oki")), "Hello world\n");
    }

    #[test]
    fn accepts_whitespace_and_unicode_strings() {
        assert_eq!(
            output(" \r\nprint \t( \"¡Hola, 世界! 🦀\" )\r\n"),
            "¡Hola, 世界! 🦀\n"
        );
    }

    #[test]
    fn executes_print_statements_in_order() {
        assert_eq!(
            output("print(\"uno\")\nprint(\"\")\nprint(\"tres\")"),
            "uno\n\ntres\n"
        );
    }

    #[test]
    fn empty_program_has_no_output() {
        assert_eq!(output(" \r\n\t"), "");
    }

    #[test]
    fn rejects_invalid_programs_before_printing() {
        for invalid in [
            "print(\"sin cerrar)",
            "print(\"texto\"",
            "print \"texto\")",
            "print()",
            "print(123)",
            "print(\"texto\", \"otro\")",
            "print(\"texto\"))",
            "printf(\"texto\")",
            "print(\"texto\") basura",
        ] {
            let source = format!("print(\"no debe imprimirse\")\n{invalid}");
            let mut bytes = Vec::new();
            let error = run(&source, &mut bytes).unwrap_err().to_string();
            assert!(error.contains("Línea 2:"), "{source}: {error}");
            assert!(
                bytes.is_empty(),
                "Se ejecutó un programa inválido: {source}"
            );
        }
    }

    #[test]
    fn reports_output_errors() {
        let mut buffer = [0_u8; 2];
        assert!(run("print(\"Hello world\")", &mut buffer[..]).is_err());
    }
}
