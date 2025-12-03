//! Module registry (mod-rec.toml) management

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use super::error::{ModuleError, ModuleResult};
use super::types::ProjectMetadata;

/// Module record entry in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRecord {
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub submodules: HashMap<String, String>,  // name -> relative path (using forward slashes)
}

/// The complete module registry (mod-rec.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRegistry {
    /// Project metadata section
    #[serde(default)]
    pub project: ProjectMetadata,
    
    /// Registered modules
    #[serde(default)]
    pub modules: HashMap<String, ModuleRecord>,
    
    /// Runtime-only: project root path (not serialized)
    #[serde(skip)]
    project_root: Option<PathBuf>,
}

impl ModuleRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            project: ProjectMetadata::default(),
            modules: HashMap::new(),
            project_root: None,
        }
    }
    
    /// Create a new registry with project name
    pub fn new_with_name(name: &str) -> Self {
        Self {
            project: ProjectMetadata {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                ..Default::default()
            },
            modules: HashMap::new(),
            project_root: None,
        }
    }

    /// Load registry from mod-rec.toml file
    pub fn load<P: AsRef<Path>>(path: P) -> ModuleResult<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;
        let mut registry: ModuleRegistry = toml::from_str(&content)?;
        
        // Store the project root for path resolution
        if let Some(parent) = path.parent() {
            registry.project_root = Some(parent.to_path_buf());
        }
        
        Ok(registry)
    }

    /// Save registry to mod-rec.toml file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> ModuleResult<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
    
    /// Set the project root path (used for resolving relative paths)
    pub fn set_project_root<P: AsRef<Path>>(&mut self, root: P) {
        self.project_root = Some(root.as_ref().to_path_buf());
    }
    
    /// Get the project root path
    pub fn project_root(&self) -> Option<&Path> {
        self.project_root.as_deref()
    }
    
    /// Get the project name
    pub fn project_name(&self) -> &str {
        &self.project.name
    }
    
    /// Get the project version
    pub fn project_version(&self) -> &str {
        &self.project.version
    }

    /// Scan the src/ directory and generate/update the module registry
    /// The project_root is the directory containing mod-rec.toml
    /// Preserves existing project metadata if mod-rec.toml exists
    pub fn scan_and_update<P: AsRef<Path>>(project_root: P, project_name: Option<&str>) -> ModuleResult<Self> {
        let project_root = project_root.as_ref();
        let src_path = project_root.join("src");
        let mod_rec_path = project_root.join("mod-rec.toml");
        
        // Try to load existing project metadata if mod-rec.toml exists
        let existing_project = if mod_rec_path.exists() {
            Self::load(&mod_rec_path).ok().map(|r| r.project)
        } else {
            None
        };
        
        // Use existing metadata, or create new with provided/inferred name
        let project_metadata = existing_project.unwrap_or_else(|| {
            let name = project_name
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    project_root
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("untitled")
                        .to_string()
                });
            ProjectMetadata {
                name,
                version: "0.1.0".to_string(),
                ..Default::default()
            }
        });
        
        let mut registry = Self {
            project: project_metadata,
            modules: HashMap::new(),
            project_root: Some(project_root.to_path_buf()),
        };

        // Walk through src/ directory looking for module directories
        if src_path.exists() && src_path.is_dir() {
            registry.scan_directory(project_root, &src_path)?;
        }

        Ok(registry)
    }

    /// Recursively scan a directory for modules
    fn scan_directory(&mut self, project_root: &Path, current_path: &Path) -> ModuleResult<()> {
        for entry in fs::read_dir(current_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Check if this directory has an export.nlang file
                let export_file = path.join("export.nlang");
                if export_file.exists() {
                    // This is a module directory
                    self.scan_module(project_root, &path)?;
                } else {
                    // Continue scanning subdirectories
                    self.scan_directory(project_root, &path)?;
                }
            }
        }

        Ok(())
    }

    /// Scan a module directory and register all its submodules
    fn scan_module(&mut self, project_root: &Path, module_path: &Path) -> ModuleResult<()> {
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
                    // Store relative path using forward slashes (cross-platform)
                    let relative_path = Self::to_relative_path(project_root, &path);
                    record.submodules.insert(
                        submodule_name.to_string(),
                        relative_path
                    );
                }
            }
        }

        self.modules.insert(module_name, record);
        Ok(())
    }
    
    /// Convert an absolute path to a relative path with forward slashes
    fn to_relative_path(base: &Path, target: &Path) -> String {
        // Get the relative path
        if let Ok(relative) = target.strip_prefix(base) {
            // Convert to forward slashes for cross-platform compatibility
            relative.to_string_lossy().replace('\\', "/")
        } else {
            // Fallback: just use the filename if we can't make it relative
            target.to_string_lossy().replace('\\', "/")
        }
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

    /// Get the path to a specific submodule (resolves relative to absolute)
    pub fn get_submodule_path(&self, module: &str, submodule: &str) -> Option<PathBuf> {
        let relative_path = self.modules
            .get(module)?
            .submodules
            .get(submodule)?;
        
        // If we have a project root, resolve the relative path
        if let Some(root) = &self.project_root {
            Some(root.join(relative_path))
        } else {
            // Fallback: treat as-is (might be absolute or relative to cwd)
            Some(PathBuf::from(relative_path))
        }
    }
    
    /// Get the raw relative path string to a submodule (as stored in the registry)
    pub fn get_submodule_relative_path(&self, module: &str, submodule: &str) -> Option<&str> {
        self.modules
            .get(module)?
            .submodules
            .get(submodule)
            .map(|s| s.as_str())
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
