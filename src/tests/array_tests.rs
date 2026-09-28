use super::*;

#[test]
fn executes_arrays_example() {
    assert_eq!(
        output(include_str!("../../examples/arrays.oki")),
        "[1, 2, 3]\n10\n[1, 10, 3]\n[99, 10, 3]\n[\"Ana\", \"世界\"]\n[]\n3\ntrue\n"
    );
}

#[test]
fn executes_array_mutation_example() {
    assert_eq!(
        output(include_str!("../../examples/modificar_arrays.oki")),
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
