# Cómo funciona internamente

Cada etapa resuelve una tarea distinta: reconocer las piezas del texto, comprobar su estructura, verificar nombres y tipos, y ejecutar el resultado. La implementación se ha dividido en módulos para mantener visibles estas responsabilidades.

```text
archivo.oki
    │ main.rs: run_file() lee el archivo
    ▼
texto fuente
    │ scanner.rs: Scanner reconoce tokens
    ▼
tokens
    │ parser.rs: Parser construye el árbol
    ▼
AST (árbol de sintaxis)
    │ type_checker.rs: TypeChecker comprueba nombres y tipos
    ▼
AST validado
    │ interpreter.rs: Interpreter guarda valores y ejecuta
    ▼
salida de texto
```

El AST validado es el mismo árbol: la comprobación no lo modifica ni genera otro programa.

## 1. Leer y coordinar

[main.rs](../src/main.rs) conserva `main()`, `run_file()` y las pruebas. `run_file()` obtiene la ruta mediante `env::args_os().nth(1)`, lee el archivo con `fs::read_to_string` y llama a `run()` con el texto y la salida estándar.

```rust
let tokens = Scanner::new(source).scan_tokens()?;
let statements = Parser::new(tokens).parse()?;
TypeChecker::default().check(&statements)?;
Interpreter::new(output).interpret(&statements)?;
```

El operador `?` devuelve inmediatamente un error si una etapa falla. El intérprete solo empieza cuando se han analizado todas las instrucciones y comprobado todos sus tipos. Por eso una incompatibilidad al final del archivo impide también imprimir las instrucciones anteriores.

## 2. Scanner: de caracteres a tokens

Un **token** es una pieza del lenguaje: una palabra reservada, un nombre, un literal o un signo. [scanner.rs](../src/scanner.rs) define `TokenKind`, `Token` y `Scanner`. Cada token conserva la línea donde empieza.

Usaremos este programa como ejemplo:

```oki
int edad = 25;
println(edad);
```

Los tokens son:

| Texto | Token | Línea |
| --- | --- | --- |
| `int` | `Type(Int)` | 1 |
| `edad` | `Identifier("edad")` | 1 |
| `=` | `Equal` | 1 |
| `25` | `Number("25")` | 1 |
| `;` | `Semicolon` | 1 |
| `println` | `Println` | 2 |
| `(` | `LeftParen` | 2 |
| `edad` | `Identifier("edad")` | 2 |
| `)` | `RightParen` | 2 |
| `;` | `Semicolon` | 2 |
| Fin del archivo | `Eof` | 2 sin salto final; 3 con él |

`Eof` es una marca añadida, no un texto del archivo. `Scanner` recorre caracteres Unicode con `Peekable<Chars>`: puede mirar el siguiente carácter antes de consumirlo. Incrementa el contador al encontrar saltos de línea, también dentro de las comillas.

`identifier()` lee primero el nombre completo y después reconoce las palabras reservadas. Así distingue `int` de `int2` y `println` de `println2`. Reconoce `const` como el token `Const`, los cinco tipos y los booleanos `true` y `false`, además de las instrucciones de impresión.

`quoted()` recoge texto hasta la siguiente comilla del mismo tipo. Para comillas dobles produce un `Value::String`; para comillas simples exige exactamente un valor escalar Unicode y produce `Value::Char`. No procesa escapes, conservando el comportamiento previo de las cadenas.

`number()` reconoce dígitos, una parte decimal opcional y un exponente opcional. Guarda el texto en `Number` para que `number()` del parser lo convierta junto con el posible signo. El signo `-` es un token separado. Esta decisión permite aceptar `-9223372036854775808`: su magnitud positiva no cabe en `i64`, pero el número completo sí.

El scanner reconoce además `[` (`LeftBracket`), `]` (`RightBracket`) y `,` (`Comma`), para tipos de array, literales y accesos. El parser decide qué función cumplen según dónde aparezcan.

