use std::collections::HashSet;
use std::path::Path;
pub mod color;

/// Represents a source code location with precise positioning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
    
    pub fn unknown() -> Self {
        Self { line: 1, column: 1 }
    }
}

/// Severity level for diagnostic messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Represents a complete diagnostic message with context
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub title: String,
    pub message: String,
    pub span: Option<Span>,
    pub suggestions: Vec<String>,
    pub related_info: Vec<RelatedInformation>,
}

/// Additional contextual information for a diagnostic
#[derive(Debug, Clone)]
pub struct RelatedInformation {
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            title: title.into(),
            message: message.into(),
            span: None,
            suggestions: Vec::new(),
            related_info: Vec::new(),
        }
    }
    
    pub fn warning(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            title: title.into(),
            message: message.into(),
            span: None,
            suggestions: Vec::new(),
            related_info: Vec::new(),
        }
    }
    
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
    
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }
    
    pub fn with_related(mut self, span: Span, message: impl Into<String>) -> Self {
        self.related_info.push(RelatedInformation {
            span,
            message: message.into(),
        });
        self
    }
}

fn get_line(source: &str, line: usize) -> Option<&str> {
    source.lines().nth(line.saturating_sub(1))
}

fn find_column_in_line(line_text: &str, needle: Option<&str>) -> usize {
    if let Some(n) = needle {
        if !n.is_empty() {
            if let Some(pos) = line_text.find(n) {
                return pos + 1;
            }
        }
    }
    1
}

