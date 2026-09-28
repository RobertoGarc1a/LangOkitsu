use super::*;

#[test]
fn executes_functions_example() {
    assert_eq!(
        output(include_str!("../../examples/funciones.oki")),
        "Hola, Ana\nHola, 世界\n10\n5\n10\n9\n10\n25\n3\n2\n1\n"
    );
}

#[test]
fn functions_bind_parameters_use_scopes_and_read_globals() {
    assert_eq!(
        output(
            "int base = 1; function sumar(int a, int b) { int c = a + b; println(c + base); } sumar(2, 3); base = 10; sumar(2, 3);"
        ),
        "6\n15\n"
    );
    assert_eq!(
        output(
            "int x = 1; function mostrar(int x) { println(x); x = 99; println(x); } mostrar(9); println(x);"
        ),
        "9\n99\n1\n"
    );
    assert_eq!(
        output(
            "int[] valores = [1, 2, 3]; function imprimir(int[] datos) { foreach (int n in datos) { print(n * 2); } println(\"\"); } imprimir(valores);"
        ),
        "246\n"
    );
    rejects_without_output(
        "function f() { int local = 1; } println(local);",
        "no está declarada",
    );
}

#[test]
fn functions_call_only_previously_declared_names_and_chain_calls() {
    assert_eq!(
        output("function a() { println(\"a\"); } function b() { a(); println(\"b\"); } b();"),
        "a\nb\n"
    );
    rejects_without_output("f(); function f() { }", "desconocid");
    rejects_without_output("function b() { a(); } function a() { }", "desconocid");
    rejects_without_output("function f() { } g();", "desconocid");
}

#[test]
fn functions_require_exact_arity_and_types_before_output() {
    rejects_without_output(
        "function f(int a) { }\nprintln(\"previo\");\nf(1.0);",
        "se esperaba int, se recibió float",
    );
    rejects_without_output(
        "function f(int[] a) { }\nprintln(\"previo\");\nf(1);",
        "se esperaba int[], se recibió int",
    );
    rejects_without_output(
        "function f(int a) { }\nprintln(\"previo\");\nf();",
        "esperaba 1 argumentos",
    );
    rejects_without_output(
        "function f() { }\nprintln(\"previo\");\nf(1);",
        "esperaba 0 argumentos",
    );
    rejects_without_output("function f(int a) { } f(desconocida);", "no está declarada");
}

#[test]
fn functions_cannot_be_values_and_calls_are_only_statements() {
    rejects_without_output("function f() { } int x = f();", "no devuelve un valor");
    rejects_without_output("function f() { } println(f());", "no devuelve un valor");
    rejects_without_output("function f() { } println(1 + f());", "no devuelve un valor");
}

#[test]
fn rejects_invalid_function_declarations_before_output() {
    for (source, message) in [
        ("function f() { } function f() { }", "ya está declarada"),
        (
            "function f() { } int f = 1;",
            "ya está declarado como función",
        ),
        (
            "int f = 1; function f() { }",
            "ya está declarado como variable",
        ),
        ("function f(int a, int a) { }", "está repetido"),
        (
            "if (true) { function f() { } }",
            "solo se permiten en el ámbito global",
        ),
        (
            "while (false) { function f() { } }",
            "solo se permiten en el ámbito global",
        ),
    ] {
        rejects_without_output(&format!("println(\"previo\");\n{source}"), message);
    }
    for source in [
        "function f { }",
        "function (int a) { }",
        "function f(int a { }",
        "function f(int a,) { }",
        "function f()",
        "function f() { ",
        "int function = 1;",
    ] {
        rejects_without_output(&format!("println(\"previo\");\n{source}"), "Línea 2:");
    }
    assert_eq!(output("int funcion = 1; println(funcion);"), "1\n");
}

#[test]
fn functions_allow_recursion_and_limit_call_depth() {
    assert_eq!(
        output(
            "function contar(int n) { if (n > 0) { print(n); contar(n - 1); } } contar(3); println(\"\");"
        ),
        "321\n"
    );
    let mut bytes = Vec::new();
    let error = run(
        "function infinito() { infinito(); } infinito();",
        &mut bytes,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("profundidad máxima de llamadas"), "{error}");
    assert!(bytes.is_empty());
}

