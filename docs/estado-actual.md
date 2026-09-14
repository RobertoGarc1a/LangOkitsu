# Qué está implementado

OkitsuLang lee un archivo de texto y ejecuta instrucciones que imprimen cadenas. Está escrito en Rust, usa la edición 2024 y no tiene dependencias externas. Toda la implementación y sus pruebas están en [src/main.rs](../src/main.rs).

## Cómo probarlo

Con Rust y Cargo disponibles, ejecutar desde la raíz del proyecto:

```sh
cargo run -- hello.oki
```

Cargo compila el intérprete y lo ejecuta. El separador `--` hace que `hello.oki` se pase al programa como argumento.

El archivo [hello.oki](../hello.oki) contiene:

```oki
print("Hello world")
```

La salida del programa es:

```text
Hello world
```

También se puede indicar otra ruta. El programa lee el primer argumento como ruta de un archivo UTF-8; `.oki` es la extensión que usamos, pero el código todavía no comprueba la extensión. Los argumentos adicionales se ignoran.

## Reglas actuales

| Elemento | Comportamiento |
| --- | --- |
| Instrucción | `print("texto")`, con paréntesis obligatorios y sin punto y coma. |
| Argumento | Exactamente una cadena entre comillas dobles. Puede estar vacía. |
| Varios `print` | Se ejecutan en el orden del archivo; cada uno añade un salto de línea a su salida. |
| Espacios entre tokens | Se ignoran espacios, tabulaciones, retornos de carro y saltos de línea. |
| Contenido de una cadena | Se conserva el texto, incluidos Unicode y saltos de línea reales. |
| Secuencias de escape | No se interpretan: `\n` dentro del archivo son dos caracteres, barra y letra `n`. Una barra no permite escapar una comilla. |
| Archivo vacío | Se acepta y no imprime nada. |
| Errores de sintaxis o caracteres inválidos | Se informa del primer error y su línea; no se ejecuta ninguna instrucción del archivo. |

Por ejemplo, este programa es válido:

```oki
print ( "Hola" )
print("Mundo")
```

Produce `Hola` y `Mundo` en líneas separadas. El salto de línea entre instrucciones facilita la lectura, pero no es un separador obligatorio: el paréntesis de cierre termina cada `print`.

## Límites de esta versión

Todavía no hay números, operaciones, variables, comentarios, condiciones, bucles ni funciones definidas por el usuario. `print` es una instrucción reservada; sus paréntesis no implican que ya exista un sistema general de llamadas a funciones.

El scanner reconoce nombres como `nombre` o `print2`, pero el parser los rechaza: reconocer un token no significa que el lenguaje permita usarlo.

La ejecución recorre un árbol de sintaxis. No se genera código máquina ni bytecode, y no se han medido prestaciones del lenguaje. Las ampliaciones se decidirán paso a paso con el usuario.

## Qué se comprueba

Las seis pruebas actuales verifican el ejemplo `hello.oki`, espacios y Unicode, orden de varios `print` y cadenas vacías, archivo vacío, rechazo de programas inválidos sin salida parcial por errores de análisis, y propagación de errores de escritura.

```sh
cargo test
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
```

Para seguir el recorrido de una instrucción, leer [cómo funciona internamente](funcionamiento-interno.md). Para conocer la evolución, consultar el [historial](historial.md).
