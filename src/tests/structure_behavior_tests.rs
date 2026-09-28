use super::*;

#[test]
fn executes_structures_example() {
    assert_eq!(
        output(include_str!("../../examples/estructuras.oki")),
        "Persona { nombre: \"Ana\", edad: 30 }\n31\nLuis\nAna\nAna\n"
    );
}

#[test]
fn structures_print_fields_in_declaration_order_and_compare_by_value() {
    assert_eq!(
        output(
            r#"
        struct Dato { int n; float f; bool b; char c; string s; }
        Dato a = Dato { s: "世界", c: 'ñ', b: true, f: 1.5, n: 2 };
        Dato b = Dato { n: 2, f: 1.5, b: true, c: 'ñ', s: "世界" };
        println(a); println(a == b); b.n++; println(a != b);
        println(a.n + 3 * 2); println(a.f); println(a.b);
        println(a.c); println(a.s);
        struct Marca {}
        Marca marca = Marca {}; println(marca); println(marca == Marca {});
        int structura = 4; println(structura);
    "#
        ),
        "Dato { n: 2, f: 1.5, b: true, c: 'ñ', s: \"世界\" }\ntrue\ntrue\n8\n1.5\ntrue\nñ\n世界\nMarca { }\ntrue\n4\n"
    );
}

#[test]
fn nested_structure_and_array_copies_are_independent() {
    assert_eq!(
        output(
            r#"
        struct Punto { int x; }
        struct Caja { Punto punto; Punto[] puntos; int[][] tabla; }
        const Caja original = Caja { punto: Punto { x: 1 }, puntos: [Punto { x: 2 }], tabla: [[], [3]] };
        Caja copia = original;
        copia.punto.x += 4; copia.puntos[0].x--; copia.tabla[1][0] = 8;
        Punto extraido = copia.punto; extraido.x = 99;
        println(original); println(copia); println(extraido.x);
        Caja[] cajas = [copia]; Caja[] otras = cajas;
        otras[0].puntos[0].x = 7;
        println(cajas[0].puntos[0].x); println(otras[0].puntos[0].x);
        copia.punto = Punto { x: 6 }; println(copia.punto.x);
    "#
        ),
        "Caja { punto: Punto { x: 1 }, puntos: [Punto { x: 2 }], tabla: [[], [3]] }\nCaja { punto: Punto { x: 5 }, puntos: [Punto { x: 1 }], tabla: [[], [8]] }\n99\n1\n7\n6\n"
    );
}

#[test]
fn structure_fields_work_with_array_library_and_foreach_copies() {
    assert_eq!(
        output(
            r#"
        import std::Array; use std::Array;
        struct Caja { int[] datos; }
        Caja[] cajas = [Caja { datos: [] }];
        cajas[0].datos.push(1); std::Array::push(cajas[0].datos, 2);
        println(cajas[0].datos.len()); println(Array::pop(cajas[0].datos));
        foreach (Caja caja in cajas) { caja.datos[0] = 99; println(caja.datos); }
        println(cajas[0].datos); println(Caja { datos: [4, 5] }.datos.len());
        Caja[] vacias = []; vacias.push(Caja { datos: [] });
        println(vacias.pop().datos);
    "#
        ),
        "2\n2\n[99]\n[1]\n2\n[]\n"
    );
}

#[test]
fn structures_support_functions_returns_inout_and_lexical_scopes() {
    assert_eq!(
        output(
            r#"
        struct Punto { int x; }
        Punto global = Punto { x: 3 };
        function copiar(Punto p) -> Punto { p.x += global.x; return p; }
        function modificar(inout Punto p) { p.x++; }
        function reenviar(inout Punto p) { modificar(inout p); }
        function dos(inout Punto a, inout Punto b) { a.x++; b.x += a.x; }
        Punto copia = copiar(global); println(copia.x); println(global.x);
        reenviar(inout global); dos(inout global, inout global); println(global.x);
        println(copiar(Punto { x: 1 }).x);
        if (true) { Punto global = Punto { x: 20 }; modificar(inout global); println(global.x); }
        println(global.x);
        for (Punto contador = Punto { x: 0 }; contador.x < 2; contador.x++) { print(contador.x); }
        println("");
    "#
        ),
        "6\n3\n10\n11\n21\n10\n01\n"
    );
}

