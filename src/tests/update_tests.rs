use super::*;

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
