use crate::{
    interpreter::Interpreter, parser::Parser, run, scanner::Scanner, type_checker::TypeChecker,
};

fn output(source: &str) -> String {
    let mut bytes = Vec::new();
    run(source, &mut bytes).unwrap();
    String::from_utf8(bytes).unwrap()
}

fn rejects(source: &str, expected: &str) {
    let source = format!("println(\"no debe salir\");\n{source}");
    let mut bytes = Vec::new();
    let error = run(&source, &mut bytes).unwrap_err().to_string();
    assert!(error.contains(expected), "{source}: {error}");
    assert!(bytes.is_empty(), "{source}: {bytes:?}");
}

#[test]
fn executes_enums_example() {
    assert_eq!(
        output(include_str!("../../examples/enums.oki")),
        "Estado::Pendiente\ntrue\nhecho\n4\ndivisor cero\nResultado::Ok(4)\n"
    );
}

#[test]
fn enums_are_nominal_values_with_assignment_comparison_and_printing() {
    assert_eq!(
        output(
            r#"
        enum E { A, B, }
        enum F { A }
        E e = E::A; E copia = e; e = E::B;
        println(copia); println(e); println(e == copia); println(e != copia);
        println([E::A, E::B]); println(F::A);
    "#
        ),
        "E::A\nE::B\nfalse\ntrue\n[E::A, E::B]\nF::A\n"
    );
    for expression in ["E::A == F::A", "E::A < E::B", "E::A + E::B", "!E::A"] {
        rejects(
            &format!("enum E {{ A, B }} enum F {{ A }} println({expression});"),
            "no admite",
        );
    }
    rejects("enum E { A } enum F { A } E e = F::A;", "Tipo incompatible");
}

#[test]
fn enums_require_prior_unique_global_names_and_valid_variants() {
    for (source, message) in [
        ("enum E {}", "al menos una variante"),
        ("enum E { A, A }", "variante está repetida"),
        ("enum E { A } enum E { B }", "ya está declarado"),
        ("struct E {} enum E { A }", "ya está declarado"),
        ("enum E { A } struct E {}", "ya está declarado"),
        ("int E = 1; enum E { A }", "ya está declarado"),
        ("function E() {} enum E { A }", "ya está declarado"),
        ("enum E { A } int E = 1;", "declarado como enum"),
        ("enum E { A } function E() {}", "declarado como enum"),
        ("if (true) { enum E { A } }", "ámbito global"),
        ("function f() { enum E { A } }", "ámbito global"),
        ("E e = E::A; enum E { A }", "no está declarado"),
        (
            "println(Desconocido::A);",
            "enum 'Desconocido' no está declarado",
        ),
        ("enum E { A } println(E::B);", "variante 'B' no existe"),
        ("enum E { A } println(A);", "no está declarada"),
        ("enum E { A } println(E::A());", "sin paréntesis"),
    ] {
        rejects(source, message);
    }
}

#[test]
fn match_selects_each_variant_and_evaluates_the_subject_once() {
    assert_eq!(
        output(
            r#"
        enum E { A, B }
        function elegir(inout int llamadas) -> E { llamadas++; return E::B; }
        int llamadas = 0;
        match elegir(inout llamadas) {
            E::A => { println(1 / 0); }, E::B => { println("B"); },
        }
        println(llamadas);
        foreach (E e in [E::A, E::B]) {
            match e { E::B => { println(2); }, E::A => { println(1); } }
        }
        match E::A { E::A => { println(3); }, E::B => { println(1 / 0); } }
    "#
        ),
        "B\n1\n1\n2\n3\n"
    );
}

