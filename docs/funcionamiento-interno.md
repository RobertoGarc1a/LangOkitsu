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

El AST validado conserva el mismo programa. El comprobador completa únicamente el tipo de elemento de cada literal array, necesario para reconocer su tipo incluso cuando está vacío.

## 1. Leer y coordinar

[main.rs](../src/main.rs) conserva `main()`, `run_file()` y las pruebas generales; las pruebas de ampliaciones de estructuras están en [structure_tests.rs](../src/structure_tests.rs) y las de enums y match en [enum_tests.rs](../src/enum_tests.rs). `run_file()` obtiene la ruta mediante `env::args_os().nth(1)`, lee el archivo con `fs::read_to_string` y llama a `run()` con el texto y la salida estándar.

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

`identifier()` lee primero el nombre completo y después reconoce las palabras reservadas. Así distingue `int` de `int2` y `println` de `println2`. Reconoce `const` como el token `Const`, los cinco tipos, `break`, `continue`, `function`, `struct` (token `Struct`), `enum` (token `Enum`), `match` (token `Match`), `return`, `inout` (token `InOut`), `type` (token `TypeOf`) y los booleanos `true` y `false`, además de las instrucciones de impresión.

`quoted()` recoge texto hasta la siguiente comilla del mismo tipo. Para comillas dobles produce un `Value::String`; para comillas simples exige exactamente un valor escalar Unicode y produce `Value::Char`. No procesa escapes, conservando el comportamiento previo de las cadenas.

`number()` reconoce dígitos, una parte decimal opcional y un exponente opcional. Guarda el texto en `Number` para que `number()` del parser lo convierta junto con el posible signo. El signo `-` es un token separado. Esta decisión permite aceptar `-9223372036854775808`: su magnitud positiva no cabe en `i64`, pero el número completo sí.

El scanner reconoce además `[` (`LeftBracket`), `]` (`RightBracket`) y `,` (`Comma`), para tipos de array, literales y accesos. El parser decide qué función cumplen según dónde aparezcan.

El scanner reconoce `:` como `Colon` para campos de una construcción y `::` como `ColonColon` para rutas de biblioteca; `.` sigue siendo `Dot` para campos, métodos de Array y `cast`.

El scanner también reconoce los operadores `+`, `-`, `*`, `/`, `%`, `!`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `&&` y `||`, además de `++`, `--`, `+=` y `-=`. `paired()` mira si un signo lleva un segundo `=`: así distingue la asignación `=` de la igualdad `==`. `compound_or_single()` mira un carácter por delante de `+` o `-` y decide entre el operador doble (`++`, `--`), la variante con `=` (`+=`, `-=`) y el signo aislado. Un `&` o `|` aislado se rechaza. El signo del exponente sigue perteneciendo al número (`1e-2`), mientras que en `2-1` el menos es un token independiente. Como `--` es ahora un token propio, `1--2` ya no significa `1 - (-2)` y se rechaza.

El scanner comprueba la forma del literal numérico; no comprueba declaraciones ni imprime nada.

Para distinguir `25.0` de `25.cast(float)`, `number()` mira el carácter posterior al punto: solo lo consume como decimal si hay un dígito. En el segundo caso emite `Number("25")` y deja que el recorrido principal reconozca `Dot` e `Identifier("cast")`. Los tipos siguen usando `TokenKind::Type`; no se añaden palabras reservadas para Casting.

## 3. Parser y gramática

El **parser**, o analizador sintáctico, comprueba cómo encajan los tokens. [parser.rs](../src/parser.rs) contiene las reglas y las estructuras del AST:

```text
program     → statement* EOF
statement   → simpleStmt ";" | ifStmt | whileStmt | forStmt | foreachStmt | functionStmt | structStmt | enumStmt | matchStmt | "break" ";" | "continue" ";"
simpleStmt  → declaration | assignment | printStmt | importStmt | callStmt | returnStmt
callStmt    → postfix  (LibraryCall, Cast, Call o Qualified con argumentos)
functionStmt→ "function" IDENTIFIER "(" (parameter ("," parameter)*)? ")" ("->" unionType)? block
structStmt  → "struct" IDENTIFIER "{" fieldDef* "}"
enumStmt    → "enum" IDENTIFIER "{" (variantDef ("," variantDef)* ","?)? "}"
variantDef  → IDENTIFIER ("(" type IDENTIFIER ("," type IDENTIFIER)* ")")?
matchStmt   → "match" expression "{" (matchArm ("," matchArm)* ","?)? "}"
matchArm    → IDENTIFIER "::" IDENTIFIER ("(" IDENTIFIER ("," IDENTIFIER)* ")")? "=>" block
qualified   → IDENTIFIER "::" IDENTIFIER ("(" arguments? ")")?
fieldDef    → "const"? unionType IDENTIFIER ("=" expression)? ";"
structExpr  → IDENTIFIER "{" (fieldInit ("," fieldInit)*)? "}"
fieldInit   → IDENTIFIER ":" expression
unionType   → type ("||" type)*
parameter   → "inout"? type IDENTIFIER
returnStmt  → "return" expression?
callExpr    → IDENTIFIER "(" (callArgument ("," callArgument)*)? ")"
callArgument→ expression | "inout" IDENTIFIER
importStmt  → ("import" | "use") path
path        → IDENTIFIER ("::" IDENTIFIER)*
ifStmt      → "if" "(" expression ")" block ("else" (ifStmt | block))?
whileStmt   → "while" "(" expression ")" block
forStmt     → "for" "(" (declaration | assignment) ";" expression ";" assignment ")" block
foreachStmt → "foreach" "(" type IDENTIFIER "in" expression ")" block
block       → "{" statement* "}"
declaration → "const"? unionType IDENTIFIER "=" expression
assignment  → IDENTIFIER ("[" expression "]" | "." IDENTIFIER)* assignTail
assignTail  → "=" expression | "+=" expression | "-=" expression | "++" | "--"
printStmt   → ("print" | "println") "(" expression ")"
type        → (basicType | IDENTIFIER) ("[" "]")*
basicType   → "int" | "float" | "bool" | "char" | "string"
expression  → or
or          → and ("||" and)*
and         → equality ("&&" equality)*
equality    → comparison (("==" | "!=") comparison)*
comparison  → term (("<" | "<=" | ">" | ">=") term)*
term        → factor (("+" | "-") factor)*
factor      → unary (("*" | "/" | "%") unary)*
unary       → ("!" | "-" | "+") unary | postfix
postfix     → primary ("[" expression "]" | "." IDENTIFIER | "." IDENTIFIER "(" arguments? ")"
                      | "." "cast" "(" type ")")*
arguments   → expression ("," expression)*
libraryCall → path "(" arguments? ")"
conversion  → (path "::")? basicType "(" expression ")"
typeCheck   → "type" IDENTIFIER ("." IDENTIFIER)* ("==" | "!=") type
primary     → typeCheck | STRING | CHAR | "true" | "false" | NUMBER | IDENTIFIER
            | qualified | structExpr | libraryCall | callExpr | conversion | "(" expression ")" | "[" (expression ("," expression)*)? "]"
```

`→` significa «se compone de», `|` indica alternativas, `*` permite cero o más repeticiones y `?` indica una parte opcional. `STRING`, `CHAR` y los booleanos son variantes del token `Literal`; `NUMBER` corresponde al token `Number`. El `;` que separa las tres partes de un `for` no pertenece a `declaration` ni a `assignment`: la sentencia simple lo añade al final y `for_statement()` lo exige entre partes. `assignTail` reúne las cuatro modificaciones de una variable ya declarada: asignación, asignación compuesta e incremento/decremento. La flecha `->` es el token `Arrow`; introduce el tipo de retorno de una función y es opcional. `returnStmt` admite `return;` sin expresión, útil en funciones sin valor.

La forma léxica de `NUMBER` es `dígitos ("." dígitos)? (("e" | "E") ("+" | "-")? dígitos)?`. Cada grupo de dígitos contiene al menos uno; el signo inicial se maneja en `unary()`.

En el sufijo con punto, un nombre sin `(` produce acceso a campo. Si hay `(`, `cast` recibe un tipo y los métodos de biblioteca reciben expresiones. Esto permite que un campo se llame `cast` sin confundirse con una conversión. El comprobador restringe los destinos de Casting a tipos básicos y valida la ruta opcional como `std::Casting` o `Casting`. `path` sigue sirviendo sin cambios para las importaciones; `primary()` permite un tipo reservado al final de una ruta de conversión. Al inicio de una instrucción, un tipo seguido de `(` indica una conversión; en otro caso inicia una declaración.

