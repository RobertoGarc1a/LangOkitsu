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

El scanner también reconoce los operadores `+`, `-`, `*`, `/`, `%`, `!`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `&&` y `||`, además de `++`, `--`, `+=` y `-=`. `paired()` mira si un signo lleva un segundo `=`: así distingue la asignación `=` de la igualdad `==`. `compound_or_single()` mira un carácter por delante de `+` o `-` y decide entre el operador doble (`++`, `--`), la variante con `=` (`+=`, `-=`) y el signo aislado. Un `&` o `|` aislado se rechaza. El signo del exponente sigue perteneciendo al número (`1e-2`), mientras que en `2-1` el menos es un token independiente. Como `--` es ahora un token propio, `1--2` ya no significa `1 - (-2)` y se rechaza.

El scanner comprueba la forma del literal numérico; no comprueba declaraciones ni imprime nada.

## 3. Parser y gramática

El **parser**, o analizador sintáctico, comprueba cómo encajan los tokens. [parser.rs](../src/parser.rs) contiene las reglas y las estructuras del AST:

```text
program     → statement* EOF
statement   → simpleStmt ";" | ifStmt | whileStmt | forStmt | foreachStmt
simpleStmt  → declaration | assignment | printStmt | importStmt | callStmt
callStmt    → postfix  (su nodo exterior debe ser LibraryCall)
importStmt  → ("import" | "use") path
path        → IDENTIFIER ("::" IDENTIFIER)*
ifStmt      → "if" "(" expression ")" block ("else" (ifStmt | block))?
whileStmt   → "while" "(" expression ")" block
forStmt     → "for" "(" (declaration | assignment) ";" expression ";" assignment ")" block
foreachStmt → "foreach" "(" type IDENTIFIER "in" expression ")" block
block       → "{" statement* "}"
declaration → "const"? type IDENTIFIER "=" expression
assignment  → IDENTIFIER ("[" expression "]")* assignTail
assignTail  → "=" expression | "+=" expression | "-=" expression | "++" | "--"
printStmt   → ("print" | "println") "(" expression ")"
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
postfix     → primary ("[" expression "]" | "." IDENTIFIER "(" arguments? ")")*
arguments   → expression ("," expression)*
libraryCall → path "(" arguments? ")"
primary     → STRING | CHAR | "true" | "false" | NUMBER | IDENTIFIER
            | libraryCall | "(" expression ")" | "[" (expression ("," expression)*)? "]"
```

`→` significa «se compone de», `|` indica alternativas, `*` permite cero o más repeticiones y `?` indica una parte opcional. `STRING`, `CHAR` y los booleanos son variantes del token `Literal`; `NUMBER` corresponde al token `Number`. El `;` que separa las tres partes de un `for` no pertenece a `declaration` ni a `assignment`: la sentencia simple lo añade al final y `for_statement()` lo exige entre partes. `assignTail` reúne las cuatro modificaciones de una variable ya declarada: asignación, asignación compuesta e incremento/decremento.

La forma léxica de `NUMBER` es `dígitos ("." dígitos)? (("e" | "E") ("+" | "-")? dígitos)?`. Cada grupo de dígitos contiene al menos uno; el signo inicial se maneja en `unary()`.

