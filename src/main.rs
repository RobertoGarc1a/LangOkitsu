use std::{env, error::Error, fs, io, io::Write, process::ExitCode};

mod interpreter;
mod parser;
mod scanner;
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
    fn executes_arrays_example() {
        assert_eq!(
            output(include_str!("../examples/arrays.oki")),
            "[1, 2, 3]\n10\n[1, 10, 3]\n[99, 10, 3]\n[\"Ana\", \"世界\"]\n[]\n3\ntrue\n"
        );
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
            ("--1", "1"),
            ("1--2", "3"),
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
            "--9223372036854775808",
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
}