#[test]
fn function_calls_evaluate_arguments_in_order_and_preserve_runtime_errors() {
    assert_eq!(
        output(
            "import std::Array; function consumir(int a, int b) { println(a); println(b); } int[] datos = [1, 2]; consumir(datos.pop(), datos.pop()); println(datos);"
        ),
        "2\n1\n[]\n"
    );
    let mut bytes = Vec::new();
    let error = run(
        "function dividir(int a) { println(a / 0); } println(\"previo\"); dividir(5);",
        &mut bytes,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("por cero"), "{error}");
    assert_eq!(bytes, b"previo\n");
}

#[test]
fn functions_read_constants_and_accept_arrays_with_context() {
    assert_eq!(
        output("const int limite = 7; function mostrar() { println(limite); } mostrar();"),
        "7\n"
    );
    rejects_without_output(
        "const int limite = 1; function cambiar() { limite = 2; } cambiar();",
        "No se puede reasignar la constante 'limite'.",
    );
    assert_eq!(
        output(
            "import std::Array; function longitud(int[] a) { println(a.len()); } int[] vacio = []; longitud(vacio); longitud([1, 2, 3]);"
        ),
        "0\n3\n"
    );
}

#[test]
fn typed_functions_return_each_basic_type() {
    assert_eq!(
        output(
            "function i() -> int { return 7; } function f() -> float { return 1.5; } function b() -> bool { return true; } function c() -> char { return 'ñ'; } function s() -> string { return \"Hola\"; } println(i()); println(f()); println(b()); println(c()); println(s());"
        ),
        "7\n1.5\ntrue\nñ\nHola\n"
    );
}

#[test]
fn valued_calls_compose_in_expressions_and_branch_returns() {
    assert_eq!(
        output(
            "function max(int a, int b) -> int { if (a > b) { return a; } else { return b; } } println(max(3, 8)); println(max(max(1, 5), 2) + 1);"
        ),
        "8\n6\n"
    );
}

#[test]
fn valued_functions_support_arrays_recursion_and_loops() {
    assert_eq!(
        output(
            "import std::Array; function factorial(int n) -> int { if (n <= 1) { return 1; } else { return n * factorial(n - 1); } } function buscar(int[] a, int objetivo) -> int { for (int i = 0; i < a.len(); i++) { if (a[i] == objetivo) { return i; } } return -1; } function primero(int[] a, int umbral) -> int { foreach (int n in a) { if (n > umbral) { return n; } } return 0; } int[] d = [5, 6, 7]; println(factorial(5)); println(buscar(d, 7)); println(primero([1, 2, 3], 1));"
        ),
        "120\n2\n2\n"
    );
}

#[test]
fn return_type_must_match_and_cover_all_paths() {
    rejects_without_output(
        "function f() -> int { return 1.0; } println(f());",
        "el retorno debe ser int; se recibió float",
    );
    rejects_without_output(
        "println(\"previo\");\nfunction f() -> int { println(\"x\"); }",
        "debe devolver un valor en todos los caminos",
    );
    rejects_without_output(
        "function f() -> int { return; }",
        "debe devolver un valor de tipo int",
    );
    rejects_without_output(
        "function f() -> int { if (true) { return 1; } }",
        "debe devolver un valor en todos los caminos",
    );
    rejects_without_output(
        "function f() { return 1; }",
        "no devuelve un valor; usa 'return;'",
    );
}

#[test]
fn return_only_inside_functions_and_void_early_exit() {
    rejects_without_output("return;", "'return' solo se permite dentro de una función");
    rejects_without_output(
        "if (true) { return; }",
        "'return' solo se permite dentro de una función",
    );
    assert_eq!(
        output(
            "function salir(int n) { if (n > 0) { return; } println(\"alcanzado\"); } salir(5); salir(0);"
        ),
        "alcanzado\n"
    );
}

#[test]
fn valued_and_void_functions_coexist() {
    rejects_without_output("function v() { } println(v());", "no devuelve un valor");
    assert_eq!(
        output(
            "function v() { println(\"v\"); } function g() -> int { v(); return 3; } println(g());"
        ),
        "v\n3\n"
    );
}

#[test]
fn rejects_invalid_return_type_annotations() {
    for source in [
        "function f() -> { }",
        "function f() -> int int { return 1; }",
        "function f() -> 5 { return 1; }",
    ] {
        rejects_without_output(&format!("println(\"previo\");\n{source}"), "Línea 2:");
    }
    rejects_without_output("int return = 1;", "Línea 1:");
}