1. `parse()` recoge instrucciones hasta `Eof`.
2. `statement()` distingue por el primer token las sentencias simples (declaración, modificación de variable, llamada, impresión), que exigen el `;` final, y las que terminan en `}` ( `if`, `while`, `for` y `foreach`), que no lo llevan. Ante un identificador, paréntesis o corchete analiza `postfix()`: si obtiene una llamada la envuelve en `Stmt::Call`; en otro caso vuelve al inicio para analizar la asignación. Este paso solo construye el árbol, sin ejecutar sus expresiones. `declaration()` consume el `const` opcional, delega el tipo en `array_type()` y recoge nombre e inicializador. `assignment()` recoge el nombre y los índices opcionales y, según el token siguiente, construye una asignación (`=`), una asignación compuesta (`+=`, `-=`) o un incremento/decremento (`++`, `--`); se separa de `statement()` para poder reutilizarla en la cabecera de un `for`. `array_type()` consume el tipo básico y envuelve cada par `[]` en `Type::Array`.
3. `if_statement()` consume `if`, exige la condición entre paréntesis y analiza un bloque. Si aparece `else`, analiza otro bloque o encadena un `if` anidado. `block()` recoge instrucciones hasta `}` y avisa si se alcanza el final del archivo.
4. `while_statement()` consume `while`, exige la condición y un bloque. `for_statement()` exige `(`, analiza como inicialización una declaración o una modificación de variable, y a continuación la condición y la actualización separadas por `;`; la actualización admite asignación, asignación compuesta o incremento. `foreach_statement()` exige el tipo, el nombre, la palabra reservada `in`, el array y un bloque.
5. `name()` exige un identificador y conserva su texto y línea en `Name`.
6. `print_statement()` exige los paréntesis alrededor de una expresión.
7. `expression()` baja por niveles de precedencia: `or()`, `and()`, `equality()`, `comparison()`, `term()`, `factor()`, `unary()`, `postfix()` y `primary()`.
8. Cada nivel binario usa `binary()` para encadenar sus operadores de izquierda a derecha. Cada operando se analiza en el siguiente nivel, que tiene mayor precedencia.
9. `unary()` admite signos y negación lógica de forma recursiva. Si `-` precede directamente a un token `Number`, llama a `number(true, line)` y convierte juntos signo y dígitos para aceptar el mínimo de `i64`. Los demás unarios generan un nodo `Unary`. Después de convertir un número con signo, `finish_postfix()` consume posibles índices para que también se comprueben accesos inválidos como `-1[0]`.
10. `postfix()` y `finish_postfix()` construyen un nodo `Index` por cada acceso y un `LibraryCall` por cada método con punto. `index()` recoge la expresión del índice y la línea del corchete de apertura, y exige el cierre. La misma regla se usa al asignar elementos.
11. `primary()` crea literales básicos, referencias, arrays (`Expr::Array`), llamadas de biblioteca por ruta (`Expr::LibraryCall`) o analiza una expresión entre paréntesis. En un array recoge expresiones separadas por comas, sin coma final. `number()` convierte a `i64` o `f64`, rechazando enteros fuera de rango y float no finitos. Por ello `-9223372036854775808` es válido, pero `-(9223372036854775808)` se rechaza: el literal positivo interior ya está fuera de rango.

`peek()` consulta el token actual y `consume()` exige un token concreto y avanza. El análisis es **descendente**: empieza en el programa y baja hacia sus componentes. Los paréntesis cambian la agrupación del árbol sin necesitar un nodo propio.

El parser acepta la estructura de `float precio = 25;`: las piezas están bien colocadas. Es la siguiente etapa la que detecta que un `int` no puede inicializar un `float`.

## 4. AST, tipos y valores

**AST** significa árbol de sintaxis abstracta. Guarda lo necesario para ejecutar el programa; los delimitadores ya han cumplido su función. Las expresiones producen valores y las instrucciones realizan acciones:

```text
Expr
├── LibraryCall { path: Vec<Name>, receiver: Option<Box<Expr>>, arguments: Vec<Expr> }
├── Literal(Value)
├── Variable(Name)
├── Array { elements: Vec<Expr>, line }
├── Index { array: Box<Expr>, index: Box<Expr>, line }
├── Unary { operator: UnaryOp, operand: Box<Expr>, line }
└── Binary { left: Box<Expr>, operator: BinaryOp, right: Box<Expr>, line }

Stmt
├── Call(Expr)
├── Import { path: Vec<Name>, is_use: bool, line }
├── Declare { declared_type, is_constant, name, initializer }
├── Assign { name, indices: Vec<(Expr, línea)>, value }
├── CompoundAssign { name, indices, operator: AssignOp, value, line }
├── Increment { name, indices, operator: IncrementOp, line }
├── If { condition, then_branch: Vec<Stmt>, else_branch: Option<Vec<Stmt>>, line }
├── While { condition, body: Vec<Stmt>, line }
├── For { initializer: Box<Stmt>, condition, update: Box<Stmt>, body: Vec<Stmt>, line }
├── Foreach { declared_type, name, iterable, body: Vec<Stmt>, line }
├── Print(Expr)
└── Println(Expr)
```

