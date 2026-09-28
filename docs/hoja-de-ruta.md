# Hoja de ruta de OkitsuLang

Este documento convierte ideas en encargos pequeños y revisables. **H01, H02a, H02b y H04 están implementados; los demás hitos son propuestas y este documento no autoriza por sí mismo a desarrollarlos**. Las instrucciones del repositorio exigen un encargo del usuario para cambiar el código.

## Estado de partida

OkitsuLang es un intérprete de árbol en Rust. El programa se lee desde un archivo UTF-8; las etapas actuales separan scanner, parser y AST, comprobador de tipos e intérprete. La biblioteca estándar ofrece `Array` y `Casting`. El lenguaje tiene los tipos `int`, `float`, `bool`, `char`, `string` y arrays, además de declaraciones, asignaciones, `if`/`else`, `while`, `for`, `foreach`, `break`, `continue`, impresión y conversiones explícitas. Las variables requieren tipo y valor inicial. La comprobación de tipos ocurre antes de ejecutar. Hay funciones propias con parámetros tipados y valor de retorno opcional: se declaran con `function` solo en el ámbito global, usan `return`, admiten recursión directa con un límite de profundidad y exigen el tipo de retorno tras `->` cuando devuelven un valor. Los parámetros `inout` exigen la marca también en la llamada y permiten modificar variables completas del llamador. Hay estructuras de valor con campos públicos, constantes por campo, valores por defecto, campos unión y autorrecursión finita; hay enums con y sin datos y match exhaustivo; no hay entrada estándar, lectura de archivos, módulos del programa ni vistas de arrays. El `foreach` actual recorre una copia.

La sintaxis de los hitos pendientes es tentativa y no está implementada; H02, H04 y H05 describen ya la sintaxis real. Las etapas mencionadas son las responsabilidades actuales: `scanner`, `parser` (donde se define también el AST), `type_checker`, `interpreter` y `stdlib`. Cuando una característica afecte al recorrido o las responsabilidades internas, se actualizará además `docs/funcionamiento-interno.md`.

## Cómo ejecutar esta hoja de ruta con agentes

Cuando el usuario encargue un hito, asignarlo por defecto a un agente de menor coste disponible (por ejemplo, `gpt-6-luna`) si el alcance está acotado y las decisiones semánticas ya están cerradas. Cada encargo debe citar el identificador del hito, pedir que lea `README.md`, `docs/estado-actual.md`, esta hoja y las secciones pertinentes de `docs/funcionamiento-interno.md`, y limitar los cambios a ese hito. No se deben iniciar hitos dependientes en paralelo. El agente debe entregar cambios, pruebas y documentación; el agente coordinador revisa las decisiones, conflictos e integración.

Plantilla de encargo para un agente:

> Implementa únicamente el hito `[ID]` de `docs/hoja-de-ruta.md`, que el usuario ha solicitado explícitamente. Antes de editar, lee la documentación indicada por `AGENTS.md` y revisa el código relacionado. Respeta las decisiones de sintaxis y semántica del hito; no adelantes hitos posteriores. Añade pruebas de comportamiento, conserva `hello.oki`, actualiza estado, funcionamiento interno e historial según `AGENTS.md`, ejecuta las comprobaciones requeridas y comunica exactamente sus resultados. No marques otros hitos como completados.

Los hitos pequeños y no solapados (por ejemplo, `break` y `continue` después de acordar su semántica) pueden encargarse a agentes económicos. Para cambios que comparten AST, tabla de símbolos, representación de valores o estrategia de memoria, asignar primero un diseño y después una implementación coordinada evita conflictos. La delegación no amplía la autorización: cada hito sigue necesitando un encargo explícito.

## Orden propuesto

Las dependencias indican el orden aconsejado. H01, H02a, H02b, H04 y H05 están completados; H03 se ha aplazado por indicación del usuario y los demás hitos siguen pendientes.

### H01 — `break` y `continue`

**Estado: implementado el 2026-09-26.** Estas instrucciones funcionan en los bucles existentes, con cuerpos entre llaves y punto y coma obligatorio.

