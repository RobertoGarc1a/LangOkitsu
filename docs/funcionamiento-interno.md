# Cómo funciona internamente

La idea central es resolver una tarea distinta en cada etapa: reconocer las piezas del texto, comprobar cómo encajan y ejecutar lo que significan. Todas las estructuras mencionadas están en [src/main.rs](../src/main.rs).

```text
archivo.oki
    │ run_file(): lee el archivo
    ▼
texto fuente
    │ Scanner: reconoce las piezas
    ▼
tokens
    │ Parser: comprueba la gramática
    ▼
AST (árbol de sintaxis)
    │ Interpreter: ejecuta las instrucciones
    ▼
salida de texto
```

Usaremos `print("Hello world")` como ejemplo en todo el recorrido.

## 1. Leer el archivo y coordinar las etapas

`main()` llama a `run_file()`. Esta función obtiene la ruta mediante `env::args_os().nth(1)` y lee el contenido con `fs::read_to_string`. El resultado es un `String` de Rust con todo el código fuente.

`run_file()` pasa ese texto y la salida estándar a `run()`. Esta última conecta las etapas:

```rust
let tokens = Scanner::new(source).scan_tokens()?;
let statements = Parser::new(tokens).parse()?;
Interpreter { output }.interpret(&statements)?;
```

El operador `?` significa: si la operación falla, devolver ese error inmediatamente; si funciona, continuar con su resultado. Por eso el intérprete solo recibe un programa cuyo análisis completo ha terminado bien.

## 2. Scanner: de caracteres a tokens

Un **token** es una pieza con significado para la sintaxis, como una palabra reservada, un paréntesis o una cadena. `TokenKind` enumera los tipos disponibles y `Token` guarda el tipo y la línea donde empieza.

Para el ejemplo, `scan_tokens()` produce esta secuencia:

| Texto leído | Tipo de token | Línea |
| --- | --- | --- |
| `print` | `Print` | 1 |
| `(` | `LeftParen` | 1 |
| `"Hello world"` | `String("Hello world")` | 1 |
| `)` | `RightParen` | 1 |
| Fin del archivo | `Eof` | 1 si no hay salto final; 2 si lo hay |

`Eof` es una marca añadida por el scanner, no un texto que haya que escribir en el archivo.

`Scanner` usa `Peekable<Chars>`: `Chars` recorre caracteres Unicode y `Peekable` permite mirar el siguiente sin consumirlo. El contador `line` empieza en 1 y aumenta al encontrar un salto de línea.

Cuando aparece una comilla, `string()` recoge el contenido hasta la siguiente comilla y lo guarda sin las comillas delimitadoras. Si el archivo acaba antes, devuelve un error. Los espacios dentro de la cadena se conservan; los espacios entre tokens se ignoran.

`identifier()` lee un nombre completo antes de decidir si es la palabra reservada `print`. Así, `printf` se reconoce como un identificador completo y no como `print` seguido de una `f`. Los nombres reconocidos empiezan por una letra ASCII o `_` y pueden continuar con letras ASCII, dígitos o `_`.

Esta etapa no comprueba si los paréntesis están bien colocados y no imprime nada.

## 3. Parser: de tokens a una instrucción válida

El **parser**, o analizador sintáctico, comprueba si los tokens cumplen la gramática:

```text
program    → statement* EOF
statement  → "print" "(" expression ")"
expression → STRING
```

`→` se lee como «se compone de» y `*` significa «cero o más repeticiones». `STRING` representa un token de cadena; `EOF` representa el final del archivo.

`Parser` conserva los tokens y un índice `current` que señala el siguiente token pendiente. Sus métodos siguen estas reglas:

1. `parse()` recoge instrucciones hasta encontrar `Eof`.
2. `statement()` exige `Print` y llama a `print_statement()`.
3. `print_statement()` exige `(`, analiza una expresión y exige `)`.
4. `expression()` acepta por ahora únicamente un token de cadena.

`peek()` consulta el token actual. `consume()` comprueba el tipo esperado y avanza; si no coincide, devuelve un error con la línea del token encontrado. Este estilo se llama **análisis descendente** porque parte de la regla del programa y baja a reglas más pequeñas. La gramática actual es muy sencilla; aún no necesita reglas de precedencia ni expresiones recursivas.

