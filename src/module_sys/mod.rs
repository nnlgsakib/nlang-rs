//! NLang Module System
//! 
//! This module implements the file-system-driven module and project management system.
//! It handles project creation, module registry (mod-rec.toml), and module resolution.

pub mod project;
pub mod registry;
pub mod resolver;
pub mod types;
pub mod error;

// Re-export commonly used types
pub use project::{Project, ProjectConfig};
pub use registry::{ModuleRegistry, ModuleRecord};
pub use resolver::ModuleResolver;
pub use types::{ModuleDeclaration, SubModuleDeclaration, ModuleKind, SymbolInfo};
pub use error::{ModuleError, ModuleResult};