1. `parse()` recoge instrucciones hasta `Eof`.
2. `statement()` distingue por el primer token las sentencias simples (declaración, modificación de variable, llamada, impresión), que exigen el `;` final, y las que terminan en `}` (`struct`, `enum`, `match`, `function`, `if`, `while`, `for` y `foreach`), que no lo llevan. Ante un identificador seguido de otro nombre (con posibles pares `[]` intermedios) o de `||`, `starts_named_declaration()` distingue una declaración con tipo nombrado. No necesita saber si el tipo existe; eso corresponde al comprobador. Para el resto de identificadores, paréntesis o corchetes analiza `postfix()`: si el nodo exterior es una llamada de biblioteca, una conversión o una llamada propia, lo envuelve en `Stmt::Call`; en otro caso vuelve al inicio para analizar la asignación. Este paso solo construye el árbol, sin ejecutar sus expresiones. `declaration()` consume el `const` opcional, delega el tipo en `union_type()` y recoge nombre e inicializador. `assignment()` recoge el nombre y una ruta opcional de campos e índices y, según el token siguiente, construye una asignación (`=`), una asignación compuesta (`+=`, `-=`) o un incremento/decremento (`++`, `--`); se separa de `statement()` para poder reutilizarla en la cabecera de un `for`. `array_type()` consume el tipo básico o nombrado y envuelve cada par `[]` en `Type::Array`.
3. `if_statement()` consume `if`, exige la condición entre paréntesis y analiza un bloque. Si aparece `else`, analiza otro bloque o encadena un `if` anidado. `block()` recoge instrucciones hasta `}` y avisa si se alcanza el final del archivo.
4. `while_statement()` consume `while`, exige la condición y un bloque. `for_statement()` exige `(`, analiza como inicialización una declaración o una modificación de variable, y a continuación la condición y la actualización separadas por `;`; la actualización admite asignación, asignación compuesta o incremento. `foreach_statement()` exige el tipo, el nombre, la palabra reservada `in`, el array y un bloque. `break` y `continue` se analizan como instrucciones simples y requieren `;`. Una declaración `function` se reconoce al inicio de `statement()` y no exige `;`; `function_declaration()` consume `function`, el nombre, la lista de parámetros `inout? tipo nombre` separados por comas, un `-> unionType` opcional y un bloque. `union_type()` recoge las alternativas separadas por el token existente `OrOr`, elimina duplicados y crea `Type::Union` cuando queda más de una; `array_type()` sigue atendiendo cada alternativa concreta y los tipos de parámetros, `foreach`, conversiones y comprobaciones `type`. `union_type()` también se usa en declaraciones de variables, constantes y campos de estructuras. `return_statement()` reconoce `return` seguido de una expresión o de `;`, y la sentencia simple añade el `;` final.
5. `name()` exige un identificador y conserva su texto y línea en `Name`.
6. `print_statement()` exige los paréntesis alrededor de una expresión.
7. `expression()` baja por niveles de precedencia: `or()`, `and()`, `equality()`, `comparison()`, `term()`, `factor()`, `unary()`, `postfix()` y `primary()`.
8. Cada nivel binario usa `binary()` para encadenar sus operadores de izquierda a derecha. Cada operando se analiza en el siguiente nivel, que tiene mayor precedencia.
9. `unary()` admite signos y negación lógica de forma recursiva. Si `-` precede directamente a un token `Number`, llama a `number(true, line)` y convierte juntos signo y dígitos para aceptar el mínimo de `i64`. Los demás unarios generan un nodo `Unary`. Después de convertir un número con signo, `finish_postfix()` consume posibles índices para que también se comprueben accesos inválidos como `-1[0]`.
10. `postfix()` y `finish_postfix()` construyen un nodo `Index` por cada acceso y un `Field` por cada nombre tras el punto sin paréntesis, y un `LibraryCall` por cada método con punto, salvo `cast`, que produce `Cast`. `index()` recoge la expresión del índice y la línea del corchete de apertura, y exige el cierre. La misma regla se usa al asignar elementos.
11. `primary()` crea literales básicos, referencias, arrays (`Expr::Array`), llamadas de biblioteca por ruta (`Expr::LibraryCall`), conversiones (`Expr::Cast`) o analiza una expresión entre paréntesis. Un único identificador seguido de `(` produce `Expr::Call`, una llamada a una función propia; una ruta de dos nombres con `::` produce `Expr::Qualified` y una ruta más larga conserva `Expr::LibraryCall`. Un único identificador seguido de `{` llama a `struct_literal()` y produce `Expr::Struct` con pares de nombre y expresión en el orden escrito, sin coma final. Sin paréntesis ni llave y con un solo nombre produce `Expr::Variable`. `struct_declaration()` recoge el nombre del tipo y cada `FieldDef`: nombre, tipo concreto o unión, marca `const` opcional y expresión de valor por defecto opcional. No ejecuta ni construye valores. Reutiliza los tokens `Const`, `OrOr` y `Equal`. En un array recoge expresiones separadas por comas, sin coma final. `number()` convierte a `i64` o `f64`, rechazando enteros fuera de rango y float no finitos. Por ello `-9223372036854775808` es válido, pero `-(9223372036854775808)` se rechaza: el literal positivo interior ya está fuera de rango.

`peek()` consulta el token actual y `consume()` exige un token concreto y avanza. El análisis es **descendente**: empieza en el programa y baja hacia sus componentes. Los paréntesis cambian la agrupación del árbol sin necesitar un nodo propio.

El parser acepta la estructura de `float precio = 25;`: las piezas están bien colocadas. Es la siguiente etapa la que detecta que un `int` no puede inicializar un `float`.

## 4. AST, tipos y valores

**AST** significa árbol de sintaxis abstracta. Guarda lo necesario para ejecutar el programa; los delimitadores ya han cumplido su función. Las expresiones producen valores y las instrucciones realizan acciones:

```text
Expr
├── Qualified { path: Vec<Name>, arguments: Option<Vec<Expr>> }
├── Struct { name: Name, fields: Vec<(Name, Expr)> }
├── Field { object: Box<Expr>, name: Name }
├── LibraryCall { path: Vec<Name>, receiver: Option<Box<Expr>>, arguments: Vec<Expr> }
├── Call { name: Name, arguments: Vec<CallArgument> }
├── Cast { path: Vec<Name>, target: Type, value: Box<Expr> }
├── TypeCheck { name: Name, fields: Vec<Name>, target: Type, negated: bool }
├── Literal(Value)
├── Variable(Name)
├── Array { elements: Vec<Expr>, line, element_type: RefCell<Option<Type>> }
├── Index { array: Box<Expr>, index: Box<Expr>, line }
├── Unary { operator: UnaryOp, operand: Box<Expr>, line }
└── Binary { left: Box<Expr>, operator: BinaryOp, right: Box<Expr>, line }

Stmt
├── Enum { name: Name, variants: Vec<VariantDef> }
├── Match { value: Expr, arms: Vec<MatchArm>, line }
├── Struct { name: Name, fields: Vec<FieldDef> }
├── Call(Expr)
├── Function { name: Name, parameters: Vec<Parameter>, return_type: Option<Type>, body: Vec<Stmt>, line }
├── Return { value: Option<Expr>, line }
├── Import { path: Vec<Name>, is_use: bool, line }
├── Declare { declared_type, is_constant, name, initializer }
├── Assign { name, steps: Vec<TargetStep>, value }
├── CompoundAssign { name, steps, operator: AssignOp, value, line }
├── Increment { name, steps, operator: IncrementOp, line }
├── If { condition, then_branch: Vec<Stmt>, else_branch: Option<Vec<Stmt>>, line }
├── While { condition, body: Vec<Stmt>, line }
├── For { initializer: Box<Stmt>, condition, update: Box<Stmt>, body: Vec<Stmt>, line }
├── Foreach { declared_type, name, iterable, body: Vec<Stmt>, line }
├── Print(Expr)
└── Println(Expr)
```

`FieldDef` conserva `name`, `declared_type`, `is_constant` y `default_value: Option<Expr>`. Guardar la expresión permite evaluarla para cada instancia, en vez de compartir un valor calculado al declarar el tipo. `Expr::TypeCheck` guarda una variable raíz y una cadena de nombres de campo; no admite índices ni llamadas. `Expr::field_path()` reconoce esas mismas rutas estables para los refinamientos.

`TargetStep` distingue `Index(Expr, línea)` de `Field(Name)`. Las modificaciones guardan esta ruta sobre una variable raíz. `Expr::into_target()` extrae la misma ruta de una lectura para que `push` y `pop` también puedan modificar arrays dentro de campos.

`Parameter` conserva `declared_type`, `name` e `is_inout`. `CallArgument` distingue `Value(Expr)` de `InOut(Name)`. `user_arguments()` reconoce la marca solo en llamadas propias y exige un nombre completo sin índices ni operaciones. La lista de biblioteca sigue usando `arguments()` y `Vec<Expr>`.

Cada rama de un `if` o `match` y cada cuerpo de bucle es un `Vec<Stmt>`: la lista de instrucciones de su bloque. `else_branch` guarda `None` si no hay `else`; un `else if` queda como un `Vec` con un único `Stmt::If` interior. En `For`, `initializer` y `update` son instrucciones completas (`Declare`, `Assign`, `CompoundAssign` o `Increment`), guardadas en `Box` para no dar un tamaño infinito al enum. `AssignOp` distingue `+=` de `-=` e `IncrementOp`, `++` de `--`; cada uno sabe qué operación binaria equivale a la forma abreviada.

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