Cada rama de un `if` y cada cuerpo de bucle es un `Vec<Stmt>`: la lista de instrucciones de su bloque. `else_branch` guarda `None` si no hay `else`; un `else if` queda como un `Vec` con un único `Stmt::If` interior. En `For`, `initializer` y `update` son instrucciones completas (`Declare`, `Assign`, `CompoundAssign` o `Increment`), guardadas en `Box` para no dar un tamaño infinito al enum. `AssignOp` distingue `+=` de `-=` e `IncrementOp`, `++` de `--`; cada uno sabe qué operación binaria equivale a la forma abreviada.

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
- En una asignación, busca la información del nombre y rechaza la operación si `is_constant` es `true`, incluso si el valor no cambiaría. Para las demás variables, resuelve el tipo del destino: sin índices es el declarado; cada índice exige un array y un `int`, y desciende al tipo de elemento. Compara ese tipo con el de la expresión asignada y lo proporciona como contexto para arrays vacíos. No cambia el tipo almacenado ni declara variables nuevas. `assignment_target()` reúne la búsqueda, el rechazo de constantes y el recorrido de índices; lo comparten la asignación, la compuesta y el incremento.
- En una asignación compuesta (`+=`, `-=`), obtiene el tipo del destino como en una asignación y el de la derecha, y aplica las reglas de `+` o `-` mediante `binary_result()`. El destino debe admitir la operación con el tipo de la derecha: `+=` vale para `int`, `float` y `string`, y `-=` para `int` y `float`. El error señala la línea del operador.
- En un incremento o decremento (`++`, `--`), el destino debe ser `int` o `float`; no hay operando derecho que comprobar. El paso de una unidad se decide al ejecutar a partir del tipo del destino.
- En una impresión, comprueba que la expresión sea válida; una referencia debe existir previamente.
- En un `if`, obtiene el tipo de la condición y exige `Bool` (si no, el error señala la línea del `if`). Después abre un ámbito para la rama `then` y lo cierra al terminar. Si hay `else`, repite el proceso con su propia lista de instrucciones, de modo que las dos ramas se comprueban aunque solo se vaya a ejecutar una. `check_block()` hace el `push` y el `pop` del ámbito; declarar un nombre solo comprueba duplicados en el ámbito actual, así que se permite reutilizar el nombre en un bloque interior.
- En un `while`, exige que la condición sea `Bool` (error en la línea del `while`) y comprueba el cuerpo en un ámbito nuevo.
- En un `for`, abre un ámbito para todo el bucle, comprueba la inicialización (una declaración o una asignación), exige `Bool` a la condición y comprueba la actualización. Después comprueba el cuerpo en un ámbito interior; el contador declarado en la inicialización deja de existir al cerrar el ámbito del bucle. `check_statement()` permite reutilizar la lógica de declaración y asignación sin duplicarla.
- En un `foreach`, obtiene el tipo de la expresión y exige que sea `Type::Array`. El tipo del elemento debe coincidir exactamente con el declarado; si no, el error señala el nombre. Registra la variable del bucle en un ámbito nuevo y comprueba el cuerpo en un ámbito interior.

El mensaje de condición incorrecta lo produce `require_bool()`, compartido por `if`, `while` y `for`, e indica la línea del bucle.

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

En `Unary` y `Binary`, `expression_type()` comprueba recursivamente los operandos y aplica las reglas de cada operador. Exige igualdad de tipos entre ambos operandos; aritmética conserva el tipo numérico, concatenación produce `String`, y comparaciones y lógica producen `Bool`. La igualdad y desigualdad admiten arrays del mismo tipo; las demás operaciones no admiten arrays completos. Comprueba ambos lados de `&&` y `||` aunque después pueda omitirse uno. Así `true || desconocida` y `false && 1` fallan antes de emitir salida. Las reglas comunes viven en `binary_result()`, que devuelve `None` cuando los operandos no comparten tipo o el operador no los admite; la asignación compuesta la reutiliza con `+` o `-`.

## 6. Intérprete y entorno de valores

[interpreter.rs](../src/interpreter.rs) contiene `Interpreter<W: Write>`. Recibe un programa validado y guarda otro entorno: ahora una pila de ámbitos `Vec<HashMap<String, Value>>`, porque los bloques de un `if` o de un bucle pueden anidarse. El primer mapa es el ámbito global.

