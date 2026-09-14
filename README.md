# OkitsuLang

Intérprete escrito en Rust. Por ahora la única instrucción es `print`, con una cadena entre comillas dobles:

```oki
print("Hello world")
```

```sh
cargo run -- hello.oki
```

## Documentación para aprender

Leer en este orden:

1. [Estado actual](docs/estado-actual.md): qué está hecho, cómo probarlo y qué límites tiene.
2. [Funcionamiento interno](docs/funcionamiento-interno.md): recorrido de `print` desde el archivo hasta la pantalla, con ejemplos de tokens y del árbol de sintaxis.
3. [Historial de desarrollo](docs/historial.md): qué se ha ido haciendo, por qué y cómo se ha comprobado.

[AGENTS.md](AGENTS.md) contiene las pautas de trabajo y establece cómo mantener esta documentación con cada avance.

## Estructura

Seguimos la primera parte práctica de [Crafting Interpreters](https://www.craftinginterpreters.com/contents.html): un intérprete que recorre un árbol de sintaxis. Todo está en `src/main.rs`, sin dependencias externas.

1. `Scanner` transforma los caracteres en tokens (capítulo 4).
2. `Expr` y `Stmt` representan el árbol de sintaxis o AST (capítulo 5).
3. `Parser` construye el AST mediante análisis descendente (capítulos 6 y 8).
4. `Interpreter` evalúa expresiones y ejecuta instrucciones (capítulos 7 y 8).

La estructura adapta el enfoque del libro a Rust; solo implementa el fragmento necesario para imprimir texto. Conservamos la sintaxis de Okitsu: `print("texto")`, con paréntesis y sin punto y coma, en lugar de `print "texto";` de Lox.

Gramática actual:

```text
program    → statement* EOF
statement  → "print" "(" expression ")"
expression → STRING
```

Un archivo puede contener varios `print`. Los espacios y saltos de línea entre tokens se ignoran. Las cadenas admiten Unicode y saltos de línea, y terminan en la siguiente comilla doble; todavía no se interpretan secuencias de escape. Los errores léxicos y sintácticos indican la línea e impiden ejecutar el archivo.

Para ampliar el lenguaje, añadiremos los tokens, nodos y reglas necesarios en cada etapa. Esta versión ejecuta el AST; todavía no implementa la máquina virtual de bytecode de la segunda parte del libro.

## Comprobaciones

```sh
cargo test
cargo fmt -- --check
```
