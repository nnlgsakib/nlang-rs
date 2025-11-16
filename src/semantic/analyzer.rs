use crate::ast::{Program, Statement, Expr, Type, Literal, BinaryOperator, WhenCase, MatchCase};
use crate::std_lib::StdLib;
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use super::error::SemanticError;
use super::symbol::{Symbol, ModuleInfo};
use super::type_inference::TypeInferenceEngine;

pub struct SemanticAnalyzer {
    // For tracking nested scopes
    scopes: Vec<HashMap<String, Symbol>>,
    // Track current function context for return type checking
    current_function_return_type: Option<Type>,
    // Standard library for built-in functions
    std_lib: StdLib,
    // Module cache to avoid reloading the same module
    module_cache: HashMap<PathBuf, ModuleInfo>,
    // Current working directory for resolving relative imports
    current_dir: PathBuf,
    // Advanced type inference engine
    type_inference: TypeInferenceEngine,
    std_imported: bool,
    allow_builtin_override: bool,
}

impl SemanticAnalyzer {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::new_with_file_path(None)
    }
    
    pub fn new_with_file_path(file_path: Option<&std::path::Path>) -> Self {
        let current_dir = if let Some(path) = file_path {
            // Use the directory of the file being analyzed
            path.parent().unwrap_or_else(|| std::path::Path::new(".")).to_path_buf()
        } else {
            // Fallback to current working directory
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        };
        
        Self {
            scopes: vec![HashMap::new()], // Global scope
            current_function_return_type: None,
            std_lib: StdLib::new(),
            module_cache: HashMap::new(),
            current_dir,
            type_inference: TypeInferenceEngine::new(),
            std_imported: false,
            allow_builtin_override: false,
        }
    }
    
    fn has_return_statement(&self, statements: &[Statement]) -> bool {
        for stmt in statements {
            if self.statement_has_return(stmt) {
                return true;
            }
        }
        false
    }
    
    fn statement_has_return(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Return { .. } => true,
            Statement::If { then_branch, else_branch, .. } => {
                self.statement_has_return(then_branch) || 
                else_branch.as_ref().map_or(false, |else_stmt| self.statement_has_return(else_stmt))
            }
            Statement::While { body, .. } => {
                self.statement_has_return(body)
            }
            Statement::Block { statements } => {
                self.has_return_statement(statements)
            }
            _ => false,
        }
    }
    
    fn find_return_type_in_statements(&mut self, statements: &[Statement]) -> Option<Type> {
        // First pass: analyze variable declarations to build the symbol table
        for stmt in statements.iter() {
            match stmt {
                Statement::LetDeclaration { name, initializer, .. } => {
                    // Analyze the initializer if present
                     let var_type = if let Some(init_expr) = initializer {
                         match self.analyze_expr(init_expr.clone()) {
                            Ok(analyzed_expr) => {
                                match self.infer_type(&analyzed_expr) {
                                    Ok(inferred_type) => inferred_type,
                                    Err(_) => Type::Integer, // Default fallback
                                }
                            }
                            Err(_) => Type::Integer, // Default fallback
                        }
                    } else {
                        Type::Integer // Default type for uninitialized variables
                    };
                    
                    // Define the variable in the current scope
                    let _ = self.define_symbol(name.clone(), Symbol::Variable { var_type, is_mutable: false });
                }
                _ => {} // Skip other statements in first pass
            }
        }
        
        // Second pass: look for return statements
        for stmt in statements.iter() {
            if let Some(return_type) = self.find_return_type_in_statement(stmt) {
                return Some(return_type);
            }
        }
        None
    }
    
    fn find_return_type_in_statement(&mut self, stmt: &Statement) -> Option<Type> {
        match stmt {
            Statement::Return { value } => {
                if let Some(return_expr) = value {
                    match self.analyze_expr(*return_expr.clone()) {
                        Ok(analyzed_expr) => {
                            if let Ok(expr_type) = self.infer_type(&analyzed_expr) {
                                return Some(expr_type);
                            }
                        }
                        Err(_) => {
                            return None;
                        }
                    }
                }
                None
            }
            Statement::If { condition: _, then_branch, else_branch } => {
                if let Some(return_type) = self.find_return_type_in_statement(then_branch) {
                    return Some(return_type);
                }
                if let Some(else_stmt) = else_branch {
                    if let Some(return_type) = self.find_return_type_in_statement(else_stmt) {
                        return Some(return_type);
                    }
                }
                None
            }
            Statement::While { condition: _, body } => {
                self.find_return_type_in_statement(body)
            }
            Statement::Block { statements } => {
                self.find_return_type_in_statements(statements)
            }
            _ => None,
        }
    }
    
    pub fn analyze_program(&mut self, program: Program, is_main_program: bool) -> Result<Program, SemanticError> {
        // First pass: collect function declarations (without return type inference)
        for stmt in &program.statements {
            if let Statement::FunctionDeclaration { name, parameters, return_type, .. } = stmt {
                let func_return_type = return_type.clone().unwrap_or(Type::Void);
                self.define_symbol(name.clone(), Symbol::Function { 
                    return_type: func_return_type, 
                    parameters: parameters.clone() 
                })?;
            }
        }

        // Second pass: infer return types for all functions before analyzing other statements
        for stmt in &program.statements {
            if let Statement::FunctionDeclaration { name, parameters, body, return_type, .. } = stmt {
                if return_type.is_none() {
                    // Enter a temporary scope with parameters to infer types used in returns
                    self.begin_scope();
                    for param in parameters {
                        let param_type = param.param_type.clone().unwrap_or(Type::Unknown);
                        self.define_symbol(param.name.clone(), Symbol::Variable { var_type: param_type, is_mutable: false })?;
                    }
                    if let Some(inferred) = self.find_return_type_in_statements(body) {
                        // Update the function symbol with the inferred return type
                        for scope in self.scopes.iter_mut().rev() {
                            if let Some(sym) = scope.get_mut(name) {
                                if let Symbol::Function { parameters: existing_params, .. } = sym {
                                    *sym = Symbol::Function {
                                        return_type: inferred.clone(),
                                        parameters: existing_params.clone(),
                                    };
                                }
                                break;
                            }
                        }
                    }
                    self.end_scope();
                }
            }
        }

        // Third pass: analyze all statements and collect imported function definitions
        let mut analyzed_statements = Vec::new();
        let mut imported_function_definitions = Vec::new();
        
        for stmt in program.statements {
            let analyzed_stmt = self.analyze_statement(stmt)?;
            
            // Check if this is an import statement and collect imported function definitions
            if let Statement::Import { module, alias: _ } = &analyzed_stmt {
                if module != "std" {
                    let module_path = self.resolve_module_path(module);
                    let module_info = self.load_module(&module_path)?;
                    for (symbol_name, symbol) in &module_info.exported_symbols {
                        if let Symbol::Function { return_type: _, parameters: _ } = symbol {
                            if let Some(func_stmt) = self.find_function_definition_in_module(&module_path, symbol_name) {
                                imported_function_definitions.push(func_stmt);
                            }
                        }
                    }
                }
            }
            
            analyzed_statements.push(analyzed_stmt);
        }
        
        // Add imported function definitions to the analyzed program
        analyzed_statements.extend(imported_function_definitions);
        
        // Validate that a main function exists and has the correct signature (only for main program)
        if is_main_program {
            self.validate_main_function()?;
        }
        
        Ok(Program { statements: analyzed_statements })
    }
    
    fn analyze_statement(&mut self, stmt: Statement) -> Result<Statement, SemanticError> {
        match stmt {
            Statement::Expression(expr) => {
                let analyzed_expr = self.analyze_expr(expr)?;
                Ok(Statement::Expression(analyzed_expr))
            },
            Statement::LetDeclaration { name, initializer, var_type, is_mutable, is_exported } => {
                let analyzed_initializer = match initializer {
                    Some(expr) => Some(self.analyze_expr(expr)?),
                    None => None,
                };
                
                // Use declared type if provided, otherwise infer from initializer using enhanced type inference
                let final_var_type = if let Some(declared_type) = var_type {
                    // Validate that initializer type matches declared type if present
                    if let Some(ref init) = analyzed_initializer {
                        // Use enhanced type inference for the initializer
                        let init_type = self.type_inference.infer_expression_type(init);

                        if !self.are_types_compatible(&init_type, &declared_type) {
                            return Err(SemanticError {
                                message: format!("Type mismatch: variable '{}' declared as {:?} but initializer is of type {:?}", name, declared_type, init_type),
                            });
                        }

                        // Validate literal bounds for integer types
                        if let Expr::Literal(literal) = init {
                            self.validate_literal_bounds(literal, &declared_type)?;
                        }

                        // Store the inferred type in the type inference engine
                        self.type_inference.set_variable_type(name.clone(), declared_type.clone());
                    }
                    declared_type
                } else if let Some(ref init) = analyzed_initializer {
                    // Use enhanced type inference from initializer
                    let inferred_type = self.type_inference.infer_expression_type(init);

                    // Handle unknown types by falling back to basic inference
                    let final_type = if inferred_type == Type::Unknown {
                        self.infer_type(init)?
                    } else {
                        inferred_type
                    };

                    // Store the inferred type in the type inference engine
                    self.type_inference.set_variable_type(name.clone(), final_type.clone());
                    final_type
                } else {
                    // Default type for uninitialized variables
                    let default_type = Type::Integer;
                    self.type_inference.set_variable_type(name.clone(), default_type.clone());
                    default_type
                };
                
                self.define_symbol(name.clone(), Symbol::Variable { var_type: final_var_type.clone(), is_mutable })?;
                
                Ok(Statement::LetDeclaration {
                    name,
                    initializer: analyzed_initializer,
                    var_type: Some(final_var_type),
                    is_mutable,
                    is_exported,
                })
            },
            Statement::FunctionDeclaration { name, parameters, body, return_type, is_exported } => {
                // Create mutable parameters for type inference
                let mut inferred_parameters = parameters.clone();

                // Enter function context for type inference
                self.type_inference.enter_function_context(inferred_parameters.clone(), return_type.clone());

                // First pass: Infer parameter types from usage in function body
                if let Err(e) = self.type_inference.infer_function_parameter_types(&mut inferred_parameters, &body) {
                    self.type_inference.exit_function_context();
                    return Err(e);
                }

                // Resolve parameter types (use inferred if not explicitly declared)
                for param in &mut inferred_parameters {
                    let final_type = if let Some(explicit_type) = &param.param_type {
                        explicit_type.clone()
                    } else if let Some(inferred_type) = &param.inferred_type {
                        inferred_type.clone()
                    } else {
                        Type::Unknown
                    };

                    // Set the final type for the parameter
                    param.param_type = Some(final_type.clone());
                    self.type_inference.set_variable_type(param.name.clone(), final_type);
                }

                // Infer return type if not explicitly declared
                let inferred_return_type = if return_type.is_none() {
                    if let Some(inferred) = self.type_inference.infer_function_return_type(&body) {
                        inferred
                    } else {
                        Type::Void
                    }
                } else {
                    return_type.clone().unwrap()
                };

                // Set current function context
                let previous_return_type = self.current_function_return_type.clone();
                self.current_function_return_type = Some(inferred_return_type.clone());

                // Enter function scope for actual analysis
                self.begin_scope();

                // Add parameters to the scope with their resolved types
                for param in &inferred_parameters {
                    let param_type = param.param_type.as_ref().unwrap();
                    self.define_symbol(
                        param.name.clone(),
                        Symbol::Variable { var_type: param_type.clone(), is_mutable: false }
                    )?;
                }

                // Analyze function body
                let mut analyzed_body = Vec::new();

                for stmt in body {
                    analyzed_body.push(self.analyze_statement(stmt)?);
                }

                // Check if non-void function has return statement (recursively)
                let has_return = self.has_return_statement(&analyzed_body);
                if inferred_return_type != Type::Void && !has_return {
                    // Exit function context and scope before returning error
                    self.type_inference.exit_function_context();
                    self.end_scope();
                    self.current_function_return_type = previous_return_type;

                    return Err(SemanticError {
                        message: format!("Function '{}' with return type {:?} must have a return statement", name, inferred_return_type),
                    });
                }

                // Update the return type check to work with inferred types
                // If we inferred a non-void type but the function was analyzed assuming void,
                // we need to re-analyze with the correct return type
                if return_type.is_none() && inferred_return_type != Type::Void {
                    // Update the function's return type in the symbol table
                    for scope in self.scopes.iter_mut().rev() {
                        if let Some(sym) = scope.get_mut(&name) {
                            if let Symbol::Function { return_type: ret_type, .. } = sym {
                                *ret_type = inferred_return_type.clone();
                                break;
                            }
                        }
                    }
                }

                // Exit function scope and restore previous context
                self.end_scope();
                self.current_function_return_type = previous_return_type;
                self.type_inference.exit_function_context();

                Ok(Statement::FunctionDeclaration {
                    name,
                    parameters: inferred_parameters,
                    body: analyzed_body,
                    return_type: Some(inferred_return_type),
                    is_exported,
                })
            },
            Statement::Block { statements } => {
                self.begin_scope();
                
                let mut analyzed_statements = Vec::new();
                for stmt in statements {
                    analyzed_statements.push(self.analyze_statement(stmt)?);
                }
                
                self.end_scope();
                
                Ok(Statement::Block { statements: analyzed_statements })
            },
            Statement::If { condition, then_branch, else_branch } => {
                let analyzed_condition = self.analyze_expr(*condition)?;
                
                // Check that condition is of boolean type
                if self.infer_type(&analyzed_condition)? != Type::Boolean {
                    return Err(SemanticError {
                        message: "If condition must be of boolean type".to_string(),
                    });
                }
                
                let analyzed_then = Box::new(self.analyze_statement(*then_branch)?);
                let analyzed_else = match else_branch {
                    Some(branch) => Some(Box::new(self.analyze_statement(*branch)?)),
                    None => None,
                };
                
                Ok(Statement::If {
                    condition: Box::new(analyzed_condition),
                    then_branch: analyzed_then,
                    else_branch: analyzed_else,
                })
            },
            Statement::While { condition, body } => {
                let analyzed_condition = self.analyze_expr(*condition)?;
                
                // Check that condition is of boolean type
                if self.infer_type(&analyzed_condition)? != Type::Boolean {
                    return Err(SemanticError {
                        message: "While condition must be of boolean type".to_string(),
                    });
                }
                
                let analyzed_body = Box::new(self.analyze_statement(*body)?);
                
                Ok(Statement::While {
                    condition: Box::new(analyzed_condition),
                    body: analyzed_body,
                })
            },
            Statement::For { initializer, condition, increment, body } => {
                self.begin_scope();

                let analyzed_initializer = if let Some(init) = initializer {
                    Some(Box::new(self.analyze_statement(*init)?))
                } else {
                    None
                };

                let analyzed_condition = if let Some(cond) = condition {
                    let analyzed_cond_expr = self.analyze_expr(*cond)?;
                    if self.infer_type(&analyzed_cond_expr)? != Type::Boolean {
                        return Err(SemanticError {
                            message: "For loop condition must be of boolean type".to_string(),
                        });
                    }
                    Some(Box::new(analyzed_cond_expr))
                } else {
                    None
                };

                let analyzed_body = Box::new(self.analyze_statement(*body)?);

                let analyzed_increment = if let Some(inc) = increment {
                    Some(Box::new(self.analyze_expr(*inc)?))
                } else {
                    None
                };

                self.end_scope();

                Ok(Statement::For {
                    initializer: analyzed_initializer,
                    condition: analyzed_condition,
                    increment: analyzed_increment,
                    body: analyzed_body,
                })
            },
            Statement::Return { value } => {
                let analyzed_value = match value {
                    Some(expr) => Some(Box::new(self.analyze_expr(*expr)?)),
                    None => None,
                };
                
                // Validate return type matches function signature
                if let Some(expected_return_type) = &self.current_function_return_type {
                    match (&analyzed_value, expected_return_type) {
                        (None, Type::Void) => {}, // void return with no value is OK
                        (Some(val), expected_type) => {
                            let actual_type = self.infer_type(val)?;
                            if !self.are_types_compatible(&actual_type, expected_type) {
                                return Err(SemanticError {
                                    message: format!(
                                        "Return type mismatch: expected {:?}, got {:?}",
                                        expected_type, actual_type
                                    ),
                                });
                            }
                        },
                        (None, expected_type) if *expected_type != Type::Void => {
                            return Err(SemanticError {
                                message: format!(
                                    "Function expects return type {:?}, but no value returned",
                                    expected_type
                                ),
                            });
                        },
                        _ => {},
                    }
                }
                
                Ok(Statement::Return { value: analyzed_value })
            },
            Statement::Import { module, alias } => {
                if module == "std" {
                    let module_info = self.load_std_module()?;
                    self.std_imported = true;
                    // Always provide a 'std' namespace
                    self.define_symbol("std".to_string(), Symbol::Namespace { module_name: "std".to_string() })?;
                    // Expose namespaced and direct symbols (override built-ins)
                    let prev_override = self.allow_builtin_override;
                    self.allow_builtin_override = true;
                    for (symbol_name, symbol) in &module_info.exported_symbols {
                        let namespaced_name = format!("std.{}", symbol_name);
                        self.define_symbol(namespaced_name, symbol.clone())?;
                        if !self.scopes.last().unwrap().contains_key(symbol_name) {
                            self.define_symbol(symbol_name.clone(), symbol.clone())?;
                        }
                    }
                    self.allow_builtin_override = prev_override;
                    return Ok(Statement::Import { module, alias });
                }
                // Resolve and load non-std modules
                let module_path = self.resolve_module_path(&module);
                let module_info = self.load_module(&module_path)?;
                
                // Add the module's exported symbols to the current scope
                if let Some(alias_name) = &alias {
                    // Create a namespace symbol for the alias
                    self.define_symbol(alias_name.clone(), Symbol::Namespace { module_name: module.clone() })?;
                    
                    // Create a namespace for the module under the alias
                    for (symbol_name, symbol) in &module_info.exported_symbols {
                        let namespaced_name = format!("{}.{}", alias_name, symbol_name);
                        self.define_symbol(namespaced_name, symbol.clone())?;
                    }
                } else {
                    // Add all exported symbols directly to the current scope
                    for (symbol_name, symbol) in &module_info.exported_symbols {
                        if !self.scopes.last().unwrap().contains_key(symbol_name) {
                            self.define_symbol(symbol_name.clone(), symbol.clone())?;
                        }
                    }
                }
                
                Ok(Statement::Import { module, alias })
            },
            Statement::ImportFrom { module, items } => {
                let module_info = if module == "std" { 
                    self.std_imported = true; 
                    self.load_std_module()? 
                } else {
                    let module_path = self.resolve_module_path(&module);
                    self.load_module(&module_path)?
                };
                
                // Import specific items from the module
                let prev_override = self.allow_builtin_override;
                if module == "std" { self.allow_builtin_override = true; }
                for (item, alias) in &items {
                    let symbol_name = alias.as_ref().unwrap_or(item);
                    if let Some(symbol) = module_info.exported_symbols.get(item) {
                        self.define_symbol(symbol_name.clone(), symbol.clone())?;
                    } else {
                        return Err(SemanticError { message: format!("Symbol '{}' not found in module '{}'", item, module) });
                    }
                }
                self.allow_builtin_override = prev_override;
                
                Ok(Statement::ImportFrom { module, items })
            },
            Statement::AssignMain { function_name } => {
                // Validate that the function exists
                if !self.symbol_exists(&function_name) {
                    return Err(SemanticError {
                        message: format!("Function '{}' not found for ASSIGN_MAIN", function_name),
                    });
                }
                
                // Validate that it's actually a function
                match self.get_symbol(&function_name)? {
                    Symbol::Function { .. } => {},
                    _ => {
                        return Err(SemanticError {
                            message: format!("'{}' is not a function and cannot be assigned as main", function_name),
                        });
                    }
                }
                
                Ok(Statement::AssignMain { function_name })
            },
            Statement::Break => {
                // Break statements are valid - they will be handled by the interpreter
                Ok(Statement::Break)
            },
            Statement::Continue => {
                // Continue statements are valid - they will be handled by the interpreter
                Ok(Statement::Continue)
            },
            Statement::Pick { expression, cases, default } => {
                let analyzed_expression = self.analyze_expr(*expression)?;
                let expr_type = self.infer_type(&analyzed_expression)?;

                let mut analyzed_cases = Vec::new();
                for case in cases {
                    let mut analyzed_values = Vec::new();
                    for value in case.values {
                        let analyzed_value = self.analyze_expr(value)?;
                        let value_type = self.infer_type(&analyzed_value)?;
                        if !self.are_types_compatible(&value_type, &expr_type) {
                            return Err(SemanticError {
                                message: format!(
                                    "Type mismatch in pick case: expression is {:?}, but case value is {:?}",
                                    expr_type, value_type
                                ),
                            });
                        }
                        analyzed_values.push(analyzed_value);
                    }

                    self.begin_scope();
                    let analyzed_body = Box::new(self.analyze_statement(*case.body)?);
                    self.end_scope();

                    analyzed_cases.push(WhenCase {
                        values: analyzed_values,
                        body: analyzed_body,
                    });
                }

                let analyzed_default = if let Some(default_body) = default {
                    self.begin_scope();
                    let analyzed_default_body = Box::new(self.analyze_statement(*default_body)?);
                    self.end_scope();
                    Some(analyzed_default_body)
                } else {
                    None
                };

                Ok(Statement::Pick {
                    expression: Box::new(analyzed_expression),
                    cases: analyzed_cases,
                    default: analyzed_default,
                })
            },
            Statement::RepeatUntil { body, condition } => {
                let analyzed_body = Box::new(self.analyze_statement(*body)?);
                let analyzed_condition = self.analyze_expr(*condition)?;
                
                // Check that condition is of boolean type
                if self.infer_type(&analyzed_condition)? != Type::Boolean {
                    return Err(SemanticError {
                        message: "Repeat-until condition must be of boolean type".to_string(),
                    });
                }
                
                Ok(Statement::RepeatUntil {
                    body: analyzed_body,
                    condition: Box::new(analyzed_condition),
                })
            },
            Statement::Loop { body } => {
                let analyzed_body = Box::new(self.analyze_statement(*body)?);
                
                Ok(Statement::Loop {
                    body: analyzed_body,
                })
            },
        }
    }
    
    fn analyze_expr(&mut self, expr: Expr) -> Result<Expr, SemanticError> {
        match expr {
            Expr::Variable(name) => {
                // Check if variable is declared
                if !self.symbol_exists(&name) {
                    return Err(SemanticError {
                        message: format!("Undefined variable: {}", name),
                    });
                }
                Ok(Expr::Variable(name))
            },
            Expr::Literal(literal) => {
                Ok(Expr::Literal(literal))
            },
            Expr::Binary { left, operator, right } => {
                let analyzed_left = Box::new(self.analyze_expr(*left)?);
                let analyzed_right = Box::new(self.analyze_expr(*right)?);
                
                // Use the improved type inference to validate the binary operation
                let binary_expr = Expr::Binary {
                    left: analyzed_left.clone(),
                    operator: operator.clone(),
                    right: analyzed_right.clone(),
                };
                
                // This will perform proper type checking and promotion
                self.infer_type(&binary_expr)?;
                
                Ok(Expr::Binary {
                    left: analyzed_left,
                    operator,
                    right: analyzed_right,
                })
            },
            Expr::Unary { operator, operand } => {
                let analyzed_operand = Box::new(self.analyze_expr(*operand)?);
                
                // Type checking for unary operations
                match operator {
                    crate::ast::UnaryOperator::Not => {
                        let operand_type = self.infer_type(&analyzed_operand)?;
                        if operand_type != Type::Boolean {
                            return Err(SemanticError {
                                message: "Operand of 'not' must be of boolean type".to_string(),
                            });
                        }
                    },
                    crate::ast::UnaryOperator::Negate => {
                        let operand_type = self.infer_type(&analyzed_operand)?;
                        if !(
                            operand_type == Type::Integer ||
                            operand_type == Type::I8 ||
                            operand_type == Type::I16 ||
                            operand_type == Type::I32 ||
                            operand_type == Type::I64 ||
                            operand_type == Type::ISize ||
                            operand_type == Type::U8 ||
                            operand_type == Type::U16 ||
                            operand_type == Type::U32 ||
                            operand_type == Type::U64 ||
                            operand_type == Type::USize ||
                            operand_type == Type::Float ||
                            operand_type == Type::F32 ||
                            operand_type == Type::F64
                        ) {
                            return Err(SemanticError {
                                message: "Operand of unary minus must be of numeric type".to_string(),
                            });
                        }
                    },
                    crate::ast::UnaryOperator::BitNot => {
                        let operand_type = self.infer_type(&analyzed_operand)?;
                        if !(
                            operand_type == Type::Unknown ||
                            operand_type == Type::Integer || operand_type == Type::I8 || operand_type == Type::I16 || operand_type == Type::I32 || operand_type == Type::I64 || operand_type == Type::ISize || operand_type == Type::U8 || operand_type == Type::U16 || operand_type == Type::U32 || operand_type == Type::U64 || operand_type == Type::USize
                        ) {
                            return Err(SemanticError { message: "Operand of bitwise NOT must be integer".to_string() });
                        }
                    },
                }
                
                Ok(Expr::Unary {
                    operator,
                    operand: analyzed_operand,
                })
            },
            Expr::Borrow { target, mutable } => {
                let analyzed_target = Box::new(self.analyze_expr(*target)?);
                Ok(Expr::Borrow { target: analyzed_target, mutable })
            },
            Expr::Call { callee, arguments } => {
                let mut analyzed_arguments = Vec::new();
                for arg in arguments {
                    analyzed_arguments.push(self.analyze_expr(arg)?);
                }
                // Fast-path: allow pool/tree 'add' methods without namespace conversion
                if let Expr::Get { object, name } = callee.as_ref() {
                    if name == "add" {
                        let analyzed_object = self.analyze_expr(*object.clone())?;
                        return Ok(Expr::Call { callee: Box::new(Expr::Get { object: Box::new(analyzed_object), name: name.clone() }), arguments: analyzed_arguments });
                    }
                }
                let func_name = match callee.as_ref() {
                    Expr::Variable(name) => name.clone(),
                    Expr::Get { object, name } => {
                        let analyzed_object = self.analyze_expr(*object.clone())?;
                        let obj_type = self.infer_type(&analyzed_object)?;
                        match obj_type {
                            Type::String => {
                                match name.as_str() {
                                    "upper" | "lower" | "trim" => {
                                        if !analyzed_arguments.is_empty() {
                                            return Err(SemanticError { message: format!("Method '{}' on string takes 0 arguments", name) });
                                        }
                                        return Ok(Expr::Call { callee: Box::new(Expr::Get { object: Box::new(analyzed_object), name: name.clone() }), arguments: analyzed_arguments });
                                    }
                                    "contains" => {
                                        if analyzed_arguments.len() != 1 {
                                            return Err(SemanticError { message: "String.contains() expects 1 argument".to_string() });
                                        }
                                        let arg_t = self.infer_type(&analyzed_arguments[0])?;
                                        if arg_t != Type::String {
                                            return Err(SemanticError { message: "String.contains() argument must be string".to_string() });
                                        }
                                        return Ok(Expr::Call { callee: Box::new(Expr::Get { object: Box::new(analyzed_object), name: name.clone() }), arguments: analyzed_arguments });
                                    }
                                    "split" => {
                                        if analyzed_arguments.len() != 1 { return Err(SemanticError { message: "String.split(delim) expects 1 argument".to_string() }); }
                                        let arg_t = self.infer_type(&analyzed_arguments[0])?;
                                        if arg_t != Type::String { return Err(SemanticError { message: "String.split() delimiter must be string".to_string() }); }
                                        return Ok(Expr::Call { callee: Box::new(Expr::Get { object: Box::new(analyzed_object), name: name.clone() }), arguments: analyzed_arguments });
                                    }
                                    "replace" => {
                                        if analyzed_arguments.len() != 2 { return Err(SemanticError { message: "String.replace(from,to) expects 2 arguments".to_string() }); }
                                        for a in &analyzed_arguments { if self.infer_type(a)? != Type::String { return Err(SemanticError { message: "String.replace() args must be strings".to_string() }); } }
                                        return Ok(Expr::Call { callee: Box::new(Expr::Get { object: Box::new(analyzed_object), name: name.clone() }), arguments: analyzed_arguments });
                                    }
                                    "substring" => {
                                        if analyzed_arguments.len() != 2 { return Err(SemanticError { message: "String.substring(start,end) expects 2 arguments".to_string() }); }
                                        // allow integer types
                                        let t0 = self.infer_type(&analyzed_arguments[0])?; let t1 = self.infer_type(&analyzed_arguments[1])?;
                                        let ok_int = |t: &Type| matches!(t, Type::Integer | Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::ISize | Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::USize);
                                        if !ok_int(&t0) || !ok_int(&t1) { return Err(SemanticError { message: "substring indices must be integer types".to_string() }); }
                                        return Ok(Expr::Call { callee: Box::new(Expr::Get { object: Box::new(analyzed_object), name: name.clone() }), arguments: analyzed_arguments });
                                    }
                                    "regex" => {
                                        if analyzed_arguments.len() != 1 { return Err(SemanticError { message: "String.regex(pattern) expects 1 argument".to_string() }); }
                                        if self.infer_type(&analyzed_arguments[0])? != Type::String { return Err(SemanticError { message: "regex pattern must be string".to_string() }); }
                                        return Ok(Expr::Call { callee: Box::new(Expr::Get { object: Box::new(analyzed_object), name: name.clone() }), arguments: analyzed_arguments });
                                    }
                                    _ => {
                                        return Err(SemanticError { message: format!("Unknown string method: {}", name) });
                                    }
                                }
                            }
                            Type::Array(_, _) => {
                                match name.as_str() {
                                    "len" => {
                                        if !analyzed_arguments.is_empty() {
                                            return Err(SemanticError { message: "Array.len() takes 0 arguments".to_string() });
                                        }
                                        return Ok(Expr::Call { callee: Box::new(Expr::Get { object: Box::new(analyzed_object), name: name.clone() }), arguments: analyzed_arguments });
                                    }
                                    "join" => {
                                        if analyzed_arguments.len() != 1 { return Err(SemanticError { message: "Array.join(delim) expects 1 argument".to_string() }); }
                                        if self.infer_type(&analyzed_arguments[0])? != Type::String { return Err(SemanticError { message: "join delimiter must be string".to_string() }); }
                                        return Ok(Expr::Call { callee: Box::new(Expr::Get { object: Box::new(analyzed_object), name: name.clone() }), arguments: analyzed_arguments });
                                    }
                                    _ => {
                                        return Err(SemanticError { message: format!("Unknown array method: {}", name) });
                                    }
                                }
                            }
                            Type::Pool(_) => {
                                match name.as_str() {
                                    "add" => {
                                        if analyzed_arguments.len() != 1 { return Err(SemanticError { message: "Pool.add() expects 1 argument".to_string() }); }
                                        return Ok(Expr::Call { callee: Box::new(Expr::Get { object: Box::new(analyzed_object), name: name.clone() }), arguments: analyzed_arguments });
                                    }
                                    _ => return Err(SemanticError { message: format!("Unknown pool method: {}", name) }),
                                }
                            }
                            Type::Tree(_) => {
                                match name.as_str() {
                                    "add" => {
                                        if analyzed_arguments.len() != 1 { return Err(SemanticError { message: "Tree.add() expects 1 argument".to_string() }); }
                                        return Ok(Expr::Call { callee: Box::new(Expr::Get { object: Box::new(analyzed_object), name: name.clone() }), arguments: analyzed_arguments });
                                    }
                                    _ => return Err(SemanticError { message: format!("Unknown tree method: {}", name) }),
                                }
                            }
                            _ => {
                                // Allow method call to be validated at runtime for other types
                                return Ok(Expr::Call { callee: Box::new(Expr::Get { object: Box::new(analyzed_object), name: name.clone() }), arguments: analyzed_arguments });
                            }
                        }
                    }
                    _ => {
                        return Err(SemanticError { message: "Complex function calls not yet supported".to_string() });
                    }
                };

                // Require std import for core std functions
                let requires_std = matches!(func_name.as_str(),
                    "print" | "println" | "len" | "int" | "float" | "str" | "sha256" | "sha256_random" | "add" | "multiply"
                );
                if requires_std && !self.std_imported {
                    return Err(SemanticError { message: format!("Using std function '{}' without 'import std'", func_name) });
                }

                // Prefer user-defined/imported functions over built-ins
                if let Ok(Symbol::Function { .. }) = self.get_symbol(&func_name) {
                    return Ok(Expr::Call { callee: Box::new(Expr::Variable(func_name)), arguments: analyzed_arguments });
                }

                if func_name == "vault" || func_name == "pool" || func_name == "tree" {
                    return Ok(Expr::Call { callee: Box::new(Expr::Variable(func_name)), arguments: analyzed_arguments });
                }

                if func_name == "print" || func_name == "println" {
                    // These are variadic, so we don't check argument count or types.
                } else if func_name == "len" {
                    if analyzed_arguments.len() != 1 {
                        return Err(SemanticError {
                            message: "len() function expects exactly one argument".to_string(),
                        });
                    }
                    let arg_type = self.infer_type(&analyzed_arguments[0])?;
                    match arg_type {
                        Type::String | Type::Array(_, _) | Type::Unknown => {
                            // Valid or deferred to runtime
                        }
                        _ => {
                            return Err(SemanticError {
                                message: format!("len() function cannot be called on type {:?}", arg_type),
                            });
                        }
                    }
                } else if func_name == "sha256" {
                    if analyzed_arguments.len() != 1 {
                        return Err(SemanticError { message: "sha256() expects exactly one argument".to_string() });
                    }
                    let arg_t = self.infer_type(&analyzed_arguments[0])?;
                    match arg_t {
                        Type::String | Type::Array(_, _) => {},
                        _ => {
                            return Err(SemanticError { message: format!("sha256() cannot be called on type {:?}", arg_t) });
                        }
                    }
                } else if self.std_lib.is_builtin_function(&func_name) {
                    // Check if it's a built-in function first
                    // Get argument types for overload resolution
                    let mut arg_types = Vec::new();
                    for arg in &analyzed_arguments {
                        arg_types.push(self.infer_type(arg)?);
                    }

                    // Try to find function with matching signature
                    if self.std_lib.get_builtin_function_by_signature(&func_name, &arg_types).is_some() {
                        // Found exact match - no need for further type checking
                    } else {
                        // No exact match found - try the old method for backward compatibility
                        if let Some(builtin_func) = self.std_lib.get_builtin_function(&func_name) {
                            if analyzed_arguments.len() != builtin_func.parameters.len() {
                                return Err(SemanticError {
                                    message: format!(
                                        "Built-in function '{}' expects {} arguments, but {} were provided",
                                        func_name,
                                        builtin_func.parameters.len(),
                                        analyzed_arguments.len()
                                    ),
                                });
                            }

                        for (i, arg) in analyzed_arguments.iter().enumerate() {
                            let mut arg_type = self.infer_type(arg)?;
                            let param_type = &builtin_func.parameters[i];

                                // Special case for println and print - they can accept any type
                                if func_name == "println" || func_name == "print" {
                                    // Skip type checking for println and print - they handle conversion internally
                                    continue;
                                }

                                // Special handling for Void types - they might be function calls whose return types
                                // haven't been inferred yet. Use type inference to resolve them.
                                if arg_type == Type::Void {
                                    let inferred_arg_type = self.infer_type(arg)?;
                                    if inferred_arg_type != Type::Void {
                                        arg_type = inferred_arg_type;
                                    } else {
                                        continue;
                                    }
                                }

                                if arg_type == Type::Unknown {
                                    continue;
                                }
                                if arg_type != *param_type {
                                    return Err(SemanticError {
                                        message: format!(
                                            "Type mismatch in argument {} of built-in function '{}': expected {:?}, got {:?}",
                                            i + 1,
                                            func_name,
                                            param_type,
                                            arg_type
                                        ),
                                    });
                                }
                            }
                        } else {
                            return Err(SemanticError {
                                message: format!("No matching overload found for built-in function '{}'", func_name),
                            });
                        }
                    }
                } else {
                    // Check user-defined functions
                    if let Ok(Symbol::Function { parameters, .. }) = self.get_symbol(&func_name) {
                        if analyzed_arguments.len() != parameters.len() {
                            return Err(SemanticError {
                                message: format!(
                                    "Function '{}' expects {} arguments, but {} were provided",
                                    func_name,
                                    parameters.len(),
                                    analyzed_arguments.len()
                                ),
                            });
                        }

                        for (i, arg) in analyzed_arguments.iter().enumerate() {
                            let arg_type = self.infer_type(arg)?;
                            let param_type = &parameters[i].param_type.as_ref().unwrap_or(&Type::Unknown);
                            if !self.are_types_compatible(&arg_type, param_type) {
                                return Err(SemanticError {
                                    message: format!(
                                        "Type mismatch in argument {} of function '{}': expected {:?}, got {:?}",
                                        i + 1,
                                        func_name,
                                        param_type,
                                        arg_type
                                    ),
                                });
                            }
                        }
                    } else {
                        return Err(SemanticError {
                            message: format!("Undefined function '{}'", func_name),
                        });
                    }
                }

                Ok(Expr::Call {
                    callee: Box::new(Expr::Variable(func_name)),
                    arguments: analyzed_arguments,
                })
            },
            Expr::Function { parameters, body, return_type } => {
                // Enter function scope
                self.begin_scope();
                
                // Add parameters to the scope
                for param in &parameters {
                    let param_type = param.param_type.clone().unwrap_or(Type::Unknown);
                    self.define_symbol(
                        param.name.clone(),
                        Symbol::Variable { var_type: param_type, is_mutable: false }
                    )?;
                }
                
                // Analyze function body
                let mut analyzed_body = Vec::new();
                for stmt in body {
                    analyzed_body.push(self.analyze_statement(stmt)?);
                }
                
                // Exit function scope
                self.end_scope();
                
                Ok(Expr::Function {
                    parameters,
                    body: analyzed_body,
                    return_type,
                })
            },
            Expr::Get { object, name } => {
                // Handle module-qualified access (e.g., math_utils.multiply)
                if let Expr::Variable(module_name) = object.as_ref() {
                    // First, check if the module_name is a valid namespace
                    if let Ok(Symbol::Namespace { .. }) = self.get_symbol(module_name) {
                        let qualified_name = format!("{}.{}", module_name, name);
                        if self.symbol_exists(&qualified_name) {
                            // Convert the Get expression to a Variable expression with the qualified name
                            return Ok(Expr::Variable(qualified_name));
                        } else {
                            return Err(SemanticError {
                                message: format!("Symbol '{}' not found in namespace '{}'", name, module_name),
                            });
                        }
                    }
                }
                
                // For other cases, analyze the object normally
                let analyzed_object = Box::new(self.analyze_expr(*object)?);
                Ok(Expr::Get { object: analyzed_object, name })
            },
            Expr::Set { object, name, value } => {
                let analyzed_object = Box::new(self.analyze_expr(*object)?);
                let analyzed_value = Box::new(self.analyze_expr(*value)?);
                Ok(Expr::Set { object: analyzed_object, name, value: analyzed_value })
            },
            Expr::Index { sequence, index } => {
                let analyzed_sequence = Box::new(self.analyze_expr(*sequence)?);
                let analyzed_index = Box::new(self.analyze_expr(*index)?);
                Ok(Expr::Index { sequence: analyzed_sequence, index: analyzed_index })
            },
            Expr::Assign { name, value } => {
                // Check if variable is declared
                if !self.symbol_exists(&name) {
                    return Err(SemanticError {
                        message: format!("Cannot assign to undeclared variable: {}", name),
                    });
                }
                
                let analyzed_value = Box::new(self.analyze_expr(*value)?);
                
                // Type checking for assignment
                let var_symbol = self.get_symbol(&name)?;
                if let Symbol::Variable { var_type, is_mutable } = var_symbol {
                    if !is_mutable {
                        return Err(SemanticError { message: format!("Cannot assign to immutable variable '{}'", name) });
                    }
                    let value_type = self.infer_type(&analyzed_value)?;
                    if !self.are_types_compatible(&var_type, &value_type) {
                        return Err(SemanticError {
                            message: format!("Type mismatch in assignment: expected {:?}, got {:?}", var_type, value_type),
                        });
                    }
                    
                    // Validate literal bounds for integer types
                    if let Expr::Literal(literal) = analyzed_value.as_ref() {
                        self.validate_literal_bounds(literal, &var_type)?;
                    }
                }
                
                Ok(Expr::Assign { name, value: analyzed_value })
            },
            Expr::AssignIndex { sequence, index, value } => {
                // Analyze the sequence, index, and value expressions
                let analyzed_sequence = Box::new(self.analyze_expr(*sequence)?);
                let analyzed_index = Box::new(self.analyze_expr(*index)?);
                let analyzed_value = Box::new(self.analyze_expr(*value)?);
                
                let sequence_type = self.infer_type(&analyzed_sequence)?;
                match sequence_type {
                    Type::Array(element_type, _) => {
                        // For indexed assignment into arrays, require sequence to be a mutable variable
                        if let Expr::Variable(seq_name) = analyzed_sequence.as_ref() {
                            if let Ok(Symbol::Variable { is_mutable, .. }) = self.get_symbol(seq_name) {
                                if !is_mutable {
                                    return Err(SemanticError { message: format!("Cannot mutate immutable variable '{}' via index assignment", seq_name) });
                                }
                            }
                        }
                        let index_type = self.infer_type(&analyzed_index)?;
                        if !matches!(index_type, Type::Integer | Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::ISize | Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::USize) {
                            return Err(SemanticError { message: format!("Array index must be integer type, got: {:?}", index_type) });
                        }
                        let value_type = self.infer_type(&analyzed_value)?;
                        if !self.are_types_compatible(&element_type, &value_type) {
                            return Err(SemanticError { message: format!("Type mismatch in array assignment: expected {:?}, got {:?}", element_type, value_type) });
                        }
                        if let Expr::Literal(literal) = analyzed_value.as_ref() { self.validate_literal_bounds(literal, &element_type)?; }
                        Ok(Expr::AssignIndex { sequence: analyzed_sequence, index: analyzed_index, value: analyzed_value })
                    }
                    Type::Vault(_, val_t) => {
                        let index_type = self.infer_type(&analyzed_index)?;
                        if index_type != Type::String { return Err(SemanticError { message: "Vault key must be string".to_string() }); }
                        let value_type = self.infer_type(&analyzed_value)?;
                        if !self.are_types_compatible(&val_t, &value_type) {
                            return Err(SemanticError { message: format!("Type mismatch in vault assignment: expected {:?}, got {:?}", val_t, value_type) });
                        }
                        Ok(Expr::AssignIndex { sequence: analyzed_sequence, index: analyzed_index, value: analyzed_value })
                    }
                    Type::Unknown => {
                        let index_type = self.infer_type(&analyzed_index)?;
                        if !matches!(index_type, Type::Integer | Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::ISize | Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::USize) {
                            return Err(SemanticError { message: format!("Array index must be integer type, got: {:?}", index_type) });
                        }
                        Ok(Expr::AssignIndex { sequence: analyzed_sequence, index: analyzed_index, value: analyzed_value })
                    }
                    _ => Err(SemanticError { message: format!("Cannot index into non-array type: {:?}", sequence_type) }),
                }
            },
            Expr::ArrayLiteral { elements } => {
                // Check array literal bounds (max 1024 elements)
                if elements.len() > 1024 {
                    return Err(SemanticError {
                        message: format!("Array literal too large ({} elements), maximum allowed is 1024", elements.len()),
                    });
                }
                
                // Analyze each element in the array literal
                let mut analyzed_elements = Vec::new();
                for element in elements {
                    analyzed_elements.push(self.analyze_expr(element)?);
                }
                
                // Use type inference to validate that all elements have the same type
                self.infer_type(&Expr::ArrayLiteral { elements: analyzed_elements.clone() })?;
                
                Ok(Expr::ArrayLiteral { elements: analyzed_elements })
            },
            Expr::VaultLiteral { entries } => {
                let mut analyzed: Vec<(String, Expr)> = Vec::new();
                for (k, v) in entries {
                    let v_an = self.analyze_expr(v)?;
                    analyzed.push((k, v_an));
                }
                Ok(Expr::VaultLiteral { entries: analyzed })
            },
            Expr::PoolLiteral { elements } => {
                let mut analyzed_elements = Vec::new();
                for e in elements {
                    analyzed_elements.push(self.analyze_expr(e)?);
                }
                Ok(Expr::PoolLiteral { elements: analyzed_elements })
            },
            Expr::TreeLiteral { root, children } => {
                let root_an = Box::new(self.analyze_expr(*root)?);
                let mut ch_an = Vec::new();
                for c in children {
                    ch_an.push(self.analyze_expr(c)?);
                }
                Ok(Expr::TreeLiteral { root: root_an, children: ch_an })
            },
            // Advanced expression types - not yet fully implemented
            Expr::Tuple { elements } => {
                let mut analyzed_elements = Vec::new();
                for element in elements {
                    analyzed_elements.push(self.analyze_expr(element)?);
                }
                Ok(Expr::Tuple { elements: analyzed_elements })
            },
            Expr::IfExpression { condition, then_branch, else_branch } => {
                let analyzed_condition = self.analyze_expr(*condition)?;
                let analyzed_then = self.analyze_expr(*then_branch)?;
                let analyzed_else = self.analyze_expr(*else_branch)?;

                // Condition must be boolean
                let condition_type = self.infer_type(&analyzed_condition)?;
                if condition_type != Type::Boolean {
                    return Err(SemanticError {
                        message: format!("If condition must be boolean, got {:?}", condition_type),
                    });
                }

                Ok(Expr::IfExpression {
                    condition: Box::new(analyzed_condition),
                    then_branch: Box::new(analyzed_then),
                    else_branch: Box::new(analyzed_else),
                })
            },
            Expr::Match { expression, cases } => {
                let analyzed_expression = self.analyze_expr(*expression)?;
                let mut analyzed_cases = Vec::new();

                for case in cases {
                    // Analyze pattern and body expression
                    let analyzed_body = self.analyze_expr(*case.body)?;
                    let analyzed_pattern = self.analyze_pattern(&case.pattern)?;
                    analyzed_cases.push(MatchCase {
                        pattern: analyzed_pattern,
                        body: Box::new(analyzed_body),
                    });
                }

                Ok(Expr::Match {
                    expression: Box::new(analyzed_expression),
                    cases: analyzed_cases,
                })
            },
        }
    }
    
    fn infer_type(&self, expr: &Expr) -> Result<Type, SemanticError> {
        match expr {
            Expr::Literal(literal) => {
                match literal {
                    Literal::Integer(_) => Ok(Type::Integer),
                    Literal::I8(_) => Ok(Type::I8),
                    Literal::I16(_) => Ok(Type::I16),
                    Literal::I32(_) => Ok(Type::I32),
                    Literal::I64(_) => Ok(Type::I64),
                    Literal::ISize(_) => Ok(Type::ISize),
                    Literal::U8(_) => Ok(Type::U8),
                    Literal::U16(_) => Ok(Type::U16),
                    Literal::U32(_) => Ok(Type::U32),
                    Literal::U64(_) => Ok(Type::U64),
                    Literal::USize(_) => Ok(Type::USize),
                    // Float literals default to f64
                    Literal::Float(_) => Ok(Type::F64),
                    Literal::Boolean(_) => Ok(Type::Boolean),
                    Literal::String(_) => Ok(Type::String),
                    Literal::Null => Ok(Type::Void), // Null literals have void type
                }
            },
            Expr::Variable(name) => {
                match self.get_symbol(name)? {
                    Symbol::Variable { var_type, .. } => Ok(var_type),
                    Symbol::Function { .. } => Err(SemanticError {
                        message: format!("Expected variable, found function: {}", name),
                    }),
                    Symbol::Namespace { .. } => Ok(Type::Unknown),
                }
            },
            Expr::Binary { left, operator, right } => {
                let left_type = self.infer_type(left)?;
                let right_type = self.infer_type(right)?;
                
                match operator {
                    BinaryOperator::Plus => {
                        // Plus operator: supports both arithmetic and string concatenation
                        if left_type == Type::String && right_type == Type::String {
                            Ok(Type::String)
                        } else if left_type == Type::Float || right_type == Type::Float ||
                                  left_type == Type::F32 || right_type == Type::F32 ||
                                  left_type == Type::F64 || right_type == Type::F64 {
                            if (left_type == Type::Unknown || left_type == Type::Integer || left_type == Type::I8 || left_type == Type::I16 || left_type == Type::I32 || left_type == Type::I64 || left_type == Type::ISize || left_type == Type::U8 || left_type == Type::U16 || left_type == Type::U32 || left_type == Type::U64 || left_type == Type::USize || left_type == Type::Float || left_type == Type::F32 || left_type == Type::F64) &&
                               (right_type == Type::Unknown || right_type == Type::Integer || right_type == Type::I8 || right_type == Type::I16 || right_type == Type::I32 || right_type == Type::I64 || right_type == Type::ISize || right_type == Type::U8 || right_type == Type::U16 || right_type == Type::U32 || right_type == Type::U64 || right_type == Type::USize || right_type == Type::Float || right_type == Type::F32 || right_type == Type::F64) {
                                Ok(Type::F64)
                            } else {
                                Err(SemanticError {
                                    message: format!("Cannot perform arithmetic on {:?} and {:?}", left_type, right_type),
                                })
                            }
                        } else if (left_type == Type::Unknown || left_type == Type::Integer || left_type == Type::I8 || left_type == Type::I16 || left_type == Type::I32 || left_type == Type::I64 || left_type == Type::ISize || left_type == Type::U8 || left_type == Type::U16 || left_type == Type::U32 || left_type == Type::U64 || left_type == Type::USize) && 
                                  (right_type == Type::Unknown || right_type == Type::Integer || right_type == Type::I8 || right_type == Type::I16 || right_type == Type::I32 || right_type == Type::I64 || right_type == Type::ISize || right_type == Type::U8 || right_type == Type::U16 || right_type == Type::U32 || right_type == Type::U64 || right_type == Type::USize) {
                            // For integer types, preserve the more specific type if both are the same
                            if left_type == right_type {
                                Ok(left_type)
                            } else {
                                Ok(Type::Integer)
                            }
                        } else {
                            Err(SemanticError {
                                message: format!("Cannot perform arithmetic on {:?} and {:?}", left_type, right_type),
                            })
                        }
                    },
                    BinaryOperator::Minus | BinaryOperator::Star => {
                        // Arithmetic operations: promote to float if either operand is float
                        if left_type == Type::Float || right_type == Type::Float ||
                           left_type == Type::F32 || right_type == Type::F32 ||
                           left_type == Type::F64 || right_type == Type::F64 {
                            if (left_type == Type::Unknown || left_type == Type::Integer || left_type == Type::I8 || left_type == Type::I16 || left_type == Type::I32 || left_type == Type::I64 || left_type == Type::ISize || left_type == Type::U8 || left_type == Type::U16 || left_type == Type::U32 || left_type == Type::U64 || left_type == Type::USize || left_type == Type::Float || left_type == Type::F32 || left_type == Type::F64) &&
                               (right_type == Type::Unknown || right_type == Type::Integer || right_type == Type::I8 || right_type == Type::I16 || right_type == Type::I32 || right_type == Type::I64 || right_type == Type::ISize || right_type == Type::U8 || right_type == Type::U16 || right_type == Type::U32 || right_type == Type::U64 || right_type == Type::USize || right_type == Type::Float || right_type == Type::F32 || right_type == Type::F64) {
                                Ok(Type::F64)
                            } else {
                                Err(SemanticError {
                                    message: format!("Cannot perform arithmetic on {:?} and {:?}", left_type, right_type),
                                })
                            }
                        } else if (left_type == Type::Unknown || left_type == Type::Integer || left_type == Type::I8 || left_type == Type::I16 || left_type == Type::I32 || left_type == Type::I64 || left_type == Type::ISize || left_type == Type::U8 || left_type == Type::U16 || left_type == Type::U32 || left_type == Type::U64 || left_type == Type::USize) && 
                                  (right_type == Type::Unknown || right_type == Type::Integer || right_type == Type::I8 || right_type == Type::I16 || right_type == Type::I32 || right_type == Type::I64 || right_type == Type::ISize || right_type == Type::U8 || right_type == Type::U16 || right_type == Type::U32 || right_type == Type::U64 || right_type == Type::USize) {
                            // For integer types, preserve the more specific type if both are the same
                            if left_type == right_type {
                                Ok(left_type)
                            } else {
                                Ok(Type::Integer)
                            }
                        } else {
                            Err(SemanticError {
                                message: format!("Cannot perform arithmetic on {:?} and {:?}", left_type, right_type),
                            })
                        }
                    },
                    BinaryOperator::Slash => {
                        // Division always returns float, even for integer operands
                        if (left_type == Type::Unknown || left_type == Type::Integer || left_type == Type::I8 || left_type == Type::I16 || left_type == Type::I32 || left_type == Type::I64 || left_type == Type::ISize || left_type == Type::U8 || left_type == Type::U16 || left_type == Type::U32 || left_type == Type::U64 || left_type == Type::USize || left_type == Type::Float || left_type == Type::F32 || left_type == Type::F64) &&
                           (right_type == Type::Unknown || right_type == Type::Integer || right_type == Type::I8 || right_type == Type::I16 || right_type == Type::I32 || right_type == Type::I64 || right_type == Type::ISize || right_type == Type::U8 || right_type == Type::U16 || right_type == Type::U32 || right_type == Type::U64 || right_type == Type::USize || right_type == Type::Float || right_type == Type::F32 || right_type == Type::F64) {
                            Ok(Type::F64)
                        } else {
                            Err(SemanticError {
                                message: format!("Cannot perform division on {:?} and {:?}", left_type, right_type),
                            })
                        }
                    },
                    BinaryOperator::Percent => {
                        // Modulo only works with integers
                        if (left_type == Type::Unknown || left_type == Type::Integer || left_type == Type::I8 || left_type == Type::I16 || left_type == Type::I32 || left_type == Type::I64 || left_type == Type::ISize || left_type == Type::U8 || left_type == Type::U16 || left_type == Type::U32 || left_type == Type::U64 || left_type == Type::USize) && 
                           (right_type == Type::Unknown || right_type == Type::Integer || right_type == Type::I8 || right_type == Type::I16 || right_type == Type::I32 || right_type == Type::I64 || right_type == Type::ISize || right_type == Type::U8 || right_type == Type::U16 || right_type == Type::U32 || right_type == Type::U64 || right_type == Type::USize) {
                            // For integer types, preserve the more specific type if both are the same
                            if left_type == right_type {
                                Ok(left_type)
                            } else {
                                Ok(Type::Integer)
                            }
                        } else {
                            Err(SemanticError {
                                message: "Modulo operator requires integer operands".to_string(),
                            })
                        }
                    },
                    BinaryOperator::EqualEqual | BinaryOperator::NotEqual => {
                        // Equality comparison: operands must be compatible types
                        if left_type == Type::Unknown || right_type == Type::Unknown {
                            return Ok(Type::Boolean);
                        }
                        if left_type == right_type || 
                           (left_type == Type::Integer && right_type == Type::Float) ||
                           (left_type == Type::Float && right_type == Type::Integer) ||
                           (left_type == Type::Integer && (right_type == Type::F32 || right_type == Type::F64)) ||
                           ((left_type == Type::F32 || left_type == Type::F64) && right_type == Type::Integer) ||
                           // Cross-float width comparisons
                           (left_type == Type::F32 && right_type == Type::F64) ||
                           (left_type == Type::F64 && right_type == Type::F32) ||
                           (left_type == Type::Float && (right_type == Type::F32 || right_type == Type::F64)) ||
                           ((left_type == Type::F32 || left_type == Type::F64) && right_type == Type::Float) ||
                           (left_type == Type::Integer && right_type == Type::I8) ||
                           (left_type == Type::I8 && right_type == Type::Integer) ||
                           (left_type == Type::Integer && right_type == Type::I16) ||
                           (left_type == Type::I16 && right_type == Type::Integer) ||
                           (left_type == Type::Integer && right_type == Type::I32) ||
                           (left_type == Type::I32 && right_type == Type::Integer) ||
                           (left_type == Type::Integer && right_type == Type::I64) ||
                           (left_type == Type::I64 && right_type == Type::Integer) ||
                           (left_type == Type::Integer && right_type == Type::ISize) ||
                           (left_type == Type::ISize && right_type == Type::Integer) ||
                           (left_type == Type::Integer && right_type == Type::U8) ||
                           (left_type == Type::U8 && right_type == Type::Integer) ||
                           (left_type == Type::Integer && right_type == Type::U16) ||
                           (left_type == Type::U16 && right_type == Type::Integer) ||
                           (left_type == Type::Integer && right_type == Type::U32) ||
                           (left_type == Type::U32 && right_type == Type::Integer) ||
                           (left_type == Type::Integer && right_type == Type::U64) ||
                           (left_type == Type::U64 && right_type == Type::Integer) ||
                           (left_type == Type::Integer && right_type == Type::USize) ||
                           (left_type == Type::USize && right_type == Type::Integer) ||
                           (left_type == Type::I8 && (right_type == Type::Float || right_type == Type::F32 || right_type == Type::F64)) ||
                           ((left_type == Type::Float || left_type == Type::F32 || left_type == Type::F64) && right_type == Type::I8) ||
                           (left_type == Type::I16 && (right_type == Type::Float || right_type == Type::F32 || right_type == Type::F64)) ||
                           ((left_type == Type::Float || left_type == Type::F32 || left_type == Type::F64) && right_type == Type::I16) ||
                           (left_type == Type::I32 && (right_type == Type::Float || right_type == Type::F32 || right_type == Type::F64)) ||
                           ((left_type == Type::Float || left_type == Type::F32 || left_type == Type::F64) && right_type == Type::I32) ||
                           (left_type == Type::I64 && (right_type == Type::Float || right_type == Type::F32 || right_type == Type::F64)) ||
                           ((left_type == Type::Float || left_type == Type::F32 || left_type == Type::F64) && right_type == Type::I64) ||
                           (left_type == Type::ISize && (right_type == Type::Float || right_type == Type::F32 || right_type == Type::F64)) ||
                           ((left_type == Type::Float || left_type == Type::F32 || left_type == Type::F64) && right_type == Type::ISize) ||
                           (left_type == Type::U8 && (right_type == Type::Float || right_type == Type::F32 || right_type == Type::F64)) ||
                           ((left_type == Type::Float || left_type == Type::F32 || left_type == Type::F64) && right_type == Type::U8) ||
                           (left_type == Type::U16 && (right_type == Type::Float || right_type == Type::F32 || right_type == Type::F64)) ||
                           ((left_type == Type::Float || left_type == Type::F32 || left_type == Type::F64) && right_type == Type::U16) ||
                           (left_type == Type::U32 && (right_type == Type::Float || right_type == Type::F32 || right_type == Type::F64)) ||
                           ((left_type == Type::Float || left_type == Type::F32 || left_type == Type::F64) && right_type == Type::U32) ||
                           (left_type == Type::U64 && (right_type == Type::Float || right_type == Type::F32 || right_type == Type::F64)) ||
                           ((left_type == Type::Float || left_type == Type::F32 || left_type == Type::F64) && right_type == Type::U64) ||
                           (left_type == Type::USize && (right_type == Type::Float || right_type == Type::F32 || right_type == Type::F64)) ||
                           ((left_type == Type::Float || left_type == Type::F32 || left_type == Type::F64) && right_type == Type::USize) {
                            Ok(Type::Boolean)
                        } else {
                            Err(SemanticError {
                                message: format!("Cannot compare {:?} and {:?}", left_type, right_type),
                            })
                        }
                    },
                    BinaryOperator::Less | BinaryOperator::LessEqual | 
                    BinaryOperator::Greater | BinaryOperator::GreaterEqual => {
                        // Relational comparison: only numeric types
                        if (
                            left_type == Type::Unknown || left_type == Type::Integer || left_type == Type::I8 || left_type == Type::I16 || left_type == Type::I32 || left_type == Type::I64 || left_type == Type::ISize || left_type == Type::U8 || left_type == Type::U16 || left_type == Type::U32 || left_type == Type::U64 || left_type == Type::USize || left_type == Type::Float || left_type == Type::F32 || left_type == Type::F64
                        ) && (
                            right_type == Type::Unknown || right_type == Type::Integer || right_type == Type::I8 || right_type == Type::I16 || right_type == Type::I32 || right_type == Type::I64 || right_type == Type::ISize || right_type == Type::U8 || right_type == Type::U16 || right_type == Type::U32 || right_type == Type::U64 || right_type == Type::USize || right_type == Type::Float || right_type == Type::F32 || right_type == Type::F64
                        ) {
                            Ok(Type::Boolean)
                        } else {
                            Err(SemanticError {
                                message: format!("Cannot compare {:?} and {:?}", left_type, right_type),
                            })
                        }
                    },
                    BinaryOperator::And | BinaryOperator::Or => {
                        // Logical operators require boolean operands
                        if left_type == Type::Boolean && right_type == Type::Boolean {
                            Ok(Type::Boolean)
                        } else {
                            Err(SemanticError {
                                message: "Logical operators require boolean operands".to_string(),
                            })
                        }
                    },
                    BinaryOperator::BitAnd | BinaryOperator::BitOr | BinaryOperator::BitXor | BinaryOperator::ShiftLeft | BinaryOperator::ShiftRight => {
                        // Bitwise operations require integer-like operands and return integer
                        if (
                            left_type == Type::Unknown || left_type == Type::Integer || left_type == Type::I8 || left_type == Type::I16 || left_type == Type::I32 || left_type == Type::I64 || left_type == Type::ISize || left_type == Type::U8 || left_type == Type::U16 || left_type == Type::U32 || left_type == Type::U64 || left_type == Type::USize
                        ) && (
                            right_type == Type::Unknown || right_type == Type::Integer || right_type == Type::I8 || right_type == Type::I16 || right_type == Type::I32 || right_type == Type::I64 || right_type == Type::ISize || right_type == Type::U8 || right_type == Type::U16 || right_type == Type::U32 || right_type == Type::U64 || right_type == Type::USize
                        ) {
                            Ok(Type::Integer)
                        } else {
                            Err(SemanticError { message: "Bitwise operations require integer operands".to_string() })
                        }
                    },
                }
            },
            Expr::Unary { operator, operand } => {
                let operand_type = self.infer_type(operand)?;
                match operator {
                    crate::ast::UnaryOperator::Not => {
                        if operand_type == Type::Boolean {
                            Ok(Type::Boolean)
                        } else {
                            Err(SemanticError {
                                message: "Logical NOT requires boolean operand".to_string(),
                            })
                        }
                    },
                    crate::ast::UnaryOperator::Negate => {
                        if operand_type == Type::Integer || operand_type == Type::I8 || operand_type == Type::I16 || operand_type == Type::I32 || operand_type == Type::I64 || operand_type == Type::ISize || operand_type == Type::Float {
                            Ok(operand_type)
                        } else {
                            Err(SemanticError {
                                message: "Negation requires numeric operand".to_string(),
                            })
                        }
                    },
                    crate::ast::UnaryOperator::BitNot => {
                        if operand_type == Type::Unknown || operand_type == Type::Integer || operand_type == Type::I8 || operand_type == Type::I16 || operand_type == Type::I32 || operand_type == Type::I64 || operand_type == Type::ISize || operand_type == Type::U8 || operand_type == Type::U16 || operand_type == Type::U32 || operand_type == Type::U64 || operand_type == Type::USize {
                            Ok(Type::Integer)
                        } else {
                            Err(SemanticError { message: "Bitwise NOT requires integer operand".to_string() })
                        }
                    },
                }
            },
            Expr::Call { callee, arguments: _ } => {
                match callee.as_ref() {
                    Expr::Variable(func_name) => {
                        if func_name == "vault" {
                            return Ok(Type::Vault(Box::new(Type::String), Box::new(Type::Unknown)));
                        } else if func_name == "pool" {
                            return Ok(Type::Pool(Box::new(Type::Unknown)));
                        } else if func_name == "tree" {
                            return Ok(Type::Tree(Box::new(Type::Unknown)));
                        } else if func_name == "len" {
                            return Ok(Type::Integer);
                        }
                        // Check built-in functions first
                        if self.std_lib.is_builtin_function(func_name) {
                            // For type inference, we need to analyze the arguments first to get their types
                            // This is a simplified approach - in a full implementation, we'd need to handle this more carefully
                            if let Some(builtin_func) = self.std_lib.get_builtin_function(func_name) {
                                Ok(builtin_func.return_type.clone())
                            } else {
                                Err(SemanticError {
                                    message: format!("Built-in function '{}' not found", func_name),
                                })
                            }
                        } else {
                            // Check user-defined functions
                            match self.get_symbol(func_name) {
                                Ok(Symbol::Function { return_type, .. }) => {
                                    if return_type == Type::Void {
                                        // Function exists but return type is Void - this might be a function
                                        // whose return type will be inferred later. For now, return Void
                                        // and let the actual type checking happen during expression analysis
                                        Ok(Type::Void)
                                    } else {
                                        Ok(return_type)
                                    }
                                },
                                Ok(Symbol::Variable { .. }) => Err(SemanticError {
                                    message: format!("Expected function, found variable: {}", func_name),
                                }),
                                Ok(Symbol::Namespace { .. }) => Err(SemanticError {
                                    message: format!("Expected function, found namespace: {}", func_name),
                                }),
                                Err(_) => Err(SemanticError {
                                    message: format!("Undefined function: {}", func_name),
                                }),
                            }
                        }
                    },
                    Expr::Get { object, name } => {
                        // Handle namespace function calls first (e.g., std.sum)
                        if let Expr::Variable(namespace_name) = object.as_ref() {
                            if let Ok(Symbol::Namespace { .. }) = self.get_symbol(namespace_name) {
                                let qualified_name = format!("{}.{}", namespace_name, name);
                                match self.get_symbol(&qualified_name) {
                                    Ok(Symbol::Function { return_type, .. }) => return Ok(return_type),
                                    Ok(_) => return Err(SemanticError { message: format!("'{}' is not a function", qualified_name) }),
                                    Err(_) => return Err(SemanticError { message: format!("Undefined function: {}", qualified_name) }),
                                }
                            }
                        }
                        let obj_type = self.infer_type(object)?;
                        match obj_type {
                            Type::String => {
                                match name.as_str() {
                                    "upper" | "lower" | "trim" => Ok(Type::String),
                                    "contains" => Ok(Type::Boolean),
                                    "split" => Ok(Type::Array(Box::new(Type::String), 0)),
                                    "replace" => Ok(Type::String),
                                    "substring" => Ok(Type::String),
                                    "regex" => Ok(Type::Array(Box::new(Type::String), 0)),
                                    _ => Err(SemanticError { message: format!("Unknown string method: {}", name) }),
                                }
                            }
                            Type::Array(_inner, _) => {
                                match name.as_str() {
                                    "len" => Ok(Type::Integer),
                                    "join" => Ok(Type::String),
                                    _ => Err(SemanticError { message: format!("Unknown array method: {}", name) }),
                                }
                            }
                            _ => {
                                Err(SemanticError { message: "Complex function call expressions not yet supported".to_string() })
                            }
                        }
                    }
                    _ => Err(SemanticError {
                        message: "Complex function call expressions not yet supported".to_string(),
                    }),
                }
            },
            Expr::Get { object, name } => {
                // Handle namespace variable access (e.g., math.PI)
                if let Expr::Variable(namespace_name) = object.as_ref() {
                    let qualified_name = format!("{}.{}", namespace_name, name);
                    match self.get_symbol(&qualified_name) {
                        Ok(Symbol::Variable { var_type, .. }) => Ok(var_type),
                        Ok(Symbol::Function { .. }) => Err(SemanticError {
                            message: format!("'{}' is a function, not a variable", qualified_name),
                        }),
                        Ok(Symbol::Namespace { .. }) => Err(SemanticError {
                            message: format!("'{}' is a namespace, not a variable", qualified_name),
                        }),
                        Err(_) => Err(SemanticError {
                            message: format!("Symbol '{}' not found in namespace '{}'", name, namespace_name),
                        }),
                    }
                } else {
                    Err(SemanticError {
                        message: "Complex field access expressions not yet supported".to_string(),
                    })
                }
            },
            Expr::Index { sequence, index } => {
                let sequence_type = self.infer_type(sequence)?;
                let index_type = self.infer_type(index)?;
                match sequence_type {
                    Type::Array(element_type, _) => {
                        if index_type == Type::Integer || index_type == Type::I8 || index_type == Type::I16 || index_type == Type::I32 || index_type == Type::I64 || index_type == Type::ISize || index_type == Type::U8 || index_type == Type::U16 || index_type == Type::U32 || index_type == Type::U64 || index_type == Type::USize {
                            if let Expr::Literal(Literal::Integer(index_value)) = index.as_ref() {
                                if let Expr::ArrayLiteral { elements } = sequence.as_ref() {
                                    let index_usize = *index_value as usize;
                                    if index_usize >= elements.len() {
                                        return Err(SemanticError { message: format!("Array index {} out of bounds for array of size {}", index_value, elements.len()) });
                                    }
                                }
                            }
                            Ok(*element_type)
                        } else {
                            Err(SemanticError { message: format!("Array index must be an integer type, got {:?}", index_type) })
                        }
                    }
                    Type::Vault(_key_t, val_t) => {
                        if index_type == Type::String { Ok(*val_t) } else { Err(SemanticError { message: "Vault key must be string".to_string() }) }
                    }
                    Type::Unknown => {
                        if matches!(index_type, Type::Integer | Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::ISize | Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::USize) {
                            Ok(Type::Unknown)
                        } else {
                            Err(SemanticError { message: "Index type must be integer for Unknown sequence".to_string() })
                        }
                    }
                    _ => Err(SemanticError { message: format!("Cannot index non-array type: {:?}", sequence_type) }),
                }
            },
            Expr::ArrayLiteral { elements } => {
                // Array literal: [expr1, expr2, ...]
                if elements.is_empty() {
                    return Err(SemanticError {
                        message: "Array literals must have at least one element".to_string(),
                    });
                }
                
                // Infer type of first element
                let first_type = self.infer_type(&elements[0])?;
                
                // Check that all elements have the same type
                for (i, element) in elements.iter().enumerate().skip(1) {
                    let element_type = self.infer_type(element)?;
                    if element_type != first_type {
                        return Err(SemanticError {
                            message: format!("Array element {} has type {:?}, expected {:?}", 
                                           i, element_type, first_type),
                        });
                    }
                }
                
                // Return array type with the element type and size
                Ok(Type::Array(Box::new(first_type), elements.len()))
            },
            Expr::VaultLiteral { .. } => {
                Ok(Type::Vault(Box::new(Type::String), Box::new(Type::Unknown)))
            },
            Expr::PoolLiteral { .. } => {
                Ok(Type::Pool(Box::new(Type::Unknown)))
            },
            Expr::TreeLiteral { .. } => {
                Ok(Type::Tree(Box::new(Type::Unknown)))
            },
            _ => {
                Err(SemanticError {
                    message: "Type inference not implemented for this expression type".to_string(),
                })
            },
        }
    }
    
    fn define_symbol(&mut self, name: String, symbol: Symbol) -> Result<(), SemanticError> {
        
        // Check if symbol already exists in current scope
        if self.scopes.last().unwrap().contains_key(&name) {
            return Err(SemanticError {
                message: format!("Symbol '{}' already defined in current scope", name),
            });
        }
        
        // Add to the current scope
        self.scopes.last_mut().unwrap().insert(name, symbol);
        Ok(())
    }
    
    fn symbol_exists(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if scope.contains_key(name) {
                return true;
            }
        }
        false
    }
    
    fn get_symbol(&self, name: &str) -> Result<Symbol, SemanticError> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Ok(symbol.clone());
            }
        }
        
        Err(SemanticError {
            message: format!("Symbol '{}' not found", name),
        })
    }
    
    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }
    
    fn end_scope(&mut self) {
        self.scopes.pop();
    }
    
    fn resolve_module_path(&self, module_name: &str) -> PathBuf {
        // Convert module name to file path
        // e.g., "math.utils" -> "math/utils.nlang"
        // e.g., "06_functions" -> "06_functions.nlang" (in current directory)
        let path_parts: Vec<&str> = module_name.split('.').collect();
        let mut path = self.current_dir.clone();
        
        if path_parts.len() > 1 {
            // For dotted module names, create subdirectories
            for part in &path_parts[..path_parts.len() - 1] {
                path.push(part);
            }
        }
        
        path.push(format!("{}.nlang", path_parts.last().unwrap()));
        path
    }
    
    fn load_module(&mut self, module_path: &Path) -> Result<ModuleInfo, SemanticError> {
        // Check if module is already cached
        if let Some(cached_module) = self.module_cache.get(module_path) {
            return Ok(cached_module.clone());
        }
        
        // Read the module file
        let source = fs::read_to_string(module_path)
            .map_err(|e| SemanticError {
                message: format!("Failed to read module file '{}': {}", module_path.display(), e),
            })?;
        
        // Parse the module
        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize()
            .map_err(|e| SemanticError {
                message: format!("Lexer error in module '{}': {}", module_path.display(), e),
            })?;
        
        let mut parser = Parser::new(&tokens);
        let program = parser.parse_program()
            .map_err(|e| SemanticError {
                message: format!("Parser error in module '{}': {}", module_path.display(), e),
            })?;
        
        // Analyze the module to extract exported symbols
        let mut module_analyzer = SemanticAnalyzer::new_with_file_path(Some(module_path));
        let analyzed_program = module_analyzer.analyze_program(Program { statements: program.clone() }, false)?; // false indicates this is not the main program
        
        // Extract exported symbols
        let mut exported_symbols = HashMap::new();
        self.extract_exported_symbols(&analyzed_program.statements, &mut exported_symbols)?;
        
        let module_info = ModuleInfo {
            exported_symbols,
            original_program: Program { statements: program },
        };
        
        // Cache the module
        self.module_cache.insert(module_path.to_path_buf(), module_info.clone());
        
        Ok(module_info)
    }

    fn load_std_module(&mut self) -> Result<ModuleInfo, SemanticError> {
        let source = crate::std_lib::nlang::std_module();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().map_err(|e| SemanticError { message: format!("Lexer error in std module: {}", e) })?;
        let mut parser = Parser::new(&tokens);
        let program = parser.parse_program().map_err(|e| SemanticError { message: format!("Parser error in std module: {}", e) })?;
        let mut module_analyzer = SemanticAnalyzer::new_with_file_path(None);
        module_analyzer.std_imported = true;
        module_analyzer.allow_builtin_override = true;
        let analyzed_program = module_analyzer.analyze_program(Program { statements: program.clone() }, false)?;
        let mut exported_symbols = HashMap::new();
        self.extract_exported_symbols(&analyzed_program.statements, &mut exported_symbols)?;
        Ok(ModuleInfo { exported_symbols, original_program: Program { statements: program } })
    }
    
    fn extract_exported_symbols(&self, statements: &[Statement], symbols: &mut HashMap<String, Symbol>) -> Result<(), SemanticError> {
        for stmt in statements {
            match stmt {
                Statement::FunctionDeclaration { name, parameters, return_type, is_exported, .. } => {
                    if *is_exported {
                        let func_return_type = return_type.clone().unwrap_or(Type::Void);
                        symbols.insert(name.clone(), Symbol::Function {
                            return_type: func_return_type,
                            parameters: parameters.clone(),
                        });
                    }
                },
                Statement::LetDeclaration { name, initializer, var_type: _, is_mutable, is_exported } => {
                    if *is_exported {
                        // Infer type from initializer if available
                        let var_type = if let Some(init_expr) = initializer {
                        self.infer_type(init_expr)?
                        } else {
                            Type::Integer // Default type for uninitialized variables
                        };
                        symbols.insert(name.clone(), Symbol::Variable { var_type, is_mutable: *is_mutable });
                    }
                },
                Statement::Block { statements } => {
                    self.extract_exported_symbols(statements, symbols)?;
                },
                _ => {}
            }
        }
        Ok(())
    }

    fn analyze_pattern(&self, pattern: &crate::ast::Pattern) -> Result<crate::ast::Pattern, SemanticError> {
        Ok(pattern.clone())
    }

    fn validate_main_function(&self) -> Result<(), SemanticError> {
        // Check if main function exists
        match self.get_symbol("main") {
            Ok(Symbol::Function { return_type, parameters }) => {
                // Validate main function signature: fn main() -> void (or no return type)
                if !parameters.is_empty() {
                    return Err(SemanticError {
                        message: "Main function should not have parameters".to_string(),
                    });
                }
                
                // Main function should return void or have no explicit return type
                if return_type != Type::Void {
                    return Err(SemanticError {
                        message: "Main function should return void or have no return type".to_string(),
                    });
                }
                
                Ok(())
            },
            Ok(_) => Err(SemanticError {
                message: "Symbol 'main' exists but is not a function".to_string(),
            }),
            Err(_) => Err(SemanticError {
                message: "No main function found. Programs must have a main function as entry point".to_string(),
            }),
        }
    }
    
    /// Check if two types are compatible for assignment/initialization
    fn are_types_compatible(&self, type1: &Type, type2: &Type) -> bool {
        if *type1 == Type::Unknown || *type2 == Type::Unknown {
            return true;
        }
        // Same types are always compatible
        if type1 == type2 {
            return true;
        }

        // Helper function to check if a type is any integer type
        let is_integer_type = |t: &Type| {
            matches!(t, Type::Integer | Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::ISize |
                     Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::USize)
        };

        // All integer types are compatible with each other (with potential promotion)
        if is_integer_type(type1) && is_integer_type(type2) {
            return true;
        }

        // Any integer type can be converted to float types
        if is_integer_type(type1) {
            if matches!(type2, Type::Float | Type::F32 | Type::F64) {
                return true;
            }
        }

        // Float types can be converted between each other (with potential precision loss)
        if matches!(type1, Type::Float | Type::F32 | Type::F64) && matches!(type2, Type::Float | Type::F32 | Type::F64) {
            return true;
        }

        // Array type compatibility - inner types must be compatible and sizes must match
        if let (Type::Array(inner1, size1), Type::Array(inner2, size2)) = (type1, type2) {
            return size1 == size2 && self.are_types_compatible(inner1, inner2);
        }

        false
    }

    /// Validate that a literal value is within the bounds of its target type
    #[allow(unused_comparisons)] // Needed for bounds checking of literal values
    fn validate_literal_bounds(&self, literal: &Literal, target_type: &Type) -> Result<(), SemanticError> {
        match (literal, target_type) {
            (Literal::I8(value), Type::I8) => {
                // i8 range: -128 to 127
                if *value < -128 || *value > 127 {
                    return Err(SemanticError {
                        message: format!("Value {} is out of bounds for i8 type (-128 to 127)", value),
                    });
                }
            }
            (Literal::I16(value), Type::I16) => {
                // i16 range: -32768 to 32767
                if *value < -32768 || *value > 32767 {
                    return Err(SemanticError {
                        message: format!("Value {} is out of bounds for i16 type (-32768 to 32767)", value),
                    });
                }
            }
            (Literal::I32(value), Type::I32) => {
                // i32 range: -2147483648 to 2147483647
                if *value < -2147483648 || *value > 2147483647 {
                    return Err(SemanticError {
                        message: format!("Value {} is out of bounds for i32 type (-2147483648 to 2147483647)", value),
                    });
                }
            }
            (Literal::I64(value), Type::I64) => {
                // i64 range: -9223372036854775808 to 9223372036854775807
                if *value < -9223372036854775808 || *value > 9223372036854775807 {
                    return Err(SemanticError {
                        message: format!("Value {} is out of bounds for i64 type", value),
                    });
                }
            }
            (Literal::ISize(value), Type::ISize) => {
                // isize range: platform-dependent, but we'll use i64 range for validation
                if *value < -9223372036854775808 || *value > 9223372036854775807 {
                    return Err(SemanticError {
                        message: format!("Value {} is out of bounds for isize type", value),
                    });
                }
            }
            (Literal::U8(value), Type::U8) => {
                // u8 range: 0 to 255
                if *value > 255 {
                    return Err(SemanticError {
                        message: format!("Value {} is out of bounds for u8 type (0 to 255)", value),
                    });
                }
            }
            (Literal::U16(value), Type::U16) => {
                // u16 range: 0 to 65535
                if *value > 65535 {
                    return Err(SemanticError {
                        message: format!("Value {} is out of bounds for u16 type (0 to 65535)", value),
                    });
                }
            }
            (Literal::U32(value), Type::U32) => {
                // u32 range: 0 to 4294967295
                if *value > 4294967295 {
                    return Err(SemanticError {
                        message: format!("Value {} is out of bounds for u32 type (0 to 4294967295)", value),
                    });
                }
            }
            (Literal::U64(_value), Type::U64) => {
                // u64 range: 0 to 18446744073709551615
                // No upper bound check needed since u64 can represent any u64 value
                // The value is already unsigned, so no need to check < 0
            }
            (Literal::USize(_value), Type::USize) => {
                // usize range: platform-dependent
                // The value is already unsigned, so no need to check < 0
            }
            // For other combinations, no bounds checking is needed
            _ => {}
        }
        
        Ok(())
    }
    
    /// Find a function definition by name in a loaded module
    fn find_function_definition_in_module(&self, module_path: &Path, function_name: &str) -> Option<Statement> {
        // Look for the module in the cache
        if let Some(module_info) = self.module_cache.get(module_path) {
            // Search through the module's statements to find the function definition
            for stmt in &module_info.original_program.statements {
                if let Statement::FunctionDeclaration { name, .. } = stmt {
                    if name == function_name {
                        return Some(stmt.clone());
                    }
                }
            }
        }
        None
    }
}
