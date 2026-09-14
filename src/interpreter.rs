use std::{collections::HashMap, error::Error, io::Write};

use crate::{
    parser::{BinaryOp, Expr, Name, Stmt, UnaryOp},
    value::Value,
};

// El entorno de ejecución relaciona cada nombre con su valor actual.
// Cada bloque abre un ámbito nuevo; el último de la pila es el actual.
pub struct Interpreter<W: Write> {
    output: W,
    scopes: Vec<HashMap<String, Value>>,
}

impl<W: Write> Interpreter<W> {
    pub fn new(output: W) -> Self {
        Self {
            output,
            scopes: vec![HashMap::new()],
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
                self.scopes
                    .last_mut()
                    .expect("ámbito abierto")
                    .insert(name.text.clone(), value);
            }
            Stmt::Assign {
                name,
                indices,
                value,
            } => {
                let scope = self.scope_containing(&name.text).ok_or_else(|| {
                    name.error(&format!("La variable '{}' no está declarada.", name.text))
                })?;
                // Resolver y validar el destino antes del valor nuevo. No hay
                // cambios parciales si falla un índice o la expresión asignada.
                let mut positions = Vec::new();
                {
                    let mut target = &self.scopes[scope][&name.text];
                    for (index, line) in indices {
                        let (elements, position) =
                            Self::array_position(target, self.evaluate(index)?, *line)?;
                        positions.push(position);
                        target = &elements[position];
                    }
                }
                let value = self.evaluate(value)?;
                let mut target = self.scopes[scope]
                    .get_mut(&name.text)
                    .expect("nombre validado");
                for position in positions {
                    let Value::Array(elements) = target else {
                        unreachable!("destino validado")
                    };
                    target = &mut elements[position];
                }
                *target = value;
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let Value::Bool(condition) = self.evaluate(condition)? else {
                    return Err("La condición de 'if' debe ser bool.".into());
                };
                if condition {
                    self.execute_block(then_branch)?;
                } else if let Some(else_branch) = else_branch {
                    self.execute_block(else_branch)?;
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                while self.evaluate_bool(condition, "while")? {
                    self.execute_block(body)?;
                }
            }
            Stmt::For {
                initializer,
                condition,
                update,
                body,
                ..
            } => {
                // El contador vive en un ámbito propio que envuelve el bucle.
                self.scopes.push(HashMap::new());
                let result = self.execute_for(initializer, condition, update, body);
                self.scopes.pop();
                result?;
            }
            Stmt::Foreach {
                name,
                iterable,
                body,
                ..
            } => {
                let value = self.evaluate(iterable)?;
                let Value::Array(elements) = value else {
                    return Err("'foreach' solo recorre arrays.".into());
                };
                self.scopes.push(HashMap::new());
                let result = self.execute_foreach(name, elements, body);
                self.scopes.pop();
                result?;
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

    fn execute_block(&mut self, statements: &[Stmt]) -> Result<(), Box<dyn Error>> {
        self.scopes.push(HashMap::new());
        for statement in statements {
            self.execute(statement)?;
        }
        self.scopes.pop();
        Ok(())
    }

    fn evaluate_bool(&self, condition: &Expr, keyword: &str) -> Result<bool, String> {
        let Value::Bool(value) = self.evaluate(condition)? else {
            return Err(format!("La condición de '{keyword}' debe ser bool."));
        };
        Ok(value)
    }

    fn execute_for(
        &mut self,
        initializer: &Stmt,
        condition: &Expr,
        update: &Stmt,
        body: &[Stmt],
    ) -> Result<(), Box<dyn Error>> {
        self.execute(initializer)?;
        // La condición se comprueba antes de cada vuelta; la actualización
        // ocurre después del cuerpo, como en el 'for' de C.
        while self.evaluate_bool(condition, "for")? {
            self.execute_block(body)?;
            self.execute(update)?;
        }
        Ok(())
    }

    fn execute_foreach(
        &mut self,
        name: &Name,
        elements: Vec<Value>,
        body: &[Stmt],
    ) -> Result<(), Box<dyn Error>> {
        for element in elements {
            // Cada vuelta reinicia la variable con una copia del elemento.
            self.scopes
                .last_mut()
                .expect("ámbito abierto")
                .insert(name.text.clone(), element);
            self.execute_block(body)?;
        }
        Ok(())
    }

    fn scope_containing(&self, name: &str) -> Option<usize> {
        self.scopes
            .iter()
            .rposition(|scope| scope.contains_key(name))
    }

    fn evaluate(&self, expression: &Expr) -> Result<Value, String> {
        match expression {
            Expr::Literal(value) => Ok(value.clone()),
            Expr::Variable(name) => self
                .scopes
                .iter()
                .rev()
                .find_map(|scope| scope.get(&name.text))
                .cloned()
                .ok_or_else(|| {
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
