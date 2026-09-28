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

## Conversiones explícitas: std::Casting

`import std::Casting;` habilita `int(valor)`, `float(valor)`, `bool(valor)`, `char(valor)` y `string(valor)`, además de `valor.cast(tipo)`. Las dos formas tienen las mismas reglas y devuelven un valor nuevo del tipo indicado; no modifican la variable original.

```oki
import std::Casting;
int edad = 25;
float numero = 19.95;
println(float(edad) + 0.5);
println("Edad: " + string(edad));
println(numero.cast(int));
println(numero);
println("42".cast(int) + 8);
```

Imprime `25.5`, `Edad: 25`, `19`, `19.95` y `50`, en líneas separadas. La conversión de `float` a `int` **trunca hacia cero**: elimina la parte decimal, de modo que `int(-2.9)` da `-2`.

### Formas de llamada e importación

| Forma | Requisito |
| --- | --- |
| `float(edad)` o `edad.cast(float)` | `import std::Casting;` anterior. |
| `std::Casting::float(edad)` | La misma importación anterior. |
| `Casting::float(edad)` | Además, `use std::Casting;` anterior. |

Las mismas formas sirven para los cinco tipos básicos. Las directivas son globales, llevan `;` y siguen las reglas de `Array`: `use` requiere `import`, repetirlas es válido y cada archivo empieza sin bibliotecas habilitadas. Las dos bibliotecas se habilitan por separado. `Casting` y `cast` no son palabras reservadas; los tipos siguen siéndolo. Los nombres de bibliotecas distinguen mayúsculas.

La función recibe exactamente una expresión; `.cast(...)` recibe exactamente un **tipo escrito en el código**, sin comillas: `numero.cast(int)`, no `numero.cast("int")` ni una variable que contenga el nombre del tipo. Se admiten constantes, expresiones agrupadas, elementos de array y llamadas encadenadas: `(edad + 1).cast(float).cast(string)`. También `25.cast(float)` funciona: el punto inicia un método porque no va seguido de un dígito.

La conversión se puede usar donde se necesita una expresión, incluidos elementos de arrays, condiciones y argumentos de `push`. También puede escribirse como instrucción con `;` para descartar el resultado. Su valor de entrada se evalúa una sola vez, respetando los efectos de `pop` y el cortocircuito de `&&` y `||`.

### Conversiones admitidas

| Origen → destino | Regla |
| --- | --- |
| Tipo básico → el mismo tipo | Conserva el valor. |
| `int` → `float` | Convierte a `f64`; los enteros grandes pueden perder precisión por redondeo. |
| `float` → `int` | Trunca hacia cero y rechaza resultados fuera del rango de `int`. |
| Cualquier tipo básico → `string` | Usa el mismo texto que `print`: `string(1.0)` es `"1.0"`, `string(true)` es `"true"`, `string('ñ')` es `"ñ"`. |
| `string` → `int` | Entero decimal con signo `+`/`-` opcional y al menos un dígito ASCII; sin espacios, decimales, exponentes ni separadores. Valida el rango de 64 bits. |
| `string` → `float` | Texto decimal con signo y exponente opcionales; admite `"25"`, `".5"`, `"1."` y `"2.5e1"`. No admite espacios, coma decimal, prefijos hexadecimales ni separadores. Rechaza NaN, infinito y desbordamientos; números muy pequeños pueden redondearse a cero. |
| `string` → `bool` | Solo `"true"` o `"false"`, exactamente y sin espacios. |
| `string` → `char` | Exactamente un valor escalar Unicode; rechaza el texto vacío o con varias unidades, incluido `"é"` (letra y acento combinante). |
| `char` → `int` | Devuelve su número Unicode: `int('ñ')` es `241`; `int('7')` es `55`, no `7`. |
| `int` → `char` | Interpreta un número Unicode entre 0 y 1114111, excluyendo 55296–57343, que no son valores escalares válidos. |

Los demás pares no están admitidos: por ejemplo, `bool(1)`, `int(true)` y `float('a')` son errores de tipos. Casting tampoco convierte arrays completos, ni siquiera a `string` o al mismo tipo de array. Para convertir elementos, se llama a la conversión sobre cada uno.

El tipado sigue siendo estricto: `float x = 1;`, `edad = float(edad);` si `edad` es `int`, y `"Edad: " + edad` siguen siendo errores. La conversión explícita debe producir el tipo requerido en ese lugar.

### Errores de conversión

Las rutas, importaciones, sintaxis y pares de tipos inválidos se rechazan antes de ejecutar cualquier instrucción. El contenido se valida durante la ejecución: `int("hola")`, `char("ab")` o `int(9223372036854775808.0)` fallan al evaluar la conversión. Este último valor es 2 elevado a 63, fuera del rango de `int`: se rechaza sin saturarlo al máximo.

Los errores señalan la línea del nombre de la función o de `cast`. Conservan la salida previa y detienen las instrucciones posteriores. Por ejemplo:

```oki
import std::Casting;
println("previo");
println(int("hola"));
```

Imprime `previo` y después informa `Línea 3: No se puede convertir de string a int: se esperaba texto entero decimal sin espacios.`. Un error al evaluar la entrada, como `int(1 / 0)`, conserva la línea y el mensaje de esa operación. Las conversiones de ramas omitidas no se ejecutan, pero sus tipos sí se comprueban.

El ejemplo [conversiones.oki](../examples/conversiones.oki) imprime `25.0`, `Edad: 25`, `19`, `19.95`, `50`, `true`, `ñ`, `241`, `25.0` y `25`, cada valor en su línea.

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
- La reasignación tiene la forma `nombre = expresión;`, `nombre[índice] = expresión;` para un elemento de array o `nombre.campo = expresión;` para un campo de estructura. Solo se permite para variables ya declaradas sin `const` y conserva su tipo. No es una declaración nueva.
- El valor debe pertenecer al tipo declarado tanto al declarar como al reasignar; una unión permite cualquiera de sus alternativas. No se convierte automáticamente entre `int` y `float`, entre `char` y `string`, ni entre ningún otro par de tipos.
- Las variables deben declararse antes de usarlas. No se permite `int x = x;`, ni declarar dos veces el mismo nombre.
- Existe un ámbito global por archivo y, dentro de él, cada bloque `{ ... }` de un `if` abre un ámbito propio. La búsqueda de un nombre empieza en el bloque actual y continúa hacia fuera. Cada ejecución empieza vacía.
- Los nombres empiezan por letra ASCII o `_`, y continúan con letras ASCII, dígitos o `_`. Distinguen mayúsculas de minúsculas.
- `int`, `float`, `bool`, `char`, `string`, `const`, `if`, `else`, `while`, `for`, `foreach`, `break`, `continue`, `function`, `struct`, `type`, `return`, `inout`, `in`, `import`, `use`, `true`, `false`, `print` y `println` son palabras reservadas. Nombres como `int2`, `println2`, `func` o `funcion` sí se permiten.

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


## Variables unión y comprobación de tipos

