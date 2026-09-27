use std::collections::{HashMap, HashSet};

use crate::{
    parser::{BinaryOp, CallArgument, Expr, Name, Stmt, UnaryOp},
    stdlib::{ArrayFunction, StandardLibrary},
    value::Type,
};

#[derive(Clone)]
struct VariableInfo {
    narrowed_type: Option<Type>,
    declared_type: Type,
    is_constant: bool,
}

// Una firma guarda lo necesario para comprobar las llamadas: los tipos de los
// parámetros y, si la función devuelve un valor, el tipo de retorno.
#[derive(Clone)]
struct FunctionSignature {
    parameters: Vec<(Type, bool)>,
    return_type: Option<Type>,
}

// Este entorno guarda tipos y si el nombre es constante, pero no valores.
// Cada bloque abre un ámbito nuevo; el último de la pila es el actual.
#[derive(Clone, Default)]
pub struct TypeChecker {
    scopes: Vec<HashMap<String, VariableInfo>>,
    functions: HashMap<String, FunctionSignature>,
    // Tipo esperado en un `return`. La pila distingue "fuera de función" (vacía)
    // de una función sin valor (`Some(None)`) o con valor (`Some(Some(tipo))`).
    return_types: Vec<Option<Type>>,
    library: StandardLibrary,
    loop_depth: usize,
}

