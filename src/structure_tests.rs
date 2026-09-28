use crate::run;

fn output(source: &str) -> String {
    let mut bytes = Vec::new();
    run(source, &mut bytes).unwrap();
    String::from_utf8(bytes).unwrap()
}

fn rejects(source: &str, expected: &str) {
    let mut bytes = Vec::new();
    let error = run(source, &mut bytes).unwrap_err().to_string();
    assert!(error.contains(expected), "{source}: {error}");
    assert!(bytes.is_empty(), "{source}: {bytes:?}");
}

#[test]
fn executes_extended_structure_example() {
    assert_eq!(
        output(include_str!("../examples/estructuras_campos.oki")),
        "6\n5\nlisto!\nNodo { valor: 1, hijos: [Nodo { valor: 2, hijos: [] }] }\nNodo { valor: 1, hijos: [Nodo { valor: 9, hijos: [] }] }\n20\n"
    );
}

#[test]
fn constant_fields_protect_their_entire_value_but_allow_new_instances() {
    assert_eq!(
        output(
            r#"
        struct P { const int id = 1; int edad = 20; }
        P p = P {}; p.edad++; println(p);
        P copia = p; copia.edad = 30; println(p.edad); println(copia.edad);
        p = P { id: 2 }; println(p.id);
        function reemplazar(inout P destino) { destino = P { id: 3 }; }
        reemplazar(inout p); println(p.id);
        struct Caja { const int[] datos = [1]; }
        Caja caja = Caja {}; int[] datos = caja.datos; datos[0] = 9;
        println(caja.datos); println(datos);
    "#
        ),
        "P { id: 1, edad: 21 }\n21\n30\n2\n3\n[1]\n[9]\n"
    );
    let prefix = "import std::Array; struct P { int edad; } struct Caja { const P p; const P[] ps; const int[] datos; } Caja c = Caja { p: P { edad: 1 }, ps: [P { edad: 2 }], datos: [3] }; ";
    for change in [
        "c.p = P { edad: 3 };",
        "c.p.edad++;",
        "c.p.edad += 1;",
        "c.ps[0].edad = 4;",
        "c.datos[0]++;",
        "c.datos.push(4);",
        "std::Array::pop(c.datos);",
    ] {
        rejects(
            &format!("{prefix}println(\"previo\"); {change}"),
            "campo constante",
        );
    }
    rejects(
        "struct P { const int id; } function f(inout P p) { p.id++; }",
        "campo constante 'id'",
    );
    rejects(
        "struct P { const int id; } P p = P {};",
        "Falta inicializar",
    );
}

#[test]
fn defaults_are_per_instance_ordered_and_can_use_previous_supplied_fields() {
    assert_eq!(
        output(
            r#"
        function marcar(int n) -> int { println(n); return n; }
        struct P {
            int a;
            const int doble = marcar(a * 2);
            int otro = marcar(doble + 1);
            int[] datos = [];
        }
        P p = P { otro: marcar(8), a: marcar(3) };
        P q = P { a: 4 };
        println(p); println(q);
        import std::Array; p.datos.push(1); println(q.datos);
        P r = P { a: 5, doble: 100, otro: 101 }; println(r.doble);
    "#
        ),
        "8\n3\n6\n8\n9\nP { a: 3, doble: 6, otro: 8, datos: [] }\nP { a: 4, doble: 8, otro: 9, datos: [] }\n[]\n100\n"
    );
}

#[test]
fn defaults_have_lexical_names_and_do_not_inherit_caller_locals_or_guards() {
    assert_eq!(
        output(
            r#"
        int base = 2;
        struct P { int valor = base; }
        base = 3;
        function crear(int base) -> P { return P {}; }
        println(crear(99).valor);
        if (true) { int base = 88; println(P {}.valor); }
        struct Q { int base; int copia = base; }
        println(Q { base: 4 }.copia);
        struct Prueba { int || string dato = 1; bool numero = type dato == int && dato > 0; }
        println(Prueba {}.numero); println(Prueba { dato: "texto" }.numero);
    "#
        ),
        "3\n3\n4\ntrue\nfalse\n"
    );
    for (source, error) in [
        (
            "int siguiente = 3; struct P { int anterior = siguiente; int siguiente = 2; }",
            "no está declarada",
        ),
        ("struct P { int x = x; }", "no está declarada"),
        (
            "struct P { int x = despues(); } function despues() -> int { return 1; }",
            "Llamada desconocida",
        ),
        ("struct P { float x = 1; }", "se esperaba float"),
        (
            "struct P { int[] datos = [1.0]; }",
            "elemento de array incompatible",
        ),
        ("struct P { int[] || string[] datos = []; }", "ambiguo"),
        ("struct P { int x = ; }", "Línea"),
        ("struct P { const const int x; }", "Se esperaba un tipo"),
    ] {
        rejects(source, error);
    }
}

