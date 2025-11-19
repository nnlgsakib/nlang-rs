use regex::Regex;
use crate::interpreter::value::Value;

pub fn split(s: &str, delim: &str) -> Vec<Value> {
    if delim.is_empty() {
        return s.chars().map(|c| Value::String(c.to_string())).collect();
    }
    s.split(delim).map(|part| Value::String(part.to_string())).collect()
}

pub fn join(elements: &[Value], delim: &str) -> Result<String, String> {
    let mut out = String::new();
    for (i, v) in elements.iter().enumerate() {
        if i > 0 { out.push_str(delim); }
        match v {
            Value::String(s) => out.push_str(s),
            _ => return Err("join() elements must be strings".to_string()),
        }
    }
    Ok(out)
}

pub fn replace(s: &str, from: &str, to: &str) -> String {
    s.replace(from, to)
}

pub fn substring(s: &str, start: usize, end: usize) -> Result<String, String> {
    if start > end { return Err("substring start > end".to_string()); }
    let bytes = s.as_bytes();
    if start > bytes.len() || end > bytes.len() { return Err("substring out of range".to_string()); }
    let sub = &s[start..end];
    Ok(sub.to_string())
}

pub fn regex_matches(s: &str, pattern: &str) -> Result<Vec<Value>, String> {
    let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
    let mut out = Vec::new();
    for cap in re.captures_iter(s) {
        if let Some(m) = cap.get(0) {
            out.push(Value::String(m.as_str().to_string()));
        }
    }
    Ok(out)
}