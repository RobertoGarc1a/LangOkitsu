use std::collections::HashMap;

use crate::{
    parser::{BinaryOp, Expr, Name, Stmt, UnaryOp},
    value::Type,
};

struct VariableInfo {
    declared_type: Type,
    is_constant: bool,
}

// Este entorno guarda tipos y si el nombre es constante, pero no valores.
#[derive(Default)]
pub struct TypeChecker {
    types: HashMap<String, VariableInfo>,
}

impl TypeChecker {
    pub fn check(&mut self, statements: &[Stmt]) -> Result<(), String> {
        for statement in statements {
            match statement {
                Stmt::Declare {
                    declared_type,
                    is_constant,
                    name,
                    initializer,
                } => {
                    if self.types.contains_key(&name.text) {
                        return Err(
                            name.error(&format!("La variable '{}' ya está declarada.", name.text))
                        );
                    }
                    let actual = self.expression_type_expected(initializer, Some(declared_type))?;
                    Self::require_type(name, declared_type, &actual)?;
                    // Registrar después del inicializador impide int x = x;.
                    self.types.insert(
                        name.text.clone(),
                        VariableInfo {
                            declared_type: declared_type.clone(),
                            is_constant: *is_constant,
                        },
                    );
                }
                Stmt::Assign {
                    name,
                    indices,
                    value,
                } => {
                    let expected = self.lookup(name)?;
                    if expected.is_constant {
                        return Err(name.error(&format!(
                            "No se puede reasignar la constante '{}'.",
                            name.text
                        )));
                    }
                    let mut target_type = expected.declared_type.clone();
                    for (index, line) in indices {
                        target_type = self.indexed_type(target_type, index, *line)?;
                    }
                    let actual = self.expression_type_expected(value, Some(&target_type))?;
                    Self::require_type(name, &target_type, &actual)?;
                }
                Stmt::Print(expression) | Stmt::Println(expression) => {
                    self.expression_type(expression)?;
                }
            }
        }
        Ok(())
    }

    fn expression_type(&self, expression: &Expr) -> Result<Type, String> {
        self.expression_type_expected(expression, None)
    }

    // El tipo declarado da contexto a [], sin inferir tipos de variables.
    fn expression_type_expected(
        &self,
        expression: &Expr,
        expected: Option<&Type>,
    ) -> Result<Type, String> {
        match expression {
            Expr::Literal(value) => value
                .value_type()
                .ok_or_else(|| "Literal sin tipo de elemento.".to_string()),
            Expr::Variable(name) => Ok(self.lookup(name)?.declared_type.clone()),
            Expr::Array { elements, line } => {
                let element_type = match expected {
                    Some(Type::Array(element)) => element.as_ref().clone(),
                    _ => {
                        let first = elements.first().ok_or_else(|| format!(
                            "Línea {line}: un array vacío necesita un tipo declarado, por ejemplo int[] datos = [];"
                        ))?;
                        self.expression_type(first)?
                    }
                };
                for element in elements {
                    let actual = self.expression_type_expected(element, Some(&element_type))?;
                    if actual != element_type {
                        return Err(format!(
                            "Línea {line}: elemento de array incompatible: se esperaba {element_type}, se recibió {actual}. No hay conversiones implícitas."
                        ));
                    }
                }
                Ok(Type::Array(Box::new(element_type)))
            }
            Expr::Index { array, index, line } => {
                self.indexed_type(self.expression_type(array)?, index, *line)
            }
            Expr::Unary {
                operator,
                operand,
                line,
            } => {
                let kind = self.expression_type(operand)?;
                let valid = match operator {
                    UnaryOp::Plus | UnaryOp::Minus => matches!(kind, Type::Int | Type::Float),
                    UnaryOp::Not => kind == Type::Bool,
                };
                if valid {
                    Ok(kind)
                } else {
                    Err(format!(
                        "Línea {line}: el operador '{}' no admite {kind}.",
                        operator.symbol()
                    ))
                }
            }
            Expr::Binary {
                left,
                operator,
                right,
                line,
            } => {
                // Incluso las ramas que podrían omitirse por cortocircuito
                // deben tener nombres y tipos válidos antes de ejecutar.
                let left = self.expression_type(left)?;
                let right = self.expression_type(right)?;
                let numeric = matches!(left, Type::Int | Type::Float);
                let result = match operator {
                    BinaryOp::Add if numeric || left == Type::String => Some(left.clone()),
                    BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder
                        if numeric =>
                    {
                        Some(left.clone())
                    }
                    BinaryOp::Equal | BinaryOp::NotEqual => Some(Type::Bool),
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual
                        if numeric || matches!(left, Type::Char | Type::String) =>
                    {
                        Some(Type::Bool)
                    }
                    BinaryOp::And | BinaryOp::Or if left == Type::Bool => Some(Type::Bool),
                    _ => None,
                };
                result.filter(|_| left == right).ok_or_else(|| format!(
                    "Línea {line}: el operador '{}' no admite {left} y {right}. No hay conversiones implícitas.", operator.symbol()
                ))
            }
        }
    }

    fn indexed_type(&self, array: Type, index: &Expr, line: usize) -> Result<Type, String> {
        let Type::Array(element) = array else {
            return Err(format!(
                "Línea {line}: solo se pueden indexar arrays; se recibió {array}."
            ));
        };
        let actual = self.expression_type(index)?;
        if actual != Type::Int {
            return Err(format!(
                "Línea {line}: el índice debe ser int; se recibió {actual}."
            ));
        }
        Ok(*element)
    }

    fn lookup(&self, name: &Name) -> Result<&VariableInfo, String> {
        self.types.get(&name.text).ok_or_else(|| name.error(&format!(
            "La variable '{}' no está declarada. Debe declararse antes de usarla, indicando su tipo.", name.text
        )))
    }

    fn require_type(name: &Name, expected: &Type, actual: &Type) -> Result<(), String> {
        if expected != actual {
            return Err(name.error(&format!(
                "Tipo incompatible para '{}': se esperaba {expected}, se recibió {actual}. No hay conversiones implícitas.", name.text
            )));
        }
        Ok(())
    }
}
