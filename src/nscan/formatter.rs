pub fn format(text: &str) -> String {
    let mut out = String::new();
    let mut indent = 0usize;
    for line in text.lines() {
        let l = line.trim().to_string();
        if l.starts_with('}') { if indent > 0 { indent -= 1; } }
        let pad = "    ".repeat(indent);
        out.push_str(&pad);
        out.push_str(&l);
        out.push('\n');
        if l.ends_with('{') { indent += 1; }
    }
    out
}