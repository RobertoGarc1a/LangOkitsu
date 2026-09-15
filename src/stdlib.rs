use crate::{
    parser::Name,
    value::{Type, Value},
};

// La biblioteca viene incluida en el intérprete. Estas marcas habilitan sus
// nombres durante la comprobación; no descargan ni ejecutan otro archivo.
#[derive(Default)]
pub struct ArrayLibrary {
    imported: bool,
    short_name: bool,
}

impl ArrayLibrary {
    pub fn import(&mut self, path: &[Name], is_use: bool, line: usize) -> Result<(), String> {
        if !matches!(path, [root, module] if root.text == "std" && module.text == "Array") {
            return Err(format!(
                "Línea {line}: biblioteca desconocida '{}'; solo está disponible 'std::Array'.",
                path_text(path)
            ));
        }
        if is_use {
            if !self.imported {
                return Err(format!(
                    "Línea {line}: 'use std::Array;' requiere un 'import std::Array;' anterior."
                ));
            }
            self.short_name = true;
        } else {
            self.imported = true;
        }
        Ok(())
    }

    pub fn resolve(&self, path: &[Name], method: bool) -> Result<ArrayFunction, String> {
        let function = ArrayFunction::resolve(path, method)?;
        let name = path.last().expect("ruta con nombre de método");
        if !self.imported {
            return Err(name.error("El método 'len' requiere un 'import std::Array;' anterior."));
        }
        if !method && path.len() == 2 && !self.short_name {
            return Err(name.error("El nombre corto 'Array' requiere un 'use std::Array;' anterior; también puedes escribir 'std::Array::len(array)'."));
        }
        Ok(function)
    }
}

fn path_text(path: &[Name]) -> String {
    path.iter()
        .map(|name| name.text.as_str())
        .collect::<Vec<_>>()
        .join("::")
}

pub enum ArrayFunction {
    Len,
}

impl ArrayFunction {
    pub fn resolve(path: &[Name], method: bool) -> Result<Self, String> {
        let name = path.last().expect("ruta con nombre de método");
        let valid_path = if method {
            path.len() == 1
        } else {
            matches!(path, [root, module, _] if root.text == "std" && module.text == "Array")
                || matches!(path, [module, _] if module.text == "Array")
        };
        if !valid_path {
            return Err(name.error(&format!(
                "Llamada de biblioteca desconocida '{}'.",
                path_text(path)
            )));
        }
        match name.text.as_str() {
            "len" => Ok(Self::Len),
            _ => Err(name.error(&format!(
                "Método de biblioteca desconocido '{}'; std::Array solo ofrece 'len'.",
                name.text
            ))),
        }
    }

    pub fn check_arity(&self, arguments: usize, method: bool, name: &Name) -> Result<(), String> {
        let expected = if method { 0 } else { 1 };
        if arguments != expected {
            return Err(name.error(&format!(
                "'len' esperaba {expected} argumentos entre paréntesis; recibió {arguments}."
            )));
        }
        Ok(())
    }

    pub fn result_type(&self, array: &Type, name: &Name) -> Result<Type, String> {
        if !matches!(array, Type::Array(_)) {
            return Err(name.error(&format!("'len' solo admite arrays; se recibió {array}.")));
        }
        Ok(Type::Int)
    }

    pub fn evaluate(&self, array: Value, name: &Name) -> Result<Value, String> {
        let Value::Array(elements) = array else {
            return Err(name.error("'len' solo admite arrays."));
        };
        i64::try_from(elements.len())
            .map(Value::Int)
            .map_err(|_| name.error("La longitud del array está fuera del rango de int."))
    }
}
