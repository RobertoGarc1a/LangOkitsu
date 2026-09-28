use super::*;

#[test]
fn executes_casting_example() {
    assert_eq!(
        output(include_str!("../../examples/conversiones.oki")),
        "25.0\nEdad: 25\n19\n19.95\n50\ntrue\nñ\n241\n25.0\n25\n"
    );
}

#[test]
fn casting_forms_return_the_same_values_for_all_supported_pairs() {
    for (source, target, expected) in [
        ("25", "int", "25"),
        ("1.0", "float", "1.0"),
        ("true", "bool", "true"),
        ("'ñ'", "char", "ñ"),
        (r#""hola""#, "string", "hola"),
        ("25", "float", "25.0"),
        ("2.9", "int", "2"),
        ("-2.9", "int", "-2"),
        ("-0.9", "int", "0"),
        ("25", "string", "25"),
        ("1.0", "string", "1.0"),
        ("false", "string", "false"),
        ("'🦀'", "string", "🦀"),
        (r#""+42""#, "int", "42"),
        (r#""-42""#, "int", "-42"),
        (r#""2.5e1""#, "float", "25.0"),
        (r#"".5""#, "float", "0.5"),
        (r#""1.""#, "float", "1.0"),
        (r#""false""#, "bool", "false"),
        (r#""true""#, "bool", "true"),
        (r#""🦀""#, "char", "🦀"),
        ("'🦀'", "int", "129408"),
        ("129408", "char", "🦀"),
    ] {
        let calls = [
            format!("{target}({source})"),
            format!("({source}).cast({target})"),
            format!("std::Casting::{target}({source})"),
            format!("Casting::{target}({source})"),
        ];
        for call in calls {
            assert_eq!(
                output(&format!(
                    "import std::Casting; use std::Casting; {target} resultado = {call}; println(resultado);"
                )),
                format!("{expected}\n"),
                "{call}"
            );
        }
    }
}

#[test]
fn casts_compose_without_changing_sources_or_evaluating_twice() {
    assert_eq!(
        output(
            r#"
        import std::Casting; import std::Array;
        const int edad = 25;
        println("Edad: " + string(edad));
        println(25.cast(float) + 0.5);
        println((edad + 1).cast(float).cast(string));
        int[] a = [1, 2, 3];
        println(float(a.pop()) + a.pop().cast(float));
        println(a);
        float[] b = [float(edad), a[0].cast(float)];
        b.push(float(2)); println(b);
        println(false && bool("inválido"));
        println(true || int("inválido") == 0);
        if (false) { println(char("ab")); }
        float(edad); edad.cast(string); 2.cast(string); "x".cast(char);
        println(edad);
    "#
        ),
        "Edad: 25\n25.5\n26.0\n5.0\n[1]\n[25.0, 1.0, 2.0]\nfalse\ntrue\n25\n"
    );
}

#[test]
fn casting_checks_import_order_names_and_independent_libraries() {
    for call in [
        "float(2)",
        "2.cast(float)",
        "std::Casting::float(2)",
        "Casting::float(2)",
    ] {
        rejects_without_output(
            &format!("println(1); println({call}); import std::Casting;"),
            "requiere un 'import std::Casting;' anterior",
        );
    }
    rejects_without_output(
        "import std::Array; println(float(2));",
        "import std::Casting",
    );
    rejects_without_output(
        "import std::Casting; println([1].len());",
        "import std::Array",
    );
    rejects_without_output(
        "use std::Casting; import std::Casting;",
        "requiere un 'import std::Casting;' anterior",
    );
    rejects_without_output(
        "import std::Casting; println(Casting::int(1)); use std::Casting;",
        "requiere un 'use std::Casting;' anterior",
    );
    rejects_without_output(
        "import std::Casting; import std::Array; use std::Array; println(Casting::int(1));",
        "use std::Casting",
    );
    for source in [
        "if (false) { import std::Casting; }",
        "while (false) { use std::Casting; }",
    ] {
        rejects_without_output(source, "solo se permiten en el ámbito global");
    }
    for call in [
        "std::Array::int(1)",
        "Other::float(1)",
        "std::casting::int(1)",
        "std::Casting::missing(1)",
        "cast(1)",
    ] {
        rejects_without_output(
            &format!("import std::Casting; println({call});"),
            "desconocid",
        );
    }
    assert_eq!(
        output(
            "import std::Casting; import std::Casting; use std::Casting; use std::Casting; int Casting = 3; int cast = 4; println(Casting::int(cast) + Casting);"
        ),
        "7\n"
    );
    rejects_without_output("println(int(1));", "import std::Casting");
}

#[test]
fn casting_rejects_unsupported_pairs_and_preserves_strict_types() {
    for call in [
        "int(true)",
        "float(false)",
        "bool(1)",
        "bool(1.0)",
        "char(1.0)",
        "float('a')",
        "bool('a')",
        "char(true)",
        "string([1])",
        "[1].cast(int[])",
        "1.cast(int[])",
    ] {
        rejects_without_output(
            &format!("import std::Casting; println(1); println({call});"),
            "Conversión no admitida",
        );
    }
    for source in [
        "float x = 1;",
        "int n = 1; n = float(n);",
        "println(1 + float(2));",
        "println(false && bool(1));",
        "float[] a = [1];",
        "const int n = 1; n = int(2);",
    ] {
        rejects_without_output(
            &format!("import std::Casting; println(1); {source}"),
            "Línea 1:",
        );
    }
    rejects_without_output(
        "import std::Casting; println(string([]));",
        "array vacío necesita",
    );
    rejects_without_output(
        "import std::Casting; import std::Array; int[] a = []; println(int(a.push(1)));",
        "no devuelve un valor",
    );
}

#[test]
fn casting_rejects_invalid_syntax_and_arity_before_execution() {
    for source in [
        "println(int());",
        "println(float(1, 2));",
        "println(std::Casting::int());",
        "println(1.cast());",
        "println(1.cast(int, float));",
        "println(1.cast(\"int\"));",
        "int destino = 0; println(1.cast(destino));",
        "println(1.cast(int)); float(2)",
        "println(int);",
        "println(1.cast(int);",
        "println(float(1,));",
        "int float = 1;",
        "println(1.cast(int()));",
        "println(Casting::float(1, 2));",
    ] {
        rejects_without_output(
            &format!("import std::Casting; use std::Casting; println(1); {source}"),
            "Línea 1:",
        );
    }
}

#[test]
fn casting_validates_numeric_and_unicode_boundaries() {
    assert_eq!(
        output(
            r#"import std::Casting;
        println(int("9223372036854775807"));
        println(int("-9223372036854775808"));
        println(int(-9223372036854775808.0));
        println(int(9223372036854774784.0));
        println(float(9007199254740993) == 9007199254740992.0);
        println(float("1e-9999"));
        println(int(char(0))); println(int(char(55295)));
        println(int(char(57344))); println(int(char(1114111)));
    "#
        ),
        "9223372036854775807\n-9223372036854775808\n-9223372036854775808\n9223372036854774784\ntrue\n0.0\n0\n55295\n57344\n1114111\n"
    );
}

#[test]
fn casting_runtime_errors_preserve_output_and_conversion_lines() {
    for (value, target) in [
        ("9223372036854775808.0", "int"),
        ("-9223372036854777856.0", "int"),
        (r#""9223372036854775808""#, "int"),
        (r#""-9223372036854775809""#, "int"),
        (r#""1.5""#, "int"),
        (r#""1e2""#, "int"),
        (r#""0xff""#, "int"),
        (r#"" 1""#, "int"),
        (r#""""#, "int"),
        (r#""+""#, "int"),
        (r#""1e999""#, "float"),
        (r#""NaN""#, "float"),
        (r#""inf""#, "float"),
        (r#"" 1.0""#, "float"),
        (r#""1.0 ""#, "float"),
        (r#""x""#, "float"),
        (r#""True""#, "bool"),
        (r#""1""#, "bool"),
        (r#""""#, "char"),
        (r#""ab""#, "char"),
        (r#""é""#, "char"),
        ("-1", "char"),
        ("55296", "char"),
        ("57343", "char"),
        ("1114112", "char"),
    ] {
        for call in [
            format!("{target}({value})"),
            format!("({value}).cast({target})"),
        ] {
            let source = format!(
                "import std::Casting;\nprintln(\"previo\");\nprintln({call}); println(\"posterior\");"
            );
            let mut bytes = Vec::new();
            let error = run(&source, &mut bytes).unwrap_err().to_string();
            assert!(
                error.starts_with("Línea 3: No se puede convertir"),
                "{source}: {error}"
            );
            assert_eq!(bytes, b"previo\n");
        }
    }
    rejects_without_output(
        "import std::Casting; println((1 / 0).cast(float));",
        "por cero",
    );
    rejects_without_output(
        "import std::Casting; println([1][2].cast(float));",
        "fuera de rango",
    );
    rejects_without_output(
        "import std::Casting; println(1.0\n.cast(bool));",
        "Línea 2: Conversión no admitida",
    );
}