[value.rs](../src/value.rs) distingue `Type`, que identifica un tipo básico, `Named(String)` (el nombre de una estructura o enum), `Array(Box<Type>)` o `Union(Vec<Type>)`, de `Value`, que además contiene el dato: `Int(i64)`, `Float(f64)`, `Bool(bool)`, `Char(char)`, `String(String)`, `Struct { name: String, fields: Vec<(String, Value)> }`, `Enum { name: String, variant: String, values: Vec<Value> }` o `Array { elements: Vec<Value>, element_type: Type }`. `Type::Array` conserva el tipo de elemento; la longitud no forma parte del tipo. `Type` usa `Clone` en lugar de `Copy` porque puede contener otro tipo mediante `Box`.

`value_type()` devuelve `Type`: todo valor conserva un tipo concreto, incluidos los arrays vacíos. Para arrays usa `element_type`, no deduce el tipo mirando su primer elemento. El comprobador calcula el tipo de cada `Expr::Array` y lo guarda en su campo `element_type: RefCell<Option<Type>>`. `RefCell` permite completar ese dato durante el análisis sin cambiar las referencias compartidas al resto del árbol; el parser deja `None` y el intérprete solo recibe arrays ya comprobados con `Some(tipo)`. Si el comprobador prueba varias alternativas de una unión, vuelve a comprobar la alternativa elegida para fijar también las anotaciones de los arrays interiores. Al ejecutar, el literal construye `Value::Array` con sus elementos y ese tipo concreto. Copias, argumentos, retornos y operaciones conservan la información.

`UnaryOp` y `BinaryOp` representan las operaciones sin depender de los tokens del scanner. Cada nodo operador guarda su línea para los errores. `Box<Expr>` guarda una subexpresión mediante un puntero: permite que el árbol sea recursivo sin que cada nodo necesite un tamaño infinito.

El parser copia el contenido de los tokens al árbol. Para evaluar un literal, el intérprete obtiene ese valor; para evaluar una referencia, debe consultar el entorno.

## 5. Comprobación de tipos antes de ejecutar

[type_checker.rs](../src/type_checker.rs) introduce `TypeChecker` y una pila de ámbitos `Vec<HashMap<String, VariableInfo>>`. Un **entorno** relaciona nombres con información; en esta etapa cada `VariableInfo` contiene `declared_type` (el tipo declarado), `narrowed_type` (el tipo más concreto de la variable conocido por una condición, o `None`), `narrowed_fields` (tipos conocidos por rutas de campos), `is_constant` (si se prohíbe modificar) e `is_inout` (si el parámetro puede compartir almacenamiento con otros alias), sin guardar valores. El último mapa de la pila es el ámbito actual; `lookup()` busca desde el más interno hacia fuera. Además guarda `functions: HashMap<String, FunctionSignature>`; una **firma** conserva el tipo y la marca `inout` de cada parámetro (`Vec<(Type, bool)>`) y, si la función devuelve un valor, su tipo de retorno (`return_type: Option<Type>`). También guarda `return_types`, una pila que indica el tipo esperado dentro de un `return`; estar vacía significa que se está fuera de cualquier función. También conserva `structures: HashMap<String, Vec<FieldDef>>`, que relaciona cada tipo nombrado con los campos en el orden de declaración. También guarda `enums: HashMap<String, Vec<VariantDef>>`, con las variantes y sus datos. Estos registros se reinician al empezar cada archivo.

`validate_type()` comprueba los nombres de estructuras y enums dentro de anotaciones, arrays y uniones. Al declarar una estructura, se registra su nombre antes de validar los campos: así puede nombrarse a sí misma, además de usar tipos anteriores. `has_finite_alternative()` rechaza un campo que solo pueda contener la propia estructura: un array puede terminar vacío y una unión puede terminar en una alternativa no recursiva. No se admiten tipos adelantados ni recursión mutua. `field_definition()` exige un tipo estructura concreto y busca el campo en el registro; `field_type()` obtiene su tipo declarado.

Los valores por defecto se comprueban al declarar el tipo. Se prepara un contexto con globales y campos anteriores de solo lectura; se ocultan todos los nombres de campo del mapa global para que un campo posterior no se resuelva por accidente como un global homónimo. No se heredan refinamientos globales de una condición anterior. Las funciones y bibliotecas deben estar disponibles en ese punto. El tipo del campo sirve de contexto a su expresión, incluidos arrays vacíos y uniones. Se anotan los arrays en el AST original, que después usará el intérprete. Pasar un campo anterior o un global como `inout`, o usar `push`/`pop` sobre ellos, se rechaza; una función por valor puede operar con su copia.

`check()` abre el ámbito global y recorre las instrucciones en orden:

- En una declaración, rechaza nombres repetidos, obtiene el tipo del inicializador usando el tipo declarado como contexto y exige que todos sus tipos posibles estén incluidos en el declarado mediante `Type::accepts()`. Solo entonces registra el nombre junto con su tipo y la marca `is_constant`. Así `int x = x;` falla: `x` todavía no está disponible. En el ámbito global también rechaza un nombre que coincida con una función ya declarada.
- En una declaración `function`, exige el ámbito global (`scopes.len()` debe ser 1), rechaza un nombre de función repetido o que coincida con una variable global, y rechaza parámetros con el mismo nombre. Registra la firma, con su tipo de retorno, antes de comprobar el cuerpo, de modo que la función puede llamarse a sí misma. Después abre un ámbito, registra cada parámetro como variable no constante, apila el tipo de retorno en `return_types` y comprueba el cuerpo. Al terminar, si la función declara `-> tipo` exige que `always_returns()` garantice un `return` con valor en todos los caminos; el análisis es conservador y solo acepta un `return` directo o un `if`/`else` con ambas ramas devolviendo. Al cerrar el ámbito, los parámetros dejan de existir.
- En un `return`, comprueba que la pila `return_types` no esté vacía; si no, el error indica que solo se permite dentro de una función. Con tipo de retorno declarado, exige una expresión compatible mediante `Type::accepts()`: un tipo concreto debe coincidir exactamente y una unión debe incluir todos los tipos posibles de la expresión; sin él, exige `return;` sin expresión. En una llamada `Expr::Call`, exige un nombre declarado antes, el número exacto de argumentos y tipos idénticos, sin conversiones. También exige que las marcas `inout` coincidan: para `InOut(Name)` valida el destino con `assignment_target()`, y para `Value(Expr)` comprueba la expresión con el tipo esperado. `check_call()` devuelve `Some(tipo)` si la función devuelve un valor y `None` si no; por eso una llamada sin valor solo vale como instrucción y una función declarada más adelante no está disponible.
- En una asignación, busca la información del nombre y rechaza la operación si `is_constant` es `true`, incluso si el valor no cambiaría. Dentro de una función también rechaza destinos globales: si `return_types` no está vacío, el nombre debe encontrarse en algún ámbito posterior al global. Esta regla se aplica también al paso de argumentos `inout` y a los métodos de modificación de arrays. Para las demás variables, resuelve el tipo del destino: sin campos ni índices es el declarado; cada campo comprueba su marca `const` y cada índice exige un array y un `int`, y desciende al tipo de elemento. La asignación simple usa el tipo declarado del último campo para permitir cambiar la alternativa de una unión; los accesos intermedios usan los tipos refinados. Compara ese tipo con el de la expresión asignada y lo proporciona como contexto para arrays vacíos. No cambia la anotación declarada ni declara variables nuevas. Una asignación completa descarta los refinamientos de la raíz y sus campos; reemplazar un campo descarta su ruta y las rutas interiores. Los accesos intermedios requieren un tipo concreto comprobado. Las asignaciones compuestas y los incrementos usan el tipo refinado y lo conservan. `assignment_target()` reúne la búsqueda, el rechazo de constantes y el recorrido de campos e índices; lo comparten la asignación, la compuesta y el incremento.
- En una asignación compuesta (`+=`, `-=`), obtiene el tipo del destino como en una asignación y el de la derecha, y aplica las reglas de `+` o `-` mediante `binary_result()`. El destino debe admitir la operación con el tipo de la derecha: `+=` vale para `int`, `float` y `string`, y `-=` para `int` y `float`. El error señala la línea del operador.
- En un incremento o decremento (`++`, `--`), el destino debe ser `int` o `float`; no hay operando derecho que comprobar. El paso de una unidad se decide al ejecutar a partir del tipo del destino.
- En una impresión, comprueba que la expresión sea válida; una referencia debe existir previamente.
- En un `if`, obtiene el tipo de la condición y exige `Bool` (si no, el error señala la línea del `if`). `with_condition()` prepara copias del contexto con los tipos conocidos para verdadero y falso. Después abre un ámbito para cada rama y lo cierra al terminar; `merge_scopes()` reúne los tipos posibles al terminar ambas ramas. Si hay `else`, repite el proceso con su propia lista de instrucciones, de modo que las dos ramas se comprueban aunque solo se vaya a ejecutar una. `check_block()` hace el `push` y el `pop` del ámbito; declarar un nombre solo comprueba duplicados en el ámbito actual, así que se permite reutilizar el nombre en un bloque interior.
- En un `while`, olvida refinamientos sobre nombres reasignados por el cuerpo, exige que la condición sea `Bool` (error en la línea del `while`) y comprueba el cuerpo en un ámbito nuevo con lo sabido cuando la condición es verdadera.
- En un `for`, abre un ámbito para todo el bucle, comprueba la inicialización (una declaración o una asignación), descarta refinamientos sobre nombres reasignados en cuerpo y actualización y exige `Bool` a la condición. Con lo sabido cuando es verdadera, comprueba el cuerpo en un ámbito interior y después la actualización; el contador declarado en la inicialización deja de existir al cerrar el ámbito del bucle. `check_statement()` permite reutilizar la lógica de declaración y asignación sin duplicarla.
- En un `foreach`, obtiene el tipo de la expresión y exige que sea `Type::Array`. El tipo del elemento debe coincidir exactamente con el declarado; si no, el error señala el nombre. Registra la variable del bucle en un ámbito nuevo y comprueba el cuerpo en un ámbito interior.