#[test]
fn structures_can_be_union_alternatives_and_use_type_guards() {
    assert_eq!(
        output(
            r#"
        struct Punto { int x; }
        function elegir(bool punto) -> Punto || string {
            if (punto) { return Punto { x: 4 }; } else { return "otro"; }
        }
        Punto || string dato = elegir(true);
        if (type dato == Punto) { dato.x++; println(dato.x); } else { println(dato + "!"); }
        dato = elegir(false); println(type dato != Punto);
        Punto[] || string vacios = []; println(type vacios == Punto[]);
    "#
        ),
        "5\ntrue\ntrue\n"
    );
    rejects_without_output(
        "struct P { int x; } P || string p = P { x: 1 }; println(p.x);",
        "Solo las estructuras tienen campos",
    );
}

#[test]
fn structures_require_unique_global_prior_declarations_and_known_field_types() {
    for (source, error) in [
        (
            "struct P {} struct P {}",
            "La estructura 'P' ya está declarada",
        ),
        ("if (true) { struct P {} }", "ámbito global"),
        ("function f() { struct P {} }", "ámbito global"),
        (
            "struct P { int x; string x; }",
            "El campo 'x' está repetido",
        ),
        ("P p = P {}; struct P {}", "no está declarado"),
        (
            "println(P {}); struct P {}",
            "La estructura 'P' no está declarada",
        ),
        (
            "struct P { Q q; } struct Q {}",
            "tipo 'Q' no está declarado",
        ),
        ("struct P { P p; }", "no permite un valor finito"),
        ("function f(Q q) {}", "tipo 'Q' no está declarado"),
        (
            "function f() -> Q { return 1; }",
            "tipo 'Q' no está declarado",
        ),
        (
            "int x = 1; println(type x == Q);",
            "tipo 'Q' no está declarado",
        ),
        ("foreach (Q q in [1]) {}", "tipo 'Q' no está declarado"),
        ("struct P {} int P = 1;", "declarado como estructura"),
        ("int P = 1; struct P {}", "variable o función global"),
        ("struct P {} function P() {}", "declarado como estructura"),
        ("function P() {} struct P {}", "variable o función global"),
    ] {
        rejects_without_output(&format!("println(\"previo\"); {source}"), error);
    }
    // Cada archivo crea un registro de tipos nuevo.
    assert_eq!(output("struct P {} println(P {});"), "P { }\n");
    rejects_without_output("println(P {});", "no está declarada");
}

#[test]
fn structures_reject_missing_extra_duplicate_and_incompatible_fields() {
    let prefix = "struct P { int x; string nombre; } println(\"previo\"); ";
    for (source, error) in [
        ("P p = P { x: 1 };", "Falta inicializar el campo 'nombre'"),
        (
            "P p = P { x: 1, nombre: \"a\", otro: 2 };",
            "El campo 'otro' no existe",
        ),
        (
            "P p = P { x: 1, nombre: \"a\", x: 2 };",
            "El campo 'x' está repetido",
        ),
        ("P p = P { x: 1.0, nombre: \"a\" };", "se esperaba int"),
        (
            "P p = P { x: 1, nombre: \"a\" }; p.x = \"b\";",
            "se esperaba int",
        ),
        (
            "P p = P { x: 1, nombre: \"a\" }; println(p.otro);",
            "El campo 'otro' no existe",
        ),
        (
            "P p = P { x: 1, nombre: \"a\" }; p.otro = 1;",
            "El campo 'otro' no existe",
        ),
        (
            "int n = 1; println(n.x);",
            "Solo las estructuras tienen campos",
        ),
        ("int n = 1; n.x = 2;", "Solo las estructuras tienen campos"),
        (
            "P p = P { x: 1, nombre: \"a\" }; p.x += 1.0;",
            "no admite int y float",
        ),
        (
            "P p = P { x: 1, nombre: \"a\" }; p.nombre++;",
            "no admite string",
        ),
    ] {
        rejects_without_output(&format!("{prefix}{source}"), error);
    }
}

#[test]
fn structures_are_nominal_and_have_no_whole_value_operators_or_casts() {
    let prefix = "struct P { int x; } struct Q { int x; } P p = P { x: 1 }; Q q = Q { x: 1 }; ";
    for (source, error) in [
        ("p = q;", "se esperaba P"),
        ("println(p == q);", "no admite P y Q"),
        ("println(p + p);", "no admite P y P"),
        ("println(p < p);", "no admite P y P"),
        ("P[] datos = [p, q];", "elemento de array incompatible"),
        ("function f(P v) {} f(q);", "Argumento incompatible"),
        ("function f() -> P { return q; }", "el retorno debe ser P"),
        (
            "import std::Casting; println(string(p));",
            "Conversión no admitida",
        ),
        (
            "import std::Casting; println(p.cast(P));",
            "Conversión no admitida",
        ),
    ] {
        rejects_without_output(&format!("{prefix}{source}"), error);
    }
}

