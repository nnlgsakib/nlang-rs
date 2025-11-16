use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower_lsp::lsp_types::{Url, Diagnostic};

pub struct DocumentData {
    pub uri: Url,
    pub path: PathBuf,
    pub text: String,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct SymbolLocation {
    pub uri: Url,
    pub line: u32,
    pub character: u32,
}

pub struct StateInner {
    pub documents: HashMap<Url, DocumentData>,
    pub symbols: HashMap<String, Vec<SymbolLocation>>, 
    pub std_symbols: HashMap<String, Vec<SymbolLocation>>, 
    pub std_functions: Vec<String>,
    pub string_methods: Vec<String>,
    pub array_methods: Vec<String>,
}

#[derive(Clone)]
pub struct State(pub Arc<Mutex<StateInner>>);

impl State {
    pub fn new() -> Self {
        State(Arc::new(Mutex::new(StateInner { documents: HashMap::new(), symbols: HashMap::new(), std_symbols: HashMap::new(), std_functions: Vec::new(), string_methods: Vec::new(), array_methods: Vec::new() })))
    }
}