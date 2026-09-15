use std::collections::HashMap;

use crate::{
    parser::{BinaryOp, Expr, Name, Stmt, UnaryOp},
    stdlib::{ArrayFunction, StandardLibrary},
    value::Type,
};

struct VariableInfo {
    declared_type: Type,
    is_constant: bool,
}

// Este entorno guarda tipos y si el nombre es constante, pero no valores.
// Cada bloque abre un ámbito nuevo; el último de la pila es el actual.
#[derive(Default)]
pub struct TypeChecker {
    scopes: Vec<HashMap<String, VariableInfo>>,
    library: StandardLibrary,
}

impl TypeChecker {
    pub fn check(&mut self, statements: &[Stmt]) -> Result<(), String> {
        self.library = StandardLibrary::default();
        self.scopes.push(HashMap::new());
        let result = self.check_statements(statements);
        self.scopes.pop();
        result
    }

    fn check_statements(&mut self, statements: &[Stmt]) -> Result<(), String> {
        for statement in statements {
            self.check_statement(statement)?;
        }
        Ok(())
    }

    fn check_statement(&mut self, statement: &Stmt) -> Result<(), String> {
        match statement {
            Stmt::Call(expression) => {
                self.check_call(expression)?;
            }
            Stmt::Import { path, is_use, line } => {
                if self.scopes.len() != 1 {
                    return Err(format!(
                        "Línea {line}: 'import' y 'use' solo se permiten en el ámbito global del archivo."
                    ));
                }
                self.library.import(path, *is_use, *line)?;
            }
            Stmt::Declare {
                declared_type,
                is_constant,
                name,
                initializer,
            } => {
                if self
                    .scopes
                    .last()
                    .is_some_and(|s| s.contains_key(&name.text))
                {
                    return Err(
                        name.error(&format!("La variable '{}' ya está declarada.", name.text))
                    );
                }
                let actual = self.expression_type_expected(initializer, Some(declared_type))?;
                Self::require_type(name, declared_type, &actual)?;
                // Registrar después del inicializador impide int x = x;.
                self.scopes.last_mut().expect("ámbito abierto").insert(
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
                let target_type = self.assignment_target(name, indices)?;
                let actual = self.expression_type_expected(value, Some(&target_type))?;
                Self::require_type(name, &target_type, &actual)?;
            }
            Stmt::CompoundAssign {
                name,
                indices,
                operator,
                value,
                line,
            } => {
                let target_type = self.assignment_target(name, indices)?;
                let operand_type = self.expression_type_expected(value, Some(&target_type))?;
                Self::binary_result(&target_type, operator.binary(), &operand_type).ok_or_else(
                    || {
                        format!(
                            "Línea {line}: el operador '{}' no admite {target_type} y {operand_type}. No hay conversiones implícitas.",
                            operator.symbol()
                        )
                    },
                )?;
            }
            Stmt::Increment {
                name,
                indices,
                operator,
                line,
            } => {
                let target_type = self.assignment_target(name, indices)?;
                if !matches!(target_type, Type::Int | Type::Float) {
                    return Err(format!(
                        "Línea {line}: el operador '{}' no admite {target_type}.",
                        operator.symbol()
                    ));
                }
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                line,
            } => {
                self.require_bool(condition, "if", *line)?;
                self.check_block(then_branch)?;
                if let Some(else_branch) = else_branch {
                    self.check_block(else_branch)?;
                }
            }
            Stmt::While {
                condition,
                body,
                line,
            } => {
                self.require_bool(condition, "while", *line)?;
                self.check_block(body)?;
            }
            Stmt::For {
                initializer,
                condition,
                update,
                body,
                line,
            } => {
                // El ámbito del contador abarca inicialización, condición,
                // actualización y cuerpo; el cuerpo abre además el suyo.
                self.scopes.push(HashMap::new());
                let result = self.check_for(initializer, condition, update, body, *line);
                self.scopes.pop();
                result?;
            }
            Stmt::Foreach {
                declared_type,
                name,
                iterable,
                body,
                line,
            } => {
                let iterable_type = self.expression_type(iterable)?;
                let Type::Array(element) = &iterable_type else {
                    return Err(format!(
                        "Línea {line}: 'foreach' solo recorre arrays; se recibió {iterable_type}."
                    ));
                };
                let element = element.as_ref().clone();
                Self::require_type(name, declared_type, &element)?;
                self.scopes.push(HashMap::new());
                self.scopes.last_mut().expect("ámbito abierto").insert(
                    name.text.clone(),
                    VariableInfo {
                        declared_type: declared_type.clone(),
                        is_constant: false,
                    },
                );
                let result = self.check_block(body);
                self.scopes.pop();
                result?;
            }
            Stmt::Print(expression) | Stmt::Println(expression) => {
                self.expression_type(expression)?;
            }
        }
        Ok(())
    }

    fn check_for(
        &mut self,
        initializer: &Stmt,
        condition: &Expr,
        update: &Stmt,
        body: &[Stmt],
        line: usize,
    ) -> Result<(), String> {
        self.check_statement(initializer)?;
        self.require_bool(condition, "for", line)?;
        self.check_statement(update)?;
        self.check_block(body)
    }

    fn require_bool(&self, condition: &Expr, keyword: &str, line: usize) -> Result<(), String> {
        let actual = self.expression_type(condition)?;
        if actual != Type::Bool {
            return Err(format!(
                "Línea {line}: la condición de '{keyword}' debe ser bool; se recibió {actual}."
            ));
        }
        Ok(())
    }

    fn check_block(&mut self, statements: &[Stmt]) -> Result<(), String> {
        self.scopes.push(HashMap::new());
        let result = self.check_statements(statements);
        self.scopes.pop();
        result
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
            Expr::Cast {
                path,
                target,
                value,
            } => self
                .library
                .check_cast(path, &self.expression_type(value)?, target),
            Expr::LibraryCall { path, .. } => self.check_call(expression)?.ok_or_else(|| {
                path.last()
                    .expect("ruta con nombre de método")
                    .error("'push' no devuelve un valor; úsalo como instrucción con ';'.")
            }),
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
                Self::binary_result(&left, *operator, &right).ok_or_else(|| {
                    format!(
                        "Línea {line}: el operador '{}' no admite {left} y {right}. No hay conversiones implícitas.",
                        operator.symbol()
                    )
                })
            }
        }
    }

    fn check_call(&self, expression: &Expr) -> Result<Option<Type>, String> {
        if matches!(expression, Expr::Cast { .. }) {
            return self.expression_type(expression).map(Some);
        }
        let Expr::LibraryCall {
            path,
            receiver,
            arguments,
        } = expression
        else {
            unreachable!("instrucción de llamada validada por el parser")
        };
        let function = self.library.resolve(path, receiver.is_some())?;
        let name = path.last().expect("ruta con nombre de método");
        function.check_arity(arguments.len(), receiver.is_some(), name)?;
        let array = receiver.as_deref().unwrap_or_else(|| &arguments[0]);
        let array_type = self.expression_type(array)?;
        let result = function.result_type(&array_type, name)?;
        if !matches!(function, ArrayFunction::Len) {
            let (target, indices) = array.clone().into_target().ok_or_else(|| {
                name.error("Se necesita una variable array modificable o uno de sus subarrays.")
            })?;
            self.assignment_target(&target, &indices)?;
        }
        if matches!(function, ArrayFunction::Push) {
            let Type::Array(element) = array_type else {
                unreachable!("tipo comprobado")
            };
            let value = arguments.last().expect("argumento de push validado");
            let actual = self.expression_type_expected(value, Some(&element))?;
            Self::require_type(name, &element, &actual)?;
        }
        Ok(result)
    }

    // Reglas de tipos de un operador binario. Devuelve None si los operandos no
    // son compatibles o no tienen el mismo tipo. Lo reutiliza `+=` y `-=`.
    fn binary_result(left: &Type, operator: BinaryOp, right: &Type) -> Option<Type> {
        let numeric = matches!(left, Type::Int | Type::Float);
        let result = match operator {
            BinaryOp::Add if numeric || *left == Type::String => Some(left.clone()),
            BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder
                if numeric =>
            {
                Some(left.clone())
            }
            BinaryOp::Equal | BinaryOp::NotEqual => Some(Type::Bool),
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual
                if numeric || matches!(left, Type::Char | Type::String) =>
            {
                Some(Type::Bool)
            }
            BinaryOp::And | BinaryOp::Or if *left == Type::Bool => Some(Type::Bool),
            _ => None,
        };
        result.filter(|_| left == right)
    }

    // Tipo del destino de una asignación, rechazando constantes y comprobando
    // cada índice. Lo comparten la asignación, la compuesta y el incremento.
    fn assignment_target(&self, name: &Name, indices: &[(Expr, usize)]) -> Result<Type, String> {
        let info = self.lookup(name)?;
        if info.is_constant {
            return Err(name.error(&format!(
                "No se puede reasignar la constante '{}'.",
                name.text
            )));
        }
        let mut target_type = info.declared_type.clone();
        for (index, line) in indices {
            target_type = self.indexed_type(target_type, index, *line)?;
        }
        Ok(target_type)
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
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&name.text))
            .ok_or_else(|| name.error(&format!(
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
