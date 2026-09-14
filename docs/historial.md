# Historial de desarrollo

Este archivo registra avances realizados. Las ideas futuras no implican que ya estén implementadas ni que haya que desarrollarlas sin una nueva petición.

## 2026-09-14 — Primera ejecución de un archivo

- **Qué se hizo:** lectura de una ruta recibida por argumento y ejecución de `print("texto")` mediante una comprobación directa del texto.
- **Por qué:** completar el primer paso mínimo: abrir `hello.oki` e imprimir `Hello world`.
- **Archivos:** `src/main.rs` y `hello.oki`.
- **Validación realizada:** compilación y ejecución del ejemplo con salida `Hello world`; comprobación de formato con Cargo.
- **Evolución:** esta implementación fue sustituida por las etapas descritas en la siguiente entrada.

## 2026-09-14 — Base de un intérprete de árbol

- **Qué se hizo:** scanner con tokens y líneas, parser descendente, nodos `Expr` y `Stmt`, e intérprete que ejecuta el AST. Se mantiene únicamente la instrucción `print` con cadenas y se permite una lista de instrucciones.
- **Por qué:** seguir el enfoque de *Crafting Interpreters* y disponer de etapas que se puedan ampliar progresivamente.
- **Archivos:** `src/main.rs` y `README.md`; se conserva `hello.oki` como ejemplo y entrada de una prueba.
- **Errores:** el análisis del archivo completo precede a la ejecución; los errores léxicos y sintácticos indican la línea. Los errores de escritura también se propagan.
- **Validación realizada:** seis pruebas superadas; ejecución de `hello.oki` con salida `Hello world`; comprobaciones de formato y Clippy con `-D warnings` superadas.

## 2026-09-14 — Documentación para aprender y continuar

- **Qué se hizo:** documentación del estado actual, explicación del recorrido de `print` con tokens y AST, y este historial de avances. Se añadió `AGENTS.md` con pautas para tareas posteriores y para actualizar las explicaciones junto con el código.
- **Por qué:** permitir comprender lo que se ha construido y mantener esa información al avanzar.
- **Archivos:** `AGENTS.md`, `README.md`, `docs/estado-actual.md`, `docs/funcionamiento-interno.md` y `docs/historial.md`.
- **Validación:** contraste de las explicaciones con `src/main.rs` y revisión de enlaces locales. Este cambio es exclusivamente documental; no requiere nuevas pruebas del intérprete.
