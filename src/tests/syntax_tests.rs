use super::*;

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