`interpret()` llama a `execute()` para cada instrucción. Una declaración evalúa el inicializador y guarda el resultado en el ámbito actual (el último de la pila). Una asignación busca el ámbito que contiene el nombre, resuelve primero el destino y comprueba todos sus índices de izquierda a derecha; después evalúa el nuevo valor y sustituye el anterior. Si falla un índice o la expresión asignada, no modifica el destino. `resolve_target()` devuelve el ámbito y las posiciones ya validadas, y `write_target()` escribe en ellas; entre ambos, `target_value()` obtiene una copia del valor actual. Una asignación compuesta evalúa la derecha, aplica `binary()` con el operando actual y escribe el resultado; un incremento usa `1` o `1.0` como paso según el tipo del destino. Si la operación desborda, no se escribe nada. `evaluate()` devuelve una copia del literal o del valor consultado; esto también copia el contenido de las cadenas y de todos los arrays anidados, y evita que dos variables compartan cambios. Para consultar una variable se recorre la pila de dentro hacia fuera, de modo que un nombre declarado en un bloque oculta al de un ámbito exterior mientras dura.

Un `Stmt::If` evalúa su condición y, según sea `true` o `false`, ejecuta el bloque `then` o el `else`. `execute_block()` abre un ámbito con `push`, ejecuta sus instrucciones y lo cierra con `pop`; si una instrucción falla, el error se propaga y el programa se detiene. Como el comprobador ya garantizó que las condiciones son `bool`, el intérprete no repite esa comprobación de tipos.

Los bucles usan la misma pila. `Stmt::While` evalúa la condición antes de cada vuelta y ejecuta el cuerpo con `execute_block()`. `Stmt::For` abre un ámbito, ejecuta la inicialización una vez y repite condición, cuerpo y actualización. `Stmt::Foreach` evalúa el array una sola vez (obtiene una copia), abre un ámbito y, por cada elemento, guarda una copia en la variable del bucle y ejecuta el cuerpo; el ámbito del contador o del elemento se cierra al terminar, aunque el bucle se detenga por una condición falsa en la primera vuelta.

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

`evaluate()` construye `Value::Array` evaluando los elementos de izquierda a derecha. Para leer un `Index`, evalúa primero el array y después el índice; `array_position()` convierte el índice con `usize::try_from`, rechaza negativos o posiciones fuera de rango y devuelve la posición válida. Se copia el elemento seleccionado. En escrituras, `execute()` guarda las posiciones ya comprobadas, evalúa el nuevo valor y recorre el destino con referencias mutables para sustituir solo ese elemento. Como `pop` puede cambiar arrays dentro de las expresiones, `target_value()` y `target_mut()` comprueban que las posiciones sigan existiendo. Un destino invalidado produce un error en la línea de la variable raíz, sin acceso fuera de límites en Rust.

`int[][] tabla = [[], [1]];` tiene tipo `Array(Array(Int))`: el contexto de la declaración permite comprobar el vacío interior. Una copia de `tabla` clona ambos niveles. La igualdad usa la comparación recursiva de `Value`: comprueba longitud, orden y valores. Un índice fuera de rango como `datos[2]` se rechaza al evaluar y señala la línea de su `[`. Los arrays se guardan en `Vec`; `std::Array` permite consultar su longitud con `len` y modificar su final con `push` y `pop`.

### Recorrido de una llamada a std::Array

Con esta entrada:

```oki
import std::Array;
use std::Array;
int[] numeros = [10, 20, 30];
println(numeros.len());
println(std::Array::len(numeros));
println(Array::len(numeros));
```

El scanner reconoce las nuevas palabras reservadas `Import` y `Use`, el separador `ColonColon` (`::`) y el punto `Dot`. `std`, `Array` y `len` se mantienen como identificadores. La lectura de números conserva su regla de punto decimal; por ejemplo, `1.0` sigue siendo un único token numérico.

El parser conserva la estructura de las llamadas sin decidir todavía si existe la biblioteca. `path()` recoge los nombres separados por `::`, `arguments()` recoge las expresiones entre paréntesis e `import_statement()` crea `Stmt::Import`, con `is_use` para distinguir las dos instrucciones. `primary()` distingue una variable de una llamada por su ruta y paréntesis. `finish_postfix()` ahora encadena índices y métodos: `tabla[0].len()` conserva el acceso como receptor de la llamada.