El mensaje de condición incorrecta lo produce `require_bool()`, compartido por `if`, `while` y `for`, e indica la línea del bucle.

Para `int edad = 25;`, `expression_type()` obtiene `Int` del literal. Coincide con la anotación y se guarda `edad → VariableInfo { declared_type: Int, narrowed_type: None, narrowed_fields: {}, is_constant: false, is_inout: false }`. Cuando llega `println(edad);`, la consulta obtiene `Int` del campo `declared_type`.

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

El tipo explícito obligatorio, la compatibilidad sin conversiones implícitas y la comprobación previa son decisiones relacionadas pero distintas. Para una unión, la compatibilidad significa que todos los tipos posibles del valor están permitidos por la anotación. Esta etapa es una adaptación propia: el intérprete Lox del libro comprueba tipos durante la ejecución.

`expression_type_expected()` comprueba una expresión con un tipo esperado opcional. Para `Expr::Array`, usa el tipo del elemento esperado o, si no existe, el de la primera expresión. Comprueba todos los elementos recursivamente con ese contexto y exige igualdad exacta. Un literal vacío sin contexto produce un error en su corchete de apertura: no hay un elemento que permita determinar su tipo. No busca un tipo en elementos posteriores ni lo propaga entre operandos de una operación. Por eso `int[][] a = [[], [1]];` es válido y `println([[], [1]]);` no lo es. Los paréntesis no eliminan el contexto porque no generan un nodo propio.

`indexed_type()` exige `Type::Array` en el objeto y `Type::Int` en el índice, tanto para `Expr::Index` como para cada índice de `Stmt::Assign`. Devuelve el tipo del elemento. No comprueba límites aquí: la longitud y el índice son valores de ejecución. La marca `is_constant` se comprueba antes de recorrer los índices y protege todo el valor, incluidos arrays interiores.

En `Unary` y `Binary`, `expression_type()` comprueba recursivamente los operandos y aplica las reglas de cada operador. Exige igualdad de tipos entre ambos operandos; aritmética conserva el tipo numérico, concatenación produce `String`, y comparaciones y lógica producen `Bool`. La igualdad y desigualdad admiten arrays, estructuras o enums del mismo tipo; las demás operaciones no admiten estos valores completos. Comprueba ambos lados de `&&` y `||` aunque después pueda omitirse uno. Así `true || desconocida` y `false && 1` fallan antes de emitir salida. Las reglas comunes viven en `binary_result()`, que devuelve `None` cuando los operandos no comparten tipo o el operador no los admite; la asignación compuesta la reutiliza con `+` o `-`.

## 6. Intérprete y entorno de valores

[interpreter.rs](../src/interpreter.rs) contiene `Interpreter<W: Write>`. Recibe un programa validado y guarda otro entorno: ahora una pila de ámbitos `Vec<HashMap<String, Binding>>`, porque los bloques de un `if` o `match`, los bucles y las llamadas a funciones pueden anidarse. El primer mapa es el ámbito global. También guarda `functions`, un mapa del nombre de cada función propia a su cuerpo ya resuelto (compartido con `Rc` para no duplicar las instrucciones en cada llamada), `structures`, un mapa del nombre de cada estructura a sus definiciones de campo en orden (`Rc<Vec<FieldDef>>`), `enums`, un conjunto de nombres de enum registrados, y `call_bases`, una pila con el índice donde comienza cada llamada o ámbito de valores por defecto. Los contadores separados `function_depth` y `construction_depth` limitan conjuntamente a 100 las llamadas y construcciones simultáneas. `Binding::Owned(Value)` guarda un valor propio; `Binding::Alias { scope, name }` apunta al almacenamiento original de un parámetro `inout`. El alias no es un `Value`: no puede devolverse ni guardarse dentro de un array. Un `return` se representa con la señal `Control::Return`, que recorre bloques y bucles hasta la llamada más cercana.

`interpret()` llama a `execute()` para cada instrucción. Una declaración evalúa el inicializador y guarda el resultado en el ámbito actual (el último de la pila). Una asignación busca el ámbito que contiene el nombre, resuelve primero el destino y comprueba todos sus índices de izquierda a derecha; después evalúa el nuevo valor y sustituye el anterior. Si falla un índice o la expresión asignada, no modifica el destino. `resolve_target()` devuelve el ámbito y las posiciones ya validadas, y `write_target()` escribe en ellas; entre ambos, `target_value()` obtiene una copia del valor actual. Una asignación compuesta evalúa la derecha, aplica `binary()` con el operando actual y escribe el resultado; un incremento usa `1` o `1.0` como paso según el tipo del destino. Si la operación desborda, no se escribe nada. `evaluate()` devuelve una copia del literal o del valor consultado; esto también copia el contenido de las cadenas, de los campos de estructuras y de todos los arrays anidados, y evita que dos variables compartan cambios. Para consultar una variable, `scope_containing()` busca desde el bloque actual hasta el inicio de la llamada y después en el global. Los ámbitos del llamador quedan fuera de esa búsqueda, aunque sigan vivos en la pila. Un nombre local oculta al global; `storage_location()` sigue los alias para leer o escribir en el almacenamiento original. Los parámetros normales y los valores devueltos siguen siendo copias independientes.

Un `Stmt::If` evalúa su condición y, según sea `true` o `false`, ejecuta el bloque `then` o el `else`. `execute_block()` abre un ámbito con `push`, ejecuta sus instrucciones y lo cierra con `pop`; si una instrucción falla, el error se propaga y el programa se detiene. Como el comprobador ya garantizó que las condiciones son `bool`, el intérprete no repite esa comprobación de tipos.

Los bucles usan la misma pila. `Stmt::While` evalúa la condición antes de cada vuelta y ejecuta el cuerpo con `execute_block()`. `Stmt::For` abre un ámbito, ejecuta la inicialización una vez y repite condición, cuerpo y actualización. `Stmt::Foreach` evalúa el array una sola vez (obtiene una copia), abre un ámbito y, por cada elemento, guarda una copia en la variable del bucle y ejecuta el cuerpo. Las instrucciones de control devuelven una señal `Control`: `Break`, `Continue` o `Return(Option<Value>)`; `execute_block()` las propaga por bloques y ramas `if` anidadas, cerrando el ámbito antes de salir. Cada bucle consume `Break` y `Continue`, pero deja pasar `Return` hacia la llamada que lo envuelve: `while` reevalúa la condición al continuar; `for` ejecuta la actualización también al continuar; `foreach` salta al próximo elemento; un `return` interrumpe el bucle y sale de la función. El ámbito del contador o elemento se cierra al terminar.

Por ejemplo, con `for (int i = 0; i < 4; i++) { if (i == 1) { continue; } if (i == 3) { break; } print(i); }`, se imprime `02`: el `continue` omite la impresión de `1` pero ejecuta `i++`, y el `break` sale antes de imprimir `3`. `TypeChecker` lleva una profundidad de bucle mientras comprueba cuerpos; así admite saltos bajo `if` dentro del bucle y rechaza ambos fuera de cualquier bucle antes de ejecutar.

En el ejemplo, se almacena `edad → Value::Int(25)`. La impresión consulta ese valor y escribe `25` seguido de un salto de línea.

Si se añaden estas instrucciones válidas:

```oki
edad = 26;
println(edad);
```

El entorno se actualiza a `edad → Value::Int(26)` y se imprime `26` en otra línea. El entorno de tipos sigue indicando `Int`.

`Value` implementa `Display`, la capacidad de formatear un dato en Rust. Cadenas y caracteres se escriben sin comillas, booleanos como `true` o `false`, enteros en decimal y float mediante el formato que mantiene `1.0` distinguible de `1`. Las estructuras se formatean como `Nombre { campo: valor, ... }` en orden de declaración. Los enums muestran `Nombre::Variante` y, si los tienen, sus datos entre paréntesis. Los arrays se formatean entre corchetes y sus elementos se separan por comas. Las cadenas y caracteres interiores se rodean de comillas conservando su contenido sin escapes, por lo que no se garantiza que esta salida sea código reutilizable. El formateo para imprimir no convierte el tipo de una variable.

`write!` implementa `print` y `writeln!` implementa `println`. El destino genérico `W: Write` permite usar tanto la salida estándar como un `Vec<u8>` en las pruebas. No se vuelve a analizar texto al ejecutar.

### Recorrido de un enum y match

```oki
enum Resultado { Ok(int valor), Error(string mensaje) }
Resultado resultado = Resultado::Ok(5);
match resultado {
    Resultado::Ok(numero) => { println(numero + 1); },
    Resultado::Error(texto) => { println(texto); }
}
```

