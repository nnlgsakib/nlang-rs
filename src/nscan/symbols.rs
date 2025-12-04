//! Symbol Extraction and Indexing
//!
//! Provides utilities for extracting identifiers from NLang source code
//! and building symbol indexes for go-to-definition and rename operations.

use std::collections::HashMap;
use tower_lsp::lsp_types::Url;

/// Extract all identifiers from source text with their locations
///
/// Returns tuples of (name, line, character) for each identifier found.
/// An identifier is any sequence of ASCII alphabetic characters, digits, or underscores
/// starting with a letter or underscore.
pub fn extract_idents(text: &str) -> Vec<(String, u32, u32)> {
    let mut v = Vec::new();
    
    for (i, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        let mut idx = 0usize;
        
        while idx < bytes.len() {
            let ch = bytes[idx] as char;
            
            // Start of identifier: letter or underscore
            if ch.is_ascii_alphabetic() || ch == '_' {
                let start = idx;
                idx += 1;
                
                // Continue identifier: letter, digit, or underscore
                while idx < bytes.len() {
                    let c = bytes[idx] as char;
                    if c.is_ascii_alphanumeric() || c == '_' {
                        idx += 1;
                    } else {
                        break;
                    }
                }
                
                let ident = &line[start..idx];
                v.push((ident.to_string(), i as u32, start as u32));
            } else {
                idx += 1;
            }
        }
    }
    
    v
}

/// Build symbol index from source text
///
/// Returns a map from symbol names to all their occurrences in the file.
/// Each occurrence includes the URI and position (line, character).
pub fn index_symbols(text: &str, uri: &Url) -> HashMap<String, Vec<(Url, u32, u32)>> {
    let mut map: HashMap<String, Vec<(Url, u32, u32)>> = HashMap::new();
    
    for (name, line, col) in extract_idents(text) {
        let e = map.entry(name).or_default();
        e.push((uri.clone(), line, col));
    }
    
    map
}