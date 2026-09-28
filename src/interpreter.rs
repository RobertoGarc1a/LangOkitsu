use std::{
    collections::{HashMap, HashSet},
    error::Error,
    io::Write,
    rc::Rc,
};

use crate::{
    parser::{BinaryOp, CallArgument, Expr, FieldDef, MatchArm, Name, Stmt, TargetStep, UnaryOp},
    stdlib::{array, casting},
    value::{Type, Value},
};

// La recursión sin fin no debe abortar el proceso: se detiene con un error
// propio cuando se superan demasiadas llamadas anidadas.
const MAX_CALL_DEPTH: usize = 100;

// Cuerpo ya resuelto de una función propia. Se guarda tras un `Rc` para
// clonar la definición al llamarla sin duplicar sus instrucciones.
struct FunctionDef {
    parameters: Vec<Name>,
    body: Vec<Stmt>,
}

// Un alias inout apunta al almacenamiento de una llamada anterior, que sigue
// vivo hasta que regresemos. Nunca se guarda un alias dentro de un Value.
enum Binding {
    Owned(Value),
    Alias { scope: usize, name: String },
}

// La ruta ya evaluada conserva posiciones de array y nombres de campo.
// No contiene expresiones que puedan volver a ejecutar efectos.
enum TargetPosition {
    Index { index: usize, element_type: Type },
    Field { name: String, structure: String },
}

// El entorno de ejecución relaciona cada nombre con su valor actual.
// Cada bloque abre un ámbito nuevo; el último de la pila es el actual.
pub struct Interpreter<W: Write> {
    output: W,
    scopes: Vec<HashMap<String, Binding>>,
    functions: HashMap<String, Rc<FunctionDef>>,
    structures: HashMap<String, Rc<Vec<FieldDef>>>,
    enums: HashSet<String>,
    call_bases: Vec<usize>,
    function_depth: usize,
    construction_depth: usize,
}

// Señal de control que atraviesa bloques: los bucles consumen 'break' y
// 'continue', y un 'return' viaja hasta la llamada más cercana cerrando los
// ámbitos por el camino. `Return(None)` corresponde a `return;` sin valor.
enum Control {
    None,
    Break,
    Continue,
    Return(Option<Value>),
}

impl<W: Write> Interpreter<W> {
    pub fn new(output: W) -> Self {
        Self {
            output,
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            structures: HashMap::new(),
            enums: HashSet::new(),
            call_bases: Vec::new(),
            function_depth: 0,
            construction_depth: 0,
        }
    }

    // run() solo entrega programas que han superado la comprobación de tipos.
    pub fn interpret(&mut self, statements: &[Stmt]) -> Result<(), Box<dyn Error>> {
        for statement in statements {
            self.execute(statement)?;
        }
        Ok(())
    }

    fn execute(&mut self, statement: &Stmt) -> Result<Control, Box<dyn Error>> {
        // Las llamadas recursivas atraviesan un despacho pequeño: las variables
        // temporales de otras instrucciones no ocupan pila en cada llamada.
        match statement {
            Stmt::Match { value, arms, .. } => self.execute_match(value, arms),
            Stmt::Return { value, .. } => Ok(Control::Return(
                value
                    .as_ref()
                    .map(|value| self.evaluate(value))
                    .transpose()?,
            )),
            Stmt::Call(expression) => {
                self.evaluate_call(expression)?;
                Ok(Control::None)
            }
            _ => self.execute_other(statement),
        }
    }