El scanner también reconoce los operadores `+`, `-`, `*`, `/`, `%`, `!`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `&&` y `||`. `paired()` mira si un signo lleva un segundo `=`: así distingue la asignación `=` de la igualdad `==`. Un `&` o `|` aislado se rechaza. El signo del exponente sigue perteneciendo al número (`1e-2`), mientras que en `2-1` el menos es un token independiente.

El scanner comprueba la forma del literal numérico; no comprueba declaraciones ni imprime nada.

## 3. Parser y gramática

El **parser**, o analizador sintáctico, comprueba cómo encajan los tokens. [parser.rs](../src/parser.rs) contiene las reglas y las estructuras del AST:

```text
program     → statement* EOF
statement   → declaration | assignment | printStmt | ifStmt
ifStmt      → "if" "(" expression ")" block ("else" (ifStmt | block))?
block       → "{" statement* "}"
declaration → "const"? type IDENTIFIER "=" expression ";"
assignment  → IDENTIFIER ("[" expression "]")* "=" expression ";"
printStmt   → ("print" | "println") "(" expression ")" ";"
type        → basicType ("[" "]")*
basicType   → "int" | "float" | "bool" | "char" | "string"
expression  → or
or          → and ("||" and)*
and         → equality ("&&" equality)*
equality    → comparison (("==" | "!=") comparison)*
comparison  → term (("<" | "<=" | ">" | ">=") term)*
term        → factor (("+" | "-") factor)*
factor      → unary (("*" | "/" | "%") unary)*
unary       → ("!" | "-" | "+") unary | postfix
postfix     → primary ("[" expression "]")*
primary     → STRING | CHAR | "true" | "false" | NUMBER | IDENTIFIER
            | "(" expression ")" | "[" (expression ("," expression)*)? "]"
```

`→` significa «se compone de», `|` indica alternativas, `*` permite cero o más repeticiones y `?` indica una parte opcional. `STRING`, `CHAR` y los booleanos son variantes del token `Literal`; `NUMBER` corresponde al token `Number`.

La forma léxica de `NUMBER` es `dígitos ("." dígitos)? (("e" | "E") ("+" | "-")? dígitos)?`. Cada grupo de dígitos contiene al menos uno; el signo inicial se maneja en `unary()`.

1. `parse()` recoge instrucciones hasta `Eof`.
2. `statement()` distingue declaración, asignación, impresión e `if` por el primer token. Declaración, asignación e impresión exigen el `;` final; el `if` termina en `}` y no lo lleva. `declaration()` consume el `const` opcional, exige el tipo y recoge nombre e inicializador. Cada par `[]` tras el tipo básico lo envuelve en `Type::Array`. Para asignaciones, `statement()` recoge los índices opcionales después del nombre.
3. `if_statement()` consume `if`, exige la condición entre paréntesis y analiza un bloque. Si aparece `else`, analiza otro bloque o encadena un `if` anidado. `block()` recoge instrucciones hasta `}` y avisa si se alcanza el final del archivo.
4. `name()` exige un identificador y conserva su texto y línea en `Name`.
5. `print_statement()` exige los paréntesis alrededor de una expresión.
6. `expression()` baja por niveles de precedencia: `or()`, `and()`, `equality()`, `comparison()`, `term()`, `factor()`, `unary()`, `postfix()` y `primary()`.
7. Cada nivel binario usa `binary()` para encadenar sus operadores de izquierda a derecha. Cada operando se analiza en el siguiente nivel, que tiene mayor precedencia.
8. `unary()` admite signos y negación lógica de forma recursiva. Si `-` precede directamente a un token `Number`, llama a `number(true, line)` y convierte juntos signo y dígitos para aceptar el mínimo de `i64`. Los demás unarios generan un nodo `Unary`. Después de convertir un número con signo, `finish_postfix()` consume posibles índices para que también se comprueben accesos inválidos como `-1[0]`.
9. `postfix()` y `finish_postfix()` construyen un nodo `Index` por cada acceso. `index()` recoge la expresión del índice y la línea del corchete de apertura, y exige el cierre. La misma regla se usa al asignar elementos.
10. `primary()` crea literales básicos, referencias, arrays (`Expr::Array`) o analiza una expresión entre paréntesis. En un array recoge expresiones separadas por comas, sin coma final. `number()` convierte a `i64` o `f64`, rechazando enteros fuera de rango y float no finitos. Por ello `-9223372036854775808` es válido, pero `-(9223372036854775808)` se rechaza: el literal positivo interior ya está fuera de rango.

