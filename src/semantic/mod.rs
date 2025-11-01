use crate::ast::{Program, Statement, Expr, Type, Literal, Parameter, BinaryOperator};
use crate::std_lib::StdLib;
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct SemanticError {
    pub message: String,
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Semantic error: {}", self.message)
    }
}

impl std::error::Error for SemanticError {}

pub fn analyze(program: Program) -> Result<Program, SemanticError> {
    analyze_with_file_path(program, None)
}

pub fn analyze_with_file_path(program: Program, file_path: Option<&std::path::Path>) -> Result<Program, SemanticError> {
    let mut analyzer = SemanticAnalyzer::new_with_file_path(file_path);
    analyzer.analyze_program(program, true) // true indicates this is the main program
}

struct SemanticAnalyzer {
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
}

#[derive(Debug, Clone)]
struct ModuleInfo {
    // Exported symbols from the module
    exported_symbols: HashMap<String, Symbol>,
}

#[derive(Debug, Clone)]
enum Symbol {
    Variable { var_type: Type },
    Function { 
        return_type: Type, 
        parameters: Vec<Parameter> 
    },
    Namespace { 
        #[allow(dead_code)]
        module_name: String 
    },
}

impl SemanticAnalyzer {
    fn new() -> Self {
        Self::new_with_file_path(None)
    }
    
