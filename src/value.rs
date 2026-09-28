use std::fmt;

// El tipo declarado y el valor guardado son conceptos distintos.
#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Char,
    String,
    Struct(String),
    Array(Box<Type>),
    Union(Vec<Type>),
}

impl Type {
    // Todos los tipos posibles del valor deben estar permitidos por la anotación.
    pub fn accepts(&self, actual: &Type) -> bool {
        match (self, actual) {
            (_, Self::Union(types)) => types.iter().all(|kind| self.accepts(kind)),
            (Self::Union(types), _) => types.iter().any(|kind| kind == actual),
            _ => self == actual,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Int => "int",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::Char => "char",
            Self::String => "string",
            Self::Struct(name) => return f.write_str(name),
            Self::Array(element) => return write!(f, "{element}[]"),
            Self::Union(types) => {
                for (index, kind) in types.iter().enumerate() {
                    if index > 0 {
                        f.write_str(" || ")?;
                    }
                    write!(f, "{kind}")?;
                }
                return Ok(());
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
    Struct {
        name: String,
        fields: Vec<(String, Value)>,
    },
    Array {
        elements: Vec<Value>,
        element_type: Type,
    },
}

// Un tipo recursivo puede crecer también en bucles, sin llamadas anidadas.
// Acotar los valores mantiene seguras las copias y el formateo recursivos.
const MAX_VALUE_DEPTH: usize = 100;

impl Value {
    pub fn check_depth(&self, ancestors: usize, line: usize) -> Result<(), String> {
        let mut pending = vec![(self, ancestors)];
        while let Some((value, depth)) = pending.pop() {
            let children: Vec<&Value> = match value {
                Self::Struct { fields, .. } => fields.iter().map(|(_, value)| value).collect(),
                Self::Array { elements, .. } => elements.iter().collect(),
                _ => continue,
            };
            if depth >= MAX_VALUE_DEPTH {
                return Err(format!(
                    "Línea {line}: se superó la profundidad máxima de valores ({MAX_VALUE_DEPTH})."
                ));
            }
            pending.extend(children.into_iter().map(|value| (value, depth + 1)));
        }
        Ok(())
    }

    fn fmt_nested(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(text) => write!(f, "\"{text}\""),
            Self::Char(character) => write!(f, "'{character}'"),
            _ => write!(f, "{self}"),
        }
    }

    pub fn value_type(&self) -> Type {
        match self {
            Self::Int(_) => Type::Int,
            Self::Float(_) => Type::Float,
            Self::Bool(_) => Type::Bool,
            Self::Char(_) => Type::Char,
            Self::String(_) => Type::String,
            Self::Struct { name, .. } => Type::Struct(name.clone()),
            // El tipo del elemento se conserva incluso cuando no quedan valores.
            Self::Array { element_type, .. } => Type::Array(Box::new(element_type.clone())),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(value) => write!(f, "{value}"),
            // Conserva la distinción visual entre 1 (int) y 1.0 (float).
            Self::Float(value) => write!(f, "{value:?}"),
            Self::Bool(value) => write!(f, "{value}"),
            Self::Char(value) => write!(f, "{value}"),
            Self::String(value) => f.write_str(value),
            Self::Struct { name, fields } => {
                write!(f, "{name} {{")?;
                for (index, (field, value)) in fields.iter().enumerate() {
                    if index > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, " {field}: ")?;
                    value.fmt_nested(f)?;
                }
                f.write_str(" }")
            }
            Self::Array {
                elements: values, ..
            } => {
                f.write_str("[")?;
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    value.fmt_nested(f)?;
                }
                f.write_str("]")
            }
        }
    }
}
