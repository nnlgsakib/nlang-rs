use crate::ast::{Expr, Literal, Type};
use crate::nlang_libs::common::LibraryDefinition;
use std::env;

pub fn create_env_man_lib() -> LibraryDefinition {
    let mut lib = LibraryDefinition::new("env_man");

    lib.add_function("get", vec![Type::String], Type::String, env_get);
    lib.add_function("set", vec![Type::String, Type::String], Type::Void, env_set);
    lib.add_function("unset", vec![Type::String], Type::Void, env_unset);
    lib.add_function("list", vec![], Type::String, env_list);
    lib.add_function("os", vec![], Type::String, env_os);

    // Set the library-level C implementation (headers and helper functions)
    lib.c_implementation = Some(include_str!("env_man.c").to_string());

    // Set the function-level C implementation (call templates)
    for func in &mut lib.functions {
        match func.name.as_str() {
            "get" => func.c_implementation = Some("env_man_get({0})".to_string()),
            "set" => func.c_implementation = Some("env_man_set({0}, {1})".to_string()),
            "unset" => func.c_implementation = Some("env_man_unset({0})".to_string()),
            "list" => func.c_implementation = Some("env_man_list()".to_string()),
            "os" => func.c_implementation = Some("env_man_os()".to_string()),
            _ => {}
        }
    }

    lib
}

fn env_get(args: &[Expr]) -> Result<Expr, String> {
    if let Expr::Literal(Literal::String(key)) = &args[0] {
        let val = env::var(key).unwrap_or_default();
        Ok(Expr::Literal(Literal::String(val)))
    } else {
        Err("Invalid argument type for env_man.get".to_string())
    }
}

fn env_set(args: &[Expr]) -> Result<Expr, String> {
    if let (Expr::Literal(Literal::String(key)), Expr::Literal(Literal::String(value))) =
        (&args[0], &args[1])
    {
        unsafe {
            env::set_var(key, value);
        }
        Ok(Expr::Literal(Literal::Null))
    } else {
        Err("Invalid argument type for env_man.set".to_string())
    }
}

fn env_unset(args: &[Expr]) -> Result<Expr, String> {
    if let Expr::Literal(Literal::String(key)) = &args[0] {
        unsafe {
            env::remove_var(key);
        }
        Ok(Expr::Literal(Literal::Null))
    } else {
        Err("Invalid argument type for env_man.unset".to_string())
    }
}

fn env_list(_args: &[Expr]) -> Result<Expr, String> {
    let mut result = String::new();
    for (key, value) in env::vars() {
        result.push_str(&format!("{}={}\n", key, value));
    }
    Ok(Expr::Literal(Literal::String(result)))
}

fn env_os(_args: &[Expr]) -> Result<Expr, String> {
    Ok(Expr::Literal(Literal::String(env::consts::OS.to_string())))
}