impl TypeChecker {
    pub fn check(&mut self, statements: &[Stmt]) -> Result<(), String> {
        self.library = StandardLibrary::default();
        self.functions.clear();
        self.return_types.clear();
        self.loop_depth = 0;
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
            Stmt::Function {
                name,
                parameters,
                return_type,
                body,
                line,
            } => {
                // Solo en el ámbito global, como 'import' y 'use'.
                if self.scopes.len() != 1 {
                    return Err(format!(
                        "Línea {line}: las funciones solo se permiten en el ámbito global del archivo."
                    ));
                }
                if self.functions.contains_key(&name.text) {
                    return Err(
                        name.error(&format!("La función '{}' ya está declarada.", name.text))
                    );
                }
                if self.scopes[0].contains_key(&name.text) {
                    return Err(name.error(&format!(
                        "El nombre '{}' ya está declarado como variable.",
                        name.text
                    )));
                }
                let mut seen = HashSet::new();
                for parameter in parameters {
                    let parameter = &parameter.name;
                    if !seen.insert(parameter.text.clone()) {
                        return Err(parameter
                            .error(&format!("El parámetro '{}' está repetido.", parameter.text)));
                    }
                }
                self.functions.insert(
                    name.text.clone(),
                    FunctionSignature {
                        parameters: parameters
                            .iter()
                            .map(|p| (p.declared_type.clone(), p.is_inout))
                            .collect(),
                        return_type: return_type.clone(),
                    },
                );
                // Los parámetros viven en el ámbito propio de la llamada; el
                // cuerpo solo puede escribir en locales y parámetros.
                self.scopes.push(HashMap::new());
                for parameter in parameters {
                    self.scopes.last_mut().expect("ámbito abierto").insert(
                        parameter.name.text.clone(),
                        VariableInfo {
                            narrowed_type: None,
                            declared_type: parameter.declared_type.clone(),
                            is_constant: false,
                        },
                    );
                }
                self.return_types.push(return_type.clone());
                let result = self.check_statements(body).and_then(|()| {
                    // Análisis conservador de rutas: una función con valor debe
                    // garantizar un `return` en todos los caminos.
                    if return_type.is_some() && !Self::always_returns(body) {
                        Err(name.error(&format!(
                            "La función '{}' debe devolver un valor en todos los caminos de ejecución.",
                            name.text
                        )))
                    } else {
                        Ok(())
                    }
                });
                self.return_types.pop();
                self.scopes.pop();
                result?;
            }
            Stmt::Return { value, line } => {
                let Some(expected) = self.return_types.last() else {
                    return Err(format!(
                        "Línea {line}: 'return' solo se permite dentro de una función."
                    ));
                };
                match (expected, value) {
                    (Some(expected), Some(value)) => {
                        let actual = self.expression_type_expected(value, Some(expected))?;
                        if !expected.accepts(&actual) {
                            return Err(format!(
                                "Línea {line}: el retorno debe ser {expected}; se recibió {actual}. No hay conversiones implícitas."
                            ));
                        }
                    }
                    (Some(expected), None) => {
                        return Err(format!(
                            "Línea {line}: esta función debe devolver un valor de tipo {expected}."
                        ));
                    }
                    (None, Some(_)) => {
                        return Err(format!(
                            "Línea {line}: esta función no devuelve un valor; usa 'return;' sin expresión."
                        ));
                    }
                    (None, None) => {}
                }
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
                if self.scopes.len() == 1 && self.functions.contains_key(&name.text) {
                    return Err(name.error(&format!(
                        "El nombre '{}' ya está declarado como función.",
                        name.text
                    )));
                }
                let actual = self.expression_type_expected(initializer, Some(declared_type))?;
                Self::require_type(name, declared_type, &actual)?;
                // Registrar después del inicializador impide int x = x;.
                self.scopes.last_mut().expect("ámbito abierto").insert(
                    name.text.clone(),
                    VariableInfo {
                        narrowed_type: None,
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
                let checked_target = self.assignment_target(name, indices)?;
                let target_type = if indices.is_empty() {
                    self.lookup(name)?.declared_type.clone()
                } else {
                    checked_target
                };
                let actual = self.expression_type_expected(value, Some(&target_type))?;
                Self::require_type(name, &target_type, &actual)?;
                if indices.is_empty() {
                    self.forget_name(&name.text);
                }
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
                let mut yes = self.with_condition(condition, true);
                yes.check_block(then_branch)?;
                let mut no = self.with_condition(condition, false);
                if let Some(else_branch) = else_branch {
                    no.check_block(else_branch)?;
                }
                self.merge_scopes(&yes, &no);
            }
            Stmt::While {
                condition,
                body,
                line,
            } => {
                self.forget_loop_writes(body);
                self.require_bool(condition, "while", *line)?;
                let mut iteration = self.with_condition(condition, true);
                iteration.loop_depth += 1;
                iteration.check_block(body)?;
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
                self.loop_depth += 1;
                let result = self.check_for(initializer, condition, update, body, *line);
                self.loop_depth -= 1;
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
                        narrowed_type: None,
                        declared_type: declared_type.clone(),
                        is_constant: false,
                    },
                );
                self.forget_loop_writes(body);
                self.loop_depth += 1;
                let result = self.check_block(body);
                self.loop_depth -= 1;
                self.scopes.pop();
                result?;
            }
            Stmt::Break { line } | Stmt::Continue { line } => {
                if self.loop_depth == 0 {
                    let keyword = if matches!(statement, Stmt::Break { .. }) {
                        "break"
                    } else {
                        "continue"
                    };
                    return Err(format!(
                        "Línea {line}: '{keyword}' solo se permite dentro de un bucle."
                    ));
                }
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
        self.forget_loop_writes(body);
        self.forget_statement_writes(update);
        self.require_bool(condition, "for", line)?;
        let mut iteration = self.with_condition(condition, true);
        iteration.check_block(body)?;
        iteration.check_statement(update)
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
            Expr::TypeCheck { name, .. } => {
                self.lookup(name)?;
                Ok(Type::Bool)
            }
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
            Expr::Call { name, .. } => self.check_call(expression)?.ok_or_else(|| {
                name.error(&format!(
                    "La función '{}' no devuelve un valor; úsala como instrucción con ';'.",
                    name.text
                ))
            }),
            Expr::Literal(value) => Ok(value.value_type()),
            Expr::Variable(name) => self.variable_type(name),
            Expr::Array {
                elements,
                line,
                element_type: resolved_element,
            } => {
                // Un literal vacío puede necesitar una alternativa array como
                // contexto. Si encaja en varias, no elegimos una arbitrariamente.
                if let Some(expected @ Type::Union(types)) = expected {
                    let mut candidates = Vec::new();
                    for kind in types.iter().filter(|kind| matches!(kind, Type::Array(_))) {
                        if let Ok(actual) = self.expression_type_expected(expression, Some(kind)) {
                            candidates.push(actual);
                        }
                    }
                    match candidates.len() {
                        0 => {}
                        1 => {
                            // El último intento pudo usar otro contexto en arrays interiores.
                            return self.expression_type_expected(expression, Some(&candidates[0]));
                        }
                        _ => {
                            return Err(format!(
                                "Línea {line}: el array es ambiguo entre los tipos {expected}; usa una variable array con tipo explícito."
                            ));
                        }
                    }
                }
                let element_type = match expected {
                    Some(Type::Array(element)) => element.as_ref().clone(),
                    _ => {
                        let first = elements.first().ok_or_else(|| format!(
                            "Línea {line}: un array vacío necesita un tipo declarado, por ejemplo int[] datos = [];"
                        ))?;
                        self.expression_type(first)?
                    }
                };
                if matches!(element_type, Type::Union(_)) {
                    return Err(format!(
                        "Línea {line}: un elemento de array no puede tener tipo unión {element_type}; el array debe tener un tipo de elemento concreto."
                    ));
                }
                for element in elements {
                    let actual = self.expression_type_expected(element, Some(&element_type))?;
                    if actual != element_type {
                        return Err(format!(
                            "Línea {line}: elemento de array incompatible: se esperaba {element_type}, se recibió {actual}. No hay conversiones implícitas."
                        ));
                    }
                }
                *resolved_element.borrow_mut() = Some(element_type.clone());
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
                let left_type = self.expression_type(left)?;
                let right_type = match operator {
                    BinaryOp::And => self.with_condition(left, true).expression_type(right)?,
                    BinaryOp::Or => self.with_condition(left, false).expression_type(right)?,
                    _ => self.expression_type(right)?,
                };
                let (left, right) = (left_type, right_type);
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
        if let Expr::Call { name, arguments } = expression {
            let signature = self.functions.get(&name.text).ok_or_else(|| {
                name.error(&format!(
                    "Llamada desconocida '{}': no está declarada como función.",
                    name.text
                ))
            })?;
            if arguments.len() != signature.parameters.len() {
                return Err(name.error(&format!(
                    "La función '{}' esperaba {} argumentos entre paréntesis; recibió {}.",
                    name.text,
                    signature.parameters.len(),
                    arguments.len()
                )));
            }
            for (argument, (expected, is_inout)) in arguments.iter().zip(&signature.parameters) {
                let actual = match (argument, is_inout) {
                    (CallArgument::Value(value), false) => self.expression_type_expected(value, Some(expected))?,
                    (CallArgument::InOut(target), true) => {
                        self.assignment_target(target, &[])?;
                        self.lookup(target)?.declared_type.clone()
                    },
                    (CallArgument::Value(_), true) => return Err(name.error(
                        "Falta 'inout' en la llamada para un parámetro declarado inout."
                    )),
                    (CallArgument::InOut(target), false) => return Err(target.error(
                        "Este parámetro no está declarado inout; no se admite 'inout' en la llamada."
                    )),
                };
                if &actual != expected {
                    return Err(name.error(&format!(
                        "Argumento incompatible para '{}': se esperaba {expected}, se recibió {actual}. No hay conversiones implícitas.",
                        name.text
                    )));
                }
            }
            return Ok(signature.return_type.clone());
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
        if matches!(left, Type::Union(_)) || matches!(right, Type::Union(_)) {
            return None;
        }
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
        if !self.return_types.is_empty()
            && !self
                .scopes
                .iter()
                .skip(1)
                .any(|scope| scope.contains_key(&name.text))
        {
            return Err(name.error(&format!(
                "No se puede modificar la variable global '{}' desde una función; recíbela mediante un parámetro inout.",
                name.text
            )));
        }
        let mut target_type = self.variable_type(name)?;
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

    fn variable_type(&self, name: &Name) -> Result<Type, String> {
        let info = self.lookup(name)?;
        Ok(info
            .narrowed_type
            .as_ref()
            .unwrap_or(&info.declared_type)
            .clone())
    }

    fn forget_name(&mut self, name: &str) {
        if let Some(info) = self
            .scopes
            .iter_mut()
            .rev()
            .find_map(|scope| scope.get_mut(name))
        {
            info.narrowed_type = None;
        }
    }

    // Una vuelta posterior puede encontrar otro tipo. Antes de comprobar un
    // bucle se descartan las conclusiones sobre variables que este reasigna.
    fn forget_loop_writes(&mut self, statements: &[Stmt]) {
        for statement in statements {
            self.forget_statement_writes(statement);
        }
    }

    fn forget_statement_writes(&mut self, statement: &Stmt) {
        match statement {
            Stmt::Assign { name, indices, .. } if indices.is_empty() => {
                self.forget_name(&name.text)
            }
            Stmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                self.forget_loop_writes(then_branch);
                if let Some(branch) = else_branch {
                    self.forget_loop_writes(branch);
                }
            }
            Stmt::While { body, .. } | Stmt::Foreach { body, .. } => self.forget_loop_writes(body),
            Stmt::For {
                initializer,
                update,
                body,
                ..
            } => {
                self.forget_statement_writes(initializer);
                self.forget_loop_writes(body);
                self.forget_statement_writes(update);
            }
            _ => {}
        }
    }

    fn with_condition(&self, condition: &Expr, truth: bool) -> Self {
        let mut context = self.clone();
        context.assume_condition(condition, truth);
        context
    }

    fn assume_condition(&mut self, condition: &Expr, truth: bool) {
        match condition {
            Expr::TypeCheck {
                name,
                target,
                negated,
            } => {
                let Some(info) = self
                    .scopes
                    .iter_mut()
                    .rev()
                    .find_map(|scope| scope.get_mut(&name.text))
                else {
                    return;
                };
                let current = info.narrowed_type.as_ref().unwrap_or(&info.declared_type);
                let alternatives = match current {
                    Type::Union(types) => types.clone(),
                    kind => vec![kind.clone()],
                };
                let mut remaining: Vec<_> = alternatives
                    .into_iter()
                    .filter(|kind| (kind == target) == (truth != *negated))
                    .collect();
                // Una rama imposible sigue comprobándose sin inventar un tipo.
                info.narrowed_type = match remaining.len() {
                    0 => info.narrowed_type.clone(),
                    1 => Some(remaining.remove(0)),
                    _ => Some(Type::Union(remaining)),
                };
            }
            Expr::Unary {
                operator: UnaryOp::Not,
                operand,
                ..
            } => self.assume_condition(operand, !truth),
            Expr::Binary {
                left,
                operator: BinaryOp::And,
                right,
                ..
            } if truth => {
                self.assume_condition(left, true);
                self.assume_condition(right, true);
            }
            Expr::Binary {
                left,
                operator: BinaryOp::Or,
                right,
                ..
            } if !truth => {
                self.assume_condition(left, false);
                self.assume_condition(right, false);
            }
            Expr::Binary {
                left,
                operator: BinaryOp::And,
                right,
                ..
            } => {
                let no_left = self.with_condition(left, false);
                let no_right = self.with_condition(left, true).with_condition(right, false);
                self.merge_scopes(&no_left, &no_right);
            }
            Expr::Binary {
                left,
                operator: BinaryOp::Or,
                right,
                ..
            } => {
                let yes_left = self.with_condition(left, true);
                let yes_right = self.with_condition(left, false).with_condition(right, true);
                self.merge_scopes(&yes_left, &yes_right);
            }
            _ => {}
        }
    }

    // Al reunir caminos, se conservan todos los tipos posibles de ambos.
    fn merge_scopes(&mut self, left: &Self, right: &Self) {
        for (index, scope) in self.scopes.iter_mut().enumerate() {
            for (name, info) in scope {
                let a = &left.scopes[index][name];
                let b = &right.scopes[index][name];
                let a = a.narrowed_type.as_ref().unwrap_or(&a.declared_type);
                let b = b.narrowed_type.as_ref().unwrap_or(&b.declared_type);
                let alternatives = match &info.declared_type {
                    Type::Union(types) => types.clone(),
                    kind => vec![kind.clone()],
                };
                let mut possible: Vec<_> = alternatives
                    .into_iter()
                    .filter(|kind| a.accepts(kind) || b.accepts(kind))
                    .collect();
                info.narrowed_type = match possible.len() {
                    0 => None,
                    1 => Some(possible.remove(0)),
                    _ => Some(Type::Union(possible)),
                };
            }
        }
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

    // ¿Toda ruta del bloque termina en un `return` con valor? Es un análisis
    // conservador: un bucle no cuenta como garantía aunque sea infinito.
    fn always_returns(statements: &[Stmt]) -> bool {
        statements.iter().any(Self::statement_always_returns)
    }

    fn statement_always_returns(statement: &Stmt) -> bool {
        match statement {
            Stmt::Return { value: Some(_), .. } => true,
            Stmt::If {
                then_branch,
                else_branch: Some(else_branch),
                ..
            } => Self::always_returns(then_branch) && Self::always_returns(else_branch),
            _ => false,
        }
    }

    fn require_type(name: &Name, expected: &Type, actual: &Type) -> Result<(), String> {
        if !expected.accepts(actual) {
            return Err(name.error(&format!(
                "Tipo incompatible para '{}': se esperaba {expected}, se recibió {actual}. No hay conversiones implícitas.", name.text
            )));
        }
        Ok(())
    }
}