`peek()` consulta el token actual y `consume()` exige un token concreto y avanza. El análisis es **descendente**: empieza en el programa y baja hacia sus componentes. Los paréntesis cambian la agrupación del árbol sin necesitar un nodo propio.

El parser acepta la estructura de `float precio = 25;`: las piezas están bien colocadas. Es la siguiente etapa la que detecta que un `int` no puede inicializar un `float`.

## 4. AST, tipos y valores

**AST** significa árbol de sintaxis abstracta. Guarda lo necesario para ejecutar el programa; los delimitadores ya han cumplido su función. Las expresiones producen valores y las instrucciones realizan acciones:

```text
Expr
├── Literal(Value)
├── Variable(Name)
├── Array { elements: Vec<Expr>, line }
├── Index { array: Box<Expr>, index: Box<Expr>, line }
├── Unary { operator: UnaryOp, operand: Box<Expr>, line }
└── Binary { left: Box<Expr>, operator: BinaryOp, right: Box<Expr>, line }

Stmt
├── Declare { declared_type, is_constant, name, initializer }
├── Assign { name, indices: Vec<(Expr, línea)>, value }
├── If { condition, then_branch: Vec<Stmt>, else_branch: Option<Vec<Stmt>>, line }
├── Print(Expr)
└── Println(Expr)
```

Cada rama de un `if` es un `Vec<Stmt>`: la lista de instrucciones de su bloque. `else_branch` guarda `None` si no hay `else`; un `else if` queda como un `Vec` con un único `Stmt::If` interior.

Para el ejemplo:

```text
Vec<Stmt>
├── Declare
│   ├── declared_type: Int
│   ├── is_constant: false
│   ├── name: Name("edad", línea 1)
│   └── initializer: Literal(Int(25))
└── Println
    └── Variable(Name("edad", línea 2))
```

[value.rs](../src/value.rs) distingue `Type`, que identifica un tipo básico o `Array(Box<Type>)`, de `Value`, que además contiene el dato: `Int(i64)`, `Float(f64)`, `Bool(bool)`, `Char(char)`, `String(String)` o `Array(Vec<Value>)`. `Type::Array` conserva el tipo de elemento; la longitud no forma parte del tipo. `Type` usa `Clone` en lugar de `Copy` porque puede contener otro tipo mediante `Box`.

`value_type()` devuelve `Option<Type>`: obtiene el tipo de un valor cuando hay datos para conocerlo y devuelve `None` si encuentra un array vacío al consultar su primer elemento. El comprobador solo lo usa en `Expr::Literal`, que el parser reserva a valores básicos. Los literales de array son nodos `Expr::Array` separados: su tipo se comprueba a partir de las expresiones y del contexto, sin evaluar sus valores.

`UnaryOp` y `BinaryOp` representan las operaciones sin depender de los tokens del scanner. Cada nodo operador guarda su línea para los errores. `Box<Expr>` guarda una subexpresión mediante un puntero: permite que el árbol sea recursivo sin que cada nodo necesite un tamaño infinito.

El parser copia el contenido de los tokens al árbol. Para evaluar un literal, el intérprete obtiene ese valor; para evaluar una referencia, debe consultar el entorno.

## 5. Comprobación de tipos antes de ejecutar

[type_checker.rs](../src/type_checker.rs) introduce `TypeChecker` y una pila de ámbitos `Vec<HashMap<String, VariableInfo>>`. Un **entorno** relaciona nombres con información; en esta etapa cada `VariableInfo` contiene `declared_type` (el tipo) e `is_constant` (si se prohíbe reasignar), sin guardar valores. El último mapa de la pila es el ámbito actual; `lookup()` busca desde el más interno hacia fuera.