1. El scanner reconoce `enum`, `match` y `=>` como `Enum`, `Match` y `FatArrow`. Conserva `ColonColon` para `::`, `EqualEqual` para `==` y `Arrow` para `->`.
2. `enum_declaration()` construye `Stmt::Enum` con dos `VariantDef`: `Ok → [(Int, valor)]` y `Error → [(String, mensaje)]`. La anotación de la variable es `Type::Named("Resultado")`, igual que cualquier tipo propio nombrado. Los registros separados del comprobador distinguen estructuras y enums, evitando expandir sus definiciones dentro de `Type`.
3. `primary()` conserva `Resultado::Ok(5)` en `Expr::Qualified { path: [Resultado, Ok], arguments: Some([Literal(Int(5))]) }`. Para una variante sin datos, `arguments` es `None`. No decide si la ruta corresponde a un enum o a una biblioteca. `qualified_type()` consulta el registro de enums y valida la variante, aridad y tipos; si hay argumentos y el nombre no es un enum, delega en `check_library_call()`. Las llamadas cortas `Array::len(...)` siguen funcionando. Si un enum se llama `Array`, sus rutas de dos nombres corresponden al enum; la biblioteca conserva su ruta completa `std::Array::len(...)`.
4. `match_statement()` produce `Stmt::Match` con la expresión consultada y una lista de `MatchArm`. Cada rama conserva enum, variante, capturas opcionales y cuerpo. La llave después de un nombre simple abre las ramas; no inicia un literal de estructura. Un literal de estructura como sujeto puede agruparse entre paréntesis y será rechazado por no ser enum.
5. El comprobador exige `Named("Resultado")` registrado como enum. Comprueba pertenencia, variantes únicas y cobertura de todas ellas. Cada rama parte del mismo entorno posterior a evaluar el sujeto, abre un ámbito y registra `numero → Int` o `texto → String` como variables locales por valor. Comprueba todas las instrucciones, cierra el ámbito y reúne la información posible de las ramas con `merge_scopes()`. Los efectos del sujeto y de los argumentos de construcción invalidan refinamientos en orden; las escrituras dentro de `match` se incluyen al analizar siguientes vueltas de un bucle. `always_returns()` reconoce un `match` exhaustivo cuyos cuerpos garantizan retorno.
6. El intérprete registra el nombre al ejecutar `Stmt::Enum`. `evaluate_qualified()` evalúa los argumentos una vez, de izquierda a derecha, y crea `Value::Enum { name: "Resultado", variant: "Ok", values: [Int(5)] }`. Reutiliza el límite de profundidad de valores, incluyendo los datos interiores del enum. Las llamadas de biblioteca delegan en `evaluate_library_call()`; comprobación y ejecución usan las expresiones originales, conservando los tipos ya fijados de arrays vacíos.
7. `execute_match()` evalúa el sujeto una vez y busca la rama `Ok`. Abre un ámbito con la copia `numero → Int(5)`, ejecuta su cuerpo mediante `execute_statements()` y lo cierra, también si hay error, retorno o salto. La salida es `6` y `resultado` conserva `Ok(5)`. Las señales de control viajan hasta la función o bucle correspondiente. `execute_block()` usa el mismo cierre de ámbito ante errores.
8. La impresión representa la variante como `Resultado::Ok(5)`; la igualdad deriva de nombre, variante y contenido. El comprobador solo permite compararla con otro valor del mismo tipo nominal. No hay acceso por campo a un enum: el dato se extrae en el patrón, y esa captura es una copia profunda.

`execute()` y `evaluate()` separan el despacho de llamadas, retornos y `match` del resto de operaciones. Esto reduce las variables temporales que ocupan pila en llamadas recursivas con `match` y permite alcanzar el límite existente de 100 llamadas con un error del lenguaje. Las dieciocho pruebas de [enum_tests.rs](../src/enum_tests.rs) cubren este recorrido y sus errores; el programa completo está en [enums.oki](../examples/enums.oki).

### Recorrido de una estructura

Con esta entrada:

```oki
struct Persona { string nombre; int edad; }
Persona ana = Persona { edad: 30, nombre: "Ana" };
Persona copia = ana;
copia.edad++;
println(ana);
println(copia.edad);
```

El scanner añade `Struct` y reconoce `Colon` para cada `:`. El nombre `Persona` sigue siendo un identificador: el scanner no decide si un nombre representa un tipo o una variable. El parser construye:

```text
Struct(Persona, [FieldDef(nombre, String, no constante, sin defecto),
                 FieldDef(edad, Int, no constante, sin defecto)])
Declare(ana, Named("Persona"),
        Struct(Persona, [(edad, Literal(Int(30))), (nombre, Literal(String("Ana")))]))
Declare(copia, Named("Persona"), Variable(ana))
Increment(copia, steps: [Field(edad)], Increment)
Println(Variable(ana))
Println(Field(Variable(copia), edad))
```

El comprobador registra los dos `FieldDef` de `Persona`, valida la anotación `Named("Persona")` y comprueba que el literal contiene ambos campos una vez y con su tipo exacto. El nombre define la identidad del tipo; otra estructura con los mismos campos sigue siendo distinta. El campo array recibe su tipo declarado como contexto, igual que una variable array, lo que permite `struct Caja { int[] datos; }` y `Caja { datos: [] }`.

El intérprete registra el orden de campos al ejecutar `Stmt::Struct`. Al evaluar la construcción, obtiene primero `30` y luego `"Ana"`, según el orden escrito. Reordena los valores ya evaluados según la declaración y guarda `Value::Struct { name: "Persona", fields: [("nombre", String("Ana")), ("edad", Int(30))] }`. Esta representación hace que impresión e igualdad no dependan del orden de construcción. La copia de `ana` clona todo el valor. `resolve_target()` convierte `[Field(edad)]` en una ruta evaluada con `TargetPosition::Field { name: "edad", structure: "Persona" }`; `target_value()` lee `30`, el incremento calcula `31` y `target_mut()` encuentra el campo que `write_target()` sustituye. `ana` mantiene `30`. Se imprime `Persona { nombre: "Ana", edad: 30 }` y después `31`.

En `grupo.personas[0].edad` la ruta combina `Field(personas)`, `Index(Int(0), línea)` y `Field(edad)`. Los índices se evalúan una sola vez, en orden, antes del valor de la derecha. Cada posición guarda también el tipo del elemento array o el nombre nominal de la estructura. Se vuelve a recorrer la ruta al leer o escribir, comprobando límites y tipos: un `pop` interior puede eliminar una posición y una llamada `inout` puede reemplazar una alternativa unión. En este último caso se rechaza escribir en un destino que haya cambiado de tipo, aunque la nueva estructura tenga un campo con el mismo nombre. `push` y `pop` conservan y vuelven a comprobar el tipo del array destino. `assignment_target()` protege la variable raíz frente a `const` y la escritura directa de globales en funciones. Esta protección se aplica a toda la ruta y también a `push`/`pop` sobre un campo array. Se comprueba además la marca `const` de cada campo atravesado, para proteger todo su contenido.

Las funciones usan `Named("Persona")` como cualquier tipo concreto en sus firmas; los parámetros normales y retornos copian el valor, mientras que `inout` mantiene el alias de la variable completa. `type dato == Persona` compara `value_type()` con el tipo nombrado y reutiliza los refinamientos de variables unión. Casting excluye `Struct` explícitamente, para que las reglas generales de conversión a texto no lo acepten por accidente. Todos los campos son públicos. No se incorporan métodos propios ni referencias compartidas; la recursión usa valores finitos independientes.

### Campos constantes, valores por defecto y uniones

