//! Module registry (mod-rec.toml) management

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use super::error::{ModuleError, ModuleResult};

/// Module record entry in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRecord {
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub submodules: HashMap<String, String>,  // name -> absolute path
}

/// The complete module registry (mod-rec.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRegistry {
    #[serde(default)]
    pub modules: HashMap<String, ModuleRecord>,
}

impl ModuleRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    /// Load registry from mod-rec.toml file
    pub fn load<P: AsRef<Path>>(path: P) -> ModuleResult<Self> {
        let content = fs::read_to_string(path)?;
        let registry: ModuleRegistry = toml::from_str(&content)?;
        Ok(registry)
    }

    /// Save registry to mod-rec.toml file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> ModuleResult<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Scan the src/ directory and generate/update the module registry
    pub fn scan_and_update<P: AsRef<Path>>(src_path: P) -> ModuleResult<Self> {
        let src_path = src_path.as_ref();
        let mut registry = Self::new();

        // Walk through src/ directory looking for module directories
        if src_path.exists() && src_path.is_dir() {
            registry.scan_directory(src_path, src_path)?;
        }

        Ok(registry)
    }

    /// Recursively scan a directory for modules
    fn scan_directory(&mut self, base_path: &Path, current_path: &Path) -> ModuleResult<()> {
        for entry in fs::read_dir(current_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Check if this directory has an export.nlang file
                let export_file = path.join("export.nlang");
                if export_file.exists() {
                    // This is a module directory
                    self.scan_module(&path)?;
                } else {
                    // Continue scanning subdirectories
                    self.scan_directory(base_path, &path)?;
                }
            }
        }

        Ok(())
    }

    /// Scan a module directory and register all its submodules
    fn scan_module(&mut self, module_path: &Path) -> ModuleResult<()> {
        // Read the export.nlang file to get the module name
        let export_file = module_path.join("export.nlang");
        let export_content = fs::read_to_string(&export_file)?;

        // Parse module name from "entry mod MODULE_NAME;"
        let module_name = Self::parse_module_name(&export_content)
            .ok_or_else(|| ModuleError::InvalidModuleDeclaration(
                export_file.clone(),
                "Could not find 'entry mod' declaration".to_string()
            ))?;

        let mut record = ModuleRecord {
            submodules: HashMap::new(),
        };

        // Scan all .nlang files in the module directory (except export.nlang)
        for entry in fs::read_dir(module_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() 
                && path.extension().and_then(|s| s.to_str()) == Some("nlang")
                && path.file_name().and_then(|s| s.to_str()) != Some("export.nlang")
            {
                // Extract submodule name from filename
                if let Some(submodule_name) = path.file_stem().and_then(|s| s.to_str()) {
                    // Store absolute path
                    let abs_path = fs::canonicalize(&path)?;
                    record.submodules.insert(
                        submodule_name.to_string(),
                        abs_path.to_string_lossy().to_string()
                    );
                }
            }
        }

        self.modules.insert(module_name, record);
        Ok(())
    }

    /// Parse module name from export.nlang content
    fn parse_module_name(content: &str) -> Option<String> {
        // Look for "entry mod NAME;"
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("entry mod ") && line.ends_with(';') {
                let name = line
                    .strip_prefix("entry mod ")?
                    .strip_suffix(';')?
                    .trim();
                return Some(name.to_string());
            }
        }
        None
    }

    /// Get a module record by name
    pub fn get_module(&self, name: &str) -> Option<&ModuleRecord> {
        self.modules.get(name)
    }

    /// Get the path to a specific submodule
    pub fn get_submodule_path(&self, module: &str, submodule: &str) -> Option<PathBuf> {
        self.modules
            .get(module)?
            .submodules
            .get(submodule)
            .map(|s| PathBuf::from(s))
    }

    /// Check if a module exists
    pub fn has_module(&self, name: &str) -> bool {
        self.modules.contains_key(name)
    }

    /// Check if a submodule exists in a module
    pub fn has_submodule(&self, module: &str, submodule: &str) -> bool {
        self.modules
            .get(module)
            .map(|m| m.submodules.contains_key(submodule))
            .unwrap_or(false)
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
