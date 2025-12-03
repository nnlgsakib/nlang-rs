//! Module resolver - handles module loading and symbol resolution

use std::collections::HashMap;
use std::path::Path;
use std::fs;
use super::error::{ModuleError, ModuleResult};
use super::registry::ModuleRegistry;
use super::types::{ModuleInfo, SubModuleInfo, SymbolInfo, ModuleKind};
use crate::ast::{Statement, Type};
use crate::lexer::Lexer;
use crate::parser::Parser;

/// Resolves and loads modules based on the registry
pub struct ModuleResolver {
    registry: ModuleRegistry,
    loaded_modules: HashMap<String, ModuleInfo>,
    loading_stack: Vec<String>,  // For circular dependency detection
}

impl ModuleResolver {
    /// Create a new resolver with the given registry
    pub fn new(registry: ModuleRegistry) -> Self {
        Self {
            registry,
            loaded_modules: HashMap::new(),
            loading_stack: Vec::new(),
        }
    }

    /// Load a module by name
    pub fn load_module(&mut self, module_name: &str) -> ModuleResult<&ModuleInfo> {
        // Check if already loaded
        if self.loaded_modules.contains_key(module_name) {
            return Ok(&self.loaded_modules[module_name]);
        }

        // Check for circular dependencies
        if self.loading_stack.contains(&module_name.to_string()) {
            return Err(ModuleError::CircularDependency(
                format!("{} -> {}", self.loading_stack.join(" -> "), module_name)
            ));
        }

        // Get module record from registry
        let record = self.registry.get_module(module_name)
            .ok_or_else(|| ModuleError::ModuleNotFound(module_name.to_string()))?;

        self.loading_stack.push(module_name.to_string());

        // Load all submodules
        let mut submodules = HashMap::new();
        for (submodule_name, submodule_path) in &record.submodules {
            let submodule_info = self.load_submodule(
                module_name,
                submodule_name,
                Path::new(submodule_path)
            )?;
            submodules.insert(submodule_name.clone(), submodule_info);
        }

        // Load export.nlang to get module declaration and re-exports
        let module_dir = record.submodules.values()
            .next()
            .and_then(|p| Path::new(p).parent())
            .ok_or_else(|| ModuleError::ModuleNotFound(module_name.to_string()))?;

        let export_file = module_dir.join("export.nlang");
        let (kind, exports) = self.parse_export_file(&export_file)?;

        // Validate that exported submodules exist
        for export in &exports {
            if !submodules.contains_key(export) {
                return Err(ModuleError::ExportedSubModuleNotFound(
                    export.clone(),
                    module_name.to_string()
                ));
            }
        }

        let module_info = ModuleInfo {
            name: module_name.to_string(),
            path: module_dir.to_path_buf(),
            kind,
            submodules,
            exports,
        };

        self.loading_stack.pop();
        self.loaded_modules.insert(module_name.to_string(), module_info);
        
        Ok(&self.loaded_modules[module_name])
    }

    /// Load a single submodule file
    fn load_submodule(
        &self,
        _module_name: &str,
        submodule_name: &str,
        path: &Path
    ) -> ModuleResult<SubModuleInfo> {
        // Read and parse the submodule file
        let source = fs::read_to_string(path)
            .map_err(|e| ModuleError::Io(e))?;

        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize()
            .map_err(|e| ModuleError::InvalidModuleDeclaration(
                path.to_path_buf(),
                format!("Lexer error: {}", e)
            ))?;

        let mut parser = Parser::new(&tokens);
        let program = parser.parse_program()
            .map_err(|e| ModuleError::InvalidModuleDeclaration(
                path.to_path_buf(),
                format!("Parser error: {}", e)
            ))?;

        // Extract exported symbols
        let exported_symbols = self.extract_exported_symbols(&program)?;

        Ok(SubModuleInfo {
            name: submodule_name.to_string(),
            path: path.to_path_buf(),
            exported_symbols,
        })
    }

