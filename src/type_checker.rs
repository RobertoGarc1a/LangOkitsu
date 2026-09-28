use std::collections::{HashMap, HashSet};

use crate::{
    parser::{BinaryOp, CallArgument, Expr, FieldDef, Name, Stmt, TargetStep, UnaryOp, VariantDef},
    stdlib::{ArrayFunction, StandardLibrary},
    value::Type,
};

#[derive(Clone)]
struct VariableInfo {
    narrowed_type: Option<Type>,
    narrowed_fields: HashMap<Vec<String>, Type>,
    declared_type: Type,
    is_constant: bool,
    is_inout: bool,
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
    structures: HashMap<String, Vec<FieldDef>>,
    enums: HashMap<String, Vec<VariantDef>>,
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
        self.structures.clear();
        self.enums.clear();
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
            Stmt::Enum { name, variants } => {
                if self.scopes.len() != 1 {
                    return Err(
                        name.error("Los enums solo se permiten en el ámbito global del archivo.")
                    );
                }
                if self.structures.contains_key(&name.text)
                    || self.enums.contains_key(&name.text)
                    || self.functions.contains_key(&name.text)
                    || self.scopes[0].contains_key(&name.text)
                {
                    return Err(name.error("El nombre del enum ya está declarado."));
                }
                if variants.is_empty() {
                    return Err(name.error("Un enum requiere al menos una variante."));
                }
                let mut seen = HashSet::new();
                for variant in variants {
                    if !seen.insert(&variant.name.text) {
                        return Err(variant.name.error("La variante está repetida."));
                    }
                    let mut fields = HashSet::new();
                    for (kind, field) in &variant.fields {
                        self.validate_type(kind, field)?;
                        if !fields.insert(&field.text) {
                            return Err(field.error("El nombre del dato está repetido."));
                        }
                    }
                }
                self.enums.insert(name.text.clone(), variants.clone());
            }
            Stmt::Match { value, arms, line } => {
                let kind = self.expression_type(value)?;
                let Type::Named(enum_name) = &kind else {
                    return Err(format!(
                        "Línea {line}: match requiere un tipo enum concreto; se recibió {kind}."
                    ));
                };
                let variants = self.enums.get(enum_name).cloned().ok_or_else(|| {
                    format!(
                        "Línea {line}: match requiere un tipo enum concreto; se recibió {kind}."
                    )
                })?;
                self.forget_expression_effects(value);
                let mut seen = HashSet::new();
                let mut merged: Option<Self> = None;
                for arm in arms {
                    if arm.enum_name.text != *enum_name {
                        return Err(arm
                            .enum_name
                            .error(&format!("El patrón debe pertenecer al enum '{enum_name}'.")));
                    }
                    let variant = self.enum_variant(&arm.enum_name, &arm.variant)?;
                    if !seen.insert(arm.variant.text.clone()) {
                        return Err(arm.variant.error("La rama de esta variante está repetida."));
                    }
                    let bindings = arm.bindings.as_deref().unwrap_or(&[]);
                    if bindings.len() != variant.fields.len()
                        || arm.bindings.is_some() != !variant.fields.is_empty()
                    {
                        return Err(arm.variant.error(&format!("El patrón requiere {} capturas; las variantes sin datos se escriben sin paréntesis.", variant.fields.len())));
                    }
                    let mut branch = self.clone();
                    branch.scopes.push(HashMap::new());
                    for (binding, (kind, _)) in bindings.iter().zip(&variant.fields) {
                        let scope = branch.scopes.last_mut().expect("ámbito abierto");
                        if scope.contains_key(&binding.text) {
                            return Err(binding.error("El nombre de captura está repetido."));
                        }
                        scope.insert(
                            binding.text.clone(),
                            VariableInfo {
                                narrowed_type: None,
                                narrowed_fields: HashMap::new(),
                                declared_type: kind.clone(),
                                is_constant: false,
                                is_inout: false,
                            },
                        );
                    }
                    branch.check_statements(&arm.body)?;
                    branch.scopes.pop();
                    if let Some(previous) = merged.take() {
                        let mut next = self.clone();
                        next.merge_scopes(&previous, &branch);
                        merged = Some(next);
                    } else {
                        merged = Some(branch);
                    }
                }
                let missing: Vec<_> = variants
                    .iter()
                    .filter(|v| !seen.contains(&v.name.text))
                    .map(|v| v.name.text.as_str())
                    .collect();
                if !missing.is_empty() {
                    return Err(format!(
                        "Línea {line}: match no exhaustivo; faltan variantes de '{enum_name}': {}.",
                        missing.join(", ")
                    ));
                }
                if let Some(merged) = merged {
                    self.scopes = merged.scopes;
                }
            }
            Stmt::Struct { name, fields } => {
                if self.scopes.len() != 1 {
                    return Err(name.error(
                        "Las estructuras solo se permiten en el ámbito global del archivo.",
                    ));
                }
                if self.structures.contains_key(&name.text) {
                    return Err(
                        name.error(&format!("La estructura '{}' ya está declarada.", name.text))
                    );
                }
                if self.enums.contains_key(&name.text) {
                    return Err(name.error("El nombre ya está declarado como enum."));
                }
                if self.functions.contains_key(&name.text)
                    || self.scopes[0].contains_key(&name.text)
                {
                    return Err(name.error("El nombre de la estructura ya está declarado como variable o función global."));
                }
                let mut seen = HashSet::new();
                for field in fields {
                    if !seen.insert(&field.name.text) {
                        return Err(field
                            .name
                            .error(&format!("El campo '{}' está repetido.", field.name.text)));
                    }
                }
                // El nombre propio existe al validar sus campos. Un array vacío
                // o una alternativa no recursiva permite terminar el valor.
                self.structures.insert(name.text.clone(), fields.clone());
                for field in fields {
                    self.validate_type(&field.declared_type, &field.name)?;
                    if !Self::has_finite_alternative(&field.declared_type, &name.text) {
                        return Err(field.name.error("El campo recursivo no permite un valor finito; usa un array o una unión con una alternativa no recursiva."));
                    }
                }
                let mut defaults = self.clone();
                defaults.scopes = vec![self.scopes[0].clone(), HashMap::new()];
                for info in defaults.scopes[0].values_mut() {
                    info.is_constant = true;
                    info.narrowed_type = None;
                    info.narrowed_fields.clear();
                }
                // Los nombres de campos ocultan globales incluso antes de su
                // inicialización: un valor por defecto solo ve campos anteriores.
                for field in fields {
                    defaults.scopes[0].remove(&field.name.text);
                }
                for field in fields {
                    if let Some(value) = &field.default_value {
                        let actual =
                            defaults.expression_type_expected(value, Some(&field.declared_type))?;
                        Self::require_type(&field.name, &field.declared_type, &actual)?;
                    }
                    defaults.scopes[1].insert(
                        field.name.text.clone(),
                        VariableInfo {
                            declared_type: field.declared_type.clone(),
                            narrowed_type: None,
                            narrowed_fields: HashMap::new(),
                            is_constant: true,
                            is_inout: false,
                        },
                    );
                }
            }
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
                if self.structures.contains_key(&name.text) {
                    return Err(name.error("El nombre ya está declarado como estructura."));
                }
                if self.enums.contains_key(&name.text) {
                    return Err(name.error("El nombre ya está declarado como enum."));
                }
                if let Some(kind) = return_type {
                    self.validate_type(kind, name)?;
                }
                for parameter in parameters {
                    self.validate_type(&parameter.declared_type, &parameter.name)?;
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
                let globals = self.scopes[0].clone();
                for info in self.scopes[0].values_mut() {
                    info.narrowed_fields.clear();
                }
                self.scopes.push(HashMap::new());
                for parameter in parameters {
                    self.scopes.last_mut().expect("ámbito abierto").insert(
                        parameter.name.text.clone(),
                        VariableInfo {
                            narrowed_type: None,
                            narrowed_fields: HashMap::new(),
                            declared_type: parameter.declared_type.clone(),
                            is_constant: false,
                            is_inout: parameter.is_inout,
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
                self.scopes[0] = globals;
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
                self.validate_type(declared_type, name)?;
                if self.scopes.len() == 1 && self.structures.contains_key(&name.text) {
                    return Err(name.error("El nombre ya está declarado como estructura."));
                }
                if self.scopes.len() == 1 && self.enums.contains_key(&name.text) {
                    return Err(name.error("El nombre ya está declarado como enum."));
                }
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
                self.forget_expression_effects(initializer);
                // Registrar después del inicializador impide int x = x;.
                self.scopes.last_mut().expect("ámbito abierto").insert(
                    name.text.clone(),
                    VariableInfo {
                        narrowed_type: None,
                        narrowed_fields: HashMap::new(),
                        declared_type: declared_type.clone(),
                        is_constant: *is_constant,
                        is_inout: false,
                    },
                );
            }
            Stmt::Assign { name, steps, value } => {
                let checked_target = self.assignment_target_kind(name, steps, true)?;
                let target_type = if steps.is_empty() {
                    self.lookup(name)?.declared_type.clone()
                } else {
                    checked_target
                };
                let actual = self
                    .after_indices(steps)
                    .expression_type_expected(value, Some(&target_type))?;
                Self::require_type(name, &target_type, &actual)?;
                self.forget_target(name, steps);
            }
            Stmt::CompoundAssign {
                name,
                steps,
                operator,
                value,
                line,
            } => {
                let target_type = self.assignment_target(name, steps)?;
                let operand_type = self
                    .after_indices(steps)
                    .expression_type_expected(value, Some(&target_type))?;
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
                steps,
                operator,
                line,
            } => {
                let target_type = self.assignment_target(name, steps)?;
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
                self.validate_type(declared_type, name)?;
                let iterable_type = self.expression_type(iterable)?;
                let Type::Array(element) = &iterable_type else {
                    return Err(format!(
                        "Línea {line}: 'foreach' solo recorre arrays; se recibió {iterable_type}."
                    ));
                };
                let element = element.as_ref().clone();
                self.forget_expression_effects(iterable);
                Self::require_type(name, declared_type, &element)?;
                self.scopes.push(HashMap::new());
                self.scopes.last_mut().expect("ámbito abierto").insert(
                    name.text.clone(),
                    VariableInfo {
                        narrowed_type: None,
                        narrowed_fields: HashMap::new(),
                        declared_type: declared_type.clone(),
                        is_constant: false,
                        is_inout: false,
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
        if !matches!(statement, Stmt::Declare { .. }) {
            self.forget_statement_effects(statement);
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

    fn enum_variant(&self, name: &Name, variant: &Name) -> Result<&VariantDef, String> {
        self.enums
            .get(&name.text)
            .ok_or_else(|| name.error(&format!("El enum '{}' no está declarado.", name.text)))?
            .iter()
            .find(|v| v.name.text == variant.text)
            .ok_or_else(|| {
                variant.error(&format!(
                    "La variante '{}' no existe en '{}'.",
                    variant.text, name.text
                ))
            })
    }

    fn qualified_type(
        &self,
        path: &[Name],
        arguments: &Option<Vec<Expr>>,
    ) -> Result<Option<Type>, String> {
        if self.enums.contains_key(&path[0].text) || arguments.is_none() {
            let variant = self.enum_variant(&path[0], &path[1])?;
            let supplied = arguments.as_deref().unwrap_or(&[]);
            if supplied.len() != variant.fields.len()
                || arguments.is_some() != !variant.fields.is_empty()
            {
                return Err(path[1].error(&format!("La variante esperaba {} argumentos; las variantes sin datos se escriben sin paréntesis.", variant.fields.len())));
            }
            let mut context = self.clone();
            for (value, (expected, field)) in supplied.iter().zip(&variant.fields) {
                let actual = context.expression_type_expected(value, Some(expected))?;
                Self::require_type(field, expected, &actual)?;
                context.forget_expression_effects(value);
            }
            Ok(Some(Type::Named(path[0].text.clone())))
        } else {
            self.check_library_call(
                path,
                None,
                arguments.as_deref().expect("llamada con argumentos"),
            )
        }
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
            Expr::Qualified { path, arguments } => {
                self.qualified_type(path, arguments)?.ok_or_else(|| {
                    path[1].error("'push' no devuelve un valor; úsalo como instrucción con ';'.")
                })
            }
            Expr::Struct { name, fields } => {
                let declared = self.structures.get(&name.text).ok_or_else(|| {
                    name.error(&format!("La estructura '{}' no está declarada.", name.text))
                })?;
                let mut seen = HashSet::new();
                let mut context = self.clone();
                for (field, value) in fields {
                    if !seen.insert(&field.text) {
                        return Err(
                            field.error(&format!("El campo '{}' está repetido.", field.text))
                        );
                    }
                    let kind = self.field_type(Type::Named(name.text.clone()), field)?;
                    let actual = context.expression_type_expected(value, Some(&kind))?;
                    context.forget_expression_effects(value);
                    Self::require_type(field, &kind, &actual)?;
                }
                for field in declared {
                    if field.default_value.is_none() && !seen.contains(&field.name.text) {
                        return Err(name.error(&format!(
                            "Falta inicializar el campo '{}' de '{}'.",
                            field.name.text, name.text
                        )));
                    }
                }
                Ok(Type::Named(name.text.clone()))
            }
            Expr::Field { object, name } => {
                if let Some((root, fields)) = expression.field_path() {
                    self.path_type(&root, &fields, false)
                } else {
                    self.field_type(self.expression_type(object)?, name)
                }
            }
            Expr::TypeCheck {
                name,
                fields,
                target,
                ..
            } => {
                self.validate_type(target, name)?;
                self.path_type(name, fields, false)?;
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
                let mut context = self.clone();
                for element in elements {
                    let actual = context.expression_type_expected(element, Some(&element_type))?;
                    context.forget_expression_effects(element);
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
                let kind = self.expression_type(array)?;
                let mut context = self.clone();
                context.forget_expression_effects(array);
                context.indexed_type(kind, index, *line)
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
                let mut after_left = self.clone();
                after_left.forget_expression_effects(left);
                let right_type = match operator {
                    BinaryOp::And => self.with_condition(left, true).expression_type(right)?,
                    BinaryOp::Or => self.with_condition(left, false).expression_type(right)?,
                    _ => after_left.expression_type(right)?,
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
        if let Expr::Qualified { path, arguments } = expression {
            return self.qualified_type(path, arguments);
        }
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
            let mut context = self.clone();
            for (argument, (expected, is_inout)) in arguments.iter().zip(&signature.parameters) {
                let actual = match (argument, is_inout) {
                    (CallArgument::Value(value), false) => {
                        let kind = context.expression_type_expected(value, Some(expected))?;
                        context.forget_expression_effects(value);
                        kind
                    },
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
        self.check_library_call(path, receiver.as_deref(), arguments)
    }

    fn check_library_call(
        &self,
        path: &[Name],
        receiver: Option<&Expr>,
        arguments: &[Expr],
    ) -> Result<Option<Type>, String> {
        let function = self.library.resolve(path, receiver.is_some())?;
        let name = path.last().expect("ruta con nombre de método");
        function.check_arity(arguments.len(), receiver.is_some(), name)?;
        let array = receiver.unwrap_or_else(|| &arguments[0]);
        let array_type = self.expression_type(array)?;
        let result = function.result_type(&array_type, name)?;
        if !matches!(function, ArrayFunction::Len) {
            let (target, steps) = array.clone().into_target().ok_or_else(|| {
                name.error("Se necesita una variable array modificable o uno de sus subarrays.")
            })?;
            self.assignment_target(&target, &steps)?;
        }
        if matches!(function, ArrayFunction::Push) {
            let Type::Array(element) = array_type else {
                unreachable!("tipo comprobado")
            };
            let value = arguments.last().expect("argumento de push validado");
            let mut context = self.clone();
            context.forget_expression_effects(array);
            let actual = context.expression_type_expected(value, Some(&element))?;
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
    fn assignment_target(&self, name: &Name, steps: &[TargetStep]) -> Result<Type, String> {
        self.assignment_target_kind(name, steps, false)
    }

    fn after_indices(&self, steps: &[TargetStep]) -> Self {
        let mut context = self.clone();
        for step in steps {
            if let TargetStep::Index(index, _) = step {
                context.forget_expression_effects(index);
            }
        }
        context
    }

    fn assignment_target_kind(
        &self,
        name: &Name,
        steps: &[TargetStep],
        declared_leaf: bool,
    ) -> Result<Type, String> {
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
        let context = self.after_indices(steps);
        let mut target_type = context.variable_type(name)?;
        let mut path = Some(Vec::new());
        for (position, step) in steps.iter().enumerate() {
            target_type = match step {
                TargetStep::Index(index, line) => {
                    path = None;
                    context.indexed_type(target_type, index, *line)?
                }
                TargetStep::Field(field) => {
                    let definition = context.field_definition(&target_type, field)?;
                    if definition.is_constant {
                        return Err(field.error(&format!(
                            "No se puede modificar el campo constante '{}'.",
                            field.text
                        )));
                    }
                    let mut kind = definition.declared_type.clone();
                    if let Some(path) = &mut path {
                        path.push(field.text.clone());
                        if !(declared_leaf && position + 1 == steps.len())
                            && let Some(refined) = context.lookup(name)?.narrowed_fields.get(path)
                        {
                            kind = refined.clone();
                        }
                    }
                    kind
                }
            };
        }
        Ok(target_type)
    }

    fn validate_type(&self, kind: &Type, name: &Name) -> Result<(), String> {
        match kind {
            Type::Named(structure) if !self.structures.contains_key(structure) && !self.enums.contains_key(structure) => {
                Err(name.error(&format!("El tipo '{structure}' no está declarado; declara las estructuras o enums antes de usarlos.")))
            }
            Type::Array(element) => self.validate_type(element, name),
            Type::Union(types) => {
                for kind in types { self.validate_type(kind, name)?; }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn has_finite_alternative(kind: &Type, own_name: &str) -> bool {
        match kind {
            Type::Named(name) => name != own_name,
            Type::Union(types) => types
                .iter()
                .any(|kind| Self::has_finite_alternative(kind, own_name)),
            _ => true,
        }
    }

    fn field_definition(&self, object: &Type, name: &Name) -> Result<&FieldDef, String> {
        let Type::Named(structure) = object else {
            return Err(name.error(&format!(
                "Solo las estructuras tienen campos; se recibió {object}."
            )));
        };
        self.structures
            .get(structure)
            .ok_or_else(|| {
                name.error(&format!(
                    "Solo las estructuras tienen campos; se recibió {object}."
                ))
            })?
            .iter()
            .find(|field| field.name.text == name.text)
            .ok_or_else(|| {
                name.error(&format!(
                    "El campo '{}' no existe en '{structure}'.",
                    name.text
                ))
            })
    }

    fn field_type(&self, object: Type, name: &Name) -> Result<Type, String> {
        Ok(self.field_definition(&object, name)?.declared_type.clone())
    }

    fn path_type(&self, root: &Name, fields: &[Name], declared_leaf: bool) -> Result<Type, String> {
        let info = self.lookup(root)?;
        let mut kind = self.variable_type(root)?;
        let mut path = Vec::new();
        for (index, field) in fields.iter().enumerate() {
            kind = self.field_type(kind, field)?;
            path.push(field.text.clone());
            if !(declared_leaf && index + 1 == fields.len())
                && let Some(refined) = info.narrowed_fields.get(&path)
            {
                kind = refined.clone();
            }
        }
        Ok(kind)
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
            info.narrowed_fields.clear();
        }
    }

    fn forget_alias_fields(&mut self, root: &Name) {
        let alias = self.lookup(root).is_ok_and(|info| info.is_inout);
        if alias {
            // Dos parámetros inout pueden señalar la misma variable global.
            for (index, scope) in self.scopes.iter_mut().enumerate() {
                for info in scope.values_mut() {
                    if index == 0 || info.is_inout {
                        info.narrowed_fields.clear();
                    }
                }
            }
        } else if let Some(info) = self
            .scopes
            .iter_mut()
            .rev()
            .find_map(|scope| scope.get_mut(&root.text))
        {
            info.narrowed_fields.clear();
        }
    }

    fn forget_target(&mut self, root: &Name, steps: &[TargetStep]) {
        if self.lookup(root).is_ok_and(|info| info.is_inout) {
            self.forget_alias_fields(root);
        }
        if steps.is_empty() {
            self.forget_name(&root.text);
            return;
        }
        let path: Option<Vec<_>> = steps
            .iter()
            .map(|step| match step {
                TargetStep::Field(field) => Some(field.text.clone()),
                TargetStep::Index(_, _) => None,
            })
            .collect();
        if let Some(info) = self
            .scopes
            .iter_mut()
            .rev()
            .find_map(|scope| scope.get_mut(&root.text))
        {
            info.narrowed_fields
                .retain(|key, _| path.as_ref().is_some_and(|path| !key.starts_with(path)));
        }
    }

    // Las funciones reciben estructuras completas. Un permiso inout puede
    // cambiar cualquier campo, aunque la función concreta no lo haga.
    fn forget_expression_effects(&mut self, expression: &Expr) {
        match expression {
            Expr::Qualified { arguments, .. } => {
                if let Some(arguments) = arguments {
                    for argument in arguments {
                        self.forget_expression_effects(argument);
                    }
                }
            }
            Expr::Call { arguments, .. } => {
                for argument in arguments {
                    match argument {
                        CallArgument::Value(value) => self.forget_expression_effects(value),
                        CallArgument::InOut(name) => self.forget_alias_fields(name),
                    }
                }
            }
            Expr::Struct { fields, .. } => {
                for (_, value) in fields {
                    self.forget_expression_effects(value);
                }
            }
            Expr::Array { elements, .. } => {
                for element in elements {
                    self.forget_expression_effects(element);
                }
            }
            Expr::LibraryCall {
                receiver,
                arguments,
                ..
            } => {
                if let Some(receiver) = receiver {
                    self.forget_expression_effects(receiver);
                }
                for argument in arguments {
                    self.forget_expression_effects(argument);
                }
            }
            Expr::Field { object, .. } => self.forget_expression_effects(object),
            Expr::Cast { value, .. } | Expr::Unary { operand: value, .. } => {
                self.forget_expression_effects(value)
            }
            Expr::Index { array, index, .. } => {
                self.forget_expression_effects(array);
                self.forget_expression_effects(index);
            }
            Expr::Binary { left, right, .. } => {
                self.forget_expression_effects(left);
                self.forget_expression_effects(right);
            }
            Expr::Literal(_) | Expr::Variable(_) | Expr::TypeCheck { .. } => {}
        }
    }

    fn forget_statement_effects(&mut self, statement: &Stmt) {
        match statement {
            Stmt::Match { value, arms, .. } => {
                self.forget_expression_effects(value);
                for arm in arms {
                    for statement in &arm.body {
                        self.forget_statement_effects(statement);
                    }
                }
            }
            Stmt::Call(value)
            | Stmt::Print(value)
            | Stmt::Println(value)
            | Stmt::Declare {
                initializer: value, ..
            } => self.forget_expression_effects(value),
            Stmt::Assign { steps, value, .. } | Stmt::CompoundAssign { steps, value, .. } => {
                for step in steps {
                    if let TargetStep::Index(index, _) = step {
                        self.forget_expression_effects(index);
                    }
                }
                self.forget_expression_effects(value);
            }
            Stmt::Increment { steps, .. } => {
                for step in steps {
                    if let TargetStep::Index(index, _) = step {
                        self.forget_expression_effects(index);
                    }
                }
            }
            Stmt::Return {
                value: Some(value), ..
            } => self.forget_expression_effects(value),
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.forget_expression_effects(condition);
                for statement in then_branch {
                    self.forget_statement_effects(statement);
                }
                if let Some(branch) = else_branch {
                    for statement in branch {
                        self.forget_statement_effects(statement);
                    }
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                self.forget_expression_effects(condition);
                for statement in body {
                    self.forget_statement_effects(statement);
                }
            }
            Stmt::For {
                initializer,
                condition,
                update,
                body,
                ..
            } => {
                self.forget_statement_effects(initializer);
                self.forget_expression_effects(condition);
                for statement in body {
                    self.forget_statement_effects(statement);
                }
                self.forget_statement_effects(update);
            }
            Stmt::Foreach { iterable, body, .. } => {
                self.forget_expression_effects(iterable);
                for statement in body {
                    self.forget_statement_effects(statement);
                }
            }
            _ => {}
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
        self.forget_statement_effects(statement);
        match statement {
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    self.forget_loop_writes(&arm.body);
                }
            }
            Stmt::Assign { name, steps, .. } => self.forget_target(name, steps),
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
        context.forget_expression_effects(condition);
        context
    }

    fn assume_condition(&mut self, condition: &Expr, truth: bool) {
        match condition {
            Expr::TypeCheck {
                name,
                fields,
                target,
                negated,
            } => {
                let Ok(current) = self.path_type(name, fields, false) else {
                    return;
                };
                let alternatives = match current {
                    Type::Union(types) => types,
                    kind => vec![kind],
                };
                let mut remaining: Vec<_> = alternatives
                    .into_iter()
                    .filter(|kind| (kind == target) == (truth != *negated))
                    .collect();
                let refined = match remaining.len() {
                    0 => return,
                    1 => remaining.remove(0),
                    _ => Type::Union(remaining),
                };
                if let Some(info) = self
                    .scopes
                    .iter_mut()
                    .rev()
                    .find_map(|scope| scope.get_mut(&name.text))
                {
                    if fields.is_empty() {
                        info.narrowed_type = Some(refined);
                    } else {
                        info.narrowed_fields.insert(
                            fields.iter().map(|field| field.text.clone()).collect(),
                            refined,
                        );
                    }
                }
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
                info.narrowed_fields = a
                    .narrowed_fields
                    .iter()
                    .filter_map(|(path, kind)| {
                        let other = b.narrowed_fields.get(path)?;
                        let mut types = match kind {
                            Type::Union(types) => types.clone(),
                            kind => vec![kind.clone()],
                        };
                        for kind in match other {
                            Type::Union(types) => types.clone(),
                            kind => vec![kind.clone()],
                        } {
                            if !types.contains(&kind) {
                                types.push(kind);
                            }
                        }
                        Some((
                            path.clone(),
                            if types.len() == 1 {
                                types.remove(0)
                            } else {
                                Type::Union(types)
                            },
                        ))
                    })
                    .collect();
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
            Stmt::Match { arms, .. } => {
                !arms.is_empty() && arms.iter().all(|arm| Self::always_returns(&arm.body))
            }
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