```text
Import(path: [std, Array], is_use: false)
Import(path: [std, Array], is_use: true)
Declare(numeros, Array(Int), initializer: [Int(10), Int(20), Int(30)])
Println(LibraryCall(path: [len], receiver: Variable(numeros), arguments: []))
Println(LibraryCall(path: [std, Array, len], receiver: None, arguments: [Variable(numeros)]))
Println(LibraryCall(path: [Array, len], receiver: None, arguments: [Variable(numeros)]))
```

[stdlib.rs](../src/stdlib.rs) concentra las reglas de la biblioteca incluida en Rust. `ArrayLibrary` guarda dos marcas: biblioteca importada y nombre corto habilitado. El comprobador las reinicia al empezar el archivo y las actualiza al encontrar `import` y `use` en orden. Rechaza estas instrucciones dentro de bloques; no habilita nombres a partir de una rama que quizá no se ejecute. Repetir una directiva ya válida no tiene efecto adicional.

Para cada `LibraryCall` de este ejemplo, `ArrayLibrary::resolve()` comprueba la ruta y las marcas y obtiene `ArrayFunction::Len`. `check_arity()` verifica el número de argumentos: cero con receptor o uno sin él. El comprobador obtiene el tipo del array y `result_type()` exige `Type::Array` y devuelve `Some(Type::Int)`. Este `Option` distingue las llamadas con resultado de `push`, que no devuelve valor. No necesita el tamaño real ni modifica el AST. Los nombres de las rutas se resuelven aparte de las variables; `use` solo habilita `Array::`, sin crear una variable llamada `Array`.

El intérprete no realiza acciones al encontrar `Stmt::Import`: esas instrucciones ya se comprobaron. En `LibraryCall` identifica la operación, evalúa una sola vez el receptor o el único argumento y entrega su valor a `ArrayFunction::evaluate()`. `len` obtiene el número de elementos del `Vec` y lo convierte a `i64` con comprobación de rango. En el ejemplo, las tres impresiones producen `3` con salto final.

La evaluación conserva la semántica actual de copia: consultar una variable array mediante `evaluate()` copia su contenido antes de medirlo. Todavía no se ha optimizado esa lectura. La llamada no escribe en el array y admite `const`. `len` solo cuenta el nivel exterior; el acceso previo de `tabla[0].len()` selecciona qué array se mide. Los errores al evaluar el receptor, como un índice fuera de rango, se propagan conservando su línea y la salida previa. El cortocircuito de `&&` y `||` puede omitir la evaluación de una llamada, pero nunca su comprobación de tipos.

Una ruta, método, importación, cantidad de argumentos o tipo incorrecto falla antes de ejecutar. Los errores de llamadas señalan la línea del nombre del método; los de directivas, la de `import` o `use`. No se implementa un sistema general de funciones ni un cargador de archivos: la única biblioteca actual es `std::Array`, con `len`, `push` y `pop`.

### Recorrido de push y pop

```oki
import std::Array;
int[][] tabla = [[]];
tabla[0].push(4);
int ultimo = std::Array::pop(tabla[0]);
println(ultimo);
println(tabla);
```

El scanner no necesita nuevos tokens: `push` y `pop` son identificadores. El parser conserva `LibraryCall` para ambas formas. La instrucción de inserción se representa como `Call(LibraryCall(path: [push], receiver: Index(Variable(tabla), Int(0)), arguments: [Int(4)]))`. El inicializador de `ultimo` contiene `LibraryCall(path: [std, Array, pop], receiver: None, arguments: [Index(Variable(tabla), Int(0))])`.

`TypeChecker::check_call()` resuelve la operación y comprueba su cantidad de argumentos. `Expr::into_target()` descompone una variable seguida de índices en el nombre raíz y su lista de accesos; rechaza literales y resultados temporales. La comprobación reutiliza `assignment_target()` para impedir cambios sobre constantes, también anidadas. Para `push`, el tipo del elemento del destino se entrega como contexto al argumento: así se acepta `tabla.push([])`. `result_type()` devuelve `None` para `push` y `Some(tipo_del_elemento)` para `pop`. `Stmt::Call` permite descartar el resultado; si una expresión necesita el valor de `push`, se rechaza antes de ejecutar. No se añade un tipo `void` al lenguaje.