- **Sintaxis implementada:** `break;` y `continue;` dentro de `while`, `for` y `foreach`.
- **Semántica implementada:** ambas instrucciones solo son válidas dentro de un bucle; los ámbitos se cierran al salir o saltar a la siguiente iteración. En `for`, `continue` ejecuta la actualización y después comprueba la condición; en `foreach`, avanza al siguiente elemento. `break` sale únicamente del bucle más cercano. No hay etiquetas ni salida de varios bucles.
- **Etapas modificadas:** `scanner` (tokens), `parser`/AST (nuevas instrucciones), `type_checker` (validación dentro de un bucle) e `interpreter` (señal de control que atraviesa bloques). `stdlib` no cambió.
- **Dependencias:** ninguna.
- **Pruebas realizadas:** salida de `while`, `for` y `foreach` con salida anticipada; `continue` que omite el resto del cuerpo y ejecuta la actualización del `for`; rechazo antes de ejecutar fuera de un bucle; bucles anidados y cierre de ámbitos.
- **Documentación actualizada:** `docs/estado-actual.md`, `docs/funcionamiento-interno.md` con un ejemplo concreto, `docs/historial.md` y enlace desde `README.md`.

### H02 — Funciones propias, declaración y llamada

**Estado: implementado el 2026-09-26 en H02a y H02b.** Nota de sintaxis: la palabra reservada acordada inicialmente como `func` se cambió a `function` a petición del usuario antes de implementar H02b; `func` queda como un identificador normal.

#### H02a — Declaraciones, parámetros y funciones que no devuelven valor

**Estado: implementado el 2026-09-26.** Sintaxis implementada: `function nombre(tipo param, ...) { ... }` y llamada `nombre(argumentos);`.

- **Decisiones fijadas:** `function` es palabra reservada; la declaración solo se admite en el ámbito global del archivo y antes de sus llamadas; sin `->` la función no devuelve valor ni existe `void`; cada parámetro lleva tipo obligatorio (básico o array) y no se repite nombre; la llamada exige aridad y tipos exactos, sin conversiones implícitas, y evalúa los argumentos de izquierda a derecha; una llamada sin valor solo se admite como instrucción; cada llamada abre un ámbito propio donde los parámetros y locales no escapan, y desde ella se pueden leer globales; desde la incorporación de `inout`, escribir en ellas exige recibirlas como parámetros `inout`; los nombres de función son únicos y no colisionan con variables globales; se admite la recursión directa y cada ejecución limita las llamadas anidadas a 100 para no abortar el proceso.
- **Etapas modificadas:** `scanner` (token `Function`), `parser`/AST (`Stmt::Function` y `Expr::Call`), `type_checker` (firmas, ámbitos, colisiones y validación de llamadas) e `interpreter` (definiciones, llamada, ámbito local y límite de profundidad). `stdlib` no cambió.
- **Dependencias:** ninguna; H02a precede a H02b.
- **Pruebas realizadas:** nueve pruebas nuevas en `src/main.rs` y el ejemplo `funciones.oki`. Cubren parámetros básicos y de array, ámbito local, ocultación de nombres, lectura y reasignación de globales y rechazo de constantes, encadenamiento y declaración previa obligatoria, aridad y tipos exactos, rechazo de la llamada como valor, declaraciones inválidas, recursión directa, límite de profundidad, orden de evaluación de argumentos y conservación de la salida ante errores.
- **Documentación actualizada:** `docs/estado-actual.md` con gramática y límites, `docs/funcionamiento-interno.md` con el recorrido y un ejemplo concreto, `docs/historial.md`, `README.md` y este archivo.

#### H02b — Valores de retorno y funciones tipadas

**Estado: implementado el 2026-09-26.** Sintaxis implementada: `function sumar(int a, int b) -> int { return a + b; }` y uso en expresiones como `int total = sumar(2, 3);`.

