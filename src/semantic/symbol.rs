use crate::ast::{Program, Type, Parameter};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    // Exported symbols from the module
    pub exported_symbols: HashMap<String, Symbol>,
    // Original parsed program (for function definitions)
    pub original_program: Program,
}

#[derive(Debug, Clone)]
pub enum Symbol {
    Variable { var_type: Type, is_mutable: bool },
    Function { 
        return_type: Type, 
        parameters: Vec<Parameter> 
    },
    Namespace { 
        #[allow(dead_code)]
        module_name: String 
    },
}
