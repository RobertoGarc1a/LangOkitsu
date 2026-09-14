use std::{collections::HashMap, error::Error, io::Write};

use crate::{
    parser::{BinaryOp, Expr, Stmt, UnaryOp},
    value::Value,
};

// El entorno de ejecución relaciona cada nombre con su valor actual.
pub struct Interpreter<W: Write> {
    output: W,
    values: HashMap<String, Value>,
}

impl<W: Write> Interpreter<W> {
    pub fn new(output: W) -> Self {
        Self {
            output,
            values: HashMap::new(),
        }
    }

    // run() solo entrega programas que han superado la comprobación de tipos.
    pub fn interpret(&mut self, statements: &[Stmt]) -> Result<(), Box<dyn Error>> {
        for statement in statements {
            self.execute(statement)?;
        }
        Ok(())
    }

    fn execute(&mut self, statement: &Stmt) -> Result<(), Box<dyn Error>> {
        match statement {
            Stmt::Declare {
                name, initializer, ..
            } => {
                let value = self.evaluate(initializer)?;
                self.values.insert(name.text.clone(), value);
            }
            Stmt::Assign {
                name,
                indices,
                value,
            } => {
                // Resolver y validar el destino antes del valor nuevo. No hay
                // cambios parciales si falla un índice o la expresión asignada.
                let mut positions = Vec::new();
                let mut target = self.values.get(&name.text).ok_or_else(|| {
                    name.error(&format!("La variable '{}' no está declarada.", name.text))
                })?;
                for (index, line) in indices {
                    let (elements, position) =
                        Self::array_position(target, self.evaluate(index)?, *line)?;
                    positions.push(position);
                    target = &elements[position];
                }
                let value = self.evaluate(value)?;
                let mut target = self.values.get_mut(&name.text).expect("nombre validado");
                for position in positions {
                    let Value::Array(elements) = target else {
                        unreachable!("destino validado")
                    };
                    target = &mut elements[position];
                }
                *target = value;
            }
            Stmt::Print(expression) => {
                let value = self.evaluate(expression)?;
                write!(self.output, "{value}")?;
            }
            Stmt::Println(expression) => {
                let value = self.evaluate(expression)?;
                writeln!(self.output, "{value}")?;
            }
        }
        Ok(())
    }