#[test]
fn defaults_cannot_modify_previous_fields_or_globals_even_through_calls() {
    for source in [
        "function f(inout int n) -> int { n++; return n; } struct P { int a = 1; int b = f(inout a); }",
        "function f(inout int n) -> int { n++; return n; } int global = 1; struct P { int b = f(inout global); }",
        "import std::Array; struct P { int[] datos = [1]; int ultimo = datos.pop(); }",
    ] {
        rejects(source, "constante");
    }
    rejects(
        "struct P { int dato; } P global = P { dato: 1 }; function f(inout P p) -> int { p.dato++; return p.dato; } struct Q { int n = f(inout global); }",
        "constante",
    );
}

#[test]
fn union_fields_refine_in_nested_paths_branches_short_circuit_and_loops() {
    assert_eq!(
        output(
            r#"
        struct Interior { int || string dato = 1; }
        struct P { Interior interior = Interior {}; int[] || string lista = []; }
        P p = P {};
        if (type p.interior.dato == int && p.interior.dato > 0) {
            p.interior.dato += 2; println(p.interior.dato);
        } else { println(p.interior.dato); }
        p.interior.dato = "hola";
        if (type p.interior.dato != int) { println(p.interior.dato + "!"); }
        import std::Array;
        if (!(type p.lista != int[])) { p.lista.push(4); println(p.lista.pop()); }
        if (type p.interior.dato == int || p.interior.dato == "hola") { println(true); }
        p.interior.dato = 0;
        while (type p.interior.dato == int && p.interior.dato < 2) { p.interior.dato++; }
        if (type p.interior.dato == int) { println(p.interior.dato); }
        for (int i = 0; type p.interior.dato == int && p.interior.dato < 4; i++) { p.interior.dato++; }
        if (type p.interior.dato == int) { println(p.interior.dato); }
        function doble(int n) -> int { return n * 2; }
        if (type p.interior.dato == int) { println(doble(p.interior.dato)); }
    "#
        ),
        "3\nhola!\n4\ntrue\n2\n4\n8\n"
    );
}

#[test]
fn union_field_replacements_and_shadowing_discard_only_validated_facts() {
    assert_eq!(
        output(
            r#"
        struct P { int || string dato = 1; int || string otro = 2; }
        P p = P {};
        if (type p.dato == int && type p.otro == int) {
            p.dato = "cambio"; println(p.otro + 1);
            if (type p.dato == string) { println(p.dato + "!"); }
        }
        p.dato = 5;
        if (type p.dato == int) {
            if (true) { P p = P { dato: "interior" }; println(p.dato); }
            println(p.dato + 1);
        }
        const P fijo = P {};
        if (type fijo.dato == int) { println(fijo.dato + 1); }
    "#
        ),
        "3\ncambio!\ninterior\n6\n2\n"
    );
    let prefix = "struct P { int || string dato = 1; } P p = P {}; ";
    for source in [
        "if (type p.dato == int) { p.dato = \"x\"; println(p.dato + 1); }",
        "if (type p.dato == int) { p = P { dato: \"x\" }; println(p.dato + 1); }",
        "if (type p.dato == int) { if (true) { p.dato = \"x\"; } println(p.dato + 1); }",
        "if (type p.dato == int) { while (false) { p.dato = \"x\"; } println(p.dato + 1); }",
        "p.dato = 2; println(p.dato + 1);",
        "bool entero = type p.dato == int; if (entero) { println(p.dato + 1); }",
    ] {
        rejects(&format!("{prefix}{source}"), "no admite");
    }
    rejects(
        "struct I { int || string dato = 1; } struct P { I i = I {}; } P p = P {}; if (type p.i.dato == int) { p.i = I { dato: \"x\" }; println(p.i.dato + 1); }",
        "no admite",
    );
}

#[test]
fn union_field_guards_are_invalidated_by_inout_calls_in_every_expression_order() {
    let prefix = "struct P { int || string dato = 1; } function cambiar(inout P p) -> int { p.dato = \"x\"; return 0; } P p = P {}; ";
    assert_eq!(
        output(&format!(
            "{prefix}if (type p.dato == int) {{ println(p.dato + cambiar(inout p)); }} println(p.dato);"
        )),
        "1\nx\n"
    );
    for source in [
        "if (type p.dato == int) { cambiar(inout p); println(p.dato + 1); }",
        "if (type p.dato == int) { println(cambiar(inout p) + p.dato); }",
        "if (type p.dato == int && cambiar(inout p) == 0 && p.dato > 0) {}",
        "if (type p.dato == int && cambiar(inout p) == 0) { println(p.dato + 1); }",
    ] {
        rejects(&format!("{prefix}{source}"), "no admite");
    }
    rejects(
        &format!("{prefix}if (type p.dato == int) {{ int[] datos = [cambiar(inout p), p.dato]; }}"),
        "elemento de array incompatible",
    );
    rejects(
        &format!(
            "{prefix}function sumar(int a, int b) -> int {{ return a + b; }} if (type p.dato == int) {{ println(sumar(cambiar(inout p), p.dato)); }}"
        ),
        "Argumento incompatible",
    );
    rejects(
        &format!(
            "{prefix}struct Par {{ int a; int b; }} if (type p.dato == int) {{ Par par = Par {{ a: cambiar(inout p), b: p.dato }}; }}"
        ),
        "se esperaba int",
    );
    rejects(
        "struct P { int || string dato = 1; } function indices(inout P p) -> int[] { p.dato = \"x\"; return [1]; } P p = P {}; if (type p.dato == int) { println(indices(inout p)[p.dato]); }",
        "índice debe ser int",
    );
}