Por ejemplo, `print()` llega a `expression()` con un `RightParen` pendiente. Como se esperaba una cadena, se informa del error en lugar de crear una instrucción incompleta.

## 4. AST: representar el significado del programa

**AST** significa árbol de sintaxis abstracta. Conserva la estructura necesaria para ejecutar el código; los paréntesis ya han cumplido su función durante el análisis.

Tenemos dos enumeraciones de Rust:

```rust
enum Expr {
    String(String),
}

enum Stmt {
    Print(Expr),
}
```

Una **expresión** produce un valor. Una **instrucción** realiza una acción. En nuestro ejemplo, la expresión es el texto y la acción es imprimirlo.

El resultado conceptual del parser es:

```text
Programa: Vec<Stmt>
└── Stmt::Print
    └── Expr::String("Hello world")
```

El campo `String` del token contiene una cadena propia. `expression()` la clona para que el AST tenga su propia copia. El programa es un `Vec<Stmt>`: una lista de instrucciones en el orden en que aparecen en el archivo.

Separar estas representaciones permite añadir más variantes y reglas cuando se necesiten. No hace falta que el intérprete vuelva a leer comillas o paréntesis.

## 5. Intérprete: ejecutar el árbol

`Interpreter::interpret()` recorre la lista de instrucciones. Para cada una llama a `execute()`.

Cuando `execute()` recibe `Stmt::Print`, llama a `evaluate()` para obtener el valor de su expresión. Como solo existen cadenas, `evaluate()` devuelve un `&str`, una referencia al texto almacenado en el AST; no necesita copiarlo de nuevo.

Finalmente, `writeln!` escribe el texto y un salto de línea. El resultado es:

```text
Hello world
```

`Interpreter<W: Write>` acepta cualquier destino que implemente la capacidad de escritura `Write`. En la ejecución normal recibe la salida estándar; en las pruebas recibe un `Vec<u8>` para guardar los bytes y comparar el resultado. La lógica que ejecuta `print` es la misma en ambos casos.

## 6. Cómo se comunican los errores

| Etapa | Ejemplo de fallo | Qué sucede |
| --- | --- | --- |
| Lectura | No se proporciona una ruta o el archivo no existe. | Se devuelve un error antes del análisis. |
| Scanner | Cadena sin cerrar o carácter no reconocido fuera de una cadena. | Se detiene el análisis y se informa de la línea. |
| Parser | Falta un paréntesis o se usa un nombre distinto de `print`. | Se detiene el análisis y se informa de la línea. |
| Intérprete | El destino de salida no acepta la escritura. | Se propaga el error de entrada/salida. |

Scanner y parser devuelven mensajes `String`; la escritura devuelve `io::Error`. `run()` los reúne mediante `Box<dyn Error>`, que permite propagar distintos tipos de error a través del mismo resultado.

`main()` muestra el error por la salida de errores (`stderr`) y termina con un código de fallo. Si todo sale bien, termina con un código de éxito. Los mensajes de error no se mezclan con la salida normal de `print`.

Si hay un error léxico o sintáctico en una segunda instrucción, la primera tampoco se ejecuta: `run()` analiza el archivo completo antes de interpretar. En cambio, un error al escribir puede ocurrir después de haber emitido texto; no hay un mecanismo que deshaga esa salida.

El parser presupone que recibe la secuencia terminada en `Eof` que genera el scanner. Actualmente se informa solo del primer error; no hay recuperación para continuar buscando otros.

## Relación con el libro

La estructura sigue el recorrido del intérprete de árbol de [Crafting Interpreters](https://www.craftinginterpreters.com/contents.html): scanner (capítulo 4), AST (5), parser (6), evaluación (7) e instrucciones (8). Solo se ha implementado la parte necesaria para imprimir cadenas, adaptada a los `enum` y `match` de Rust.

La sintaxis elegida sigue siendo `print("texto")`; el libro usa `print "texto";` para Lox. La máquina virtual y el bytecode pertenecen a una etapa posterior del libro y no están implementados aquí.