#[test]
fn inout_requires_matching_modes_mutability_and_exact_types() {
    for (source, message) in [
        (
            "function f(inout int n) {} int x = 0; f(x);",
            "Falta 'inout'",
        ),
        (
            "function f(int n) {} int x = 0; f(inout x);",
            "no está declarado inout",
        ),
        (
            "function f(inout int n) {} const int x = 0; f(inout x);",
            "constante 'x'",
        ),
        (
            "function f(inout int[] n) {} const int[] x = []; f(inout x);",
            "constante 'x'",
        ),
        (
            "function f(inout int n) {} float x = 0.0; f(inout x);",
            "se esperaba int, se recibió float",
        ),
        (
            "function f(inout int[] n) {} float[] x = []; f(inout x);",
            "se esperaba int[], se recibió float[]",
        ),
        (
            "function f(inout int n) {} f(inout x);",
            "no está declarada",
        ),
        ("function f(inout int n) {} f();", "esperaba 1 argumentos"),
        ("function f(inout int n, int n) {}", "está repetido"),
    ] {
        rejects_without_output(&format!("println(\"previo\");\n{source}"), message);
    }
    rejects_without_output("function f(inout int n) {}\nint x = 0;\nf(x);", "Línea 3:");
}

#[test]
fn inout_syntax_only_accepts_whole_variables_in_user_calls() {
    for source in [
        "function f(inout int n) {} f(inout 5);",
        "function f(inout int n) {} int x = 0; f(inout x + 1);",
        "function f(inout int n) {} int[] x = [0]; f(inout x[0]);",
        "function f(inout int n) {} int x = 0; f(inout (x));",
        "function f(inout int n) {} function g() -> int { return 0; } f(inout g());",
        "function f(int inout n) {}",
        "function f(inout n) {}",
        "int inout = 0;",
        "int x = 0; println(inout x);",
        "import std::Array; int[] x = []; x.push(inout x);",
        "import std::Array; int[] x = []; std::Array::len(inout x);",
        "import std::Casting; int x = 0; float(inout x);",
        "function f(inout int n) {} int x = 0; f(inout x)",
    ] {
        rejects_without_output(&format!("println(\"previo\");\n{source}"), "Línea");
    }
    assert_eq!(output("int inout2 = 7; println(inout2);"), "7\n");
}

#[test]
fn functions_cannot_write_globals_or_pass_them_on_as_inout() {
    for statement in [
        "total = 1;",
        "total += 1;",
        "total -= 1;",
        "total++;",
        "total--;",
    ] {
        rejects_without_output(
            &format!("int total = 0; println(\"previo\"); function f() {{ {statement} }}"),
            "No se puede modificar la variable global 'total'",
        );
    }
    for statement in [
        "datos[0] = 1;",
        "datos[0]++;",
        "datos.push(1);",
        "datos.pop();",
        "std::Array::push(datos, 1);",
        "std::Array::pop(datos);",
    ] {
        rejects_without_output(
            &format!("import std::Array; int[] datos = [0]; function f() {{ {statement} }}"),
            "No se puede modificar la variable global 'datos'",
        );
    }
    rejects_without_output(
        "int x = 0; function f(inout int n) { n++; } function g() { f(inout x); } g();",
        "No se puede modificar la variable global 'x'",
    );
    assert_eq!(
        output(
            "int x = 0; if (true) { x++; } function f(inout int n) { n++; } if (true) { f(inout x); } println(x);"
        ),
        "2\n"
    );
}

#[test]
fn inout_forwards_through_calls_recursion_and_early_returns() {
    assert_eq!(
        output(
            r#"
        function sumar(inout int n, int veces) -> int {
            if (veces == 0) { return n; }
            n++;
            return sumar(inout n, veces - 1);
        }
        function exterior(inout int n) { println(sumar(inout n, 3)); }
        function salir(inout int n) { while (true) { n += 2; return; } }
        function local() { int propio = 10; exterior(inout propio); salir(inout propio); println(propio); }
        int total = 0; exterior(inout total); salir(inout total); println(total); local();
    "#
        ),
        "3\n5\n13\n15\n"
    );
}

