pub use self::environment::Environment;
pub use self::error::InterpreterError;
pub use self::value::{Function, Value};
use crate::ast::{BinaryOperator, Expr, Literal, Program, Statement, UnaryOperator};
use crate::interpreter::value::{SimpleValue, TreeNode};
use crate::lexer::Lexer;
use crate::nlang_libs::registry::{LibraryRegistry, get_default_registry};
use crate::nlang_libs::std_lib::nlang::std_module;
use crate::nlang_libs::std_lib::string as string_lib;
use crate::parser::Parser;
use chrono::{Datelike, TimeZone, Timelike};
use std::collections::HashMap;
use std::fs;

pub mod environment;
pub mod error;
pub mod value;

pub struct Interpreter {
    global_env: Environment,
    module_cache: HashMap<String, Program>,
    registry: LibraryRegistry,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            global_env: Environment::new(),
            module_cache: HashMap::new(),
            registry: get_default_registry(),
        }
    }

    pub fn execute_program(&mut self, program: &Program) -> Result<i32, InterpreterError> {
        self.execute_program_with_path(program, None)
    }

    pub fn execute_program_with_path(
        &mut self,
        program: &Program,
        file_path: Option<&str>,
    ) -> Result<i32, InterpreterError> {
        // First pass: handle imports
        for statement in &program.statements {
            match statement {
                Statement::Import { module, alias } => {
                    self.load_module(module, alias.as_deref(), file_path)?;
                }
                Statement::ImportFrom { module, items } => {
                    self.load_module_items(module, items, file_path)?;
                }
                _ => {}
            }
        }

        // Second pass: collect all function declarations
        for statement in &program.statements {
            if let Statement::FunctionDeclaration {
                name,
                parameters,
                body,
                return_type,
                ..
            } = statement
            {
                let func = Function {
                    name: name.clone(),
                    parameters: parameters.clone(),
                    body: body.clone(),
                    return_type: return_type.clone(),
                    native_impl: None,
                };
                self.global_env.define_function(func);
            }
        }

        // Execute main function if it exists
        if let Ok(main_func) = self.global_env.get_function("main") {
            let main_func = main_func.clone();
            match self.execute_function(&main_func, &[]) {
                Ok(value) => Ok(value.to_int().unwrap_or(0) as i32),
                Err(InterpreterError::ReturnValue(value)) => Ok(value.to_int().unwrap_or(0) as i32),
                Err(e) => Err(e),
            }
        } else {
            // Execute statements in order
            let mut env = self.global_env.clone();
            for statement in &program.statements {
                match self.execute_statement(statement, &mut env) {
                    Ok(_) => {} // Ignore successful execution of top-level statements
                    Err(InterpreterError::ReturnValue(value)) => {
                        return Ok(value.to_int().unwrap_or(0) as i32);
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(0)
        }
    }

    fn load_module(
        &mut self,
        module_path: &str,
        alias: Option<&str>,
        importing_file: Option<&str>,
    ) -> Result<(), InterpreterError> {
        let module_program = self.parse_module(module_path, importing_file)?;
        let namespace = alias.unwrap_or(module_path).to_string();
        self.module_cache.insert(namespace, module_program);
        Ok(())
    }

    fn load_module_items(
        &mut self,
        module_path: &str,
        imports: &[(String, Option<String>)],
        importing_file: Option<&str>,
    ) -> Result<(), InterpreterError> {
        let module_program = self.parse_module(module_path, importing_file)?;

        for (item_name, alias) in imports {
            let local_name = alias.as_ref().unwrap_or(item_name);

            // Look for exported functions
            for statement in &module_program.statements {
                if let Statement::FunctionDeclaration {
                    name,
                    parameters,
                    body,
                    return_type,
                    is_exported,
                } = statement
                {
                    if name == item_name && *is_exported {
                        let mut native_impl = None;
                        if let Some(lib) = self.registry.get_library(module_path) {
                            if let Some(func_def) = lib.functions.iter().find(|f| f.name == *name) {
                                native_impl = func_def.implementation;
                            }
                        }

                        let func = Function {
                            name: local_name.clone(),
                            parameters: parameters.clone(),
                            body: body.clone(),
                            return_type: return_type.clone(),
                            native_impl,
                        };
                        self.global_env.define_function(func);
                        break;
                    }
                }
            }

            // Look for exported constants
            for statement in &module_program.statements {
                if let Statement::LetDeclaration {
                    name,
                    initializer,
                    var_type: _,
                    is_mutable: _,
                    is_exported,
                } = statement
                {
                    if name == item_name && *is_exported {
                        if let Some(init_expr) = initializer {
                            let mut temp_env = self.global_env.clone();
                            let value = self.evaluate_expression(init_expr, &mut temp_env)?;
                            self.global_env.define_variable(local_name.clone(), value);
                        }
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn ensure_function_loaded(&mut self, name: &str) -> bool {
        if self.global_env.get_function(name).is_ok() {
            return true;
        }
        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() == 2 {
            return self.ensure_function_loaded_qualified(parts[0], parts[1]);
        }
        self.ensure_function_loaded_direct(name)
    }

    fn ensure_function_loaded_direct(&mut self, func_name: &str) -> bool {
        for (ns, prog) in self.module_cache.clone() {
            for stmt in &prog.statements {
                if let Statement::FunctionDeclaration {
                    name,
                    parameters,
                    body,
                    return_type,
                    is_exported,
                } = stmt
                {
                    if *is_exported && name == func_name {
                        let mut native_impl = None;
                        // Check if the namespace corresponds to a registered library
                        if let Some(lib) = self.registry.get_library(&ns) {
                            if let Some(func_def) = lib.functions.iter().find(|f| f.name == *name) {
                                native_impl = func_def.implementation;
                            }
                        }

                        let f = Function {
                            name: func_name.to_string(),
                            parameters: parameters.clone(),
                            body: body.clone(),
                            return_type: return_type.clone(),
                            native_impl,
                        };
                        self.global_env.define_function(f);
                        return true;
                    }
                }
            }
            let _ = ns;
        }
        false
    }

    fn ensure_function_loaded_qualified(&mut self, namespace: &str, func_name: &str) -> bool {
        if let Some(prog) = self.module_cache.get(namespace) {
            for stmt in &prog.statements {
                if let Statement::FunctionDeclaration {
                    name,
                    parameters,
                    body,
                    return_type,
                    is_exported,
                } = stmt
                {
                    if *is_exported && name == func_name {
                        let mut native_impl = None;
                        if let Some(lib) = self.registry.get_library(namespace) {
                            if let Some(func_def) = lib.functions.iter().find(|f| f.name == *name) {
                                native_impl = func_def.implementation;
                            }
                        }

                        let qualified = format!("{}.{}", namespace, func_name);
                        let f = Function {
                            name: qualified.clone(),
                            parameters: parameters.clone(),
                            body: body.clone(),
                            return_type: return_type.clone(),
                            native_impl,
                        };
                        self.global_env.define_function(f);
                        return true;
                    }
                }
            }
        }
        false
    }

    fn parse_module(
        &self,
        module_path: &str,
        importing_file: Option<&str>,
    ) -> Result<Program, InterpreterError> {
        if module_path == "std" {
            let content = std_module();
            let mut lexer = Lexer::new(content);
            let tokens = lexer
                .tokenize()
                .map_err(|e| InterpreterError::InvalidOperation {
                    message: format!("Lexer error in module {}: {:?}", module_path, e),
                })?;
            let mut parser = Parser::new(&tokens);
            let statements =
                parser
                    .parse_program()
                    .map_err(|e| InterpreterError::InvalidOperation {
                        message: format!("Parser error in module {}: {:?}", module_path, e),
                    })?;
            return Ok(Program { statements });
        }

        // Check for registered built-in libraries
        if let Some(lib) = self.registry.get_library(module_path) {
            println!("DEBUG: Found built-in library: {}", module_path);
            let mut statements = Vec::new();
            for func in &lib.functions {
                statements.push(Statement::FunctionDeclaration {
                    name: func.name.clone(),
                    parameters: func
                        .parameters
                        .iter()
                        .enumerate()
                        .map(|(i, t)| crate::ast::Parameter {
                            name: format!("arg{}", i),
                            param_type: Some(t.clone()),
                            inferred_type: None,
                        })
                        .collect(),
                    body: func.ast_body.clone().unwrap_or_default(),
                    return_type: Some(func.return_type.clone()),
                    is_exported: true,
                });
            }
            return Ok(Program { statements });
        } else {
            println!(
                "DEBUG: Library '{}' not found in registry. Available: {:?}",
                module_path,
                self.registry.get_registered_libs()
            );
        }

        let file_path = if let Some(importing_file) = importing_file {
            // Resolve relative to the importing file's directory
            let importing_dir = std::path::Path::new(importing_file)
                .parent()
                .unwrap_or(std::path::Path::new("."));
            importing_dir
                .join(format!("{}.nlang", module_path))
                .to_string_lossy()
                .to_string()
        } else {
            format!("{}.nlang", module_path)
        };

        let content =
            fs::read_to_string(&file_path).map_err(|_| InterpreterError::InvalidOperation {
                message: format!("Could not read module file: {}", file_path),
            })?;

        let mut lexer = Lexer::new(&content);
        let tokens = lexer
            .tokenize()
            .map_err(|e| InterpreterError::InvalidOperation {
                message: format!("Lexer error in module {}: {:?}", module_path, e),
            })?;

        let mut parser = Parser::new(&tokens);
        let statements =
            parser
                .parse_program()
                .map_err(|e| InterpreterError::InvalidOperation {
                    message: format!("Parser error in module {}: {:?}", module_path, e),
                })?;

        Ok(Program { statements })
    }

    fn execute_function(
        &mut self,
        func: &Function,
        args: &[Value],
    ) -> Result<Value, InterpreterError> {
        // Check for native implementation
        if let Some(native_impl) = &func.native_impl {
            // Convert Values to Exprs for the native function
            let mut expr_args = Vec::new();
            for arg in args {
                let expr = match arg {
                    Value::Integer(i) => Expr::Literal(Literal::Integer(*i)),
                    Value::Float(f) => Expr::Literal(Literal::Float(*f)),
                    Value::Boolean(b) => Expr::Literal(Literal::Boolean(*b)),
                    Value::String(s) => Expr::Literal(Literal::String(s.clone())),
                    Value::Array(_) => {
                        return Err(InterpreterError::InvalidOperation {
                            message: "Arrays not yet supported in native calls".to_string(),
                        });
                    }
                    Value::Vault(_) => {
                        return Err(InterpreterError::InvalidOperation {
                            message: "Vaults not yet supported in native calls".to_string(),
                        });
                    }
                    Value::Pool(_) => {
                        return Err(InterpreterError::InvalidOperation {
                            message: "Pools not yet supported in native calls".to_string(),
                        });
                    }
                    Value::Tree(_) => {
                        return Err(InterpreterError::InvalidOperation {
                            message: "Trees not yet supported in native calls".to_string(),
                        });
                    }
                    Value::Lambda { .. } => {
                        return Err(InterpreterError::InvalidOperation {
                            message: "Lambdas not yet supported in native calls".to_string(),
                        });
                    }
                };
                expr_args.push(expr);
            }

            // Call the native function
            match native_impl(&expr_args) {
                Ok(result_expr) => {
                    // Evaluate the result expression to get a Value
                    // We need a temporary environment for this evaluation, although native functions usually return literals
                    let mut temp_env = self.global_env.clone();
                    return self.evaluate_expression(&result_expr, &mut temp_env);
                }
                Err(msg) => return Err(InterpreterError::InvalidOperation { message: msg }),
            }
        }

        let mut local_env = self.global_env.clone();

        // Bind parameters
        for (param, arg) in func.parameters.iter().zip(args.iter()) {
            local_env.define_variable(param.name.clone(), arg.clone());
        }

        // Execute function body
        for statement in &func.body {
            match self.execute_statement(statement, &mut local_env) {
                Ok(_) => {} // Ignore successful execution of statements within the function body
                Err(InterpreterError::ReturnValue(value)) => return Ok(value),
                Err(e) => return Err(e),
            }
        }

        // Default return value if no explicit return is found
        Ok(Value::Integer(0))
    }

    fn execute_statement(
        &mut self,
        stmt: &Statement,
        env: &mut Environment,
    ) -> Result<(), InterpreterError> {
        match stmt {
            Statement::LetDeclaration {
                name, initializer, ..
            } => {
                if let Some(init_expr) = initializer {
                    let val = self.evaluate_expression(init_expr, env)?;
                    env.define_variable(name.clone(), val);
                } else {
                    // Default initialization for variables without explicit initializer (e.g., to 0 for integers)
                    // This assumes a default type, which might need refinement based on actual type system.
                    env.define_variable(name.clone(), Value::Integer(0));
                }
                Ok(())
            }
            Statement::Return { value } => {
                if let Some(ret_expr) = value {
                    let val = self.evaluate_expression(ret_expr, env)?;
                    Err(InterpreterError::ReturnValue(val))
                } else {
                    // Return default value (e.g., 0 for int) if no value is specified
                    Err(InterpreterError::ReturnValue(Value::Integer(0)))
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.evaluate_expression(condition, env)?;
                if cond_val.to_bool()? {
                    self.execute_statement(then_branch, env)?;
                } else if let Some(else_stmt) = else_branch {
                    self.execute_statement(else_stmt, env)?;
                }
                Ok(())
            }
            Statement::While { condition, body } => {
                loop {
                    let cond_val = self.evaluate_expression(condition, env)?;
                    if !cond_val.to_bool()? {
                        break;
                    }
                    match self.execute_statement(body, env) {
                        Ok(()) => {}                                 // Continue loop
                        Err(InterpreterError::Break) => break,       // Exit loop
                        Err(InterpreterError::Continue) => continue, // Skip to next iteration
                        Err(other) => return Err(other),             // Propagate other errors
                    }
                }
                Ok(())
            }
            Statement::RepeatUntil { body, condition } => {
                loop {
                    // Execute the body first (do-while behavior)
                    match self.execute_statement(body, env) {
                        Ok(()) => {}                                 // Continue loop
                        Err(InterpreterError::Break) => break,       // Exit loop
                        Err(InterpreterError::Continue) => continue, // Skip to next iteration
                        Err(other) => return Err(other),             // Propagate other errors
                    }

                    // Check the condition after executing the body
                    let cond_val = self.evaluate_expression(condition, env)?;
                    if cond_val.to_bool()? {
                        break; // Exit when condition is true
                    }
                }
                Ok(())
            }
            Statement::Loop { body } => {
                loop {
                    match self.execute_statement(body, env) {
                        Ok(()) => {}                                 // Continue loop
                        Err(InterpreterError::Break) => break,       // Exit loop
                        Err(InterpreterError::Continue) => continue, // Skip to next iteration
                        Err(other) => return Err(other),             // Propagate other errors
                    }
                }
                Ok(())
            }
            Statement::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                let mut declared_var_name: Option<String> = None;

                if let Some(init_stmt) = initializer {
                    if let Statement::LetDeclaration { name, .. } = &**init_stmt {
                        declared_var_name = Some(name.clone());
                    }
                    self.execute_statement(init_stmt, env)?;
                }

                loop {
                    let cond_val = if let Some(cond_expr) = condition {
                        self.evaluate_expression(cond_expr, env)?.to_bool()?
                    } else {
                        true // No condition means infinite loop
                    };

                    if !cond_val {
                        break;
                    }

                    match self.execute_statement(body, env) {
                        Ok(()) => {}                           // Continue loop
                        Err(InterpreterError::Break) => break, // Exit loop
                        Err(InterpreterError::Continue) => {
                            if let Some(inc_expr) = increment {
                                self.evaluate_expression(inc_expr, env)?;
                            }
                            continue; // Skip to next iteration
                        }
                        Err(other) => {
                            // Clean up declared variable if an error occurs within the loop body
                            if let Some(name) = declared_var_name {
                                env.variables.remove(&name);
                            }
                            return Err(other);
                        }
                    }

                    if let Some(inc_expr) = increment {
                        self.evaluate_expression(inc_expr, env)?;
                    }
                }

                // Clean up the loop variable after the loop finishes
                if let Some(name) = declared_var_name {
                    env.variables.remove(&name);
                }

                Ok(())
            }
            Statement::FunctionDeclaration { .. } => {
                // Function declarations are handled in the first pass of execute_program_with_path
                Ok(())
            }
            Statement::Expression(expr) => {
                self.evaluate_expression(expr, env)?;
                Ok(())
            }
            Statement::Block { statements } => {
                for stmt in statements {
                    match self.execute_statement(stmt, env) {
                        Ok(()) => {} // Continue executing statements in the block
                        Err(InterpreterError::Break) => return Err(InterpreterError::Break), // Propagate break
                        Err(InterpreterError::Continue) => return Err(InterpreterError::Continue), // Propagate continue
                        Err(other) => return Err(other), // Propagate other errors
                    }
                }
                Ok(())
            }
            Statement::Break => Err(InterpreterError::Break),
            Statement::Continue => Err(InterpreterError::Continue),
            Statement::Pick {
                expression,
                cases,
                default,
            } => {
                let value_to_match = self.evaluate_expression(expression, env)?;
                let mut matched = false;

                for case in cases {
                    for case_value_expr in &case.values {
                        let case_value = self.evaluate_expression(case_value_expr, env)?;
                        if value_to_match == case_value {
                            self.execute_statement(&case.body, env)?;
                            matched = true;
                            break;
                        }
                    }
                    if matched {
                        break;
                    }
                }

                if !matched {
                    if let Some(default_body) = default {
                        self.execute_statement(default_body, env)?;
                    }
                }

                Ok(())
            }
            _ => {
                // Handle other statement types as needed
                Ok(())
            }
        }
    }

    fn evaluate_expression(
        &mut self,
        expr: &Expr,
        env: &mut Environment,
    ) -> Result<Value, InterpreterError> {
        match expr {
            Expr::Literal(literal) => {
                match literal {
                    Literal::Integer(i) => Ok(Value::Integer(*i)),
                    Literal::I32(i) => Ok(Value::Integer(*i as i64)), // Convert i32 to i64 for interpreter
                    Literal::I8(i) => Ok(Value::Integer(*i as i64)),
                    Literal::I16(i) => Ok(Value::Integer(*i as i64)),
                    Literal::I64(i) => Ok(Value::Integer(*i)),
                    Literal::ISize(i) => Ok(Value::Integer(*i as i64)),
                    Literal::U8(i) => Ok(Value::Integer(*i as i64)),
                    Literal::U16(i) => Ok(Value::Integer(*i as i64)),
                    Literal::U32(i) => Ok(Value::Integer(*i as i64)),
                    Literal::U64(i) => Ok(Value::Integer(*i as i64)),
                    Literal::USize(i) => Ok(Value::Integer(*i as i64)),
                    Literal::Float(f) => Ok(Value::Float(*f)),
                    Literal::Boolean(b) => Ok(Value::Boolean(*b)),
                    Literal::String(s) => Ok(Value::String(s.clone())),
                    Literal::Null => Ok(Value::Integer(0)), // Default null to 0
                }
            }
            Expr::Variable(name) => env.get_variable(name),
            Expr::Binary {
                left,
                operator,
                right,
            } => {
                let left_val = self.evaluate_expression(left, env)?;
                let right_val = self.evaluate_expression(right, env)?;
                self.evaluate_binary_op(&left_val, operator, &right_val)
            }
            Expr::Unary { operator, operand } => {
                let val = self.evaluate_expression(operand, env)?;
                self.evaluate_unary_op(operator, &val)
            }
            Expr::Call { callee, arguments } => {
                let func_name = match callee.as_ref() {
                    Expr::Variable(name) => name.clone(),
                    Expr::Get { object, name } => {
                        let obj_val = self.evaluate_expression(object, env)?;
                        match obj_val {
                            Value::String(s) => match name.as_str() {
                                "upper" => {
                                    if !arguments.is_empty() {
                                        return Err(InterpreterError::InvalidOperation {
                                            message: "String.upper() takes 0 arguments".to_string(),
                                        });
                                    }
                                    return Ok(Value::String(s.to_uppercase()));
                                }
                                "lower" => {
                                    if !arguments.is_empty() {
                                        return Err(InterpreterError::InvalidOperation {
                                            message: "String.lower() takes 0 arguments".to_string(),
                                        });
                                    }
                                    return Ok(Value::String(s.to_lowercase()));
                                }
                                "trim" => {
                                    if !arguments.is_empty() {
                                        return Err(InterpreterError::InvalidOperation {
                                            message: "String.trim() takes 0 arguments".to_string(),
                                        });
                                    }
                                    return Ok(Value::String(s.trim().to_string()));
                                }
                                "split" => {
                                    if arguments.len() != 1 {
                                        return Err(InterpreterError::InvalidOperation {
                                            message: "String.split(delim) expects 1 argument"
                                                .to_string(),
                                        });
                                    }
                                    let delim_v = self.evaluate_expression(&arguments[0], env)?;
                                    let delim = match delim_v {
                                        Value::String(d) => d,
                                        _ => {
                                            return Err(InterpreterError::InvalidOperation {
                                                message: "split delimiter must be string"
                                                    .to_string(),
                                            });
                                        }
                                    };
                                    let parts = string_lib::split(&s, &delim);
                                    return Ok(Value::Array(parts));
                                }
                                "replace" => {
                                    if arguments.len() != 2 {
                                        return Err(InterpreterError::InvalidOperation {
                                            message: "String.replace(from,to) expects 2 arguments"
                                                .to_string(),
                                        });
                                    }
                                    let from = match self.evaluate_expression(&arguments[0], env)? {
                                        Value::String(d) => d,
                                        _ => {
                                            return Err(InterpreterError::InvalidOperation {
                                                message: "replace 'from' must be string"
                                                    .to_string(),
                                            });
                                        }
                                    };
                                    let to = match self.evaluate_expression(&arguments[1], env)? {
                                        Value::String(d) => d,
                                        _ => {
                                            return Err(InterpreterError::InvalidOperation {
                                                message: "replace 'to' must be string".to_string(),
                                            });
                                        }
                                    };
                                    let out = string_lib::replace(&s, &from, &to);
                                    return Ok(Value::String(out));
                                }
                                "substring" => {
                                    if arguments.len() != 2 {
                                        return Err(InterpreterError::InvalidOperation {
                                            message:
                                                "String.substring(start,end) expects 2 arguments"
                                                    .to_string(),
                                        });
                                    }
                                    let start = match self
                                        .evaluate_expression(&arguments[0], env)?
                                    {
                                        Value::Integer(i) => i as usize,
                                        _ => {
                                            return Err(InterpreterError::InvalidOperation {
                                                message: "substring start must be int".to_string(),
                                            });
                                        }
                                    };
                                    let end = match self.evaluate_expression(&arguments[1], env)? {
                                        Value::Integer(i) => i as usize,
                                        _ => {
                                            return Err(InterpreterError::InvalidOperation {
                                                message: "substring end must be int".to_string(),
                                            });
                                        }
                                    };
                                    let out =
                                        string_lib::substring(&s, start, end).map_err(|e| {
                                            InterpreterError::InvalidOperation { message: e }
                                        })?;
                                    return Ok(Value::String(out));
                                }
                                "regex" => {
                                    if arguments.len() != 1 {
                                        return Err(InterpreterError::InvalidOperation {
                                            message: "String.regex(pattern) expects 1 argument"
                                                .to_string(),
                                        });
                                    }
                                    let pat = match self.evaluate_expression(&arguments[0], env)? {
                                        Value::String(p) => p,
                                        _ => {
                                            return Err(InterpreterError::InvalidOperation {
                                                message: "regex pattern must be string".to_string(),
                                            });
                                        }
                                    };
                                    let matches =
                                        string_lib::regex_matches(&s, &pat).map_err(|e| {
                                            InterpreterError::InvalidOperation { message: e }
                                        })?;
                                    return Ok(Value::Array(matches));
                                }
                                "contains" => {
                                    if arguments.len() != 1 {
                                        return Err(InterpreterError::InvalidOperation {
                                            message: "String.contains() expects 1 argument"
                                                .to_string(),
                                        });
                                    }
                                    let needle = self.evaluate_expression(&arguments[0], env)?;
                                    match needle {
                                        Value::String(n) => {
                                            return Ok(Value::Boolean(s.contains(&n)));
                                        }
                                        _ => {
                                            return Err(InterpreterError::InvalidOperation {
                                                message:
                                                    "String.contains() argument must be string"
                                                        .to_string(),
                                            });
                                        }
                                    }
                                }
                                _ => {
                                    return Err(InterpreterError::InvalidOperation {
                                        message: format!("Unknown string method: {}", name),
                                    });
                                }
                            },
                            Value::Pool(mut set) => match name.as_str() {
                                "add" => {
                                    if arguments.len() != 1 {
                                        return Err(InterpreterError::InvalidOperation {
                                            message: "Pool.add() expects 1 argument".to_string(),
                                        });
                                    }
                                    let v = self.evaluate_expression(&arguments[0], env)?;
                                    let sv = SimpleValue::from_value(v)?;
                                    set.insert(sv);
                                    return Ok(Value::Integer(set.len() as i64));
                                }
                                _ => {
                                    return Err(InterpreterError::InvalidOperation {
                                        message: format!("Unknown pool method: {}", name),
                                    });
                                }
                            },
                            Value::Tree(node_box) => {
                                let mut node = *node_box;
                                match name.as_str() {
                                    "add" => {
                                        if arguments.len() != 1 {
                                            return Err(InterpreterError::InvalidOperation {
                                                message: "Tree.add() expects 1 argument"
                                                    .to_string(),
                                            });
                                        }
                                        let v = self.evaluate_expression(&arguments[0], env)?;
                                        let sv = SimpleValue::from_value(v)?;
                                        node.add_child(sv);
                                        return Ok(Value::Integer(node.children.len() as i64));
                                    }
                                    _ => {
                                        return Err(InterpreterError::InvalidOperation {
                                            message: format!("Unknown tree method: {}", name),
                                        });
                                    }
                                }
                            }
                            Value::Array(a) => match name.as_str() {
                                "len" => {
                                    if !arguments.is_empty() {
                                        return Err(InterpreterError::InvalidOperation {
                                            message: "Array.len() takes 0 arguments".to_string(),
                                        });
                                    }
                                    return Ok(Value::Integer(a.len() as i64));
                                }
                                "join" => {
                                    if arguments.len() != 1 {
                                        return Err(InterpreterError::InvalidOperation {
                                            message: "Array.join(delim) expects 1 argument"
                                                .to_string(),
                                        });
                                    }
                                    let delim =
                                        match self.evaluate_expression(&arguments[0], env)? {
                                            Value::String(d) => d,
                                            _ => {
                                                return Err(InterpreterError::InvalidOperation {
                                                    message: "join delimiter must be string"
                                                        .to_string(),
                                                });
                                            }
                                        };
                                    let out = string_lib::join(&a, &delim).map_err(|e| {
                                        InterpreterError::InvalidOperation { message: e }
                                    })?;
                                    return Ok(Value::String(out));
                                }
                                _ => {
                                    return Err(InterpreterError::InvalidOperation {
                                        message: format!("Unknown array method: {}", name),
                                    });
                                }
                            },
                            _ => {
                                if let Expr::Variable(namespace_name) = object.as_ref() {
                                    format!("{}.{}", namespace_name, name)
                                } else {
                                    return Err(InterpreterError::InvalidOperation {
                                        message: "Complex function calls not yet supported"
                                            .to_string(),
                                    });
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(InterpreterError::InvalidOperation {
                            message: "Complex function calls not yet supported".to_string(),
                        });
                    }
                };

                // Handle built-in and constructors
                match func_name.as_str() {
                    "vault" => {
                        if arguments.len() > 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "vault() takes 0 or 1 arguments".to_string(),
                            });
                        }
                        let map = std::collections::HashMap::new();
                        Ok(Value::Vault(map))
                    }
                    "pool" => {
                        let mut set = std::collections::HashSet::new();
                        for arg_expr in arguments {
                            let v = self.evaluate_expression(arg_expr, env)?;
                            let sv = SimpleValue::from_value(v)?;
                            set.insert(sv);
                        }
                        Ok(Value::Pool(set))
                    }
                    "tree" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "tree(root) expects 1 argument".to_string(),
                            });
                        }
                        let root_v = self.evaluate_expression(&arguments[0], env)?;
                        let root = SimpleValue::from_value(root_v)?;
                        Ok(Value::Tree(Box::new(TreeNode::new(root))))
                    }
                    "print" => {
                        let mut evaluated_args = Vec::new();
                        for arg_expr in arguments {
                            evaluated_args.push(self.evaluate_expression(arg_expr, env)?);
                        }

                        if !evaluated_args.is_empty() {
                            if let Value::String(format_str) = &evaluated_args[0] {
                                if arguments.len() > 1 && format_str.contains("[]") {
                                    // Placeholder logic
                                    let parts: Vec<&str> = format_str.split("[]").collect();
                                    let mut arg_idx = 1;
                                    for (i, part) in parts.iter().enumerate() {
                                        print!("{}", part);
                                        if i < parts.len() - 1 {
                                            if arg_idx < evaluated_args.len() {
                                                print!("{}", evaluated_args[arg_idx]);
                                                arg_idx += 1;
                                            } else {
                                                print!("[]");
                                            }
                                        }
                                    }
                                    // Print remaining arguments if any
                                    while arg_idx < evaluated_args.len() {
                                        print!(" {}", evaluated_args[arg_idx]);
                                        arg_idx += 1;
                                    }
                                } else {
                                    // Old logic: print all args space-separated
                                    for (i, arg) in evaluated_args.iter().enumerate() {
                                        if i > 0 {
                                            print!(" ");
                                        }
                                        print!("{}", arg);
                                    }
                                }
                            } else {
                                // First arg not a string, print all args space-separated
                                for (i, arg) in evaluated_args.iter().enumerate() {
                                    if i > 0 {
                                        print!(" ");
                                    }
                                    print!("{}", arg);
                                }
                            }
                        }
                        use std::io::{self, Write};
                        io::stdout().flush().unwrap();
                        Ok(Value::Integer(0))
                    }
                    "println" => {
                        let mut evaluated_args = Vec::new();
                        for arg_expr in arguments {
                            evaluated_args.push(self.evaluate_expression(arg_expr, env)?);
                        }

                        if !evaluated_args.is_empty() {
                            if let Value::String(format_str) = &evaluated_args[0] {
                                if arguments.len() > 1 && format_str.contains("[]") {
                                    // Placeholder logic
                                    let parts: Vec<&str> = format_str.split("[]").collect();
                                    let mut arg_idx = 1;
                                    for (i, part) in parts.iter().enumerate() {
                                        print!("{}", part);
                                        if i < parts.len() - 1 {
                                            if arg_idx < evaluated_args.len() {
                                                print!("{}", evaluated_args[arg_idx]);
                                                arg_idx += 1;
                                            } else {
                                                print!("[]");
                                            }
                                        }
                                    }
                                    // Print remaining arguments if any
                                    while arg_idx < evaluated_args.len() {
                                        print!(" {}", evaluated_args[arg_idx]);
                                        arg_idx += 1;
                                    }
                                } else {
                                    // Old logic: print all args space-separated
                                    for (i, arg) in evaluated_args.iter().enumerate() {
                                        if i > 0 {
                                            print!(" ");
                                        }
                                        print!("{}", arg);
                                    }
                                }
                            } else {
                                // First arg not a string, print all args space-separated
                                for (i, arg) in evaluated_args.iter().enumerate() {
                                    if i > 0 {
                                        print!(" ");
                                    }
                                    print!("{}", arg);
                                }
                            }
                        }
                        println!();
                        Ok(Value::Integer(0))
                    }
                    "len" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "len function requires 1 argument".to_string(),
                            });
                        }
                        let arg = self.evaluate_expression(&arguments[0], env)?;
                        match arg {
                            Value::String(s) => Ok(Value::Integer(s.len() as i64)),
                            Value::Array(a) => Ok(Value::Integer(a.len() as i64)),
                            _ => Err(InterpreterError::InvalidOperation {
                                message: "len function can only be used on strings and arrays"
                                    .to_string(),
                            }),
                        }
                    }
                    "input" => {
                        if arguments.len() > 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "input function takes 0 or 1 arguments".to_string(),
                            });
                        }
                        if arguments.len() == 1 {
                            let prompt = self.evaluate_expression(&arguments[0], env)?;
                            print!("{}", prompt);
                            use std::io::{self, Write};
                            io::stdout().flush().unwrap();
                        }

                        let mut input = String::new();
                        match std::io::stdin().read_line(&mut input) {
                            Ok(_) => {
                                // Remove trailing newline
                                if input.ends_with('\n') {
                                    input.pop();
                                    if input.ends_with('\r') {
                                        input.pop();
                                    }
                                }
                                Ok(Value::String(input))
                            }
                            Err(e) => Err(InterpreterError::InvalidOperation {
                                message: format!("Failed to read line from stdin: {}", e),
                            }),
                        }
                    }
                    "add" => {
                        if arguments.len() != 2 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "add function requires 2 arguments".to_string(),
                            });
                        }
                        let arg1 = self.evaluate_expression(&arguments[0], env)?;
                        let arg2 = self.evaluate_expression(&arguments[1], env)?;
                        self.evaluate_binary_op(&arg1, &BinaryOperator::Plus, &arg2)
                    }
                    "multiply" => {
                        if arguments.len() != 2 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "multiply function requires 2 arguments".to_string(),
                            });
                        }
                        let arg1 = self.evaluate_expression(&arguments[0], env)?;
                        let arg2 = self.evaluate_expression(&arguments[1], env)?;
                        self.evaluate_binary_op(&arg1, &BinaryOperator::Star, &arg2)
                    }
                    "str" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "str function requires 1 argument".to_string(),
                            });
                        }
                        let arg = self.evaluate_expression(&arguments[0], env)?;
                        match arg {
                            Value::Integer(i) => Ok(Value::String(i.to_string())),
                            Value::Float(f) => Ok(Value::String(f.to_string())),
                            Value::Boolean(b) => Ok(Value::String(b.to_string())),
                            Value::String(s) => Ok(Value::String(s)), // Already a string
                            Value::Array(_) => Err(InterpreterError::InvalidOperation {
                                message: "Cannot convert array to string".to_string(),
                            }),
                            Value::Lambda { .. } => Err(InterpreterError::InvalidOperation {
                                message: "Cannot convert lambda to string".to_string(),
                            }),
                            _ => Err(InterpreterError::InvalidOperation {
                                message: "Cannot convert value to string".to_string(),
                            }),
                        }
                    }
                    "sha256" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "sha256 function requires 1 argument (string)".to_string(),
                            });
                        }
                        let arg = self.evaluate_expression(&arguments[0], env)?;
                        match arg {
                            Value::String(s) => {
                                use sha2::{Digest, Sha256};
                                let mut hasher = Sha256::new();
                                hasher.update(s.as_bytes());
                                let result = hasher.finalize();
                                let hex_str = hex::encode(result);
                                Ok(Value::String(hex_str))
                            }
                            Value::Array(arr) => {
                                // Treat array of integers as bytes
                                use sha2::{Digest, Sha256};
                                let mut hasher = Sha256::new();
                                let mut buf: Vec<u8> = Vec::with_capacity(arr.len());
                                for v in arr {
                                    if let Value::Integer(i) = v {
                                        buf.push((i as i64 & 0xFF) as u8);
                                    } else {
                                        return Err(InterpreterError::InvalidOperation {
                                            message: "sha256 array must contain integers"
                                                .to_string(),
                                        });
                                    }
                                }
                                hasher.update(&buf);
                                let result = hasher.finalize();
                                let hex_str = hex::encode(result);
                                Ok(Value::String(hex_str))
                            }
                            _ => Err(InterpreterError::InvalidOperation {
                                message: "sha256 expects string or array of integers".to_string(),
                            }),
                        }
                    }
                    "sha256_random" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "sha256_random function requires 1 argument (length)"
                                    .to_string(),
                            });
                        }
                        let n_val = self.evaluate_expression(&arguments[0], env)?;
                        let n = match n_val {
                            Value::Integer(i) => i,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "sha256_random expects integer length".to_string(),
                                });
                            }
                        };
                        if n <= 0 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "sha256_random length must be > 0".to_string(),
                            });
                        }
                        use rand::RngCore;
                        use sha2::{Digest, Sha256};
                        let mut buf = vec![0u8; n as usize];
                        rand::rngs::OsRng.fill_bytes(&mut buf);
                        let mut hasher = Sha256::new();
                        hasher.update(&buf);
                        let result = hasher.finalize();
                        let hex_str = hex::encode(result);
                        Ok(Value::String(hex_str))
                    }
                    "int" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "int function requires 1 argument".to_string(),
                            });
                        }
                        let arg = self.evaluate_expression(&arguments[0], env)?;
                        match arg {
                            Value::Integer(i) => Ok(Value::Integer(i)), // Already an integer
                            Value::Float(f) => Ok(Value::Integer(f as i64)),
                            Value::Boolean(b) => Ok(Value::Integer(if b { 1 } else { 0 })),
                            Value::String(s) => match s.parse::<i64>() {
                                Ok(i) => Ok(Value::Integer(i)),
                                Err(_) => Err(InterpreterError::InvalidOperation {
                                    message: format!("Cannot convert '{}' to integer", s),
                                }),
                            },
                            Value::Array(_) => Err(InterpreterError::InvalidOperation {
                                message: "Cannot convert array to integer".to_string(),
                            }),
                            Value::Lambda { .. } => Err(InterpreterError::InvalidOperation {
                                message: "Cannot convert lambda to integer".to_string(),
                            }),
                            _ => Err(InterpreterError::InvalidOperation {
                                message: "Cannot convert value to integer".to_string(),
                            }),
                        }
                    }
                    "float" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "float function requires 1 argument".to_string(),
                            });
                        }
                        let arg = self.evaluate_expression(&arguments[0], env)?;
                        match arg {
                            Value::Integer(i) => Ok(Value::Float(i as f64)),
                            Value::Float(f) => Ok(Value::Float(f)), // Already a float
                            Value::Boolean(b) => Ok(Value::Float(if b { 1.0 } else { 0.0 })),
                            Value::String(s) => match s.parse::<f64>() {
                                Ok(f) => Ok(Value::Float(f)),
                                Err(_) => Err(InterpreterError::InvalidOperation {
                                    message: format!("Cannot convert '{}' to float", s),
                                }),
                            },
                            Value::Array(_) => Err(InterpreterError::InvalidOperation {
                                message: "Cannot convert array to float".to_string(),
                            }),
                            Value::Lambda { .. } => Err(InterpreterError::InvalidOperation {
                                message: "Cannot convert lambda to float".to_string(),
                            }),
                            _ => Err(InterpreterError::InvalidOperation {
                                message: "Cannot convert value to float".to_string(),
                            }),
                        }
                    }
                    "abs" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "abs function requires 1 argument".to_string(),
                            });
                        }
                        let arg = self.evaluate_expression(&arguments[0], env)?;
                        match arg {
                            Value::Integer(i) => Ok(Value::Integer(i.abs())),
                            _ => Err(InterpreterError::TypeMismatch {
                                expected: "integer".to_string(),
                                actual: arg.type_name().to_string(),
                            }),
                        }
                    }
                    "abs_float" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "abs_float function requires 1 argument".to_string(),
                            });
                        }
                        let arg = self.evaluate_expression(&arguments[0], env)?;
                        match arg {
                            Value::Float(f) => Ok(Value::Float(f.abs())),
                            Value::Integer(i) => Ok(Value::Float((i as f64).abs())),
                            _ => Err(InterpreterError::TypeMismatch {
                                expected: "float or integer".to_string(),
                                actual: arg.type_name().to_string(),
                            }),
                        }
                    }
                    "timestamp" => {
                        if !arguments.is_empty() {
                            return Err(InterpreterError::InvalidOperation {
                                message: "timestamp() takes 0 arguments".into(),
                            });
                        }
                        let secs = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64;
                        Ok(Value::Integer(secs))
                    }
                    "timestamp_ms" => {
                        let ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as i64;
                        Ok(Value::Integer(ms))
                    }
                    "timestamp_us" => {
                        let us = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_micros() as i64;
                        Ok(Value::Integer(us))
                    }
                    "timestamp_ns" => {
                        let ns = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos() as i64;
                        Ok(Value::Integer(ns))
                    }
                    "sleep" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "sleep(seconds) expects 1 argument".into(),
                            });
                        }
                        let s = self.evaluate_expression(&arguments[0], env)?;
                        let secs = match s {
                            Value::Integer(i) => i,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "sleep expects integer".into(),
                                });
                            }
                        };
                        std::thread::sleep(std::time::Duration::from_secs(secs.max(0) as u64));
                        Ok(Value::Integer(0))
                    }
                    "sleep_ms" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "sleep_ms(ms) expects 1 argument".into(),
                            });
                        }
                        let s = self.evaluate_expression(&arguments[0], env)?;
                        let ms = match s {
                            Value::Integer(i) => i,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "sleep_ms expects integer".into(),
                                });
                            }
                        };
                        std::thread::sleep(std::time::Duration::from_millis(ms.max(0) as u64));
                        Ok(Value::Integer(0))
                    }
                    "sleep_ns" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "sleep_ns(ns) expects 1 argument".into(),
                            });
                        }
                        let s = self.evaluate_expression(&arguments[0], env)?;
                        let ns = match s {
                            Value::Integer(i) => i,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "sleep_ns expects integer".into(),
                                });
                            }
                        };
                        std::thread::sleep(std::time::Duration::from_nanos(ns.max(0) as u64));
                        Ok(Value::Integer(0))
                    }
                    "now" | "now_local" => {
                        let dt = chrono::Local::now();
                        Ok(Value::String(dt.format("%Y-%m-%d %H:%M:%S").to_string()))
                    }
                    "now_utc" => {
                        let dt = chrono::Utc::now();
                        Ok(Value::String(dt.format("%Y-%m-%d %H:%M:%S").to_string()))
                    }
                    "year" => Ok(Value::Integer(chrono::Local::now().year() as i64)),
                    "month" => Ok(Value::Integer(chrono::Local::now().month() as i64)),
                    "day" => Ok(Value::Integer(chrono::Local::now().day() as i64)),
                    "weekday" => Ok(Value::Integer(
                        chrono::Local::now().weekday().num_days_from_monday() as i64,
                    )),
                    "hour" => Ok(Value::Integer(chrono::Local::now().hour() as i64)),
                    "minute" => Ok(Value::Integer(chrono::Local::now().minute() as i64)),
                    "second" => Ok(Value::Integer(chrono::Local::now().second() as i64)),
                    "nanosecond" => Ok(Value::Integer(chrono::Local::now().nanosecond() as i64)),
                    "format" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "format(fmt) expects 1 argument".into(),
                            });
                        }
                        let fmtv = self.evaluate_expression(&arguments[0], env)?;
                        let fmt = match fmtv {
                            Value::String(s) => s,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "format expects string".into(),
                                });
                            }
                        };
                        Ok(Value::String(chrono::Local::now().format(&fmt).to_string()))
                    }
                    "time_to_string" => Ok(Value::String(
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    )),
                    "to_local" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "to_local(secs) expects 1 argument".into(),
                            });
                        }
                        let sv = self.evaluate_expression(&arguments[0], env)?;
                        let secs = match sv {
                            Value::Integer(i) => i,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "to_local expects seconds".into(),
                                });
                            }
                        };
                        let dt = chrono::Local.timestamp_opt(secs, 0).single().ok_or(
                            InterpreterError::InvalidOperation {
                                message: "invalid epoch".into(),
                            },
                        )?;
                        Ok(Value::String(dt.format("%Y-%m-%d %H:%M:%S").to_string()))
                    }
                    "to_utc" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "to_utc(secs) expects 1 argument".into(),
                            });
                        }
                        let sv = self.evaluate_expression(&arguments[0], env)?;
                        let secs = match sv {
                            Value::Integer(i) => i,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "to_utc expects seconds".into(),
                                });
                            }
                        };
                        let dt = chrono::Utc.timestamp_opt(secs, 0).single().ok_or(
                            InterpreterError::InvalidOperation {
                                message: "invalid epoch".into(),
                            },
                        )?;
                        Ok(Value::String(dt.format("%Y-%m-%d %H:%M:%S").to_string()))
                    }
                    "from_timestamp" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "from_timestamp(secs) expects 1 argument".into(),
                            });
                        }
                        let sv = self.evaluate_expression(&arguments[0], env)?;
                        let secs = match sv {
                            Value::Integer(i) => i,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "from_timestamp expects seconds".into(),
                                });
                            }
                        };
                        let dt = chrono::Utc.timestamp_opt(secs, 0).single().ok_or(
                            InterpreterError::InvalidOperation {
                                message: "invalid epoch".into(),
                            },
                        )?;
                        Ok(Value::String(dt.format("%Y-%m-%d %H:%M:%S").to_string()))
                    }
                    "from_timestamp_ms" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "from_timestamp_ms(ms) expects 1 argument".into(),
                            });
                        }
                        let sv = self.evaluate_expression(&arguments[0], env)?;
                        let ms = match sv {
                            Value::Integer(i) => i,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "from_timestamp_ms expects integer".into(),
                                });
                            }
                        };
                        let secs = ms / 1000;
                        let dt = chrono::Utc.timestamp_opt(secs, 0).single().ok_or(
                            InterpreterError::InvalidOperation {
                                message: "invalid epoch".into(),
                            },
                        )?;
                        Ok(Value::String(dt.format("%Y-%m-%d %H:%M:%S").to_string()))
                    }
                    "parse" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "parse(date) expects 1 argument".into(),
                            });
                        }
                        let sv = self.evaluate_expression(&arguments[0], env)?;
                        let s = match sv {
                            Value::String(t) => t,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "parse expects string".into(),
                                });
                            }
                        };
                        let dt =
                            chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(|e| {
                                InterpreterError::InvalidOperation {
                                    message: e.to_string(),
                                }
                            })?;
                        let dt =
                            dt.and_hms_opt(0, 0, 0)
                                .ok_or(InterpreterError::InvalidOperation {
                                    message: "invalid time".into(),
                                })?;
                        let ts = chrono::Local
                            .from_local_datetime(&dt)
                            .single()
                            .ok_or(InterpreterError::InvalidOperation {
                                message: "invalid local".into(),
                            })?
                            .timestamp();
                        Ok(Value::Integer(ts))
                    }
                    "parse_rfc3339" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "parse_rfc3339(s) expects 1 argument".into(),
                            });
                        }
                        let sv = self.evaluate_expression(&arguments[0], env)?;
                        let s = match sv {
                            Value::String(t) => t,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "parse_rfc3339 expects string".into(),
                                });
                            }
                        };
                        let dt = chrono::DateTime::parse_from_rfc3339(&s).map_err(|e| {
                            InterpreterError::InvalidOperation {
                                message: e.to_string(),
                            }
                        })?;
                        Ok(Value::Integer(dt.timestamp()))
                    }
                    "parse_rfc2822" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "parse_rfc2822(s) expects 1 argument".into(),
                            });
                        }
                        let sv = self.evaluate_expression(&arguments[0], env)?;
                        let s = match sv {
                            Value::String(t) => t,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "parse_rfc2822 expects string".into(),
                                });
                            }
                        };
                        let dt = chrono::DateTime::parse_from_rfc2822(&s).map_err(|e| {
                            InterpreterError::InvalidOperation {
                                message: e.to_string(),
                            }
                        })?;
                        Ok(Value::Integer(dt.timestamp()))
                    }
                    "duration_from_seconds" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "duration_from_seconds(x) expects 1 argument".into(),
                            });
                        }
                        let sv = self.evaluate_expression(&arguments[0], env)?;
                        let s = match sv {
                            Value::Integer(i) => i,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "expects integer".into(),
                                });
                            }
                        };
                        let mut map = std::collections::HashMap::new();
                        map.insert("nanos".to_string(), Value::Integer(s * 1_000_000_000));
                        Ok(Value::Vault(map))
                    }
                    "duration_from_millis" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "duration_from_millis(x) expects 1 argument".into(),
                            });
                        }
                        let sv = self.evaluate_expression(&arguments[0], env)?;
                        let ms = match sv {
                            Value::Integer(i) => i,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "expects integer".into(),
                                });
                            }
                        };
                        let mut map = std::collections::HashMap::new();
                        map.insert("nanos".to_string(), Value::Integer(ms * 1_000_000));
                        Ok(Value::Vault(map))
                    }
                    "duration_from_nanos" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "duration_from_nanos(x) expects 1 argument".into(),
                            });
                        }
                        let sv = self.evaluate_expression(&arguments[0], env)?;
                        let ns = match sv {
                            Value::Integer(i) => i,
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "expects integer".into(),
                                });
                            }
                        };
                        let mut map = std::collections::HashMap::new();
                        map.insert("nanos".to_string(), Value::Integer(ns));
                        Ok(Value::Vault(map))
                    }
                    "duration_as_secs" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "duration_as_secs(d) expects 1 argument".into(),
                            });
                        }
                        let sv = self.evaluate_expression(&arguments[0], env)?;
                        if let Value::Vault(map) = sv {
                            if let Some(Value::Integer(ns)) = map.get("nanos") {
                                Ok(Value::Integer(ns / 1_000_000_000))
                            } else {
                                Err(InterpreterError::InvalidOperation {
                                    message: "duration missing nanos".into(),
                                })
                            }
                        } else {
                            Err(InterpreterError::InvalidOperation {
                                message: "expects duration vault".into(),
                            })
                        }
                    }
                    "duration_as_millis" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "duration_as_millis(d) expects 1 argument".into(),
                            });
                        }
                        let sv = self.evaluate_expression(&arguments[0], env)?;
                        if let Value::Vault(map) = sv {
                            if let Some(Value::Integer(ns)) = map.get("nanos") {
                                Ok(Value::Integer(ns / 1_000_000))
                            } else {
                                Err(InterpreterError::InvalidOperation {
                                    message: "duration missing nanos".into(),
                                })
                            }
                        } else {
                            Err(InterpreterError::InvalidOperation {
                                message: "expects duration vault".into(),
                            })
                        }
                    }
                    "duration_add" => {
                        if arguments.len() != 2 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "duration_add(a,b) expects 2 arguments".into(),
                            });
                        }
                        let a = self.evaluate_expression(&arguments[0], env)?;
                        let b = self.evaluate_expression(&arguments[1], env)?;
                        if let (Value::Vault(ma), Value::Vault(mb)) = (a, b) {
                            let na = if let Some(Value::Integer(v)) = ma.get("nanos") {
                                *v
                            } else {
                                0
                            };
                            let nb = if let Some(Value::Integer(v)) = mb.get("nanos") {
                                *v
                            } else {
                                0
                            };
                            let mut map = std::collections::HashMap::new();
                            map.insert("nanos".to_string(), Value::Integer(na + nb));
                            Ok(Value::Vault(map))
                        } else {
                            Err(InterpreterError::InvalidOperation {
                                message: "expects duration vaults".into(),
                            })
                        }
                    }
                    "duration_sub" => {
                        if arguments.len() != 2 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "duration_sub(a,b) expects 2 arguments".into(),
                            });
                        }
                        let a = self.evaluate_expression(&arguments[0], env)?;
                        let b = self.evaluate_expression(&arguments[1], env)?;
                        if let (Value::Vault(ma), Value::Vault(mb)) = (a, b) {
                            let na = if let Some(Value::Integer(v)) = ma.get("nanos") {
                                *v
                            } else {
                                0
                            };
                            let nb = if let Some(Value::Integer(v)) = mb.get("nanos") {
                                *v
                            } else {
                                0
                            };
                            let mut map = std::collections::HashMap::new();
                            map.insert("nanos".to_string(), Value::Integer(na - nb));
                            Ok(Value::Vault(map))
                        } else {
                            Err(InterpreterError::InvalidOperation {
                                message: "expects duration vaults".into(),
                            })
                        }
                    }
                    "timer_start" => {
                        let ns = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos() as i64;
                        let mut map = std::collections::HashMap::new();
                        map.insert("start_ns".to_string(), Value::Integer(ns));
                        Ok(Value::Vault(map))
                    }
                    "timer_elapsed" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "timer_elapsed(timer) expects 1 argument".into(),
                            });
                        }
                        let tv = self.evaluate_expression(&arguments[0], env)?;
                        if let Value::Vault(map) = tv {
                            if let Some(Value::Integer(start)) = map.get("start_ns") {
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_nanos() as i64;
                                Ok(Value::Integer((now - start) / 1_000_000))
                            } else {
                                Err(InterpreterError::InvalidOperation {
                                    message: "timer missing start_ns".into(),
                                })
                            }
                        } else {
                            Err(InterpreterError::InvalidOperation {
                                message: "expects timer vault".into(),
                            })
                        }
                    }
                    "timer_reset" => {
                        if arguments.len() != 1 {
                            return Err(InterpreterError::InvalidOperation {
                                message: "timer_reset(timer) expects 1 argument".into(),
                            });
                        }
                        let _ = self.evaluate_expression(&arguments[0], env)?;
                        let ns = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos() as i64;
                        let mut map = std::collections::HashMap::new();
                        map.insert("start_ns".to_string(), Value::Integer(ns));
                        Ok(Value::Vault(map))
                    }
                    _ => {
                        // Check if it's a registered library function
                        let library_func = {
                            let parts: Vec<&str> = func_name.split('.').collect();
                            if parts.len() == 2 {
                                let lib_name = parts[0];
                                let func_short_name = parts[1];
                                if let Some(lib) = self.registry.get_library(lib_name) {
                                    lib.functions
                                        .iter()
                                        .find(|f| f.name == func_short_name)
                                        .cloned()
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        };

                        if let Some(func) = library_func {
                            if let Some(impl_fn) = func.implementation {
                                let mut evaluated_args = Vec::new();
                                for arg_expr in arguments {
                                    evaluated_args.push(self.evaluate_expression(arg_expr, env)?);
                                }

                                // Convert Values to Exprs for native call
                                let mut expr_args = Vec::new();
                                for val in evaluated_args {
                                    expr_args.push(Self::value_to_expr(val)?);
                                }

                                // Call native function
                                let result_expr = (impl_fn)(&expr_args)
                                    .map_err(|e| InterpreterError::RuntimeError { message: e })?;

                                // Convert result Expr back to Value
                                return Self::expr_to_value_simple(result_expr);
                            }
                        }

                        if !self.ensure_function_loaded(&func_name) {
                            let parts: Vec<&str> = func_name.split('.').collect();
                            if parts.len() == 2 {
                                let _ = self.ensure_function_loaded_qualified(parts[0], parts[1]);
                            } else {
                                let _ = self.ensure_function_loaded_direct(&func_name);
                            }
                        }
                        if let Ok(func) = env.get_function(&func_name) {
                            let func = func.clone();
                            let mut args = Vec::new();
                            for arg_expr in arguments {
                                args.push(self.evaluate_expression(arg_expr, env)?);
                            }
                            return self.execute_function(&func, &args);
                        }
                        if let Ok(func) = self.global_env.get_function(&func_name) {
                            let func = func.clone();
                            env.define_function(func.clone());
                            let mut args = Vec::new();
                            for arg_expr in arguments {
                                args.push(self.evaluate_expression(arg_expr, env)?);
                            }
                            return self.execute_function(&func, &args);
                        }
                        Err(InterpreterError::FunctionNotFound { name: func_name })
                    }
                }
            }
            Expr::Get { object, name } => {
                // Handle namespace access like math.PI
                if let Expr::Variable(obj_name) = object.as_ref() {
                    if obj_name == "math" {
                        match name.as_str() {
                            "PI" => Ok(Value::Float(std::f64::consts::PI)),
                            _ => Err(InterpreterError::InvalidOperation {
                                message: format!("Unknown math property: {}", name),
                            }),
                        }
                    } else {
                        Err(InterpreterError::InvalidOperation {
                            message: format!("Unknown namespace: {}", obj_name),
                        })
                    }
                } else {
                    Err(InterpreterError::InvalidOperation {
                        message: "Complex object access not yet supported".to_string(),
                    })
                }
            }
            Expr::Assign { name, value } => {
                let val = self.evaluate_expression(value, env)?;
                env.set_variable(name.clone(), val.clone())?;
                Ok(val)
            }
            Expr::AssignIndex {
                sequence,
                index,
                value,
            } => {
                // Evaluate the sequence (array), index, and value
                let sequence_val = self.evaluate_expression(sequence, env)?;
                let index_val = self.evaluate_expression(index, env)?;
                let value_val = self.evaluate_expression(value, env)?;

                match (sequence_val, index_val) {
                    (Value::Vault(mut map), Value::String(key)) => {
                        map.insert(key.clone(), value_val.clone());
                        match sequence.as_ref() {
                            Expr::Variable(var_name) => {
                                env.set_variable(var_name.clone(), Value::Vault(map))?;
                            }
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "Complex vault assignment not yet supported"
                                        .to_string(),
                                });
                            }
                        }
                        Ok(value_val)
                    }
                    (Value::Array(mut arr), Value::Integer(idx)) => {
                        // Check bounds
                        if idx < 0 || idx >= arr.len() as i64 {
                            return Err(InterpreterError::IndexOutOfBounds {
                                index: idx,
                                length: arr.len(),
                            });
                        }

                        // Update the array element
                        arr[idx as usize] = value_val.clone();

                        // Update the array in the environment
                        // Handle the sequence based on its type to support more complex assignments
                        match sequence.as_ref() {
                            Expr::Variable(var_name) => {
                                // Simple case: arr[index] = value
                                env.set_variable(var_name.clone(), Value::Array(arr))?;
                            }
                            Expr::Index {
                                sequence: inner_sequence,
                                index: inner_index,
                            } => {
                                // Handle nested case: arr[expr1][expr2] = value
                                // First evaluate the inner array
                                let container_val =
                                    self.evaluate_expression(inner_sequence, env)?;
                                let idx_val = self.evaluate_expression(inner_index, env)?;

                                if let (
                                    Value::Array(mut container_arr),
                                    Value::Integer(container_idx),
                                ) = (container_val, idx_val)
                                {
                                    if container_idx < 0
                                        || container_idx >= container_arr.len() as i64
                                    {
                                        return Err(InterpreterError::IndexOutOfBounds {
                                            index: container_idx,
                                            length: container_arr.len(),
                                        });
                                    }

                                    // Replace the target array at the container index
                                    container_arr[container_idx as usize] = Value::Array(arr);

                                    // Update the container array in the environment
                                    if let Expr::Variable(container_var_name) =
                                        inner_sequence.as_ref()
                                    {
                                        env.set_variable(
                                            container_var_name.clone(),
                                            Value::Array(container_arr),
                                        )?;
                                    } else {
                                        return Err(InterpreterError::InvalidOperation {
                                            message:
                                                "Complex nested array assignment not yet supported"
                                                    .to_string(),
                                        });
                                    }
                                } else {
                                    return Err(InterpreterError::InvalidOperation {
                                        message: "Container must be an array with integer index"
                                            .to_string(),
                                    });
                                }
                            }
                            _ => {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "Complex array assignment not yet supported"
                                        .to_string(),
                                });
                            }
                        }

                        Ok(value_val)
                    }
                    _ => Err(InterpreterError::InvalidOperation {
                        message: "Array assignment only supported for arrays with integer indices"
                            .to_string(),
                    }),
                }
            }
            Expr::ArrayLiteral { elements } => {
                // Evaluate all elements in the array literal
                let mut evaluated_elements = Vec::new();
                for element in elements {
                    evaluated_elements.push(self.evaluate_expression(element, env)?);
                }
                Ok(Value::Array(evaluated_elements))
            }
            Expr::VaultLiteral { entries } => {
                let mut map = std::collections::HashMap::new();
                for (k, vexpr) in entries {
                    let v = self.evaluate_expression(&vexpr, env)?;
                    map.insert(k.clone(), v);
                }
                Ok(Value::Vault(map))
            }
            Expr::PoolLiteral { elements } => {
                let mut set = std::collections::HashSet::new();
                for e in elements {
                    let v = self.evaluate_expression(e, env)?;
                    let sv = crate::interpreter::value::SimpleValue::from_value(v)?;
                    set.insert(sv);
                }
                Ok(Value::Pool(set))
            }
            Expr::TreeLiteral { root, children } => {
                let rv = self.evaluate_expression(root, env)?;
                let rs = crate::interpreter::value::SimpleValue::from_value(rv)?;
                let mut node = crate::interpreter::value::TreeNode::new(rs);
                for c in children {
                    let cv = self.evaluate_expression(&c, env)?;
                    let cs = crate::interpreter::value::SimpleValue::from_value(cv)?;
                    node.add_child(cs);
                }
                Ok(Value::Tree(Box::new(node)))
            }
            Expr::Index { sequence, index } => {
                // Evaluate the sequence (array) and index
                let sequence_val = self.evaluate_expression(sequence, env)?;
                let index_val = self.evaluate_expression(index, env)?;

                match (sequence_val, index_val) {
                    (Value::Vault(map), Value::String(key)) => {
                        if let Some(v) = map.get(&key) {
                            Ok(v.clone())
                        } else {
                            Ok(Value::Integer(0))
                        }
                    }
                    (Value::Array(arr), Value::Integer(idx)) => {
                        // Check bounds
                        if idx < 0 || idx >= arr.len() as i64 {
                            return Err(InterpreterError::IndexOutOfBounds {
                                index: idx,
                                length: arr.len(),
                            });
                        }
                        Ok(arr[idx as usize].clone())
                    }
                    _ => Err(InterpreterError::InvalidOperation {
                        message: "Indexing supported for arrays[int] and vault[string]".to_string(),
                    }),
                }
            }
            _ => Err(InterpreterError::InvalidOperation {
                message: "Expression type not yet supported".to_string(),
            }),
        }
    }

    fn evaluate_binary_op(
        &self,
        left: &Value,
        op: &BinaryOperator,
        right: &Value,
    ) -> Result<Value, InterpreterError> {
        match op {
            BinaryOperator::Plus => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => {
                    if let Some(sum) = a.checked_add(*b) {
                        Ok(Value::Integer(sum))
                    } else {
                        Ok(Value::Float(*a as f64 + *b as f64))
                    }
                }
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a + *b as f64)),
                (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "numeric or string".to_string(),
                    actual: format!("{} and {}", left.type_name(), right.type_name()),
                }),
            },
            BinaryOperator::Minus => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => {
                    if let Some(diff) = a.checked_sub(*b) {
                        Ok(Value::Integer(diff))
                    } else {
                        Ok(Value::Float(*a as f64 - *b as f64))
                    }
                }
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a - *b as f64)),
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "numeric".to_string(),
                    actual: format!("{} and {}", left.type_name(), right.type_name()),
                }),
            },
            BinaryOperator::Star => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => {
                    if let Some(prod) = a.checked_mul(*b) {
                        Ok(Value::Integer(prod))
                    } else {
                        Ok(Value::Float(*a as f64 * *b as f64))
                    }
                }
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a * *b as f64)),
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "numeric".to_string(),
                    actual: format!("{} and {}", left.type_name(), right.type_name()),
                }),
            },
            BinaryOperator::Slash => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => {
                    if *b == 0 {
                        Err(InterpreterError::DivisionByZero)
                    } else {
                        Ok(Value::Float(*a as f64 / *b as f64))
                    }
                }
                (Value::Float(a), Value::Float(b)) => {
                    if *b == 0.0 {
                        Err(InterpreterError::DivisionByZero)
                    } else {
                        Ok(Value::Float(a / b))
                    }
                }
                (Value::Integer(a), Value::Float(b)) => {
                    if *b == 0.0 {
                        Err(InterpreterError::DivisionByZero)
                    } else {
                        Ok(Value::Float(*a as f64 / b))
                    }
                }
                (Value::Float(a), Value::Integer(b)) => {
                    if *b == 0 {
                        Err(InterpreterError::DivisionByZero)
                    } else {
                        Ok(Value::Float(a / *b as f64))
                    }
                }
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "numeric".to_string(),
                    actual: format!("{} and {}", left.type_name(), right.type_name()),
                }),
            },
            BinaryOperator::Percent => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => {
                    if *b == 0 {
                        Err(InterpreterError::DivisionByZero)
                    } else {
                        Ok(Value::Integer(a % b))
                    }
                }
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "integer".to_string(),
                    actual: format!("{} and {}", left.type_name(), right.type_name()),
                }),
            },
            BinaryOperator::EqualEqual => Ok(Value::Boolean(left == right)),
            BinaryOperator::NotEqual => Ok(Value::Boolean(left != right)),
            BinaryOperator::Less => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a < b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a < b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) < *b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a < (*b as f64))),
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "numeric".to_string(),
                    actual: format!("{} and {}", left.type_name(), right.type_name()),
                }),
            },
            BinaryOperator::LessEqual => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a <= b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a <= b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) <= *b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a <= (*b as f64))),
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "numeric".to_string(),
                    actual: format!("{} and {}", left.type_name(), right.type_name()),
                }),
            },
            BinaryOperator::Greater => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a > b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a > b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) > *b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a > (*b as f64))),
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "numeric".to_string(),
                    actual: format!("{} and {}", left.type_name(), right.type_name()),
                }),
            },
            BinaryOperator::GreaterEqual => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a >= b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a >= b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) >= *b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a >= (*b as f64))),
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "numeric".to_string(),
                    actual: format!("{} and {}", left.type_name(), right.type_name()),
                }),
            },
            BinaryOperator::And => Ok(Value::Boolean(left.to_bool()? && right.to_bool()?)),
            BinaryOperator::Or => Ok(Value::Boolean(left.to_bool()? || right.to_bool()?)),
            BinaryOperator::BitAnd => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a & b)),
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "integer".to_string(),
                    actual: format!("{} and {}", left.type_name(), right.type_name()),
                }),
            },
            BinaryOperator::BitOr => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a | b)),
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "integer".to_string(),
                    actual: format!("{} and {}", left.type_name(), right.type_name()),
                }),
            },
            BinaryOperator::BitXor => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a ^ b)),
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "integer".to_string(),
                    actual: format!("{} and {}", left.type_name(), right.type_name()),
                }),
            },
            BinaryOperator::ShiftLeft => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a << (*b as u32))),
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "integer".to_string(),
                    actual: format!("{} and {}", left.type_name(), right.type_name()),
                }),
            },
            BinaryOperator::ShiftRight => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a >> (*b as u32))),
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "integer".to_string(),
                    actual: format!("{} and {}", left.type_name(), right.type_name()),
                }),
            },
        }
    }

    fn evaluate_unary_op(
        &self,
        op: &UnaryOperator,
        operand: &Value,
    ) -> Result<Value, InterpreterError> {
        match op {
            UnaryOperator::Negate => match operand {
                Value::Integer(i) => Ok(Value::Integer(-i)),
                Value::Float(f) => Ok(Value::Float(-f)),
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "numeric".to_string(),
                    actual: operand.type_name().to_string(),
                }),
            },
            UnaryOperator::Not => Ok(Value::Boolean(!operand.to_bool()?)),
            UnaryOperator::BitNot => match operand {
                Value::Integer(i) => Ok(Value::Integer(!i)),
                _ => Err(InterpreterError::TypeMismatch {
                    expected: "integer".to_string(),
                    actual: operand.type_name().to_string(),
                }),
            },
        }
    }

    fn value_to_expr(value: Value) -> Result<Expr, InterpreterError> {
        match value {
            Value::Integer(i) => Ok(Expr::Literal(Literal::Integer(i))),
            Value::Float(f) => Ok(Expr::Literal(Literal::Float(f))),
            Value::Boolean(b) => Ok(Expr::Literal(Literal::Boolean(b))),
            Value::String(s) => Ok(Expr::Literal(Literal::String(s))),
            Value::Array(arr) => {
                let mut elements = Vec::new();
                for v in arr {
                    elements.push(Self::value_to_expr(v)?);
                }
                Ok(Expr::ArrayLiteral { elements })
            }
            Value::Vault(_) | Value::Pool(_) | Value::Tree(_) | Value::Lambda { .. } => {
                Err(InterpreterError::InvalidOperation {
                    message: format!(
                        "Cannot convert {} to expression for native call",
                        value.type_name()
                    ),
                })
            }
        }
    }

    fn expr_to_value_simple(expr: Expr) -> Result<Value, InterpreterError> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Integer(i) => Ok(Value::Integer(i)),
                Literal::Float(f) => Ok(Value::Float(f)),
                Literal::Boolean(b) => Ok(Value::Boolean(b)),
                Literal::String(s) => Ok(Value::String(s)),
                Literal::Null => Ok(Value::Integer(0)),
                Literal::I8(i) => Ok(Value::Integer(i as i64)),
                Literal::I16(i) => Ok(Value::Integer(i as i64)),
                Literal::I32(i) => Ok(Value::Integer(i as i64)),
                Literal::I64(i) => Ok(Value::Integer(i)),
                Literal::ISize(i) => Ok(Value::Integer(i as i64)),
                Literal::U8(i) => Ok(Value::Integer(i as i64)),
                Literal::U16(i) => Ok(Value::Integer(i as i64)),
                Literal::U32(i) => Ok(Value::Integer(i as i64)),
                Literal::U64(i) => Ok(Value::Integer(i as i64)),
                Literal::USize(i) => Ok(Value::Integer(i as i64)),
            },
            Expr::ArrayLiteral { elements } => {
                let mut vals = Vec::new();
                for e in elements {
                    vals.push(Self::expr_to_value_simple(e)?);
                }
                Ok(Value::Array(vals))
            }
            _ => Err(InterpreterError::InvalidOperation {
                message: "Native function returned non-literal expression".to_string(),
            }),
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    #[test]
    fn test_simple_arithmetic() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            statements: vec![
                Statement::LetDeclaration {
                    name: "x".to_string(),
                    initializer: Some(Expr::Literal(Literal::Integer(5))),
                    var_type: None,
                    is_mutable: false,
                    is_exported: false,
                },
                Statement::Return {
                    value: Some(Box::new(Expr::Variable("x".to_string()))),
                },
            ],
        };

        let result = interpreter.execute_program(&program);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
    }

    #[test]
    fn test_string_methods() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            statements: vec![Statement::FunctionDeclaration {
                name: "main".to_string(),
                parameters: vec![],
                body: vec![Statement::Return {
                    value: Some(Box::new(Expr::Call {
                        callee: Box::new(Expr::Get {
                            object: Box::new(Expr::Literal(Literal::String("  Abc ".to_string()))),
                            name: "upper".to_string(),
                        }),
                        arguments: vec![],
                    })),
                }],
                return_type: Some(Type::String),
                is_exported: false,
            }],
        };
        let result = interpreter.execute_program(&program);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_array_len_method() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            statements: vec![Statement::FunctionDeclaration {
                name: "main".to_string(),
                parameters: vec![],
                body: vec![
                    Statement::LetDeclaration {
                        name: "a".to_string(),
                        initializer: Some(Expr::ArrayLiteral {
                            elements: vec![
                                Expr::Literal(Literal::Integer(1)),
                                Expr::Literal(Literal::Integer(2)),
                                Expr::Literal(Literal::Integer(3)),
                            ],
                        }),
                        var_type: Some(Type::Array(Box::new(Type::Integer), 3)),
                        is_mutable: false,
                        is_exported: false,
                    },
                    Statement::Return {
                        value: Some(Box::new(Expr::Call {
                            callee: Box::new(Expr::Get {
                                object: Box::new(Expr::Variable("a".to_string())),
                                name: "len".to_string(),
                            }),
                            arguments: vec![],
                        })),
                    },
                ],
                return_type: Some(Type::Integer),
                is_exported: false,
            }],
        };
        let result = interpreter.execute_program(&program);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
    }

    #[test]
    fn test_vault_index_assignment() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            statements: vec![Statement::FunctionDeclaration {
                name: "main".to_string(),
                parameters: vec![],
                body: vec![
                    Statement::LetDeclaration {
                        name: "users".to_string(),
                        initializer: Some(Expr::Call {
                            callee: Box::new(Expr::Variable("vault".to_string())),
                            arguments: vec![],
                        }),
                        var_type: Some(Type::Vault(
                            Box::new(Type::String),
                            Box::new(Type::Unknown),
                        )),
                        is_mutable: false,
                        is_exported: false,
                    },
                    Statement::Expression(Expr::AssignIndex {
                        sequence: Box::new(Expr::Variable("users".to_string())),
                        index: Box::new(Expr::Literal(Literal::String("Alice".to_string()))),
                        value: Box::new(Expr::Literal(Literal::Integer(25))),
                    }),
                    Statement::Return {
                        value: Some(Box::new(Expr::Index {
                            sequence: Box::new(Expr::Variable("users".to_string())),
                            index: Box::new(Expr::Literal(Literal::String("Alice".to_string()))),
                        })),
                    },
                ],
                return_type: Some(Type::Integer),
                is_exported: false,
            }],
        };
        let result = interpreter.execute_program(&program);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 25);
    }

    #[test]
    fn test_pool_add_method() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            statements: vec![Statement::FunctionDeclaration {
                name: "main".to_string(),
                parameters: vec![],
                body: vec![
                    Statement::LetDeclaration {
                        name: "primes".to_string(),
                        initializer: Some(Expr::Call {
                            callee: Box::new(Expr::Variable("pool".to_string())),
                            arguments: vec![
                                Expr::Literal(Literal::Integer(2)),
                                Expr::Literal(Literal::Integer(3)),
                                Expr::Literal(Literal::Integer(5)),
                            ],
                        }),
                        var_type: Some(Type::Pool(Box::new(Type::Integer))),
                        is_mutable: false,
                        is_exported: false,
                    },
                    Statement::Expression(Expr::Call {
                        callee: Box::new(Expr::Get {
                            object: Box::new(Expr::Variable("primes".to_string())),
                            name: "add".to_string(),
                        }),
                        arguments: vec![Expr::Literal(Literal::Integer(7))],
                    }),
                    Statement::Return {
                        value: Some(Box::new(Expr::Literal(Literal::Integer(0)))),
                    },
                ],
                return_type: Some(Type::Integer),
                is_exported: false,
            }],
        };
        let result = interpreter.execute_program(&program);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_tree_add_method() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            statements: vec![Statement::FunctionDeclaration {
                name: "main".to_string(),
                parameters: vec![],
                body: vec![
                    Statement::LetDeclaration {
                        name: "family".to_string(),
                        initializer: Some(Expr::Call {
                            callee: Box::new(Expr::Variable("tree".to_string())),
                            arguments: vec![Expr::Literal(Literal::String("root".to_string()))],
                        }),
                        var_type: Some(Type::Tree(Box::new(Type::String))),
                        is_mutable: false,
                        is_exported: false,
                    },
                    Statement::Expression(Expr::Call {
                        callee: Box::new(Expr::Get {
                            object: Box::new(Expr::Variable("family".to_string())),
                            name: "add".to_string(),
                        }),
                        arguments: vec![Expr::Literal(Literal::String("child1".to_string()))],
                    }),
                    Statement::Return {
                        value: Some(Box::new(Expr::Literal(Literal::Integer(0)))),
                    },
                ],
                return_type: Some(Type::Integer),
                is_exported: false,
            }],
        };
        let result = interpreter.execute_program(&program);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}
