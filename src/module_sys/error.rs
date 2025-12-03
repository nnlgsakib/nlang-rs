//! Error types for the module system

use std::path::PathBuf;
use thiserror::Error;

pub type ModuleResult<T> = Result<T, ModuleError>;

#[derive(Error, Debug)]
pub enum ModuleError {
    #[error("Project not found in directory: {0}")]
    ProjectNotFound(PathBuf),

    #[error("Project already exists at: {0}")]
    ProjectAlreadyExists(PathBuf),

    #[error("Module '{0}' not found in registry")]
    ModuleNotFound(String),

    #[error("Submodule '{0}' not found in module '{1}'")]
    SubModuleNotFound(String, String),

    #[error("Missing export.nlang file in module directory: {0}")]
    MissingExportFile(PathBuf),

    #[error("Invalid module declaration in {0}: {1}")]
    InvalidModuleDeclaration(PathBuf, String),

    #[error("Duplicate submodule '{0}' in module '{1}'")]
    DuplicateSubModule(String, String),

    #[error("Module '{0}' has no entry declaration")]
    NoEntryDeclaration(String),

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("Invalid project structure: {0}")]
    InvalidProjectStructure(String),

    #[error("Exported submodule '{0}' not found in module '{1}'")]
    ExportedSubModuleNotFound(String, String),
}
