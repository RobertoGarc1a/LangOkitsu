# Instrucciones para trabajar en OkitsuLang

Estas pautas se aplican a todo el repositorio. El proyecto sirve para aprender a construir un lenguaje de programación en Rust, avanzando en pasos pequeños.

## Alcance y enfoque

- Implementar únicamente la funcionalidad solicitada por el usuario. Las posibles ampliaciones descritas en la documentación no son tareas autorizadas.
- Seguir el enfoque de *Crafting Interpreters*, adaptándolo a Rust y a las decisiones de sintaxis de OkitsuLang. No copiar automáticamente todas las características de Lox.
- Las instrucciones disponibles son `print("texto");` (sin salto final) y `println("texto");` (con salto final). El punto y coma es obligatorio. Conservar esta sintaxis salvo que el usuario pida cambiarla.
- Mantener separadas las responsabilidades de scanner, parser, AST e intérprete. No sustituir estas etapas por comprobaciones del texto completo.
- Priorizar código sencillo y comprensible. Se puede mantener todo en `src/main.rs` mientras resulte manejable; separar módulos cuando el crecimiento lo justifique.
- Evitar dependencias y abstracciones sin una necesidad concreta. El rendimiento es un objetivo futuro: no asumir que usar Rust basta para que el lenguaje sea rápido ni adelantar una máquina virtual sin que forme parte de la tarea.

## Antes de cambiar código

1. Leer `README.md`, `docs/estado-actual.md` y las secciones pertinentes de `docs/funcionamiento-interno.md`.
2. Revisar el código y las pruebas que afectan al cambio. La documentación debe describir el comportamiento real; corregir las discrepancias que se encuentren dentro del alcance.
3. Identificar qué etapas hay que modificar y conservar los cambios del usuario que no pertenezcan a la tarea.

## Documentación que debe acompañar a los cambios

- Escribir las explicaciones y los mensajes dirigidos al usuario en español, con lenguaje claro. Explicar los términos técnicos cuando se introduzcan.
- Actualizar `docs/estado-actual.md` si cambian la sintaxis, capacidades, limitaciones o forma de ejecutar el programa.
- Actualizar `docs/funcionamiento-interno.md` si cambia el recorrido del código, las estructuras, las responsabilidades o el tratamiento de errores. Incluir un ejemplo concreto que conecte entrada, representación interna y resultado.
- Añadir una entrada a `docs/historial.md` por cada avance significativo: fecha, qué se hizo, por qué, archivos implicados y comprobaciones realizadas. Conservar las entradas anteriores y no inventar resultados de pruebas.
- Si una explicación nueva necesita su propio archivo, crearlo dentro de `docs/` y enlazarlo desde `README.md`. Evitar duplicar explicaciones extensas.
- Distinguir siempre lo implementado de las ideas futuras. Los ejemplos presentados como funcionales deben corresponder al intérprete actual.
- Mantener los comentarios del código centrados en decisiones o conceptos que ayuden a aprender, sin narrar cada línea obvia.

## Validación y entrega

- Para cambios de comportamiento, añadir o ajustar pruebas que comprueben resultados observables y errores relevantes. Conservar la prueba de `hello.oki`.
- Tras modificar Rust, ejecutar `cargo fmt -- --check`, `cargo test` y `cargo clippy --all-targets -- -D warnings`. Si el formato falla, aplicar `cargo fmt` y volver a comprobarlo.
- Si cambia la lectura del archivo o la ejecución, comprobar también `cargo run -- examples/hello.oki`.
- Para cambios exclusivamente documentales, revisar los enlaces locales, ejemplos y concordancia con el código; no hace falta añadir pruebas ni recompilar.
- Si una comprobación no se puede ejecutar, indicar cuál y por qué. No presentarla como superada.
- En la entrega, resumir qué se hizo, dónde se explica y cómo se verificó. El usuario debe poder entender cada avance a partir de los archivos del proyecto.

## Referencia

[Crafting Interpreters — índice](https://www.craftinginterpreters.com/contents.html). La base actual corresponde al intérprete de árbol de los capítulos 4 a 8, limitada al fragmento necesario para `print`.
