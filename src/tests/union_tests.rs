use super::*;

#[test]
fn union_returns_execute_both_branches_and_preserve_logical_or() {
    assert_eq!(
        output(include_str!("../../examples/retornos_union.oki")),
        "5\ntest\n"
    );
    assert_eq!(
        output(
            r#"
        function escoger(int n) -> int || float || bool || char || string {
            if (n == 0) { return 7; }
            if (n == 1) { return 2.5; }
            if (n == 2) { return false || true; }
            if (n == 3) { return 'ñ'; }
            return "texto";
        }
        for (int n = 0; n < 5; n++) { println(escoger(n)); }
        println(false || true && false);
    "#
        ),
        "7\n2.5\ntrue\nñ\ntexto\nfalse\n"
    );
}

#[test]
fn union_returns_forward_subsets_reordered_types_and_recursion() {
    assert_eq!(
        output(
            r#"
        function origen(int n) -> int || string {
            if (n > 0) { return n; } else { return "fin"; }
        }
        function inversa(int n) -> string || int { return origen(n); }
        function amplia(int n) -> bool || string || int { return inversa(n); }
        function recursiva(int n) -> int || string {
            if (n > 0) { return recursiva(n - 1); }
            return origen(n);
        }
        function repetida() -> int || int { return 3; }
        function duplicada() -> int || string || int { return origen(1); }
        int numero = repetida();
        println(amplia(5)); println(recursiva(2)); println(numero);
        println(duplicada());
    "#
        ),
        "5\nfin\n3\n1\n"
    );
    rejects_without_output(
        "function origen() -> int || string { return 1; } function destino() -> int || bool { return origen(); }",
        "el retorno debe ser int || bool; se recibió int || string",
    );
    rejects_without_output(
        "function origen() -> int || string { return 1; } function destino() -> int { return origen(); }",
        "el retorno debe ser int; se recibió int || string",
    );
}

#[test]
fn union_returns_reject_wrong_types_missing_values_and_incomplete_paths() {
    for (source, error) in [
        (
            "function f() -> int || string {\nreturn true;\n}",
            "Línea 3: el retorno debe ser int || string; se recibió bool",
        ),
        (
            "function f() -> int || string { return; }",
            "debe devolver un valor de tipo int || string",
        ),
        (
            "function f() -> int || string { if (true) { return 1; } }",
            "todos los caminos",
        ),
        (
            "function f() -> int || string { if (true) { return 1; } else { return false; } }",
            "se recibió bool",
        ),
    ] {
        rejects_without_output(&format!("println(\"previo\");\n{source}"), error);
    }
}

#[test]
fn union_returns_do_not_narrow_to_a_single_type_at_call_sites() {
    let definition = "function f() -> int || string { return 1; }";
    for (usage, error) in [
        ("int n = f();", "se esperaba int, se recibió int || string"),
        (
            "int n = 0; n = f();",
            "se esperaba int, se recibió int || string",
        ),
        ("function g(int n) {} g(f());", "Argumento incompatible"),
        ("println(f() + 1);", "el operador '+'"),
        ("println(f() == f());", "el operador '=='"),
        ("println(!f());", "el operador '!'"),
        ("if (f()) { println(1); }", "bool"),
        ("println([f(), f()]);", "tipo de elemento concreto"),
        (
            "import std::Casting; println(string(f()));",
            "Conversión no admitida",
        ),
        (
            "import std::Casting; println(f().cast(int));",
            "Conversión no admitida",
        ),
        (
            "import std::Array; int[] a = []; a.push(f());",
            "se esperaba int, se recibió int || string",
        ),
    ] {
        rejects_without_output(&format!("println(\"previo\"); {definition} {usage}"), error);
    }
}

#[test]
fn union_return_arrays_use_unambiguous_context_and_remain_homogeneous() {
    assert_eq!(
        output(
            r#"
        function datos(bool vacio) -> int[] || string {
            if (vacio) { return []; } else { return [1, 2]; }
        }
        function matriz() -> int[][] || string { return [[], [1]]; }
        function varios(bool texto) -> int[] || string[] {
            if (texto) { return ["hola"]; } else { return [3]; }
        }
        function vacio() -> int[] || string[] { int[] a = []; return a; }
        println(datos(true)); println(datos(false)); println(matriz());
        println(varios(true)); println(varios(false)); println(vacio());
    "#
        ),
        "[]\n[1, 2]\n[[], [1]]\n[\"hola\"]\n[3]\n[]\n"
    );
    for (source, error) in [
        (
            "function f() -> int[] || string[] { return []; }",
            "array es ambiguo",
        ),
        (
            "function f() -> int[][] || string[][] { return [[]]; }",
            "array es ambiguo",
        ),
        (
            "function f() -> int[] || string { return [true]; }",
            "se recibió bool[]",
        ),
        (
            "function f() -> int[] || string[] { return [1, \"a\"]; }",
            "elemento de array incompatible",
        ),
        (
            "function f() -> int[] || string { return [1]; } println(f()[0]);",
            "indexar",
        ),
        (
            "import std::Array; function f() -> int[] || string { return [1]; } println(f().len());",
            "array",
        ),
    ] {
        rejects_without_output(source, error);
    }
}