#[test]
fn aliased_inout_parameters_cannot_keep_stale_union_field_guards() {
    for source in [
        "struct P { int || string dato = 1; } function f(inout P a, inout P b) { if (type a.dato == int) { b.dato = \"x\"; println(a.dato + 1); } }",
        "struct P { int || string dato = 1; } P global = P {}; function f(inout P p) { if (type global.dato == int) { p.dato = \"x\"; println(global.dato + 1); } }",
    ] {
        rejects(source, "no admite");
    }
    assert_eq!(
        output(
            r#"
        struct P { int || string dato = 1; }
        function f(inout P a, inout P b) {
            b.dato = "x";
            if (type a.dato == int) { println(a.dato + 1); }
            else { println(a.dato + "!"); }
        }
        P p = P {}; f(inout p, inout p);
    "#
        ),
        "x!\n"
    );
}

#[test]
fn recursive_trees_lists_copies_functions_and_array_operations_work() {
    assert_eq!(
        output(
            r#"
        import std::Array;
        struct Nodo { const int id; Nodo[] hijos = []; }
        function contar(Nodo n) -> int {
            int total = 1;
            foreach (Nodo hijo in n.hijos) { total += contar(hijo); }
            return total;
        }
        Nodo raiz = Nodo { id: 1, hijos: [Nodo { id: 2 }, Nodo { id: 3 }] };
        Nodo copia = raiz; copia.hijos.pop(); copia.hijos.push(Nodo { id: 4 });
        println(contar(raiz)); println(copia == raiz);
        struct Enlace { int n; Enlace || bool siguiente = false; }
        function sumar(Enlace enlace) -> int {
            if (type enlace.siguiente == Enlace) { return enlace.n + sumar(enlace.siguiente); }
            return enlace.n;
        }
        Enlace lista = Enlace { n: 1, siguiente: Enlace { n: 2 } };
        println(sumar(lista)); Enlace otra = lista;
        if (type otra.siguiente == Enlace) { otra.siguiente.n = 9; }
        println(sumar(lista)); println(sumar(otra));
        struct Tabla { Tabla[][] hijos = []; } println(Tabla {});
    "#
        ),
        "3\nfalse\n3\n3\n10\nTabla { hijos: [] }\n"
    );
    rejects(
        "struct Nodo { Nodo siguiente; }",
        "no permite un valor finito",
    );
    rejects(
        "struct Nodo { Otro[] hijos; } struct Otro {}",
        "no está declarado",
    );
    rejects(
        "struct Nodo { Nodo[] hijos = []; } Nodo n = Nodo { hijos: [1] };",
        "elemento de array incompatible",
    );
    rejects(
        "struct Nodo { const int id = 1; Nodo[] hijos = []; } Nodo n = Nodo { hijos: [Nodo {}] }; n.hijos[0].id++;",
        "campo constante",
    );
    rejects(
        "struct P { int || string dato = 1; } P[] ps = [P {}]; println(type ps[0].dato == int);",
        "Se esperaba",
    );
}

