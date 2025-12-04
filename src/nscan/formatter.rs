//! Code Formatting for NLang
//!
//! Provides automatic code formatting with consistent indentation.
//! Simple brace-based indentation with 4-space indent.

/// Format NLang source code with consistent indentation
///
/// Rules:
/// - 4 spaces per indent level
/// - Increase indent after lines ending with '{'
/// - Decrease indent before lines starting with '}'
/// - Trim all leading/trailing whitespace per line
pub fn format(text: &str) -> String {
    let mut out = String::new();
    let mut indent = 0usize;
    
    for line in text.lines() {
        let trimmed = line.trim();
        
        // Skip empty lines but preserve them
        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }
        
        // Decrease indent for closing braces
        if trimmed.starts_with('}') {
            if indent > 0 {
                indent -= 1;
            }
        }
        
        // Apply indentation
        let padding = "    ".repeat(indent);
        out.push_str(&padding);
        out.push_str(trimmed);
        out.push('\n');
        
        // Increase indent for opening braces
        if trimmed.ends_with('{') {
            indent += 1;
        }
    }
    
    out
}