use super::*;

#[test]
fn executes_conditions_example() {
    assert_eq!(
        output(include_str!("../../examples/condiciones.oki")),
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
        output(include_str!("../../examples/bucles.oki")),
        "15\n11\n12\n21\n22\nAna\nLuis\nMar\n20\n"
    );
}

#[test]
fn executes_asignaciones_example() {
    assert_eq!(
        output(include_str!("../../examples/asignaciones.oki")),
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
fn break_and_continue_follow_each_loop_semantics() {
    assert_eq!(
        output(
            "int i = 0; while (true) { i++; if (i == 2) { continue; } if (i == 4) { break; } print(i); } println(\"!\"); for (int n = 0; n < 5; n++) { if (n == 1) { continue; } if (n == 3) { break; } print(n); } println(\"!\"); int[] a = [1, 2, 3, 4]; foreach (int n in a) { if (n == 2) { continue; } if (n == 4) { break; } print(n); }"
        ),
        "13!\n02!\n13"
    );
}

#[test]
fn loop_control_targets_inner_loop_and_closes_scopes() {
    assert_eq!(
        output(
            "int n = 0; while (n < 2) { int oculto = n; n++; while (true) { if (oculto == 0) { break; } break; } println(n); }"
        ),
        "1\n2\n"
    );
    assert_eq!(
        output(
            "int valor = 1; while (true) { if (true) { int valor = 2; break; } } println(valor); int i = 0; while (i < 2) { i++; if (true) { int valor = 3; continue; } } println(valor);"
        ),
        "1\n1\n"
    );
}

#[test]
fn loop_control_outside_a_loop_is_rejected_before_execution() {
    rejects_without_output(
        "println(\"previo\"); break;",
        "solo se permite dentro de un bucle",
    );
    rejects_without_output(
        "if (true) { continue; }",
        "solo se permite dentro de un bucle",
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
