//! Core types for the module system

use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Represents the kind of module declaration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleKind {
    /// Entry module: `entry mod game;`
    Entry { name: String },
    /// Sub module: `sub mod character;`
    Sub { name: String },
}

/// Module declaration in source files
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDeclaration {
    pub kind: ModuleKind,
    pub exports: Vec<String>,  // List of exported submodule names
}

/// Sub-module declaration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubModuleDeclaration {
    pub name: String,
    pub path: PathBuf,
}

/// Information about a loaded module
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub path: PathBuf,
    pub kind: ModuleKind,
    pub submodules: HashMap<String, SubModuleInfo>,
    pub exports: Vec<String>,  // Re-exported submodules
}

/// Information about a sub-module
#[derive(Debug, Clone)]
pub struct SubModuleInfo {
    pub name: String,
    pub path: PathBuf,
    pub exported_symbols: HashMap<String, SymbolInfo>,
}

/// Information about an exported symbol
#[derive(Debug, Clone)]
pub enum SymbolInfo {
    Function {
        name: String,
        parameters: Vec<crate::ast::Parameter>,
        return_type: crate::ast::Type,
    },
    Variable {
        name: String,
        var_type: crate::ast::Type,
        is_mutable: bool,
    },
}

/// Project metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

impl Default for ProjectMetadata {
    fn default() -> Self {
        Self {
            name: "untitled".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            authors: None,
            license: None,
            repository: None,
        }
    }
}
