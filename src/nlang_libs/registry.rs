use crate::nlang_libs::common::LibraryDefinition;
use crate::nlang_libs::test_lib::create_test_lib_lib;
use std::collections::HashMap;

pub struct LibraryRegistry {
    libraries: HashMap<String, LibraryDefinition>,
}

impl LibraryRegistry {
    pub fn new() -> Self {
        let registry = Self {
            libraries: HashMap::new(),
        };

        // Register std lib implicitly or explicitly?
        // The user wants std to be treated as a builtin module.
        // We can wrap StdLib into LibraryDefinition for consistency if needed,
        // or just handle it alongside.
        // For now, let's register any *new* libraries here.
        // StdLib is handled separately in the current architecture, but we can expose it here too.

        registry
    }

    pub fn register_library(&mut self, lib: LibraryDefinition) {
        self.libraries.insert(lib.name.clone(), lib);
    }

    pub fn get_library(&self, name: &str) -> Option<&LibraryDefinition> {
        self.libraries.get(name)
    }

    pub fn is_builtin_module(&self, name: &str) -> bool {
        if name == "std" {
            return true;
        }
        self.libraries.contains_key(name)
    }

    /// Returns a list of all registered library names (excluding std for now if handled separately)
    pub fn get_registered_libs(&self) -> Vec<String> {
        self.libraries.keys().cloned().collect()
    }
}

// Global/Shared instance helper if needed, or just let components create it.
// Since we want to be flexible, we might want a function that returns a populated registry.
pub fn get_default_registry() -> LibraryRegistry {
    let mut registry = LibraryRegistry::new();

    // Here we would register any default libraries if we had any others.
    // For example:
    // registry.register_library(create_fs_lib());
    registry.register_library(create_test_lib_lib());

    registry
}