#[test]
fn match_requires_exactly_one_branch_per_variant_of_the_subject_enum() {
    for (source, message) in [
        (
            "enum E { A, B } E e = E::A; match e { E::A => {} }",
            "faltan variantes de 'E': B",
        ),
        ("enum E { A } match E::A {}", "no exhaustivo"),
        (
            "enum E { A } match E::A { E::A => {}, E::A => {} }",
            "rama de esta variante está repetida",
        ),
        (
            "enum E { A } enum F { A } match E::A { F::A => {} }",
            "debe pertenecer al enum 'E'",
        ),
        ("enum E { A } match E::A { E::B => {} }", "no existe"),
        ("enum E { A } match 1 { E::A => {} }", "tipo enum concreto"),
        (
            "struct S {} enum E { A } match (S {}) { E::A => {} }",
            "tipo enum concreto",
        ),
        (
            "enum E { A } E || int dato = E::A; match dato { E::A => {} }",
            "tipo enum concreto",
        ),
        (
            "enum E { A, B } match E::A { E::A => {}, E::B => { println(desconocida); } }",
            "no está declarada",
        ),
    ] {
        rejects(source, message);
    }
}

#[test]
fn match_has_local_scopes_and_preserves_outer_changes() {
    assert_eq!(
        output(
            r#"
        enum E { A, B } int n = 1;
        match E::A { E::A => { n = 3; string local = "A"; println(local); },
                    E::B => { string local = "B"; println(local); } }
        println(n);
        match E::B { E::A => {}, E::B => { int n = 7; println(n); } }
        println(n);
    "#
        ),
        "A\n3\n7\n3\n"
    );
    rejects(
        "enum E { A } match E::A { E::A => { int local = 1; } } println(local);",
        "no está declarada",
    );
    rejects(
        "enum E { A } const int n = 1; match E::A { E::A => { n = 2; } }",
        "constante",
    );
}

#[test]
fn payloads_are_positional_typed_values_with_nested_printing_and_equality() {
    assert_eq!(
        output(
            r#"
        enum Color { Rojo }
        struct Punto { int x; }
        enum Dato { Vacio, Datos(int n, float f, bool b, char c, string s, int[][] a, Punto p, Color color) }
        Dato d = Dato::Datos(2, 1.5, true, 'ñ', "界", [[]], Punto { x: 3 }, Color::Rojo);
        println(d);
        println(d == Dato::Datos(2, 1.5, true, 'ñ', "界", [[]], Punto { x: 3 }, Color::Rojo));
        println(d != Dato::Vacio);
        match d { Dato::Vacio => {}, Dato::Datos(n, f, b, c, s, a, p, color) => {
            println(n + p.x); println(f); println(b); println(c); println(s); println(a); println(color);
        } }
    "#
        ),
        "Dato::Datos(2, 1.5, true, 'ñ', \"界\", [[]], Punto { x: 3 }, Color::Rojo)\ntrue\ntrue\n5\n1.5\ntrue\nñ\n界\n[[]]\nColor::Rojo\n"
    );
}

#[test]
fn payload_declarations_constructors_and_patterns_reject_wrong_types_and_arity() {
    for (source, message) in [
        ("enum E { A(Q dato) }", "no está declarado"),
        ("enum E { A(E dato) }", "no está declarado"),
        (
            "enum E { A(int n, int n) }",
            "nombre del dato está repetido",
        ),
        (
            "enum E { A(int n) } println(E::A);",
            "esperaba 1 argumentos",
        ),
        (
            "enum E { A(int n) } println(E::A());",
            "esperaba 1 argumentos",
        ),
        (
            "enum E { A(int n) } println(E::A(1, 2));",
            "esperaba 1 argumentos",
        ),
        (
            "enum E { A(int n) } println(E::A(1.0));",
            "Tipo incompatible",
        ),
        (
            "enum E { A(int[] n) } println(E::A([true]));",
            "incompatible",
        ),
        (
            "enum E { A(int n) } match E::A(1) { E::A => {} }",
            "requiere 1 capturas",
        ),
        ("enum E { A } match E::A { E::A() => {} }", "sin paréntesis"),
        (
            "enum E { A(int n) } match E::A(1) { E::A(x, y) => {} }",
            "requiere 1 capturas",
        ),
        (
            "enum E { A(int n, string s) } match E::A(1, \"a\") { E::A(x, x) => {} }",
            "captura está repetido",
        ),
        (
            "enum E { A(int n) } match E::A(1) { E::A(x) => { string s = x; } }",
            "Tipo incompatible",
        ),
        (
            "enum E { A(int n) } match E::A(1) { E::A(x) => { int x = 2; } }",
            "ya está declarada",
        ),
        (
            "enum E { A(int n) } E e = E::A(1); println(e.n);",
            "Solo las estructuras tienen campos",
        ),
    ] {
        rejects(source, message);
    }
}