- **Decisiones fijadas:** el tipo de retorno se escribe tras `->` y es opcional; sin `->` la función no devuelve valor y no existe la palabra `void`. `return expresión;` devuelve un valor que debe coincidir exactamente con el tipo indicado; `return;` sin expresión solo vale en funciones sin retorno; `return` fuera de una función es un error. Una función con `-> tipo` debe devolver un valor en todos los caminos, con un análisis conservador (un `return` directo o un `if`/`else` con ambas ramas devolviendo; un bucle no garantiza el retorno). Una función con valor se usa como cualquier expresión; `return` interrumpe bucles y bloques hasta la llamada. Las funciones leen globales; desde la incorporación de `inout`, escribir en ellas exige recibirlas como parámetros `inout`.
- **Etapas modificadas:** `scanner` (tokens `Return` y `Arrow`), `parser`/AST (`return_type`, `Stmt::Return`), `type_checker` (tipo en la firma, pila `return_types` y análisis de rutas) e `interpreter` (señal `Control::Return` propagada por bloques y bucles, valor devuelto por la llamada). `stdlib` no cambió.
- **Dependencias:** H02a y las decisiones de firma, ámbito y llamada.
- **Pruebas realizadas:** siete pruebas nuevas más en `src/main.rs` (dieciséis de funciones en total) y el ejemplo `funciones.oki` ampliado. Cubren el retorno de cada tipo básico, la composición de llamadas con valor, el retorno en ramas y bucles, la recursión tipada, la coincidencia exacta del tipo, la cobertura de todos los caminos, `return` fuera de función, el `return;` sin valor y la convivencia de funciones con y sin retorno.
- **Documentación actualizada:** `docs/estado-actual.md` con la nueva sintaxis y los límites, `docs/funcionamiento-interno.md` con la comprobación y la propagación del retorno, `docs/historial.md`, `README.md` y este archivo.

### H03 — Operaciones de texto

**Estado: pendiente; no implementado.** Ampliar las operaciones para `string` de manera coherente con la biblioteca estándar existente.

- **Sintaxis tentativa:** métodos como `texto.len()`, `texto.contains("x")`, `texto.starts_with("x")` y `texto.substring(inicio, fin)`, habilitados con una biblioteca `std::String`; los nombres y si se usa biblioteca quedan por decidir. `len` devolvería una cantidad de valores escalares Unicode o bytes, decisión que debe quedar explícita.
- **Semántica por decidir:** unidad de longitud e índices (bytes, escalares Unicode o grafemas), límites y fragmentos vacíos, qué ocurre ante índices fuera de rango, si las operaciones son puras y si `substring` devuelve una copia. No prometer manipulación de grafemas sin una estrategia Unicode definida.
- **Etapas:** principalmente `stdlib` y `type_checker` para disponibilidad, firmas y tipos de argumentos; `interpreter` para evaluar métodos. `scanner`/`parser`/AST solo si se adopta una sintaxis nueva de método; reutilizar la llamada actual si es suficiente.
- **Dependencias:** ninguna obligatoria; conviene decidir convenciones junto con H07, que introduce errores recuperables.
- **Pruebas observables:** cadena vacía, ASCII y Unicode, resultado y tipo correcto, biblioteca sin habilitar, argumentos incompatibles y límites acordados.
- **Documentación al implementarlo:** estado con unidad Unicode y errores; funcionamiento interno si cambian llamadas; ejemplo, historial y enlace desde README.

### H04 — Estructuras con campos

**Estado: implementado y ampliado el 2026-09-28.** H03 queda para más adelante por indicación del usuario; H04 no depende de él.