    fn execute_other(&mut self, statement: &Stmt) -> Result<Control, Box<dyn Error>> {
        match statement {
            Stmt::Enum { name, .. } => {
                self.enums.insert(name.text.clone());
            }
            Stmt::Match { .. } | Stmt::Return { .. } | Stmt::Call(_) => {
                unreachable!("despachado por execute")
            }
            Stmt::Struct { name, fields } => {
                self.structures
                    .insert(name.text.clone(), Rc::new(fields.clone()));
            }
            Stmt::Function {
                name,
                parameters,
                body,
                ..
            } => {
                self.functions.insert(
                    name.text.clone(),
                    Rc::new(FunctionDef {
                        parameters: parameters
                            .iter()
                            .map(|parameter| parameter.name.clone())
                            .collect(),
                        body: body.clone(),
                    }),
                );
            }
            Stmt::Import { .. } => {}
            Stmt::Declare {
                name, initializer, ..
            } => {
                let value = self.evaluate(initializer)?;
                self.scopes
                    .last_mut()
                    .expect("ámbito abierto")
                    .insert(name.text.clone(), Binding::Owned(value));
            }
            Stmt::Assign { name, steps, value } => {
                let (scope, positions) = self.resolve_target(name, steps)?;
                let value = self.evaluate(value)?;
                self.write_target(scope, name, &positions, value)?;
            }
            Stmt::CompoundAssign {
                name,
                steps,
                operator,
                value,
                line,
            } => {
                // Equivale a `x = x <op> v`: se lee el destino, se evalúa el
                // valor nuevo y se escribe solo si la operación tiene éxito.
                let (scope, positions) = self.resolve_target(name, steps)?;
                let current = self.target_value(scope, name, &positions)?;
                let operand = self.evaluate(value)?;
                let result = Self::binary(current, operator.binary(), operand, *line)?;
                self.write_target(scope, name, &positions, result)?;
            }
            Stmt::Increment {
                name,
                steps,
                operator,
                line,
            } => {
                let (scope, positions) = self.resolve_target(name, steps)?;
                let current = self.target_value(scope, name, &positions)?;
                // El paso conserva el tipo del destino: 1 para int, 1.0 para float.
                let step = match current {
                    Value::Int(_) => Value::Int(1),
                    Value::Float(_) => Value::Float(1.0),
                    _ => unreachable!("tipo validado"),
                };
                let result = Self::binary(current, operator.binary(), step, *line)?;
                self.write_target(scope, name, &positions, result)?;
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
                    return self.execute_block(then_branch);
                } else if let Some(else_branch) = else_branch {
                    return self.execute_block(else_branch);
                }
            }
            Stmt::While {
                condition, body, ..
            } => loop {
                if !self.evaluate_bool(condition, "while")? {
                    break;
                }
                match self.execute_block(body)? {
                    Control::Break => break,
                    Control::Return(value) => return Ok(Control::Return(value)),
                    Control::Continue | Control::None => {}
                }
            },
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
                return result;
            }
            Stmt::Foreach {
                name,
                iterable,
                body,
                ..
            } => {
                let value = self.evaluate(iterable)?;
                let Value::Array { elements, .. } = value else {
                    return Err("'foreach' solo recorre arrays.".into());
                };
                self.scopes.push(HashMap::new());
                let result = self.execute_foreach(name, elements, body);
                self.scopes.pop();
                return result;
            }
            Stmt::Print(expression) => {
                let value = self.evaluate(expression)?;
                write!(self.output, "{value}")?;
            }
            Stmt::Println(expression) => {
                let value = self.evaluate(expression)?;
                writeln!(self.output, "{value}")?;
            }
            Stmt::Break { .. } => return Ok(Control::Break),
            Stmt::Continue { .. } => return Ok(Control::Continue),
        }
        Ok(Control::None)
    }

    fn execute_match(
        &mut self,
        expression: &Expr,
        arms: &[MatchArm],
    ) -> Result<Control, Box<dyn Error>> {
        let Value::Enum {
            variant, values, ..
        } = self.evaluate(expression)?
        else {
            unreachable!("enum comprobado")
        };
        let arm = arms
            .iter()
            .find(|arm| arm.variant.text == variant)
            .expect("match exhaustivo");
        self.scopes.push(HashMap::new());
        for (binding, value) in arm.bindings.iter().flatten().zip(values) {
            self.scopes
                .last_mut()
                .expect("ámbito abierto")
                .insert(binding.text.clone(), Binding::Owned(value));
        }
        // Capturas y declaraciones comparten ámbito; se cierra ante errores y saltos.
        let result = self.execute_statements(&arm.body);
        self.scopes.pop();
        result
    }

    fn execute_block(&mut self, statements: &[Stmt]) -> Result<Control, Box<dyn Error>> {
        self.scopes.push(HashMap::new());
        let result = self.execute_statements(statements);
        self.scopes.pop();
        result
    }

    fn execute_statements(&mut self, statements: &[Stmt]) -> Result<Control, Box<dyn Error>> {
        for statement in statements {
            let control = self.execute(statement)?;
            if !matches!(control, Control::None) {
                return Ok(control);
            }
        }
        Ok(Control::None)
    }

    fn evaluate_bool(&mut self, condition: &Expr, keyword: &str) -> Result<bool, String> {
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
    ) -> Result<Control, Box<dyn Error>> {
        self.execute(initializer)?;
        // La condición se comprueba antes de cada vuelta; la actualización
        // ocurre después del cuerpo, como en el 'for' de C.
        while self.evaluate_bool(condition, "for")? {
            match self.execute_block(body)? {
                Control::Break => break,
                Control::Return(value) => return Ok(Control::Return(value)),
                Control::Continue | Control::None => self.execute(update)?,
            };
        }
        Ok(Control::None)
    }

    fn execute_foreach(
        &mut self,
        name: &Name,
        elements: Vec<Value>,
        body: &[Stmt],
    ) -> Result<Control, Box<dyn Error>> {
        for element in elements {
            // Cada vuelta reinicia la variable con una copia del elemento.
            self.scopes
                .last_mut()
                .expect("ámbito abierto")
                .insert(name.text.clone(), Binding::Owned(element));
            match self.execute_block(body)? {
                Control::Break => break,
                Control::Return(value) => return Ok(Control::Return(value)),
                Control::Continue | Control::None => {}
            }
        }
        Ok(Control::None)
    }

    fn scope_containing(&self, name: &str) -> Option<usize> {
        // Una función ve sus propios ámbitos y el global, nunca los locales
        // del llamador. Los alias inout proporcionan el acceso explícito.
        let base = self.call_bases.last().copied().unwrap_or(0);
        (base..self.scopes.len())
            .rev()
            .find(|&index| self.scopes[index].contains_key(name))
            .or_else(|| self.scopes[0].contains_key(name).then_some(0))
    }

    fn storage_location(&self, mut scope: usize, name: &str) -> (usize, String) {
        let mut name = name.to_string();
        while let Binding::Alias {
            scope: target_scope,
            name: target_name,
        } = &self.scopes[scope][&name]
        {
            scope = *target_scope;
            name = target_name.clone();
        }
        (scope, name)
    }

    // Evalúa cada índice una vez y consulta el destino después de sus efectos.
    fn resolve_target(
        &mut self,
        name: &Name,
        steps: &[TargetStep],
    ) -> Result<(usize, Vec<TargetPosition>), String> {
        let scope = self.scope_containing(&name.text).ok_or_else(|| {
            name.error(&format!("La variable '{}' no está declarada.", name.text))
        })?;
        let mut positions = Vec::new();
        for step in steps {
            match step {
                TargetStep::Index(index, line) => {
                    let index = self.evaluate(index)?;
                    let target = self.target_value(scope, name, &positions)?;
                    let (_, position) = Self::array_position(&target, index, *line)?;
                    let Value::Array { element_type, .. } = target else {
                        unreachable!("array comprobado")
                    };
                    positions.push(TargetPosition::Index {
                        index: position,
                        element_type,
                    });
                }
                TargetStep::Field(field) => {
                    let Value::Struct {
                        name: structure, ..
                    } = self.target_value(scope, name, &positions)?
                    else {
                        return Err(name.error("El destino cambió de tipo durante la evaluación."));
                    };
                    positions.push(TargetPosition::Field {
                        name: field.text.clone(),
                        structure,
                    });
                    self.target_value(scope, name, &positions)?;
                }
            }
        }
        Ok((scope, positions))
    }

    // Copia del valor actual del destino ya validado.
    fn target_value(
        &self,
        scope: usize,
        name: &Name,
        positions: &[TargetPosition],
    ) -> Result<Value, String> {
        let (scope, storage_name) = self.storage_location(scope, &name.text);
        let Binding::Owned(value) = &self.scopes[scope][&storage_name] else {
            unreachable!("alias resuelto")
        };
        let mut target = value;
        for position in positions {
            target = match (target, position) {
                (
                    Value::Array {
                        elements,
                        element_type,
                    },
                    TargetPosition::Index {
                        index,
                        element_type: expected,
                    },
                ) => {
                    if element_type != expected {
                        return Err(name.error("El destino cambió de tipo durante la evaluación."));
                    }
                    elements.get(*index)
                }
                (
                    Value::Struct {
                        name: structure,
                        fields,
                    },
                    TargetPosition::Field {
                        name: field,
                        structure: expected,
                    },
                ) => {
                    if structure != expected {
                        return Err(name.error("El destino cambió de tipo durante la evaluación."));
                    }
                    fields
                        .iter()
                        .find(|(name, _)| name == field)
                        .map(|(_, value)| value)
                }
                _ => None,
            }
            .ok_or_else(|| name.error("El destino quedó fuera de rango durante la evaluación."))?;
        }
        Ok(target.clone())
    }

    // Sustituye el valor del destino ya validado.
    fn write_target(
        &mut self,
        scope: usize,
        name: &Name,
        positions: &[TargetPosition],
        value: Value,
    ) -> Result<(), String> {
        value.check_depth(positions.len(), name.line)?;
        *self.target_mut(scope, name, positions)? = value;
        Ok(())
    }

    // Un pop en otro argumento puede haber acortado el destino ya resuelto.
    fn target_mut(
        &mut self,
        scope: usize,
        name: &Name,
        positions: &[TargetPosition],
    ) -> Result<&mut Value, String> {
        let (scope, storage_name) = self.storage_location(scope, &name.text);
        let Binding::Owned(mut_target) = self.scopes[scope]
            .get_mut(&storage_name)
            .expect("nombre validado")
        else {
            unreachable!("alias resuelto")
        };
        let mut target = mut_target;
        for position in positions {
            target = match (target, position) {
                (
                    Value::Array {
                        elements,
                        element_type,
                    },
                    TargetPosition::Index {
                        index,
                        element_type: expected,
                    },
                ) => {
                    if element_type != expected {
                        return Err(name.error("El destino cambió de tipo durante la evaluación."));
                    }
                    elements.get_mut(*index)
                }
                (
                    Value::Struct {
                        name: structure,
                        fields,
                    },
                    TargetPosition::Field {
                        name: field,
                        structure: expected,
                    },
                ) => {
                    if structure != expected {
                        return Err(name.error("El destino cambió de tipo durante la evaluación."));
                    }
                    fields
                        .iter_mut()
                        .find(|(name, _)| name == field)
                        .map(|(_, value)| value)
                }
                _ => None,
            }
            .ok_or_else(|| name.error("El destino quedó fuera de rango durante la evaluación."))?;
        }
        Ok(target)
    }

    fn evaluate_call(&mut self, expression: &Expr) -> Result<Option<Value>, String> {
        if let Expr::Qualified { path, arguments } = expression {
            if self.enums.contains(&path[0].text) {
                return self.evaluate(expression).map(Some);
            }
            return self.evaluate_library_call(
                path,
                None,
                arguments.as_deref().expect("llamada de biblioteca"),
            );
        }
        if matches!(expression, Expr::Cast { .. }) {
            return self.evaluate(expression).map(Some);
        }
        if let Expr::Call { name, arguments } = expression {
            return self.call_user_function(name, arguments);
        }
        let Expr::LibraryCall {
            path,
            receiver,
            arguments,
        } = expression
        else {
            unreachable!("instrucción de llamada validada")
        };
        self.evaluate_library_call(path, receiver.as_deref(), arguments)
    }

    fn evaluate_library_call(
        &mut self,
        path: &[Name],
        receiver: Option<&Expr>,
        arguments: &[Expr],
    ) -> Result<Option<Value>, String> {
        array::resolve(path, receiver.is_some())?;
        let name = path.last().expect("ruta con nombre de método");
        array::check_arity(arguments.len(), receiver.is_some(), name)?;
        let array = receiver.unwrap_or_else(|| &arguments[0]);
        if !array::mutates(name) {
            return array::evaluate(&mut self.evaluate(array)?, None, name);
        }
        let (target, steps) = array.clone().into_target().expect("destino comprobado");
        let (scope, positions) = self.resolve_target(&target, &steps)?;
        let expected = self.target_value(scope, &target, &positions)?.value_type();
        let value = if array::takes_value(name) {
            Some(self.evaluate(arguments.last().expect("argumento validado"))?)
        } else {
            None
        };
        if let Some(value) = &value {
            value.check_depth(positions.len() + 1, name.line)?;
        }
        let array = self.target_mut(scope, &target, &positions)?;
        if array.value_type() != expected {
            return Err(target.error("El destino cambió de tipo durante la evaluación."));
        }
        array::evaluate(array, value, name)
    }

    // Llama a una función propia. Los argumentos se evalúan de izquierda a
    // derecha y los parámetros viven en un ámbito nuevo que se cierra al salir.
    fn call_user_function(
        &mut self,
        name: &Name,
        arguments: &[CallArgument],
    ) -> Result<Option<Value>, String> {
        let function =
            self.functions.get(&name.text).cloned().ok_or_else(|| {
                name.error(&format!("La función '{}' no está declarada.", name.text))
            })?;
        let mut values = Vec::with_capacity(arguments.len());
        for argument in arguments {
            values.push(match argument {
                CallArgument::Value(value) => Binding::Owned(self.evaluate(value)?),
                CallArgument::InOut(target) => {
                    let scope = self
                        .scope_containing(&target.text)
                        .expect("nombre comprobado");
                    let (scope, name) = self.storage_location(scope, &target.text);
                    Binding::Alias { scope, name }
                }
            });
        }
        if self.function_depth + self.construction_depth >= MAX_CALL_DEPTH {
            return Err(name.error(&format!(
                "Se superó la profundidad máxima de llamadas ({MAX_CALL_DEPTH})."
            )));
        }
        self.function_depth += 1;
        self.call_bases.push(self.scopes.len());
        self.scopes.push(HashMap::new());
        for (parameter, value) in function.parameters.iter().zip(values) {
            self.scopes
                .last_mut()
                .expect("ámbito abierto")
                .insert(parameter.text.clone(), value);
        }
        // El cuerpo usa la ruta de errores general; se conserva el mensaje y se
        // devuelve a la ruta de evaluación, que trabaja con `String`.
        let result = self
            .execute_block(&function.body)
            .map_err(|error| error.to_string());
        self.scopes.pop();
        self.call_bases.pop();
        self.function_depth -= 1;
        match result? {
            Control::Return(value) => Ok(value),
            Control::None => Ok(None),
            Control::Break | Control::Continue => {
                unreachable!("el comprobador rechaza saltos fuera de un bucle")
            }
        }
    }

    fn construct(&mut self, name: &Name, initializers: &[(Name, Expr)]) -> Result<Value, String> {
        if self.function_depth + self.construction_depth >= MAX_CALL_DEPTH {
            return Err(name.error(&format!(
                "Se superó la profundidad máxima de construcciones y llamadas ({MAX_CALL_DEPTH})."
            )));
        }
        let definition = self.structures[&name.text].clone();
        self.construction_depth += 1;
        let result = (|| {
            let mut supplied = HashMap::new();
            // Los campos explícitos pertenecen al contexto del llamador.
            for (field, value) in initializers {
                supplied.insert(field.text.clone(), self.evaluate(value)?);
            }
            // Los valores por defecto solo ven los campos anteriores y las
            // globales; nunca heredan nombres locales de quien construye.
            self.call_bases.push(self.scopes.len());
            self.scopes.push(HashMap::new());
            let result = (|| {
                let mut fields = Vec::new();
                for field in definition.iter() {
                    let value = match supplied.remove(&field.name.text) {
                        Some(value) => value,
                        None => self.evaluate(
                            field
                                .default_value
                                .as_ref()
                                .expect("campo con valor por defecto"),
                        )?,
                    };
                    self.scopes
                        .last_mut()
                        .expect("ámbito abierto")
                        .insert(field.name.text.clone(), Binding::Owned(value.clone()));
                    fields.push((field.name.text.clone(), value));
                }
                Ok(Value::Struct {
                    name: name.text.clone(),
                    fields,
                })
            })();
            self.scopes.pop();
            self.call_bases.pop();
            result
        })();
        self.construction_depth -= 1;
        result.and_then(|value| {
            value.check_depth(0, name.line)?;
            Ok(value)
        })
    }

    fn evaluate_qualified(
        &mut self,
        path: &[Name],
        arguments: &Option<Vec<Expr>>,
    ) -> Result<Value, String> {
        if !self.enums.contains(&path[0].text) {
            return self
                .evaluate_library_call(
                    path,
                    None,
                    arguments.as_deref().expect("llamada de biblioteca"),
                )?
                .ok_or_else(|| path[1].error("'push' no devuelve un valor."));
        }
        let values = arguments
            .iter()
            .flatten()
            .map(|value| self.evaluate(value))
            .collect::<Result<Vec<_>, _>>()?;
        let value = Value::Enum {
            name: path[0].text.clone(),
            variant: path[1].text.clone(),
            values,
        };
        value.check_depth(0, path[1].line)?;
        Ok(value)
    }

    fn evaluate(&mut self, expression: &Expr) -> Result<Value, String> {
        match expression {
            Expr::Qualified { path, arguments } => self.evaluate_qualified(path, arguments),
            Expr::Call { name, arguments } => {
                self.call_user_function(name, arguments)?.ok_or_else(|| {
                    name.error(&format!("La función '{}' no devuelve un valor.", name.text))
                })
            }
            _ => self.evaluate_other(expression),
        }
    }

    fn evaluate_other(&mut self, expression: &Expr) -> Result<Value, String> {
        match expression {
            Expr::Qualified { .. } | Expr::Call { .. } => unreachable!("despachado por evaluate"),
            Expr::Struct { name, fields } => self.construct(name, fields),
            Expr::Field { object, name } => {
                let Value::Struct { fields, .. } = self.evaluate(object)? else {
                    unreachable!("estructura comprobada")
                };
                Ok(fields
                    .into_iter()
                    .find(|(field, _)| *field == name.text)
                    .expect("campo comprobado")
                    .1)
            }
            Expr::TypeCheck {
                name,
                fields,
                target,
                negated,
            } => {
                let scope = self
                    .scope_containing(&name.text)
                    .expect("variable comprobada");
                let mut value = self.target_value(scope, name, &[])?;
                for field in fields {
                    let Value::Struct { fields, .. } = value else {
                        unreachable!("campo comprobado")
                    };
                    value = fields
                        .into_iter()
                        .find(|(name, _)| name == &field.text)
                        .expect("campo comprobado")
                        .1;
                }
                Ok(Value::Bool((value.value_type() == *target) != *negated))
            }
            Expr::Cast {
                path,
                target,
                value,
            } => {
                let value = self.evaluate(value)?;
                casting::evaluate(value, target, path.last().expect("ruta de conversión"))
            }
            Expr::LibraryCall { path, .. } => self.evaluate_call(expression)?.ok_or_else(|| {
                path.last()
                    .expect("ruta con nombre de método")
                    .error("'push' no devuelve un valor.")
            }),
            Expr::Literal(value) => Ok(value.clone()),
            Expr::Variable(name) => {
                let scope = self.scope_containing(&name.text).ok_or_else(|| {
                    name.error(&format!("La variable '{}' no está declarada.", name.text))
                })?;
                self.target_value(scope, name, &[])
            }
            Expr::Array {
                elements,
                element_type,
                line,
            } => {
                let value = Value::Array {
                    elements: elements
                        .iter()
                        .map(|element| self.evaluate(element))
                        .collect::<Result<Vec<_>, _>>()?,
                    element_type: element_type.borrow().clone().expect("array comprobado"),
                };
                value.check_depth(0, *line)?;
                Ok(value)
            }
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
        let (Value::Array { elements, .. }, Value::Int(index)) = (array, index) else {
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
