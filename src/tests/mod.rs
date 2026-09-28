use crate::{
    interpreter::Interpreter, parser::Parser, run, scanner::Scanner, type_checker::TypeChecker,
};

fn rejects_without_output(source: &str, message: &str) {
    let mut bytes = Vec::new();
    let error = run(source, &mut bytes).unwrap_err().to_string();
    assert!(error.contains(message), "{source}: {error}");
    assert!(bytes.is_empty(), "{source}: {bytes:?}");
}

fn output(source: &str) -> String {
    let mut bytes = Vec::new();
    run(source, &mut bytes).unwrap();
    String::from_utf8(bytes).unwrap()
}

mod array_library_tests;
mod array_tests;
mod casting_tests;
mod control_flow_tests;
mod enum_tests;
mod function_tests;
mod smoke_tests;
mod structure_behavior_tests;
mod structure_tests;
mod syntax_tests;
mod type_tests;
mod union_tests;
mod update_tests;