- **Sintaxis implementada:** `struct Persona { string nombre; int edad; }`, construcción `Persona { nombre: "Ana", edad: 30 }`, acceso y escritura de campos. Se admiten `const int id;`, valores por defecto `int edad = 0;`, campos unión `int || string dato;` y pruebas `type persona.dato == int`. La declaración del tipo no lleva `;` final.
- **Decisiones fijadas:** tipos globales declarados antes de usar, nombres únicos sin colisión con variables o funciones globales; campos públicos sin duplicados, obligatorios cuando no tengan valor por defecto. Los campos explícitos se evalúan en el orden escrito y los omitidos, después y en el orden de declaración, con acceso de solo lectura a campos anteriores y globales. Las estructuras son valores con copias profundas e igualdad nominal por contenido. `const` protege variables completas o campos, incluidos sus contenidos; reemplazar un contenedor modificable crea un valor nuevo. Se admiten estructuras vacías, tipos anteriores y autorrecursión finita mediante arrays o uniones con una alternativa que termine. Se conservan funciones, `inout` y protección de globales. Los refinamientos de campos se invalidan por reemplazos y efectos de llamadas, considerando alias. Las rutas comprueban índices y tipos antes de escribir. Las llamadas/construcciones simultáneas y el contenido anidado de valores tienen límites de 100 para detener la recursión excesiva. Por decisión del usuario, no se añaden campos privados ni métodos propios; estos últimos quedan reservados para futuras clases. No hay referencias compartidas, tipos adelantados, recursión mutua ni conversiones de Casting de estructuras.
- **Etapas modificadas:** `scanner` (`Struct` y `Colon`, manteniendo `Dot` y `ColonColon`), `parser`/AST (declaraciones, campos con atributos e inicializadores, construcción, rutas y pruebas de tipo), `type_checker` (registro nominal, campos, constantes, contexto de inicializadores y refinamientos), `interpreter`/`value` (valores, construcción y lectura/escritura). Se excluyen estructuras en Casting; no se amplía la API de biblioteca.
- **Dependencias:** H02 permite usarlas en funciones y las uniones existentes permiten terminar cadenas. No se implementan H03, enums de H05 ni los tipos opcionales de H06.
- **Pruebas realizadas:** trece pruebas iniciales en `src/main.rs`, adaptadas a la ampliación, y catorce nuevas en `src/structure_tests.rs`. Cubren comportamiento, errores antes y durante la ejecución y efectos que invalidan refinamientos o destinos. Se conserva `hello.oki`.
- **Documentación actualizada:** estado, funcionamiento interno con recorrido de AST, tipo y valor, ejemplos `estructuras.oki` y `estructuras_campos.oki`, historial, README y este archivo.

### H05 — Enums y `match`

**Estado: implementado completo el 2026-09-28**, incluyendo la parte opcional H05c por el encargo «H05 completo».

#### H05a — Enums sin datos

- **Sintaxis implementada:** `enum Estado { Pendiente, Hecho }`, `Estado::Hecho`. Declaración global antes de usar y sin punto y coma final; al menos una variante, nombres únicos y sin colisiones con tipos, variables o funciones globales.
- **Semántica:** variantes siempre calificadas, sin paréntesis cuando no tienen datos. Copias independientes, igualdad nominal y por contenido; impresión `Estado::Hecho`. Se integran en arrays, estructuras, funciones, `inout`, constantes y uniones.
- **Etapas:** scanner, parser/AST, comprobador e intérprete/valores. `Type::Named` unifica las anotaciones de tipos propios; los registros distinguen enum de estructura. Casting sigue limitado a tipos básicos.

#### H05b — `match` exhaustivo

- **Sintaxis implementada:** `match estado { Estado::Pendiente => { ... }, Estado::Hecho => { ... } }`. Ramas con bloques de instrucciones, separadas por comas y con coma final opcional; no es una expresión ni lleva `;` final.
- **Semántica:** sujeto enum concreto evaluado una vez; todas las variantes exactamente una vez, sin comodín ni guardas. Cada rama tiene su ámbito propio. Se comprueban todas antes de ejecutar; se ejecuta solo la seleccionada. Retornos y saltos atraviesan las ramas; todas las ramas con retorno permiten garantizar el retorno de una función.
- **Etapas:** tokens `Match` y `FatArrow`, `Stmt::Match` / `MatchArm`, exhaustividad y combinación de refinamientos en el comprobador, selección y cierre de ámbitos en el intérprete.

#### H05c — Variantes con datos

- **Sintaxis implementada:** `enum Resultado { Ok(int valor), Error(string mensaje) }`, `Resultado::Ok(5)` y `Resultado::Ok(numero) => { println(numero); }`.
- **Semántica:** uno o varios datos con tipos concretos ya disponibles, construcción y captura por posición, aridad y tipos exactos. Datos evaluados una vez y en orden; capturas locales modificables por copia profunda, con tipo obtenido de la definición. Sin acceso directo por campo, patrones anidados, tipos adelantados o enums autorrecursivos; sin uniones, constantes ni valores por defecto en sus datos. Los datos pueden contener estructuras y enums anteriores y arrays.
- **Etapas:** `VariantDef`, rutas `Expr::Qualified` resueltas como variante o biblioteca, `Value::Enum`, impresión e igualdad por variante y datos. Se conserva la API de Array y el contexto de arrays vacíos.

