use crate::ast::{Expr, Literal, Statement, Type};
use crate::nlang_libs::common::LibraryDefinition;

pub fn create_test_lib_lib() -> LibraryDefinition {
    let mut lib = LibraryDefinition::new("test_lib");

    lib.add_ast_function(
        "hello",
        vec![],
        Type::String,
        vec![Statement::Return {
            value: Some(Box::new(Expr::Literal(Literal::String(
                "Hello from test_lib!".to_string(),
            )))),
        }],
    );

    lib
}