#[test]
fn union_annotations_reject_invalid_syntax_and_union_parameters() {
    for source in [
        "function f() -> int || {}",
        "function f() -> || int { return 1; }",
        "function f() -> int | string { return 1; }",
        "function f() -> int || || string { return 1; }",
        "function f() -> int || string",
        "function f() -> int || string { return 1 }",
        "function f(int || string n) {}",
    ] {
        rejects_without_output(source, "Línea 1:");
    }
}

#[test]
fn union_variables_example_and_basic_type_tests() {
    assert_eq!(
        output(include_str!("../../examples/variables_union.oki")),
        "6\ntest!\nfalse\n"
    );
    assert_eq!(
        output(
            r#"
        int || string || float || bool || char valor = 1;
        println(type valor == int); println(type valor != string);
        valor = "hola"; println(type valor == string);
        valor = 2.5; println(type valor == float);
        valor = true; println(type valor == bool);
        valor = 'ñ'; println(type valor == char);
        bool prueba = type valor != char; println(prueba);
        int typewriter = 2; println(type typewriter == int);
    "#
        ),
        "true\ntrue\ntrue\ntrue\ntrue\ntrue\nfalse\ntrue\n"
    );
}

#[test]
fn union_variables_copy_accept_subsets_and_reject_other_types() {
    assert_eq!(
        output(
            r#"
        int || string a = 5;
        string || int || bool b = a;
        a = "nuevo";
        println(a); println(b);
        const string || int || bool c = b;
        println(c);
        int || int n = 4; println(n + 1);
    "#
        ),
        "nuevo\n5\n5\n5\n"
    );
    for (source, error) in [
        ("int || string a = true;", "se recibió bool"),
        ("int || string a = 1; a = false;", "se recibió bool"),
        (
            "int || string a = 1; int b = a;",
            "se recibió int || string",
        ),
        (
            "int || bool a = 1; int || string b = a;",
            "se recibió int || bool",
        ),
        ("const int || string a = 1; a = 1;", "constante"),
        ("int || string a = a;", "no está declarada"),
        ("int || string a = 1; println(a + 1);", "el operador '+'"),
    ] {
        rejects_without_output(source, error);
    }
}

#[test]
fn type_guards_narrow_then_else_else_if_and_value_arguments() {
    assert_eq!(
        output(
            r#"
        import std::Casting;
        function doble(int n) -> int { return n * 2; }
        function leer(bool texto) -> int || string || bool {
            int || string || bool valor = 6;
            if (texto) { valor = "sí"; }
            if (type valor == int) { return doble(valor); }
            else if (type valor == string) { return valor + "!"; }
            else { return !valor; }
        }
        println(leer(false)); println(leer(true));
        int || string valor = "hola";
        if (type valor != int) { println(valor + "!"); }
        else { println(valor + 1); }
        if (!(type valor != string)) { println(string(valor)); }
    "#
        ),
        "12\nsí!\nhola!\nhola\n"
    );
    rejects_without_output(
        "int || string x = 1; if (type x == int) { println(x + 1); } println(x + 1);",
        "el operador '+'",
    );
    rejects_without_output(
        "int || string x = 1; bool entero = type x == int; if (entero) { println(x + 1); }",
        "el operador '+'",
    );
}

#[test]
fn type_guards_follow_short_circuit_truth_paths() {
    assert_eq!(
        output(
            r#"
        int || string x = 4;
        if (type x == int && x > 3) { println(x + 2); }
        println(type x != int || x > 3);
        if (!(type x != int || x < 0)) { println(x + 3); }
        x = "hola";
        if (type x == int && x > 3) { println(x); }
        else if (type x == string && x == "hola") { println(x + "!"); }
        println(type x == string || x > 0);
        if (type x != int || x < 0) { println(x); }
        else { println(x + 10); }
        if ((type x == int && true) || (type x == int && false)) { println(x + 1); }
        int || string || bool tres = false;
        if (type tres == int || type tres == string) {
            int || string dos = tres;
            println(dos);
        } else { println(!tres); }
    "#
        ),
        "6\ntrue\n7\nhola!\ntrue\nhola\ntrue\n"
    );
    rejects_without_output(
        "int || string x = 1; if (type x == int || true) { println(x + 1); }",
        "el operador '+'",
    );
    rejects_without_output(
        "int || string x = 1; if (type x == int && true) {} else { println(x + 1); }",
        "el operador '+'",
    );
    rejects_without_output(
        "int || string x = 1; println(type x == int || desconocida);",
        "no está declarada",
    );
}