    /// Parse export.nlang file to get module declaration and exports
    fn parse_export_file(&self, path: &Path) -> ModuleResult<(ModuleKind, Vec<String>)> {
        let content = fs::read_to_string(path)?;
        
        let mut module_kind: Option<ModuleKind> = None;
        let mut exports = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            
            // Parse "entry mod NAME;"
            if line.starts_with("entry mod ") && line.ends_with(';') {
                if let Some(name) = line.strip_prefix("entry mod ")
                    .and_then(|s| s.strip_suffix(';'))
                    .map(|s| s.trim().to_string())
                {
                    module_kind = Some(ModuleKind::Entry { name });
                }
            }
            
            // Parse "export submodule1, submodule2;"
            if line.starts_with("export ") && line.ends_with(';') {
                if let Some(export_list) = line.strip_prefix("export ")
                    .and_then(|s| s.strip_suffix(';'))
                {
                    for item in export_list.split(',') {
                        let item = item.trim();
                        if !item.is_empty() {
                            exports.push(item.to_string());
                        }
                    }
                }
            }
        }

        let kind = module_kind.ok_or_else(|| ModuleError::InvalidModuleDeclaration(
            path.to_path_buf(),
            "Missing 'entry mod' declaration".to_string()
        ))?;

        Ok((kind, exports))
    }

    /// Extract exported symbols from AST statements
    fn extract_exported_symbols(
        &self,
        statements: &[Statement]
    ) -> ModuleResult<HashMap<String, SymbolInfo>> {
        let mut symbols = HashMap::new();

        for stmt in statements {
            match stmt {
                Statement::FunctionDeclaration {
                    name,
                    parameters,
                    return_type,
                    is_exported,
                    ..
                } if *is_exported => {
                    symbols.insert(
                        name.clone(),
                        SymbolInfo::Function {
                            name: name.clone(),
                            parameters: parameters.clone(),
                            return_type: return_type.clone().unwrap_or(Type::Void),
                        }
                    );
                }
                Statement::LetDeclaration {
                    name,
                    var_type,
                    is_exported,
                    is_mutable,
                    ..
                } if *is_exported => {
                    let var_type = var_type.clone().unwrap_or(Type::Integer);
                    symbols.insert(
                        name.clone(),
                        SymbolInfo::Variable {
                            name: name.clone(),
                            var_type,
                            is_mutable: *is_mutable,
                        }
                    );
                }
                _ => {}
            }
        }

        Ok(symbols)
    }

    /// Get all exported symbols from a submodule (for internal module access)
    pub fn get_submodule_symbols(
        &mut self,
        module_name: &str,
        submodule_name: &str
    ) -> ModuleResult<HashMap<String, SymbolInfo>> {
        let module = self.load_module(module_name)?;
        
        let submodule = module.submodules.get(submodule_name)
            .ok_or_else(|| ModuleError::SubModuleNotFound(
                submodule_name.to_string(),
                module_name.to_string()
            ))?;

        Ok(submodule.exported_symbols.clone())
    }

    /// Get all symbols available within a module (automatic internal imports)
    pub fn get_module_internal_symbols(
        &mut self,
        module_name: &str,
        current_submodule: &str
    ) -> ModuleResult<HashMap<String, SymbolInfo>> {
        let module = self.load_module(module_name)?;
        
        let mut all_symbols = HashMap::new();

        // Collect exported symbols from all OTHER submodules in the same module
        for (submodule_name, submodule) in &module.submodules {
            if submodule_name != current_submodule {
                for (symbol_name, symbol_info) in &submodule.exported_symbols {
                    // Prefix with submodule name to avoid conflicts
                    // e.g., "body.eye" for eye() function in body.nlang
                    all_symbols.insert(
                        format!("{}.{}", submodule_name, symbol_name),
                        symbol_info.clone()
                    );
                }
            }
        }

        Ok(all_symbols)
    }

    /// Get the registry
    pub fn registry(&self) -> &ModuleRegistry {
        &self.registry
    }
}
