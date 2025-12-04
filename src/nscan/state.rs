//! LSP Server State Management
//!
//! Thread-safe state container for tracking open documents,
//! symbol tables, and standard library metadata.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower_lsp::lsp_types::{Url, Diagnostic};
use nlang::module_sys::ModuleRegistry;

/// Represents an open document in the workspace
#[derive(Debug, Clone)]
pub struct DocumentData {
    pub uri: Url,
    pub path: PathBuf,
    pub text: String,
    pub diagnostics: Vec<Diagnostic>,
}

/// Location of a symbol in source code
#[derive(Debug, Clone)]
pub struct SymbolLocation {
    pub uri: Url,
    pub line: u32,
    pub character: u32,
}

/// Inner state protected by mutex for thread safety
#[derive(Debug)]
pub struct StateInner {
    /// All open documents indexed by URI
    pub documents: HashMap<Url, DocumentData>,
    
    /// User-defined symbols (functions, variables) indexed by name
    pub symbols: HashMap<String, Vec<SymbolLocation>>,
    
    /// Standard library symbols indexed by name
    pub std_symbols: HashMap<String, Vec<SymbolLocation>>,
    
    /// Standard library function names
    pub std_functions: Vec<String>,
    
    /// Available string methods
    pub string_methods: Vec<String>,
    
    /// Available array/list methods
    pub array_methods: Vec<String>,
    
    /// Current project root (if in multi-module workspace)
    pub project_root: Option<PathBuf>,
    
    /// Module registry for multi-module projects (cached, not Debug)
    pub module_registry: Option<ModuleRegistry>,
    
    /// Module symbols indexed by module path (e.g., "game.character")
    pub module_symbols: HashMap<String, Vec<SymbolLocation>>,
}

/// Thread-safe shared state for LSP server
#[derive(Clone, Debug)]
pub struct State(pub Arc<Mutex<StateInner>>);

impl State {
    /// Create new empty state
    pub fn new() -> Self {
        State(Arc::new(Mutex::new(StateInner {
            documents: HashMap::new(),
            symbols: HashMap::new(),
            std_symbols: HashMap::new(),
            std_functions: Vec::new(),
            string_methods: Vec::new(),
            array_methods: Vec::new(),
            project_root: None,
            module_registry: None,
            module_symbols: HashMap::new(),
        })))
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}