    fn new_with_file_path(file_path: Option<&std::path::Path>) -> Self {
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
        for stmt in statements {
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
                        Err(_) => return None,
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
    
    fn analyze_program(&mut self, program: Program, is_main_program: bool) -> Result<Program, SemanticError> {
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
        
        // Second pass: analyze all statements
        let mut analyzed_statements = Vec::new();
        for stmt in program.statements {
            analyzed_statements.push(self.analyze_statement(stmt)?);
        }
        
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
            Statement::LetDeclaration { name, initializer, is_exported } => {
                let analyzed_initializer = match initializer {
                    Some(expr) => Some(self.analyze_expr(expr)?),
                    None => None,
                };
                
                // Infer type from initializer or use default
                let var_type = if let Some(ref init) = analyzed_initializer {
                    self.infer_type(init)?
                } else {
                    Type::Integer // Default type for uninitialized variables
                };
                
                self.define_symbol(name.clone(), Symbol::Variable { var_type })?;
                
                Ok(Statement::LetDeclaration {
                    name,
                    initializer: analyzed_initializer,
                    is_exported,
                })
            },
            Statement::FunctionDeclaration { name, parameters, body, return_type, is_exported } => {
                let func_return_type = return_type.clone().unwrap_or(Type::Void);
                
                // If no explicit return type, try to infer it from return statements
                let mut inferred_return_type = func_return_type.clone();
                if return_type.is_none() {
                    // Enter function scope to analyze return statements
                    self.begin_scope();
                    
                    // Add parameters to the scope for analysis
                    for param in &parameters {
                        self.define_symbol(
                            param.name.clone(), 
                            Symbol::Variable { var_type: param.param_type.clone() }
                        )?;
                    }
                    
                    // Look for return statements to infer type
                    if let Some(return_type) = self.find_return_type_in_statements(&body) {
                        inferred_return_type = return_type;
                    }
                    
                    // Exit the temporary scope
                    self.end_scope();
                    
                    // Update the function's return type in the symbol table
                    if inferred_return_type != Type::Void {
                        // Remove the old entry and add the new one with correct return type
                        if let Some(current_scope) = self.scopes.last_mut() {
                            current_scope.insert(name.clone(), Symbol::Function {
                                return_type: inferred_return_type.clone(),
                                parameters: parameters.clone(),
                            });
                        }
                    }
                }
                
                // Set current function context
                let previous_return_type = self.current_function_return_type.clone();
                self.current_function_return_type = Some(inferred_return_type.clone());
                
                // Enter function scope for actual analysis
                self.begin_scope();
                
                // Add parameters to the scope
                for param in &parameters {
                    self.define_symbol(
                        param.name.clone(), 
                        Symbol::Variable { var_type: param.param_type.clone() }
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
                    return Err(SemanticError {
                        message: format!("Function '{}' with return type {:?} must have a return statement", name, inferred_return_type),
                    });
                }
                
                // Exit function scope and restore previous context
                self.end_scope();
                self.current_function_return_type = previous_return_type;
                
                Ok(Statement::FunctionDeclaration {
                    name,
                    parameters,
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
                            if actual_type != *expected_type {
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
                // Resolve and load the module
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
                        self.define_symbol(symbol_name.clone(), symbol.clone())?;
                    }
                }
                
                Ok(Statement::Import { module, alias })
            },
            Statement::ImportFrom { module, items } => {
                // Resolve and load the module
                let module_path = self.resolve_module_path(&module);
                let module_info = self.load_module(&module_path)?;
                
                // Import specific items from the module
                for (item, alias) in &items {
                    let symbol_name = alias.as_ref().unwrap_or(item);
                    
                    // Check if the item exists in the module's exported symbols
                    if let Some(symbol) = module_info.exported_symbols.get(item) {
                        self.define_symbol(symbol_name.clone(), symbol.clone())?;
                    } else {
                        return Err(SemanticError {
                            message: format!("Symbol '{}' not found in module '{}'", item, module),
                        });
                    }
                }
                
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
                        if operand_type != Type::Integer && operand_type != Type::Float {
                            return Err(SemanticError {
                                message: "Operand of unary minus must be of numeric type".to_string(),
                            });
                        }
                    },
                }
                
                Ok(Expr::Unary {
                    operator,
                    operand: analyzed_operand,
                })
            },
            Expr::Call { callee, arguments } => {
                // Handle different types of function calls
                let func_name = match callee.as_ref() {
                    Expr::Variable(name) => name.clone(),
                    Expr::Get { object, name } => {
                        // Handle module-qualified function calls (e.g., math.add())
                        if let Expr::Variable(namespace_name) = object.as_ref() {
                            format!("{}.{}", namespace_name, name)
                        } else {
                            return Err(SemanticError {
                                message: "Complex function calls not yet supported".to_string(),
                            });
                        }
                    },
                    _ => {
                        return Err(SemanticError {
                            message: "Complex function calls not yet supported".to_string(),
                        });
                    }
                };
                
                let mut analyzed_arguments = Vec::new();                                                          
                for arg in arguments {                                                                            
                    analyzed_arguments.push(self.analyze_expr(arg)?);                                             
                }                                                                                                 
                                                                                                                  
                // Check function signature
                    // Check if it's a built-in function first
                    if self.std_lib.is_builtin_function(&func_name) {
                        // Get argument types for overload resolution
                        let mut arg_types = Vec::new();
                        for arg in &analyzed_arguments {
                            arg_types.push(self.infer_type(arg)?);
                        }
                        
                        // Try to find function with matching signature
                        if let Some(_builtin_func) = self.std_lib.get_builtin_function_by_signature(&func_name, &arg_types) {
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
                                    let arg_type = self.infer_type(arg)?;
                                    let param_type = &builtin_func.parameters[i];
                                    
                                    // Special case for println and print - they can accept any type
                                    if func_name == "println" || func_name == "print" {
                                        // Skip type checking for println and print - they handle conversion internally
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
                                let param_type = &parameters[i].param_type;
                                if arg_type != *param_type {
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
                    self.define_symbol(
                        param.name.clone(), 
                        Symbol::Variable { var_type: param.param_type.clone() }
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
                if let Symbol::Variable { var_type } = var_symbol {
                    let value_type = self.infer_type(&analyzed_value)?;
                    if var_type != value_type {
                        return Err(SemanticError {
                            message: format!("Type mismatch in assignment: expected {:?}, got {:?}", var_type, value_type),
                        });
                    }
                }
                
                Ok(Expr::Assign { name, value: analyzed_value })
            },
        }
    }
    
    fn infer_type(&self, expr: &Expr) -> Result<Type, SemanticError> {
        match expr {
            Expr::Literal(literal) => {
                match literal {
                    Literal::Integer(_) => Ok(Type::Integer),
                    Literal::Float(_) => Ok(Type::Float),
                    Literal::Boolean(_) => Ok(Type::Boolean),
                    Literal::String(_) => Ok(Type::String),
                    Literal::Null => Ok(Type::Void), // Null literals have void type
                }
            },
            Expr::Variable(name) => {
                match self.get_symbol(name)? {
                    Symbol::Variable { var_type } => Ok(var_type),
                    Symbol::Function { .. } => Err(SemanticError {
                        message: format!("Expected variable, found function: {}", name),
                    }),
                    Symbol::Namespace { .. } => Err(SemanticError {
                        message: format!("Cannot use namespace '{}' as a value", name),
                    }),
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
                        } else if left_type == Type::Float || right_type == Type::Float {
                            if (left_type == Type::Integer || left_type == Type::Float) &&
                               (right_type == Type::Integer || right_type == Type::Float) {
                                Ok(Type::Float)
                            } else {
                                Err(SemanticError {
                                    message: format!("Cannot perform arithmetic on {:?} and {:?}", left_type, right_type),
                                })
                            }
                        } else if left_type == Type::Integer && right_type == Type::Integer {
                            Ok(Type::Integer)
                        } else {
                            Err(SemanticError {
                                message: format!("Cannot perform arithmetic on {:?} and {:?}", left_type, right_type),
                            })
                        }
                    },
                    BinaryOperator::Minus | BinaryOperator::Star => {
                        // Arithmetic operations: promote to float if either operand is float
                        if left_type == Type::Float || right_type == Type::Float {
                            if left_type == Type::Integer || left_type == Type::Float {
                                if right_type == Type::Integer || right_type == Type::Float {
                                    Ok(Type::Float)
                                } else {
                                    Err(SemanticError {
                                        message: format!("Cannot perform arithmetic on {:?} and {:?}", left_type, right_type),
                                    })
                                }
                            } else {
                                Err(SemanticError {
                                    message: format!("Cannot perform arithmetic on {:?} and {:?}", left_type, right_type),
                                })
                            }
                        } else if left_type == Type::Integer && right_type == Type::Integer {
                            Ok(Type::Integer)
                        } else {
                            Err(SemanticError {
                                message: format!("Cannot perform arithmetic on {:?} and {:?}", left_type, right_type),
                            })
                        }
                    },
                    BinaryOperator::Slash => {
                        // Division always returns float, even for integer operands
                        if (left_type == Type::Integer || left_type == Type::Float) &&
                           (right_type == Type::Integer || right_type == Type::Float) {
                            Ok(Type::Float)
                        } else {
                            Err(SemanticError {
                                message: format!("Cannot perform division on {:?} and {:?}", left_type, right_type),
                            })
                        }
                    },
                    BinaryOperator::Percent => {
                        // Modulo only works with integers
                        if left_type == Type::Integer && right_type == Type::Integer {
                            Ok(Type::Integer)
                        } else {
                            Err(SemanticError {
                                message: "Modulo operator requires integer operands".to_string(),
                            })
                        }
                    },
                    BinaryOperator::EqualEqual | BinaryOperator::NotEqual => {
                        // Equality comparison: operands must be compatible types
                        if left_type == right_type || 
                           (left_type == Type::Integer && right_type == Type::Float) ||
                           (left_type == Type::Float && right_type == Type::Integer) {
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
                        if (left_type == Type::Integer || left_type == Type::Float) &&
                           (right_type == Type::Integer || right_type == Type::Float) {
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
                        if operand_type == Type::Integer || operand_type == Type::Float {
                            Ok(operand_type)
                        } else {
                            Err(SemanticError {
                                message: "Negation requires numeric operand".to_string(),
                            })
                        }
                    },
                }
            },
            Expr::Call { callee, .. } => {
                // For function calls, we need to look up the return type in the symbol table
                match callee.as_ref() {
                    Expr::Variable(func_name) => {
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
                                Ok(Symbol::Function { return_type, .. }) => Ok(return_type),
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
                        // Handle namespace function calls (e.g., math.add())
                        if let Expr::Variable(namespace_name) = object.as_ref() {
                            let qualified_name = format!("{}.{}", namespace_name, name);
                            match self.get_symbol(&qualified_name) {
                                Ok(Symbol::Function { return_type, .. }) => Ok(return_type),
                                Ok(_) => Err(SemanticError {
                                    message: format!("'{}' is not a function", qualified_name),
                                }),
                                Err(_) => Err(SemanticError {
                                    message: format!("Undefined function: {}", qualified_name),
                                }),
                            }
                        } else {
                            Err(SemanticError {
                                message: "Complex function call expressions not yet supported".to_string(),
                            })
                        }
                    },
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
                        Ok(Symbol::Variable { var_type }) => Ok(var_type),
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
            _ => {
                Err(SemanticError {
                    message: "Type inference not implemented for this expression type".to_string(),
                })
            },
        }
    }
    
    fn define_symbol(&mut self, name: String, symbol: Symbol) -> Result<(), SemanticError> {
        // Check if it's a built-in function
        if self.std_lib.is_builtin_function(&name) {
            return Err(SemanticError {
                message: format!("Cannot redefine built-in function '{}'", name),
            });
        }
        
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
        let path_parts: Vec<&str> = module_name.split('.').collect();
        let mut path = self.current_dir.clone();
        
        for part in &path_parts[..path_parts.len() - 1] {
            path.push(part);
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
        let mut module_analyzer = SemanticAnalyzer::new();
        let analyzed_program = module_analyzer.analyze_program(Program { statements: program }, false)?; // false indicates this is not the main program
        
        // Extract exported symbols
        let mut exported_symbols = HashMap::new();
        self.extract_exported_symbols(&analyzed_program.statements, &mut exported_symbols)?;
        
        let module_info = ModuleInfo {
            exported_symbols,
        };
        
        // Cache the module
        self.module_cache.insert(module_path.to_path_buf(), module_info.clone());
        
        Ok(module_info)
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
                Statement::LetDeclaration { name, initializer, is_exported } => {
                    if *is_exported {
                        // Infer type from initializer if available
                        let var_type = if let Some(init_expr) = initializer {
                        self.infer_type(init_expr)?
                        } else {
                            Type::Integer // Default type for uninitialized variables
                        };
                        symbols.insert(name.clone(), Symbol::Variable { var_type });
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
}