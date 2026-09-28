use crate::{
    parser::Name,
    value::{Type, Value},
};

use super::path_text;

pub fn resolve(path: &[Name], method: bool) -> Result<(), String> {
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
        "len" | "push" | "pop" => Ok(()),
        _ => Err(unknown_method(name)),
    }
}

pub fn mutates(name: &Name) -> bool {
    matches!(name.text.as_str(), "push" | "pop")
}

pub fn takes_value(name: &Name) -> bool {
    name.text == "push"
}

pub fn check_arity(arguments: usize, method: bool, name: &Name) -> Result<(), String> {
    let expected = usize::from(!method) + usize::from(takes_value(name));
    if arguments != expected {
        return Err(name.error(&format!(
            "'{}' esperaba {expected} argumentos entre paréntesis; recibió {arguments}.",
            name.text
        )));
    }
    Ok(())
}

pub fn result_type(array: &Type, name: &Name) -> Result<Option<Type>, String> {
    let Type::Array(element) = array else {
        return Err(name.error(&format!(
            "'{}' solo admite arrays; se recibió {array}.",
            name.text
        )));
    };
    match name.text.as_str() {
        "len" => Ok(Some(Type::Int)),
        "push" => Ok(None),
        "pop" => Ok(Some(element.as_ref().clone())),
        _ => Err(unknown_method(name)),
    }
}

pub fn evaluate(
    array: &mut Value,
    value: Option<Value>,
    name: &Name,
) -> Result<Option<Value>, String> {
    let Value::Array { elements, .. } = array else {
        return Err(name.error(&format!("'{}' solo admite arrays.", name.text)));
    };
    match name.text.as_str() {
        "len" => i64::try_from(elements.len())
            .map(Value::Int)
            .map(Some)
            .map_err(|_| name.error("La longitud del array está fuera del rango de int.")),
        "push" => {
            elements.push(value.expect("argumento de push validado"));
            Ok(None)
        }
        "pop" => elements
            .pop()
            .map(Some)
            .ok_or_else(|| name.error("No se puede hacer 'pop' de un array vacío.")),
        _ => Err(unknown_method(name)),
    }
}

fn unknown_method(name: &Name) -> String {
    name.error(&format!(
        "Método de biblioteca desconocido '{}'; std::Array ofrece 'len', 'push' y 'pop'.",
        name.text
    ))
}
