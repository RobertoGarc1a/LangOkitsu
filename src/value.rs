use std::fmt;

// El tipo declarado y el valor guardado son conceptos distintos.
#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Char,
    String,
    Array(Box<Type>),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Int => "int",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::Char => "char",
            Self::String => "string",
            Self::Array(element) => return write!(f, "{element}[]"),
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
    Array(Vec<Value>),
}

impl Value {
    pub fn value_type(&self) -> Option<Type> {
        Some(match self {
            Self::Int(_) => Type::Int,
            Self::Float(_) => Type::Float,
            Self::Bool(_) => Type::Bool,
            Self::Char(_) => Type::Char,
            Self::String(_) => Type::String,
            // Un array vacío no permite deducir el tipo a partir de sus valores.
            Self::Array(values) => Type::Array(Box::new(values.first()?.value_type()?)),
        })
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
            Self::Array(values) => {
                f.write_str("[")?;
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    match value {
                        Self::String(text) => write!(f, "\"{text}\"")?,
                        Self::Char(character) => write!(f, "'{character}'")?,
                        _ => write!(f, "{value}")?,
                    }
                }
                f.write_str("]")
            }
        }
    }
}
