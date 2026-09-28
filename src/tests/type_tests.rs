use super::*;

#[test]
fn executes_basic_types_example() {
    assert_eq!(
        output(include_str!("../../examples/tipos.oki")),
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

#[test]
fn executes_constants_example() {
    assert_eq!(
        output(include_str!("../../examples/constantes.oki")),
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
        output(include_str!("../../examples/operaciones.oki")),
        "14\n20\n3\n1\n3.5\ntrue\ntrue\nHola, mundo\ntrue\nfalse\n"
    );
}
