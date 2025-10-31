use crate::ast::{Program, Statement, Expr, Literal, BinaryOperator, UnaryOperator, Type};
use std::collections::HashMap;

#[derive(Debug)]
pub struct LLVMCodeGenError {
    pub message: String,
}

impl std::fmt::Display for LLVMCodeGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LLVM CodeGen error: {}", self.message)
    }
}

impl std::error::Error for LLVMCodeGenError {}

pub struct LLVMCodeGenerator {
    module_name: String,
    #[allow(dead_code)]
    functions: Vec<String>,
    #[allow(dead_code)]
    current_function: String,
    basic_blocks: Vec<String>,
    variables: HashMap<String, String>,
    string_constants: HashMap<String, String>,
    temp_counter: usize,
    label_counter: usize,
    string_counter: usize,
    // Loop context for break/continue
    loop_stack: Vec<LoopContext>,
}

#[derive(Clone)]
struct LoopContext {
    continue_label: String,
    break_label: String,
}

impl LLVMCodeGenerator {
    pub fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            functions: Vec::new(),
            current_function: String::new(),
            basic_blocks: Vec::new(),
            variables: HashMap::new(),
            string_constants: HashMap::new(),
            temp_counter: 0,
            label_counter: 0,
            string_counter: 0,
            loop_stack: Vec::new(),
        }
    }

    pub fn generate_program(&mut self, program: &Program) -> Result<String, LLVMCodeGenError> {
        let mut output = String::new();
        
        // Module header
        output.push_str(&format!("; ModuleID = '{}'\n", self.module_name));
        output.push_str("target datalayout = \"e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n");
        output.push_str("target triple = \"x86_64-pc-windows-msvc\"\n\n");

        // First pass: collect all string literals
        self.collect_string_literals(program);

        // Declare external functions
        output.push_str("; External function declarations\n");
        output.push_str("declare i32 @printf(i8*, ...)\n");
        output.push_str("declare i32 @puts(i8*)\n");
        output.push_str("declare void @llvm.memcpy.p0i8.p0i8.i64(i8*, i8*, i64, i1)\n\n");

        // String constants for print functions
        output.push_str("; String constants\n");
        output.push_str("@.str = private unnamed_addr constant [4 x i8] c\"%s\\0A\\00\", align 1\n");
        output.push_str("@.str.1 = private unnamed_addr constant [3 x i8] c\"%s\\00\", align 1\n");
        output.push_str("@.str.2 = private unnamed_addr constant [4 x i8] c\"%d\\0A\\00\", align 1\n");
        output.push_str("@.str.3 = private unnamed_addr constant [3 x i8] c\"%d\\00\", align 1\n");
        output.push_str("@.str.4 = private unnamed_addr constant [4 x i8] c\"%f\\0A\\00\", align 1\n");
        output.push_str("@.str.5 = private unnamed_addr constant [3 x i8] c\"%f\\00\", align 1\n");
        output.push_str("@.str.bool_true = private unnamed_addr constant [5 x i8] c\"true\\00\", align 1\n");
        output.push_str("@.str.bool_false = private unnamed_addr constant [6 x i8] c\"false\\00\", align 1\n");

        // Generate string constants
        for (content, name) in &self.string_constants {
            let escaped_content = content.replace("\\", "\\\\").replace("\"", "\\22");
            output.push_str(&format!("{} = private unnamed_addr constant [{} x i8] c\"{}\\00\", align 1\n", 
                name, content.len() + 1, escaped_content));
        }
        output.push('\n');

        // Generate functions
        for stmt in &program.statements {
            if let Statement::FunctionDeclaration { .. } = stmt {
                let func_ir = self.generate_function(stmt)?;
                output.push_str(&func_ir);
                output.push('\n');
            }
        }

        Ok(output)
    }

    fn collect_string_literals(&mut self, program: &Program) {
        for stmt in &program.statements {
            self.collect_strings_from_statement(stmt);
        }
    }

    fn collect_strings_from_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::FunctionDeclaration { body, .. } => {
                for body_stmt in body {
                    self.collect_strings_from_statement(body_stmt);
                }
            }
            Statement::Expression(expr) => {
                self.collect_strings_from_expression(expr);
            }
            Statement::Return { value } => {
                if let Some(expr) = value {
                    self.collect_strings_from_expression(expr);
                }
            }
            Statement::LetDeclaration { initializer, .. } => {
                if let Some(expr) = initializer {
                    self.collect_strings_from_expression(expr);
                }
            }
            Statement::While { condition, body } => {
                self.collect_strings_from_expression(condition);
                self.collect_strings_from_statement(body);
            }
            Statement::If { condition, then_branch, else_branch } => {
                self.collect_strings_from_expression(condition);
                self.collect_strings_from_statement(then_branch);
                if let Some(else_stmt) = else_branch {
                    self.collect_strings_from_statement(else_stmt);
                }
            }
            Statement::Block { statements } => {
                for statement in statements {
                    self.collect_strings_from_statement(statement);
                }
            }
            Statement::Break | Statement::Continue => {
                // No strings to collect from break/continue
            }
            _ => {}
        }
    }

    fn collect_strings_from_expression(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(Literal::String(s)) => {
                if !self.string_constants.contains_key(s) {
                    let name = format!("@.str.{}", self.string_counter + 4); // +4 to avoid conflicts with printf format strings
                    self.string_constants.insert(s.clone(), name);
                    self.string_counter += 1;
                }
            }
            Expr::Binary { left, right, .. } => {
                self.collect_strings_from_expression(left);
                self.collect_strings_from_expression(right);
            }
            Expr::Call { arguments, .. } => {
                for arg in arguments {
                    self.collect_strings_from_expression(arg);
                }
            }
            _ => {}
        }
}

    fn generate_function(&mut self, stmt: &Statement) -> Result<String, LLVMCodeGenError> {
        if let Statement::FunctionDeclaration { name, parameters, body, return_type, .. } = stmt {
            let mut func_ir = String::new();
            
            // Reset state for new function
            self.variables.clear();
            self.temp_counter = 0;
            self.label_counter = 0;
            self.basic_blocks.clear();
            
            // Function signature
            let ret_type = match return_type.as_ref().unwrap_or(&Type::Void) {
                Type::Integer => "i64",
                Type::Float => "double",
                Type::Boolean => "i1",
                Type::String => "i8*",
                Type::Void => "void",
                _ => "i64", // Default
            };

            func_ir.push_str(&format!("define {} @{}(", ret_type, name));
            
            // Parameters
            for (i, param) in parameters.iter().enumerate() {
                if i > 0 {
                    func_ir.push_str(", ");
                }
                let param_type = match param.param_type {
                    Type::Integer => "i64",
                    Type::Float => "double",
                    Type::Boolean => "i1",
                    Type::String => "i8*",
                    _ => "i64",
                };
                func_ir.push_str(&format!("{} %{}", param_type, param.name));
                self.variables.insert(param.name.clone(), format!("%{}", param.name));
            }
            
            func_ir.push_str(") {\n");
            func_ir.push_str("entry:\n");

            // Generate function body
            for stmt in body {
                let stmt_ir = self.generate_statement(stmt)?;
                func_ir.push_str(&stmt_ir);
            }

            // Add default return if needed
            if !body.iter().any(|s| matches!(s, Statement::Return { .. })) {
                match return_type.as_ref().unwrap_or(&Type::Void) {
                    Type::Void => func_ir.push_str("  ret void\n"),
                    Type::Integer => func_ir.push_str("  ret i64 0\n"),
                    Type::Float => func_ir.push_str("  ret double 0.0\n"),
                    Type::Boolean => func_ir.push_str("  ret i1 false\n"),
                    Type::String => func_ir.push_str("  ret i8* null\n"),
                    _ => func_ir.push_str("  ret i64 0\n"),
                }
            }

            func_ir.push_str("}\n");
            Ok(func_ir)
        } else {
            Err(LLVMCodeGenError {
                message: "Expected function declaration".to_string(),
            })
        }
    }

    fn generate_statement(&mut self, stmt: &Statement) -> Result<String, LLVMCodeGenError> {
        match stmt {
            Statement::LetDeclaration { name, initializer, .. } => {
                let mut stmt_ir = String::new();
                
                if let Some(init_expr) = initializer {
                    let (expr_ir, expr_result) = self.generate_expression(init_expr)?;
                    stmt_ir.push_str(&expr_ir);
                    
                    // Allocate space for the variable
                    let var_name = format!("%{}", name);
                    stmt_ir.push_str(&format!("  {} = alloca i64, align 8\n", var_name));
                    stmt_ir.push_str(&format!("  store i64 {}, i64* {}, align 8\n", expr_result, var_name));
                    
                    self.variables.insert(name.clone(), var_name);
                } else {
                    // Default initialization
                    let var_name = format!("%{}", name);
                    stmt_ir.push_str(&format!("  {} = alloca i64, align 8\n", var_name));
                    stmt_ir.push_str(&format!("  store i64 0, i64* {}, align 8\n", var_name));
                    
                    self.variables.insert(name.clone(), var_name);
                }
                
                Ok(stmt_ir)
            }
            Statement::Expression(expr) => {
                let (expr_ir, _) = self.generate_expression(expr)?;
                Ok(expr_ir)
            }
            Statement::Return { value } => {
                let mut stmt_ir = String::new();
                
                if let Some(expr) = value {
                    let (expr_ir, expr_result) = self.generate_expression(expr)?;
                    stmt_ir.push_str(&expr_ir);
                    stmt_ir.push_str(&format!("  ret i64 {}\n", expr_result));
                } else {
                    stmt_ir.push_str("  ret void\n");
                }
                
                Ok(stmt_ir)
            }
            Statement::While { condition, body } => {
                let mut stmt_ir = String::new();
                
                // Generate unique labels for the loop
                let loop_start = self.next_label();
                let loop_body = self.next_label();
                let loop_end = self.next_label();
                
                // Push loop context for break/continue
                self.loop_stack.push(LoopContext {
                    continue_label: loop_start.clone(),
                    break_label: loop_end.clone(),
                });
                
                // Jump to loop start
                stmt_ir.push_str(&format!("  br label %{}\n", loop_start));
                
                // Loop start: check condition
                stmt_ir.push_str(&format!("{}:\n", loop_start));
                let (cond_ir, cond_result) = self.generate_expression(condition)?;
                stmt_ir.push_str(&cond_ir);
                stmt_ir.push_str(&format!("  br i1 {}, label %{}, label %{}\n", cond_result, loop_body, loop_end));
                
                // Loop body
                stmt_ir.push_str(&format!("{}:\n", loop_body));
                let body_ir = self.generate_statement(body)?;
                stmt_ir.push_str(&body_ir);
                stmt_ir.push_str(&format!("  br label %{}\n", loop_start));
                
                // Loop end
                stmt_ir.push_str(&format!("{}:\n", loop_end));
                
                // Pop loop context
                self.loop_stack.pop();
                
                Ok(stmt_ir)
            }
            Statement::If { condition, then_branch, else_branch } => {
                let mut stmt_ir = String::new();
                
                let then_label = self.next_label();
                let else_label = self.next_label();
                let end_label = self.next_label();
                
                // Generate condition
                let (cond_ir, cond_result) = self.generate_expression(condition)?;
                stmt_ir.push_str(&cond_ir);
                
                if else_branch.is_some() {
                    stmt_ir.push_str(&format!("  br i1 {}, label %{}, label %{}\n", cond_result, then_label, else_label));
                } else {
                    stmt_ir.push_str(&format!("  br i1 {}, label %{}, label %{}\n", cond_result, then_label, end_label));
                }
                
                // Then branch
                stmt_ir.push_str(&format!("{}:\n", then_label));
                let then_ir = self.generate_statement(then_branch)?;
                stmt_ir.push_str(&then_ir);
                stmt_ir.push_str(&format!("  br label %{}\n", end_label));
                
                // Else branch (if exists)
                if let Some(else_stmt) = else_branch {
                    stmt_ir.push_str(&format!("{}:\n", else_label));
                    let else_ir = self.generate_statement(else_stmt)?;
                    stmt_ir.push_str(&else_ir);
                    stmt_ir.push_str(&format!("  br label %{}\n", end_label));
                }
                
                // End label
                stmt_ir.push_str(&format!("{}:\n", end_label));
                
                Ok(stmt_ir)
            }
            Statement::Block { statements } => {
                let mut stmt_ir = String::new();
                for statement in statements {
                    let sub_ir = self.generate_statement(statement)?;
                    stmt_ir.push_str(&sub_ir);
                }
                Ok(stmt_ir)
            }
            Statement::Break => {
                if let Some(loop_ctx) = self.loop_stack.last() {
                    Ok(format!("  br label %{}\n", loop_ctx.break_label))
                } else {
                    Err(LLVMCodeGenError {
                        message: "Break statement outside of loop".to_string(),
                    })
                }
            }
            Statement::Continue => {
                if let Some(loop_ctx) = self.loop_stack.last() {
                    Ok(format!("  br label %{}\n", loop_ctx.continue_label))
                } else {
                    Err(LLVMCodeGenError {
                        message: "Continue statement outside of loop".to_string(),
                    })
                }
            }
            _ => Ok(String::new()), // Skip other statements for now
        }
    }

    fn generate_expression(&mut self, expr: &Expr) -> Result<(String, String), LLVMCodeGenError> {
        match expr {
            Expr::Literal(literal) => {
                match literal {
                    Literal::Integer(value) => Ok((String::new(), value.to_string())),
                    Literal::Float(value) => Ok((String::new(), value.to_string())),
                    Literal::Boolean(value) => Ok((String::new(), if *value { "1" } else { "0" }.to_string())),
                    Literal::String(value) => {
                        // Get the string constant name
                        if let Some(str_name) = self.string_constants.get(value) {
                            Ok((String::new(), str_name.clone()))
                        } else {
                            Err(LLVMCodeGenError {
                                message: format!("String constant not found: {}", value),
                            })
                        }
                    }
                    Literal::Null => Ok((String::new(), "null".to_string())),
                }
            }
            Expr::Variable(name) => {
                if let Some(var_ref) = self.variables.get(name).cloned() {
                    let temp_name = self.next_temp();
                    let load_ir = format!("  {} = load i64, i64* {}, align 8\n", temp_name, var_ref);
                    Ok((load_ir, temp_name))
                } else {
                    Err(LLVMCodeGenError {
                        message: format!("Undefined variable: {}", name),
                    })
                }
            }
            Expr::Binary { left, operator, right } => {
                let (left_ir, left_result) = self.generate_expression(left)?;
                let (right_ir, right_result) = self.generate_expression(right)?;
                
                let temp_name = self.next_temp();
                let mut expr_ir = String::new();
                expr_ir.push_str(&left_ir);
                expr_ir.push_str(&right_ir);
                
                let op_instr = match operator {
                    BinaryOperator::Plus => "add",
                    BinaryOperator::Minus => "sub",
                    BinaryOperator::Star => "mul",
                    BinaryOperator::Slash => "sdiv",
                    BinaryOperator::Percent => "srem",
                    BinaryOperator::EqualEqual => "icmp eq",
                    BinaryOperator::NotEqual => "icmp ne",
                    BinaryOperator::Less => "icmp slt",
                    BinaryOperator::LessEqual => "icmp sle",
                    BinaryOperator::Greater => "icmp sgt",
                    BinaryOperator::GreaterEqual => "icmp sge",
                    BinaryOperator::And => "and",
                    BinaryOperator::Or => "or",
                };
                
                expr_ir.push_str(&format!("  {} = {} i64 {}, {}\n", temp_name, op_instr, left_result, right_result));
                Ok((expr_ir, temp_name))
            }
            Expr::Unary { operator, operand } => {
                let (operand_ir, operand_result) = self.generate_expression(operand)?;
                let temp_name = self.next_temp();
                let mut expr_ir = String::new();
                expr_ir.push_str(&operand_ir);
                
                match operator {
                    UnaryOperator::Negate => {
                        expr_ir.push_str(&format!("  {} = sub i64 0, {}\n", temp_name, operand_result));
                    }
                    UnaryOperator::Not => {
                        expr_ir.push_str(&format!("  {} = xor i1 {}, true\n", temp_name, operand_result));
                    }
                }
                
                Ok((expr_ir, temp_name))
            }
            Expr::Call { callee, arguments } => {
                if let Expr::Variable(func_name) = callee.as_ref() {
                    match func_name.as_str() {
                        "print" | "println" => {
                            if arguments.len() != 1 {
                                return Err(LLVMCodeGenError {
                                    message: format!("{} expects exactly 1 argument", func_name),
                                });
                            }
                            
                            let (arg_ir, arg_result) = self.generate_expression(&arguments[0])?;
                            let mut call_ir = String::new();
                            call_ir.push_str(&arg_ir);
                            
                            match &arguments[0] {
                                // String literals
                                Expr::Literal(Literal::String(s)) => {
                                    if let Some(str_name) = self.string_constants.get(s) {
                                        let format_str = if func_name == "println" { "@.str" } else { "@.str.1" };
                                        call_ir.push_str(&format!("  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([4 x i8], [4 x i8]* {}, i32 0, i32 0), i8* getelementptr inbounds ([{} x i8], [{} x i8]* {}, i32 0, i32 0))\n", 
                                            format_str, s.len() + 1, s.len() + 1, str_name));
                                    } else {
                                        return Err(LLVMCodeGenError {
                                            message: format!("String constant not found: {}", s),
                                        });
                                    }
                                }
                                // Integer literals
                                Expr::Literal(Literal::Integer(_)) => {
                                    let format_str = if func_name == "println" { "@.str.2" } else { "@.str.3" };
                                    call_ir.push_str(&format!("  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([4 x i8], [4 x i8]* {}, i32 0, i32 0), i64 {})\n", 
                                        format_str, arg_result));
                                }
                                // Float literals
                                Expr::Literal(Literal::Float(_)) => {
                                    let format_str = if func_name == "println" { "@.str.4" } else { "@.str.5" };
                                    call_ir.push_str(&format!("  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([4 x i8], [4 x i8]* {}, i32 0, i32 0), double {})\n", 
                                        format_str, arg_result));
                                }
                                // Boolean literals
                                Expr::Literal(Literal::Boolean(b)) => {
                                    let bool_str = if *b { "@.str.bool_true" } else { "@.str.bool_false" };
                                    let format_str = if func_name == "println" { "@.str" } else { "@.str.1" };
                                    call_ir.push_str(&format!("  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([4 x i8], [4 x i8]* {}, i32 0, i32 0), i8* getelementptr inbounds ([{} x i8], [{} x i8]* {}, i32 0, i32 0))\n", 
                                        format_str, if *b { 5 } else { 6 }, if *b { 5 } else { 6 }, bool_str));
                                }
                                // Variables and expressions - assume integer for now (needs type system for proper handling)
                                _ => {
                                    let format_str = if func_name == "println" { "@.str.2" } else { "@.str.3" };
                                    call_ir.push_str(&format!("  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([4 x i8], [4 x i8]* {}, i32 0, i32 0), i64 {})\n", 
                                        format_str, arg_result));
                                }
                            }
                            
                            Ok((call_ir, "0".to_string())) // print/println return void, represented as 0
                        }
                        _ => {
                            // Regular function call
                            let mut call_ir = String::new();
                            let mut arg_results = Vec::new();
                            
                            for arg in arguments {
                                let (arg_ir, arg_result) = self.generate_expression(arg)?;
                                call_ir.push_str(&arg_ir);
                                arg_results.push(arg_result);
                            }
                            
                            let temp_name = self.next_temp();
                            call_ir.push_str(&format!("  {} = call i64 @{}(", temp_name, func_name));
                            
                            for (i, arg_result) in arg_results.iter().enumerate() {
                                if i > 0 {
                                    call_ir.push_str(", ");
                                }
                                call_ir.push_str(&format!("i64 {}", arg_result));
                            }
                            
                            call_ir.push_str(")\n");
                            Ok((call_ir, temp_name))
                        }
                    }
                } else {
                    Err(LLVMCodeGenError {
                        message: "Complex function calls not supported yet".to_string(),
                    })
                }
            }
            Expr::Assign { name, value } => {
                // Generate code for the value expression
                let (value_ir, value_result) = self.generate_expression(value)?;
                
                // Check if variable exists, if not create it
                let var_ref = if let Some(existing_var) = self.variables.get(name).cloned() {
                    existing_var
                } else {
                    // Create new variable
                    let var_name = format!("%{}", name);
                    self.variables.insert(name.clone(), var_name.clone());
                    // Add alloca to the beginning of current function (this is a simplification)
                    // In a real implementation, we'd need to track function entry blocks
                    var_name
                };
                
                // Generate store instruction
                let mut assign_ir = String::new();
                assign_ir.push_str(&value_ir);
                assign_ir.push_str(&format!("  store i64 {}, i64* {}, align 8\n", value_result, var_ref));
                
                // Assignment returns the assigned value
                Ok((assign_ir, value_result))
            }
            Expr::Get { object, name } => {
                // Handle module-qualified access like math.PI
                if let Expr::Variable(module_name) = object.as_ref() {
                    let qualified_name = format!("{}.{}", module_name, name);
                    if let Some(var_ref) = self.variables.get(&qualified_name).cloned() {
                        let temp_name = self.next_temp();
                        let load_ir = format!("  {} = load i64, i64* {}, align 8\n", temp_name, var_ref);
                        Ok((load_ir, temp_name))
                    } else {
                        Err(LLVMCodeGenError {
                            message: format!("Undefined variable: {}", qualified_name),
                        })
                    }
                } else {
                    Err(LLVMCodeGenError {
                        message: "Complex object access not yet supported".to_string(),
                    })
                }
            }
            _ => Err(LLVMCodeGenError {
                message: format!("Expression type not implemented: {:?}", expr),
            }),
        }
    }

    fn next_temp(&mut self) -> String {
        let temp = format!("%{}", self.temp_counter);
        self.temp_counter += 1;
        temp
    }

    #[allow(dead_code)]
    fn next_label(&mut self) -> String {
        let label = format!("label{}", self.label_counter);
        self.label_counter += 1;
        label
    }
}