Una variable puede admitir varios tipos explícitos separados por `||`. La anotación permanece fija, pero su valor puede cambiar entre esas alternativas:

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
bool es_entero = type resultado == int;
println(es_entero);
```

Imprime `6`, `test!` y `false`. El ejemplo [variables_union.oki](../examples/variables_union.oki) muestra además cómo guardar un retorno unión de una función.

- Se admiten tipos básicos, arrays y estructuras, incluidos `const int || string fijo = 1;` e `int[] || string datos = [];`. No hay conversiones implícitas. El inicializador y cada asignación deben tener todos sus tipos posibles incluidos en la declaración. Se pueden copiar uniones iguales o más pequeñas, aunque se escriban en distinto orden; las copias son independientes.
- `type nombre == tipo` y `type nombre != tipo` producen `bool`. Se comprueba el tipo **actual del valor**, no la lista de alternativas declarada. `type` es una palabra reservada nueva. Recibe un nombre de variable o una ruta de campos como `dato.valor`, no una llamada, un literal ni una ruta con índices como `datos[0]`; a la derecha se escribe un tipo concreto, básico, array o estructura, sin comillas. También se pueden comprobar variables de un solo tipo.
- En `if (type resultado == int)`, la rama verdadera conoce `resultado` como `int`. En el `else`, se excluye `int`: para `int || string`, queda `string`; para `int || string || bool`, quedan `string || bool` y hace falta otra comprobación. Esto se llama **refinamiento**: reducir temporalmente los tipos posibles a partir de una condición.
- Funcionan `else if`, la negación `!`, `&&` y `||`. El lado derecho de `&&` utiliza lo sabido cuando la izquierda es verdadera, y el de `||`, cuando es falsa. Por ejemplo, `type resultado == int && resultado > 3` es válido. Al reunir caminos distintos se incluyen los tipos posibles de cualquiera de ellos; el análisis no intenta demostrar todas las equivalencias lógicas. Se comprueban también las ramas imposibles, sin saltarse errores.
- La prueba puede usarse en `println`, en una variable `bool`, en un retorno o en condiciones de `while` y `for`. El cuerpo de esos bucles recibe lo conocido cuando la condición es verdadera. Guardar la prueba en un `bool` no conserva la relación con la variable original: `if (es_entero)` no refina `resultado`.
- Dentro de una rama con tipo concreto, se permiten sus operaciones habituales, conversiones de Casting, copia a una variable de ese tipo, argumentos por valor y retornos. Si es un array, se permiten índices, `foreach`, `len`, `push` y `pop` con las reglas habituales, incluidas importaciones y constantes. Sin una comprobación suficiente, una unión no admite operaciones, comparaciones, conversión ni usarse directamente como condición o elemento de array.
- Una asignación completa como `resultado = "otro";` se valida contra la **unión declarada** y descarta el refinamiento previo. Incluso tras `resultado = 1;`, vuelve a ser necesaria una prueba para operar como entero: todavía no se deduce el tipo a partir de asignaciones. `++`, `--`, `+=`, `-=`, las escrituras de elementos y los métodos de arrays conservan el tipo comprobado. Las ramas tienen ámbitos propios: declarar otro `resultado` dentro no cambia el exterior.
- Antes de comprobar un bucle, se descartan las conclusiones sobre nombres reasignados en su cuerpo o actualización, porque otra vuelta podría encontrar otro tipo. Se vuelve a aplicar lo demostrado por la condición. Este análisis es conservador: puede pedir otra prueba incluso cuando la reasignación corresponde a una variable interior del mismo nombre. No se deduce un tipo nuevo al salir del bucle ni a partir de `return`, `break` o `continue`.
- Los arrays vacíos conservan el tipo del elemento: un `int[]` sigue dando `true` en `type datos == int[]` después de quitar su último elemento. Los arrays anidados conservan también cada nivel. `int[] || string[] datos = [];` es ambiguo; declara primero un array concreto y copia su valor a la unión.
- Los parámetros y la variable de `foreach` siguen declarando un solo tipo. Una variable declarada unión no puede pasarse mediante `inout` a un parámetro concreto, aunque esté refinada dentro de un `if`; se conserva la exigencia de que coincida el tipo declarado del almacenamiento. Sí puedes copiar el valor refinado a una variable concreta y pasar esa copia.


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

- Se admite `const` con `int`, `float`, `bool`, `char`, `string`, estructuras, arrays de estos tipos (también anidados) y uniones de estos tipos. En un array constante tampoco se permite cambiar ningún elemento o subarray. El tipo y el inicializador son obligatorios; todos los tipos posibles del inicializador deben estar permitidos por la anotación.
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

## Estructuras con campos

Una **estructura** es un tipo nombrado que agrupa valores de tipos distintos. Cada campo tiene nombre y un tipo declarado, que puede ser concreto o unión:

```oki
struct Persona {
    string nombre;
    int edad;
}
Persona ana = Persona { edad: 30, nombre: "Ana" };
Persona copia = ana;
copia.edad++;
println(ana);
println(copia.edad);
```

Produce `Persona { nombre: "Ana", edad: 30 }` y `31`. La construcción usa `:` entre cada nombre y su valor, y comas entre campos. La declaración del tipo termina en `}` sin `;`; las declaraciones de campos y variables sí exigen `;`.

- Los tipos se declaran solo en el ámbito global y antes de cualquier uso, también antes de usarlos en una firma de función o en otra estructura. No pueden repetirse ni coincidir con una variable o función global. Los nombres locales pueden ocultar variables globales; la anotación de tipo y la construcción siguen identificando el tipo por su nombre.
- Los campos admiten los cinco tipos básicos, arrays, estructuras anteriores, el propio tipo y uniones con `||`. También admiten `const` y un inicializador opcional `= expresión`. Todos son públicos; no hay modificadores de visibilidad ni métodos propios. Se permiten estructuras vacías: `struct Marca {}` y `Marca {}`. Las reglas de cada ampliación se explican a continuación.
- La construcción exige los campos sin valor por defecto. Un campo con valor por defecto puede omitirse o indicarse explícitamente para sustituir ese valor. No se admiten campos adicionales ni repetidos, ni conversiones implícitas: cada valor debe pertenecer al tipo declarado. Los campos explícitos se pueden escribir en cualquier orden y se evalúan una sola vez, de izquierda a derecha en el orden escrito. No se permite una coma final.
- `persona.edad` lee un campo y puede encadenarse con campos e índices: `grupo.personas[0].edad`. También se leen campos de construcciones o valores devueltos por funciones. Para escribir debe existir una variable raíz modificable: `persona.edad = 31;`, `persona.edad++;` o `persona.edad += 2;`, con las reglas del tipo del campo. No se asignan campos de valores temporales.
- Las copias, asignaciones, parámetros normales y retornos son independientes, incluidos todos los campos y arrays interiores. `const Persona ana = ...;` impide reemplazarla, cambiar sus campos o modificar sus arrays con índices, `push` o `pop`. Una copia sin `const` sí puede cambiar.
- Cada nombre define un tipo distinto: dos estructuras con campos iguales no son intercambiables. Esta regla se llama **identidad nominal**. `==` y `!=` comparan todos los campos por valor entre estructuras del mismo tipo, incluidos arrays y estructuras interiores. El orden usado en la construcción no afecta a la igualdad. No hay operaciones aritméticas ni de orden sobre estructuras completas, ni conversiones de Casting desde o hacia ellas.
- Las estructuras pueden ser elementos de arrays (`Persona[]`), parámetros, retornos y alternativas de variables o retornos unión (`Persona || string`). Para acceder al campo de una variable unión hay que comprobar antes `type dato == Persona`. Los campos también pueden tener una unión, que se comprueba con `type dato.campo == tipo`.
- Las funciones reciben copias salvo con parámetros `inout Persona persona`. Para modificar una global deben recibirla como `inout`; la protección también cubre campos y arrays interiores. `inout` admite la variable completa, no campos como `inout persona.edad`. No se añaden métodos propios.
- Los arrays guardados en campos usan `std::Array` con las reglas existentes: `grupo.personas.push(ana);`, `std::Array::len(grupo.personas)` o, con `use`, `Array::pop(grupo.personas)`. El `foreach` conserva las copias de entrada y de cada elemento.
- Se imprime el nombre seguido de los campos en el orden de declaración: `Persona { nombre: "Ana", edad: 30 }`. Las cadenas y caracteres interiores llevan comillas; se conserva su contenido sin escapes, igual que en arrays. Es un formato para leer, no una garantía de código reutilizable.
- Los errores de tipos, campos ausentes, desconocidos o repetidos impiden ejecutar todo el archivo. Se indica la línea del nombre de campo, o la del nombre de construcción si falta alguno. Los índices fuera de rango y errores al evaluar un campo ocurren en ejecución, conservando la salida y los efectos ya completados. Si otra llamada elimina una posición del destino o cambia el tipo de una estructura o array de la ruta antes de escribir, se vuelve a comprobar y se informa del destino fuera de rango o cambiado de tipo.

El ejemplo [estructuras.oki](../examples/estructuras.oki) combina construcción, copia, retorno, `inout`, arrays interiores y `foreach`. Produce:

```text
Persona { nombre: "Ana", edad: 30 }
31
Luis
Ana
Ana
```

### Campos constantes y valores por defecto

```oki
struct Registro {
    const int id;
    int doble = id * 2;
    int || string valor = 0;
    int[] notas = [];
}
Registro dato = Registro { id: 3 };
println(dato.doble);
if (type dato.valor == int) {
    dato.valor++;
    println(dato.valor);
}
```

Produce `6` y `1`. Solo `id` es obligatorio en esta construcción. Cada instancia recibe su propio array `notas`.

- `const` delante del tipo protege el campo completo: no se sustituye ni se modifican sus campos, elementos o subarrays, tampoco mediante `push` o `pop`. Se aplica aunque la estructura raíz sea modificable y aunque se reciba mediante `inout`. El error señala el campo constante y ocurre antes de ejecutar.
- Un campo `const` admite un valor por defecto o uno explícito al construir. Su anotación se conserva en todas las copias de esa estructura. Leer su array y guardarlo en una variable nueva sí permite modificar esa copia independiente.
- Se puede reemplazar una estructura modificable completa, por ejemplo `dato = Registro { id: 4 };`. Esto construye un valor nuevo con otro `id`; también se puede reemplazar un campo contenedor modificable. `const` por campo impide escribir a través de ese campo, no crear otros valores de su tipo. Una variable raíz `const` sigue impidiendo también esos reemplazos.
- Primero se evalúan todos los campos explícitos en el orden escrito. Después se recorren los campos en el orden de declaración: se usa el valor explícito si existe y, si falta, se calcula su valor por defecto. Un valor por defecto sustituido explícitamente no se evalúa.
- Cada valor por defecto se calcula de nuevo para cada construcción; no se evalúa al declarar el tipo. Puede leer campos anteriores, incluso si eran obligatorios y se proporcionaron explícitamente. No ve campos posteriores ni a sí mismo. Los nombres de campos ocultan globales del mismo nombre.
- Los valores por defecto pueden leer globales y llamar a funciones o bibliotecas declaradas o habilitadas antes del tipo. Las globales se leen con su valor actual al construir. Los nombres locales del llamador no cambian su significado, aunque tengan el mismo nombre. Campos anteriores y globales son de solo lectura dentro de esos inicializadores: no se admiten `inout`, `push` o `pop` sobre ellos. Una función por valor sí puede imprimir o modificar sus copias locales.
- Todos los inicializadores se comprueban al declarar el tipo, aunque después se sustituyan explícitamente o no se construya ninguna instancia. Deben tener un tipo compatible, sin conversiones implícitas. Los arrays vacíos reciben contexto del tipo del campo; una unión de varios tipos array puede exigir una variable concreta si el vacío es ambiguo.
- Si un valor por defecto falla durante una construcción, se conservan los efectos y la salida anteriores, se cierran sus ámbitos y no se guarda una instancia incompleta.

### Campos unión

Un campo `int || string valor` admite un entero o texto. Para operar sobre él hay que distinguir su alternativa actual:

```oki
struct Dato { int || string valor = 0; }
Dato dato = Dato {};
dato.valor = "listo";
if (type dato.valor == int) {
    println(dato.valor + 1);
} else {
    println(dato.valor + "!");
}
```

Produce `listo!`. También se admite `!=`, negación, cortocircuito, condiciones de bucles y rutas anidadas como `type dato.interior.valor == int`.

- Una asignación completa al campo se comprueba contra su unión declarada y descarta el tipo refinado de ese campo y sus descendientes. Reemplazar la estructura contenedora descarta las conclusiones sobre sus campos. Una modificación que conserva el tipo, como `++` o `push`, mantiene el refinamiento salvo otros efectos de sus expresiones.
- Una llamada con `inout dato` puede cambiar cualquiera de sus campos y descarta los refinamientos correspondientes, también entre operandos, argumentos o inicializadores. Cuando se escribe o reenvía un parámetro `inout`, se descartan además conclusiones sobre otros parámetros `inout` y globales porque pueden señalar la misma variable. El análisis es conservador y puede exigir otra prueba aunque la función concreta no cambie ese campo.
- Las ramas reúnen sus tipos posibles. Los bucles olvidan conclusiones sobre campos que se reasignan o estructuras que reciben llamadas con `inout`, antes de comprobar otra vuelta. Las pruebas guardadas en un `bool` y los saltos de control no deducen refinamientos.
- Las pruebas solo admiten un nombre y campos, sin índices ni llamadas. Para inspeccionar un elemento de array se puede copiar a una variable de estructura y comprobar sus campos allí. Los parámetros y elementos de arrays siguen exigiendo un tipo concreto; un campo refinado sí puede pasarse por valor a una función de ese tipo. `inout` mantiene la restricción de variables completas.

### Estructuras recursivas

Una estructura puede contener su propio tipo cuando existe una forma de terminar el valor. Para árboles, un array vacío actúa como terminación:

```oki
struct Nodo { int valor; Nodo[] hijos = []; }
Nodo raiz = Nodo { valor: 1, hijos: [Nodo { valor: 2 }] };
println(raiz.hijos[0].valor);
```

Produce `2`. Los hijos son valores independientes. También se puede representar una cadena con `Enlace || bool siguiente = false`: cada eslabón contiene otro `Enlace` o un booleano de terminación. El uso de `false` es una convención del programa; el tipo también permite `true`.

- El propio nombre del tipo se admite dentro de sus campos; otras estructuras siguen teniendo que declararse antes. No hay declaraciones adelantadas ni recursión mutua entre tipos.
- `struct Nodo { Nodo siguiente; }` se rechaza porque exige otra instancia sin una alternativa que termine. Un campo array permite terminar con `[]`; una unión permite terminar con una alternativa que no sea el propio tipo. No hay valores nulos ni tipos opcionales nuevos.
- Se conservan las copias profundas, la igualdad por contenido, los campos constantes y las reglas de `inout`. No se introducen punteros, referencias compartidas ni ciclos entre instancias: son árboles de valores, aunque el tipo sea recursivo.
- El contenido de un valor tiene un máximo de 100 niveles combinados de estructuras y arrays. Se comprueba al construir y antes de escribir o añadir elementos; también limita los valores que crecen en un bucle. Superarlo produce un error de ejecución, conservando los efectos ya completados.
- Las llamadas a funciones y construcciones simultáneas comparten un límite de profundidad de 100. Un valor por defecto como `Nodo[] hijos = [Nodo {}]` se comprueba por tipos, pero si se usa sin terminación falla en ejecución al alcanzar ese límite. Proporcionar `hijos: []` evita evaluar ese valor por defecto.

El ejemplo [estructuras_campos.oki](../examples/estructuras_campos.oki) reúne estas ampliaciones y produce:

```text
6
5
listo!
Nodo { valor: 1, hijos: [Nodo { valor: 2, hijos: [] }] }
Nodo { valor: 1, hijos: [Nodo { valor: 9, hijos: [] }] }
20
```

### Ejemplo de lista enlazada

Una **lista enlazada** organiza sus elementos como nodos: cada uno guarda un valor y el siguiente nodo. El ejemplo [lista_enlazada.oki](../examples/lista_enlazada.oki) usa `Nodo || bool siguiente = false` para terminar la cadena y una estructura `Lista` cuyo campo `primero` también admite `false`, para representar una lista vacía. En este ejemplo solo se usa `false` como terminación, aunque el tipo permite ambos booleanos.

`insertar_al_inicio(inout Lista lista, int valor)` construye un nodo cuyo `siguiente` es la cabeza anterior y lo guarda en `lista.primero`. Al insertar `30`, `20` y `10`, en ese orden, queda `10 -> 20 -> 30 -> fin`. `imprimir_lista`, `longitud` y `contiene` recorren la cadena con un `while`: antes de acceder a `valor` o `siguiente`, comprueban `type actual == Nodo`. `quitar_primero` sustituye la cabeza por su siguiente nodo y devuelve `false` si la lista estaba vacía.

Son nodos guardados por valor: copiar la lista copia toda su cadena y avanzar el recorrido obtiene una copia de la parte restante. Se aplica el límite de profundidad de valores ya descrito. El ejemplo enseña la organización y las operaciones de una lista; las inserciones y recorridos conservan el coste de las copias profundas del intérprete actual.

Se ejecuta con `cargo run -- examples/lista_enlazada.oki` y produce:

```text
Lista vacía:
fin
false
Lista con tres nodos:
10 -> 20 -> 30 -> fin
Longitud:
3
Contiene 20:
true
Contiene 99:
false
Quitar el primer nodo de la copia:
true
20 -> 30 -> fin
La original conserva sus nodos:
10 -> 20 -> 30 -> fin
```

## Enums y match

Un **enum** es un tipo propio cuyas variantes forman un conjunto cerrado. A diferencia de una unión como `int || string`, cada alternativa tiene un nombre y pertenece al mismo tipo enum.

```oki
enum Estado { Pendiente, Hecho }
Estado estado = Estado::Pendiente;
println(estado);
estado = Estado::Hecho;
println(estado == Estado::Hecho);
match estado {
    Estado::Pendiente => { println("pendiente"); },
    Estado::Hecho => { println("hecho"); }
}
```

Imprime `Estado::Pendiente`, `true` y `hecho`, cada uno en su línea.

- La declaración es global, antes del uso y sin `;` final. Requiere al menos una variante y nombres sin duplicados. El nombre del tipo no puede coincidir con otra estructura, enum, variable o función global. Distintos enums pueden reutilizar nombres de variantes.
- Las variantes siempre se califican con su enum: `Estado::Hecho`. Una variante sin datos se construye y se escribe en patrones sin paréntesis; `Estado::Hecho()` es un error.
- Los enums admiten declaración, asignación, `const`, copias independientes, arrays, campos de estructuras, parámetros por valor o `inout`, retornos y uniones. `type dato == Estado` comprueba el tipo completo; no comprueba la variante.
- `==` y `!=` comparan variante y contenido entre valores del mismo enum. Otro enum sigue siendo un tipo distinto aunque tenga variantes iguales. No admiten aritmética, ordenación ni conversiones de Casting.
- `match expresión { ... }` es una instrucción y no devuelve un valor. Evalúa la expresión una sola vez, exige un enum concreto y ejecuta únicamente la rama de su variante. Una unión debe comprobarse con `type` antes de usar su alternativa enum.
- Cada patrón es `Enum::Variante`, con capturas si tiene datos, seguido de `=>` y un bloque `{ ... }`. Las ramas se separan con comas; se permite una coma final tanto en ramas como en variantes. El `match` completo no lleva `;`; sus instrucciones interiores sí.
- **Exhaustivo** significa que cubre todas las variantes, exactamente una vez. Se rechazan variantes omitidas, repetidas, desconocidas o de otro enum antes de ejecutar, incluso si la entrada conocida seleccionaría una rama válida. También se comprueban los tipos y nombres de todas las ramas.
- Cada rama tiene su ámbito propio: sus declaraciones y capturas no escapan. Puede modificar variables externas conforme a las reglas de `const` e `inout`. `return`, `break` y `continue` se propagan; `match` no es un bucle y no habilita por sí mismo los dos últimos.
- Una función con retorno puede garantizar todos sus caminos mediante un `match` exhaustivo cuando cada rama garantiza un `return` con valor. Si alguna rama puede terminar sin devolver, necesita un retorno posterior.

### Variantes con datos

```oki
enum Resultado { Ok(int valor), Error(string mensaje) }
Resultado resultado = Resultado::Ok(5);
match resultado {
    Resultado::Ok(valor) => { println(valor + 1); },
    Resultado::Error(mensaje) => { println(mensaje); }
}
println(resultado);
```

Imprime `6` y `Resultado::Ok(5)`.

- Cada dato declara un tipo concreto y un nombre: `Datos(int numero, string texto)`. No admite uniones en esos datos, `const` por dato ni valores por defecto. Una variante puede tener varios datos; sus nombres no se repiten dentro de la variante. Los tipos pueden ser básicos, arrays, estructuras o enums ya declarados.
- Se construyen con argumentos por posición, de izquierda a derecha y una sola vez: `Datos::Par(3, "hola")`. Deben coincidir el número y los tipos exactos, sin conversiones implícitas. El tipo declarado da contexto a arrays vacíos.
- Los patrones extraen los datos por posición: `Datos::Par(numero, texto) => { ... }`. Las capturas adquieren sus tipos automáticamente del patrón; esto no cambia la obligación de indicar tipo al declarar variables normales. No necesitan coincidir con los nombres de la definición, pero deben ser únicas dentro de la rama y cubrir todos los datos.
- Las capturas son variables locales modificables que reciben copias profundas. Modificarlas no cambia el enum original, aunque proceda de una constante. Para conservar un cambio se construye y asigna una variante nueva.
- La impresión usa `Enum::Variante` o `Enum::Variante(valor, ...)`, con cadenas y caracteres interiores entre comillas. El límite de profundidad de valores incluye los datos del enum.
- No hay acceso directo a los datos con `.campo`, comodines, guardas, patrones literales o anidados, `match` como expresión, enums genéricos ni enums que se nombren a sí mismos o a tipos futuros. Para datos anidados se usa otro `match` dentro de la rama.

El ejemplo [enums.oki](../examples/enums.oki) incluye ambos tipos de variante, asignación, igualdad, funciones que construyen un enum y capturas. Su salida completa es:

```text
Estado::Pendiente
true
hecho
4
divisor cero
Resultado::Ok(4)
```

`Resultado` es un enum del ejemplo: H05 no añade un tipo genérico predefinido ni las operaciones de errores recuperables de H07.

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

- El tipo de la variable sigue siendo obligatorio: `int[]`, `float[]`, `bool[]`, `char[]`, `string[]` o un array de estructuras como `Persona[]`. Todos los elementos deben coincidir exactamente con su tipo, sin conversiones implícitas. `float[] precios = [1];` es inválido; se escribe `[1.0]`.
- Los elementos pueden ser expresiones y se evalúan de izquierda a derecha. Sin un tipo esperado, como en `println([1, 2]);`, el primer elemento determina el tipo del literal y se comprueban los demás. Esto no permite omitir el tipo de una declaración.
- Un array vacío se escribe `[]` y necesita el contexto de una declaración o asignación: `int[] vacio = [];`, `vacio = [];`. También puede recibir el contexto de un literal exterior cuyo tipo ya se conoce. `println([]);`, `println([[], [1]]);` y comparar una variable directamente con `[]` se rechazan porque ahí no se proporciona el tipo esperado. Se puede declarar el vacío e imprimirlo o comparar dos variables vacías del mismo tipo.
- Se accede con `array[índice]`. El índice debe ser `int` y estar entre `0` y la longitud menos uno. Se admiten expresiones como `numeros[1 + 1]`, lecturas de literales como `[10, 20][0]` y asignaciones `numeros[0] = 10;`. Un nombre declarado seguido de campos o índices puede ser destino de asignación.
- Un índice negativo o mayor o igual que la longitud produce un error durante la ejecución. No se aceptan índices negativos para contar desde el final. La comprobación de límites también se aplica al escribir: no añade elementos ni amplía el array. El error señala la línea del corchete `[` del acceso, conserva la salida previa y detiene las instrucciones posteriores.
- La longitud no forma parte del tipo. Se puede reemplazar todo el array por otro del mismo tipo y distinta longitud: `numeros = [4, 5];`. La biblioteca `std::Array` permite consultar la longitud con `len`, añadir al final con `push` y eliminar el último elemento con `pop`. No se pueden insertar o eliminar posiciones arbitrarias ni extraer intervalos.
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
- Existen `std::Array` y `std::Casting`, respetando mayúsculas y con importaciones independientes. `std`, `Array` y `len` no son palabras reservadas: las rutas con `::` se resuelven aparte de las variables. `import` y `use` sí son palabras reservadas.
- `len()` no recibe argumentos cuando se escribe sobre un array. Las formas `std::Array::len(array)` y `Array::len(array)` reciben exactamente uno. `use` no habilita `len(array)` como función suelta.
- Admite arrays de cualquiera de los tipos actuales, incluidos vacíos declarados, constantes y arrays anidados. Cuenta los elementos del nivel consultado: para `int[][] tabla = [[], [1, 2, 3]];`, `tabla.len()` es `2` y `tabla[1].len()` es `3`.
- La consulta no modifica el array. Refleja su valor en ese momento y se puede usar en condiciones, índices y operaciones: `for (int i = 0; i < numeros.len(); i++) { println(numeros[i]); }`.
- Se admiten receptores que sean expresiones de array, como `[10, 20].len()`, `(numeros).len()` o `tabla[0].len()`. Los vacíos conservan la regla de tipo: `int[] vacio = []; println(vacio.len());` funciona después del `import`, pero `[].len()` y `std::Array::len([])` se rechazan por falta de tipo de elemento.
- Están implementados `len`, `push` y `pop`. Una llamada puede ser una instrucción completa con `;`, descartando su resultado si lo tiene. `len` y `pop` también se usan como expresiones; `push` no devuelve un valor. No hay otros métodos de cadenas aparte de Casting, ni importaciones de archivos, alias personalizados o comodines. Las funciones definidas por el usuario admiten el retorno opcional descrito en su sección.

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

### Añadir y eliminar elementos

`push` añade un elemento al final del array. `pop` elimina el último y devuelve ese elemento, con su tipo original:

```oki
import std::Array;
use std::Array;
int[] numeros = [];
for (int i = 1; i <= 3; i++) {
    numeros.push(i);
}
std::Array::push(numeros, 4);
Array::push(numeros, 5);
int ultimo = numeros.pop();
println(ultimo);
println(Array::pop(numeros));
println(numeros);
```

Imprime `5`, `4` y `[1, 2, 3]` en líneas separadas.

| Operación | Método sobre el array | Llamada por biblioteca | Resultado |
| --- | --- | --- | --- |
| Añadir al final | `numeros.push(4);` | `std::Array::push(numeros, 4);` | No devuelve valor. |
| Eliminar el último | `numeros.pop()` | `std::Array::pop(numeros)` | El elemento eliminado. |

- Las dos formas requieren un `import std::Array;` anterior. Con `use std::Array;` se admite también `Array::push` y `Array::pop`. Igual que con `len`, no se habilitan funciones sueltas `push(numeros, 4)` ni `pop(numeros)`.
- El destino debe ser un array dentro de una variable sin `const`, también un campo o subarray: `tabla[0].push(4);`, `std::Array::pop(tabla[0]);`. Se permiten paréntesis alrededor del destino. No se puede modificar un literal, un array devuelto por otra llamada ni una constante, a ninguna profundidad.
- `push` exige exactamente el tipo del elemento, sin conversiones implícitas. Para `int[][] tabla = [];`, `tabla.push([]);` es válido: el tipo del destino proporciona el contexto del nuevo subarray vacío. Las copias siguen siendo independientes, incluidos los arrays insertados o extraídos.
- `push` se usa como instrucción con `;`. `println(numeros.push(4));` es un error antes de ejecutar. `pop` puede aparecer en un inicializador, una impresión, una operación o como instrucción que descarta el valor: `numeros.pop();`.
- `pop` sobre un vacío produce un error de ejecución en la línea del nombre `pop`, conserva la salida previa y detiene el programa. No devuelve `null` ni un valor por defecto.
- Se resuelve el destino evaluando sus índices una sola vez, de izquierda a derecha, y después se evalúa el elemento de `push`. `numeros.push(numeros.pop());` quita el último y lo vuelve a añadir. Si una llamada dentro de un índice o un argumento elimina parte del destino, se vuelven a comprobar las posiciones antes de escribir. Si ya no existen, se informa de que el destino quedó fuera de rango, en la línea de la variable raíz. Los efectos de llamadas que ya terminaron no se deshacen ante un error posterior.
- `len` refleja la longitud actual. El cortocircuito puede omitir un `pop` en el operando derecho de `&&` o `||`. `foreach` sigue recorriendo una copia tomada al entrar, aunque se añadan o eliminen elementos del original durante el bucle. Las llamadas se permiten en el cuerpo de `for`; su inicialización y actualización conservan las formas de declaración/asignación ya documentadas.

El ejemplo [modificar_arrays.oki](../examples/modificar_arrays.oki) imprime `[1, 2, 3, 4]`, `4`, `3`, `[1, 2]`, `2`, `1` y `0`, en líneas separadas.

## Operaciones básicas

| Tipo de los operandos | Operaciones | Tipo del resultado |
| --- | --- | --- |
| `int` | `+`, `-`, `*`, `/`, `%`; signos unarios `+` y `-`. | `int` |
| `float` | `+`, `-`, `*`, `/`, `%`; signos unarios `+` y `-`. | `float` |
| `bool` | Negación `!`, conjunción `&&` («y»), disyunción `\|\|` («o»). | `bool` |
| `string` | Concatenación `+`. | `string` |
| Cualquiera de los tipos básicos, arrays o estructuras | Igualdad `==` y desigualdad `!=` entre valores del mismo tipo. | `bool` |
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
- El destino puede ser una variable, un campo de estructura o un elemento de array, con los mismos índices que una asignación: `numeros[i + 1] += 2`, `tabla[0][1]--`. El índice debe ser `int` y estar dentro de los límites al ejecutar.
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

Dentro de cualquier bucle, `break;` termina el bucle más interno y `continue;` termina la vuelta actual. En `while`, `continue` vuelve a evaluar la condición; en `for`, ejecuta primero la actualización y después evalúa la condición; en `foreach`, pasa al elemento siguiente. Ambos requieren `;` y fuera de un bucle producen un error de tipos antes de ejecutar.

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

## Funciones propias

Una **función** agrupa instrucciones bajo un nombre y recibe parámetros con tipo. Se escribe con `function`, un nombre, la lista de parámetros entre paréntesis y un bloque. Sin `-> tipo` no devuelve valor; con `-> tipo` devuelve un valor mediante `return`:

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

Imprime `Hola, Ana`, `5` y `10`. La declaración termina en `}` y no lleva `;`; cada instrucción de dentro sí lo lleva.

- `function`, `return` e `inout` son palabras reservadas. La declaración solo se admite en el ámbito global del archivo, fuera de cualquier bloque, como `import` y `use`. Debe aparecer antes de las llamadas que la usan.
- Cada parámetro tiene la forma `tipo nombre` o `inout tipo nombre`; el tipo es obligatorio y puede ser básico, array o estructura. No hay valores por defecto ni parámetros sin tipo, y dos parámetros no pueden repetir nombre.
- El tipo de retorno se escribe tras `->` y admite tipos básicos, arrays, estructuras o una unión de estos separados por `||`. No existe `void`: sin `->` la función no devuelve valor.
- `return expresión;` termina la función y devuelve ese valor, que debe pertenecer a los tipos permitidos tras `->`, sin conversiones implícitas. `return;` sin expresión solo se permite en funciones sin tipo de retorno. `return` solo puede aparecer dentro de una función.
- Una función con `-> tipo` debe devolver un valor en todos los caminos de ejecución. La comprobación es conservadora: vale un `return` directo o un `if`/`else` donde las dos ramas devuelvan; un bucle no garantiza el retorno, aunque sea infinito, porque puede no ejecutarse ninguna vuelta.
- Una función con valor se usa como expresión donde su tipo sea compatible: en una declaración, dentro de `println`, como argumento o dentro de una operación. Los retornos de tipo unión tienen las restricciones indicadas más abajo. Una función sin valor solo se admite como instrucción `nombre(argumentos);`; usarla como expresión es un error antes de ejecutar.
- La llamada debe indicar exactamente el número de argumentos y del tipo declarado, sin conversiones implícitas. Los argumentos se evalúan de izquierda a derecha antes de entrar en la función.
- Cada llamada abre un ámbito propio: los parámetros y las variables declaradas dentro solo existen hasta la llave de cierre. Un parámetro o una variable local puede ocultar un nombre global. Desde la función se pueden leer variables globales ya declaradas, pero no escribir en ellas directamente ni pasarlas como `inout` a otra función. Para modificarlas deben recibirse como parámetros `inout`. La función no ve las variables locales de quien la llama, salvo las recibidas explícitamente como argumentos.
- Los nombres de función son únicos entre sí y no pueden coincidir con una variable global. Una función puede llamar a las funciones declaradas antes que ella y a sí misma (recursión directa); no puede llamar a una función declarada más adelante.
- Cada ejecución limita las llamadas anidadas a 100 para que una recursión sin fin no aborte el proceso. Superar el límite produce un error durante la ejecución, en la línea de la llamada, conservando la salida previa.

Este programa es **inválido** y no imprime nada:

```oki
function duplicar(int a) -> int {
    return a * 2;
}
println("previo");
println(duplicar(1.0));
```

```text
Línea 4: Argumento incompatible para 'duplicar': se esperaba int, se recibió float. No hay conversiones implícitas.
```

Otro error frecuente es no cubrir todos los caminos de una función con valor:

```oki
function f(int n) -> int {
    if (n > 0) {
        return n;
    }
}
```

```text
Línea 1: La función 'f' debe devolver un valor en todos los caminos de ejecución.
```

El ejemplo [funciones.oki](../examples/funciones.oki) declara funciones sin y con valor, con parámetros de tipo básico y de array, modifica una variable global mediante `inout` y usa recursión. Produce:

```text
Hola, Ana
Hola, 世界
10
5
10
9
10
25
3
2
1
```



### Retornos con varios tipos

Una unión enumera los tipos que puede devolver una función. `int || string` significa «entero o texto»: cada llamada devuelve un solo valor, no los dos a la vez.

```oki
function test(int numero) -> int || string {
    if (numero > 3) {
        return numero;
    } else {
        return "test";
    }
}

