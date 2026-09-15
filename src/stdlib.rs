use crate::{
    parser::Name,
    value::{Type, Value},
};

pub mod casting;

// La biblioteca viene incluida en el intérprete. Estas marcas habilitan sus
// nombres durante la comprobación; no descargan ni ejecutan otro archivo.
#[derive(Default)]
struct LibraryAccess {
    imported: bool,
    short_name: bool,
}

impl LibraryAccess {
    fn import(&mut self, module: &str, is_use: bool, line: usize) -> Result<(), String> {
        if is_use {
            if !self.imported {
                return Err(format!(
                    "Línea {line}: 'use std::{module};' requiere un 'import std::{module};' anterior."
                ));
            }
            self.short_name = true;
        } else {
            self.imported = true;
        }
        Ok(())
    }

    fn require(&self, module: &str, path: &[Name], method: bool) -> Result<(), String> {
        let name = path.last().expect("ruta con nombre de método");
        if !self.imported {
            return Err(name.error(&format!(
                "El método '{}' requiere un 'import std::{module};' anterior.",
                name.text
            )));
        }
        if !method && path.len() == 2 && !self.short_name {
            return Err(name.error(&format!("El nombre corto '{module}' requiere un 'use std::{module};' anterior; también puedes usar la ruta completa 'std::{module}'.")));
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct StandardLibrary {
    array: LibraryAccess,
    casting: LibraryAccess,
}

impl StandardLibrary {
    pub fn import(&mut self, path: &[Name], is_use: bool, line: usize) -> Result<(), String> {
        if let [root, module] = path
            && root.text == "std"
        {
            let access = match module.text.as_str() {
                "Array" => Some(&mut self.array),
                "Casting" => Some(&mut self.casting),
                _ => None,
            };
            if let Some(access) = access {
                return access.import(&module.text, is_use, line);
            }
        }
        Err(format!(
            "Línea {line}: biblioteca desconocida '{}'; están disponibles 'std::Array' y 'std::Casting'.",
            path_text(path)
        ))
    }

    pub fn resolve(&self, path: &[Name], method: bool) -> Result<ArrayFunction, String> {
        let function = ArrayFunction::resolve(path, method)?;
        self.array.require("Array", path, method)?;
        Ok(function)
    }

    pub fn check_cast(&self, path: &[Name], source: &Type, target: &Type) -> Result<Type, String> {
        let name = path.last().expect("ruta de conversión");
        let valid_path = path.len() == 1
            || matches!(path, [module, _] if module.text == "Casting")
            || matches!(path, [root, module, _] if root.text == "std" && module.text == "Casting");
        if !valid_path {
            return Err(name.error(&format!(
                "Llamada de biblioteca desconocida '{}'.",
                path_text(path)
            )));
        }
        self.casting.require("Casting", path, path.len() == 1)?;
        casting::check_type(source, target, name)?;
        Ok(target.clone())
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
    Push,
    Pop,
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
            "push" => Ok(Self::Push),
            "pop" => Ok(Self::Pop),
            _ => Err(name.error(&format!(
                "Método de biblioteca desconocido '{}'; std::Array ofrece 'len', 'push' y 'pop'.",
                name.text
            ))),
        }
    }

    pub fn check_arity(&self, arguments: usize, method: bool, name: &Name) -> Result<(), String> {
        let expected = usize::from(!method) + usize::from(matches!(self, Self::Push));
        if arguments != expected {
            return Err(name.error(&format!(
                "'{}' esperaba {expected} argumentos entre paréntesis; recibió {arguments}.",
                name.text
            )));
        }
        Ok(())
    }

    pub fn result_type(&self, array: &Type, name: &Name) -> Result<Option<Type>, String> {
        let Type::Array(element) = array else {
            return Err(name.error(&format!(
                "'{}' solo admite arrays; se recibió {array}.",
                name.text
            )));
        };
        Ok(match self {
            Self::Len => Some(Type::Int),
            Self::Push => None,
            Self::Pop => Some(element.as_ref().clone()),
        })
    }

    pub fn evaluate(
        &self,
        array: &mut Value,
        value: Option<Value>,
        name: &Name,
    ) -> Result<Option<Value>, String> {
        let Value::Array(elements) = array else {
            return Err(name.error(&format!("'{}' solo admite arrays.", name.text)));
        };
        match self {
            Self::Len => i64::try_from(elements.len())
                .map(Value::Int)
                .map(Some)
                .map_err(|_| name.error("La longitud del array está fuera del rango de int.")),
            Self::Push => {
                elements.push(value.expect("argumento de push validado"));
                Ok(None)
            }
            Self::Pop => elements
                .pop()
                .map(Some)
                .ok_or_else(|| name.error("No se puede hacer 'pop' de un array vacío.")),
        }
    }
}