`check()` abre el ámbito global y recorre las instrucciones en orden:

- En una declaración, rechaza nombres repetidos, obtiene el tipo del inicializador usando el tipo declarado como contexto y exige que coincida con el declarado. Solo entonces registra el nombre junto con su tipo y la marca `is_constant`. Así `int x = x;` falla: `x` todavía no está disponible.
- En una asignación, busca la información del nombre y rechaza la operación si `is_constant` es `true`, incluso si el valor no cambiaría. Para las demás variables, resuelve el tipo del destino: sin índices es el declarado; cada índice exige un array y un `int`, y desciende al tipo de elemento. Compara ese tipo con el de la expresión asignada y lo proporciona como contexto para arrays vacíos. No cambia el tipo almacenado ni declara variables nuevas.
- En una impresión, comprueba que la expresión sea válida; una referencia debe existir previamente.
- En un `if`, obtiene el tipo de la condición y exige `Bool` (si no, el error señala la línea del `if`). Después abre un ámbito para la rama `then` y lo cierra al terminar. Si hay `else`, repite el proceso con su propia lista de instrucciones, de modo que las dos ramas se comprueban aunque solo se vaya a ejecutar una. `check_block()` hace el `push` y el `pop` del ámbito; declarar un nombre solo comprueba duplicados en el ámbito actual, así que se permite reutilizar el nombre en un bloque interior.

Para `int edad = 25;`, `expression_type()` obtiene `Int` del literal. Coincide con la anotación y se guarda `edad → VariableInfo { declared_type: Int, is_constant: false }`. Cuando llega `println(edad);`, la consulta obtiene `Int` del campo `declared_type`.

Si escribimos este programa **inválido**:

```oki
int edad = 25;
println(edad);
edad = "veinticinco";
```

Se obtiene este error antes de imprimir:

```text
Línea 3: Tipo incompatible para 'edad': se esperaba int, se recibió string. No hay conversiones implícitas.
```

El tipo explícito obligatorio, la compatibilidad exacta y la comprobación previa son decisiones relacionadas pero distintas. En esta versión se aplican las tres. Esta etapa es una adaptación propia: el intérprete Lox del libro comprueba tipos durante la ejecución.

`expression_type_expected()` comprueba una expresión con un tipo esperado opcional. Para `Expr::Array`, usa el tipo del elemento esperado o, si no existe, el de la primera expresión. Comprueba todos los elementos recursivamente con ese contexto y exige igualdad exacta. Un literal vacío sin contexto produce un error en su corchete de apertura: no hay un elemento que permita determinar su tipo. No busca un tipo en elementos posteriores ni lo propaga entre operandos de una operación. Por eso `int[][] a = [[], [1]];` es válido y `println([[], [1]]);` no lo es. Los paréntesis no eliminan el contexto porque no generan un nodo propio.

`indexed_type()` exige `Type::Array` en el objeto y `Type::Int` en el índice, tanto para `Expr::Index` como para cada índice de `Stmt::Assign`. Devuelve el tipo del elemento. No comprueba límites aquí: la longitud y el índice son valores de ejecución. La marca `is_constant` se comprueba antes de recorrer los índices y protege todo el valor, incluidos arrays interiores.

En `Unary` y `Binary`, `expression_type()` comprueba recursivamente los operandos y aplica las reglas de cada operador. Exige igualdad de tipos entre ambos operandos; aritmética conserva el tipo numérico, concatenación produce `String`, y comparaciones y lógica producen `Bool`. La igualdad y desigualdad admiten arrays del mismo tipo; las demás operaciones no admiten arrays completos. Comprueba ambos lados de `&&` y `||` aunque después pueda omitirse uno. Así `true || desconocida` y `false && 1` fallan antes de emitir salida.

## 6. Intérprete y entorno de valores

[interpreter.rs](../src/interpreter.rs) contiene `Interpreter<W: Write>`. Recibe un programa validado y guarda otro entorno: ahora una pila de ámbitos `Vec<HashMap<String, Value>>`, porque los bloques de un `if` pueden anidarse. El primer mapa es el ámbito global.