#[test]
fn structures_protect_deep_constants_and_require_inout_to_write_globals() {
    for constant in [false, true] {
        let prefix = format!(
            "import std::Array; struct P {{ int x; int[] datos; }} {}P p = P {{ x: 1, datos: [2] }}; ",
            if constant { "const " } else { "" }
        );
        for change in [
            "p.x = 2;",
            "p.x++;",
            "p.x += 2;",
            "p.datos[0] = 3;",
            "p.datos.push(3);",
            "p.datos.pop();",
        ] {
            let source = if constant {
                format!("{prefix}{change}")
            } else {
                format!("{prefix}function f() {{ {change} }}")
            };
            rejects_without_output(
                &source,
                if constant {
                    "constante 'p'"
                } else {
                    "mediante un parámetro inout"
                },
            );
        }
    }
    rejects_without_output(
        "struct P {} function f(inout P p) {} const P p = P {}; f(inout p);",
        "constante 'p'",
    );
    rejects_without_output(
        "struct P { int x; } function f(inout int x) {} P p = P { x: 1 }; f(inout p.x);",
        "variable completa",
    );
    rejects_without_output(
        "import std::Array; struct P { int[] datos; } P { datos: [] }.datos.push(1);",
        "variable array modificable",
    );
    rejects_without_output(
        "struct P { int[] datos; } const P[] ps = [P { datos: [1] }]; ps[0].datos[0]++;",
        "constante 'ps'",
    );
}

#[test]
fn structures_evaluate_fields_and_target_indices_once_in_source_order() {
    assert_eq!(
        output(
            r#"
        import std::Array;
        struct Par { int a; int b; }
        function siguiente(inout int n) -> int { n++; return n; }
        int n = 0;
        Par p = Par { b: siguiente(inout n), a: siguiente(inout n) };
        println(p); println(n);
        struct Caja { Par[] pares; }
        Caja[] cajas = [Caja { pares: [p] }]; int[] indices = [3, 0, 0];
        cajas[indices.pop()].pares[indices.pop()].a += indices.pop();
        println(cajas[0].pares[0].a); println(indices);
        println(true || Par { a: siguiente(inout n), b: 0 }.a == 0);
        println(n);
    "#
        ),
        "Par { a: 2, b: 1 }\n2\n5\n[]\ntrue\n2\n"
    );
}

#[test]
fn structures_report_syntax_and_field_lines_before_any_output() {
    for source in [
        "struct P { int x }",
        "struct P { int x;",
        "struct P { const const int x; }",
        "struct P { int || x; }",
        "struct P {} ;",
        "struct P { int x; } P p = P { x 1 };",
        "struct P { int x; } P p = P { x: 1, };",
        "struct P { int x; } P p = P { x: 1; };",
        "struct P { int x; int y; } P p = P { x: 1 y: 2 };",
        "struct P {} P p = P {;",
        "struct P {} P p = P {}",
        "struct P {} P p = P {}; println(p.);",
    ] {
        rejects_without_output(&format!("println(\"previo\"); {source}"), "Línea");
    }
    rejects_without_output(
        "struct P { int x; }\nP p = P { x: 1 };\nprintln(p.\notro);",
        "Línea 4: El campo 'otro' no existe",
    );
    rejects_without_output("struct P { int x; }\nP p = P {\nx: \"a\"\n};", "Línea 3:");
}

#[test]
fn structures_runtime_errors_keep_previous_output_and_completed_effects() {
    let mut bytes = Vec::new();
    let error = run(
        r#"import std::Array;
struct P { int[] datos; }
function quitar(inout P[] valores) -> int {
P ultimo = valores.pop(); println(valores.len()); return ultimo.datos[0];
}
P[] ps = [P { datos: [1] }];
println("previo");
ps[0].datos.push(quitar(inout ps));"#,
        &mut bytes,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("Línea 8: El destino quedó fuera de rango"),
        "{error}"
    );
    assert_eq!(bytes, b"previo\n0\n");
    let mut bytes = Vec::new();
    let error = run(
        "struct P { int x; }\nprintln(\"previo\");\nP p = P { x: 1 / 0 };",
        &mut bytes,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("Línea 3: división o resto por cero"),
        "{error}"
    );
    assert_eq!(bytes, b"previo\n");
    rejects_without_output(
        "struct P { int[] datos; } P p = P { datos: [] }; println(p.datos[true]);",
        "índice debe ser int",
    );
    let mut bytes = Vec::new();
    let error = run("struct P { int[] datos; } P p = P { datos: [] };\nprintln(\"previo\");\nprintln(p.datos[0]);", &mut bytes).unwrap_err().to_string();
    assert!(
        error.contains("Línea 3: índice 0 fuera de rango"),
        "{error}"
    );
    assert_eq!(bytes, b"previo\n");
}