`Interpreter::evaluate()` recibe ahora `&mut self`, porque evaluar `pop` cambia el entorno. `evaluate_call()` resuelve el ámbito y los índices con `resolve_target()`, evalúa el argumento de `push` y obtiene el almacenamiento mediante `target_mut()`. `ArrayFunction::evaluate()` recibe ese valor por referencia mutable y opera sobre su `Vec`: `push` añade y `pop` extrae el último elemento. En el ejemplo, `tabla` pasa de `[[]]` a `[[4]]` y vuelve a `[[]]`; `ultimo` guarda `Value::Int(4)`. Se imprime `4` y `[[]]`.

Los índices se evalúan una sola vez, antes del argumento que se va a insertar. Si una llamada interior elimina parte del destino, se comprueban de nuevo las posiciones guardadas; no se repiten las expresiones de los índices. Por ejemplo, `int[] a = [1]; a[0] = a.pop();` extrae `1` y después falla al escribir en la posición que ya no existe. Los efectos completados no se deshacen. `pop` de un vacío falla en la línea de su nombre. Los errores conservan la salida previa. Se mantienen la evaluación de izquierda a derecha, el cortocircuito y el recorrido de una copia en `foreach`.

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

### Recorrido de una asignación abreviada

Con la entrada:

```oki
int contador = 0;
contador++;
contador += 5;
contador -= 3;
println(contador);
```

El scanner produce `Type(Int), Identifier("contador"), Equal, Number("0"), Semicolon` y, en las líneas siguientes, `Identifier("contador"), PlusPlus, Semicolon`; `Identifier("contador"), PlusEqual, Number("5"), Semicolon`; e `Identifier("contador"), MinusEqual, Number("3"), Semicolon`. El parser construye:

```text
Declare(contador, Int, Literal(Int(0)))
Increment(contador, IncrementOp::Increment, línea 2)
CompoundAssign(contador, AssignOp::Add, Literal(Int(5)), línea 3)
CompoundAssign(contador, AssignOp::Subtract, Literal(Int(3)), línea 4)
Println(Variable(contador))
```

El comprobador registra `contador → Int`, comprueba que el incremento admite `int` y que las dos asignaciones compuestas aplican `+`/`-` sobre `int` con un operando del mismo tipo. El intérprete guarda `contador → Int(0)`; en la línea 2 lee `0`, calcula `0 + 1` y escribe `1`; en la 3 calcula `1 + 5` y escribe `6`; en la 4 calcula `6 - 3` y escribe `3`. La impresión produce `3` con salto final. `contador++` equivale a `contador = contador + 1`; el paso cambia a `1.0` solo si el destino es `float`.

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

### Recorrido de un bucle

Con la entrada:

```oki
int total = 0;
for (int i = 1; i <= 3; i = i + 1) {
    total = total + i;
}
println(total);

int[] datos = [10, 20];
foreach (int n in datos) {
    print(n);
}
```

El parser construye:

```text
Declare(total, Int)
For(línea 2)
├── initializer: Declare(i, Int, Literal(Int(1)))
├── condition: Binary(LessEqual, Variable(i), Literal(Int(3)), línea 2)
├── update: Assign(i, Binary(Add, Variable(i), Literal(Int(1)), línea 2))
└── body: [Assign(total, Binary(Add, Variable(total), Variable(i)))]
Println(Variable(total))
Declare(datos, Array(Int))
Foreach(Int n, iterable: Variable(datos), línea 8)
└── body: [Print(Variable(n))]
```

El comprobador abre un ámbito para el `for`, registra `i → Int` al comprobar la inicialización, exige `Bool` a `i <= 3` y comprueba la actualización y el cuerpo; al cerrar el ámbito `i` desaparece. El intérprete ejecuta la inicialización (`i → 1`), y repite: `1 <= 3` es `true`, ejecuta el cuerpo (`total → 1`), actualiza `i → 2`; después `2 <= 3`, `total → 3`, `i → 3`; después `3 <= 3`, `total → 6`, `i → 4`; `4 <= 3` es `false` y sale. `println(total)` produce `6` con salto final.

Para el `foreach`, el comprobador obtiene `Array(Int)` de `datos`, comprueba que el elemento `Int` coincide con el tipo declarado y registra `n → Int` en un ámbito que envuelve el cuerpo. El intérprete evalúa `datos` una vez y recorre la copia: guarda `n → 10` y ejecuta el cuerpo, que escribe `10`; guarda `n → 20` y escribe `20`. La salida total es `6`, `1020`.

