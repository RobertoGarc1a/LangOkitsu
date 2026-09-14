# OkitsuLang

Intérprete escrito en Rust para aprender a construir un lenguaje paso a paso. Las variables siempre declaran su tipo y se inicializan con un valor del mismo tipo:

```oki
int edad = 25;
float precio = 19.95;
bool activo = true;
char inicial = 'ñ';
string saludo = "Hola";

println(saludo);
println(edad);
edad = 26;
println(edad);
```

Los tipos básicos son `int`, `float`, `bool`, `char` y `string`. También hay arrays del mismo tipo, escritos como `int[]`, `string[]` o `int[][]`. No hay inferencia del tipo de una variable ni conversiones implícitas: `float precio = 25;` es un error; `float precio = 25.0;` es válido. La comprobación de tipos se realiza antes de ejecutar el archivo.

Para impedir que un valor cambie, añade `const` antes del tipo:

```oki
const int limite = 10;
println(limite);
```

Una asignación posterior como `limite = 20;` es un error antes de ejecutar el archivo. `const` admite los tipos básicos y los arrays y exige un valor inicial. Consulta las [reglas de las constantes](docs/estado-actual.md#constantes).

Un **array** guarda una secuencia de elementos del mismo tipo. Los índices empiezan en cero:

```oki
int[] numeros = [1, 2, 3];
numeros[0] = 10;
println(numeros);
println(numeros[1]);
```

Imprime `[10, 2, 3]` y `2`. Se pueden copiar, reasignar y anidar arrays; `const` también impide modificar sus elementos. Consulta las [reglas de arrays](docs/estado-actual.md#arrays).

`print(expresión);` escribe sin añadir un salto de línea y `println(expresión);` añade uno. Se mantienen los paréntesis y el punto y coma obligatorio. Una expresión puede combinar literales (valores escritos directamente), variables ya declaradas y operadores, usando paréntesis para agrupar.

```oki
println(2 + 3 * 4);
println(7.0 / 2.0);
println(true && !false);
println('a' < 'z');
println("Hola" + " mundo");
```

Los números admiten `+`, `-`, `*`, `/`, `%` y signos unarios; los booleanos, `!`, `&&` y `||`; las cadenas, concatenación con `+`. Todos los tipos admiten `==` y `!=`; números, caracteres y cadenas también admiten `<`, `<=`, `>` y `>=`. Los dos operandos deben tener el mismo tipo.

```sh
cargo run -- examples/hello.oki
cargo run -- examples/tipos.oki
cargo run -- examples/operaciones.oki
cargo run -- examples/constantes.oki
cargo run -- examples/arrays.oki
```

## Documentación para aprender

Leer en este orden:

1. [Estado actual](docs/estado-actual.md): sintaxis, tipos, ejemplos ejecutables y límites.
2. [Funcionamiento interno](docs/funcionamiento-interno.md): recorrido desde los caracteres hasta la salida, pasando por el árbol de sintaxis y la comprobación de tipos.
3. [Historial de desarrollo](docs/historial.md): avances, motivos y comprobaciones.

[AGENTS.md](AGENTS.md) contiene las pautas de trabajo y establece cómo mantener esta documentación con cada avance.

## Estructura

Los programas de ejemplo están en [examples/](examples/), al mismo nivel que `src/`.

Seguimos el intérprete de árbol de [Crafting Interpreters](https://craftinginterpreters.com/contents.html), adaptándolo a Rust y a las decisiones de OkitsuLang. No hay dependencias externas.

| Archivo | Responsabilidad |
| --- | --- |
| [src/main.rs](src/main.rs) | Lectura del archivo, coordinación de etapas y pruebas. |
| [src/scanner.rs](src/scanner.rs) | Reconocer tokens y sus líneas (capítulo 4). |
| [src/parser.rs](src/parser.rs) | Definir el AST y construirlo mediante análisis descendente (capítulos 5, 6 y 8). |
| [src/value.rs](src/value.rs) | Representar tipos, valores y su impresión (capítulo 7). |
| [src/type_checker.rs](src/type_checker.rs) | Comprobar nombres y tipos antes de ejecutar; adaptación propia para el tipado estricto. |
| [src/interpreter.rs](src/interpreter.rs) | Evaluar el AST y guardar los valores de las variables (capítulos 7 y 8). |

El libro usa tipado dinámico en Lox; OkitsuLang exige anotaciones de tipo y compatibilidad exacta. Solo se implementa el fragmento descrito en la documentación: no hay todavía bloques, funciones ni máquina virtual.

## Comprobaciones

```sh
cargo fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
```
