use std::collections::HashMap;
use tower_lsp::lsp_types::Url;

pub fn extract_idents(text: &str) -> Vec<(String, u32, u32)> {
    let mut v = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        let mut idx = 0usize;
        while idx < bytes.len() {
            let ch = bytes[idx] as char;
            if ch.is_ascii_alphabetic() || ch == '_' {
                let start = idx;
                idx += 1;
                while idx < bytes.len() {
                    let c = bytes[idx] as char;
                    if c.is_ascii_alphanumeric() || c == '_' { idx += 1; } else { break; }
                }
                let ident = &line[start..idx];
                v.push((ident.to_string(), i as u32, start as u32));
            } else { idx += 1; }
        }
    }
    v
}

pub fn index_symbols(text: &str, uri: &Url) -> HashMap<String, Vec<(Url, u32, u32)>> {
    let mut map: HashMap<String, Vec<(Url, u32, u32)>> = HashMap::new();
    for (name, line, col) in extract_idents(text) {
        let e = map.entry(name).or_default();
        e.push((uri.clone(), line, col));
    }
    map
}