#[test]
fn captures_and_parameters_are_independent_copies_and_inout_replaces_the_enum() {
    assert_eq!(
        output(
            r#"
        import std::Array; use std::Array;
        struct P { int n; }
        enum E { Datos(int[] numeros, P p), Vacio }
        const E fijo = E::Datos([1], P { n: 2 }); E copia = fijo;
        match copia { E::Vacio => {}, E::Datos(numeros, p) => {
            numeros.push(3); p.n++; println(numeros); println(p.n);
        } }
        println(fijo); println(copia);
        function reemplazar(inout E e) { e = E::Vacio; }
        reemplazar(inout copia); println(copia);
        E[] datos = [fijo]; datos.push(E::Vacio); println(Array::len(datos));
        println(Array::pop(datos)); println(datos[0] == fijo);
    "#
        ),
        "[1, 3]\n3\nE::Datos([1], P { n: 2 })\nE::Datos([1], P { n: 2 })\nE::Vacio\n2\nE::Vacio\ntrue\n"
    );
    rejects("enum E { A } const E e = E::A; e = E::A;", "constante");
    rejects(
        "enum E { A } E e = E::A; function f() { e = E::A; }",
        "variable global",
    );
}

#[test]
fn match_returns_breaks_and_continues_close_scopes_and_follow_loop_control() {
    assert_eq!(
        output(
            r#"
        enum E { A, B(int n) }
        function valor(E e) -> int {
            match e { E::A => { return 0; }, E::B(n) => { return n; } }
        }
        println(valor(E::A)); println(valor(E::B(7)));
        for (int i = 0; i < 4; i++) {
            match E::B(i) { E::A => { break; }, E::B(n) => {
                if (n == 1) { continue; } if (n == 3) { break; } println(n);
            } }
        }
        int n = 9; println(n);
    "#
        ),
        "0\n7\n0\n2\n9\n"
    );
    rejects(
        "enum E { A, B } function f(E e) -> int { match e { E::A => { return 1; }, E::B => {} } }",
        "todos los caminos",
    );
    rejects(
        "enum E { A } match E::A { E::A => { break; } }",
        "dentro de un bucle",
    );
    rejects(
        "enum E { A } match E::A { E::A => { return 1; } }",
        "dentro de una función",
    );
}

#[test]
fn enums_work_in_struct_fields_unions_defaults_and_typed_function_returns() {
    assert_eq!(
        output(
            r#"
        enum E { A, B(int n) }
        struct Caja { E valor = E::B(3); }
        Caja c = Caja {}; match c.valor { E::A => {}, E::B(n) => { println(n); } }
        function elegir(bool si) -> E || int { if (si) { return E::A; } return 4; }
        E || int dato = elegir(true);
        if (type dato == E) { match dato { E::A => { println(dato); }, E::B(n) => { println(n); } } }
        dato = 4; println(dato);
    "#
        ),
        "3\nE::A\n4\n"
    );
    rejects(
        "import std::Casting; enum E { A } println(E::A.cast(string));",
        "Conversión no admitida",
    );
}

#[test]
fn payload_arguments_evaluate_once_in_order_and_discard_stale_field_guards() {
    assert_eq!(
        output(
            r#"
        enum E { Datos(int a, int b) }
        function paso(inout int contador) -> int { contador++; return contador; }
        int contador = 0; E e = E::Datos(paso(inout contador), paso(inout contador));
        println(e); println(contador);
    "#
        ),
        "E::Datos(1, 2)\n2\n"
    );
    rejects(
        r#"
        struct S { int || string dato = 1; }
        enum E { Datos(int a, int b) }
        function cambiar(inout S s) -> int { s.dato = "x"; return 0; }
        S s = S {}; if (type s.dato == int) { println(E::Datos(cambiar(inout s), s.dato + 1)); }
    "#,
        "no admite",
    );
}

