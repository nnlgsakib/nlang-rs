use crate::ast::{Expr, Type};

/// Type alias for a native function implementation
pub type NativeFunction = fn(&[Expr]) -> Result<Expr, String>;

use crate::ast::Statement;

/// Represents a function definition in a library
#[derive(Clone)]
pub struct LibraryFunction {
    pub name: String,
    pub parameters: Vec<Type>,
    pub return_type: Type,
    pub implementation: Option<NativeFunction>,
    pub c_implementation: Option<String>,
    pub ast_body: Option<Vec<Statement>>,
}

/// Represents a type definition in a library
#[derive(Clone)]
pub struct LibraryType {
    pub name: String,
    pub methods: Vec<LibraryMethod>,
}

/// Represents a method definition for a type in a library
#[derive(Clone)]
pub struct LibraryMethod {
    pub name: String,
    pub parameters: Vec<Type>,
    pub return_type: Type,
}

/// Represents a complete library definition
pub struct LibraryDefinition {
    pub name: String,
    pub functions: Vec<LibraryFunction>,
    pub types: Vec<LibraryType>,
    /// C code required for this library (headers, helper functions, etc.)
    pub c_implementation: Option<String>,
}

impl LibraryDefinition {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            functions: Vec::new(),
            types: Vec::new(),
            c_implementation: None,
        }
    }

    pub fn add_function(
        &mut self,
        name: &str,
        params: Vec<Type>,
        ret: Type,
        impl_fn: NativeFunction,
    ) {
        self.functions.push(LibraryFunction {
            name: name.to_string(),
            parameters: params,
            return_type: ret,
            implementation: Some(impl_fn),
            c_implementation: None,
            ast_body: None,
        });
    }

    pub fn add_ast_function(
        &mut self,
        name: &str,
        params: Vec<Type>,
        ret: Type,
        body: Vec<Statement>,
    ) {
        self.functions.push(LibraryFunction {
            name: name.to_string(),
            parameters: params,
            return_type: ret,
            implementation: None,
            c_implementation: None,
            ast_body: Some(body),
        });
    }

    pub fn set_c_implementation(&mut self, code: &str) {
        self.c_implementation = Some(code.to_string());
    }
}
