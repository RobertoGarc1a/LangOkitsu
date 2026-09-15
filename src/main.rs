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
mod tests {
    use super::*;

    fn output(source: &str) -> String {
        let mut bytes = Vec::new();
        run(source, &mut bytes).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn executes_hello_file() {
        assert_eq!(
            output(include_str!("../examples/hello.oki")),
            "Hello World\n"
        );
    }

    #[test]
    fn executes_array_library_example() {
        assert_eq!(
            output(include_str!("../examples/biblioteca_arrays.oki")),
            "3\n3\n3\n10\n20\n30\n0\n"
        );
    }

    #[test]
    fn array_library_supports_full_short_and_method_calls() {
        assert_eq!(
            output(
                "import std::Array; int[] a = [1, 2]; println(a.len()); println(std::Array::len(a)); use std::Array; println(Array::len(a)); println(a.len()); println(std::Array::len(a));"
            ),
            "2\n2\n2\n2\n2\n"
        );
        // Los nombres de biblioteca y los de variable se distinguen por '::'.
        assert_eq!(
            output(
                "import std::Array; use std::Array; int[] Array = [1]; int std = 2; int len = 3; println(Array::len(Array)); println(Array.len() + std + len);"
            ),
            "1\n6\n"
        );
    }

    #[test]
    fn length_accepts_all_array_types_constants_and_nested_arrays() {
        for (kind, element) in [
            ("int", "1"),
            ("float", "1.0"),
            ("bool", "true"),
            ("char", "'ñ'"),
            ("string", "\"Hola\""),
        ] {
            assert_eq!(
                output(&format!(
                    "import std::Array; const {kind}[] a = [{element}, {element}]; {kind}[] empty = []; println(a.len()); println(empty.len()); println(a);"
                )),
                format!("2\n0\n[{element}, {element}]\n")
            );
        }
        assert_eq!(
            output(
                "import std::Array; const int[][] a = [[], [1, 2, 3]]; println(a.len()); println(a[0].len()); println(a[1].len()); println([[1], [2]][0].len());"
            ),
            "2\n0\n3\n1\n"
        );
    }

    #[test]
    fn length_composes_with_expressions_loops_and_reassignment() {
        assert_eq!(
            output(
                "import std::Array; int[] a = [4, 5]; int n = (a).len() + [1].len(); println(n); for (int i = 0; i < a.len(); i++) { print(a[i]); } println(\"\"); a = []; println(a.len()); a = [6]; println(a[a.len() - 1]); if (a.len() == 1) { println(std::Array::len(a)); }"
            ),
            "3\n45\n0\n6\n1\n"
        );
    }

    #[test]
    fn array_library_requires_prior_import_and_use_without_partial_output() {
        for call in ["a.len()", "std::Array::len(a)", "Array::len(a)"] {
            rejects_without_output(
                &format!("int[] a = [1]; println(\"previo\"); println({call}); import std::Array;"),
                "requiere un 'import std::Array;' anterior",
            );
        }
        rejects_without_output(
            "import std::Array; int[] a = [1]; println(a.len()); println(Array::len(a)); use std::Array;",
            "requiere un 'use std::Array;' anterior",
        );
        rejects_without_output(
            "println(\"previo\"); use std::Array; import std::Array;",
            "requiere un 'import std::Array;' anterior",
        );
        assert_eq!(
            output(
                "import std::Array; import std::Array; use std::Array; use std::Array; println(Array::len([1]));"
            ),
            "1\n"
        );
        // Una ejecución anterior no habilita bibliotecas en otro archivo.
        rejects_without_output(
            "println([1].len());",
            "requiere un 'import std::Array;' anterior",
        );
        assert_eq!(
            output("int[] a = [1]; a[0] = 2; foreach (int n in a) { println(n); }"),
            "2\n"
        );
    }

    #[test]
    fn rejects_unknown_libraries_and_imports_inside_blocks() {
        for directive in [
            "import std::Arrays;",
            "import Array;",
            "import std::String;",
            "use other::Array;",
            "import std::Array::len;",
        ] {
            rejects_without_output(
                &format!("println(\"previo\");\n{directive}"),
                "Línea 2: biblioteca desconocida",
            );
        }
        for statement in [
            "if (false) { import std::Array; }",
            "while (false) { import std::Array; }",
            "for (int i = 0; i < 0; i++) { use std::Array; }",
            "foreach (int n in [1]) { import std::Array; }",
        ] {
            rejects_without_output(
                &format!("println(\"previo\"); {statement}"),
                "solo se permiten en el ámbito global",
            );
        }
    }

    #[test]
    fn rejects_invalid_library_calls_before_execution() {
        for call in ["[1].len(0)", "std::Array::len()", "Array::len([1], [2])"] {
            rejects_without_output(
                &format!(
                    "import std::Array; use std::Array; println(\"previo\"); println({call});"
                ),
                "argumentos entre paréntesis",
            );
        }
        for call in [
            "(1).len()",
            "1.0.len()",
            "true.len()",
            "'a'.len()",
            "\"abc\".len()",
            "std::Array::len(1)",
            "Array::len(false)",
            "[1].len().len()",
        ] {
            rejects_without_output(
                &format!(
                    "import std::Array; use std::Array; println(\"previo\"); println({call});"
                ),
                "solo admite arrays",
            );
        }
        for call in [
            "[1].size()",
            "std::Array::missing([1])",
            "Array::missing([1])",
            "std::String::len([1])",
            "len([1])",
        ] {
            rejects_without_output(
                &format!(
                    "import std::Array; use std::Array; println(\"previo\"); println({call});"
                ),
                "desconocid",
            );
        }
        rejects_without_output(
            "import std::Array; println([].len());",
            "un array vacío necesita un tipo declarado",
        );
        rejects_without_output(
            "import std::Array; println(std::Array::len([]));",
            "un array vacío necesita un tipo declarado",
        );
        rejects_without_output(
            "import std::Array; println(missing.len());",
            "no está declarada",
        );
        rejects_without_output(
            "import std::Array; float n = [1].len();",
            "se esperaba float, se recibió int",
        );
        rejects_without_output(
            "import std::Array; println(true || [1].len() == false);",
            "no admite",
        );
    }

    #[test]
    fn rejects_incomplete_library_syntax() {
        for source in [
            "import std:Array;",
            "import std::;",
            "import;",
            "import std::Array",
            "use;",
            "use std::Array",
            "println([1].len);",
            "println([1].());",
            "println([1].len(;",
            "println(std::Array::len([1],));",
            "println(std::Array::len([1]);",
            "println(std::Array::);",
        ] {
            rejects_without_output(&format!("println(\"previo\"); {source}"), "Línea 1:");
        }
    }

    #[test]
    fn library_errors_preserve_lines_and_runtime_short_circuiting() {
        rejects_without_output("println(\"previo\");\nprintln([1].\nlen());", "Línea 3:");
        rejects_without_output(
            "import std::Array;\nprintln(\"previo\");\nprintln(std::Array::\nlen(false));",
            "Línea 4:",
        );
        assert_eq!(
            output("import std::Array; println(true || [1 / 0].len() == 0);"),
            "true\n"
        );
        let mut bytes = Vec::new();
        let error = run("import std::Array; println(\"previo\");\nprintln([1 / 0].len()); println(\"posterior\");", &mut bytes).unwrap_err().to_string();
        assert!(
            error.contains("Línea 2:") && error.contains("por cero"),
            "{error}"
        );
        assert_eq!(bytes, b"previo\n");
        let mut bytes = Vec::new();
        let error = run(
            "import std::Array; int[][] a = [[1]]; println(\"previo\");\nprintln(a[1].len());",
            &mut bytes,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Línea 2:"), "{error}");
        assert_eq!(bytes, b"previo\n");
    }

    #[test]
    fn executes_arrays_example() {
        assert_eq!(
            output(include_str!("../examples/arrays.oki")),
            "[1, 2, 3]\n10\n[1, 10, 3]\n[99, 10, 3]\n[\"Ana\", \"世界\"]\n[]\n3\ntrue\n"
        );
    }

    #[test]
    fn executes_array_mutation_example() {
        assert_eq!(
            output(include_str!("../examples/modificar_arrays.oki")),
            "[1, 2, 3, 4]\n4\n3\n[1, 2]\n2\n1\n0\n"
        );
    }

    #[test]
    fn push_and_pop_support_both_syntaxes_and_all_element_types() {
        for (kind, value) in [
            ("int", "4"),
            ("float", "4.0"),
            ("bool", "true"),
            ("char", "'ñ'"),
            ("string", "\"世界\""),
        ] {
            for (push, pop) in [
                (format!("a.push({value})"), "a.pop()"),
                (
                    format!("std::Array::push(a, {value})"),
                    "std::Array::pop(a)",
                ),
                (format!("Array::push(a, {value})"), "Array::pop(a)"),
            ] {
                assert_eq!(
                    output(&format!(
                        "import std::Array; use std::Array; {kind}[] a = []; {push}; println(a.len()); {kind} last = {pop}; println(last == {value}); println(a.len()); {push}; {pop}; println(a);"
                    )),
                    "1\ntrue\n0\n[]\n"
                );
            }
        }
    }

    #[test]
    fn array_mutation_supports_nested_targets_empty_elements_and_independent_copies() {
        assert_eq!(
            output(
                "import std::Array; use std::Array; int[][] a = []; a.push([]); Array::push(a[0], 1); int[] original = [2]; a.push(original); original.push(3); int[][] copy = a; copy[0].push(9); int[] last = std::Array::pop(a); last.push(4); println(a); println(copy); println(original); println(last); (a[0]).pop(); println(a);"
            ),
            "[[1]]\n[[1, 9], [2]]\n[2, 3]\n[2, 4]\n[[]]\n"
        );
    }

    #[test]
    fn array_mutation_builds_lists_in_loops_and_respects_scopes_and_foreach_snapshot() {
        assert_eq!(
            output(
                "import std::Array; int[] a = []; for (int i = 0; i < 3; i++) { a.push(i); } if (true) { int[] a = []; a.push(9); println(a); } foreach (int n in a) { a.push(n + 3); } println(a); while (a.len() > 0) { print(a.pop()); } println(a);"
            ),
            "[9]\n[0, 1, 2, 3, 4, 5]\n543210[]\n"
        );
    }

    #[test]
    fn rejects_invalid_array_mutations_before_output() {
        for (call, error) in [
            ("a.push()", "esperaba 1 argumentos"),
            ("a.pop(0)", "esperaba 0 argumentos"),
            ("std::Array::push(a)", "esperaba 2 argumentos"),
            ("Array::pop()", "esperaba 1 argumentos"),
            ("a.push(1.0)", "se esperaba int, se recibió float"),
            ("a.push([])", "un array vacío necesita"),
            ("a.push(missing)", "no está declarada"),
            ("[1].push(2)", "variable array modificable"),
            ("std::Array::pop([1])", "variable array modificable"),
            ("Array::push(1, 2)", "solo admite arrays"),
            ("push(a, 4)", "desconocida"),
            ("pop(a)", "desconocida"),
            ("println(a.push(2))", "no devuelve un valor"),
            ("float n = a.pop()", "se esperaba float, se recibió int"),
        ] {
            rejects_without_output(
                &format!(
                    "import std::Array; use std::Array; int[] a = [1]; println(\"previo\"); {call};"
                ),
                error,
            );
        }
        for call in [
            "a.push(2)",
            "a.pop()",
            "std::Array::push(a, 2)",
            "Array::pop(a)",
        ] {
            rejects_without_output(
                &format!(
                    "import std::Array; use std::Array; const int[] a = [1]; println(\"previo\"); {call};"
                ),
                "constante 'a'",
            );
            rejects_without_output(
                &format!("int[] a = [1]; println(\"previo\"); {call}; import std::Array;"),
                "requiere un 'import std::Array;' anterior",
            );
        }
        rejects_without_output(
            "import std::Array; const int[][] a = [[1]]; a[0].pop();",
            "constante 'a'",
        );
        rejects_without_output(
            "import std::Array; int[] a = []; Array::push(a, 2); use std::Array;",
            "requiere un 'use std::Array;' anterior",
        );
        rejects_without_output(
            "import std::Array; int[] a = []; a.push(1)",
            "Se esperaba ';'",
        );
        rejects_without_output(
            "import std::Array; int[] a = []; a.push(1,);",
            "Se esperaba un literal",
        );
    }

    #[test]
    fn mutations_evaluate_once_in_order_and_short_circuit() {
        assert_eq!(
            output(
                "import std::Array; int[][] a = [[10], [20]]; int[] indices = [0, 1]; a[indices.pop()].push(indices.pop()); println(a); println(indices); int[] b = [1, 2, 3]; b.push(b.pop()); println(b); println(b.pop() - b.pop()); println(true || b.pop() == 1); println(false && b.pop() == 1); println(b);"
            ),
            "[[10], [20, 0]]\n[]\n[1, 2, 3]\n1\ntrue\nfalse\n[1]\n"
        );
        assert_eq!(
            output("import std::Array; int[] a = [1, 0]; a[a.pop()] = 7; println(a);"),
            "[7]\n"
        );
    }

    #[test]
    fn mutation_runtime_errors_preserve_output_and_do_not_panic() {
        for (source, error) in [
            (
                "int[] a = []; println(\"previo\");\na.\npop();",
                "Línea 3: No se puede hacer 'pop' de un array vacío",
            ),
            (
                "int[] a = []; println(\"previo\");\nstd::Array::pop(a);",
                "Línea 2: No se puede hacer 'pop' de un array vacío",
            ),
            (
                "int[][] a = [[]]; println(\"previo\");\na[2].push(1 / 0);",
                "Línea 2: índice 2 fuera de rango",
            ),
            (
                "int[] a = []; println(\"previo\"); a.push(1 / 0);",
                "división o resto por cero",
            ),
            (
                "int[] a = [1]; println(\"previo\"); a[0] = a.pop();",
                "destino quedó fuera de rango",
            ),
            (
                "int[] a = [1]; println(\"previo\"); a[0] += a.pop();",
                "destino quedó fuera de rango",
            ),
            (
                "int[][] a = [[1]]; println(\"previo\"); a[0].push(a.pop()[0]);",
                "destino quedó fuera de rango",
            ),
            (
                "int[][] a = [[1]]; println(\"previo\"); a[0][a.pop()[0]]++;",
                "destino quedó fuera de rango",
            ),
        ] {
            let mut bytes = Vec::new();
            let actual = run(
                &format!("import std::Array; {source} println(\"posterior\");"),
                &mut bytes,
            )
            .unwrap_err()
            .to_string();
            assert!(actual.contains(error), "{source}: {actual}");
            assert_eq!(bytes, b"previo\n");
        }
    }

    #[test]
    fn arrays_support_each_basic_type_and_empty_assignments() {
        for (kind, first, second, printed) in [
            ("int", "1", "2", "[2, 1]"),
            ("float", "1.0", "2.5", "[2.5, 1.0]"),
            ("bool", "true", "false", "[false, true]"),
            ("char", "'ñ'", "'🦀'", "['🦀', 'ñ']"),
            ("string", "\"Ana\"", "\"世界\"", "[\"世界\", \"Ana\"]"),
        ] {
            assert_eq!(
                output(&format!(
                    "{kind}[] a = []; a = [{first}, {second}]; a[0] = a[1]; a[1] = {first}; print(a); a = []; println(a);"
                )),
                format!("{printed}[]\n")
            );
        }
    }

    #[test]
    fn array_indexing_has_precedence_and_accepts_expressions() {
        assert_eq!(
            output(
                "int[] a = [2, 3 * 4, -9223372036854775808]; int i = 0; a[i + 1] = a[i] + 3; println(-a[0] * a[1]); println(([1, 2])[1] + 4); println(a[2]); bool[] b = [false]; println(!b[0]);"
            ),
            "-10\n6\n-9223372036854775808\ntrue\n"
        );
    }

    #[test]
    fn nested_arrays_and_constants_have_independent_copies() {
        assert_eq!(
            output(
                "const int[][] source = [[], [1, 2]]; int[][] copy = source; copy[1][0] = 9; copy[0] = [7]; int[] row = copy[1]; row[1] = 8; println(source); println(copy); println(row); copy = [[]]; copy[0] = []; println(copy); println([[1], [2, 3]][1][0]);"
            ),
            "[[], [1, 2]]\n[[7], [9, 2]]\n[9, 8]\n[[]]\n2\n"
        );
        for assignment in ["a = [[1]];", "a[0] = [1];", "a[0][0] = 1;"] {
            rejects_without_output(
                &format!("const int[][] a = [[1]]; println(a);\n{assignment}"),
                "Línea 2: No se puede reasignar la constante 'a'.",
            );
        }
    }

    #[test]
    fn array_equality_compares_contents_order_and_length() {
        assert_eq!(
            output(
                "int[] empty = []; int[] other = []; println(empty == other); println(empty != [1]); println([1, 2] == [1, 2]); println([1, 2] == [2, 1]); println([1] == [1, 2]); println([[1], [2]] != [[1], [3]]); println([0.0] == [-0.0]);"
            ),
            "true\ntrue\ntrue\nfalse\nfalse\ntrue\ntrue\n"
        );
    }

    #[test]
    fn rejects_array_type_mismatches_before_any_output() {
        let types = [
            ("int", "1"),
            ("float", "1.0"),
            ("bool", "true"),
            ("char", "'a'"),
            ("string", "\"a\""),
        ];
        for (kind, value) in types {
            for (other_kind, other) in types {
                if kind == other_kind {
                    continue;
                }
                for source in [
                    format!("{kind}[] a = [{other}];"),
                    format!("{kind}[] a = [{value}]; a[0] = {other};"),
                    format!("{kind}[] a = [{value}]; {other_kind}[] b = [{other}]; a = b;"),
                    format!("println([{value}, {other}]);"),
                    format!("println([{value}] == [{other}]);"),
                ] {
                    rejects_without_output(&format!("print(\"previo\");\n{source}"), "Línea 2:");
                }
            }
        }
        for source in [
            "int a = [1];",
            "int[] a = 1;",
            "int[] a = [1]; a = [[1]];",
            "int[][] a = [[1], [true]];",
            "int[] a = [a[0]];",
            "int[] a = [1]; println(a[true]);",
            "int[] a = [1]; a[1.0] = 2;",
            "println(1[0]);",
            "println(-1[0]);",
            "println(\"hola\"[0]);",
            "int a = 1; a[0] = 2;",
            "a[0] = 2;",
            "int[] a = [unknown];",
            "int[] a = []; int[] a = [];",
            "println([1] + [2]);",
            "println([1] < [2]);",
            "println(![true]);",
            "println([]);",
            "println([[], [1]]);",
            "int[] a = []; println(a == []);",
        ] {
            rejects_without_output(&format!("print(\"previo\");\n{source}"), "Línea 2:");
        }
    }

    #[test]
    fn rejects_incomplete_array_syntax_before_any_output() {
        for source in [
            "int[] a;",
            "int[2] a = [1, 2];",
            "int[] a = [1 2];",
            "int[] a = [1,];",
            "int[] a = [,1];",
            "int[] a = [1;",
            "int[] a = [1]; a[] = 2;",
            "int[] a = [1]; a[0 = 2;",
            "int[] a = [1]; a[0] = 2",
            "int[] a = [1]",
            "println([1][0);",
            "println([1][0] = 2);",
            "int[] a = [1]; a[0] = a[0] = 2;",
        ] {
            rejects_without_output(&format!("println(\"previo\");\n{source}"), "Línea 2:");
        }
    }

    #[test]
    fn array_bounds_errors_preserve_output_and_report_bracket_line() {
        for source in [
            "int[] a = [1]; println(a\n[-1]);",
            "int[] a = [1]; a\n[1] = 2;",
            "int[] a = []; println(a\n[0]);",
            "int[] a = [1]; a\n[9223372036854775807] = 2;",
            "int[][] a = [[1]]; a[0]\n[1] = 2;",
            "int[][] a = [[1]]; println(a[0]\n[-9223372036854775808]);",
        ] {
            let mut bytes = Vec::new();
            let error = run(
                &format!("println(\"previo\");\n{source} println(\"posterior\");"),
                &mut bytes,
            )
            .unwrap_err()
            .to_string();
            assert!(error.contains("Línea 3: índice"), "{source}: {error}");
            assert!(error.contains("fuera de rango"), "{error}");
            assert_eq!(bytes, b"previo\n");
        }
    }

    #[test]
    fn array_expressions_keep_evaluation_order_and_short_circuit() {
        assert_eq!(
            output("int[] a = []; println(false && a[0] == 1); println(true || a[0] == 1);"),
            "false\ntrue\n"
        );
        for (source, message) in [
            ("int[] a = [1 / 0, [1][3]];", "por cero"),
            ("int[] a = [[1][3], 1 / 0];", "índice 3 fuera de rango"),
            ("int[] a = [1]; a[3] = 1 / 0;", "índice 3 fuera de rango"),
            ("int[] a = [1]; a[1 / 0] = 2;", "por cero"),
            (
                "println(false && [1][true] == 1);",
                "el índice debe ser int",
            ),
        ] {
            rejects_without_output(source, message);
        }
    }

    #[test]
    fn accepts_whitespace_and_unicode_strings() {
        assert_eq!(
            output(" \r\nprintln \t( \"¡Hola, 世界! 🦀\" ) \n;\r\n"),
            "¡Hola, 世界! 🦀\n"
        );
    }

    #[test]
    fn executes_print_statements_in_order() {
        assert_eq!(
            output("print(\"uno\");print(\"\");println(\"dos\");\nprintln(\"\");print(\"tres\");"),
            "unodos\n\ntres"
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
            let source = format!("print(\"no debe imprimirse\");\n{invalid}");
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
    fn requires_semicolon_after_every_statement() {
        for instruction in ["print", "println"] {
            for suffix in ["", "\n", " println(\"otro\");"] {
                let source = format!("println(\"previo\");\n{instruction}(\"texto\"){suffix}");
                let mut bytes = Vec::new();
                let error = run(&source, &mut bytes).unwrap_err().to_string();
                assert!(error.contains("Se esperaba ';'"), "{source}: {error}");
                assert!(bytes.is_empty());
            }
        }
    }

    #[test]
    fn preserves_string_contents() {
        assert_eq!(output("print(\";\n世界\");println(\";\");"), ";\n世界;\n");
    }

    #[test]
    fn rejects_extra_semicolons_and_keyword_prefixes() {
        for source in [";", "print(\"x\");;", "println2(\"x\");", "println();"] {
            assert!(run(source, Vec::new()).is_err(), "{source}");
        }
    }

    #[test]
    fn reports_output_errors() {
        let mut buffer = [0_u8; 2];
        for source in ["print(\"Hello world\");", "println(\"Hello world\");"] {
            assert!(run(source, &mut buffer[..]).is_err());
        }
    }

    #[test]
    fn executes_basic_types_example() {
        assert_eq!(
            output(include_str!("../examples/tipos.oki")),
            "25\n19.95\ntrue\nñ\nHola\n26\n"
        );
    }

    #[test]
    fn prints_all_literal_types() {
        assert_eq!(
            output(
                "print(42);println(-7);println(1.0);println(-2.5);println(true);println(false);print('🦀');println(\"!\");"
            ),
            "42-7\n1.0\n-2.5\ntrue\nfalse\n🦀!\n"
        );
    }

    #[test]
    fn assigns_and_copies_each_type_without_aliasing() {
        for (kind, first, second, expected) in [
            ("int", "1", "2", "1\n2\n"),
            ("float", "1.0", "2.5", "1.0\n2.5\n"),
            ("bool", "true", "false", "true\nfalse\n"),
            ("char", "'a'", "'界'", "a\n界\n"),
            ("string", "\"uno\"", "\"dos\"", "uno\ndos\n"),
        ] {
            let source = format!(
                "{kind} original = {first}; {kind} copia = original; original = {second}; copia = copia; println(copia); println(original);"
            );
            assert_eq!(output(&source), expected);
        }
    }

    fn rejects_without_output(source: &str, message: &str) {
        let mut bytes = Vec::new();
        let error = run(source, &mut bytes).unwrap_err().to_string();
        assert!(error.contains(message), "{source}: {error}");
        assert!(
            bytes.is_empty(),
            "Se ejecutó un programa inválido: {source}"
        );
    }

    #[test]
    fn executes_constants_example() {
        assert_eq!(
            output(include_str!("../examples/constantes.oki")),
            "10\n11\n19.95\ntrue\nñ\nHola mundo\n"
        );
    }

    #[test]
    fn rejects_constant_reassignment_before_printing() {
        for (kind, initial, replacement) in [
            ("int", "1", "2"),
            ("float", "1.0", "2.0"),
            ("bool", "true", "false"),
            ("char", "'a'", "'ñ'"),
            ("string", "\"uno\"", "\"dos\""),
        ] {
            for value in [replacement, initial, "fijo"] {
                rejects_without_output(
                    &format!("const {kind} fijo = {initial};\nprintln(fijo);\nfijo = {value};"),
                    "Línea 3: No se puede reasignar la constante 'fijo'.",
                );
            }
        }
    }

    #[test]
    fn constants_copy_values_without_aliasing() {
        assert_eq!(
            output(
                r#"
                string original = "Hola";
                const string fijo = original;
                const string otro = fijo + "!";
                original = "Adiós";
                string copia = fijo;
                copia = copia + " mundo";
                println(fijo);
                println(otro);
                println(original);
                println(copia);
                int const2 = 1;
                const2 = 2;
                println(const2);
            "#
            ),
            "Hola\nHola!\nAdiós\nHola mundo\n2\n"
        );
    }

    #[test]
    fn constants_require_valid_declarations() {
        for source in [
            "const",
            "const x = 1;",
            "const int x;",
            "const int x = ;",
            "const int x = 1",
            "const int = 1;",
            "const const int x = 1;",
            "int const x = 1;",
            "int const = 1;",
            "const int const = 1;",
        ] {
            rejects_without_output(&format!("println(\"previo\");\n{source}"), "Línea 2:");
        }
        for source in [
            "const int x = x;",
            "const int x = y;",
            "println(x); const int x = 1;",
        ] {
            rejects_without_output(source, "no está declarada");
        }
        for first in ["int x = 1;", "const int x = 1;"] {
            for second in ["int x = 2;", "const int x = 2;"] {
                rejects_without_output(
                    &format!("{first} println(x); {second}"),
                    "ya está declarada",
                );
            }
        }
    }

    #[test]
    fn rejects_every_cross_type_declaration_and_assignment() {
        let values = [
            ("int", "1"),
            ("float", "1.0"),
            ("bool", "true"),
            ("char", "'x'"),
            ("string", "\"x\""),
        ];
        for (expected, initial) in values {
            for (actual, value) in values {
                if expected == actual {
                    continue;
                }
                let message = format!("se esperaba {expected}, se recibió {actual}");
                rejects_without_output(
                    &format!("println(\"previo\"); const {expected} x = {value};"),
                    &message,
                );
                rejects_without_output(
                    &format!("println(\"previo\"); {expected} x = {value};"),
                    &message,
                );
                rejects_without_output(
                    &format!("{expected} x = {initial}; println(x); x = {value};"),
                    &message,
                );
                rejects_without_output(
                    &format!("{actual} y = {value}; {expected} x = y;"),
                    &message,
                );
                rejects_without_output(
                    &format!("{expected} x = {initial}; {actual} y = {value}; x = y;"),
                    &message,
                );
            }
        }
    }

    #[test]
    fn requires_explicit_declarations_before_use() {
        for source in [
            "edad = 25;",
            "println(edad);",
            "int copia = edad;",
            "int edad = edad;",
            "println(edad); int edad = 25;",
        ] {
            rejects_without_output(source, "no está declarada");
        }
        for source in ["int x = 1; int x = 2;", "int x = 1; string x = \"x\";"] {
            rejects_without_output(source, "ya está declarada");
        }
    }

    #[test]
    fn rejects_incomplete_declarations_and_assignments() {
        for source in [
            "int x;",
            "int x = ;",
            "int x = 1",
            "int x = 1; x = 2",
            "int = 1;",
            "int bool = 1;",
            "bool true = false;",
            "int print = 1;",
            "var x = 1;",
            "let x = 1;",
            "auto x = 1;",
            "double x = 1.0;",
            "int x = 1; x = ;",
        ] {
            rejects_without_output(source, "Línea 1:");
        }
    }

    #[test]
    fn accepts_numeric_boundaries_and_exponents() {
        assert_eq!(
            output(
                "println(-9223372036854775808);println(9223372036854775807);println(0);println(-0.0);println(1e3);println(-2.5E-2);println(1e+2);println(1.7976931348623157e308);"
            ),
            "-9223372036854775808\n9223372036854775807\n0\n-0.0\n1000.0\n-0.025\n100.0\n1.7976931348623157e308\n"
        );
        for value in [
            "9223372036854775808",
            "-9223372036854775809",
            "1e309",
            "-1e309",
        ] {
            rejects_without_output(&format!("println({value});"), "fuera del rango");
        }
    }

    #[test]
    fn rejects_malformed_numbers_and_invalid_expressions() {
        for value in [
            "1.",
            ".5",
            "1e",
            "1e+",
            "1e-",
            "1.2.3",
            "12abc",
            "0x10",
            "1_000",
            "-true",
            "1 +",
            "* 2",
            "()",
            "(1 + 2",
            "1 2",
            "true & false",
            "true | false",
            "x = 2",
            "1 < 2 < 3",
            "1 ** 2",
            "!",
            "true &&",
            "false ||",
            "1 ==",
            "1 + * 2",
        ] {
            rejects_without_output(&format!("int x = 1; println({value});"), "Línea 1:");
        }
    }

    #[test]
    fn evaluates_arithmetic_precedence_grouping_and_unary_operators() {
        for (expression, expected) in [
            ("2+3*4", "14"),
            ("(2+3)*4", "20"),
            ("20-5-3", "12"),
            ("24/4/2", "3"),
            ("17%5*2", "4"),
            ("-(2+3)*+2", "-10"),
            ("+-+2", "-2"),
            ("7/2", "3"),
            ("-7/2", "-3"),
            ("7/-2", "-3"),
            ("-7%2", "-1"),
            ("7%-2", "1"),
            ("1.5+2.0*3.0", "7.5"),
            ("5.5-2.0", "3.5"),
            ("7.0/2.0", "3.5"),
            ("-7.5%2.0", "-1.5"),
            ("7.5%-2.0", "1.5"),
            ("-(1.5+2.0)", "-3.5"),
            ("+1.0", "1.0"),
            ("1e-2+1e+2", "100.01"),
        ] {
            assert_eq!(
                output(&format!("println({expression});")),
                format!("{expected}\n"),
                "{expression}"
            );
        }
    }

    #[test]
    fn expressions_work_in_declarations_assignments_and_prints() {
        assert_eq!(
            output(
                r#"
            int x = 2 + 3 * 4;
            x = (x - 4) / 2;
            float y = 2.5;
            y = -y + +y * 2.0;
            bool ok = x == 5 && y >= 2.5;
            ok = !ok || 'a' < 'ñ';
            char letter = ('界');
            string text = "Hola";
            string copy = text + " " + "🦀";
            text = text + "!";
            print(x); println(y); println(ok); println(letter); println(copy); println(text);
        "#
            ),
            "52.5\ntrue\n界\nHola 🦀\nHola!\n"
        );
    }

    #[test]
    fn compares_values_of_each_type() {
        for (first, second) in [
            ("1", "2"),
            ("1.5", "2.0"),
            ("'ñ'", "'界'"),
            ("\"a\"", "\"aa\""),
        ] {
            assert_eq!(output(&format!(
                "println({first} < {second}); println({first} <= {first}); println({second} > {first}); println({second} >= {second});
                 println({first} > {second}); println({second} <= {first}); println({first} >= {second}); println({first} < {first});"
            )), "true\ntrue\ntrue\ntrue\nfalse\nfalse\nfalse\nfalse\n");
        }
        for (first, second) in [
            ("1", "2"),
            ("1.5", "2.0"),
            ("true", "false"),
            ("'ñ'", "'界'"),
            ("\"a\"", "\"aa\""),
        ] {
            assert_eq!(
                output(&format!(
                    "println({first} == {first}); println({first} != {second}); println({first} == {second}); println({first} != {first});"
                )),
                "true\ntrue\nfalse\nfalse\n"
            );
        }
        assert_eq!(
            output(
                r#"println("A" < "a"); println("é" == "é"); println("" + "ñ" == "ñ"); println(0.0 == -0.0);"#
            ),
            "true\nfalse\ntrue\ntrue\n"
        );
    }

    #[test]
    fn evaluates_boolean_truth_tables_and_precedence() {
        for (left, right, and, or) in [
            ("false", "false", "false", "false"),
            ("false", "true", "false", "true"),
            ("true", "false", "false", "true"),
            ("true", "true", "true", "true"),
        ] {
            assert_eq!(
                output(&format!(
                    "println({left} && {right}); println({left} || {right});"
                )),
                format!("{and}\n{or}\n")
            );
        }
        assert_eq!(
            output(
                "println(!true); println(!!true); println(!false == true); println(true || false && false); println((true || false) && false); println(1+2*3 >= 7 == true && !false || false);"
            ),
            "false\ntrue\ntrue\ntrue\nfalse\ntrue\n"
        );
    }

    #[test]
    fn logical_operators_short_circuit_but_check_both_operands() {
        assert_eq!(
            output("println(false && 1/0 == 0); println(true || 9223372036854775807+1 == 0);"),
            "false\ntrue\n"
        );
        for expression in ["true && 1/0 == 0", "false || 1/0 == 0"] {
            rejects_without_output(&format!("println({expression});"), "por cero");
        }
        for expression in ["false && 1", "true || 1", "true || (1 + false == 1)"] {
            rejects_without_output(
                &format!("println(\"previo\"); println({expression});"),
                "no admite",
            );
        }
        rejects_without_output(
            "println(\"previo\"); println(true || desconocida);",
            "no está declarada",
        );
    }

    #[test]
    fn rejects_cross_type_operators_and_unsupported_type_operations() {
        let values = ["1", "1.0", "true", "'a'", "\"a\""];
        for left in values {
            for right in values {
                if left == right {
                    continue;
                }
                for operator in [
                    "+", "-", "*", "/", "%", "==", "!=", "<", "<=", ">", ">=", "&&", "||",
                ] {
                    rejects_without_output(
                        &format!("print(\"previo\");\nprintln({left} {operator} {right});"),
                        "Línea 2:",
                    );
                }
            }
        }
        for expression in [
            "true+false",
            "'a'+'b'",
            "\"a\"-\"b\"",
            "'a'*'b'",
            "true<false",
            "1&&2",
            "\"a\"||\"b\"",
            "!1",
            "+true",
            "-'a'",
            "+\"a\"",
            "!1.0",
        ] {
            rejects_without_output(
                &format!("print(\"previo\"); println({expression});"),
                "no admite",
            );
        }
        rejects_without_output(
            "print(\"previo\"); int x = 1 < 2;",
            "se esperaba int, se recibió bool",
        );
        rejects_without_output(
            "bool x = true; print(x); x = 1 + 2;",
            "se esperaba bool, se recibió int",
        );
    }

    #[test]
    fn reports_zero_division_and_overflow_without_panicking() {
        for expression in [
            "1/0", "1%0", "0/0", "1.0/0.0", "1.0%-0.0", "0.0/0.0", "1.0/-0.0",
        ] {
            rejects_without_output(&format!("println({expression});"), "por cero");
        }
        for expression in [
            "9223372036854775807+1",
            "-9223372036854775808-1",
            "9223372036854775807*2",
            "-9223372036854775808/-1",
            "-(-9223372036854775808)",
            "1e308*2.0",
            "1e308+1e308",
            "-1e308-1e308",
            "1e308/1e-308",
        ] {
            rejects_without_output(&format!("println({expression});"), "fuera del rango");
        }
        assert_eq!(
            output(
                "int min = -9223372036854775808; println(min % -1); println(min+1); println(9223372036854775807-1); println(1e-300*1e-300);"
            ),
            "0\n-9223372036854775807\n9223372036854775806\n0.0\n"
        );
        rejects_without_output(
            "int min = -9223372036854775808; println(-min);",
            "fuera del rango",
        );
    }

    #[test]
    fn operator_errors_report_the_operator_line_and_stop_execution() {
        rejects_without_output("print(\"previo\"); println(1\n+ true);", "Línea 2:");
        rejects_without_output("print(\"previo\"); println(\n!\n1);", "Línea 2:");
        let mut bytes = Vec::new();
        let error = run(
            "println(\"previo\");\nprintln(1\n/ 0); println(\"posterior\");",
            &mut bytes,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Línea 3:"), "{error}");
        assert!(error.contains("por cero"), "{error}");
        assert_eq!(bytes, b"previo\n");
    }

    #[test]
    fn executes_operations_example() {
        assert_eq!(
            output(include_str!("../examples/operaciones.oki")),
            "14\n20\n3\n1\n3.5\ntrue\ntrue\nHola, mundo\ntrue\nfalse\n"
        );
    }

    #[test]
    fn executes_conditions_example() {
        assert_eq!(
            output(include_str!("../examples/condiciones.oki")),
            "mayor de edad\nadultez\n2026\n20\nadulto\n5\n20\n"
        );
    }

    #[test]
    fn if_statements_choose_branch_and_support_else_if() {
        assert_eq!(
            output(
                "int x = 2; if (x > 3) { println(\"a\"); } else { println(\"b\"); } if (x > 3) { println(\"a\"); } else if (x == 2) { println(\"c\"); } else { println(\"d\"); } if (x == 2) { println(\"e\"); }"
            ),
            "b\nc\ne\n"
        );
        assert_eq!(
            output("if (false) { println(\"a\"); } println(\"fin\");"),
            "fin\n"
        );
    }

    #[test]
    fn blocks_have_their_own_scope_and_allow_shadowing() {
        assert_eq!(
            output(
                "int x = 1; if (true) { int x = 2; println(x); x = 3; println(x); } println(x); if (true) { x = 10; } println(x);"
            ),
            "2\n3\n1\n10\n"
        );
        rejects_without_output("if (true) { int y = 1; } println(y);", "no está declarada");
        rejects_without_output(
            "if (false) { int y = 1; } else { int z = 2; } println(z);",
            "no está declarada",
        );
        rejects_without_output(
            "const int y = 1; if (true) { y = 2; }",
            "No se puede reasignar la constante 'y'.",
        );
    }

    #[test]
    fn rejects_invalid_if_conditions_and_syntax() {
        for source in [
            "if (1) { println(\"a\"); }",
            "if (true) println(\"a\");",
            "if true { println(\"a\"); }",
            "if () { println(\"a\"); }",
            "if (true) { } else",
            "if (true) { } else println(\"a\");",
            "if (true) { int x = 1; int x = 2; }",
            "if (desconocida) { }",
            "if (true) { println(\"a\");",
            "if (true) }",
        ] {
            rejects_without_output(&format!("println(\"previo\");\n{source}"), "Línea 2:");
        }
    }

    #[test]
    fn executes_bucles_example() {
        assert_eq!(
            output(include_str!("../examples/bucles.oki")),
            "15\n11\n12\n21\n22\nAna\nLuis\nMar\n20\n"
        );
    }

    #[test]
    fn executes_asignaciones_example() {
        assert_eq!(
            output(include_str!("../examples/asignaciones.oki")),
            "2\n7\n4\n3\n12.5\n11.5\nHola, mundo\n[2, 12, 2]\n012\n"
        );
    }

    #[test]
    fn while_repeats_while_condition_is_true() {
        assert_eq!(
            output(
                "int i = 0; while (i < 3) { print(i); i = i + 1; } println(\"\"); int j = 2; while (j < 2) { println(\"no\"); } println(j);"
            ),
            "012\n2\n"
        );
        for source in [
            "while (1) { }",
            "while (desconocida) { }",
            "print(\"previo\"); while (1 + true == 2) { }",
        ] {
            rejects_without_output(source, "Línea 1:");
        }
    }

    #[test]
    fn for_loops_run_initializer_condition_and_update() {
        assert_eq!(
            output(
                "for (int i = 0; i < 3; i = i + 1) { print(i); } println(\"\"); int j = 9; for (j = 0; j < 2; j = j + 1) { print(j); } println(j); int[] a = [1, 2, 3]; for (int i = 0; i < 3; i = i + 1) { a[i] = a[i] * 2; } println(a);"
            ),
            "012\n012\n[2, 4, 6]\n"
        );
        rejects_without_output(
            "for (int i = 0; i < 1; i = i + 1) { } println(i);",
            "no está declarada",
        );
        rejects_without_output(
            "const int k = 0; for (k = 0; k < 1; k = k + 1) { }",
            "No se puede reasignar la constante 'k'.",
        );
        rejects_without_output(
            "for (const int k = 0; k < 1; k = k + 1) { }",
            "No se puede reasignar la constante 'k'.",
        );
    }

    #[test]
    fn loops_have_their_own_scope_and_allow_shadowing() {
        assert_eq!(
            output(
                "int i = 5; for (int i = 0; i < 2; i = i + 1) { println(i); } println(i); int x = 1; while (x < 2) { int y = 7; println(y); x = x + 1; } if (true) { int y = 8; println(y); }"
            ),
            "0\n1\n5\n7\n8\n"
        );
        rejects_without_output(
            "while (false) { int y = 1; } println(y);",
            "no está declarada",
        );
        rejects_without_output(
            "foreach (int n in [1]) { } println(n);",
            "no está declarada",
        );
    }

    #[test]
    fn foreach_iterates_arrays_and_copies_elements() {
        assert_eq!(
            output(
                "int[] a = [1, 2, 3]; foreach (int n in a) { print(n); } println(\"\"); string[] s = [\"x\", \"y\"]; foreach (string t in s) { println(t); } int[][] t2 = [[1, 2], [3]]; foreach (int[] fila in t2) { foreach (int n in fila) { print(n); } print(\"|\"); } println(\"\"); int[] vacio = []; foreach (int n in vacio) { println(n); } println(\"fin\");"
            ),
            "123\nx\ny\n12|3|\nfin\n"
        );
        assert_eq!(
            output(
                "int[] a = [1, 2, 3]; foreach (int n in a) { n = n * 10; a[0] = 99; print(n); print(\" \"); } println(\"\"); println(a);"
            ),
            "10 20 30 \n[99, 2, 3]\n"
        );
    }

    #[test]
    fn rejects_invalid_loop_syntax_and_foreach_types() {
        for source in [
            "while true { }",
            "while (true) println(\"a\");",
            "while (true { }",
            "while (true) { println(\"a\");",
            "for (int i = 0; i < 1; i = i + 1) println(\"a\");",
            "for (int i = 0; i < 1) { }",
            "for (i = 0; i < 1) { }",
            "for (int i = 0; i < 1; i = i + 1 { }",
            "for () { }",
            "foreach (n in a) { }",
            "foreach (int n a) { }",
            "foreach (int n in a) println(n);",
            "foreach (int n in a) { ",
        ] {
            rejects_without_output(&format!("println(\"previo\");\n{source}"), "Línea 2:");
        }
        for (source, message) in [
            ("foreach (int n in 1) { }", "solo recorre arrays"),
            (
                "int[] a = [1]; foreach (float n in a) { }",
                "se esperaba float, se recibió int",
            ),
            ("foreach (int n in desconocida) { }", "no está declarada"),
            (
                "int[][] a = [[1]]; foreach (int n in a) { }",
                "se esperaba int, se recibió int[]",
            ),
        ] {
            rejects_without_output(&format!("println(\"previo\");\n{source}"), message);
        }
    }

    #[test]
    fn char_requires_one_unicode_scalar_and_strings_keep_raw_contents() {
        assert_eq!(
            output("println('ñ');println('🦀');println('\"');println('\\');println(\"\\n\");"),
            "ñ\n🦀\n\"\n\\\n\\n\n"
        );
        for value in ["''", "'ab'", "'e\u{301}'", "'\\n'"] {
            rejects_without_output(
                &format!("char x = {value};"),
                "exactamente un carácter Unicode",
            );
        }
        rejects_without_output("char x = 'a;", "sin comillas de cierre");
    }

    #[test]
    fn errors_preserve_source_lines() {
        for (source, line) in [
            ("println(\"previo\");\nint edad = 2.0;", 2),
            ("int edad = 2;\nedad = false;", 2),
            ("println(\"uno\ndos\");\nprintln(desconocida);", 3),
            ("int edad =\n desconocida;", 2),
            ("\nchar x = 'ab';", 2),
            ("\nprintln(-9223372036854775809);", 2),
        ] {
            rejects_without_output(source, &format!("Línea {line}:"));
        }
    }

    #[test]
    fn keyword_prefixes_are_valid_variable_names_and_runs_are_isolated() {
        assert_eq!(
            output(
                "int int2 = 2; bool true_value = false; char _inicial = 'R'; string println2 = \"ok\"; println(int2);println(true_value);println(_inicial);println(println2);"
            ),
            "2\nfalse\nR\nok\n"
        );
        assert_eq!(output("int x = 1; println(x);"), "1\n");
        rejects_without_output("println(x);", "no está declarada");
    }

    #[test]
    fn increment_and_decrement_update_variables() {
        assert_eq!(
            output(
                "int i = 0; i++; i++; println(i); i--; println(i); float f = 1.5; f++; println(f); f--; f--; println(f);"
            ),
            "2\n1\n2.5\n0.5\n"
        );
    }

    #[test]
    fn compound_assignments_combine_each_supported_type() {
        assert_eq!(
            output(
                "int a = 10; a += 5; println(a); a -= 3; println(a); float b = 1.5; b += 2.0; println(b); b -= 0.5; println(b); string s = \"a\"; s += \"b\"; s += \"c\"; println(s);"
            ),
            "15\n12\n3.5\n3.0\nabc\n"
        );
    }

    #[test]
    fn updates_work_on_array_elements() {
        assert_eq!(
            output(
                "int[] a = [1, 2, 3]; a[0]++; a[1] += 10; a[2]--; println(a); int[][] m = [[1, 2], [3]]; m[0][1] += 5; m[1][0]--; println(m);"
            ),
            "[2, 12, 2]\n[[1, 7], [2]]\n"
        );
    }

    #[test]
    fn updates_work_in_for_header() {
        assert_eq!(
            output(
                "for (int i = 0; i < 3; i++) { print(i); } println(\"\"); int j = 0; for (j = 0; j < 6; j += 2) { print(j); } println(\"\");"
            ),
            "012\n024\n"
        );
    }

    #[test]
    fn rejects_updates_on_constants_before_printing() {
        for update in ["c++;", "c--;", "c += 1;", "c -= 1;"] {
            rejects_without_output(
                &format!("const int c = 1;\nprintln(c);\n{update}"),
                "Línea 3: No se puede reasignar la constante 'c'.",
            );
        }
        rejects_without_output(
            "const int[] a = [1];\nprintln(a);\na[0]++;",
            "Línea 3: No se puede reasignar la constante 'a'.",
        );
        rejects_without_output(
            "const int[][] a = [[1]];\nprintln(a);\na[0][0] += 1;",
            "Línea 3: No se puede reasignar la constante 'a'.",
        );
    }

    #[test]
    fn rejects_incompatible_update_types_before_printing() {
        for (source, message) in [
            ("float f = 1.0; f += 1;", "no admite"),
            ("int i = 1; i += 1.0;", "no admite"),
            ("string s = \"a\"; s += 'b';", "no admite"),
            ("bool b = true; b++;", "no admite"),
            ("char c = 'a'; c++;", "no admite"),
            ("string s = \"a\"; s--;", "no admite"),
            ("string s = \"a\"; s -= \"b\";", "no admite"),
            ("int[] a = [1]; a++;", "no admite"),
            ("int[] a = [1]; a -= [2];", "no admite"),
        ] {
            rejects_without_output(&format!("println(\"previo\");\n{source}"), message);
        }
    }

    #[test]
    fn rejects_incomplete_update_syntax_before_printing() {
        for source in [
            "int x = 1; x + = 2;",
            "int x = 1; x +=",
            "int x = 1; x -=",
            "int x = 1; ++x;",
            "int x = 1; x-- 2;",
            "int x = 1; x++; println(x++);",
            "int x = 1; int y = x++;",
            "x += 1;",
            "x++;",
        ] {
            rejects_without_output(&format!("println(\"previo\");\n{source}"), "Línea 2:");
        }
    }

    #[test]
    fn update_overflow_and_division_errors_stop_execution() {
        for source in [
            "int max = 9223372036854775807; max++;",
            "int min = -9223372036854775808; min--;",
            "float big = 1e308; big += 1e308;",
        ] {
            rejects_without_output(source, "fuera del rango");
        }
        rejects_without_output("int i = 1; i += 1 / 0;", "por cero");
        assert_eq!(
            output("int i = 0; i += 2 * 3; println(i); i -= 1; println(i);"),
            "6\n5\n"
        );
    }
}
