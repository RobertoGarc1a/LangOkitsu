# Qué está implementado

OkitsuLang lee un archivo UTF-8, comprueba el programa completo y ejecuta declaraciones, asignaciones e instrucciones de impresión. Está escrito en Rust, usa la edición 2024 y no tiene dependencias externas.

## Cómo probarlo

Desde la raíz del proyecto:

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
```

Cargo compila el intérprete y lo ejecuta. El separador `--` hace que la ruta llegue al programa como argumento. Se usa el primer argumento; los adicionales se ignoran. `.oki` es la extensión del proyecto, pero no se comprueba la extensión.

Se conserva [hello.oki](../examples/hello.oki) con el contenido actual del usuario:

```oki
print("Hello ");
println("World");
```

Produce `Hello World` seguido de un salto de línea.

El ejemplo [tipos.oki](../examples/tipos.oki) declara los cinco tipos, imprime sus valores y copia una variable después de reasignarla. Su salida es:

```text
25
19.95
true
ñ
Hola
26
```

## Tipos básicos

| Tipo | Representación | Ejemplos de valores |
| --- | --- | --- |
| `int` | Entero de 64 bits con signo (`i64` en Rust), de −9223372036854775808 a 9223372036854775807. | `0`, `25`, `-7` |
| `float` | Número de coma flotante de 64 bits (`f64`), con precisión aproximada. | `0.0`, `19.95`, `-2.5`, `1e3`, `2.5E-2` |
| `bool` | Valor lógico. | `true`, `false` |
| `char` | Exactamente un valor escalar Unicode entre comillas simples. | `'a'`, `'ñ'`, `'界'`, `'🦀'` |
| `string` | Cadena Unicode entre comillas dobles, de longitud variable. | `"Hola"`, `""` |

Un **valor escalar Unicode** es una unidad de texto como la que representa `char` en Rust. Un símbolo visible puede estar compuesto por varias unidades: una `e` seguida de un acento combinante no cabe en un único `char`; la `é` precompuesta sí. Las cadenas pueden contener varias unidades.

Los números se escriben en decimal. Un literal con punto o exponente es `float`; sin ambos es `int`. Debe haber dígitos antes y después del punto: usar `0.5` y `1.0`, no `.5` ni `1.`. El exponente lleva dígitos y puede tener signo. Los signos unarios `+` y `-` se aplican a literales, variables o expresiones agrupadas, también separados por espacios: `-precio`, `+(2 + 3)`. No se aceptan prefijos hexadecimales ni separadores `_`.

Se rechazan los enteros fuera de rango y los literales float que se convierten en infinito. Los float siguen el redondeo de `f64`: pueden perder precisión y los valores demasiado pequeños pueden redondearse a cero. No hay literales especiales para infinito o NaN. Al imprimir se conserva la distinción `1` frente a `1.0`; se puede utilizar notación científica para float. Los booleanos se imprimen como `true` o `false`; cadenas y caracteres se imprimen sin sus comillas.

## Variables con tipo obligatorio

```oki
int edad = 25;
int copia = edad;
edad = 26;
println(copia);
println(edad);
```

Produce `25` y `26`, cada uno en su línea. La copia guarda el valor de ese momento; las reasignaciones posteriores no la modifican.

- La declaración tiene la forma `tipo nombre = expresión;`, con `const` opcional antes del tipo para impedir reasignaciones. El tipo y el valor inicial son obligatorios; no se crean valores por defecto.
- La reasignación tiene la forma `nombre = expresión;`, o `nombre[índice] = expresión;` para un elemento de array. Solo se permite para variables ya declaradas sin `const` y conserva su tipo. No es una declaración nueva.
- El tipo debe coincidir exactamente tanto al declarar como al reasignar. No se convierte automáticamente entre `int` y `float`, entre `char` y `string`, ni entre ningún otro par de tipos.
- Las variables deben declararse antes de usarlas. No se permite `int x = x;`, ni declarar dos veces el mismo nombre.
- Existe un ámbito global por archivo y, dentro de él, cada bloque `{ ... }` de un `if` abre un ámbito propio. La búsqueda de un nombre empieza en el bloque actual y continúa hacia fuera. Cada ejecución empieza vacía.
- Los nombres empiezan por letra ASCII o `_`, y continúan con letras ASCII, dígitos o `_`. Distinguen mayúsculas de minúsculas.
- `int`, `float`, `bool`, `char`, `string`, `const`, `if`, `else`, `while`, `for`, `foreach`, `in`, `import`, `use`, `true`, `false`, `print` y `println` son palabras reservadas. Nombres como `int2` o `println2` sí se permiten.

Estos fragmentos son **inválidos**:

```oki
edad = 25;
```

Falta declarar `edad` con su tipo.

```oki
float precio = 25;
```

El inicializador es `int`; debe escribirse `25.0` para que sea `float`.

```oki
int edad = 25;
edad = "veinticinco";
```

Una variable `int` no puede recibir un `string`.

## Constantes

Una **constante** es una variable cuyo valor no se puede cambiar después de inicializarla. Se declara con `const tipo nombre = expresión;`:

```oki
int base = 5;
const int limite = base * 2;
base = 6;
int copia = limite;
copia = copia + 1;
println(limite);
println(copia);
```

Produce `10` y `11`, cada uno en su línea. El inicializador se evalúa una sola vez al ejecutar la declaración y puede usar expresiones y nombres anteriores. Cambiar `base` o una copia no cambia `limite`: no es una fórmula que se vuelva a calcular.

- Se admite `const` con `int`, `float`, `bool`, `char`, `string` y arrays de estos tipos, también anidados. En un array constante tampoco se permite cambiar ningún elemento o subarray. El tipo y el inicializador son obligatorios y deben coincidir exactamente.
- Se pueden leer constantes en operaciones, impresiones e inicializadores de otras variables o constantes.
- Cualquier reasignación está prohibida, incluso con el mismo valor: `limite = 10;` y `limite = limite;` son errores. No se puede convertir una variable ya declarada en constante ni volver a declarar el mismo nombre.
- El error señala la línea del nombre asignado y se detecta antes de ejecutar cualquier instrucción del archivo. Las declaraciones sin `const` conservan su comportamiento anterior.

Este programa es **inválido** y no imprime nada:

```oki
const int limite = 10;
println(limite);
limite = 20;
```

```text
Línea 3: No se puede reasignar la constante 'limite'.
```

El ejemplo [constantes.oki](../examples/constantes.oki) usa los cinco tipos y produce:

```text
10
11
19.95
true
ñ
Hola mundo
```

## Arrays

Un **array** es una secuencia ordenada de elementos del mismo tipo. Se añade `[]` al tipo del elemento y se escriben los valores entre corchetes, separados por comas:

```oki
int[] numeros = [1, 2 + 3, 7];
println(numeros);
numeros[0] = numeros[1] * 2;
println(numeros[0]);
int[] copia = numeros;
copia[1] = 99;
println(numeros);
println(copia);
```

Produce `[1, 5, 7]`, `10`, `[10, 5, 7]` y `[10, 99, 7]`, cada uno en su línea.

- El tipo de la variable sigue siendo obligatorio: `int[]`, `float[]`, `bool[]`, `char[]` o `string[]`. Todos los elementos deben coincidir exactamente con su tipo, sin conversiones implícitas. `float[] precios = [1];` es inválido; se escribe `[1.0]`.
- Los elementos pueden ser expresiones y se evalúan de izquierda a derecha. Sin un tipo esperado, como en `println([1, 2]);`, el primer elemento determina el tipo del literal y se comprueban los demás. Esto no permite omitir el tipo de una declaración.
- Un array vacío se escribe `[]` y necesita el contexto de una declaración o asignación: `int[] vacio = [];`, `vacio = [];`. También puede recibir el contexto de un literal exterior cuyo tipo ya se conoce. `println([]);`, `println([[], [1]]);` y comparar una variable directamente con `[]` se rechazan porque ahí no se proporciona el tipo esperado. Se puede declarar el vacío e imprimirlo o comparar dos variables vacías del mismo tipo.
- Se accede con `array[índice]`. El índice debe ser `int` y estar entre `0` y la longitud menos uno. Se admiten expresiones como `numeros[1 + 1]`, lecturas de literales como `[10, 20][0]` y asignaciones `numeros[0] = 10;`. Solo un nombre declarado seguido de índices puede ser destino de asignación.
- Un índice negativo o mayor o igual que la longitud produce un error durante la ejecución. No se aceptan índices negativos para contar desde el final. La comprobación de límites también se aplica al escribir: no añade elementos ni amplía el array. El error señala la línea del corchete `[` del acceso, conserva la salida previa y detiene las instrucciones posteriores.
- La longitud no forma parte del tipo. Se puede reemplazar todo el array por otro del mismo tipo y distinta longitud: `numeros = [4, 5];`. La biblioteca `std::Array` permite consultar la longitud con `.len()`. No hay métodos para añadir o eliminar elementos ni extraer intervalos.
- Las copias son independientes, incluidos todos los arrays interiores. `const` impide tanto sustituir el array como escribir sus elementos, a cualquier profundidad. Una copia declarada sin `const` sí puede cambiar.
- `==` y `!=` comparan contenido, orden y longitud entre arrays del mismo tipo, también anidados. No se admite aritmética, concatenación, lógica ni comparaciones de orden sobre arrays completos.
- Al imprimir se usan corchetes y comas. Dentro del array, cadenas y caracteres llevan comillas: `["Ana", "世界"]`, `['ñ', '🦀']`. Se conserva el texto original, sin introducir escapes; este formato es para lectura y no garantiza poder volver a analizarlo como código si el texto contiene comillas o saltos de línea. Al imprimir un elemento aislado se conserva el formato del tipo básico.
- No se admite una coma final (`[1, 2,]`) ni declarar la longitud con `int[3]`.

### Arrays anidados

Cada `[]` añade un nivel. Los arrays interiores pueden tener longitudes diferentes:

```oki
const int[][] tabla = [[], [1, 2]];
int[][] copia = tabla;
copia[0] = [9];
copia[1][0] = 7;
println(tabla);
println(copia);
```

Produce `[[], [1, 2]]` y `[[9], [7, 2]]`. La declaración proporciona el tipo también a los arrays vacíos interiores. `tabla[1][0] = 7;` sería un error de constante antes de ejecutar.

El ejemplo [arrays.oki](../examples/arrays.oki) produce:

```text
[1, 2, 3]
10
[1, 10, 3]
[99, 10, 3]
["Ana", "世界"]
[]
3
true
```

## Biblioteca estándar: std::Array

La **biblioteca estándar** viene incluida en el intérprete. `std::Array` es la ruta de su módulo de arrays; `::` separa los nombres de esa ruta. No requiere instalar paquetes ni lee archivos externos al importar.

```oki
import std::Array;

int[] numeros = [10, 20, 30];
println(numeros.len());
println(std::Array::len(numeros));

use std::Array;
println(Array::len(numeros));
```

Produce `3`, `3` y `3`, cada uno en su línea. Un **método** se llama sobre un valor: en `numeros.len()`, `numeros` es el receptor de la operación. Las tres formas consultan el mismo dato y devuelven `int`.

| Instrucción | Qué habilita a partir de esa línea |
| --- | --- |
| `import std::Array;` | `numeros.len()` y `std::Array::len(numeros)`. |
| `use std::Array;` | Además, el nombre corto `Array::len(numeros)`. Requiere un `import std::Array;` anterior. |

- Declarar, copiar, indexar, modificar, imprimir y recorrer arrays sigue funcionando sin importaciones. Los métodos sí necesitan importar su biblioteca.
- `import` y `use` solo se admiten en el ámbito global del archivo, fuera de bloques, y llevan `;`. Deben aparecer antes de las llamadas que los necesitan; no se aplican retroactivamente. Repetir una importación o un `use` es válido y no cambia el resultado. Cada ejecución empieza sin bibliotecas habilitadas.
- Solo existe `std::Array`, respetando mayúsculas. `std`, `Array` y `len` no son palabras reservadas: las rutas con `::` se resuelven aparte de las variables. `import` y `use` sí son palabras reservadas.
- `len()` no recibe argumentos cuando se escribe sobre un array. Las formas `std::Array::len(array)` y `Array::len(array)` reciben exactamente uno. `use` no habilita `len(array)` como función suelta.
- Admite arrays de cualquiera de los tipos actuales, incluidos vacíos declarados, constantes y arrays anidados. Cuenta los elementos del nivel consultado: para `int[][] tabla = [[], [1, 2, 3]];`, `tabla.len()` es `2` y `tabla[1].len()` es `3`.
- La consulta no modifica el array. Refleja su valor en ese momento y se puede usar en condiciones, índices y operaciones: `for (int i = 0; i < numeros.len(); i++) { println(numeros[i]); }`.
- Se admiten receptores que sean expresiones de array, como `[10, 20].len()`, `(numeros).len()` o `tabla[0].len()`. Los vacíos conservan la regla de tipo: `int[] vacio = []; println(vacio.len());` funciona después del `import`, pero `[].len()` y `std::Array::len([])` se rechazan por falta de tipo de elemento.
- Solo está implementado `len`. No hay métodos de cadenas ni funciones definidas por el usuario, importaciones de archivos, alias personalizados ni comodines. Una llamada de biblioteca se usa como expresión, por ejemplo dentro de una impresión o un inicializador; no es una instrucción suelta.

Este programa es **inválido** y no imprime nada:

```oki
int[] numeros = [1, 2];
println("previo");
println(numeros.len());
```

```text
Línea 3: El método 'len' requiere un 'import std::Array;' anterior.
```

Las bibliotecas o métodos desconocidos, la falta de `import`/`use`, los argumentos incorrectos y el uso de `len` sobre otro tipo se detectan antes de ejecutar el archivo. Si falla la evaluación del array, por ejemplo en `[1 / 0].len()`, el error sucede durante la ejecución y conserva la salida previa.

El ejemplo [biblioteca_arrays.oki](../examples/biblioteca_arrays.oki) imprime `3` tres veces, después `10`, `20`, `30` y finalmente `0`, cada valor en su línea.

## Operaciones básicas

| Tipo de los operandos | Operaciones | Tipo del resultado |
| --- | --- | --- |
| `int` | `+`, `-`, `*`, `/`, `%`; signos unarios `+` y `-`. | `int` |
| `float` | `+`, `-`, `*`, `/`, `%`; signos unarios `+` y `-`. | `float` |
| `bool` | Negación `!`, conjunción `&&` («y»), disyunción `\|\|` («o»). | `bool` |
| `string` | Concatenación `+`. | `string` |
| Cualquiera de los tipos básicos o arrays | Igualdad `==` y desigualdad `!=` entre valores del mismo tipo. | `bool` |
| `int`, `float`, `char`, `string` | `<`, `<=`, `>`, `>=` entre valores del mismo tipo. | `bool` |

Un operador **unario** recibe un valor, como `-edad`; uno **binario** recibe dos, como `edad + 1`. No hay conversiones implícitas en los operadores: `1 + 2.0`, `1 == 1.0`, `"Hola" + '!'` y `'a' + 'b'` son errores de tipos. `char` admite comparaciones, pero no aritmética ni concatenación. `bool` no admite comparaciones de orden.

- La división de `int` descarta la parte fraccionaria hacia cero: `7 / 2` da `3` y `-7 / 2` da `-3`. `7.0 / 2.0` da `3.5`.
- `%` calcula el **resto** de la división truncada hacia cero, también para float. Su signo, si no es cero, coincide con el dividendo: `-7 % 2` da `-1`, `7 % -2` da `1` y `-7.5 % 2.0` da `-1.5`. `-9223372036854775808 % -1` da `0`.
- Dividir o calcular el resto por cero es un error en ambos tipos, incluido `-0.0`. Se rechaza el desbordamiento de `int`, incluso al negar su mínimo o dividirlo por `-1`. También se rechaza cualquier resultado float no finito. Estos errores se detectan al evaluar y señalan la línea del operador; la salida previa permanece y las instrucciones posteriores no se ejecutan.
- Los float conservan la precisión aproximada de `f64`: pueden redondearse y los resultados muy pequeños pueden pasar a cero. `==` compara el valor almacenado, sin tolerancia; `0.0 == -0.0` es `true`.
- `char` se ordena por su valor escalar Unicode; `string`, lexicográficamente (comparando desde el primer carácter distinto, con el prefijo más corto primero). Se distinguen mayúsculas y minúsculas, sin reglas de ordenación por idioma ni normalización: `"A" < "a"` es `true` y la `é` precompuesta difiere de una `e` con acento combinante.
- `&&` y `||` usan **cortocircuito**: omiten la evaluación de la derecha cuando la izquierda ya decide el resultado. `false && 1 / 0 == 0` produce `false`; `true || 1 / 0 == 0` produce `true`. Aun así, ambas partes deben tener nombres y tipos válidos antes de ejecutar.

### Precedencia y paréntesis

La **precedencia** decide qué operaciones se agrupan primero. De mayor a menor:

1. Agrupación con paréntesis e indexación: `(expresión)`, `array[índice]`. Los accesos se encadenan de izquierda a derecha: `tabla[0][1]`.
2. Unarios: `!`, `-`, `+`.
3. Multiplicación, división y resto: `*`, `/`, `%`.
4. Suma, resta y concatenación: `+`, `-`.
5. Comparación de orden: `<`, `<=`, `>`, `>=`.
6. Igualdad: `==`, `!=`.
7. Conjunción: `&&`.
8. Disyunción: `||`.

Los binarios del mismo nivel se agrupan de izquierda a derecha: `20 - 5 - 3` es `(20 - 5) - 3`. Los unarios se anidan de derecha a izquierda: `--1` da `1`; no es un operador de decremento. `2 + 3 * 4` da `14`, y `(2 + 3) * 4` da `20`. Para comprobar un intervalo se escribe `1 < x && x < 3`; `1 < x < 3` intenta comparar un `bool` con un `int` y se rechaza.

Las expresiones completas pueden usarse como inicializador, valor de una asignación o argumento de impresión. [operaciones.oki](../examples/operaciones.oki) recorre los cinco tipos y produce:

```text
14
20
3
1
3.5
true
true
Hola, mundo
true
false
```

## Asignaciones abreviadas

Además de `nombre = expresión;`, una variable ya declarada se puede modificar con cuatro operadores abreviados. Son instrucciones: llevan `;` y no producen un valor, así que no se usan dentro de `println` ni como inicializador de una declaración.

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

string mensaje = "Hola";
mensaje += ", mundo";
println(mensaje);

int[] numeros = [1, 2, 3];
numeros[0]++;
numeros[1] += 10;
numeros[2]--;
println(numeros);
```

Produce `2`, `12.5`, `Hola, mundo` y `[2, 12, 2]`, cada uno en su línea.

- `nombre += expresión;` equivale a `nombre = nombre + expresión;`, y `nombre -= expresión;` a `nombre = nombre - expresión;`. Siguen las mismas reglas de tipo que `+` y `-`: `+=` admite `int`, `float` y `string` (concatenación); `-=` admite `int` y `float`. `mensaje += 'x';` es un error, igual que `"a" + 'x'`.
- `nombre++;` suma una unidad y `nombre--;` la resta. Solo se admiten sobre `int` o `float`, y la unidad conserva el tipo: en un `int` se suma `1` y en un `float`, `1.0`.
- El destino puede ser una variable o un elemento de array, con los mismos índices que una asignación: `numeros[i + 1] += 2`, `tabla[0][1]--`. El índice debe ser `int` y estar dentro de los límites al ejecutar.
- El nombre debe estar declarado y no puede ser constante. Cualquier forma abreviada sobre una constante se rechaza antes de ejecutar, igual que `limite = 0;`, incluso con el mismo valor o sobre un elemento: `a[0]++` con `const int[] a = [1];` falla.
- No hay prefijo `++x`/`--x`, ni `*=`, `/=`, `%=` ni encadenamientos como `a += b -= 1;`. La actualización de un `for` admite estas formas: `for (int i = 0; i < 3; i++) { ... }`.
- El desbordamiento de `int` y los resultados float no finitos se detectan al ejecutar y señalan la línea del operador, conservando la salida previa.

El ejemplo [asignaciones.oki](../examples/asignaciones.oki) usa las cuatro formas sobre variables y elementos de array y produce:

```text
2
7
4
3
12.5
11.5
Hola, mundo
[2, 12, 2]
012
```

## Control de flujo: if y else

Un **if** elige qué bloque de instrucciones ejecutar según una condición de tipo `bool`. Las ramas van entre llaves y el `if` completo no lleva `;` final; cada instrucción de dentro sí lo lleva:

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

string categoria = "desconocida";
if (edad >= 18) {
    categoria = "adulto";
}
println(categoria);
```

Produce `mayor de edad`, `adultez` y `adulto`, cada uno en su línea.

- La condición es una expresión cualquiera y debe ser `bool`. `if (1) { ... }` es un error de tipos antes de ejecutar; también lo es usar un nombre no declarado en la condición.
- La rama del `if` es obligatoria y va siempre entre `{` y `}`. El `else` es opcional y su rama también es un bloque. Se encadena con `else if (...) { ... }`.
- No se pueden omitir los paréntesis ni las llaves: `if true { ... }` e `if (true) println(...);` se rechazan.
- Cada bloque abre un **ámbito** propio. Un nombre declarado dentro solo existe hasta el `}`; usarlo después es un error. Se puede declarar de nuevo el mismo nombre en otro bloque (queda oculto mientras dura el interior). Las variables declaradas fuera siguen visibles dentro y se pueden reasignar, salvo las constantes.
- La comprobación de tipos recorre las dos ramas antes de ejecutar, aunque solo se vaya a ejecutar una. Un error en la rama no elegida impide ejecutar el programa.
- El ejemplo [condiciones.oki](../examples/condiciones.oki) usa la condición, la cadena `else if`/`else`, la reasignación de una variable externa y la ocultación de un nombre. Produce:

```text
mayor de edad
adultez
2026
20
adulto
5
20
```

## Bucles

Un **bucle** repite un bloque de instrucciones. Hay tres formas: `while`, `for` y `foreach`. En las tres el cuerpo va siempre entre llaves, el bucle completo no lleva `;` final y cada vuelta abre y cierra el ámbito del cuerpo.

### while

`while (condición) { ... }` repite el bloque mientras la condición sea `bool`:

```oki
int i = 3;
while (i > 0) {
    println(i);
    i = i - 1;
}
println("fin");
```

Produce `3`, `2`, `1` y `fin`, cada uno en su línea. La condición se evalúa antes de cada vuelta; si la primera vez es `false`, el cuerpo no se ejecuta. Si la condición no es `bool`, es un error antes de ejecutar: `while (1) { ... }` señala la línea del `while`.

### for

`for (inicialización; condición; actualización) { ... }` reúne las tres partes en la cabecera, separadas por `;`. La inicialización admite una declaración (`int i = 0`) o una asignación a una variable ya declarada (`i = 0`); la condición es una expresión que debe ser `bool`; la actualización es una asignación, una asignación abreviada o un incremento. Las tres son obligatorias, y el avance puede escribirse `i = i + 1`, `i += 1` o `i++`:

```oki
for (int i = 0; i < 3; i++) {
    println(i);
}
```

Produce `0`, `1` y `2`. El orden de ejecución es: la inicialización una sola vez y, en cada vuelta, la condición, el cuerpo y la actualización. La variable declarada en la inicialización solo existe dentro del `for`, condición y actualización incluidas; fuera, el nombre no está declarado. Se puede usar otro contador con el mismo nombre fuera del bucle sin conflicto.

### foreach

`foreach (tipo nombre in array) { ... }` recorre los elementos de un array, del primero al último. El tipo declarado debe coincidir exactamente con el de los elementos, que no se convierten:

```oki
int[] numeros = [10, 20, 30];
int total = 0;
foreach (int n in numeros) {
    total = total + n;
}
println(total);
```

Produce `60`. El array se evalúa una sola vez, al empezar. En cada vuelta `n` guarda una copia del elemento: modificar `n` o el array original dentro del cuerpo no cambia los elementos que quedan por recorrer. Un array vacío no ejecuta ninguna vuelta. También se pueden recorrer arrays cuyos elementos son a su vez arrays, por ejemplo `foreach (int[] fila in tabla)`.

- `foreach` recorre únicamente arrays. Una expresión de otro tipo, como `foreach (int n in 1)`, es un error antes de ejecutar.
- La variable del bucle es una variable normal dentro del cuerpo: se puede leer y reasignar, pero nunca es constante. No se puede declarar el mismo nombre en el mismo nivel que el bucle.
- El cuerpo tiene su propio ámbito, igual que una rama de `if`: un nombre declarado dentro no existe fuera y puede ocultar a un nombre exterior.

El ejemplo [bucles.oki](../examples/bucles.oki) combina `while`, dos `for` anidados y `foreach`:

```text
15
11
12
21
22
Ana
Luis
Mar
20
```

## Reglas compartidas

| Elemento | Comportamiento |
| --- | --- |
| Impresión | `print(expresión);` o `println(expresión);`, con exactamente una expresión. |
| Salida | `print` no añade salto final; `println` añade uno. Se respeta el orden del archivo. |
| Terminación | Todas las declaraciones, asignaciones, impresiones y directivas `import`/`use` terminan en `;`. Un `if`/`else`, un `while`, un `for` y un `foreach` completos terminan en `}` y no llevan `;`; cada instrucción de su interior sí lo lleva. Un salto de línea no sustituye el `;`. |
| Espacios entre tokens | Se ignoran espacios, tabulaciones, retornos de carro y saltos de línea. |
| Texto entre comillas | Conserva Unicode, espacios, punto y coma y saltos de línea reales. Termina en la siguiente comilla del mismo tipo. |
| Secuencias de escape | No se interpretan, conservando la regla anterior para cadenas. Una barra no escapa comillas. `\n` son dos caracteres y no cabe en un `char`. |
| Archivo vacío | Se acepta y no imprime nada. |
| Errores de análisis, nombres o tipos | Se informa del primer error detectado y su línea, antes de ejecutar ninguna instrucción. |
| Errores de ejecución | Índice de array fuera de rango, división o resto por cero, desbordamiento o fallo de escritura: detienen la ejecución y pueden ocurrir después de emitir parte de la salida. |

## Límites de esta versión

Todavía no hay conversión explícita de tipos, operaciones de bits, potencia, acceso por índice a cadenas ni métodos de cadenas, comentarios ni funciones definidas por el usuario. Los bucles `while`, `for` y `foreach` no admiten `break` ni `continue`, y el `for` exige sus tres partes. Las asignaciones abreviadas `+=`, `-=`, `++` y `--` son instrucciones, no expresiones: no se usan dentro de `println` ni como valor de una declaración. No hay prefijos `++x`/`--x` ni el resto de operadores compuestos (`*=`, `/=`, `%=`). No hay un bloque suelto ni una instrucción que declare un ámbito por sí misma más allá del cuerpo de un `if` o de un bucle. La asignación es una instrucción; no se permite encadenar `a = b = 1;` ni usarla dentro de `println`.

No existen `var`, `let`, `auto`, `any`, `null`, `void`, alias como `double` o `long`, tipos sin signo, otras colecciones, clases ni tipos definidos por el usuario. Se dispone de los cinco tipos básicos y arrays homogéneos, es decir, de elementos del mismo tipo. `print` y `println` siguen siendo instrucciones reservadas. Las llamadas de biblioteca se limitan a `std::Array::len`, `Array::len` y `.len()` con las directivas descritas arriba; no existe un sistema general de funciones.

La ejecución recorre un árbol de sintaxis. No se genera código máquina ni bytecode y no se han medido prestaciones.

## Qué se comprueba

Las nueve pruebas de biblioteca verifican las tres formas de llamada, importación y nombre corto en orden, aislamiento entre ejecuciones, arrays de todos los tipos, constantes, vacíos y anidados, composición con expresiones y bucles, sintaxis incompleta, métodos y rutas desconocidos, tipos y argumentos incorrectos, líneas de error, cortocircuito y conservación de salida ante errores de ejecución. Incluyen el ejemplo `biblioteca_arrays.oki` y conservan la prueba de `hello.oki`.

Las 70 pruebas de [src/main.rs](../src/main.rs) conservan los casos de impresión y `hello.oki`, y añaden el ejemplo `tipos.oki`, literales de los cinco tipos, copia y reasignación, rechazo de todas las combinaciones de tipos distintos, declaración obligatoria, uso antes de declarar, duplicados, límites numéricos, notación científica, Unicode, líneas de error y aislamiento entre ejecuciones. También se comprueban el ejemplo `operaciones.oki`, aritmética y signos, precedencia y agrupación, comparaciones de cada tipo, concatenación Unicode, tablas de verdad, cortocircuito, rechazo de mezclas de tipos en todos los operadores binarios, división por cero, desbordamiento y línea del operador. También se comprueban las constantes de los cinco tipos, copias independientes, inicializadores con expresiones, sintaxis incompleta, nombres duplicados, tipos incompatibles y rechazo de reasignaciones (incluido el mismo valor) con línea de error y sin salida parcial. Las nueve pruebas de arrays cubren el ejemplo, los cinco tipos de elementos, vacíos, anidación, lecturas y escrituras con índices, precedencia, copias independientes, protección profunda de constantes, igualdad, mezclas de tipos, sintaxis incompleta, límites negativos y extremos, líneas de error, orden de evaluación y cortocircuito. Las cuatro pruebas de condiciones cubren el ejemplo, la elección de rama con `else if`/`else`, la omisión del `else`, el ámbito propio de cada bloque, la ocultación de nombres, la reasignación de una variable externa, el rechazo de constantes y las condiciones y sintaxis inválidas. Las cinco pruebas nuevas de bucles cubren el ejemplo `bucles.oki`, la repetición de `while`, el orden inicialización-condición-cuerpo-actualización del `for`, la actualización de elementos por índice, el ámbito propio del contador, la ocultación de nombres, el recorrido y la copia de elementos de `foreach` (incluidos arrays anidados y vacíos), la comprobación del tipo de elemento, el rechazo de `for` con constante y las condiciones y sintaxis inválidas de los tres bucles. Se verifica que los errores de análisis y tipos no produzcan salida parcial y que los de ejecución conserven la salida previa. Las seis pruebas nuevas de asignaciones abreviadas cubren el ejemplo `asignaciones.oki`, el incremento y decremento de `int` y `float`, `+=` y `-=` con los tipos admitidos, la actualización de elementos de array (también anidados), el uso en la cabecera del `for`, el rechazo de constantes, las combinaciones de tipos incompatibles, la sintaxis incompleta (incluido el prefijo `++x`, que no se admite) y el desbordamiento en ejecución.

```sh
cargo fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Para seguir el recorrido, leer [cómo funciona internamente](funcionamiento-interno.md). Para conocer la evolución, consultar el [historial](historial.md).