println(test(5));
println(test(2));
```

Imprime `5` y `test`, cada uno en su línea. El programa está en [retornos_union.oki](../examples/retornos_union.oki).

- La unión se admite después de `->` y en declaraciones de variables y constantes. Sus alternativas pueden ser tipos básicos, arrays o estructuras, por ejemplo `int || string || bool` o `int[] || string`. El orden no afecta a los retornos permitidos y las repeticiones se eliminan: `int || int` equivale a `int`.
- Cada `return` debe devolver un tipo permitido. `return true;` en una función `-> int || string` se rechaza antes de ejecutar. Se sigue exigiendo un valor en todos los caminos; `return;` no sirve para una unión.
- Se puede devolver el resultado de otra función si **todos** sus tipos posibles están permitidos: una llamada de tipo `int || string` puede devolverse desde una función `-> string || int || bool`, pero no desde una `-> int`.
- La llamada conserva el tipo de retorno declarado aunque un argumento concreto permita prever la rama elegida. Por ello `int resultado = test(5);` es inválido: el comprobador ve `int || string`. También se rechaza pasar ese resultado a un parámetro `int`.
- El resultado puede imprimirse, guardarse en una variable unión compatible, descartarse con una llamada como instrucción o reenviarse mediante un retorno compatible. Para operar sobre él, guárdalo en una variable y comprueba su tipo con `type`, como se explica en la sección de variables unión. Los parámetros siguen siendo de tipo concreto.
- Los arrays devueltos conservan un tipo de elemento concreto. En `-> int[] || string`, `return [];` usa `int[]` como contexto. En `-> int[] || string[]`, ese vacío es ambiguo y se rechaza; se puede declarar `int[] vacio = [];` y devolver `vacio`. La misma regla se aplica a vacíos anidados. Un literal no vacío debe encajar en una alternativa y no puede mezclar tipos de elementos.
- Dentro de expresiones, `||` sigue siendo el operador lógico entre booleanos; la posición tras `->` permite distinguir ambos usos.


### Parámetros inout

Un parámetro normal recibe una copia: modificarlo no afecta al argumento original, tampoco si es un array. Un parámetro **inout** da acceso a la variable original y exige la marca en ambos sitios:

```oki
function acumular(inout int destino, int valor) {
    destino += valor;
}
int total = 0;
acumular(inout total, 5);
acumular(inout total, 3);
println(total);
```

Imprime `8`. `destino` es otro nombre para el almacenamiento de `total`: cada escritura cambia el original inmediatamente, sin esperar al final de la función.

- Se admiten los cinco tipos básicos, estructuras y arrays completos, incluidos vacíos y anidados. El tipo debe coincidir exactamente.
- El argumento debe tener la forma `inout nombre`, con una variable modificable ya declarada. Se rechazan constantes, literales, resultados de llamadas, operaciones, nombres entre paréntesis y elementos, subarrays o campos como `inout datos[0]` o `inout persona.edad`.
- `acumular(total, 5);` es inválido porque falta `inout`. También es inválido añadir la marca cuando el parámetro normal no la declara. Estos errores se detectan antes de imprimir o ejecutar nada.
- La función puede reasignar el parámetro, usar `+=`, `-=`, `++` o `--` cuando el tipo lo permita, y modificar elementos o usar `push` y `pop` si recibe un array e importa `std::Array`.
- Se puede reenviar el acceso: una función que recibe `inout int destino` puede llamar a otra con `inout destino`, también durante la recursión. Si se reenvía un parámetro normal, solo se modifica la copia local de esa llamada.
- Se puede pasar la misma variable a varios parámetros `inout`. Todos acceden al mismo almacenamiento y ven las escrituras anteriores; no hay copias independientes entre esos parámetros.
- Los argumentos se procesan de izquierda a derecha: los normales guardan su valor en ese momento; `inout` guarda el acceso a la variable y ve cambios posteriores en ella, incluso si los causa otro argumento de la misma llamada.
- `return` cierra el ámbito de la función, pero conserva los cambios ya realizados. Si se produce un error de ejecución, también se conservan las modificaciones completadas y la salida anterior; no se deshacen operaciones.
- `inout` se reserva para parámetros y argumentos de funciones propias. Las llamadas de biblioteca mantienen su sintaxis actual.

Las funciones pueden seguir leyendo globales, pero cualquier escritura directa en ellas está prohibida, incluida la modificación de elementos y `push`/`pop`. Una global solo se modifica desde una función a través de un parámetro `inout` recibido. Los bloques y bucles del nivel superior sí pueden actualizar variables exteriores como antes.

## Reglas compartidas

| Elemento | Comportamiento |
| --- | --- |
| Impresión | `print(expresión);` o `println(expresión);`, con exactamente una expresión. |
| Salida | `print` no añade salto final; `println` añade uno. Se respeta el orden del archivo. |
| Terminación | Todas las declaraciones, asignaciones, impresiones, directivas `import`/`use`, `return` y saltos `break`/`continue` terminan en `;`. Una declaración `struct`, una `function`, un `if`/`else`, un `while`, un `for` y un `foreach` completos terminan en `}` y no llevan `;`; cada instrucción de su interior sí lo lleva. Un salto de línea no sustituye el `;`. |
| Espacios entre tokens | Se ignoran espacios, tabulaciones, retornos de carro y saltos de línea. |
| Texto entre comillas | Conserva Unicode, espacios, punto y coma y saltos de línea reales. Termina en la siguiente comilla del mismo tipo. |
| Secuencias de escape | No se interpretan, conservando la regla anterior para cadenas. Una barra no escapa comillas. `\n` son dos caracteres y no cabe en un `char`. |
| Archivo vacío | Se acepta y no imprime nada. |
| Errores de análisis, nombres o tipos | Se informa del primer error detectado y su línea, antes de ejecutar ninguna instrucción. |
| Errores de ejecución | Índice de array fuera de rango, división o resto por cero, desbordamiento, contenido o rango inválido en una conversión, profundidad máxima de llamadas/construcciones o valores superada, destino que cambia de tipo durante la evaluación, o fallo de escritura: detienen la ejecución y pueden ocurrir después de emitir parte de la salida. |

## Límites de esta versión

Todavía no hay conversiones de arrays completos, operaciones de bits, potencia, acceso por índice a cadenas ni métodos de cadenas aparte de las conversiones de Casting, ni comentarios. Una función propia solo se admite en el ámbito global, no puede declararse dentro de un bloque ni pasar como valor, y el análisis de retorno es conservador: un bucle no basta para garantizar el retorno. El `for` exige sus tres partes. Las asignaciones abreviadas `+=`, `-=`, `++` y `--` son instrucciones, no expresiones: no se usan dentro de `println` ni como valor de una declaración. No hay prefijos `++x`/`--x` ni el resto de operadores compuestos (`*=`, `/=`, `%=`). No hay un bloque suelto ni una instrucción que declare un ámbito por sí misma más allá de los cuerpos de condiciones, bucles, funciones y ramas de `match`. La asignación es una instrucción; no se permite encadenar `a = b = 1;` ni usarla dentro de `println`.

No existen `var`, `let`, `auto`, `any`, `null`, `void`, alias como `double` o `long`, tipos sin signo, otras colecciones, clases ni enums genéricos. Los tipos definidos por el usuario incluyen enums y estructuras con campos públicos, constantes opcionales por campo, valores por defecto, uniones y recursión mediante arrays o alternativas que terminen, sin métodos ni referencias compartidas. Se dispone de los cinco tipos básicos, estructuras, enums y arrays homogéneos, es decir, de elementos del mismo tipo; las variables, constantes y los retornos de funciones también admiten uniones de estos tipos. `print` y `println` siguen siendo instrucciones reservadas. Las llamadas de biblioteca incluyen `len`, `push` y `pop` de `std::Array` y las conversiones explícitas de `std::Casting`, con las directivas descritas arriba; además existen funciones propias con parámetros tipados, valor de retorno opcional, `return` y recursión directa limitada, descritas más arriba.

La ejecución recorre un árbol de sintaxis. No se genera código máquina ni bytecode y no se han medido prestaciones.

## Qué se comprueba

Las dieciocho pruebas de H05 en [src/enum_tests.rs](../src/enum_tests.rs) cubren el ejemplo, tipos nominales, declaración y variantes válidas, exhaustividad, ámbitos, selección y evaluación única, datos de todos los tipos admitidos, capturas y copias profundas, funciones y `inout`, arrays y estructuras, uniones y refinamientos, retornos y saltos de bucle, sintaxis y errores antes de emitir salida, líneas y cierre de ámbitos ante errores de ejecución, aislamiento y límites de valores y llamadas recursivas.


Las catorce pruebas de ampliaciones de estructuras en [src/structure_tests.rs](../src/structure_tests.rs) comprueban campos `const` profundos, nuevos valores al reemplazar estructuras, valores por defecto por instancia, orden, nombres y solo lectura, contexto de arrays vacíos, campos unión, refinamientos e invalidación por escrituras, llamadas e índices, alias `inout`, sombras y `foreach`, recursión en árboles y cadenas, copias e igualdad, errores y cierre de ámbitos, límites de construcciones y de valores que crecen en bucles, y tipos de destinos cambiados durante la evaluación. Conservan los casos anteriores y añaden `estructuras_campos.oki`.

Las trece pruebas de estructuras verifican el ejemplo `estructuras.oki`, impresión y orden de campos, igualdad nominal y por contenido, estructuras vacías, anidación y copias profundas, arrays interiores y de estructuras, biblioteca Array, `foreach`, funciones por valor y `inout`, ámbitos, retornos y uniones con `type`. Cubren declaraciones y campos inválidos, sintaxis, tipos exactos, constantes, escritura de globales, Casting rechazado, evaluación única y ordenada, cortocircuito, líneas de error y destinos invalidados durante la ejecución.

Las ocho pruebas de Casting verifican el ejemplo `conversiones.oki`, las cuatro formas de llamada para todos los pares admitidos, la evaluación única de la entrada y las constantes, la convivencia con Array, el orden de importación y `use`, el aislamiento, la sintaxis y cantidad de argumentos, el rechazo de conversiones no admitidas y la conservación del tipado estricto. Incluyen límites de `i64`, redondeo de `f64`, Unicode, contenido inválido, cortocircuito, líneas de error y salida previa ante fallos de ejecución.

Las siete pruebas de modificación comprueban `push` y `pop` en las dos sintaxis, las rutas completa y corta, los cinco tipos, arrays anidados y vacíos, copias, constantes, ámbitos, construcción en bucles, `foreach`, orden de evaluación, cortocircuito, errores y destinos invalidados por llamadas interiores. Incluyen `modificar_arrays.oki`.

Las nueve pruebas de biblioteca verifican las tres formas de llamada, importación y nombre corto en orden, aislamiento entre ejecuciones, arrays de todos los tipos, constantes, vacíos y anidados, composición con expresiones y bucles, sintaxis incompleta, métodos y rutas desconocidos, tipos y argumentos incorrectos, líneas de error, cortocircuito y conservación de salida ante errores de ejecución. Incluyen el ejemplo `biblioteca_arrays.oki` y conservan la prueba de `hello.oki`.

Las 174 pruebas de [src/main.rs](../src/main.rs), [src/structure_tests.rs](../src/structure_tests.rs) y [src/enum_tests.rs](../src/enum_tests.rs) conservan los casos de impresión y `hello.oki`, y añaden el ejemplo `tipos.oki`, literales de los cinco tipos, copia y reasignación, rechazo de todas las combinaciones de tipos distintos, declaración obligatoria, uso antes de declarar, duplicados, límites numéricos, notación científica, Unicode, líneas de error y aislamiento entre ejecuciones. También se comprueban el ejemplo `operaciones.oki`, aritmética y signos, precedencia y agrupación, comparaciones de cada tipo, concatenación Unicode, tablas de verdad, cortocircuito, rechazo de mezclas de tipos en todos los operadores binarios, división por cero, desbordamiento y línea del operador. También se comprueban las constantes de los cinco tipos, copias independientes, inicializadores con expresiones, sintaxis incompleta, nombres duplicados, tipos incompatibles y rechazo de reasignaciones (incluido el mismo valor) con línea de error y sin salida parcial. Las nueve pruebas de arrays cubren el ejemplo, los cinco tipos de elementos, vacíos, anidación, lecturas y escrituras con índices, precedencia, copias independientes, protección profunda de constantes, igualdad, mezclas de tipos, sintaxis incompleta, límites negativos y extremos, líneas de error, orden de evaluación y cortocircuito. Las cuatro pruebas de condiciones cubren el ejemplo, la elección de rama con `else if`/`else`, la omisión del `else`, el ámbito propio de cada bloque, la ocultación de nombres, la reasignación de una variable externa, el rechazo de constantes y las condiciones y sintaxis inválidas. Las cinco pruebas nuevas de bucles cubren el ejemplo `bucles.oki`, la repetición de `while`, el orden inicialización-condición-cuerpo-actualización del `for`, la actualización de elementos por índice, el ámbito propio del contador, la ocultación de nombres, el recorrido y la copia de elementos de `foreach` (incluidos arrays anidados y vacíos), la comprobación del tipo de elemento, el rechazo de `for` con constante y las condiciones y sintaxis inválidas de los tres bucles. Se verifica que los errores de análisis y tipos no produzcan salida parcial y que los de ejecución conserven la salida previa. Las seis pruebas nuevas de asignaciones abreviadas cubren el ejemplo `asignaciones.oki`, el incremento y decremento de `int` y `float`, `+=` y `-=` con los tipos admitidos, la actualización de elementos de array (también anidados), el uso en la cabecera del `for`, el rechazo de constantes, las combinaciones de tipos incompatibles, la sintaxis incompleta (incluido el prefijo `++x`, que no se admite) y el desbordamiento en ejecución. Tres pruebas de control de bucles cubren la semántica de `break` y `continue` en `while`, `for` y `foreach`, la actualización del `for` al continuar, el destino en bucles anidados, el cierre de ámbitos y el rechazo antes de ejecutar cuando los saltos aparecen fuera de un bucle.

Las dieciséis pruebas de funciones propias cubren el ejemplo `funciones.oki`, los parámetros de tipo básico y de array, el ámbito local y la ocultación de nombres, la lectura de globales y el rechazo de constantes, el encadenamiento y la exigencia de declaración previa, la aridad y los tipos exactos sin conversiones, el rechazo de la llamada sin valor como expresión, las declaraciones inválidas (duplicados, colisión con variables, parámetros repetidos, funciones dentro de bloques y anotaciones de retorno mal formadas), la recursión directa, el límite de profundidad, el orden de evaluación de los argumentos y la conservación de la salida ante errores de ejecución. Las siete nuevas comprueban el retorno de cada tipo básico, la composición de llamadas con valor en expresiones, el retorno en ramas `if`/`else`, dentro de bucles y en funciones recursivas, la coincidencia exacta del tipo de retorno, la cobertura de todos los caminos, el rechazo de `return` fuera de una función, el `return;` de las funciones sin valor y la convivencia de funciones con y sin retorno.

```sh
cargo fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Para seguir el recorrido, leer [cómo funciona internamente](funcionamiento-interno.md). Para conocer la evolución, consultar el [historial](historial.md).