#[test]
fn type_guards_forget_reassignments_and_preserve_shadowing() {
    assert_eq!(
        output(
            r#"
        int || string x = 1;
        if (type x == int) {
            x++; x += 2; println(x + 1);
            if (true) { string x = "local"; println(x + "!"); }
            println(x + 1);
            x = "cambio";
            if (type x == string) { println(x + "!"); }
        }
        println(x);
    "#
        ),
        "5\nlocal!\n5\ncambio!\ncambio\n"
    );
    for source in [
        "int || string x = 1; if (type x == int) { x = \"hola\"; println(x + 1); }",
        "int || string x = 1; if (type x == int) { if (true) { x = \"hola\"; } println(x + 1); }",
        "int || string x = 1; if (type x == int) { x = 2; println(x + 1); }",
        "int || string x = 1; if (type x == int) { string x = \"local\"; println(x + 1); }",
    ] {
        rejects_without_output(source, "el operador '+'");
    }
    rejects_without_output(
        "const int || string x = 1; if (type x == int) { x++; }",
        "constante",
    );
}

#[test]
fn type_guards_work_in_loops_and_forget_previous_iterations() {
    assert_eq!(
        output(
            r#"
        int || string x = 0;
        while (type x == int && x < 3) { println(x + 1); x++; }
        for (int || string n = 0; type n == int && n < 3; n++) {
            if (n == 1) { continue; }
            println(n + 10);
        }
        while (type x == int) { x = "fin"; continue; }
        println(x);
    "#
        ),
        "1\n2\n3\n10\n12\nfin\n"
    );
    for source in [
        "int || string x = 1; if (type x == int) { while (true) { println(x + 1); x = \"hola\"; } }",
        "int || string x = 1; if (type x == int) { while (x > 0) { x = \"hola\"; } }",
        "int || string x = 1; if (type x == int) { foreach (int n in [1, 2]) { println(x + 1); x = \"hola\"; } }",
        "int || string x = 1; for (int n = 0; type x == int; x++) { x = \"hola\"; }",
        "int || string x = 1; if (type x == int) { while (type x == int) { x = \"hola\"; break; } println(x + 1); }",
    ] {
        rejects_without_output(source, "el operador");
    }
}

#[test]
fn type_guards_keep_empty_array_types_through_calls_mutation_and_copies() {
    assert_eq!(
        output(
            r#"
        import std::Array;
        function vacio(bool texto) -> int[] || string[] {
            if (texto) { string[] a = []; return a; } else { int[] a = []; return a; }
        }
        int[] || string[] datos = vacio(false);
        println(type datos == int[]); println(type datos == string[]);
        if (type datos == int[]) { datos.push(5); println(datos.pop()); println(type datos == int[]); }
        string[] || int[] copia = datos;
        datos = vacio(true);
        println(type datos == string[]); println(type copia == int[]);
        if (type datos == string[]) { datos.push("hola"); println(datos[0] + "!"); }
        int[][] || string[][] tabla = [[], [1]];
        if (type tabla == int[][]) {
            tabla[0] = []; tabla.push([]);
            int[] fila = tabla.pop(); println(type fila == int[]);
            int[] otra = []; println(tabla[0] == otra);
        }
    "#
        ),
        "true\nfalse\n5\ntrue\ntrue\ntrue\nhola!\ntrue\ntrue\n"
    );
    rejects_without_output("int[] || string[] x = [];", "ambiguo");
    rejects_without_output(
        "int || string x = 1; println([x]);",
        "tipo de elemento concreto",
    );
}

#[test]
fn type_guards_keep_union_storage_out_of_concrete_inout_parameters() {
    assert_eq!(
        output(
            r#"
        function cambiar(inout int n) { n++; }
        int || string x = 1;
        if (type x == int) { int copia = x; cambiar(inout copia); println(copia); }
    "#
        ),
        "2\n"
    );
    rejects_without_output(
        "function cambiar(inout int n) { n++; } int || string x = 1; if (type x == int) { cambiar(inout x); }",
        "Argumento incompatible",
    );
    rejects_without_output(
        "int || string x = 1; function cambiar() { if (type x == int) { x++; } }",
        "variable global",
    );
}

#[test]
fn type_tests_reject_invalid_syntax_and_unknown_names_before_output() {
    for source in [
        "int || x = 1;",
        "const int || string x;",
        "int || string x = 1",
        "int type = 1;",
        "int x = 1; if (type x = int) {}",
        "int x = 1; if (type x ==) {}",
        "int x = 1; if (type x == 1) {}",
        "int x = 1; if (type int) {}",
        "if (type 1 == int) {}",
        "int[] x = []; if (type x[0] == int) {}",
        "bool x = type desconocida == int;",
    ] {
        rejects_without_output(&format!("println(\"previo\");\n{source}"), "Línea 2:");
    }
}