#[test]
fn inout_respects_lexical_scope_shadowing_and_value_copies() {
    assert_eq!(
        output(
            r#"
        int total = 10;
        function leer() -> int { return total; }
        function cambiar(inout int destino) {
            destino++;
            if (true) { int destino = 100; destino++; println(destino); }
            destino++;
        }
        function local(int total) {
            println(leer()); cambiar(inout total); println(total);
        }
        local(1);
        if (true) { int total = 20; println(leer()); cambiar(inout total); println(total); }
        println(total);
    "#
        ),
        "10\n101\n3\n10\n101\n22\n10\n"
    );
}

#[test]
fn inout_arrays_support_mutation_replacement_and_independent_copies() {
    assert_eq!(
        output(
            r#"
        import std::Array;
        function editar(inout int[][] tabla) {
            tabla[0][0] += 4;
            tabla[0].push(9);
            println(tabla[0].pop());
            tabla.push([7]);
        }
        function reemplazar(inout int[][] tabla) { tabla = [[42]]; }
        function copia(int[][] tabla) { tabla[0][0] = 99; }
        int[][] datos = [[1]]; int[][] copia_inicial = datos;
        editar(inout datos); copia(datos); println(datos); println(copia_inicial);
        reemplazar(inout datos); println(datos);
        int[][] vacio = []; reemplazar(inout vacio); println(vacio);
    "#
        ),
        "9\n[[5], [7]]\n[[1]]\n[[42]]\n[[42]]\n"
    );
}

#[test]
fn inout_aliases_are_live_even_when_the_same_variable_is_passed_twice() {
    assert_eq!(
        output(
            r#"
        int total = 0;
        function cambiar(inout int a, inout int b) {
            a = 5; println(b); println(total); b += 2; println(a);
        }
        cambiar(inout total, inout total); println(total);
        import std::Array;
        function arrays(inout int[] a, inout int[] b) { a = [9]; b.push(10); }
        int[] datos = []; arrays(inout datos, inout datos); println(datos);
    "#
        ),
        "5\n5\n7\n7\n[9, 10]\n"
    );
}

#[test]
fn inout_arguments_evaluate_left_to_right_and_obey_short_circuiting() {
    assert_eq!(
        output(
            r#"
        function aumentar(inout int n) -> int { n++; return n; }
        function mostrar(int antes, inout int actual, int despues) {
            println(antes); println(actual); println(despues); actual++;
        }
        int total = 0;
        mostrar(total, inout total, aumentar(inout total));
        println(total);
        println(false && aumentar(inout total) > 0);
        println(true || aumentar(inout total) > 0);
        println(total);
    "#
        ),
        "0\n1\n1\n2\nfalse\ntrue\n2\n"
    );
}

#[test]
fn inout_preserves_completed_changes_and_closes_scopes_on_runtime_error() {
    let source = r#"
        int total = 0;
        function fallar(inout int destino) {
            destino = 7;
            println(destino);
            destino += 1 / 0;
        }
        fallar(inout total);
        println("no alcanzado");
    "#;
    let statements = Parser::new(Scanner::new(source).scan_tokens().unwrap())
        .parse()
        .unwrap();
    TypeChecker::default().check(&statements).unwrap();
    let mut bytes = Vec::new();
    let mut interpreter = Interpreter::new(&mut bytes);
    let error = interpreter.interpret(&statements).unwrap_err().to_string();
    assert!(
        error.contains("Línea 6:") && error.contains("por cero"),
        "{error}"
    );
    let inspect = Parser::new(Scanner::new("println(total);").scan_tokens().unwrap())
        .parse()
        .unwrap();
    interpreter.interpret(&inspect).unwrap();
    assert_eq!(bytes, b"7\n7\n");
}

#[test]
fn inout_updates_originals_while_value_parameters_keep_copies() {
    assert_eq!(
        output(
            r#"
        function acumular(inout int destino, int valor) { destino += valor; }
        function copia(int destino) { destino = 99; }
        int total = 0;
        acumular(inout total, 5);
        copia(total);
        acumular(inout total, 3);
        println(total);
        function cambiar(inout float f, inout bool b, inout char c, inout string s) {
            f += 0.5; b = !b; c = 'ñ'; s += "!";
        }
        float precio = 1.0; bool activo = false; char letra = 'a'; string texto = "Hola";
        cambiar(inout precio, inout activo, inout letra, inout texto);
        println(precio); println(activo); println(letra); println(texto);
    "#
        ),
        "8\n1.5\ntrue\nñ\nHola!\n"
    );
}