`interpret()` llama a `execute()` para cada instrucción. Una declaración evalúa el inicializador y guarda el resultado en el ámbito actual (el último de la pila). Una asignación busca el ámbito que contiene el nombre, resuelve primero el destino y comprueba todos sus índices de izquierda a derecha; después evalúa el nuevo valor y sustituye el anterior. Si falla un índice o la expresión asignada, no modifica el destino. `evaluate()` devuelve una copia del literal o del valor consultado; esto también copia el contenido de las cadenas y de todos los arrays anidados, y evita que dos variables compartan cambios. Para consultar una variable se recorre la pila de dentro hacia fuera, de modo que un nombre declarado en un bloque oculta al de un ámbito exterior mientras dura.

Un `Stmt::If` evalúa su condición y, según sea `true` o `false`, ejecuta el bloque `then` o el `else`. `execute_block()` abre un ámbito con `push`, ejecuta sus instrucciones y lo cierra con `pop`; si una instrucción falla, el error se propaga y el programa se detiene. Como el comprobador ya garantizó que la condición es `bool`, el intérprete no repite esa comprobación de tipos.

En el ejemplo, se almacena `edad → Value::Int(25)`. La impresión consulta ese valor y escribe `25` seguido de un salto de línea.

Si se añaden estas instrucciones válidas:

```oki
edad = 26;
println(edad);
```

El entorno se actualiza a `edad → Value::Int(26)` y se imprime `26` en otra línea. El entorno de tipos sigue indicando `Int`.

`Value` implementa `Display`, la capacidad de formatear un dato en Rust. Cadenas y caracteres se escriben sin comillas, booleanos como `true` o `false`, enteros en decimal y float mediante el formato que mantiene `1.0` distinguible de `1`. Los arrays se formatean entre corchetes y sus elementos se separan por comas. Las cadenas y caracteres interiores se rodean de comillas conservando su contenido sin escapes, por lo que no se garantiza que esta salida sea código reutilizable. El formateo para imprimir no convierte el tipo de una variable.

`write!` implementa `print` y `writeln!` implementa `println`. El destino genérico `W: Write` permite usar tanto la salida estándar como un `Vec<u8>` en las pruebas. No se vuelve a analizar texto al ejecutar.

### Recorrido de un array

Con la entrada:

```oki
int[] datos = [2, 3];
datos[0] = datos[1] + 4;
println(datos);
```

El scanner reconoce `Type(Int), LeftBracket, RightBracket` al principio de la declaración. Para el inicializador produce `LeftBracket, Number("2"), Comma, Number("3"), RightBracket`. El parser construye:

```text
Declare(datos, Array(Int), is_constant: false)
└── initializer: Array(línea 1)
    ├── Literal(Int(2))
    └── Literal(Int(3))
Assign(datos)
├── indices: [(Literal(Int(0)), línea 2)]
└── value: Binary(Add, línea 2)
    ├── Index(Variable(datos), Literal(Int(1)), línea 2)
    └── Literal(Int(4))
Println(Variable(datos))
```

El comprobador exige elementos `Int`, registra `datos → Array(Int)` y comprueba que los índices sean `Int` y el valor asignado también. El intérprete evalúa el literal y guarda `datos → Value::Array([Int(2), Int(3)])`. Para asignar comprueba que `0` esté dentro de los límites, evalúa `datos[1] + 4` como `3 + 4` y modifica el primer elemento. La salida es `[7, 3]` con salto final.

`evaluate()` construye `Value::Array` evaluando los elementos de izquierda a derecha. Para leer un `Index`, evalúa primero el array y después el índice; `array_position()` convierte el índice con `usize::try_from`, rechaza negativos o posiciones fuera de rango y devuelve la posición válida. Se copia el elemento seleccionado. En escrituras, `execute()` guarda las posiciones ya comprobadas, evalúa el nuevo valor y recorre el destino con referencias mutables para sustituir solo ese elemento. El árbol validado y la ausencia de efectos de asignación dentro de expresiones permiten usar esas posiciones sin que el destino cambie entre comprobación y escritura.

