use std::path::Path;
use std::collections::HashSet;
pub mod color;

pub struct Span {
    pub line: usize,
    pub column: usize,
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
                return Some((op + 1, "help: add ')' after parameters; use '()' for none".to_string()));
            }
        }
    }
    if trimmed.starts_with('.') {
        let leading_spaces = line_text.len() - trimmed.len();
        return Some((leading_spaces + 1, "help: method call missing receiver; add an object before '.' (e.g., name.upper())".to_string()));
    }
    if line_text.contains("=.") || line_text.contains("= .") {
        if let Some(pos) = line_text.find('.') {
            return Some((pos + 1, "help: method call missing receiver; add an object before '.' (e.g., obj.method())".to_string()));
        }
    }
    for i in 0..bytes.len() {
        if bytes[i] == b'.' {
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b')' {
                return Some((j + 1, "help: try adding parentheses after method name: '()'".to_string()));
            }
        }
    }
    if let Some(pos) = line_text.find(')') {
        let opens = line_text.matches('(').count();
        let closes = line_text.matches(')').count();
        if closes > opens {
            return Some((pos + 1, "help: remove the extra ')' or add a matching '('".to_string()));
        }
    }
    if let Some(dot_start) = line_text.find('.') {
        if let Some(open_after) = line_text[dot_start..].find('(') {
            let after_open = dot_start + open_after + 1;
            if let Some(inner_dot) = line_text[after_open..].find('.') {
                let col = after_open + inner_dot + 1;
                return Some((col, "help: try closing parentheses: ')' before chaining with '.'".to_string()));
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
                            return Some((close_pos + 1, "help: try adding a comma: ','".to_string()));
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
        "int","i8","i16","i32","i64","isize",
        "u8","u16","u32","u64","usize",
        "f32","f64","float","bool","string","void",
        "vault","pool","tree"
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
                if c.is_ascii_alphanumeric() || c == '_' { idx += 1; } else { break; }
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
    } else { None }
}


fn suggest(message: &str) -> Option<String> {
    let m = message.to_lowercase();
    if m.contains("missing receiver") || m.contains("got dot") {
        return Some("help: method call missing receiver; add an object before '.'".to_string());
    }
    if m.contains("expected parameter name") {
        return Some("help: add ')' after parameters; use '()' for none".to_string());
    }
    if m.contains("expected ';'") || m.contains("expected ';' after") {
        Some("help: try adding a semicolon: ';'".to_string())
    } else if m.contains("expected '}'") {
        Some("help: try adding a brace: '}'".to_string())
    } else if m.contains("expected ')'") {
        Some("help: try adding a closing parenthesis: ')'".to_string())
    } else if m.contains("expected ']'") {
        Some("help: try adding a closing bracket: ']'".to_string())
    } else if m.contains("vault key must be string") || m.contains("vault[string]") || m.contains("indexing supported for arrays[int] and vault[string]") {
        Some("help: vault uses string keys; e.g., users[\"Alice\"]".to_string())
    } else if m.contains("unknown pool method") {
        Some("help: pool supports 'add(value)'".to_string())
    } else if m.contains("unknown tree method") {
        Some("help: tree supports 'add(child)'".to_string())
    } else if m.contains("split delimiter must be string") || m.contains("join delimiter must be string") {
        Some("help: delimiter must be a string".to_string())
    } else if m.contains("substring start must be int") || m.contains("substring end must be int") {
        Some("help: substring indices must be integers".to_string())
    } else if m.contains("substring start > end") {
        Some("help: ensure start <= end for substring".to_string())
    } else if m.contains("regex pattern must be string") {
        Some("help: regex pattern must be a string literal".to_string())
    } else if m.contains("invalid regex") {
        Some("help: check regex syntax; escape special characters properly".to_string())
    } else if m.contains("expects") && m.contains("argument") {
        Some("help: check function signature and argument count".to_string())
    } else if m.contains("expected ','") {
        Some("help: try adding a comma: ','".to_string())
    } else if m.contains("expected expression") && m.contains("dot") {
        Some("help: try closing parentheses: ')' or add '()' after method name".to_string())
    } else {
        None
    }
}

fn render_caret(line_text: &str, column: usize) -> String { color::caret_line(line_text, column) }

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
                        if !l.trim().is_empty() { use_line = idx; break; }
                    }
                }
            }
        }
        let line_text = get_line(source, use_line).unwrap_or("");
        let mut col = if sp.column == 0 {
            find_column_in_line(line_text, lexeme_hint)
        } else { sp.column };
        // Apply heuristics for call/parenthesis errors when column unknown
        if sp.column == 0 {
            let is_unknown_method = {
                let tl = title.to_lowercase();
                let ml = message.to_lowercase();
                tl.contains("unknown string method") || ml.contains("unknown string method") || tl.contains("unknown array method") || ml.contains("unknown array method")
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
    out.push_str(&format!("{} {}\n", color::error_tag(), color::bold(&color::red(title))));
    out.push_str(&format!("{}\n", color::location(&file_path.display().to_string(), line, column)));
    out.push_str(&rendered);
    out.push('\n');
    out.push_str("   |\n");
    let lt = title.to_lowercase();
    out.push_str(&format!("   = {} {}\n", color::error_tag(), color::bold(&color::red(message))));
    let suppress_inline_help = lt.contains("semantic error") || lt.contains("runtime error");
    if !suppress_inline_help {
        let mut help_used = false;
        if let Some(help) = suggest(message) {
            out.push_str(&format!("   = {} {}\n", color::help_tag(), help));
            help_used = true;
        }
        if !help_used {
            if let Some(sp) = span.as_ref() {
                if let Some(line_text) = get_line(source, sp.line) {
                    let is_unknown_method = {
                        let ml = message.to_lowercase();
                        lt.contains("unknown string method") || ml.contains("unknown string method") || lt.contains("unknown array method") || ml.contains("unknown array method")
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

pub fn from_execution_error(
    file_path: &Path,
    source: &str,
    err: &ExecutionError,
) -> String {
    match err {
        ExecutionError::ParserError(pe) => {
            let mut line = pe.line.max(1);
            let mut col = 0usize;
            let ml = pe.message.to_lowercase();
            if ml.contains("expected ';'") && line > 1 {
                let prev = line - 1;
                if let Some(prev_text) = get_line(source, prev) {
                    let t = prev_text.trim_end();
                    if !t.trim().is_empty() { line = prev; col = t.len().max(1); }
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
        ExecutionError::LexerError(le) => {
            emit_basic(
                &format!("Lexer error: {}", le.message),
                file_path,
                source,
                Some(Span { line: le.line, column: 0 }),
                None,
                &format!("Lexer error on line {}: {}", le.line, le.message),
            )
        }
        ExecutionError::SemanticError(se) => {
            let msg = se.to_string();
            // Try to enrich with span and suggestions for unknown symbol or method
            let (mut span_opt, mut extra_help) = enrich_undefined_symbol(source, &msg);
            let primary_msg = if let Some((kind, name)) = extract_undefined(&msg) {
                match kind {
                    "function" => format!("function '{}' not found", name),
                    "variable" => format!("variable '{}' not found", name),
                    _ => msg.clone(),
                }
            } else { msg.clone() };
            if span_opt.is_none() {
                if let Some(mname) = extract_unknown_method(&msg) {
                    span_opt = find_method_span(source, &mname);
                    if extra_help.is_none() {
                        let lower = msg.to_lowercase();
                        if lower.contains("unknown array method") {
                            if let Some(help) = suggest_unknown_array_method(&mname) { extra_help = Some(help); }
                        } else if lower.contains("unknown pool method") {
                            if let Some(help) = suggest_unknown_pool_method(&mname) { extra_help = Some(help); }
                        } else if lower.contains("unknown tree method") {
                            if let Some(help) = suggest_unknown_tree_method(&mname) { extra_help = Some(help); }
                        } else if let Some(help) = suggest_unknown_method(&mname) { extra_help = Some(help); }
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
            } else { msg.clone() };
            if span_opt.is_none() {
                if let Some(mname) = extract_unknown_method(&msg) {
                    span_opt = find_method_span(source, &mname);
                    if extra_help.is_none() {
                        let lower = msg.to_lowercase();
                        if lower.contains("unknown array method") {
                            if let Some(help) = suggest_unknown_array_method(&mname) { extra_help = Some(help); }
                        } else if lower.contains("unknown pool method") {
                            if let Some(help) = suggest_unknown_pool_method(&mname) { extra_help = Some(help); }
                        } else if lower.contains("unknown tree method") {
                            if let Some(help) = suggest_unknown_tree_method(&mname) { extra_help = Some(help); }
                        } else if let Some(help) = suggest_unknown_method(&mname) { extra_help = Some(help); }
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
        ExecutionError::CCodeGenError(ce) => {
            emit_basic(
                "Code generation error",
                file_path,
                source,
                None,
                None,
                &ce.to_string(),
            )
        }
        ExecutionError::IoError(ioe) => {
            emit_basic(
                "I/O error",
                file_path,
                source,
                None,
                None,
                &ioe.to_string(),
            )
        }
        ExecutionError::NotImplemented { message } => {
            emit_basic(
                "Feature not implemented",
                file_path,
                source,
                None,
                None,
                message,
            )
        }
    }
}

fn extract_unknown_method(msg: &str) -> Option<String> {
    let lower = msg.to_lowercase();
    if let Some(pos) = lower.find("unknown string method:") {
        let name = msg[pos + "Unknown string method:".len()..].trim().to_string();
        let name = name.trim_matches('"').trim().to_string();
        if !name.is_empty() { return Some(name); }
    }
    if let Some(pos) = lower.find("unknown array method:") {
        let name = msg[pos + "Unknown array method:".len()..].trim().to_string();
        let name = name.trim_matches('"').trim().to_string();
        if !name.is_empty() { return Some(name); }
    }
    if let Some(pos) = lower.find("unknown pool method:") {
        let name = msg[pos + "Unknown pool method:".len()..].trim().to_string();
        let name = name.trim_matches('"').trim().to_string();
        if !name.is_empty() { return Some(name); }
    }
    if let Some(pos) = lower.find("unknown tree method:") {
        let name = msg[pos + "Unknown tree method:".len()..].trim().to_string();
        let name = name.trim_matches('"').trim().to_string();
        if !name.is_empty() { return Some(name); }
    }
    None
}

fn find_method_span(source: &str, method: &str) -> Option<Span> {
    for (i, line) in source.lines().enumerate() {
        if let Some(dot_pos) = line.find('.') {
            if let Some(mpos) = line[dot_pos + 1..].find(method) {
                return Some(Span { line: i + 1, column: dot_pos + mpos + 2 });
            }
        }
    }
    None
}

fn suggest_unknown_method(bad: &str) -> Option<String> {
    let known = ["upper", "lower", "trim", "contains", "split", "replace", "substring", "regex"]; 
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

fn enrich_undefined_symbol(source: &str, message: &str) -> (Option<Span>, Option<String>) {
    if let Some((kind, name)) = extract_undefined(message) {
        let span = find_name_span(source, &name);
        let mut combined_help = String::new();

        // If this is a std function, suggest importing std
        if kind == "function" {
            let stds = known_std_functions();
            if stds.contains(name.as_str()) {
                let has_import_std = source.lines().any(|l| l.trim_start().starts_with("import std"));
                if !has_import_std {
                    combined_help.push_str(&format!("help: '{}' is provided by the standard library. Add 'import std;' at the top", name));
                } else {
                    combined_help.push_str(&format!("help: '{}' is a std function. Ensure your std module is up to date", name));
                }
            }
        }

        // Identifier-based suggestions (edit distance); include std function names for function kind
        let mut candidates = if let Some(sp) = span.as_ref() { collect_identifiers_in_scope(source, sp.line) } else { collect_identifiers(source) };
        if kind == "function" {
            for k in known_std_functions() { candidates.push(k.to_string()); }
        }
        let mut scored: Vec<(usize, String)> = candidates
            .into_iter()
            .map(|c| (edit_distance(&name, &c), c))
            .collect();
        scored.sort_by_key(|(d, _)| *d);
        let mut help = String::new();
        if !scored.is_empty() {
            help.push_str(&format!("help: {} '{}' not found. Did you mean:", kind, name));
            let mut seen: HashSet<String> = HashSet::new();
            let mut top: Vec<String> = Vec::new();
            let threshold = if name.len() <= 3 { 1 } else { 2 };
            for (d, s) in scored.into_iter() {
                if s != name && d <= threshold && s.len() >= 2 {
                    if seen.insert(s.clone()) {
                        top.push(s);
                        if top.len() == 3 { break; }
                    }
                }
            }
            if !top.is_empty() {
                help.push_str(&format!(" {}", top.join(", ")));
            } else {
                help.push_str(" (no close matches)");
            }
        } else {
            help.push_str(&format!("help: {} '{}' not found. Consider declaring it", kind, name));
        }

        // Combine std import hint with identifier suggestions if both exist
        if !combined_help.is_empty() {
            help.push_str("\n   = ");
            help.push_str(&combined_help);
        }

        (span, Some(help))
    } else {
        (None, None)
    }
}

fn known_std_functions() -> std::collections::HashSet<&'static str> {
    [
        "pi","tau","e","ln2","ln10",
        "exp","ln","log2","log10","pow_float","powi_float",
        "sqrt","isqrt",
        "sin","cos","tan","atan","asin","acos",
        "floor","ceil","round","clamp","fmod","sign",
        "gcd","lcm","factorial","nPr","nCr",
        "sum_float","mean_float","median_float","variance_float","stddev_float",
        "sum","min","max","pow","sort","reverse"
    ].iter().cloned().collect()
}

fn extract_undefined(msg: &str) -> Option<(&'static str, String)> {
    let lower = msg.to_lowercase();
    if let Some(pos) = lower.find("undefined variable:") {
        let name = msg[pos + "Undefined variable:".len()..].trim().to_string();
        return Some(("variable", name));
    }
    if let Some(pos) = lower.find("undefined function") {
        // Handles "Undefined function 'foo'"
        let tail = msg[pos + "Undefined function".len()..].trim().trim_matches(':').trim();
        let name = tail.trim_matches('"').trim_matches('\'').to_string();
        return Some(("function", name));
    }
    if let Some(pos) = lower.find("function not found") {
        let name = msg[pos + "Function not found".len()..].trim().trim_start_matches(':').trim().to_string();
        return Some(("function", name));
    }
    None
}

fn collect_identifiers(source: &str) -> Vec<String> {
    let mut set: HashSet<String> = HashSet::new();
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("store ") {
            let rest = &t[6..];
            if let Some(name) = rest.split_whitespace().next() {
                if !name.is_empty() { set.insert(name.to_string()); }
            }
        } else if t.starts_with("def ") {
            let rest = &t[4..];
            let name = rest.split('(').next().unwrap_or("").trim();
            if !name.is_empty() { set.insert(name.to_string()); }
            if let Some(params_part) = rest.split('(').nth(1) {
                let params_text = params_part.split(')').next().unwrap_or("");
                for p in params_text.split(',') {
                    let pname = p.trim().split(':').next().unwrap_or("").trim();
                    if !pname.is_empty() { set.insert(pname.to_string()); }
                }
            }
        }
    }
    set.into_iter().collect()
}

fn collect_identifiers_in_scope(source: &str, line: usize) -> Vec<String> {
    let mut set: HashSet<String> = HashSet::new();
    let functions = collect_function_names(source);
    for f in functions.iter() { set.insert(f.clone()); }
    if let Some((_fname, start, end)) = find_enclosing_function_range(source, line) {
        for (idx, l) in source.lines().enumerate() {
            if idx + 1 >= start && idx + 1 <= end {
                let t = l.trim();
                if t.starts_with("store ") {
                    let rest = &t[6..];
                    if let Some(name) = rest.split_whitespace().next() {
                        if !name.is_empty() { set.insert(name.to_string()); }
                    }
                } else if t.starts_with("def ") {
                    let rest = &t[4..];
                    let name = rest.split('(').next().unwrap_or("").trim();
                    if !name.is_empty() { set.insert(name.to_string()); }
                    if let Some(params_part) = rest.split('(').nth(1) {
                        let params_text = params_part.split(')').next().unwrap_or("");
                        for p in params_text.split(',') {
                            let pname = p.trim().split(':').next().unwrap_or("").trim();
                            if !pname.is_empty() { set.insert(pname.to_string()); }
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
            if !name.is_empty() { v.push(name.to_string()); }
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
            if ch == '{' { depth += 1; }
            else if ch == '}' { depth -= 1; }
        }
        if let Some((ref fname, start_line)) = current {
            if depth <= 0 && idx + 1 > start_line {
                if line >= start_line && line <= idx + 1 { return Some((fname.clone(), start_line, idx + 1)); }
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
                    if c.is_ascii_alphanumeric() || c == '_' { idx += 1; } else { break; }
                }
                let ident = &line[start..idx];
                if ident == name { return Some(Span { line: i + 1, column: start + 1 }); }
            } else {
                idx += 1;
            }
        }
    }
    None
}

fn edit_distance(a: &str, b: &str) -> usize {
    let mut dp = vec![vec![0; b.len() + 1]; a.len() + 1];
    for i in 0..=a.len() { dp[i][0] = i; }
    for j in 0..=b.len() { dp[0][j] = j; }
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if ab[i - 1] == bb[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1).min(dp[i][j - 1] + 1).min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a.len()][b.len()]
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
    if span.line == 0 { span.line = 1; }
    // Special-case missing semicolon: point to end of previous non-empty line
    let ml = message.to_lowercase();
    if ml.contains("expected ';'") {
        let mut idx = span.line.saturating_sub(1);
        while idx > 0 {
            if let Some(l) = get_line(source, idx) {
                let t = l.trim_end();
                if !t.trim().is_empty() { return Span { line: idx, column: t.len().max(1) } }
            }
            idx -= 1;
        }
    }
    let line_text = get_line(source, span.line).unwrap_or("");
    let mut col = if span.column == 0 { find_column_in_line(line_text, lexeme_hint) } else { span.column };
    if span.column == 0 {
        let is_unknown_method = {
            let tl = title.to_lowercase();
            let ml = message.to_lowercase();
            tl.contains("unknown string method") || ml.contains("unknown string method") || tl.contains("unknown array method") || ml.contains("unknown array method")
        };
        if !is_unknown_method {
            if let Some((hcol, _)) = analyze_line_for_call_errors(line_text) { col = hcol; }
        }
        if title.to_lowercase().contains("expected type") {
            if let Some((tcol, _)) = analyze_line_for_type_errors(line_text) { col = tcol; }
        }
    }
    Span { line: span.line, column: col }
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
                if let Some(h) = suggest_unknown_array_method(&mname) { extra_help = Some(h); }
            } else if lower.contains("unknown pool method") {
                if let Some(h) = suggest_unknown_pool_method(&mname) { extra_help = Some(h); }
            } else if lower.contains("unknown tree method") {
                if let Some(h) = suggest_unknown_tree_method(&mname) { extra_help = Some(h); }
            } else if let Some(h) = suggest_unknown_method(&mname) { extra_help = Some(h); }
        }
    }
    extra_help
}