- **Pruebas realizadas para H05:** dieciocho nuevas en `src/enum_tests.rs`; 174 en total. Cubren variantes, tipos, exhaustividad, ámbito y capturas, copias, integración, efectos y refinamientos, errores estáticos y de ejecución, sintaxis, aislamiento y límites de profundidad. Se conserva `hello.oki`.
- **Documentación y ejemplo:** README, estado actual, recorrido interno, historial y `examples/enums.oki`.
- **Alcance:** H03 sigue aplazado y H06/H07 pendientes. `Resultado` es un enum propio del ejemplo; no se añaden opcionales ni resultados genéricos predefinidos.

### H06 — Tipos opcionales

**Estado: pendiente; no implementado.** Expresar explícitamente la presencia o ausencia de un valor.

- **Sintaxis tentativa:** `int? encontrado = ...;`, `some(3)`, `none` y `match encontrado { some(valor) => ..., none => ... }`. La forma debe coordinarse con H05; puede resolverse internamente como un enum predefinido.
- **Semántica por decidir:** si existe conversión implícita del valor a `some`, igualdad, anidamiento (`int??`), inicialización por defecto (recomendación: ninguna) y obligación de cubrir `none`.
- **Etapas:** `scanner`/`parser` para el tipo opcional y constructores si se eligen; AST; `type_checker`; `value`/`interpreter`. `stdlib` solo si se ofrecen operaciones auxiliares.
- **Dependencias:** H05a y preferiblemente H05b para el análisis exhaustivo; no confundir sintaxis de `?` con operadores de propagación de errores.
- **Pruebas observables:** `some` y `none`, extracción segura, coincidencia exhaustiva y errores por tipo o rama omitida; verificar que no se evalúa una rama no seleccionada.
- **Documentación al implementarlo:** estado con inicialización y uso; funcionamiento interno con tipo/valor; ejemplo, historial y enlaces.

### H07 — Resultados y errores recuperables

**Estado: pendiente; no implementado.** Permitir que una operación devuelva éxito o error como valor comprobable.

- **Sintaxis tentativa:** `Resultado<int, string>`, `Ok(valor)` y `Err(mensaje)`, tratados inicialmente mediante enums genéricos si el sistema de tipos lo permite. No introducir `?` de propagación en la primera entrega.
- **Semántica por decidir:** tipos genéricos, igualdad e impresión, convención del tipo de error, si `return` puede devolver `Err`, y cómo una función comunica fallos. Separar estos valores de los errores actuales del intérprete (errores de ejecución que detienen el programa).
- **Etapas:** `parser`/AST para tipos genéricos y constructores, `type_checker`, `value`/`interpreter`; `stdlib` al migrar operaciones que fallen de forma recuperable.
- **Dependencias:** H02 para funciones tipadas; H05 para distinguir variantes y leer el resultado. H06 comparte conceptos pero no es requisito.
- **Pruebas observables:** función que devuelve `Ok` y `Err`, coincidencia en ambos casos, incompatibilidad entre parámetros genéricos, y demostrar que `Err` no detiene por sí mismo el intérprete.
- **Documentación al implementarlo:** distinguir error como valor de fallo del lenguaje en estado y funcionamiento interno; ejemplos, historial y enlaces.

### H08 — Entrada estándar y archivos

**Estado: pendiente; no implementado.** Incorporar operaciones de entrada/salida a la biblioteca estándar, en lugar de añadirlas directamente al núcleo del lenguaje.

#### H08a — Entrada estándar

- **Sintaxis tentativa:** `import std::IO; string linea = IO::read_line();`.
- **Semántica por decidir:** quitar o conservar el salto final, fin de entrada, bloqueo, codificación y representación del fallo. La recomendación es devolver un resultado explícito, no convertir errores de entrada en valores vacíos.
- **Etapas:** `stdlib` para API e I/O; `type_checker` para disponibilidad y retorno; `interpreter` para invocación y propagación. Scanner/parser/AST solo si no se puede expresar como llamada de biblioteca actual.
- **Dependencias:** H07 si el tipo de retorno es `Result`; puede aplazarse con un error de ejecución explícito, pero documentado.
- **Pruebas observables:** lectura de una línea conocida, fin de entrada y error controlado; usar una fuente de entrada sustituible en pruebas para evitar pruebas interactivas.
- **Documentación al implementarlo:** actualizar estado con comportamiento de EOF y errores; funcionamiento interno de I/O; ejemplo no interactivo reproducible, historial y enlaces.