`int[][] tabla = [[], [1]];` tiene tipo `Array(Array(Int))`: el contexto de la declaración permite comprobar el vacío interior. Una copia de `tabla` clona ambos niveles. La igualdad usa la comparación recursiva de `Value`: comprueba longitud, orden y valores. Un índice fuera de rango como `datos[2]` se rechaza al evaluar y señala la línea de su `[`. Los arrays se guardan en `Vec`, pero el lenguaje todavía no ofrece operaciones para añadir elementos ni consultar su longitud.

### Recorrido de una constante

Con `const int limite = 2 * 5; println(limite);`, el scanner añade `Const` antes de `Type(Int)`. El parser construye `Declare { declared_type: Int, is_constant: true, name: limite, initializer: Binary(Multiply, Literal(Int(2)), Literal(Int(5))) }`, seguido de `Println(Variable(limite))`. El comprobador registra `limite → VariableInfo { declared_type: Int, is_constant: true }`.

El intérprete evalúa el inicializador una sola vez, guarda `limite → Value::Int(10)` e imprime `10` con salto final. La prohibición de reasignar pertenece al comprobador: el intérprete recibe el árbol validado y su tabla de valores no necesita duplicar esa marca. Si se añade `limite = 20;` en la línea 3, la comprobación falla con `Línea 3: No se puede reasignar la constante 'limite'.` y no llega a ejecutarse ninguna instrucción, incluida la impresión anterior.

### Recorrido de una operación

Con la entrada `int total = 2 + 3 * 4; println(total);`, el scanner produce para el inicializador `Number("2"), Plus, Number("3"), Star, Number("4")`. El parser construye:

```text
Declare(total, Int)
└── initializer: Binary(Add, línea 1)
    ├── Literal(Int(2))
    └── Binary(Multiply, línea 1)
        ├── Literal(Int(3))
        └── Literal(Int(4))
Println
└── Variable(total)
```

El comprobador obtiene `Int` para la multiplicación y para la suma, y registra `total → VariableInfo { declared_type: Int, is_constant: false }`. `evaluate()` evalúa los operandos de izquierda a derecha y llama a `binary()` para la operación: primero obtiene `3 * 4 = 12`, después `2 + 12 = 14`. Guarda `total → Int(14)` y la impresión produce `14` con salto final. Con `(2 + 3) * 4`, la suma queda como hijo izquierdo de la multiplicación y el resultado es `20`.

`evaluate()` también aplica los unarios. Para `&&` y `||`, evalúa la izquierda primero y devuelve inmediatamente `false` o `true`, respectivamente, si ya determina el resultado. Solo en los demás casos evalúa la derecha. Esto hace que `false && 1 / 0 == 0` sea válido y produzca `false` sin división por cero.

La aritmética entera usa operaciones `checked_*`, que devuelven un fallo en lugar de provocar un pánico de Rust o envolver el resultado fuera de rango. El resto por `-1` se resuelve como cero incluso para el mínimo de `i64`, evitando el desbordamiento del cociente intermedio. Para float se comprueba `is_finite()` después de operar. En ambos tipos se rechaza primero el divisor cero en `/` y `%`. La concatenación combina el contenido de dos cadenas; las comparaciones usan sus valores y el orden Unicode, y devuelven `Value::Bool`.

### Recorrido de un if

Con la entrada:

```oki
int x = 1;
if (x < 2) {
    int y = 10;
    println(y);
} else {
    println("otro");
}
println(x);
```

El parser construye:

```text
Declare(x, Int)
If(condición: Binary(Less, línea 2), línea 2)
├── then: [Declare(y, Int), Println(Variable(y))]
└── else: [Println(Literal(String("otro")))]
Println(Variable(x))
```