#[test]
fn default_errors_keep_output_close_scopes_and_limit_recursive_construction() {
    for (source, expected, printed) in [
        (
            "struct P { int x = 1 / 0; } println(\"previo\"); P p = P {};",
            "división o resto por cero",
            "previo\n",
        ),
        (
            "function marcar(int n) -> int { println(n); return n; } struct P { int a = marcar(7); int b = 1 / 0; } P p = P {};",
            "división o resto por cero",
            "7\n",
        ),
        (
            "struct Nodo { Nodo[] hijos = [Nodo {}]; } println(\"previo\"); Nodo n = Nodo {};",
            "profundidad máxima de construcciones",
            "previo\n",
        ),
    ] {
        let mut bytes = Vec::new();
        let error = run(source, &mut bytes).unwrap_err().to_string();
        assert!(error.contains(expected), "{error}");
        assert_eq!(String::from_utf8(bytes).unwrap(), printed);
    }
    let source = "int base = 3; struct P { int base = 4; int fallo = 1 / 0; } P p = P {}; println(base); P q = P { fallo: 2 }; println(q.base);";
    let tokens = crate::Scanner::new(source).scan_tokens().unwrap();
    let statements = crate::Parser::new(tokens).parse().unwrap();
    crate::TypeChecker::default().check(&statements).unwrap();
    let mut bytes = Vec::new();
    let mut interpreter = crate::Interpreter::new(&mut bytes);
    assert!(interpreter.interpret(&statements[..3]).is_err());
    interpreter.interpret(&statements[3..]).unwrap();
    assert_eq!(String::from_utf8(bytes).unwrap(), "3\n4\n");
    // Un campo explícito evita evaluar el valor por defecto que fallaría.
    assert_eq!(
        output("struct P { int x = 1 / 0; } println(P { x: 3 }.x);"),
        "3\n"
    );
    assert_eq!(
        output("struct Nodo { Nodo[] hijos = [Nodo {}]; } println(Nodo { hijos: [] });"),
        "Nodo { hijos: [] }\n"
    );
}

#[test]
fn saved_targets_recheck_nominal_and_array_types_after_rhs_effects() {
    for source in [
        r#"struct A { int x; } struct B { const string x; }
        struct P { A || B dato; }
        function cambiar(inout P p) -> int { p.dato = B { x: "nuevo" }; return 3; }
        P p = P { dato: A { x: 1 } };
        if (type p.dato == A) { p.dato.x = cambiar(inout p); }"#,
        r#"struct P { int[] || string[] datos; }
        function cambiar(inout P p) -> int { p.datos = ["nuevo"]; return 3; }
        P p = P { datos: [1] };
        if (type p.datos == int[]) { p.datos[0] = cambiar(inout p); }"#,
        r#"import std::Array; struct P { int[] || string[] datos; }
        function cambiar(inout P p) -> int { p.datos = ["nuevo"]; return 3; }
        P p = P { datos: [1] };
        if (type p.datos == int[]) { p.datos.push(cambiar(inout p)); }"#,
    ] {
        let mut bytes = Vec::new();
        let error = run(source, &mut bytes).unwrap_err().to_string();
        assert!(error.contains("El destino cambió de tipo"), "{error}");
        assert!(bytes.is_empty());
    }
    // La sustitución de un campo unión completo admite cambiar su alternativa.
    assert_eq!(
        output(
            r#"
        struct P { int || string dato = 1; }
        function cambiar(inout P p) -> int { p.dato = "nuevo"; return 3; }
        P p = P {};
        if (type p.dato == int) { p.dato += cambiar(inout p); }
        println(p.dato);
    "#
        ),
        "4\n"
    );
}

#[test]
fn target_indices_foreach_and_shadowed_initializers_invalidate_field_guards() {
    rejects(
        r#"
        struct P { int || string dato = 1; }
        function cambiar(inout P p) -> int { p.dato = "x"; return 0; }
        P p = P {}; int[] datos = [1];
        if (type p.dato == int) { datos[cambiar(inout p)] = p.dato; }
    "#,
        "se esperaba int",
    );
    rejects(
        r#"
        struct P { int || string dato = 1; }
        function cambiar(inout P p) -> int { p.dato = "x"; return 0; }
        P p = P {}; int[] datos = [1];
        if (type p.dato == int) { datos[cambiar(inout p)] += p.dato; }
    "#,
        "no admite",
    );
    rejects(
        r#"
        struct P { int || string dato = 1; }
        function cambiar(inout P p) -> int[] { p.dato = "x"; return [0]; }
        P p = P {};
        if (type p.dato == int) { foreach (int n in cambiar(inout p)) { println(p.dato + n); } }
    "#,
        "no admite",
    );
    rejects(
        r#"
        struct P { int || string dato = 1; }
        function cambiar(inout P p) -> P { p.dato = "x"; return p; }
        P p = P {};
        if (type p.dato == int) {
            if (true) { P p = cambiar(inout p); }
            println(p.dato + 1);
        }
    "#,
        "no admite",
    );
}

#[test]
fn recursive_values_limit_growth_even_without_recursive_function_calls() {
    for change in [
        "n = Nodo { hijos: [n] };",
        "n.hijos = [n];",
        "Nodo copia = n; n.hijos = []; n.hijos.push(copia);",
    ] {
        let source = format!(
            "import std::Array; struct Nodo {{ Nodo[] hijos = []; }} Nodo n = Nodo {{}}; println(\"previo\"); for (int i = 0; i < 101; i++) {{ {change} }}"
        );
        let mut bytes = Vec::new();
        let error = run(&source, &mut bytes).unwrap_err().to_string();
        assert!(
            error.contains("profundidad máxima de valores (100)"),
            "{error}"
        );
        assert_eq!(bytes, b"previo\n");
    }
}