Esta entrada combina las tres ampliaciones:

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
dato.valor = "listo";
if (type dato.valor == string) {
    println(dato.valor + "!");
}
```

El parser guarda tres `FieldDef`: `id` es constante y no tiene defecto; `doble` tiene la expresión `Binary(Multiply, Variable(id), Literal(Int(2)))`; `valor` tiene el tipo `Union([Int, String])` y el defecto `Literal(Int(0))`. El literal de construcción solo contiene el par explícito `(id, Literal(Int(3)))`. El comprobador permite omitir `doble` y `valor`, valida sus expresiones con los campos anteriores disponibles y rechaza cualquier escritura posterior que atraviese `id`.

Al ejecutar, `construct()` evalúa primero todos los campos explícitos una sola vez, en el orden escrito y en el contexto del llamador. Después abre un ámbito aislado para recorrer la definición en orden. Guarda `id → Int(3)` como copia de lectura para los siguientes valores por defecto; obtiene `doble → Int(6)` y `valor → Int(0)`. El resultado es `Value::Struct { name: "Registro", fields: [("id", Int(3)), ("doble", Int(6)), ("valor", Int(0))] }`. Cada instancia vuelve a calcular los defectos omitidos; un campo proporcionado evita evaluar su defecto. Estos ámbitos y sus entradas de `call_bases` se cierran también ante errores. Las globales se leen con su valor actual; las variables locales del llamador quedan ocultas.

La condición guarda `TypeCheck { name: dato, fields: [valor], target: Int, negated: false }`. `assume_condition()` añade la ruta `["valor"] → Int` a `narrowed_fields`; `path_type()` la consulta al comprobar el incremento. La asignación simple posterior usa el tipo declarado `int || string` y borra lo sabido de esa ruta. La nueva comprobación permite concatenar texto. El intérprete recorre el valor concreto del campo al evaluar cada condición. Se imprime `6`, `1` y `listo!`, cada uno en su línea.

Los refinamientos siguen rutas de nombres sin índices. Reemplazar un campo descarta lo conocido de él y de sus campos interiores; reemplazar la raíz descarta todas sus rutas. `forget_expression_effects()` recorre las llamadas y elimina información de campos cuando se pasa su raíz como `inout`. Si esa raíz ya es un parámetro `inout`, también elimina información de otros parámetros `inout` y globales, porque pueden ser alias del mismo almacenamiento. Al comprobar los operandos, argumentos, índices y campos explícitos, se consideran los efectos anteriores en el orden de evaluación. Por ejemplo, una prueba sobre `dato.valor` deja de servir tras una llamada que recibe `inout dato`, aunque la función concreta no modifique ese campo.

Las ramas conservan solo las rutas conocidas en ambos caminos y reúnen sus posibles tipos; los bucles descartan antes de comprobar su cuerpo los hechos que sus escrituras o llamadas puedan invalidar. Las funciones comprueban su cuerpo sin heredar refinamientos globales del punto de declaración. Son decisiones conservadoras: puede ser necesario repetir una comprobación de tipo. No se añade análisis de índices ni de lo que hace cada función.

### Recorrido de una estructura recursiva

```oki
struct Nodo { int valor; Nodo[] hijos = []; }
Nodo raiz = Nodo { valor: 1, hijos: [Nodo { valor: 2 }] };
Nodo copia = raiz;
copia.hijos[0].valor = 9;
println(raiz.hijos[0].valor);
println(copia.hijos[0].valor);
```

El comprobador registra `Nodo` antes de validar `Array(Named("Nodo"))`. El array vacío permite terminar: el nodo interior omite `hijos` y recibe un array vacío cuyo tipo de elemento es `Named("Nodo")`. `Type::Named` guarda solo el nombre; no expande una definición dentro de sí misma. El valor sí guarda un árbol finito de estructuras y arrays. La copia clona ese árbol; la salida es `2` y `9`. Una cadena puede terminar con una unión, por ejemplo `struct Enlace { int valor; Enlace || bool siguiente = false; }`. Un campo `Nodo siguiente;` sin alternativa finita se rechaza antes de ejecutar.

La suma de llamadas y construcciones simultáneas tiene un límite de 100. Incluye tanto los inicializadores explícitos como los valores por defecto: un defecto que se construya a sí mismo sin terminar acaba en un error del lenguaje. Además, `Value::check_depth()` recorre el contenido mediante una pila de trabajo, sin recursión en Rust, y limita a 100 sus niveles combinados de estructuras, arrays y enums. Se aplica a cada construcción, literal array, escritura y elemento añadido con `push`; las dos últimas operaciones cuentan también los contenedores de la ruta destino antes de modificarla. Esto impide que un bucle produzca valores cada vez más profundos y desborde después la pila al copiarlos, imprimirlos o destruirlos. Son límites distintos: un valor puede crecer sin llamadas anidadas.

El ejemplo [estructuras_campos.oki](../examples/estructuras_campos.oki) reúne estas ampliaciones, un árbol y una cadena finita.

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

[stdlib.rs](../src/stdlib.rs) concentra la habilitación de las bibliotecas incluidas en Rust. `StandardLibrary` guarda un `LibraryAccess` para Array y otro para Casting. Cada uno conserva dos marcas independientes: biblioteca importada y nombre corto habilitado. El comprobador las reinicia al empezar el archivo y las actualiza al encontrar `import` y `use` en orden. Rechaza estas instrucciones dentro de bloques; no habilita nombres a partir de una rama que quizá no se ejecute. Repetir una directiva ya válida no tiene efecto adicional. Las reglas específicas de arrays están en [stdlib/array.rs](../src/stdlib/array.rs), igual que las conversiones están en [stdlib/casting.rs](../src/stdlib/casting.rs).

`stdlib.rs` expone `pub mod array;` y `pub mod casting;`. Ambos ofrecen funciones que se llaman mediante el nombre del módulo. Para cada `LibraryCall` de este ejemplo, `StandardLibrary::check_array()` consulta `array::resolve()` en `stdlib/array.rs` y comprueba la ruta y las marcas de importación. `array::check_arity()` verifica el número de argumentos: cero con receptor o uno sin él. El comprobador obtiene el tipo del array y `array::result_type()` exige `Type::Array` y devuelve `Some(Type::Int)`. Este `Option` distingue las llamadas con resultado de `push`, que no devuelve valor. No necesita el tamaño real ni modifica el AST. Los nombres de las rutas se resuelven aparte de las variables; `use` solo habilita `Array::`, sin crear una variable llamada `Array`.

El intérprete no realiza acciones al encontrar `Stmt::Import`: esas instrucciones ya se comprobaron. En `LibraryCall`, `array::resolve()` valida la operación, el intérprete evalúa una sola vez el receptor o el único argumento y entrega su valor a `array::evaluate()`. Las funciones del módulo reciben el `Name` del método y distinguen `len`, `push` y `pop` por su nombre. `len` obtiene el número de elementos del `Vec` y lo convierte a `i64` con comprobación de rango. En el ejemplo, las tres impresiones producen `3` con salto final.

La evaluación conserva la semántica actual de copia: consultar una variable array mediante `evaluate()` copia su contenido antes de medirlo. Todavía no se ha optimizado esa lectura. La llamada no escribe en el array y admite `const`. `len` solo cuenta el nivel exterior; el acceso previo de `tabla[0].len()` selecciona qué array se mide. Los errores al evaluar el receptor, como un índice fuera de rango, se propagan conservando su línea y la salida previa. El cortocircuito de `&&` y `||` puede omitir la evaluación de una llamada, pero nunca su comprobación de tipos.

Una ruta, método, importación, cantidad de argumentos o tipo incorrecto falla antes de ejecutar. Los errores de llamadas señalan la línea del nombre del método; los de directivas, la de `import` o `use`. La biblioteca no es un sistema general de funciones ni un cargador de archivos: las bibliotecas actuales son `std::Array`, con `len`, `push` y `pop`, y `std::Casting`, con conversiones entre tipos básicos. Las funciones propias siguen un camino distinto, descrito en la sección anterior.

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

`TypeChecker::check_call()` valida la operación y comprueba su cantidad de argumentos. `array::mutates()` identifica `push` y `pop`, que requieren un destino modificable; `array::takes_value()` identifica `push`, que recibe el elemento que se va a insertar. `Expr::into_target()` descompone una variable seguida de índices en el nombre raíz y su lista de accesos; rechaza literales y resultados temporales. La comprobación reutiliza `assignment_target()` para impedir cambios sobre constantes, también anidadas. Para `push`, el tipo del elemento del destino se entrega como contexto al argumento: así se acepta `tabla.push([])`. `array::result_type()` devuelve `None` para `push` y `Some(tipo_del_elemento)` para `pop`. `Stmt::Call` permite descartar el resultado; si una expresión necesita el valor de `push`, se rechaza antes de ejecutar. No se añade un tipo `void` al lenguaje.

`Interpreter::evaluate()` recibe ahora `&mut self`, porque evaluar `pop` cambia el entorno. `evaluate_call()` resuelve el ámbito y los índices con `resolve_target()`, evalúa el argumento de `push` y obtiene el almacenamiento mediante `target_mut()`. `array::evaluate()` recibe ese valor por referencia mutable y opera sobre su `Vec`: `push` añade y `pop` extrae el último elemento. En el ejemplo, `tabla` pasa de `[[]]` a `[[4]]` y vuelve a `[[]]`; `ultimo` guarda `Value::Int(4)`. Se imprime `4` y `[[]]`.

Los índices se evalúan una sola vez, antes del argumento que se va a insertar. Si una llamada interior elimina parte del destino, se comprueban de nuevo las posiciones guardadas; no se repiten las expresiones de los índices. Por ejemplo, `int[] a = [1]; a[0] = a.pop();` extrae `1` y después falla al escribir en la posición que ya no existe. Los efectos completados no se deshacen. `pop` de un vacío falla en la línea de su nombre. Los errores conservan la salida previa. Se mantienen la evaluación de izquierda a derecha, el cortocircuito y el recorrido de una copia en `foreach`.

### Recorrido de una conversión con std::Casting

```oki
import std::Casting;
int edad = 25;
float decimal = float(edad);
string texto = edad.cast(string);
println(decimal);
println(texto);
```

El scanner reconoce `Type(Float)` tanto en la declaración como en la llamada; el parser distingue ambas posiciones por la gramática. `conversion_call()` recoge el tipo de destino y exige exactamente una expresión entre paréntesis. Para el método, `finish_postfix()` reconoce `cast` y recoge un tipo mediante `array_type()`. Conserva el receptor como entrada del nuevo nodo. Las dos formas producen `Expr::Cast`, con el tipo de destino separado del valor: los tipos no son valores de ejecución ni variables.

```text
Import(path: [std, Casting], is_use: false)
Declare(edad, Int, Literal(Int(25)))
Declare(decimal, Float, Cast(path: [float], target: Float, value: Variable(edad)))
Declare(texto, String, Cast(path: [cast], target: String, value: Variable(edad)))
Println(Variable(decimal))
Println(Variable(texto))
```

Para `std::Casting::float(edad)` y `Casting::float(edad)`, el nodo es el mismo y la ruta guarda los tres o dos nombres. El comprobador calcula el tipo de la entrada y llama a `StandardLibrary::check_cast()`: valida la ruta, exige importación y el `use` cuando corresponde, y delega los pares permitidos a `casting::check_type()`. Devuelve el tipo de destino, que debe coincidir exactamente con el tipo de la declaración. En el ejemplo registra `decimal → Float` y `texto → String`. No evalúa el contenido de la entrada.

El intérprete evalúa `value` una sola vez y llama a `casting::evaluate()` en [stdlib/casting.rs](../src/stdlib/casting.rs). En este caso obtiene `Value::Float(25.0)` y `Value::String("25")`. Guarda esos resultados e imprime `25.0` y `25`, en líneas separadas. La variable `edad` sigue guardando `Value::Int(25)`. Tanto el comprobador como el intérprete admiten `Cast` en `Stmt::Call` para descartar un resultado.

La biblioteca separa compatibilidad de tipos y validación del valor. `bool(1)` falla antes de ejecutar; `int("hola")` tiene tipos convertibles y falla al interpretar. La conversión de texto usa análisis decimal y comprobaciones de contenido, sin evaluar el texto como código. `string(valor)` reutiliza `Display` para mantener el formato de impresión. `char` usa valores escalares Unicode; la conversión desde `int` comprueba tanto el rango de `u32` como `char::from_u32`.

Para `float` → `int`, se trunca hacia cero y se verifica el intervalo `[-2^63, 2^63)` antes de convertir con `as`. El límite superior es exclusivo porque `i64::MAX` redondeado a `f64` ya es `2^63`: compararlo como máximo inclusivo admitiría un valor inválido y Rust lo saturaría. `int` → `float` conserva el redondeo de `f64`; no garantiza recuperar todos los enteros grandes al convertir de vuelta.

Un error de conversión usa la línea del último `Name` de la ruta, incluido `cast` en los métodos. Los errores de la entrada se propagan sin sustituir su línea. El orden de evaluación y el cortocircuito siguen perteneciendo al intérprete: `false && bool("inválido")` no transforma el texto, pero `false && bool(1)` se rechaza al comprobar sus tipos. Los arrays completos no son convertibles; sí se pueden convertir sus elementos, como `datos[0].cast(float)`.

### Recorrido de una constante

Con `const int limite = 2 * 5; println(limite);`, el scanner añade `Const` antes de `Type(Int)`. El parser construye `Declare { declared_type: Int, is_constant: true, name: limite, initializer: Binary(Multiply, Literal(Int(2)), Literal(Int(5))) }`, seguido de `Println(Variable(limite))`. El comprobador registra `limite → VariableInfo { declared_type: Int, narrowed_type: None, narrowed_fields: {}, is_constant: true, is_inout: false }`.

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

El comprobador obtiene `Int` para la multiplicación y para la suma, y registra `total → VariableInfo { declared_type: Int, narrowed_type: None, narrowed_fields: {}, is_constant: false, is_inout: false }`. `evaluate()` evalúa los operandos de izquierda a derecha y llama a `binary()` para la operación: primero obtiene `3 * 4 = 12`, después `2 + 12 = 14`. Guarda `total → Int(14)` y la impresión produce `14` con salto final. Con `(2 + 3) * 4`, la suma queda como hijo izquierdo de la multiplicación y el resultado es `20`.

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


### Recorrido de un retorno de tipo unión

El ejemplo [retornos_union.oki](../examples/retornos_union.oki) declara `function test(int numero) -> int || string`, devuelve `numero` cuando es mayor que tres y devuelve `"test"` en la otra rama. Su representación principal es:

```text
Function(test, parameters: [(Int, numero)], return_type: Some(Union([Int, String])))
└── If(Binary(Variable(numero), Greater, Literal(Int(3))))
    ├── Return(Variable(numero))
    └── Return(Literal(String("test")))
