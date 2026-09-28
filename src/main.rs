use std::{env, error::Error, fs, io, io::Write, process::ExitCode};

mod interpreter;
mod parser;
mod scanner;
mod stdlib;
mod type_checker;
mod value;

#[cfg(test)]
mod structure_tests;

#[cfg(test)]
mod enum_tests;

use interpreter::Interpreter;
use parser::Parser;
use scanner::Scanner;
use type_checker::TypeChecker;

fn run(source: &str, output: impl Write) -> Result<(), Box<dyn Error>> {
    let tokens = Scanner::new(source).scan_tokens()?;
    let statements = Parser::new(tokens).parse()?;
    TypeChecker::default().check(&statements)?;
    Interpreter::new(output).interpret(&statements)?;
    Ok(())
}

fn run_file() -> Result<(), Box<dyn Error>> {
    let path = env::args_os()
        .nth(1)
        .ok_or("Uso: cargo run -- archivo.oki")?;
    let source = fs::read_to_string(path)?;
    run(&source, io::stdout().lock())
}

fn main() -> ExitCode {
    match run_file() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(source: &str) -> String {
        let mut bytes = Vec::new();
        run(source, &mut bytes).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn executes_hello_file() {
        assert_eq!(
            output(include_str!("../examples/hello.oki")),
            "Hello World\n"
        );
    }

    #[test]
    fn executes_structures_example() {
        assert_eq!(
            output(include_str!("../examples/estructuras.oki")),
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

    #[test]
    fn union_returns_execute_both_branches_and_preserve_logical_or() {
        assert_eq!(
            output(include_str!("../examples/retornos_union.oki")),
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
            output(include_str!("../examples/variables_union.oki")),
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

    #[test]
    fn executes_casting_example() {
        assert_eq!(
            output(include_str!("../examples/conversiones.oki")),
            "25.0\nEdad: 25\n19\n19.95\n50\ntrue\nñ\n241\n25.0\n25\n"
        );
    }

    #[test]
    fn casting_forms_return_the_same_values_for_all_supported_pairs() {
        for (source, target, expected) in [
            ("25", "int", "25"),
            ("1.0", "float", "1.0"),
            ("true", "bool", "true"),
            ("'ñ'", "char", "ñ"),
            (r#""hola""#, "string", "hola"),
            ("25", "float", "25.0"),
            ("2.9", "int", "2"),
            ("-2.9", "int", "-2"),
            ("-0.9", "int", "0"),
            ("25", "string", "25"),
            ("1.0", "string", "1.0"),
            ("false", "string", "false"),
            ("'🦀'", "string", "🦀"),
            (r#""+42""#, "int", "42"),
            (r#""-42""#, "int", "-42"),
            (r#""2.5e1""#, "float", "25.0"),
            (r#"".5""#, "float", "0.5"),
            (r#""1.""#, "float", "1.0"),
            (r#""false""#, "bool", "false"),
            (r#""true""#, "bool", "true"),
            (r#""🦀""#, "char", "🦀"),
            ("'🦀'", "int", "129408"),
            ("129408", "char", "🦀"),
        ] {
            let calls = [
                format!("{target}({source})"),
                format!("({source}).cast({target})"),
                format!("std::Casting::{target}({source})"),
                format!("Casting::{target}({source})"),
            ];
            for call in calls {
                assert_eq!(
                    output(&format!(
                        "import std::Casting; use std::Casting; {target} resultado = {call}; println(resultado);"
                    )),
                    format!("{expected}\n"),
                    "{call}"
                );
            }
        }
    }

    #[test]
    fn casts_compose_without_changing_sources_or_evaluating_twice() {
        assert_eq!(
            output(
                r#"
            import std::Casting; import std::Array;
            const int edad = 25;
            println("Edad: " + string(edad));
            println(25.cast(float) + 0.5);
            println((edad + 1).cast(float).cast(string));
            int[] a = [1, 2, 3];
            println(float(a.pop()) + a.pop().cast(float));
            println(a);
            float[] b = [float(edad), a[0].cast(float)];
            b.push(float(2)); println(b);
            println(false && bool("inválido"));
            println(true || int("inválido") == 0);
            if (false) { println(char("ab")); }
            float(edad); edad.cast(string); 2.cast(string); "x".cast(char);
            println(edad);
        "#
            ),
            "Edad: 25\n25.5\n26.0\n5.0\n[1]\n[25.0, 1.0, 2.0]\nfalse\ntrue\n25\n"
        );
    }

    #[test]
    fn casting_checks_import_order_names_and_independent_libraries() {
        for call in [
            "float(2)",
            "2.cast(float)",
            "std::Casting::float(2)",
            "Casting::float(2)",
        ] {
            rejects_without_output(
                &format!("println(1); println({call}); import std::Casting;"),
                "requiere un 'import std::Casting;' anterior",
            );
        }
        rejects_without_output(
            "import std::Array; println(float(2));",
            "import std::Casting",
        );
        rejects_without_output(
            "import std::Casting; println([1].len());",
            "import std::Array",
        );
        rejects_without_output(
            "use std::Casting; import std::Casting;",
            "requiere un 'import std::Casting;' anterior",
        );
        rejects_without_output(
            "import std::Casting; println(Casting::int(1)); use std::Casting;",
            "requiere un 'use std::Casting;' anterior",
        );
        rejects_without_output(
            "import std::Casting; import std::Array; use std::Array; println(Casting::int(1));",
            "use std::Casting",
        );
        for source in [
            "if (false) { import std::Casting; }",
            "while (false) { use std::Casting; }",
        ] {
            rejects_without_output(source, "solo se permiten en el ámbito global");
        }
        for call in [
            "std::Array::int(1)",
            "Other::float(1)",
            "std::casting::int(1)",
            "std::Casting::missing(1)",
            "cast(1)",
        ] {
            rejects_without_output(
                &format!("import std::Casting; println({call});"),
                "desconocid",
            );
        }
        assert_eq!(
            output(
                "import std::Casting; import std::Casting; use std::Casting; use std::Casting; int Casting = 3; int cast = 4; println(Casting::int(cast) + Casting);"
            ),
            "7\n"
        );
        rejects_without_output("println(int(1));", "import std::Casting");
    }

    #[test]
    fn casting_rejects_unsupported_pairs_and_preserves_strict_types() {
        for call in [
            "int(true)",
            "float(false)",
            "bool(1)",
            "bool(1.0)",
            "char(1.0)",
            "float('a')",
            "bool('a')",
            "char(true)",
            "string([1])",
            "[1].cast(int[])",
            "1.cast(int[])",
        ] {
            rejects_without_output(
                &format!("import std::Casting; println(1); println({call});"),
                "Conversión no admitida",
            );
        }
        for source in [
            "float x = 1;",
            "int n = 1; n = float(n);",
            "println(1 + float(2));",
            "println(false && bool(1));",
            "float[] a = [1];",
            "const int n = 1; n = int(2);",
        ] {
            rejects_without_output(
                &format!("import std::Casting; println(1); {source}"),
                "Línea 1:",
            );
        }
        rejects_without_output(
            "import std::Casting; println(string([]));",
            "array vacío necesita",
        );
        rejects_without_output(
            "import std::Casting; import std::Array; int[] a = []; println(int(a.push(1)));",
            "no devuelve un valor",
        );
    }

    #[test]
    fn casting_rejects_invalid_syntax_and_arity_before_execution() {
        for source in [
            "println(int());",
            "println(float(1, 2));",
            "println(std::Casting::int());",
            "println(1.cast());",
            "println(1.cast(int, float));",
            "println(1.cast(\"int\"));",
            "int destino = 0; println(1.cast(destino));",
            "println(1.cast(int)); float(2)",
            "println(int);",
            "println(1.cast(int);",
            "println(float(1,));",
            "int float = 1;",
            "println(1.cast(int()));",
            "println(Casting::float(1, 2));",
        ] {
            rejects_without_output(
                &format!("import std::Casting; use std::Casting; println(1); {source}"),
                "Línea 1:",
            );
        }
    }

    #[test]
    fn casting_validates_numeric_and_unicode_boundaries() {
        assert_eq!(
            output(
                r#"import std::Casting;
            println(int("9223372036854775807"));
            println(int("-9223372036854775808"));
            println(int(-9223372036854775808.0));
            println(int(9223372036854774784.0));
            println(float(9007199254740993) == 9007199254740992.0);
            println(float("1e-9999"));
            println(int(char(0))); println(int(char(55295)));
            println(int(char(57344))); println(int(char(1114111)));
        "#
            ),
            "9223372036854775807\n-9223372036854775808\n-9223372036854775808\n9223372036854774784\ntrue\n0.0\n0\n55295\n57344\n1114111\n"
        );
    }

    #[test]
    fn casting_runtime_errors_preserve_output_and_conversion_lines() {
        for (value, target) in [
            ("9223372036854775808.0", "int"),
            ("-9223372036854777856.0", "int"),
            (r#""9223372036854775808""#, "int"),
            (r#""-9223372036854775809""#, "int"),
            (r#""1.5""#, "int"),
            (r#""1e2""#, "int"),
            (r#""0xff""#, "int"),
            (r#"" 1""#, "int"),
            (r#""""#, "int"),
            (r#""+""#, "int"),
            (r#""1e999""#, "float"),
            (r#""NaN""#, "float"),
            (r#""inf""#, "float"),
            (r#"" 1.0""#, "float"),
            (r#""1.0 ""#, "float"),
            (r#""x""#, "float"),
            (r#""True""#, "bool"),
            (r#""1""#, "bool"),
            (r#""""#, "char"),
            (r#""ab""#, "char"),
            (r#""é""#, "char"),
            ("-1", "char"),
            ("55296", "char"),
            ("57343", "char"),
            ("1114112", "char"),
        ] {
            for call in [
                format!("{target}({value})"),
                format!("({value}).cast({target})"),
            ] {
                let source = format!(
                    "import std::Casting;\nprintln(\"previo\");\nprintln({call}); println(\"posterior\");"
                );
                let mut bytes = Vec::new();
                let error = run(&source, &mut bytes).unwrap_err().to_string();
                assert!(
                    error.starts_with("Línea 3: No se puede convertir"),
                    "{source}: {error}"
                );
                assert_eq!(bytes, b"previo\n");
            }
        }
        rejects_without_output(
            "import std::Casting; println((1 / 0).cast(float));",
            "por cero",
        );
        rejects_without_output(
            "import std::Casting; println([1][2].cast(float));",
            "fuera de rango",
        );
        rejects_without_output(
            "import std::Casting; println(1.0\n.cast(bool));",
            "Línea 2: Conversión no admitida",
        );
    }

    #[test]
    fn executes_array_library_example() {
        assert_eq!(
            output(include_str!("../examples/biblioteca_arrays.oki")),
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
                &format!(
                    "import std::Array; use std::Array; println(\"previo\"); println({call});"
                ),
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
                &format!(
                    "import std::Array; use std::Array; println(\"previo\"); println({call});"
                ),
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
                &format!(
                    "import std::Array; use std::Array; println(\"previo\"); println({call});"
                ),
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
        let error = run("import std::Array; println(\"previo\");\nprintln([1 / 0].len()); println(\"posterior\");", &mut bytes).unwrap_err().to_string();
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

    #[test]
    fn executes_arrays_example() {
        assert_eq!(
            output(include_str!("../examples/arrays.oki")),
            "[1, 2, 3]\n10\n[1, 10, 3]\n[99, 10, 3]\n[\"Ana\", \"世界\"]\n[]\n3\ntrue\n"
        );
    }

    #[test]
    fn executes_array_mutation_example() {
        assert_eq!(
            output(include_str!("../examples/modificar_arrays.oki")),
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

    #[test]
    fn executes_basic_types_example() {
        assert_eq!(
            output(include_str!("../examples/tipos.oki")),
            "25\n19.95\ntrue\nñ\nHola\n26\n"
        );
    }

    #[test]
    fn prints_all_literal_types() {
        assert_eq!(
            output(
                "print(42);println(-7);println(1.0);println(-2.5);println(true);println(false);print('🦀');println(\"!\");"
            ),
            "42-7\n1.0\n-2.5\ntrue\nfalse\n🦀!\n"
        );
    }

    #[test]
    fn assigns_and_copies_each_type_without_aliasing() {
        for (kind, first, second, expected) in [
            ("int", "1", "2", "1\n2\n"),
            ("float", "1.0", "2.5", "1.0\n2.5\n"),
            ("bool", "true", "false", "true\nfalse\n"),
            ("char", "'a'", "'界'", "a\n界\n"),
            ("string", "\"uno\"", "\"dos\"", "uno\ndos\n"),
        ] {
            let source = format!(
                "{kind} original = {first}; {kind} copia = original; original = {second}; copia = copia; println(copia); println(original);"
            );
            assert_eq!(output(&source), expected);
        }
    }

    fn rejects_without_output(source: &str, message: &str) {
        let mut bytes = Vec::new();
        let error = run(source, &mut bytes).unwrap_err().to_string();
        assert!(error.contains(message), "{source}: {error}");
        assert!(
            bytes.is_empty(),
            "Se ejecutó un programa inválido: {source}"
        );
    }

    #[test]
    fn executes_constants_example() {
        assert_eq!(
            output(include_str!("../examples/constantes.oki")),
            "10\n11\n19.95\ntrue\nñ\nHola mundo\n"
        );
    }

    #[test]
    fn rejects_constant_reassignment_before_printing() {
        for (kind, initial, replacement) in [
            ("int", "1", "2"),
            ("float", "1.0", "2.0"),
            ("bool", "true", "false"),
            ("char", "'a'", "'ñ'"),
            ("string", "\"uno\"", "\"dos\""),
        ] {
            for value in [replacement, initial, "fijo"] {
                rejects_without_output(
                    &format!("const {kind} fijo = {initial};\nprintln(fijo);\nfijo = {value};"),
                    "Línea 3: No se puede reasignar la constante 'fijo'.",
                );
            }
        }
    }

    #[test]
    fn constants_copy_values_without_aliasing() {
        assert_eq!(
            output(
                r#"
                string original = "Hola";
                const string fijo = original;
                const string otro = fijo + "!";
                original = "Adiós";
                string copia = fijo;
                copia = copia + " mundo";
                println(fijo);
                println(otro);
                println(original);
                println(copia);
                int const2 = 1;
                const2 = 2;
                println(const2);
            "#
            ),
            "Hola\nHola!\nAdiós\nHola mundo\n2\n"
        );
    }

    #[test]
    fn constants_require_valid_declarations() {
        for source in [
            "const",
            "const x = 1;",
            "const int x;",
            "const int x = ;",
            "const int x = 1",
            "const int = 1;",
            "const const int x = 1;",
            "int const x = 1;",
            "int const = 1;",
            "const int const = 1;",
        ] {
            rejects_without_output(&format!("println(\"previo\");\n{source}"), "Línea 2:");
        }
        for source in [
            "const int x = x;",
            "const int x = y;",
            "println(x); const int x = 1;",
        ] {
            rejects_without_output(source, "no está declarada");
        }
        for first in ["int x = 1;", "const int x = 1;"] {
            for second in ["int x = 2;", "const int x = 2;"] {
                rejects_without_output(
                    &format!("{first} println(x); {second}"),
                    "ya está declarada",
                );
            }
        }
    }

    #[test]
    fn rejects_every_cross_type_declaration_and_assignment() {
        let values = [
            ("int", "1"),
            ("float", "1.0"),
            ("bool", "true"),
            ("char", "'x'"),
            ("string", "\"x\""),
        ];
        for (expected, initial) in values {
            for (actual, value) in values {
                if expected == actual {
                    continue;
                }
                let message = format!("se esperaba {expected}, se recibió {actual}");
                rejects_without_output(
                    &format!("println(\"previo\"); const {expected} x = {value};"),
                    &message,
                );
                rejects_without_output(
                    &format!("println(\"previo\"); {expected} x = {value};"),
                    &message,
                );
                rejects_without_output(
                    &format!("{expected} x = {initial}; println(x); x = {value};"),
                    &message,
                );
                rejects_without_output(
                    &format!("{actual} y = {value}; {expected} x = y;"),
                    &message,
                );
                rejects_without_output(
                    &format!("{expected} x = {initial}; {actual} y = {value}; x = y;"),
                    &message,
                );
            }
        }
    }

    #[test]
    fn requires_explicit_declarations_before_use() {
        for source in [
            "edad = 25;",
            "println(edad);",
            "int copia = edad;",
            "int edad = edad;",
            "println(edad); int edad = 25;",
        ] {
            rejects_without_output(source, "no está declarada");
        }
        for source in ["int x = 1; int x = 2;", "int x = 1; string x = \"x\";"] {
            rejects_without_output(source, "ya está declarada");
        }
    }

    #[test]
    fn rejects_incomplete_declarations_and_assignments() {
        for source in [
            "int x;",
            "int x = ;",
            "int x = 1",
            "int x = 1; x = 2",
            "int = 1;",
            "int bool = 1;",
            "bool true = false;",
            "int print = 1;",
            "var x = 1;",
            "let x = 1;",
            "auto x = 1;",
            "double x = 1.0;",
            "int x = 1; x = ;",
        ] {
            rejects_without_output(source, "Línea 1:");
        }
    }

    #[test]
    fn accepts_numeric_boundaries_and_exponents() {
        assert_eq!(
            output(
                "println(-9223372036854775808);println(9223372036854775807);println(0);println(-0.0);println(1e3);println(-2.5E-2);println(1e+2);println(1.7976931348623157e308);"
            ),
            "-9223372036854775808\n9223372036854775807\n0\n-0.0\n1000.0\n-0.025\n100.0\n1.7976931348623157e308\n"
        );
        for value in [
            "9223372036854775808",
            "-9223372036854775809",
            "1e309",
            "-1e309",
        ] {
            rejects_without_output(&format!("println({value});"), "fuera del rango");
        }
    }

    #[test]
    fn rejects_malformed_numbers_and_invalid_expressions() {
        for value in [
            "1.",
            ".5",
            "1e",
            "1e+",
            "1e-",
            "1.2.3",
            "12abc",
            "0x10",
            "1_000",
            "-true",
            "1 +",
            "* 2",
            "()",
            "(1 + 2",
            "1 2",
            "true & false",
            "true | false",
            "x = 2",
            "1 < 2 < 3",
            "1 ** 2",
            "!",
            "true &&",
            "false ||",
            "1 ==",
            "1 + * 2",
        ] {
            rejects_without_output(&format!("int x = 1; println({value});"), "Línea 1:");
        }
    }

    #[test]
    fn evaluates_arithmetic_precedence_grouping_and_unary_operators() {
        for (expression, expected) in [
            ("2+3*4", "14"),
            ("(2+3)*4", "20"),
            ("20-5-3", "12"),
            ("24/4/2", "3"),
            ("17%5*2", "4"),
            ("-(2+3)*+2", "-10"),
            ("+-+2", "-2"),
            ("7/2", "3"),
            ("-7/2", "-3"),
            ("7/-2", "-3"),
            ("-7%2", "-1"),
            ("7%-2", "1"),
            ("1.5+2.0*3.0", "7.5"),
            ("5.5-2.0", "3.5"),
            ("7.0/2.0", "3.5"),
            ("-7.5%2.0", "-1.5"),
            ("7.5%-2.0", "1.5"),
            ("-(1.5+2.0)", "-3.5"),
            ("+1.0", "1.0"),
            ("1e-2+1e+2", "100.01"),
        ] {
            assert_eq!(
                output(&format!("println({expression});")),
                format!("{expected}\n"),
                "{expression}"
            );
        }
    }

    #[test]
    fn expressions_work_in_declarations_assignments_and_prints() {
        assert_eq!(
            output(
                r#"
            int x = 2 + 3 * 4;
            x = (x - 4) / 2;
            float y = 2.5;
            y = -y + +y * 2.0;
            bool ok = x == 5 && y >= 2.5;
            ok = !ok || 'a' < 'ñ';
            char letter = ('界');
            string text = "Hola";
            string copy = text + " " + "🦀";
            text = text + "!";
            print(x); println(y); println(ok); println(letter); println(copy); println(text);
        "#
            ),
            "52.5\ntrue\n界\nHola 🦀\nHola!\n"
        );
    }

    #[test]
    fn compares_values_of_each_type() {
        for (first, second) in [
            ("1", "2"),
            ("1.5", "2.0"),
            ("'ñ'", "'界'"),
            ("\"a\"", "\"aa\""),
        ] {
            assert_eq!(output(&format!(
                "println({first} < {second}); println({first} <= {first}); println({second} > {first}); println({second} >= {second});
                 println({first} > {second}); println({second} <= {first}); println({first} >= {second}); println({first} < {first});"
            )), "true\ntrue\ntrue\ntrue\nfalse\nfalse\nfalse\nfalse\n");
        }
        for (first, second) in [
            ("1", "2"),
            ("1.5", "2.0"),
            ("true", "false"),
            ("'ñ'", "'界'"),
            ("\"a\"", "\"aa\""),
        ] {
            assert_eq!(
                output(&format!(
                    "println({first} == {first}); println({first} != {second}); println({first} == {second}); println({first} != {first});"
                )),
                "true\ntrue\nfalse\nfalse\n"
            );
        }
        assert_eq!(
            output(
                r#"println("A" < "a"); println("é" == "é"); println("" + "ñ" == "ñ"); println(0.0 == -0.0);"#
            ),
            "true\nfalse\ntrue\ntrue\n"
        );
    }

    #[test]
    fn evaluates_boolean_truth_tables_and_precedence() {
        for (left, right, and, or) in [
            ("false", "false", "false", "false"),
            ("false", "true", "false", "true"),
            ("true", "false", "false", "true"),
            ("true", "true", "true", "true"),
        ] {
            assert_eq!(
                output(&format!(
                    "println({left} && {right}); println({left} || {right});"
                )),
                format!("{and}\n{or}\n")
            );
        }
        assert_eq!(
            output(
                "println(!true); println(!!true); println(!false == true); println(true || false && false); println((true || false) && false); println(1+2*3 >= 7 == true && !false || false);"
            ),
            "false\ntrue\ntrue\ntrue\nfalse\ntrue\n"
        );
    }

    #[test]
    fn logical_operators_short_circuit_but_check_both_operands() {
        assert_eq!(
            output("println(false && 1/0 == 0); println(true || 9223372036854775807+1 == 0);"),
            "false\ntrue\n"
        );
        for expression in ["true && 1/0 == 0", "false || 1/0 == 0"] {
            rejects_without_output(&format!("println({expression});"), "por cero");
        }
        for expression in ["false && 1", "true || 1", "true || (1 + false == 1)"] {
            rejects_without_output(
                &format!("println(\"previo\"); println({expression});"),
                "no admite",
            );
        }
        rejects_without_output(
            "println(\"previo\"); println(true || desconocida);",
            "no está declarada",
        );
    }

    #[test]
    fn rejects_cross_type_operators_and_unsupported_type_operations() {
        let values = ["1", "1.0", "true", "'a'", "\"a\""];
        for left in values {
            for right in values {
                if left == right {
                    continue;
                }
                for operator in [
                    "+", "-", "*", "/", "%", "==", "!=", "<", "<=", ">", ">=", "&&", "||",
                ] {
                    rejects_without_output(
                        &format!("print(\"previo\");\nprintln({left} {operator} {right});"),
                        "Línea 2:",
                    );
                }
            }
        }
        for expression in [
            "true+false",
            "'a'+'b'",
            "\"a\"-\"b\"",
            "'a'*'b'",
            "true<false",
            "1&&2",
            "\"a\"||\"b\"",
            "!1",
            "+true",
            "-'a'",
            "+\"a\"",
            "!1.0",
        ] {
            rejects_without_output(
                &format!("print(\"previo\"); println({expression});"),
                "no admite",
            );
        }
        rejects_without_output(
            "print(\"previo\"); int x = 1 < 2;",
            "se esperaba int, se recibió bool",
        );
        rejects_without_output(
            "bool x = true; print(x); x = 1 + 2;",
            "se esperaba bool, se recibió int",
        );
    }

    #[test]
    fn reports_zero_division_and_overflow_without_panicking() {
        for expression in [
            "1/0", "1%0", "0/0", "1.0/0.0", "1.0%-0.0", "0.0/0.0", "1.0/-0.0",
        ] {
            rejects_without_output(&format!("println({expression});"), "por cero");
        }
        for expression in [
            "9223372036854775807+1",
            "-9223372036854775808-1",
            "9223372036854775807*2",
            "-9223372036854775808/-1",
            "-(-9223372036854775808)",
            "1e308*2.0",
            "1e308+1e308",
            "-1e308-1e308",
            "1e308/1e-308",
        ] {
            rejects_without_output(&format!("println({expression});"), "fuera del rango");
        }
        assert_eq!(
            output(
                "int min = -9223372036854775808; println(min % -1); println(min+1); println(9223372036854775807-1); println(1e-300*1e-300);"
            ),
            "0\n-9223372036854775807\n9223372036854775806\n0.0\n"
        );
        rejects_without_output(
            "int min = -9223372036854775808; println(-min);",
            "fuera del rango",
        );
    }

    #[test]
    fn operator_errors_report_the_operator_line_and_stop_execution() {
        rejects_without_output("print(\"previo\"); println(1\n+ true);", "Línea 2:");
        rejects_without_output("print(\"previo\"); println(\n!\n1);", "Línea 2:");
        let mut bytes = Vec::new();
        let error = run(
            "println(\"previo\");\nprintln(1\n/ 0); println(\"posterior\");",
            &mut bytes,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Línea 3:"), "{error}");
        assert!(error.contains("por cero"), "{error}");
        assert_eq!(bytes, b"previo\n");
    }

    #[test]
    fn executes_operations_example() {
        assert_eq!(
            output(include_str!("../examples/operaciones.oki")),
            "14\n20\n3\n1\n3.5\ntrue\ntrue\nHola, mundo\ntrue\nfalse\n"
        );
    }

    #[test]
    fn executes_conditions_example() {
        assert_eq!(
            output(include_str!("../examples/condiciones.oki")),
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
            output(include_str!("../examples/bucles.oki")),
            "15\n11\n12\n21\n22\nAna\nLuis\nMar\n20\n"
        );
    }

    #[test]
    fn executes_asignaciones_example() {
        assert_eq!(
            output(include_str!("../examples/asignaciones.oki")),
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

    #[test]
    fn executes_functions_example() {
        assert_eq!(
            output(include_str!("../examples/funciones.oki")),
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
}
