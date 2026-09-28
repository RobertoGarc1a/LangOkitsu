use crate::{parser::Name, value::Type};

pub mod array;
pub mod casting;

// La biblioteca viene incluida en el intérprete. Estas marcas habilitan sus
// nombres durante la comprobación; no descargan ni ejecutan otro archivo.
#[derive(Clone, Default)]
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

#[derive(Clone, Default)]
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

    pub fn check_array(&self, path: &[Name], method: bool) -> Result<(), String> {
        array::resolve(path, method)?;
        self.array.require("Array", path, method)?;
        Ok(())
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