Println(Call(test, [Literal(Int(5))]))
Println(Call(test, [Literal(Int(2))]))
```

El scanner ya producía `OrOr` para `||`; no necesita cambios. Tras `->`, el parser lo interpreta como separador de tipos, mientras que en una expresión sigue construyendo un operador lógico. La firma y la pila `return_types` conservan `Union([Int, String])`. `accepts()` acepta tanto `Int` como `String`; si la expresión también es una unión, comprueba todas sus alternativas. No depende del orden y no convierte valores. `always_returns()` sigue exigiendo que las dos ramas devuelvan.

`check_call()` devuelve la unión declarada sin intentar evaluar los argumentos ni adivinar qué rama se ejecutará. Las declaraciones admiten valores cuyos tipos posibles estén incluidos en la unión declarada; los parámetros conservan tipos concretos. Una variable refinada puede usarse por valor con ese tipo, mientras que `inout` exige el tipo declarado del almacenamiento. `binary_result()` rechaza operandos unión, también en igualdad; los unarios, índices y condiciones los rechazan mediante sus reglas de tipos concretos. Casting excluye explícitamente `Union`, para que sus reglas generales de conversión a texto no acepten por accidente un resultado con varios tipos posibles. Los literales de array tampoco admiten elementos de tipo unión: se mantiene su homogeneidad.

Para devolver un literal array a una unión, `expression_type_expected()` prueba el contexto de cada alternativa array sin ejecutar el código. Si hay una sola compatible, la utiliza; si hay varias, informa de ambigüedad y pide una variable de tipo concreto. Así `return [];` encaja en `int[] || string`, pero no decide entre `int[] || string[]`. Si no encaja en ninguna, el análisis ordinario permite informar del error del literal o de su tipo de retorno incompatible. Se conserva el contexto para vacíos anidados.

El intérprete sigue manejando valores concretos: la primera llamada produce `Control::Return(Some(Value::Int(5)))` y la segunda `Control::Return(Some(Value::String("test")))`. No se añade `Value::Union` ni se cambia la ejecución de funciones. La salida es `5` y `test`, cada uno en su línea. Un `return true;` se rechaza antes de ejecutar porque `Bool` no figura entre las alternativas.



### Recorrido de una variable unión y una prueba de tipo

Para esta entrada:

```oki
int || string resultado = 5;
if (type resultado == int) {
    println(resultado + 1);
} else {
    println(resultado + "!");
}
```

El scanner reconoce `type` como `TypeOf`. El parser construye `Declare(resultado, Union([Int, String]), Literal(Int(5)))` y un `If` cuya condición es `TypeCheck { name: resultado, fields: [], target: Int, negated: false }`. La prueba es una expresión de tipo `Bool`; no convierte el valor y los tipos no se vuelven valores del lenguaje.

El comprobador registra `declared_type: Union([Int, String])` y `narrowed_type: None`. El inicializador se acepta por inclusión, pero no produce un refinamiento. `with_condition()` copia el contexto; `assume_condition()` conserva las alternativas compatibles con el resultado de la prueba. En la rama verdadera queda `Int`, en la falsa `String`. `variable_type()` devuelve ese tipo refinado para comprobar cada suma. El intérprete guarda `Int(5)`, consulta `value_type()` al evaluar la condición y ejecuta la primera rama. Se imprime `6` seguido de un salto de línea.

Una prueba `!=` invierte la elección; `!` invierte el resultado esperado de su operando. Para `&&` verdadero y `||` falso se combinan las restricciones de ambos lados. Los demás resultados reúnen caminos alternativos mediante `merge_scopes()` y conservan todos los tipos posibles de cualquiera de ellos. Para comprobar el lado derecho de `&&` y `||`, `expression_type_expected()` prepara el contexto de la izquierda verdadera o falsa, respectivamente. Se verifica todo el código sin ejecutarlo, incluso ramas que resulten imposibles.

Una asignación completa usa `declared_type` para permitir cambiar a otra alternativa y luego borra `narrowed_type`. Las modificaciones que conservan el tipo, como `++` o `push`, mantienen el refinamiento. Los contextos tienen ámbitos separados: ocultar un nombre dentro de un bloque no altera la información de la variable exterior. Los argumentos por valor usan el tipo refinado, pero un argumento `inout` se compara con el tipo declarado; así una llamada no puede cambiar la alternativa de la variable raíz. Los campos unión sí pueden cambiar mediante el alias, por lo que sus refinamientos se invalidan como se explica en el recorrido de estructuras.

Antes de analizar un bucle, `forget_loop_writes()` recorre sus asignaciones completas y descarta las conclusiones sobre esos nombres, incluidas escrituras en ramas y bucles interiores. No resuelve todos los casos de ocultación: puede olvidar una conclusión exterior por una escritura a otro nombre local igual. Esta decisión conservadora evita utilizar en otra vuelta un tipo que ya pudo cambiar. `while` y `for` recuperan lo demostrado por su condición; en `for`, el cuerpo se comprueba antes de la actualización. Los saltos de control no se usan para deducir tipos nuevos. Tampoco se siguen pruebas guardadas en variables `bool`.

Para `int[] || string[] datos = obtener();`, el valor retornado conserva su `element_type`, aunque esté vacío. Por eso `TypeCheck(datos, Int[])` distingue un array entero vacío de uno de texto vacío. La impresión y la semántica de copia no cambian; el tipo de elemento acompaña a todos los arrays y a sus subarrays.


### Recorrido de una función propia con inout

Con la entrada:

```oki
function acumular(inout int destino, int valor) {
    destino += valor;
}
int total = 0;
acumular(inout total, 5);
println(total);
```

El scanner reconoce `inout` como `InOut`. El parser construye este esquema:

```text
Function(acumular, return_type: None, línea 1)
├── parameters: [Parameter(Int, destino, is_inout: true),
│                Parameter(Int, valor, is_inout: false)]
└── body: [CompoundAssign(destino, Add, Variable(valor))]
Declare(total, Int, Literal(Int(0)))
Call(acumular, [InOut(total), Value(Literal(Int(5)))], línea 5)
Println(Variable(total))
```

El comprobador registra la firma `[(Int, true), (Int, false)]`. En el cuerpo, `destino` y `valor` son parámetros modificables. Al comprobar la llamada, exige `InOut(total)` para el primer parámetro, verifica que `total` no sea constante y que su tipo sea `Int`. El segundo argumento es una expresión normal de tipo `Int`. Omitir la marca o escribir una global directamente dentro de la función produce un error antes de ejecutar.

El intérprete guarda `total → Owned(Int(0))` en el ámbito 0. Al llamar, procesa los argumentos de izquierda a derecha: el primero guarda la ubicación de `total`, el segundo copia `Int(5)`. Después añade la base de la llamada a `call_bases`, abre el ámbito de parámetros y registra `destino → Alias { scope: 0, name: "total" }` y `valor → Owned(Int(5))`. Al ejecutar `destino += valor`, `storage_location()` resuelve el alias; se lee `0`, se calcula `0 + 5` y se escribe `Int(5)` en el almacenamiento global de `total`. Al cerrar la llamada desaparece el alias, pero `total` conserva `5`. La impresión produce `5` seguido de salto de línea.

Si un parámetro `inout` se reenvía a otra llamada, se resuelve hasta su almacenamiento original. Se admiten varios alias de la misma variable: las lecturas ven inmediatamente las escrituras anteriores. Los argumentos normales conservan la copia tomada al evaluarse. Los alias solo señalan variables completas cuyos ámbitos siguen vivos; por eso esta versión no admite referencias a elementos que un `pop` podría eliminar. Sí se puede modificar cualquier elemento dentro del array recibido completo.

`return` propaga `Control::Return` hasta la llamada, que devuelve el valor o termina sin él. Tanto el retorno como los errores cierran el ámbito de parámetros y retiran su entrada de `call_bases`. Los cambios ya realizados no se deshacen, tampoco ante un error posterior. Las expresiones devuelven copias de los valores, nunca alias. Si se superan las 100 llamadas y construcciones simultáneas, se informa de un error en la línea de la llamada o construcción.

Cada llamada a `run()` crea sus dos entornos con un único ámbito global. Los bloques de un `if`, los ámbitos de un `for` o un `foreach` y el de cada llamada se añaden y retiran sobre esa pila; al terminar el programa la pila vuelve a tener solo el ámbito global.

## 7. Errores

| Etapa | Ejemplo | Resultado |
| --- | --- | --- |
| Lectura | Ruta ausente, archivo inexistente o contenido que no es UTF-8. | Error antes del análisis. |
| Scanner | Comillas sin cerrar, `char` vacío o múltiple, exponente incompleto. | Error con línea. |
| Parser | Falta un tipo válido, nombre, inicializador, paréntesis, corchete, llave, coma entre elementos o `;`; en los bucles, falta alguna de las tres partes del `for` o la palabra `in` del `foreach`; en una función, falta el `->` o el tipo tras él, un parámetro `inout` está mal formado o su argumento no es un nombre de variable completo; número fuera de rango. | Error con línea. |
| Comprobación de tipos | Variable desconocida, declaración duplicada, tipo incompatible, elementos de tipos distintos, índice que no es `int`, vacío sin contexto, condición de `if`/`while`/`for` que no es `bool`, `foreach` que no recorre un array o cuyo tipo de elemento no coincide, `break`/`continue` fuera de un bucle, reasignación de una constante, `+=`/`-=`/`++`/`--` sobre un destino que no admite la operación, función duplicada o fuera del ámbito global, colisión entre función y variable global, parámetro repetido, llamada a una función desconocida o declarada más adelante, argumentos con aridad o tipo incorrectos, marcas `inout` ausentes o sobrantes, constantes pasadas como `inout`, escritura directa de globales desde una función, uso de una llamada sin valor como expresión, `return` fuera de una función, retorno de tipo incorrecto o ausente, y función con valor que no devuelve en todos los caminos. | Error con línea, antes de ejecutar. |
| Enums y match: comprobación de tipos | Tipo o variante desconocidos, duplicados o fuera de ámbito global; argumentos/capturas incompatibles; sujeto que no es enum concreto, patrones de otro enum o match no exhaustivo. | Error con línea, antes de ejecutar. |
| Estructuras: comprobación de tipos | Tipo desconocido o duplicado, declaración fuera del ámbito global, campo desconocido, repetido, obligatorio ausente o con tipo incompatible, escritura en campo `const`, uso de un campo unión sin comprobación suficiente, valor por defecto con nombre no disponible o modificación prohibida, autorrecursión sin alternativa finita, colisión de nombres globales o acceso a campo de un tipo no estructurado. | Error con línea, antes de ejecutar. |
| Casting: comprobación de tipos | Biblioteca sin importar, ruta inválida o par no convertible, como `bool(1)`. | Error antes de ejecutar, con la línea de la función o de `cast`. |
| Casting: ejecución | Texto inválido o resultado fuera de rango, como `int("hola")`. | Error con la línea de la función o de `cast`, conservando la salida previa. |
| Intérprete | Índice de array fuera de rango, división/resto por cero, resultado numérico fuera de rango, profundidad máxima de llamadas/construcciones o de valores superada, destino que cambia de tipo durante la evaluación; fallo al escribir. | Error con línea del corchete para índices, del operador para errores numéricos de la llamada/construcción para su profundidad o de la operación que produce un valor demasiado profundo; se propaga el error de entrada/salida para escritura. |

Los errores propios del lenguaje usan `String`; la escritura y lectura pueden producir `io::Error`. `run()` los propaga mediante `Box<dyn Error>`, que admite distintos tipos de error. `main()` escribe el mensaje en `stderr` con el prefijo `Error:` y termina con código de fallo.

En declaraciones duplicadas, incompatibilidades y reasignaciones de constantes se señala la línea del nombre declarado o asignado. En referencias desconocidas se señala la línea del uso. Los errores de delimitadores señalan el token pendiente o el final del archivo. Los errores de operandos incompatibles y los errores numéricos en ejecución señalan la línea del operador. Los errores de indexación señalan la línea del `[` del acceso; los de elementos incompatibles o vacíos sin contexto, la del `[` del literal correspondiente. El parser presupone la marca `Eof` que añade el scanner.

Se devuelve el primer error detectado por las etapas, sin recuperación para buscar más. El análisis completo precede a la ejecución; un fallo de índices, numérico o de salida sí puede ocurrir después de haber escrito parte del texto, y no se deshace esa salida. Por ejemplo, `println("previo"); println(1 / 0);` imprime `previo` y después falla. No se evalúan operaciones durante la comprobación de tipos.

## Relación con Crafting Interpreters

Los [capítulos 4 a 10](https://craftinginterpreters.com/contents.html) aportan el recorrido scanner → AST → parser → intérprete. El [capítulo 7](https://craftinginterpreters.com/evaluating-expressions.html) explica la representación de valores; el [capítulo 8](https://craftinginterpreters.com/statements-and-state.html) introduce declaraciones, referencias, asignaciones, entornos y bloques; el [capítulo 9](https://craftinginterpreters.com/control-flow.html) añade `if`/`else`, `while`, `for` y los operadores lógicos; y el [capítulo 10](https://craftinginterpreters.com/functions.html) aporta las declaraciones de funciones, las llamadas y el `return`. OkitsuLang exige el tipo de retorno explícito tras `->` y comprueba que todas las rutas devuelvan un valor, en lugar de inferirlo como Lox.

OkitsuLang adapta esas ideas a `enum`, `match` y `HashMap` de Rust. Mantiene `print(expresión);` y `println(expresión);`, exige `tipo nombre = expresión;` con `const` opcional antes del tipo, distingue enteros de float y añade `char`. El `for` de OkitsuLang es de estilo C; el recorrido de arrays se separa en `foreach` con el tipo del elemento explícito. La comprobación estática, es decir, antes de ejecutar, implementa la decisión de tipado estricto del proyecto.

Las demás características de Lox y la máquina virtual de bytecode no forman parte de este avance.