    fn evaluate(&self, expression: &Expr) -> Result<Value, String> {
        match expression {
            Expr::Literal(value) => Ok(value.clone()),
            Expr::Variable(name) => self.values.get(&name.text).cloned().ok_or_else(|| {
                name.error(&format!("La variable '{}' no está declarada.", name.text))
            }),
            Expr::Array { elements, .. } => elements
                .iter()
                .map(|element| self.evaluate(element))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            Expr::Index { array, index, line } => {
                let array = self.evaluate(array)?;
                let (elements, position) =
                    Self::array_position(&array, self.evaluate(index)?, *line)?;
                Ok(elements[position].clone())
            }
            Expr::Unary {
                operator,
                operand,
                line,
            } => {
                let value = self.evaluate(operand)?;
                match (operator, value) {
                    (UnaryOp::Plus, value @ (Value::Int(_) | Value::Float(_))) => Ok(value),
                    (UnaryOp::Minus, Value::Int(value)) => {
                        value.checked_neg().map(Value::Int).ok_or_else(|| {
                            format!("Línea {line}: resultado fuera del rango de int en '-'.")
                        })
                    }
                    (UnaryOp::Minus, Value::Float(value)) => Ok(Value::Float(-value)),
                    (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
                    _ => Err(format!(
                        "Línea {line}: operando incompatible para '{}'.",
                        operator.symbol()
                    )),
                }
            }
            Expr::Binary {
                left,
                operator,
                right,
                line,
            } => {
                let left = self.evaluate(left)?;
                // Cortocircuito: el segundo operando solo se evalúa si hace falta
                // para determinar el resultado lógico.
                if matches!(
                    (operator, &left),
                    (BinaryOp::And, Value::Bool(false)) | (BinaryOp::Or, Value::Bool(true))
                ) {
                    return Ok(left);
                }
                let right = self.evaluate(right)?;
                Self::binary(left, *operator, right, *line)
            }
        }
    }

    fn array_position(
        array: &Value,
        index: Value,
        line: usize,
    ) -> Result<(&[Value], usize), String> {
        let (Value::Array(elements), Value::Int(index)) = (array, index) else {
            return Err(format!(
                "Línea {line}: se esperaba un array y un índice int."
            ));
        };
        let position = usize::try_from(index).ok().filter(|position| *position < elements.len())
            .ok_or_else(|| format!("Línea {line}: índice {index} fuera de rango para un array de longitud {}. Los índices empiezan en 0.", elements.len()))?;
        Ok((elements, position))
    }

    fn binary(left: Value, operator: BinaryOp, right: Value, line: usize) -> Result<Value, String> {
        use BinaryOp::*;

        let range_error = |kind| {
            format!(
                "Línea {line}: resultado fuera del rango de {kind} en '{}'.",
                operator.symbol()
            )
        };
        let zero_error = || {
            format!(
                "Línea {line}: división o resto por cero en '{}'.",
                operator.symbol()
            )
        };
        if matches!(operator, Equal | NotEqual) {
            return Ok(Value::Bool(if matches!(operator, Equal) {
                left == right
            } else {
                left != right
            }));
        }
        if matches!(operator, Less | LessEqual | Greater | GreaterEqual) {
            let ordering = match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => a.partial_cmp(b),
                (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
                (Value::Char(a), Value::Char(b)) => a.partial_cmp(b),
                (Value::String(a), Value::String(b)) => a.partial_cmp(b),
                _ => None,
            }
            .ok_or_else(|| format!("Línea {line}: valores no comparables."))?;
            return Ok(Value::Bool(match operator {
                Less => ordering.is_lt(),
                LessEqual => ordering.is_le(),
                Greater => ordering.is_gt(),
                GreaterEqual => ordering.is_ge(),
                _ => unreachable!(),
            }));
        }
        match (left, operator, right) {
            (
                Value::Int(a),
                op @ (Add | Subtract | Multiply | Divide | Remainder),
                Value::Int(b),
            ) => {
                if matches!(op, Divide | Remainder) && b == 0 {
                    return Err(zero_error());
                }
                let result = match op {
                    Add => a.checked_add(b),
                    Subtract => a.checked_sub(b),
                    Multiply => a.checked_mul(b),
                    Divide => a.checked_div(b),
                    // El resto al dividir por -1 es cero, incluso para i64::MIN;
                    // checked_rem rechaza ese caso por el cociente intermedio.
                    Remainder if b == -1 => Some(0),
                    Remainder => a.checked_rem(b),
                    _ => unreachable!(),
                };
                result.map(Value::Int).ok_or_else(|| range_error("int"))
            }
            (
                Value::Float(a),
                op @ (Add | Subtract | Multiply | Divide | Remainder),
                Value::Float(b),
            ) => {
                if matches!(op, Divide | Remainder) && b == 0.0 {
                    return Err(zero_error());
                }
                let result = match op {
                    Add => a + b,
                    Subtract => a - b,
                    Multiply => a * b,
                    Divide => a / b,
                    Remainder => a % b,
                    _ => unreachable!(),
                };
                if result.is_finite() {
                    Ok(Value::Float(result))
                } else {
                    Err(range_error("float finito"))
                }
            }
            (Value::String(mut a), Add, Value::String(b)) => {
                a.push_str(&b);
                Ok(Value::String(a))
            }
            (Value::Bool(a), And, Value::Bool(b)) => Ok(Value::Bool(a && b)),
            (Value::Bool(a), Or, Value::Bool(b)) => Ok(Value::Bool(a || b)),
            _ => Err(format!(
                "Línea {line}: operandos incompatibles para '{}'.",
                operator.symbol()
            )),
        }
    }
}
