use std::{env, error::Error, fs, io, io::Write, process::ExitCode};

mod interpreter;
mod parser;
mod scanner;
mod stdlib;
mod type_checker;
mod value;

use interpreter::Interpreter;
use parser::Parser;
use scanner::Scanner;
use type_checker::TypeChecker;

fn run(source: &str, output: impl Write) -> Result<(), Box<dyn Error>> {
    let tokens = Scanner::new(source).scan_tokens()?;
    let statements = Parser::new(tokens).parse()?;
    TypeChecker::default().check(&statements)?;
    Interpreter::new(output).interpret(&statements)?;
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
#[path = "tests/mod.rs"]
mod tests;