Si la condición de un bucle no fuese `bool`, por ejemplo `while (1) { ... }`, el comprobador fallaría antes de ejecutar con `Línea 1: la condición de 'while' debe ser bool; se recibió int.` y no se imprimiría nada.

Cada llamada a `run()` crea sus dos entornos con un único ámbito global. Los bloques de un `if` y los ámbitos de un `for` o un `foreach` se añaden y retiran sobre esa pila; al terminar el programa la pila vuelve a tener solo el ámbito global.

## 7. Errores

| Etapa | Ejemplo | Resultado |
| --- | --- | --- |
| Lectura | Ruta ausente, archivo inexistente o contenido que no es UTF-8. | Error antes del análisis. |
| Scanner | Comillas sin cerrar, `char` vacío o múltiple, exponente incompleto. | Error con línea. |
| Parser | Falta un tipo válido, nombre, inicializador, paréntesis, corchete, llave, coma entre elementos o `;`; en los bucles, falta alguna de las tres partes del `for` o la palabra `in` del `foreach`; número fuera de rango. | Error con línea. |
| Comprobación de tipos | Variable desconocida, declaración duplicada, tipo incompatible, elementos de tipos distintos, índice que no es `int`, vacío sin contexto, condición de `if`/`while`/`for` que no es `bool`, `foreach` que no recorre un array o cuyo tipo de elemento no coincide, reasignación de una constante, o `+=`/`-=`/`++`/`--` sobre un destino que no admite la operación. | Error con línea, antes de ejecutar. |
| Intérprete | Índice de array fuera de rango, división/resto por cero o resultado numérico fuera de rango; fallo al escribir. | Error con línea del corchete para índices o del operador para errores numéricos; se propaga el error de entrada/salida para escritura. |

Los errores propios del lenguaje usan `String`; la escritura y lectura pueden producir `io::Error`. `run()` los propaga mediante `Box<dyn Error>`, que admite distintos tipos de error. `main()` escribe el mensaje en `stderr` con el prefijo `Error:` y termina con código de fallo.

En declaraciones duplicadas, incompatibilidades y reasignaciones de constantes se señala la línea del nombre declarado o asignado. En referencias desconocidas se señala la línea del uso. Los errores de delimitadores señalan el token pendiente o el final del archivo. Los errores de operandos incompatibles y los errores numéricos en ejecución señalan la línea del operador. Los errores de indexación señalan la línea del `[` del acceso; los de elementos incompatibles o vacíos sin contexto, la del `[` del literal correspondiente. El parser presupone la marca `Eof` que añade el scanner.

Se devuelve el primer error detectado por las etapas, sin recuperación para buscar más. El análisis completo precede a la ejecución; un fallo de índices, numérico o de salida sí puede ocurrir después de haber escrito parte del texto, y no se deshace esa salida. Por ejemplo, `println("previo"); println(1 / 0);` imprime `previo` y después falla. No se evalúan operaciones durante la comprobación de tipos.

## Relación con Crafting Interpreters

Los [capítulos 4 a 9](https://craftinginterpreters.com/contents.html) aportan el recorrido scanner → AST → parser → intérprete. El [capítulo 7](https://craftinginterpreters.com/evaluating-expressions.html) explica la representación de valores; el [capítulo 8](https://craftinginterpreters.com/statements-and-state.html) introduce declaraciones, referencias, asignaciones, entornos y bloques; y el [capítulo 9](https://craftinginterpreters.com/control-flow.html) añade `if`/`else`, `while`, `for` y los operadores lógicos.

OkitsuLang adapta esas ideas a `enum`, `match` y `HashMap` de Rust. Mantiene `print(expresión);` y `println(expresión);`, exige `tipo nombre = expresión;` con `const` opcional antes del tipo, distingue enteros de float y añade `char`. El `for` de OkitsuLang es de estilo C y no ofrece `break` ni `continue`; el recorrido de arrays se separa en `foreach` con el tipo del elemento explícito. La comprobación estática, es decir, antes de ejecutar, implementa la decisión de tipado estricto del proyecto.

Las demás características de Lox y la máquina virtual de bytecode no forman parte de este avance.