fn analyze_line_for_call_errors(line_text: &str) -> Option<(usize, String)> {
    let bytes = line_text.as_bytes();
    let trimmed = line_text.trim_start();
    if trimmed.starts_with("def ") {
        if let Some(op) = line_text.find('(') {
            let after = &line_text[op + 1..];
            let close = after.find(')');
            let arrow = line_text.find("->");
            let brace = line_text.find('{');
            if close.is_none() && (arrow.is_some() || brace.is_some()) {
                return Some((
                    op + 1,
                    "help: add ')' after parameters; use '()' for none".to_string(),
                ));
            }
        }
    }
    if trimmed.starts_with('.') {
        let leading_spaces = line_text.len() - trimmed.len();
        return Some((
            leading_spaces + 1,
            "help: method call missing receiver; add an object before '.' (e.g., name.upper())"
                .to_string(),
        ));
    }
    if line_text.contains("=.") || line_text.contains("= .") {
        if let Some(pos) = line_text.find('.') {
            return Some((
                pos + 1,
                "help: method call missing receiver; add an object before '.' (e.g., obj.method())"
                    .to_string(),
            ));
        }
    }
    for i in 0..bytes.len() {
        if bytes[i] == b'.' {
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b')' {
                return Some((
                    j + 1,
                    "help: try adding parentheses after method name: '()'".to_string(),
                ));
            }
        }
    }
    if let Some(pos) = line_text.find(')') {
        let opens = line_text.matches('(').count();
        let closes = line_text.matches(')').count();
        if closes > opens {
            return Some((
                pos + 1,
                "help: remove the extra ')' or add a matching '('".to_string(),
            ));
        }
    }
    if let Some(dot_start) = line_text.find('.') {
        if let Some(open_after) = line_text[dot_start..].find('(') {
            let after_open = dot_start + open_after + 1;
            if let Some(inner_dot) = line_text[after_open..].find('.') {
                let col = after_open + inner_dot + 1;
                return Some((
                    col,
                    "help: try closing parentheses: ')' before chaining with '.'".to_string(),
                ));
            }
        }
    }
    // Detect missing comma between arguments e.g., println("text" arg)
    if let Some(first_quote) = line_text.find('"') {
        if let Some(second_quote_rel) = line_text[first_quote + 1..].find('"') {
            let close_pos = first_quote + 1 + second_quote_rel; // 0-based index of closing quote
            if close_pos + 1 <= line_text.len() {
                let after = &line_text[close_pos + 1..];
                let after_trim = after.trim_start();
                if !after_trim.is_empty() {
                    let ch = after_trim.chars().next().unwrap_or('\0');
                    if ch.is_ascii_alphabetic() || ch == '_' || ch == '(' {
                        // If not starting with a comma, likely missing comma between args
                        if !after_trim.starts_with(',') {
                            return Some((
                                close_pos + 1,
                                "help: try adding a comma: ','".to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }
    None
}

fn analyze_line_for_type_errors(line_text: &str) -> Option<(usize, String)> {
    // Known type names
    let known = [
        "int", "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize", "f32",
        "f64", "float", "bool", "string", "void", "vault", "pool", "tree",
    ];
    // Tokenize identifiers with positions
    let bytes = line_text.as_bytes();
    let mut idx = 0usize;
    let mut best: Option<(usize, String, usize)> = None; // (col, suggestion, distance)
    while idx < bytes.len() {
        let ch = bytes[idx] as char;
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = idx;
            idx += 1;
            while idx < bytes.len() {
                let c = bytes[idx] as char;
                if c.is_ascii_alphanumeric() || c == '_' {
                    idx += 1;
                } else {
                    break;
                }
            }
            let ident = &line_text[start..idx];
            if !known.contains(&ident) {
                // compare to known types
                for k in known.iter() {
                    let d = edit_distance(ident, k);
                    // threshold: allow small edits
                    let thr = if ident.len() <= 3 { 1 } else { 2 };
                    if d > 0 && d <= thr {
                        match &best {
                            Some((_, _, bd)) if *bd <= d => {}
                            _ => {
                                best = Some((start + 1, (*k).to_string(), d));
                            }
                        }
                    }
                }
            }
        } else {
            idx += 1;
        }
    }
    if let Some((col, sugg, _)) = best {
        Some((col, format!("help: unknown type. Did you mean: {}?", sugg)))
    } else {
        None
    }
}

/// Enhanced error suggestion system with production-grade accuracy
fn suggest(message: &str) -> Vec<String> {
    let m = message.to_lowercase();
    let mut suggestions = Vec::new();
    
    // Missing receiver errors
    if m.contains("missing receiver") || m.contains("got dot") {
        suggestions.push("add an object before '.' (e.g., obj.method())".to_string());
    }
    
    // Parameter errors
    if m.contains("expected parameter name") {
        suggestions.push("add ')' after parameters; use '()' for no parameters".to_string());
    }
    
    // Semicolon errors
    if m.contains("expected ';'") || m.contains("expected ';' after") {
        suggestions.push("add a semicolon: ';'".to_string());
        suggestions.push("check if the previous statement is complete".to_string());
    }
    
    // Bracket/brace/paren errors
    if m.contains("expected '}'") {
        suggestions.push("add a closing brace: '}'".to_string());
        suggestions.push("ensure all blocks are properly closed".to_string());
    }
    if m.contains("expected ')'") {
        suggestions.push("add a closing parenthesis: ')'".to_string());
        suggestions.push("verify function call arguments are complete".to_string());
    }
    if m.contains("expected ']'") {
        suggestions.push("add a closing bracket: ']'".to_string());
        suggestions.push("check array indexing syntax".to_string());
    }
    
    // Data structure specific errors
    if m.contains("vault key must be string") || m.contains("vault[string]") {
        suggestions.push("vault uses string keys; e.g., users[\"Alice\"]".to_string());
    }
    if m.contains("indexing supported for arrays[int] and vault[string]") {
        suggestions.push("use integer index for arrays, string key for vault".to_string());
    }
    if m.contains("unknown pool method") {
        suggestions.push("pool supports 'add(value)'".to_string());
    }
    if m.contains("unknown tree method") {
        suggestions.push("tree supports 'add(child)'".to_string());
    }
    
    // String method errors
    if m.contains("split delimiter must be string") || m.contains("join delimiter must be string") {
        suggestions.push("delimiter must be a string literal".to_string());
    }
    if m.contains("substring start must be int") || m.contains("substring end must be int") {
        suggestions.push("substring indices must be integers".to_string());
    }
    if m.contains("substring start > end") {
        suggestions.push("ensure start <= end for substring".to_string());
    }
    if m.contains("regex pattern must be string") {
        suggestions.push("regex pattern must be a string literal".to_string());
    }
    if m.contains("invalid regex") {
        suggestions.push("check regex syntax; escape special characters properly".to_string());
        suggestions.push("common escapes: \\., \\*, \\+, \\?, \\(, \\)".to_string());
    }
    
    // Argument errors
    if m.contains("expects") && m.contains("argument") {
        suggestions.push("check function signature and argument count".to_string());
        suggestions.push("verify argument types match function parameters".to_string());
        suggestions.push("ensure library is imported (std, fs_man, env_man, etc.)".to_string());
    }
    
    // fs_man library suggestions
    if m.contains("fs_man") || m.contains("file system") || m.contains("file not found") {
        suggestions.push("fs_man functions: read_file, write_file, copy_file, move_file, exists, is_file, is_dir".to_string());
        suggestions.push("directory ops: read_dir, list_files, list_dirs, walk_dir, create_dir, remove_dir".to_string());
        suggestions.push("path utilities: basename, dirname, extname, join, canonicalize, absolute_path".to_string());
        suggestions.push("import fs_man: 'import fs_man;' then use fs_man.function_name()".to_string());
    }
    if m.contains("permission") {
        suggestions.push("use fs_man.set_permissions(path, mode) with octal mode like 0o644".to_string());
    }
    if m.contains("symlink") || m.contains("hard link") {
        suggestions.push("fs_man.create_symlink(target, link) or fs_man.create_hard_link(target, link)".to_string());
    }
    if m.contains("temp") && (m.contains("file") || m.contains("directory")) {
        suggestions.push("fs_man.create_temp_file(prefix) or fs_man.create_temp_dir(prefix)".to_string());
    }
    
    // env_man library suggestions
    if m.contains("env_man") || m.contains("environment variable") {
        suggestions.push("env_man functions: get(key), set(key, value), unset(key), list(), os()".to_string());
        suggestions.push("import env_man: 'import env_man;' then use env_man.function_name()".to_string());
    }
    
    // Comma errors
    if m.contains("expected ','") {
        suggestions.push("add a comma: ','".to_string());
        suggestions.push("separate multiple arguments/items with commas".to_string());
    }
    
    // Expression errors
    if m.contains("expected expression") && m.contains("dot") {
        suggestions.push("close parentheses: ')' or add '()' after method name".to_string());
    }
    
    // Type errors
    if m.contains("type mismatch") || m.contains("expected type") {
        suggestions.push("ensure variable types match their usage".to_string());
        suggestions.push("use explicit type annotations if needed: 'store name:type = value'".to_string());
    }
    
    // Module errors
    if m.contains("module") && m.contains("not found") {
        suggestions.push("check mod-rec.toml for module registration".to_string());
        suggestions.push("verify module path and export.nlang file".to_string());
    }
    if m.contains("submodule") && m.contains("not found") {
        suggestions.push("check if submodule is listed in mod-rec.toml".to_string());
        suggestions.push("verify submodule is exported in export.nlang".to_string());
    }
    
    // Memory safety errors
    if m.contains("immutable") && m.contains("cannot assign") {
        suggestions.push("add '@mut' annotation to make variable mutable".to_string());
        suggestions.push("example: @mut store x = 10;".to_string());
    }
    if m.contains("moved value") {
        suggestions.push("value was moved; clone it or use a reference".to_string());
        suggestions.push("restructure code to avoid using value after move".to_string());
    }
    if m.contains("borrow") && m.contains("mutably") {
        suggestions.push("only one mutable borrow allowed at a time".to_string());
        suggestions.push("end existing borrows before creating new ones".to_string());
    }
    
    // Division errors
    if m.contains("division by zero") {
        suggestions.push("add a check: if (divisor != 0) { ... }".to_string());
    }
    
    // Array errors
    if m.contains("index out of bounds") {
        suggestions.push("verify array index is within valid range".to_string());
        suggestions.push("use len() function to check array size".to_string());
    }
    
    suggestions
}

fn render_caret(line_text: &str, column: usize) -> String {
    color::caret_line(line_text, column)
}

/// Production-ready rendering of diagnostics with full context
pub fn render_diagnostic(diag: &Diagnostic, file_path: &Path, source: &str) -> String {
    let mut out = String::new();
    
    // Severity tag and title
    let (tag, color_fn): (String, fn(&str) -> String) = match diag.severity {
        DiagnosticSeverity::Error => (color::error_tag(), color::red),
        DiagnosticSeverity::Warning => (color::warn_tag(), color::yellow),
        DiagnosticSeverity::Info => (color::info_tag(), color::blue),
        DiagnosticSeverity::Hint => ("💡".to_string(), color::blue),
    };
    
    out.push_str(&format!(
        "{} {}\n",
        tag,
        color::bold(&color_fn(&diag.title))
    ));
    
    // Location
    if let Some(span) = diag.span {
        out.push_str(&format!(
            "{}\n",
            color::location(&file_path.display().to_string(), span.line, span.column)
        ));
        
        // Source code context
        if let Some(line_text) = get_line(source, span.line) {
            out.push_str(&render_caret(line_text, span.column));
            out.push('\n');
        }
    }
    
    out.push_str("   |\n");
    
    // Main message
    out.push_str(&format!(
        "   = {} {}\n",
        tag,
        color::bold(&color_fn(&diag.message))
    ));
    
    // Suggestions
    for (idx, suggestion) in diag.suggestions.iter().enumerate() {
        if idx == 0 {
            out.push_str(&format!("   = {} {}\n", color::help_tag(), suggestion));
        } else {
            out.push_str(&format!("   = {} {}\n", color::info_tag(), suggestion));
        }
    }
    
    // Related information
    for related in &diag.related_info {
        out.push_str(&format!(
            "   = {} {}:{}:{}: {}\n",
            color::info_tag(),
            file_path.display(),
            related.span.line,
            related.span.column,
            related.message
        ));
    }
    
    out
}

pub fn emit_basic(
    title: &str,
    file_path: &Path,
    source: &str,
    span: Option<Span>,
    lexeme_hint: Option<&str>,
    message: &str,
) -> String {
    let (line, column, rendered) = if let Some(sp) = span.as_ref() {
        let mut use_line = sp.line;
        let ml = message.to_lowercase();
        if sp.column == 0 {
            if ml.contains("expected ';'") && use_line > 1 {
                for idx in (1..use_line).rev() {
                    if let Some(l) = get_line(source, idx) {
                        if !l.trim().is_empty() {
                            use_line = idx;
                            break;
                        }
                    }
                }
            }
        }
        let line_text = get_line(source, use_line).unwrap_or("");
        let mut col = if sp.column == 0 {
            find_column_in_line(line_text, lexeme_hint)
        } else {
            sp.column
        };
        // Apply heuristics for call/parenthesis errors when column unknown
        if sp.column == 0 {
            let is_unknown_method = {
                let tl = title.to_lowercase();
                let ml = message.to_lowercase();
                tl.contains("unknown string method")
                    || ml.contains("unknown string method")
                    || tl.contains("unknown array method")
                    || ml.contains("unknown array method")
            };
            if !is_unknown_method {
                if let Some((hcol, _)) = analyze_line_for_call_errors(line_text) {
                    col = hcol;
                }
            }
            if title.to_lowercase().contains("expected type") {
                if let Some((tcol, _)) = analyze_line_for_type_errors(line_text) {
                    col = tcol;
                }
            }
            if ml.contains("expected ';'") {
                let trimmed_len = line_text.trim_end().len();
                col = if trimmed_len == 0 { 1 } else { trimmed_len };
            } else if ml.contains("expected ')'") {
                // Prefer specific call error heuristics (e.g., missing comma) if available
                if analyze_line_for_call_errors(line_text).is_none() {
                    let trimmed_len = line_text.trim_end().len();
                    col = if trimmed_len == 0 { 1 } else { trimmed_len };
                }
            }
        }
        (use_line, col, render_caret(line_text, col))
    } else {
        (1, 1, String::new())
    };
    let mut out = String::new();
    out.push_str(&format!(
        "{} {}\n",
        color::error_tag(),
        color::bold(&color::red(title))
    ));
    out.push_str(&format!(
        "{}\n",
        color::location(&file_path.display().to_string(), line, column)
    ));
    out.push_str(&rendered);
    out.push('\n');
    out.push_str("   |\n");
    let lt = title.to_lowercase();
    out.push_str(&format!(
        "   = {} {}\n",
        color::error_tag(),
        color::bold(&color::red(message))
    ));
    let suppress_inline_help = lt.contains("semantic error") || lt.contains("runtime error");
    if !suppress_inline_help {
        let mut help_used = false;
        let suggestions = suggest(message);
        if !suggestions.is_empty() {
            for (idx, help) in suggestions.iter().enumerate() {
                if idx == 0 {
                    out.push_str(&format!("   = {} {}\n", color::help_tag(), help));
                } else {
                    out.push_str(&format!("   = {} {}\n", color::info_tag(), help));
                }
            }
            help_used = true;
        }
        if !help_used {
            if let Some(sp) = span.as_ref() {
                if let Some(line_text) = get_line(source, sp.line) {
                    let is_unknown_method = {
                        let ml = message.to_lowercase();
                        lt.contains("unknown string method")
                            || ml.contains("unknown string method")
                            || lt.contains("unknown array method")
                            || ml.contains("unknown array method")
                    };
                    if !is_unknown_method {
                        if let Some((_, h)) = analyze_line_for_call_errors(line_text) {
                            out.push_str(&format!("   = {} {}\n", color::help_tag(), h));
                        }
                    }
                    if lt.contains("expected type") {
                        if let Some((_, h)) = analyze_line_for_type_errors(line_text) {
                            out.push_str(&format!("   = {} {}\n", color::help_tag(), h));
                        }
                    }
                }
            }
        }
    }
    out
}

// Convenience for execution errors
use crate::execution_engine::ExecutionError;

pub fn from_execution_error(file_path: &Path, source: &str, err: &ExecutionError) -> String {
    match err {
        ExecutionError::ParserError(pe) => {
            let mut line = pe.line.max(1);
            let mut col = 0usize;
            let ml = pe.message.to_lowercase();
            if ml.contains("expected ';'") && line > 1 {
                let prev = line - 1;
                if let Some(prev_text) = get_line(source, prev) {
                    let t = prev_text.trim_end();
                    if !t.trim().is_empty() {
                        line = prev;
                        col = t.len().max(1);
                    }
                }
            }
            emit_basic(
                &format!("Parser error: {}", pe.message),
                file_path,
                source,
                Some(Span { line, column: col }),
                None,
                &pe.to_string(),
            )
        }
        ExecutionError::LexerError(le) => emit_basic(
            &format!("Lexer error: {}", le.message),
            file_path,
            source,
            Some(Span {
                line: le.line,
                column: 0,
            }),
            None,
            &format!("Lexer error on line {}: {}", le.line, le.message),
        ),
        ExecutionError::SemanticError(se) => {
            let msg = se.to_string();
            // Try to enrich with span and suggestions for unknown symbol or method
            let (mut span_opt, mut extra_help) = enrich_undefined_symbol(source, &msg);
            let (mm_span, mm_help) = enrich_memmanager_error(source, &msg);
            if span_opt.is_none() {
                span_opt = mm_span;
            }
            let primary_msg = if let Some((kind, name)) = extract_undefined(&msg) {
                match kind {
                    "function" => format!("function '{}' not found", name),
                    "variable" => format!("variable '{}' not found", name),
                    _ => msg.clone(),
                }
            } else {
                msg.clone()
            };
            if span_opt.is_none() {
                if let Some(mname) = extract_unknown_method(&msg) {
                    span_opt = find_method_span(source, &mname);
                    if extra_help.is_none() {
                        let lower = msg.to_lowercase();
                        if lower.contains("unknown array method") {
                            if let Some(help) = suggest_unknown_array_method(&mname) {
                                extra_help = Some(help);
                            }
                        } else if lower.contains("unknown pool method") {
                            if let Some(help) = suggest_unknown_pool_method(&mname) {
                                extra_help = Some(help);
                            }
                        } else if lower.contains("unknown tree method") {
                            if let Some(help) = suggest_unknown_tree_method(&mname) {
                                extra_help = Some(help);
                            }
                        } else if let Some(help) = suggest_unknown_method(&mname) {
                            extra_help = Some(help);
                        }
                    }
                }
            }
            let mut rendered = emit_basic(
                "Semantic error",
                file_path,
                source,
                span_opt,
                None,
                &primary_msg,
            );
            if let Some(help) = extra_help {
                let msg = help.strip_prefix("help: ").unwrap_or(&help);
                rendered.push_str(&format!("   = {} {}\n", color::help_tag(), msg));
            }
            if let Some(help) = mm_help {
                let msg = help.strip_prefix("help: ").unwrap_or(&help);
                rendered.push_str(&format!("   = {} {}\n", color::help_tag(), msg));
            }
            rendered
        }
        ExecutionError::InterpreterError(ie) => {
            let msg = ie.to_string();
            let (mut span_opt, mut extra_help) = enrich_undefined_symbol(source, &msg);
            let primary_msg = if let Some((kind, name)) = extract_undefined(&msg) {
                match kind {
                    "function" => format!("function '{}' not found", name),
                    "variable" => format!("variable '{}' not found", name),
                    _ => msg.clone(),
                }
            } else {
                msg.clone()
            };
            if span_opt.is_none() {
                if let Some(mname) = extract_unknown_method(&msg) {
                    span_opt = find_method_span(source, &mname);
                    if extra_help.is_none() {
                        let lower = msg.to_lowercase();
                        if lower.contains("unknown array method") {
                            if let Some(help) = suggest_unknown_array_method(&mname) {
                                extra_help = Some(help);
                            }
                        } else if lower.contains("unknown pool method") {
                            if let Some(help) = suggest_unknown_pool_method(&mname) {
                                extra_help = Some(help);
                            }
                        } else if lower.contains("unknown tree method") {
                            if let Some(help) = suggest_unknown_tree_method(&mname) {
                                extra_help = Some(help);
                            }
                        } else if let Some(help) = suggest_unknown_method(&mname) {
                            extra_help = Some(help);
                        }
                    }
                }
            }
            let mut rendered = emit_basic(
                "Runtime error",
                file_path,
                source,
                span_opt,
                None,
                &primary_msg,
            );
            if let Some(help) = extra_help {
                let msg = help.strip_prefix("help: ").unwrap_or(&help);
                rendered.push_str(&format!("   = {} {}\n", color::help_tag(), msg));
            }
            rendered
        }
        ExecutionError::CCodeGenError(ce) => emit_basic(
            "Code generation error",
            file_path,
            source,
            None,
            None,
            &ce.to_string(),
        ),
        ExecutionError::IoError(ioe) => {
            emit_basic("I/O error", file_path, source, None, None, &ioe.to_string())
        }
        ExecutionError::NotImplemented { message } => emit_basic(
            "Feature not implemented",
            file_path,
            source,
            None,
            None,
            message,
        ),
    }
}

fn extract_unknown_method(msg: &str) -> Option<String> {
    let lower = msg.to_lowercase();
    if let Some(pos) = lower.find("unknown string method:") {
        let name = msg[pos + "Unknown string method:".len()..]
            .trim()
            .to_string();
        let name = name.trim_matches('"').trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    if let Some(pos) = lower.find("unknown array method:") {
        let name = msg[pos + "Unknown array method:".len()..]
            .trim()
            .to_string();
        let name = name.trim_matches('"').trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    if let Some(pos) = lower.find("unknown pool method:") {
        let name = msg[pos + "Unknown pool method:".len()..].trim().to_string();
        let name = name.trim_matches('"').trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    if let Some(pos) = lower.find("unknown tree method:") {
        let name = msg[pos + "Unknown tree method:".len()..].trim().to_string();
        let name = name.trim_matches('"').trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

fn find_method_span(source: &str, method: &str) -> Option<Span> {
    for (i, line) in source.lines().enumerate() {
        if let Some(dot_pos) = line.find('.') {
            if let Some(mpos) = line[dot_pos + 1..].find(method) {
                return Some(Span {
                    line: i + 1,
                    column: dot_pos + mpos + 2,
                });
            }
        }
    }
    None
}

fn suggest_unknown_method(bad: &str) -> Option<String> {
    let known = [
        "upper",
        "lower",
        "trim",
        "contains",
        "split",
        "replace",
        "substring",
        "regex",
    ];
    let thr = if bad.len() <= 3 { 1 } else { 2 };
    let mut best: Option<(&str, usize)> = None;
    for k in known.iter() {
        let d = edit_distance(bad, k);
        if d > 0 && d <= thr {
            match best {
                Some((_prev, pd)) if pd <= d => {}
                _ => best = Some((*k, d)),
            }
        }
    }
    best.map(|(s, _)| format!("help: unknown method. Did you mean: {}?", s))
}

fn suggest_unknown_array_method(bad: &str) -> Option<String> {
    let known = ["len", "join"];
    let thr = if bad.len() <= 3 { 1 } else { 2 };
    let mut best: Option<(&str, usize)> = None;
    for k in known.iter() {
        let d = edit_distance(bad, k);
        if d > 0 && d <= thr {
            match best {
                Some((_prev, pd)) if pd <= d => {}
                _ => best = Some((*k, d)),
            }
        }
    }
    best.map(|(s, _)| format!("help: unknown array method. Did you mean: {}?", s))
}

fn suggest_unknown_pool_method(bad: &str) -> Option<String> {
    let known = ["add"];
    let thr = if bad.len() <= 3 { 1 } else { 2 };
    let mut best: Option<(&str, usize)> = None;
    for k in known.iter() {
        let d = edit_distance(bad, k);
        if d > 0 && d <= thr {
            match best {
                Some((_prev, pd)) if pd <= d => {}
                _ => best = Some((*k, d)),
            }
        }
    }
    best.map(|(s, _)| format!("help: unknown pool method. Did you mean: {}?", s))
}

fn suggest_unknown_tree_method(bad: &str) -> Option<String> {
    let known = ["add"];
    let thr = if bad.len() <= 3 { 1 } else { 2 };
    let mut best: Option<(&str, usize)> = None;
    for k in known.iter() {
        let d = edit_distance(bad, k);
        if d > 0 && d <= thr {
            match best {
                Some((_prev, pd)) if pd <= d => {}
                _ => best = Some((*k, d)),
            }
        }
    }
    best.map(|(s, _)| format!("help: unknown tree method. Did you mean: {}?", s))
}

pub fn resolve_span(
    title: &str,
    source: &str,
    mut span: Span,
    lexeme_hint: Option<&str>,
    message: &str,
) -> Span {
    if span.line == 0 {
        span.line = 1;
    }
    // Special-case missing semicolon: point to end of previous non-empty line
    let ml = message.to_lowercase();
    if ml.contains("expected ';'") {
        let mut idx = span.line.saturating_sub(1);
        while idx > 0 {
            if let Some(l) = get_line(source, idx) {
                let t = l.trim_end();
                if !t.trim().is_empty() {
                    return Span {
                        line: idx,
                        column: t.len().max(1),
                    };
                }
            }
            idx -= 1;
        }
    }
    let line_text = get_line(source, span.line).unwrap_or("");
    let mut col = if span.column == 0 {
        find_column_in_line(line_text, lexeme_hint)
    } else {
        span.column
    };
    if span.column == 0 {
        let is_unknown_method = {
            let tl = title.to_lowercase();
            let ml = message.to_lowercase();
            tl.contains("unknown string method")
                || ml.contains("unknown string method")
                || tl.contains("unknown array method")
                || ml.contains("unknown array method")
        };
        if !is_unknown_method {
            if let Some((hcol, _)) = analyze_line_for_call_errors(line_text) {
                col = hcol;
            }
        }
        if title.to_lowercase().contains("expected type") {
            if let Some((tcol, _)) = analyze_line_for_type_errors(line_text) {
                col = tcol;
            }
        }
    }
    Span {
        line: span.line,
        column: col,
    }
}

pub fn semantic_span(source: &str, message: &str) -> Option<Span> {
    let (mut span_opt, _) = enrich_undefined_symbol(source, message);
    if span_opt.is_none() {
        if let Some(mname) = extract_unknown_method(message) {
            span_opt = find_method_span(source, &mname);
        }
    }
    span_opt
}

pub fn semantic_extra_help(source: &str, message: &str) -> Option<String> {
    let (_, mut extra_help) = enrich_undefined_symbol(source, message);
    if extra_help.is_none() {
        if let Some(mname) = extract_unknown_method(message) {
            let lower = message.to_lowercase();
            if lower.contains("unknown array method") {
                if let Some(h) = suggest_unknown_array_method(&mname) {
                    extra_help = Some(h);
                }
            } else if lower.contains("unknown pool method") {
                if let Some(h) = suggest_unknown_pool_method(&mname) {
                    extra_help = Some(h);
                }
            } else if lower.contains("unknown tree method") {
                if let Some(h) = suggest_unknown_tree_method(&mname) {
                    extra_help = Some(h);
                }
            } else if let Some(h) = suggest_unknown_method(&mname) {
                extra_help = Some(h);
            }
        }
    }
    extra_help
}

fn extract_quoted_name(msg: &str) -> Option<String> {
    if let Some(start) = msg.find('\'') {
        if let Some(end) = msg[start + 1..].find('\'') {
            return Some(msg[start + 1..start + 1 + end].to_string());
        }
    }
    if let Some(start) = msg.find('"') {
        if let Some(end) = msg[start + 1..].find('"') {
            return Some(msg[start + 1..start + 1 + end].to_string());
        }
    }
    None
}

fn enrich_memmanager_error(source: &str, message: &str) -> (Option<Span>, Option<String>) {
    let lower = message.to_lowercase();
    if lower.contains("cannot assign to immutable variable") {
        if let Some(name) = extract_quoted_name(message) {
            let sp = find_name_span(source, &name);
            let help = format!("help: add '@mut' to declaration: @mut store {} = ...", name);
            return (sp, Some(help));
        }
    }
    if lower.contains("cannot mutate immutable variable") {
        if let Some(name) = extract_quoted_name(message) {
            let sp = find_name_span(source, &name);
            let help = format!("help: make '{}' mutable with '@mut' to update it", name);
            return (sp, Some(help));
        }
    }
    if lower.contains("use of moved value") {
        if let Some(name) = extract_quoted_name(message) {
            let sp = find_name_span(source, &name);
            let help = format!(
                "help: '{}' was moved; use a Copy type or avoid moving before use",
                name
            );
            return (sp, Some(help));
        }
    }
    if lower.contains("cannot borrow") && lower.contains("mutably") {
        if let Some(name) = extract_quoted_name(message) {
            let sp = find_name_span(source, &name);
            let help = format!(
                "help: end existing borrows of '{}' before taking a mutable borrow",
                name
            );
            return (sp, Some(help));
        }
    }
    if lower.contains("cannot use") && lower.contains("mutably borrowed") {
        if let Some(name) = extract_quoted_name(message) {
            let sp = find_name_span(source, &name);
            let help = format!(
                "help: limit the '&@mut {}' borrow scope; use after borrow ends",
                name
            );
            return (sp, Some(help));
        }
    }
    if lower.contains("cannot return reference to local variable") {
        if let Some(name) = extract_quoted_name(message) {
            let sp = find_name_span(source, &name);
            let help = format!(
                "help: return an owned value or a reference derived from parameters (not local '{}')",
                name
            );
            return (sp, Some(help));
        }
    }
    if lower.contains("conditional move") && lower.contains("loop") {
        let help = "help: ensure moves are consistent across iterations; restructure logic or clone when needed".to_string();
        return (None, Some(help));
    }
    (None, None)
}

fn enrich_undefined_symbol(source: &str, message: &str) -> (Option<Span>, Option<String>) {
    let mut span = None;
    let mut help = None;

    if let Some((kind, name)) = extract_undefined(message) {
        // 1. Try to find the span of the name
        span = find_name_span(source, &name);

        // 2. Suggestions
        if kind == "function" {
            // Check std lib
            let std_funcs = known_std_functions();
            if std_funcs.contains(name.as_str()) {
                // It is a known std function.
            }

            // Check registry
            let registry_funcs = collect_registry_functions();
            if let Some(lib_name) = registry_funcs.get(&name) {
                help = Some(format!(
                    "help: '{}' is provided by library '{}'. Import it using 'from {} import {};'",
                    name, lib_name, lib_name, name
                ));
                return (span, help);
            }

            // Fuzzy match against std + registry + local functions + imports
            let mut candidates: Vec<String> = std_funcs.iter().map(|s| s.to_string()).collect();
            candidates.extend(registry_funcs.keys().cloned());

            // Add local functions and imports
            let local_funcs = collect_function_names(source);
            candidates.extend(local_funcs);
            let imports = collect_imports(source);
            candidates.extend(imports);

            let thr = if name.len() <= 3 { 1 } else { 2 };
            let mut best: Option<(String, usize)> = None;

            for cand in candidates {
                let d = edit_distance(&name, &cand);
                if d > 0 && d <= thr {
                    match best {
                        Some((ref prev, pd)) => {
                            if d < pd {
                                best = Some((cand, d));
                            } else if d == pd {
                                // Tie-breaking:
                                // 1. Prefer candidate starting with same char
                                let name_start = name.chars().next();
                                let cand_start = cand.chars().next();
                                let prev_start = prev.chars().next();
                                if name_start == cand_start && name_start != prev_start {
                                    best = Some((cand, d));
                                }
                            }
                        }
                        None => best = Some((cand, d)),
                    }
                }
            }
            if let Some((s, _)) = best {
                help = Some(format!("help: unknown function. Did you mean: {}?", s));
            }
        } else if kind == "variable" {
            // Fuzzy match against local variables in scope + std constants
            let line = span.as_ref().map(|s| s.line).unwrap_or(0);
            let mut candidates = Vec::new();

            // Add std constants
            let std_consts = known_std_constants();
            candidates.extend(std_consts.iter().map(|s| s.to_string()));

            if line > 0 {
                let locals = collect_identifiers_in_scope(source, line);
                candidates.extend(locals);
            }

            let thr = if name.len() <= 3 { 1 } else { 2 };
            let mut best: Option<(String, usize)> = None;
            for cand in candidates {
                let d = edit_distance(&name, &cand);
                if d > 0 && d <= thr {
                    match best {
                        Some((ref prev, pd)) => {
                            if d < pd {
                                best = Some((cand, d));
                            } else if d == pd {
                                let name_start = name.chars().next();
                                let cand_start = cand.chars().next();
                                let prev_start = prev.chars().next();
                                if name_start == cand_start && name_start != prev_start {
                                    best = Some((cand, d));
                                }
                            }
                        }
                        None => best = Some((cand, d)),
                    }
                }
            }
            if let Some((s, _)) = best {
                help = Some(format!("help: unknown variable. Did you mean: {}?", s));
            }
        }
    }
    (span, help)
}

fn collect_registry_functions() -> std::collections::HashMap<String, String> {
    let registry = crate::nlang_libs::registry::get_default_registry();
    let mut map = std::collections::HashMap::new();
    for lib_name in registry.get_registered_libs() {
        if let Some(lib) = registry.get_library(&lib_name) {
            for func in &lib.functions {
                map.insert(func.name.clone(), lib_name.clone());
            }
        }
    }
    map
}

fn known_std_constants() -> std::collections::HashSet<&'static str> {
    ["pi", "tau", "e", "ln2", "ln10"].iter().cloned().collect()
}

fn known_std_functions() -> std::collections::HashSet<&'static str> {
    [
        // Math functions
        "exp",
        "ln",
        "log2",
        "log10",
        "pow_float",
        "powi_float",
        "sqrt",
        "isqrt",
        "sin",
        "cos",
        "tan",
        "atan",
        "asin",
        "acos",
        "floor",
        "ceil",
        "round",
        "clamp",
        "fmod",
        "sign",
        "gcd",
        "lcm",
        "factorial",
        "nPr",
        "nCr",
        "sum_float",
        "mean_float",
        "median_float",
        "variance_float",
        "stddev_float",
        "sum",
        "min",
        "max",
        "pow",
        "sort",
        "reverse",
        // Core I/O and conversion
        "print",
        "println",
        "input",
        "len",
        "int",
        "float",
        "str",
        "sha256",
        "sha256_random",
        // Time
        "now",
        "now_utc",
        "now_local",
        "time_to_string",
        "timestamp",
        "timestamp_ms",
        "timestamp_us",
        "timestamp_ns",
        "sleep",
        "sleep_ms",
        "sleep_ns",
        "year",
        "month",
        "day",
        "weekday",
        "hour",
        "minute",
        "second",
        "nanosecond",
        "to_local",
        "to_utc",
        "from_timestamp",
        "from_timestamp_ms",
        "format",
        "parse",
        "parse_rfc3339",
        "parse_rfc2822",
        // Duration and Timer
        "duration_from_seconds",
        "duration_from_millis",
        "duration_from_nanos",
        "duration_as_secs",
        "duration_as_millis",
        "duration_add",
        "duration_sub",
        "timer_start",
        "timer_elapsed",
        "timer_reset",
    ]
    .iter()
    .cloned()
    .collect()
}

fn extract_undefined(msg: &str) -> Option<(&'static str, String)> {
    let lower = msg.to_lowercase();
    if let Some(pos) = lower.find("undefined variable:") {
        let name = msg[pos + "Undefined variable:".len()..].trim().to_string();
        return Some(("variable", name));
    }
    if let Some(pos) = lower.find("undefined function") {
        let tail = msg[pos + "Undefined function".len()..]
            .trim()
            .trim_matches(':')
            .trim();
        let name = tail.trim_matches('"').trim_matches('\'').to_string();
        return Some(("function", name));
    }
    if let Some(pos) = lower.find("function not found") {
        let name = msg[pos + "Function not found".len()..]
            .trim()
            .trim_start_matches(':')
            .trim()
            .to_string();
        return Some(("function", name));
    }
    None
}

fn collect_imports(source: &str) -> Vec<String> {
    let mut v = Vec::new();
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("from ") {
            if let Some(import_part) = t.split(" import ").nth(1) {
                let import_list = import_part.trim_end_matches(';');
                for item in import_list.split(',') {
                    let name = item.trim();
                    if !name.is_empty() {
                        v.push(name.to_string());
                    }
                }
            }
        }
    }
    v
}

fn collect_identifiers(source: &str) -> Vec<String> {
    let mut set: HashSet<String> = HashSet::new();
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("store ") {
            let rest = &t[6..];
            if let Some(name) = rest.split_whitespace().next() {
                if !name.is_empty() {
                    set.insert(name.to_string());
                }
            }
        } else if t.starts_with("def ") {
            let rest = &t[4..];
            let name = rest.split('(').next().unwrap_or("").trim();
            if !name.is_empty() {
                set.insert(name.to_string());
            }
            if let Some(params_part) = rest.split('(').nth(1) {
                let params_text = params_part.split(')').next().unwrap_or("");
                for p in params_text.split(',') {
                    let pname = p.trim().split(':').next().unwrap_or("").trim();
                    if !pname.is_empty() {
                        set.insert(pname.to_string());
                    }
                }
            }
        }
    }
    // Add imports to identifiers
    for imp in collect_imports(source) {
        set.insert(imp);
    }
    set.into_iter().collect()
}

fn collect_identifiers_in_scope(source: &str, line: usize) -> Vec<String> {
    let mut set: HashSet<String> = HashSet::new();
    let functions = collect_function_names(source);
    for f in functions.iter() {
        set.insert(f.clone());
    }

    // Add imports
    for imp in collect_imports(source) {
        set.insert(imp);
    }

    if let Some((_fname, start, end)) = find_enclosing_function_range(source, line) {
        for (idx, l) in source.lines().enumerate() {
            if idx + 1 >= start && idx + 1 <= end {
                let t = l.trim();
                if t.starts_with("store ") {
                    let rest = &t[6..];
                    if let Some(name) = rest.split_whitespace().next() {
                        if !name.is_empty() {
                            set.insert(name.to_string());
                        }
                    }
                } else if t.starts_with("def ") {
                    let rest = &t[4..];
                    let name = rest.split('(').next().unwrap_or("").trim();
                    if !name.is_empty() {
                        set.insert(name.to_string());
                    }
                    if let Some(params_part) = rest.split('(').nth(1) {
                        let params_text = params_part.split(')').next().unwrap_or("");
                        for p in params_text.split(',') {
                            let pname = p.trim().split(':').next().unwrap_or("").trim();
                            if !pname.is_empty() {
                                set.insert(pname.to_string());
                            }
                        }
                    }
                }
            }
        }
        return set.into_iter().collect();
    }
    collect_identifiers(source)
}

fn collect_function_names(source: &str) -> Vec<String> {
    let mut v = Vec::new();
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("def ") {
            let rest = &t[4..];
            let name = rest.split('(').next().unwrap_or("").trim();
            if !name.is_empty() {
                v.push(name.to_string());
            }
        }
    }
    v
}

fn find_enclosing_function_range(source: &str, line: usize) -> Option<(String, usize, usize)> {
    let mut current: Option<(String, usize)> = None;
    let mut depth = 0i32;
    for (idx, l) in source.lines().enumerate() {
        let t = l.trim();
        if t.starts_with("def ") {
            let rest = &t[4..];
            let name = rest.split('(').next().unwrap_or("").trim().to_string();
            current = Some((name, idx + 1));
            depth = 0;
        }
        for ch in l.chars() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
            }
        }
        if let Some((ref fname, start_line)) = current {
            if depth <= 0 && idx + 1 > start_line {
                if line >= start_line && line <= idx + 1 {
                    return Some((fname.clone(), start_line, idx + 1));
                }
                current = None;
            }
        }
    }
    None
}

fn find_name_span(source: &str, name: &str) -> Option<Span> {
    for (i, line) in source.lines().enumerate() {
        let bytes = line.as_bytes();
        let mut idx = 0;
        while idx < bytes.len() {
            let ch = bytes[idx] as char;
            let is_start = ch.is_ascii_alphabetic() || ch == '_';
            if is_start {
                let start = idx;
                idx += 1;
                while idx < bytes.len() {
                    let c = bytes[idx] as char;
                    if c.is_ascii_alphanumeric() || c == '_' {
                        idx += 1;
                    } else {
                        break;
                    }
                }
                let ident = &line[start..idx];
                if ident == name {
                    return Some(Span {
                        line: i + 1,
                        column: start + 1,
                    });
                }
            } else {
                idx += 1;
            }
        }
    }
    None
}

fn edit_distance(a: &str, b: &str) -> usize {
    let mut dp = vec![vec![0; b.len() + 1]; a.len() + 1];
    for i in 0..=a.len() {
        dp[i][0] = i;
    }
    for j in 0..=b.len() {
        dp[0][j] = j;
    }
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if ab[i - 1] == bb[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a.len()][b.len()]
}