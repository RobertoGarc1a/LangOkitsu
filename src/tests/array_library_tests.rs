use super::*;

#[test]
fn executes_array_library_example() {
    assert_eq!(
        output(include_str!("../../examples/biblioteca_arrays.oki")),
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
            &format!("import std::Array; use std::Array; println(\"previo\"); println({call});"),
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
            &format!("import std::Array; use std::Array; println(\"previo\"); println({call});"),
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
            &format!("import std::Array; use std::Array; println(\"previo\"); println({call});"),
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
    let error = run(
        "import std::Array; println(\"previo\");\nprintln([1 / 0].len()); println(\"posterior\");",
        &mut bytes,
    )
    .unwrap_err()
    .to_string();
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