El comprobador registra `x → Int`, obtiene `Bool` de `x < 2` y exige ese tipo a la condición. Después comprueba la rama `then` en un ámbito nuevo (allí `y → Int`) y la rama `else` en otro; `y` no existe fuera de su bloque. El intérprete guarda `x → Int(1)` en el ámbito global, evalúa `1 < 2` como `true` y entra en el bloque `then`: abre un ámbito, guarda `y → Int(10)`, imprime `10` y cierra el ámbito. La rama `else` no se ejecuta. La última impresión consulta `x` en el ámbito global y produce `1`, así que la salida es `10` y `1`, cada uno en su línea.

Si la condición no fuese `bool`, por ejemplo `if (x) { ... }`, el comprobador fallaría antes de ejecutar con `Línea 2: la condición de 'if' debe ser bool; se recibió int.` y no se imprimiría nada.

Cada llamada a `run()` crea sus dos entornos con un único ámbito global. Los bloques de un `if` añaden y retiran ámbitos sobre esa pila; al terminar el programa la pila vuelve a tener solo el ámbito global.

## 7. Errores

| Etapa | Ejemplo | Resultado |
| --- | --- | --- |
| Lectura | Ruta ausente, archivo inexistente o contenido que no es UTF-8. | Error antes del análisis. |
| Scanner | Comillas sin cerrar, `char` vacío o múltiple, exponente incompleto. | Error con línea. |
| Parser | Falta un tipo válido, nombre, inicializador, paréntesis, corchete, llave, coma entre elementos o `;`; número fuera de rango. | Error con línea. |
| Comprobación de tipos | Variable desconocida, declaración duplicada, tipo incompatible, elementos de tipos distintos, índice que no es `int`, vacío sin contexto, condición de `if` que no es `bool` o reasignación de una constante. | Error con línea, antes de ejecutar. |
| Intérprete | Índice de array fuera de rango, división/resto por cero o resultado numérico fuera de rango; fallo al escribir. | Error con línea del corchete para índices o del operador para errores numéricos; se propaga el error de entrada/salida para escritura. |

Los errores propios del lenguaje usan `String`; la escritura y lectura pueden producir `io::Error`. `run()` los propaga mediante `Box<dyn Error>`, que admite distintos tipos de error. `main()` escribe el mensaje en `stderr` con el prefijo `Error:` y termina con código de fallo.

En declaraciones duplicadas, incompatibilidades y reasignaciones de constantes se señala la línea del nombre declarado o asignado. En referencias desconocidas se señala la línea del uso. Los errores de delimitadores señalan el token pendiente o el final del archivo. Los errores de operandos incompatibles y los errores numéricos en ejecución señalan la línea del operador. Los errores de indexación señalan la línea del `[` del acceso; los de elementos incompatibles o vacíos sin contexto, la del `[` del literal correspondiente. El parser presupone la marca `Eof` que añade el scanner.

Se devuelve el primer error detectado por las etapas, sin recuperación para buscar más. El análisis completo precede a la ejecución; un fallo de índices, numérico o de salida sí puede ocurrir después de haber escrito parte del texto, y no se deshace esa salida. Por ejemplo, `println("previo"); println(1 / 0);` imprime `previo` y después falla. No se evalúan operaciones durante la comprobación de tipos.

## Relación con Crafting Interpreters

Los [capítulos 4 a 9](https://craftinginterpreters.com/contents.html) aportan el recorrido scanner → AST → parser → intérprete. El [capítulo 7](https://craftinginterpreters.com/evaluating-expressions.html) explica la representación de valores; el [capítulo 8](https://craftinginterpreters.com/statements-and-state.html) introduce declaraciones, referencias, asignaciones, entornos y bloques; y el [capítulo 9](https://craftinginterpreters.com/control-flow.html) añade `if`/`else` y los operadores lógicos.

OkitsuLang adapta esas ideas a `enum`, `match` y `HashMap` de Rust. Mantiene `print(expresión);` y `println(expresión);`, exige `tipo nombre = expresión;` con `const` opcional antes del tipo, distingue enteros de float y añade `char`. La comprobación estática, es decir, antes de ejecutar, implementa la decisión de tipado estricto del proyecto.

Las demás características de Lox y la máquina virtual de bytecode no forman parte de este avance.