#### H08b — Archivos

- **Sintaxis tentativa:** `IO::read_file(ruta)` y `IO::write_file(ruta, contenido)`.
- **Semántica por decidir:** rutas relativas al directorio actual o al archivo fuente, sobrescritura frente a creación, permisos y errores, lectura UTF-8 y si se ofrecen operaciones por líneas. Definir límites de tamaño solo si se implementan.
- **Etapas:** `stdlib`, `type_checker`, `interpreter`; AST/parser/scanner si hace falta una nueva sintaxis. H07 para comunicar fallos recuperables.
- **Dependencias:** H08a para el módulo y H07 recomendado. Lectura binaria queda fuera del alcance inicial.
- **Pruebas observables:** leer archivo temporal UTF-8, escribir y volver a leer, archivo inexistente, contenido no UTF-8 y permisos/error simulado cuando sea portable.
- **Documentación al implementarlo:** estado y funcionamiento interno con límites/rutas; ejemplo reproducible, historial y enlaces.

### H09 — Módulos propios

**Estado: pendiente; no implementado.** Organizar programas en varios archivos. Ya existe `import`/`use` para bibliotecas estándar; no asumir que esos mecanismos soportan archivos de usuario.

- **Sintaxis tentativa:** `import "utilidades.oki";` con llamadas calificadas `Utilidades::funcion(...)`, o una forma equivalente. Elegir una sola forma tras estudiar rutas y resolución de nombres.
- **Semántica por decidir:** rutas relativas al módulo importador o a la raíz del proyecto, extensiones, ciclos, orden de evaluación, visibilidad pública/privada, nombres repetidos, importaciones transitivas y diagnóstico con ruta y línea de origen.
- **Etapas:** lectura/coordinación en `main.rs`, `scanner`/`parser` si cambia la gramática, `type_checker` con entorno entre archivos e `interpreter` con orden de ejecución. `stdlib` debe conservar su resolución y distinguir bibliotecas estándar de módulos propios.
- **Dependencias:** H02 para exportar funciones es muy recomendable; H04/H05 se incorporan solo al permitir exportarlas.
- **Pruebas observables:** importar módulo simple, dos módulos, resolución desde subdirectorio, símbolo privado, ruta inexistente y ciclo según regla adoptada. Comprobar errores con archivo y línea correctos.
- **Documentación al implementarlo:** actualizar instrucciones de ejecución/organización, funcionamiento interno del cargador y resolución, ejemplos de varios archivos, historial y enlaces.

### H10 — Vistas de arrays sin copia

**Estado: pendiente; no implementado.** Evitar copias al recorrer arrays, empezando por el caso observable de `foreach`. El comportamiento actual recorre una copia del array.

- **Sintaxis tentativa:** mantener `foreach (int n in numeros)` sin introducir anotaciones de préstamo para el usuario. Una vista explícita, si se necesita más tarde, podría tener una forma por decidir como `int[] view = numeros.view(inicio, fin);`.
- **Semántica por decidir:** si `foreach` observa cambios del array durante el bucle (recomendación inicial: impedir mutarlo mientras la vista vive), reglas de alias, duración, anidamiento, recolección/propiedad y si asignar una vista a variable es necesario. Debe definirse antes de cambiar la copia actual.
- **Etapas:** `value`/representación interna, `interpreter` (iteración sin clonar), `type_checker` para impedir mutaciones incompatibles; `parser`/AST solo si se añade sintaxis explícita. `stdlib` si la API de arrays ofrece `view`.
- **Dependencias:** decidir el modelo de mutabilidad y alias antes de H10; estructuras o funciones no son requisito.
- **Pruebas observables:** iterar arrays vacíos y no vacíos, conservar el orden y valores, comprobar la política de mutación y demostrar mediante una comprobación interna específica que la iteración no clona el almacenamiento. La prueba de no copia no debe depender de tiempos.
- **Documentación al implementarlo:** actualizar estado con reglas de alias/mutación; funcionamiento interno con representación y recorrido; ejemplo, historial y enlaces.

### H11 — Presupuestos de coste

