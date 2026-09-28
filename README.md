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

Para convertir de forma explícita, importa `std::Casting`. Puedes llamar a la función del tipo de destino o usar `.cast(tipo)`:

```oki
import std::Casting;
int edad = 25;
float decimal = float(edad);
string texto = "Edad: " + string(edad);
float numero = 19.95;
int entero = numero.cast(int);
println(decimal);
println(texto);
println(entero);
```

Imprime `25.0`, `Edad: 25` y `19`. La conversión devuelve un valor nuevo: `numero` sigue siendo `19.95`. También existen `std::Casting::float(edad)` y, tras `use std::Casting;`, `Casting::float(edad)`. Consulta las [conversiones admitidas y sus errores](docs/estado-actual.md#conversiones-explícitas-stdcasting) y el ejemplo [conversiones.oki](examples/conversiones.oki).

Para impedir que un valor cambie, añade `const` antes del tipo:

```oki
const int limite = 10;
println(limite);
```

Una asignación posterior como `limite = 20;` es un error antes de ejecutar el archivo. `const` admite los tipos básicos, las estructuras, los arrays y las uniones y exige un valor inicial. Consulta las [reglas de las constantes](docs/estado-actual.md#constantes).

Una **estructura** agrupa campos con nombres y tipos. Se declara con `struct` antes de usarla:

```oki
struct Persona {
    string nombre;
    int edad;
}
Persona ana = Persona { nombre: "Ana", edad: 30 };
Persona copia = ana;
copia.edad++;
println(ana);
println(copia.edad);
```

Imprime `Persona { nombre: "Ana", edad: 30 }` y `31`. Los campos sin valor por defecto son obligatorios; la copia es independiente y `const` protege también los campos interiores. Las estructuras pueden contener otras ya declaradas y arrays, y usarse como parámetros y retornos de funciones. Para modificar el original desde una función se conserva `inout`. Consulta las [reglas de estructuras](docs/estado-actual.md#estructuras-con-campos) y el ejemplo [estructuras.oki](examples/estructuras.oki).

Los campos también admiten `const`, uniones y valores por defecto:

```oki
struct Registro {
    const int id;
    int doble = id * 2;
    int || string valor = 0;
}
Registro dato = Registro { id: 3 };
println(dato.doble);
if (type dato.valor == int) {
    dato.valor++;
    println(dato.valor);
}
```

Imprime `6` y `1`. `id` se fija al construir; los valores por defecto se calculan para cada instancia y pueden leer campos anteriores. Una asignación completa puede cambiar la alternativa de `valor`, tras lo que hay que volver a comprobar su tipo. Todos los campos son públicos; no hay métodos propios.

Para representar un árbol, una estructura puede contener arrays de su propio tipo:

```oki
struct Nodo { int valor; Nodo[] hijos = []; }
Nodo raiz = Nodo { valor: 1, hijos: [Nodo { valor: 2 }] };
println(raiz.hijos[0].valor);
```

Imprime `2`. Un array vacío termina la recursión. También se pueden formar listas con una unión como `Nodo || bool` y un booleano como terminación, manteniendo las copias independientes. Consulta el ejemplo [estructuras_campos.oki](examples/estructuras_campos.oki) y las [reglas de recursión](docs/estado-actual.md#estructuras-recursivas).

El ejemplo [lista_enlazada.oki](examples/lista_enlazada.oki) construye `10 -> 20 -> 30 -> fin`, recorre los nodos, cuenta y busca elementos, elimina el primero y demuestra las copias independientes. Consulta su [explicación y salida](docs/estado-actual.md#ejemplo-de-lista-enlazada).

Un **enum** define un conjunto cerrado de variantes. Pueden no contener datos o guardar valores con tipo; se distinguen mediante un **match exhaustivo**, que exige una rama por variante:

```oki
enum Resultado { Ok(int valor), Error(string mensaje) }
Resultado resultado = Resultado::Ok(5);
match resultado {
    Resultado::Ok(valor) => { println(valor); },
    Resultado::Error(mensaje) => { println(mensaje); }
}
```

Imprime `5`. Las capturas reciben copias y solo existen dentro de su rama. Las variantes sin datos se escriben como `Estado::Hecho`, sin paréntesis. Consulta las [reglas de enums y match](docs/estado-actual.md#enums-y-match) y el ejemplo [enums.oki](examples/enums.oki).

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

Los tres terminan en `}` y no llevan `;` final. Su cuerpo es siempre un bloque `{ ... }` con su propio ámbito, y la variable del `for` y la del `foreach` solo existen dentro del bucle. Dentro de cualquier bucle se puede usar `break;` para salir del más interno o `continue;` para pasar a la siguiente vuelta. Consulta las [reglas de los bucles](docs/estado-actual.md#bucles).

Una **función propia** agrupa instrucciones y recibe parámetros con tipo. Se declara con `function`, solo en el ámbito global y antes de llamarla. Sin `-> tipo` no devuelve valor; con `-> tipo` devuelve un valor mediante `return`:

```oki
function saludar(string nombre) {
    println("Hola, " + nombre);
}

function sumar(int a, int b) -> int {
    return a + b;
}

function mayor(int a, int b) -> int {
    if (a > b) {
        return a;
    } else {
        return b;
    }
}

saludar("Ana");
println(sumar(2, 3));
println(mayor(10, 7));
```

Imprime `Hola, Ana`, `5` y `10`. Los parámetros y las variables locales existen solo durante la llamada; dentro se pueden leer variables globales, pero para modificarlas deben recibirse mediante un parámetro `inout`. La llamada debe indicar el número y el tipo exactos de sus argumentos. Una función con valor se usa como cualquier expresión; una sin valor, solo como instrucción `nombre(...);`. Una función con `-> tipo` debe devolver un valor de un tipo permitido por su firma en todos los caminos (`return expresión;`); una sin tipo puede usar `return;` para terminar antes. La recursión directa está permitida. Consulta las [reglas de las funciones](docs/estado-actual.md#funciones-propias) y el ejemplo [funciones.oki](examples/funciones.oki).


El retorno también puede admitir varios tipos separados por `||`: `function test(int numero) -> int || string { ... }`. Esto se llama **tipo unión**: cada llamada devuelve un solo valor, que puede ser entero o texto. Puedes imprimir el resultado o devolverlo desde otra función que admita sus posibles tipos. También puedes declarar variables unión, como `int || string resultado = test(5);`. Los parámetros siguen exigiendo un tipo concreto. Consulta las [reglas de retornos con varios tipos](docs/estado-actual.md#retornos-con-varios-tipos) y el ejemplo [retornos_union.oki](examples/retornos_union.oki).


Para distinguir el valor de una variable unión, usa `type variable == tipo` o `!=`. La comprobación produce un `bool` y permite usar el tipo concreto en la rama correspondiente:

```oki
int || string resultado = 5;
if (type resultado == int) {
    println(resultado + 1);
} else {
    println(resultado + "!");
}
resultado = "test";
if (type resultado == string && resultado != "") {
    println(resultado + "!");
}
```

Imprime `6` y `test!`. Consulta las [reglas de variables unión y comprobación de tipos](docs/estado-actual.md#variables-unión-y-comprobación-de-tipos) y el ejemplo [variables_union.oki](examples/variables_union.oki).


Para modificar una variable original desde una función, escribe `inout` tanto en el parámetro como en la llamada:

```oki
function acumular(inout int destino, int valor) {
    destino += valor;
}

int total = 0;
acumular(inout total, 5);
println(total);
```

Imprime `5`. Sin `inout`, los parámetros reciben copias independientes. Omitir la marca en uno de los dos sitios, pasar una constante o escribir directamente en una global desde una función produce un error antes de ejecutar. En esta versión se pasan variables completas, de tipo básico, estructura o array; no elementos como `inout datos[0]`. Consulta las [reglas de inout](docs/estado-actual.md#parámetros-inout).

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
cargo run -- examples/conversiones.oki
cargo run -- examples/funciones.oki
cargo run -- examples/retornos_union.oki
cargo run -- examples/variables_union.oki
cargo run -- examples/estructuras.oki
cargo run -- examples/estructuras_campos.oki
cargo run -- examples/lista_enlazada.oki
cargo run -- examples/enums.oki
```

## Documentación para aprender

Leer en este orden:

1. [Estado actual](docs/estado-actual.md): sintaxis, tipos, ejemplos ejecutables y límites.
2. [Funcionamiento interno](docs/funcionamiento-interno.md): recorrido desde los caracteres hasta la salida, pasando por el árbol de sintaxis y la comprobación de tipos.
3. [Historial de desarrollo](docs/historial.md): avances, motivos y comprobaciones.
4. [Hoja de ruta](docs/hoja-de-ruta.md): propuestas de trabajo para avances futuros.

[AGENTS.md](AGENTS.md) contiene las pautas de trabajo y establece cómo mantener esta documentación con cada avance.

## Estructura

Los programas de ejemplo están en [examples/](examples/), al mismo nivel que `src/`.

Seguimos el intérprete de árbol de [Crafting Interpreters](https://craftinginterpreters.com/contents.html), adaptándolo a Rust y a las decisiones de OkitsuLang. No hay dependencias externas.

| Archivo | Responsabilidad |
| --- | --- |
| [src/main.rs](src/main.rs) | Lectura del archivo, coordinación de etapas y pruebas. |
| [src/enum_tests.rs](src/enum_tests.rs) | Pruebas de enums, variantes con datos y match exhaustivo. |
| [src/structure_tests.rs](src/structure_tests.rs) | Pruebas de campos constantes, valores por defecto, uniones y estructuras recursivas. |
| [src/scanner.rs](src/scanner.rs) | Reconocer tokens y sus líneas (capítulo 4). |
| [src/parser.rs](src/parser.rs) | Definir el AST y construirlo mediante análisis descendente (capítulos 5, 6 y 8). |
| [src/value.rs](src/value.rs) | Representar tipos, valores básicos, arrays, estructuras y enums, y su impresión (capítulo 7). |
| [src/type_checker.rs](src/type_checker.rs) | Comprobar nombres y tipos antes de ejecutar; adaptación propia para el tipado estricto. |
| [src/interpreter.rs](src/interpreter.rs) | Evaluar el AST y guardar los valores de las variables (capítulos 7 y 8). |
| [src/stdlib.rs](src/stdlib.rs) | Habilitar por separado `Array` y `Casting`, resolver llamadas y ejecutar `len`, `push` y `pop` de arrays. |
| [src/stdlib/casting.rs](src/stdlib/casting.rs) | Comprobar los pares de tipos convertibles y transformar valores, validando texto, rangos y Unicode. |

El libro usa tipado dinámico en Lox; OkitsuLang exige anotaciones de tipo y compatibilidad sin conversiones implícitas; una unión enumera las alternativas permitidas. Solo se implementa el fragmento descrito en la documentación: hay funciones propias, con o sin valor de retorno, y llamadas a la biblioteca estándar, pero todavía no hay máquina virtual.

## Comprobaciones

```sh
cargo fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
```
