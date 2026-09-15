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

La biblioteca estándar incluye `std::Array`. Los arrays básicos funcionan sin importarla; la importación habilita sus métodos:

```oki
import std::Array;

int[] numeros = [10, 20, 30];
println(numeros.len());
println(std::Array::len(numeros));

use std::Array;
println(Array::len(numeros));
```

Imprime `3` tres veces. `import` habilita la biblioteca y `use` añade el nombre corto `Array`; `use` requiere un `import` anterior. Ambas instrucciones se escriben fuera de los bloques y antes de utilizar los nombres que habilitan. `len()` devuelve una longitud de tipo `int` sin modificar el array. Consulta las [reglas de la biblioteca estándar](docs/estado-actual.md#biblioteca-estándar-stdarray) y el ejemplo [biblioteca_arrays.oki](examples/biblioteca_arrays.oki).

Para construir listas durante la ejecución, `push` añade al final y `pop` elimina y devuelve el último elemento. Admiten las mismas dos formas que `len`:

```oki
import std::Array;
use std::Array;
int[] numeros = [1, 2, 3];
numeros.push(4);
Array::push(numeros, 5);
println(numeros.pop());
println(std::Array::pop(numeros));
println(numeros);
```

Imprime `5`, `4` y `[1, 2, 3]`. `push` no devuelve un valor. Ambas operaciones exigen un array modificable; `pop` sobre un array vacío produce un error. Consulta las [reglas de modificación](docs/estado-actual.md#añadir-y-eliminar-elementos) y el ejemplo [modificar_arrays.oki](examples/modificar_arrays.oki).

`print(expresión);` escribe sin añadir un salto de línea y `println(expresión);` añade uno. Se mantienen los paréntesis y el punto y coma obligatorio. Una expresión puede combinar literales (valores escritos directamente), variables ya declaradas, llamadas de biblioteca y operadores, usando paréntesis para agrupar.

```oki
println(2 + 3 * 4);
println(7.0 / 2.0);
println(true && !false);
println('a' < 'z');
println("Hola" + " mundo");
```

Los números admiten `+`, `-`, `*`, `/`, `%` y signos unarios; los booleanos, `!`, `&&` y `||`; las cadenas, concatenación con `+`. Todos los tipos admiten `==` y `!=`; números, caracteres y cadenas también admiten `<`, `<=`, `>` y `>=`. Los dos operandos deben tener el mismo tipo.

Para modificar una variable ya declarada existen formas abreviadas de la asignación: `+=`, `-=`, `++` y `--`. Son instrucciones completas, con `;`, y siguen las mismas reglas de tipo que `+` o `-`:

```oki
int contador = 0;
contador++;
contador += 5;
contador -= 3;
contador--;
println(contador);

float precio = 10.0;
precio += 2.5;
println(precio);
```

Imprime `2` y `12.5`. `+=` admite `int`, `float` y `string` (concatenación) igual que `+`; `-=` admite `int` y `float` igual que `-`. `++` y `--` suman o restan una unidad y solo admiten `int` y `float`. También se pueden aplicar a un elemento de array: `numeros[0]++`, `numeros[1] += 10`. Consulta las [reglas de las asignaciones abreviadas](docs/estado-actual.md#asignaciones-abreviadas).

Un **if** elige qué bloque ejecutar según una condición de tipo `bool`. Las ramas van entre llaves y el `if` completo no lleva `;` final; cada instrucción de dentro sí lo lleva:

```oki
int edad = 20;

if (edad >= 18) {
    println("mayor de edad");
} else {
    println("menor de edad");
}

if (edad < 13) {
    println("niñez");
} else if (edad < 18) {
    println("adolescencia");
} else {
    println("adultez");
}
```

Imprime `mayor de edad` y `adultez`. Se puede encadenar con `else if` y omitir el `else`. Cada bloque `{ ... }` abre un ámbito propio: los nombres declarados dentro solo existen ahí y se pueden reutilizar en otro bloque, mientras que las variables de fuera se pueden reasignar dentro. Consulta las [reglas de if/else](docs/estado-actual.md#control-de-flujo-if-y-else).

Un **while** repite un bloque mientras su condición sea `bool`:

```oki
int i = 1;
while (i <= 3) {
    println(i);
    i = i + 1;
}
```

Imprime `1`, `2` y `3`. El **for** reúne en su cabecera la inicialización, la condición y la actualización, separadas por `;`. Las tres son obligatorias. La actualización puede ser una asignación, una asignación abreviada o un incremento:

```oki
for (int i = 0; i < 3; i++) {
    println(i);
}
```

El **foreach** recorre un array del primero al último elemento, indicando el tipo de cada uno:

```oki
string[] nombres = ["Ana", "Luis"];
foreach (string nombre in nombres) {
    println(nombre);
}
```

Los tres terminan en `}` y no llevan `;` final. Su cuerpo es siempre un bloque `{ ... }` con su propio ámbito, y la variable del `for` y la del `foreach` solo existen dentro del bucle. Consulta las [reglas de los bucles](docs/estado-actual.md#bucles).

```sh
cargo run -- examples/hello.oki
cargo run -- examples/tipos.oki
cargo run -- examples/operaciones.oki
cargo run -- examples/constantes.oki
cargo run -- examples/arrays.oki
cargo run -- examples/condiciones.oki
cargo run -- examples/bucles.oki
cargo run -- examples/asignaciones.oki
cargo run -- examples/biblioteca_arrays.oki
cargo run -- examples/modificar_arrays.oki
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
| [src/stdlib.rs](src/stdlib.rs) | Habilitar nombres de la biblioteca estándar, comprobar sus llamadas y ejecutar `len`, `push` y `pop` de arrays. |

El libro usa tipado dinámico en Lox; OkitsuLang exige anotaciones de tipo y compatibilidad exacta. Solo se implementa el fragmento descrito en la documentación: hay llamadas a la biblioteca estándar, pero todavía no hay funciones definidas por el usuario ni máquina virtual.

## Comprobaciones

```sh
cargo fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
```