**Estado: pendiente; no implementado.** Explorar la característica distintiva de declarar límites de copias o asignaciones y explicar su incumplimiento. Es una línea de diseño en varias etapas, no una sola instrucción.

#### H11a — Definir el modelo de coste

- **Sintaxis tentativa:** ningún código de OkitsuLang todavía. Documento de diseño interno con unidades medibles, alcance de un presupuesto, tratamiento de llamadas y bibliotecas, y ejemplos.
- **Semántica por decidir:** distinguir copias lógicas, clones del almacenamiento, asignaciones del intérprete y asignaciones de Rust; decidir si los límites son estáticos, dinámicos o ambos. No prometer recuentos del código máquina con el intérprete actual.
- **Etapas:** revisión de `value`, `interpreter`, `stdlib` y puntos de asignación; no cambios de scanner/parser/AST salvo que el diseño lo justifique.
- **Dependencias:** H10 y una representación de arrays estable simplifican medir copias. No es requisito adoptar vistas primero, pero se debe definir cómo se cuentan.
- **Pruebas observables:** ejemplos de contabilidad del modelo y casos límite; todavía no pruebas de sintaxis ni implementación.
- **Documentación al completarlo:** crear un diseño en `docs/`, enlazarlo desde README y registrar la decisión en el historial si constituye un avance solicitado. Mantenerlo marcado como propuesta hasta su autorización e implementación.

#### H11b — Contadores dinámicos de coste

- **Sintaxis tentativa:** tras H11a, quizá `budget { copies <= 2; allocations <= 1; ... }`.
- **Semántica por decidir:** alcance del bloque, efectos de llamadas, fallo al superar el límite y presentación del contador. Los diagnósticos deben explicar qué operación consumió el presupuesto.
- **Etapas:** `scanner`, `parser`/AST, `type_checker` para validar unidades/límites e `interpreter`/`value`/`stdlib` para contabilidad. No medir aún código máquina.
- **Dependencias:** H11a y definiciones estables de copias/asignaciones.
- **Pruebas observables:** presupuesto cumplido, excedido, anidado según reglas, y costes de operaciones de arrays y biblioteca; diagnósticos reproducibles sin depender de tiempo.
- **Documentación al implementarlo:** estado y funcionamiento interno del contador; ejemplos, historial y enlaces.

#### H11c — Análisis estático o costes compilados (futuro lejano)

- **Sintaxis tentativa:** solo después de H11a/H11b y de definir si habrá compilador. No fijar sintaxis todavía.
- **Semántica por decidir:** qué garantías pueden probarse estáticamente, tratamiento de bucles y recursión, y diferencia entre el coste abstracto del lenguaje y el coste real de la máquina.
- **Etapas:** depende de la arquitectura futura; el intérprete de árbol actual no permite afirmar el coste del código máquina.
- **Dependencias:** H11a, evidencia de uso de H11b y una decisión independiente sobre compilación. No adelantar una máquina virtual por esta propuesta.
- **Pruebas observables:** casos cuya cota sea demostrable y casos que deben producir “no se puede demostrar”; contrastar únicamente métricas definidas por el modelo.
- **Documentación al implementarlo:** documentar con precisión las garantías y límites en estado y funcionamiento interno, mantener ejemplos fieles y añadir historial.

## Requisitos para cerrar cualquier hito

Un hito solo se considera completado después de que un encargo explícito autorice su implementación y se cumplan los requisitos de `AGENTS.md`: revisar y conservar cambios del usuario, añadir pruebas observables para cambios de comportamiento, mantener `hello.oki`, actualizar `docs/estado-actual.md`, `docs/funcionamiento-interno.md` cuando cambie el recorrido interno, `docs/historial.md` y los enlaces/ejemplos necesarios. Para cambios Rust, ejecutar `cargo fmt -- --check`, `cargo test` y `cargo clippy --all-targets -- -D warnings`; si cambia la ejecución desde archivo, ejecutar también `cargo run -- examples/hello.oki`. Registrar sin inventar cualquier comprobación que no se haya podido ejecutar.

La lista es orientativa y se puede reordenar mediante un encargo del usuario. El orden no autoriza cambios; H01, H02a, H02b, H04 y H05 figuran como completados; H03 queda aplazado y los demás hitos siguen pendientes.
