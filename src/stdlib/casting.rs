use crate::{
    parser::Name,
    value::{Type, Value},
};

// La compatibilidad depende de los tipos; la validez del contenido se decide
// al ejecutar, porque puede proceder de variables o de otras expresiones.
pub fn check_type(source: &Type, target: &Type, name: &Name) -> Result<(), String> {
    use Type::*;
    let basic = !matches!(source, Array(_)) && !matches!(target, Array(_));
    let compatible = source == target
        || matches!(
            (source, target),
            (_, String) | (String, _) | (Int, Float) | (Float, Int) | (Int, Char) | (Char, Int)
        );
    if basic && compatible {
        Ok(())
    } else {
        Err(name.error(&format!("Conversión no admitida de {source} a {target}.")))
    }
}

pub fn evaluate(value: Value, target: &Type, name: &Name) -> Result<Value, String> {
    let source = value
        .value_type()
        .ok_or_else(|| name.error("Casting no admite arrays."))?;
    check_type(&source, target, name)?;
    if source == *target {
        return Ok(value);
    }
    let invalid = |detail: &str| {
        name.error(&format!(
            "No se puede convertir de {source} a {target}: {detail}."
        ))
    };
    match (value, target) {
        (value, Type::String) => Ok(Value::String(value.to_string())),
        (Value::Int(value), Type::Float) => Ok(Value::Float(value as f64)),
        (Value::Float(value), Type::Int) => {
            let truncated = value.trunc();
            // i64::MAX redondeado a f64 es 2^63, que ya queda fuera del rango.
            // El límite superior debe ser exclusivo antes del `as` de Rust.
            if !truncated.is_finite()
                || !(-9223372036854775808.0..9223372036854775808.0).contains(&truncated)
            {
                return Err(invalid("valor fuera del rango de int"));
            }
            Ok(Value::Int(truncated as i64))
        }
        (Value::Char(value), Type::Int) => Ok(Value::Int(i64::from(u32::from(value)))),
        (Value::Int(value), Type::Char) => u32::try_from(value)
            .ok()
            .and_then(char::from_u32)
            .map(Value::Char)
            .ok_or_else(|| invalid("se esperaba un valor escalar Unicode válido")),
        (Value::String(text), Type::Int) => {
            let digits = text.strip_prefix(['+', '-']).unwrap_or(&text);
            if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(invalid("se esperaba texto entero decimal sin espacios"));
            }
            text.parse::<i64>()
                .map(Value::Int)
                .map_err(|_| invalid("valor fuera del rango de int"))
        }
        (Value::String(text), Type::Float) => {
            // parse admite NaN e infinito: el lenguaje solo acepta resultados finitos.
            if text.is_empty() || text.chars().any(char::is_whitespace) {
                return Err(invalid("se esperaba texto numérico sin espacios"));
            }
            let value = text
                .parse::<f64>()
                .map_err(|_| invalid("texto float inválido"))?;
            if !value.is_finite() {
                return Err(invalid("valor fuera del rango de float finito"));
            }
            Ok(Value::Float(value))
        }
        (Value::String(text), Type::Bool) => match text.as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err(invalid("se esperaba exactamente 'true' o 'false'")),
        },
        (Value::String(text), Type::Char) => {
            let mut characters = text.chars();
            match (characters.next(), characters.next()) {
                (Some(value), None) => Ok(Value::Char(value)),
                _ => Err(invalid("se esperaba exactamente un valor escalar Unicode")),
            }
        }
        _ => Err(invalid("conversión no admitida")),
    }
}