#[test]
fn match_merges_and_invalidates_union_guards_in_branches_subjects_and_loops() {
    rejects(
        r#"
        enum E { A, B } int || string dato = 1;
        if (type dato == int) {
            match E::A { E::A => { dato = "x"; }, E::B => {} }
            println(dato + 1);
        }
    "#,
        "no admite",
    );
    rejects(
        r#"
        enum E { A } int || string dato = 1;
        if (type dato == int) {
            for (int i = 0; i < 2; i++) {
                println(dato + 1); match E::A { E::A => { dato = "x"; } }
            }
        }
    "#,
        "no admite",
    );
    rejects(
        r#"
        struct S { int || string dato = 1; } enum E { A }
        function cambiar(inout S s) -> E { s.dato = "x"; return E::A; }
        S s = S {}; if (type s.dato == int) {
            match cambiar(inout s) { E::A => { println(s.dato + 1); } }
        }
    "#,
        "no admite",
    );
}

#[test]
fn enum_and_match_syntax_errors_happen_before_output() {
    for source in [
        "enum E { A B }",
        "enum E { A",
        "enum E { A() }",
        "enum E { A(int) }",
        "enum E { A(int || string dato) }",
        "enum E { A };",
        "enum E { A } match E::A { E::A -> {} }",
        "enum E { A } match E::A { A => {} }",
        "enum E { A } match E::A { _ => {} }",
        "enum E { A } match E::A { E::A => println(1); }",
        "enum E { A } match E::A { E::A => {}",
        "enum E { A } match E::A { E::A => {} };",
        "enum E { A } int n = match E::A { E::A => {} };",
        "enum E { A(int n) } match E::A(1) { E::A(1) => {} }",
    ] {
        rejects(source, "Línea");
    }
}

#[test]
fn runtime_errors_preserve_output_and_close_capture_scopes() {
    let tokens = Scanner::new(
        r#"
        enum E { A(int n) }
        match E::A(3) { E::A(captura) => { println(captura); println(1 / 0); } }
    "#,
    )
    .scan_tokens()
    .unwrap();
    let statements = Parser::new(tokens).parse().unwrap();
    TypeChecker::default().check(&statements).unwrap();
    let mut bytes = Vec::new();
    let mut interpreter = Interpreter::new(&mut bytes);
    let error = interpreter.interpret(&statements).unwrap_err().to_string();
    assert!(error.contains("Línea 3"), "{error}");
    let following = Parser::new(Scanner::new("println(captura);").scan_tokens().unwrap())
        .parse()
        .unwrap();
    assert!(
        interpreter
            .interpret(&following)
            .unwrap_err()
            .to_string()
            .contains("no está declarada")
    );
    drop(interpreter);
    assert_eq!(bytes, b"3\n");
}

#[test]
fn enum_containers_share_existing_value_depth_limits() {
    let mut bytes = Vec::new();
    let error = run(
        r#"
        struct Nodo { Nodo[] hijos = []; }
        enum E { Datos(Nodo n) }
        Nodo n = Nodo {}; for (int i = 0; i < 49; i++) { n = Nodo { hijos: [n] }; }
        println("antes"); E e = E::Datos(n);
        n = Nodo { hijos: [n] };
        e = E::Datos(n);
    "#,
        &mut bytes,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("profundidad máxima de valores"), "{error}");
    assert_eq!(bytes, b"antes\n");
}

#[test]
fn enum_definitions_do_not_leak_between_programs() {
    assert_eq!(output("enum E { A } println(E::A);"), "E::A\n");
    rejects("println(E::A);", "no está declarado");
}

#[test]
fn recursive_functions_with_match_keep_the_existing_call_limit() {
    assert_eq!(
        output(
            r#"
        enum E { Fin, Paso(int n) }
        function contar(E e) -> int {
            match e {
                E::Fin => { return 0; }, E::Paso(n) => {
                    if (n == 0) { return contar(E::Fin); }
                    return 1 + contar(E::Paso(n - 1));
                }
            }
        }
        println(contar(E::Paso(5)));
    "#
        ),
        "5\n"
    );
    let mut bytes = Vec::new();
    let error = run("enum E { A } function f(E e) -> int { match e { E::A => { return 1 + f(e); } } } println(f(E::A));", &mut bytes).unwrap_err().to_string();
    assert!(error.contains("profundidad máxima"), "{error}");
    assert!(bytes.is_empty());
}