Las diez pruebas añadidas para `inout` cubren los cinco tipos básicos, arrays completos y sus copias, reasignación y métodos de modificación, marcas obligatorias en ambos sitios, tipos exactos, constantes, sintaxis no admitida, protección de globales, reenvío, recursión, retornos anticipados, ámbitos y ocultación de nombres, varios alias de la misma variable, orden de argumentos, cortocircuito y conservación de cambios y cierre de ámbitos ante errores de ejecución. Se conserva la prueba de `hello.oki`.

Las seis pruebas de retornos de tipo unión cubren el ejemplo `retornos_union.oki`, todas las alternativas básicas, arrays y contexto de vacíos, reenvío de subconjuntos, orden y repetición de tipos, recursión, errores de tipo y caminos sin retorno, restricciones de uso y sintaxis, líneas de error y ausencia de salida ante errores estáticos. También verifican que el `||` lógico siga funcionando.

Las nueve pruebas de variables unión y comprobaciones de tipo cubren el ejemplo `variables_union.oki`, los cinco tipos básicos, copias y subconjuntos, constantes, ramas y argumentos por valor, cortocircuito, negación, ámbitos, reasignaciones, bucles y sus vueltas, arrays vacíos y anidados con métodos, restricciones de `inout`, sintaxis inválida, líneas de error y ausencia de salida ante errores estáticos.
