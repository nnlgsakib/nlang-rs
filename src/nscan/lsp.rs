use crate::formatter;
use crate::symbols;
use nlang::diagnostics;
use nlang::lexer::tokenize;
use nlang::parser::parse;
use nlang::semantic::analyze_with_file_path;
use nlang::std_lib::StdLib;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, Client};
use tracing::info;
use crate::state::{State, DocumentData, SymbolLocation};
use std::path::PathBuf;
use std::fs;
use std::collections::HashMap;

pub struct Backend {
    pub client: Client,
    pub state: State,
}

impl Backend {
    fn uri_to_path(uri: &Url) -> PathBuf { PathBuf::from(uri.to_file_path().unwrap_or_else(|_| PathBuf::from(uri.path()))) }
    fn has_import_std(text: &str) -> bool { text.lines().any(|l| l.trim_start().starts_with("import std")) }
    fn std_dir() -> PathBuf { PathBuf::from("src/std_lib/nlang") }
    fn index_std(&self) {
        let mut s = self.state.0.lock().unwrap();
        if !s.std_functions.is_empty() { return; }
        let stdlib = StdLib::new();
        s.std_functions = stdlib.functions.iter().map(|f| f.name.clone()).collect();
        if s.string_methods.is_empty() {
            for t in stdlib.types.iter() {
                if t.name == "string" {
                    for m in &t.methods { s.string_methods.push(m.name.clone()); }
                }
                if t.name == "list" || t.name == "array" {
                    for m in &t.methods { s.array_methods.push(m.name.clone()); }
                }
            }
            if s.string_methods.is_empty() { s.string_methods = vec!["upper".into(),"lower".into(),"trim".into(),"contains".into(),"split".into(),"replace".into(),"substring".into(),"regex".into()]; }
            if s.array_methods.is_empty() { s.array_methods = vec!["len".into(),"join".into()]; }
        }
        let dir = Self::std_dir();
        let mut map: HashMap<String, Vec<SymbolLocation>> = HashMap::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().map_or(false, |ext| ext == "nlang") {
                    if let Ok(src) = std::fs::read_to_string(&p) {
                        let url = Url::from_file_path(&p).unwrap_or_else(|_| Url::parse("file:///std").unwrap());
                        for (name, line, col) in symbols::extract_idents(&src) {
                            if src.lines().any(|l| l.contains(&format!("def {}", name))) {
                                map.entry(name).or_default().push(SymbolLocation { uri: url.clone(), line, character: col });
                            }
                        }
                    }
                }
            }
        }
        s.std_symbols = map;
    }

    fn extract_undefined_function(msg: &str) -> Option<String> {
        let lower = msg.to_lowercase();
        if let Some(pos) = lower.find("undefined function") {
            let tail = &msg[pos + "Undefined function".len()..];
            let name = tail.trim().trim_matches(':').trim().trim_matches('"').trim_matches('\'').to_string();
            if !name.is_empty() { return Some(name); }
        }
        if let Some(pos) = lower.find("function not found") {
            let tail = &msg[pos + "Function not found".len()..];
            let name = tail.trim().trim_matches(':').trim().trim_matches('"').trim_matches('\'').to_string();
            if !name.is_empty() { return Some(name); }
        }
        None
    }

    fn find_ident_span(text: &str, name: &str) -> Option<(u32, u32)> {
        for (i, line) in text.lines().enumerate() {
            if let Some(pos) = line.find(name) {
                let after = &line[pos + name.len()..];
                if after.trim_start().starts_with('(') || after.starts_with('(') {
                    return Some((i as u32, pos as u32));
                }
            }
        }
        for (i, line) in text.lines().enumerate() {
            if let Some(pos) = line.find(name) { return Some((i as u32, pos as u32)); }
        }
        None
    }
    async fn publish(&self, uri: Url, text: String) {
        let diag = self.make_diagnostics(&uri, &text);
        self.client.publish_diagnostics(uri.clone(), diag.clone(), None).await;
        let (path, text_for_index);
        {
            let mut s = self.state.0.lock().unwrap();
            if let Some(doc) = s.documents.get_mut(&uri) {
                doc.diagnostics = diag.clone();
                doc.text = text.clone();
            } else {
                let p = Self::uri_to_path(&uri);
                s.documents.insert(uri.clone(), DocumentData { uri: uri.clone(), path: p.clone(), text: text.clone(), diagnostics: diag.clone() });
            }
            let doc = s.documents.get(&uri).unwrap();
            path = doc.path.clone();
            text_for_index = doc.text.clone();
            let idx = symbols::index_symbols(&text_for_index, &uri);
            for (name, locs) in idx { let e = s.symbols.entry(name).or_default(); for (u,l,c) in locs { e.push(SymbolLocation { uri: u, line: l, character: c }); } }
        }
        info!(file=%path.display(), diags=diag.len(), "published diagnostics");
        self.client.log_message(MessageType::INFO, format!("checked {} ({} diagnostics)", path.display(), diag.len())).await;
    }
    fn make_diagnostics(&self, uri: &Url, text: &str) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        match tokenize(text) {
            Err(le) => {
                let base = diagnostics::Span { line: le.line.max(1), column: 0 };
                let sp = diagnostics::resolve_span("Lexer error", text, base, None, &le.message);
                let range = Range { start: Position { line: (sp.line - 1) as u32, character: (sp.column - 1) as u32 }, end: Position { line: (sp.line - 1) as u32, character: (sp.column) as u32 } };
                out.push(Diagnostic { range, severity: Some(DiagnosticSeverity::ERROR), code: Some(NumberOrString::String("lexer".into())), code_description: None, source: Some("nscan".into()), message: format!("Lexer error on line {}: {}", le.line, le.message), related_information: None, tags: None, data: None });
                return out;
            }
            Ok(tokens) => {
                match parse(&tokens) {
                    Err(pe) => {
                        let base_line = pe.line.max(1);
                        let mut sp = diagnostics::resolve_span("Parser error", text, diagnostics::Span { line: base_line, column: 0 }, None, &pe.message);
                        if pe.message.to_lowercase().contains("expected ';'") {
                            let prev_line_1based = base_line.saturating_sub(1);
                            if prev_line_1based >= 1 {
                                if let Some(prev_text) = text.lines().nth(prev_line_1based.saturating_sub(1)) {
                                    let col = prev_text.trim_end().len();
                                    sp = diagnostics::Span { line: prev_line_1based, column: if col == 0 { 1 } else { col } };
                                }
                            }
                        }
                        let range = Range { start: Position { line: (sp.line - 1) as u32, character: (sp.column - 1) as u32 }, end: Position { line: (sp.line - 1) as u32, character: (sp.column) as u32 } };
                        out.push(Diagnostic { range, severity: Some(DiagnosticSeverity::ERROR), code: Some(NumberOrString::String("parser".into())), code_description: None, source: Some("nscan".into()), message: format!("Parse error on line {}: {}", pe.line, pe.message), related_information: None, tags: None, data: None });
                        return out;
                    }
                    Ok(program) => {
                        match analyze_with_file_path(program, Some(&Self::uri_to_path(uri))) {
                            Err(se) => {
                                let msg = se.to_string();
                                let sp = if let Some(name) = Self::extract_undefined_function(&msg) {
                                    if let Some((l, c)) = Self::find_ident_span(text, &name) { diagnostics::Span { line: (l + 1) as usize, column: (c + 1) as usize } } else { diagnostics::semantic_span(text, &msg).unwrap_or(diagnostics::Span { line: 1, column: 1 }) }
                                } else { diagnostics::semantic_span(text, &msg).unwrap_or(diagnostics::Span { line: 1, column: 1 }) };
                                let mut m = msg.clone();
                                if let Some(extra) = diagnostics::semantic_extra_help(text, &msg) { m.push_str(&format!("\n{}", extra)); }
                                let end_char = if let Some(name) = Self::extract_undefined_function(&msg) { (sp.column - 1 + name.len()) as u32 } else { (sp.column) as u32 };
                                let range = Range { start: Position { line: (sp.line - 1) as u32, character: (sp.column - 1) as u32 }, end: Position { line: (sp.line - 1) as u32, character: end_char } };
                                out.push(Diagnostic { range, severity: Some(DiagnosticSeverity::ERROR), code: Some(NumberOrString::String("semantic".into())), code_description: None, source: Some("nscan".into()), message: m, related_information: None, tags: None, data: None });
                            }
                            Ok(_) => {}
                        }
                    }
                }
            }
        }
        out
    }
}

#[allow(dead_code)]
fn ident_at(text: &str, pos: Position) -> Option<(String, u32, u32)> {
    let line = text.lines().nth(pos.line as usize)?;
    let bytes = line.as_bytes();
    if (pos.character as usize) >= bytes.len() { return None; }
    let mut s = pos.character as usize;
    while s>0 { let c=bytes[s-1] as char; if c.is_ascii_alphanumeric() || c=='_' { s-=1; } else { break; } }
    let mut e = pos.character as usize;
    while e<bytes.len() { let c=bytes[e] as char; if c.is_ascii_alphanumeric() || c=='_' { e+=1; } else { break; } }
    if s<e { Some((line[s..e].to_string(), pos.line, s as u32)) } else { None }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        self.index_std();
        Ok(InitializeResult { capabilities: ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            completion_provider: Some(CompletionOptions { resolve_provider: Some(false), trigger_characters: Some(vec![".".into(),"\"".into()," ".into()]), all_commit_characters: None, work_done_progress_options: Default::default(), completion_item: None }),
            definition_provider: Some(OneOf::Left(true)),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
            workspace_symbol_provider: Some(OneOf::Left(true)),
            rename_provider: Some(OneOf::Left(true)),
            document_formatting_provider: Some(OneOf::Left(true)),
            ..Default::default()
        }, server_info: None })
    }
    async fn initialized(&self, _: InitializedParams) { let _ = &self; }
    async fn shutdown(&self) -> LspResult<()> { Ok(()) }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        info!(uri=%uri, bytes=text.len(), "did_open");
        self.publish(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.content_changes.into_iter().last().map(|c| c.text).unwrap_or_default();
        info!(uri=%uri, bytes=text.len(), "did_change");
        self.publish(uri, text).await;
    }

    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        info!(pos=?params.text_document_position.position, "completion");
            let mut items = Vec::new();
        let keys = ["def","store","@mut","import","from","as","if","else","while","for","return","break","continue","pick","when","default","repeat","until","loop","true","false","null"];
        for k in keys { items.push(CompletionItem::new_simple(k.into(), String::new())); }
        let s = self.state.0.lock().unwrap();
        if let Some(doc) = s.documents.get(&params.text_document_position.text_document.uri) {
            for (name, _) in symbols::index_symbols(&doc.text, &doc.uri) { items.push(CompletionItem::new_simple(name, String::new())); }
            if Self::has_import_std(&doc.text) {
                for f in &s.std_functions { items.push(CompletionItem::new_simple(f.clone(), "std".into())); }
            } else {
                for f in &s.std_functions { items.push(CompletionItem::new_simple(format!("std.{}", f), "std".into())); }
            }
            let pos = params.text_document_position.position;
            if let Some(line) = doc.text.lines().nth(pos.line as usize) {
                if pos.character as usize > 0 {
                    let ch = line.as_bytes()[pos.character as usize - 1] as char;
                    if ch == '.' {
                        for m in &s.string_methods { items.push(CompletionItem::new_simple(m.clone(), "string".into())); }
                        for m in &s.array_methods { items.push(CompletionItem::new_simple(m.clone(), "array".into())); }
                    }
                }
            }
        }
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> LspResult<Option<GotoDefinitionResponse>> {
        info!(pos=?params.text_document_position_params.position, "goto_definition");
        let s = self.state.0.lock().unwrap();
        if let Some(doc) = s.documents.get(&params.text_document_position_params.text_document.uri) {
            if let Some((name, _, _)) = ident_at(&doc.text, params.text_document_position_params.position) {
                if let Some(locs) = s.symbols.get(&name) { if let Some(loc) = locs.first() { return Ok(Some(GotoDefinitionResponse::Scalar(Location { uri: loc.uri.clone(), range: Range { start: Position { line: loc.line, character: loc.character }, end: Position { line: loc.line, character: loc.character+1 } } }))); } }
                if let Some(locs) = s.std_symbols.get(&name) { if let Some(loc) = locs.first() { return Ok(Some(GotoDefinitionResponse::Scalar(Location { uri: loc.uri.clone(), range: Range { start: Position { line: loc.line, character: loc.character }, end: Position { line: loc.line, character: loc.character+1 } } }))); } }
            }
        }
        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        info!(pos=?params.text_document_position_params.position, "hover");
        let s = self.state.0.lock().unwrap();
        if let Some(doc) = s.documents.get(&params.text_document_position_params.text_document.uri) {
            if let Some((name, _, _)) = ident_at(&doc.text, params.text_document_position_params.position) {
                if s.std_functions.contains(&name) {
                    return Ok(Some(Hover { contents: HoverContents::Scalar(MarkedString::String(format!("std function {}", name))), range: None }));
                }
                let line = doc.text.lines().nth(params.text_document_position_params.position.line as usize).unwrap_or("");
                if line.contains(&format!(".{}", name)) {
                    if s.string_methods.contains(&name) { return Ok(Some(Hover { contents: HoverContents::Scalar(MarkedString::String(format!("string method {}", name))), range: None })); }
                    if s.array_methods.contains(&name) { return Ok(Some(Hover { contents: HoverContents::Scalar(MarkedString::String(format!("array method {}", name))), range: None })); }
                }
                let kind = if doc.text.lines().any(|l| l.trim_start().starts_with(&format!("def {}", name))) { "function" } else { "symbol" };
                return Ok(Some(Hover { contents: HoverContents::Scalar(MarkedString::String(format!("{} {}", kind, name))), range: None }));
            }
        }
        Ok(None)
    }

    async fn code_action(&self, params: CodeActionParams) -> LspResult<Option<CodeActionResponse>> {
        info!(range=?params.range, "code_action");
        let mut actions: Vec<CodeActionOrCommand> = Vec::new();
        let s = self.state.0.lock().unwrap();
        if let Some(doc) = s.documents.get(&params.text_document.uri) {
            for d in &doc.diagnostics {
                if d.message.to_lowercase().contains("undefined function") || d.message.to_lowercase().contains("function not found") {
                    let undef = Self::extract_undefined_function(&d.message);
                    let mut replacement: Option<String> = None;
                    if let Some(extra) = diagnostics::semantic_extra_help(&doc.text, &d.message) {
                        if let Some(after) = extra.split("Did you mean:").nth(1) {
                            let first = after.split(',').next().map(|s| s.trim()).unwrap_or("");
                            if !first.is_empty() { replacement = Some(first.to_string()); }
                        }
                    }
                    if let (Some(bad), Some(rep)) = (undef, replacement.clone()) {
                        if let Some((line, col)) = Self::find_ident_span(&doc.text, &bad) {
                            let start = Position { line, character: col };
                            let end = Position { line, character: col + bad.len() as u32 };
                            let r = Range { start, end };
                            let edit = WorkspaceEdit { changes: Some([(doc.uri.clone(), vec![TextEdit { range: r, new_text: rep.clone() }])].into()), document_changes: None, change_annotations: None };
                            actions.push(CodeActionOrCommand::CodeAction(CodeAction { title: format!("Replace '{}' with {}", bad, rep), kind: Some(CodeActionKind::QUICKFIX), diagnostics: Some(vec![d.clone()]), edit: Some(edit), command: None, is_preferred: Some(true), disabled: None, data: None }));
                        } else {
                            let edit = WorkspaceEdit { changes: Some([(doc.uri.clone(), vec![TextEdit { range: d.range, new_text: rep.clone() }])].into()), document_changes: None, change_annotations: None };
                            actions.push(CodeActionOrCommand::CodeAction(CodeAction { title: format!("Replace with {}", rep), kind: Some(CodeActionKind::QUICKFIX), diagnostics: Some(vec![d.clone()]), edit: Some(edit), command: None, is_preferred: Some(true), disabled: None, data: None }));
                        }
                    }
                    let lower = d.message.to_lowercase();
                    if let Some(pos) = lower.find("undefined function '") {
                        let tail = &d.message[pos + "undefined function '".len()..];
                        if let Some(end) = tail.find("'") {
                            let fname = &tail[..end];
                            if s.std_functions.contains(&fname.to_string()) && !Self::has_import_std(&doc.text) {
                                let edit = WorkspaceEdit { changes: Some([(doc.uri.clone(), vec![TextEdit { range: Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 0 } }, new_text: "import std;\n".into() }])].into()), document_changes: None, change_annotations: None };
                                actions.push(CodeActionOrCommand::CodeAction(CodeAction { title: format!("Add import std for {}", fname), kind: Some(CodeActionKind::QUICKFIX), diagnostics: Some(vec![d.clone()]), edit: Some(edit), command: None, is_preferred: Some(true), disabled: None, data: None }));
                            }
                        }
                    }
                }
                // Quick fix: add '@mut' to variable declaration when immutability violation occurs
                let lm2 = d.message.to_lowercase();
                if lm2.contains("cannot assign to immutable variable") || lm2.contains("cannot mutate immutable variable") {
                    if let Some(name) = extract_quoted(&d.message) {
                        if let Some((line_idx, col_store)) = find_store_decl(&doc.text, &name) {
                            let start = Position { line: line_idx, character: col_store };
                            let end = Position { line: line_idx, character: col_store + 5 }; // length of 'store'
                            let r = Range { start, end };
                            let edit = WorkspaceEdit { changes: Some([(doc.uri.clone(), vec![TextEdit { range: r, new_text: "@mut store".into() }])].into()), document_changes: None, change_annotations: None };
                            actions.push(CodeActionOrCommand::CodeAction(CodeAction { title: format!("Make '{}' mutable (@mut)", name), kind: Some(CodeActionKind::QUICKFIX), diagnostics: Some(vec![d.clone()]), edit: Some(edit), command: None, is_preferred: Some(true), disabled: None, data: None }));
                        }
                    }
                }
                let lm = d.message.to_lowercase();
                if lm.contains("unknown string method") || lm.contains("unknown array method") || lm.contains("unknown method") {
                    if let Some(extra) = diagnostics::semantic_extra_help(&doc.text, &d.message) {
                        if let Some(sugg) = extra.split(':').last() {
                            let name = sugg.trim().trim_end_matches('?');
                            if !name.is_empty() {
                                let l = d.range.start.line as usize;
                                let line = doc.text.lines().nth(l).unwrap_or("");
                                if let Some(dot) = line.find('.') {
                                    let start = (dot + 1) as u32;
                                    let end = start + name.len() as u32;
                                    let r = Range { start: Position { line: d.range.start.line, character: start }, end: Position { line: d.range.start.line, character: end } };
                                    let edit = WorkspaceEdit { changes: Some([(doc.uri.clone(), vec![TextEdit { range: r, new_text: name.to_string() }])].into()), document_changes: None, change_annotations: None };
                                    actions.push(CodeActionOrCommand::CodeAction(CodeAction { title: format!("Replace method with {}", name), kind: Some(CodeActionKind::QUICKFIX), diagnostics: Some(vec![d.clone()]), edit: Some(edit), command: None, is_preferred: Some(true), disabled: None, data: None }));
                                }
                            }
                        }
                    }
                }
                if d.message.contains("len() function") { let name = "import std;"; let edit = WorkspaceEdit { changes: Some([(doc.uri.clone(), vec![TextEdit { range: Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 0 } }, new_text: format!("{}\n", name) }])].into()), document_changes: None, change_annotations: None }; actions.push(CodeActionOrCommand::CodeAction(CodeAction { title: "Add import std".into(), kind: Some(CodeActionKind::QUICKFIX), diagnostics: Some(vec![d.clone()]), edit: Some(edit), command: None, is_preferred: Some(false), disabled: None, data: None })); }
            }
            let fmt = formatter::format(&doc.text);
            let end_line = doc.text.lines().count() as u32;
            let edit = WorkspaceEdit { changes: Some([(doc.uri.clone(), vec![TextEdit { range: Range { start: Position { line: 0, character: 0 }, end: Position { line: end_line, character: 0 } }, new_text: fmt }])].into()), document_changes: None, change_annotations: None };
            actions.push(CodeActionOrCommand::CodeAction(CodeAction { title: "Format document".into(), kind: Some(CodeActionKind::REFACTOR), diagnostics: None, edit: Some(edit), command: None, is_preferred: Some(false), disabled: None, data: None }));
        }
        Ok(Some(actions))
    }


    async fn rename(&self, params: RenameParams) -> LspResult<Option<WorkspaceEdit>> {
        info!(new_name=%params.new_name, pos=?params.text_document_position.position, "rename");
        let mut changes: std::collections::HashMap<Url, Vec<TextEdit>> = std::collections::HashMap::new();
        let s = self.state.0.lock().unwrap();
        if let Some(doc) = s.documents.get(&params.text_document_position.text_document.uri) {
            if let Some((name, _, _)) = ident_at(&doc.text, params.text_document_position.position) {
                let idents = symbols::extract_idents(&doc.text);
                for (n, l, c) in idents { if n == name { let e = changes.entry(doc.uri.clone()).or_default(); e.push(TextEdit { range: Range { start: Position { line: l, character: c }, end: Position { line: l, character: c + n.len() as u32 } }, new_text: params.new_name.clone() }); } }
            }
        }
        Ok(Some(WorkspaceEdit { changes: Some(changes), document_changes: None, change_annotations: None }))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> LspResult<Option<Vec<TextEdit>>> {
        info!(uri=%params.text_document.uri, "formatting");
        let s = self.state.0.lock().unwrap();
        if let Some(doc) = s.documents.get(&params.text_document.uri) {
            let fmt = formatter::format(&doc.text);
            let end_line = doc.text.lines().count() as u32;
            return Ok(Some(vec![TextEdit { range: Range { start: Position { line: 0, character: 0 }, end: Position { line: end_line, character: 0 } }, new_text: fmt }]))
        }
        Ok(Some(vec![]))
    }

    async fn symbol(&self, params: WorkspaceSymbolParams) -> LspResult<Option<Vec<SymbolInformation>>> {
        info!(query=%params.query, "workspace/symbol");
        let s = self.state.0.lock().unwrap();
        let mut infos: Vec<SymbolInformation> = Vec::new();
        for (name, locs) in s.symbols.iter() {
            if params.query.is_empty() || name.contains(&params.query) {
                if let Some(loc) = locs.first() {
                    #[allow(deprecated)]
                    infos.push(SymbolInformation {
                        name: name.clone(),
                        kind: SymbolKind::FUNCTION,
                        location: Location { uri: loc.uri.clone(), range: Range { start: Position { line: loc.line, character: loc.character }, end: Position { line: loc.line, character: loc.character + 1 } } },
                        container_name: None,
                        deprecated: None,
                        tags: None,
                    });
                }
            }
        }
        for (name, locs) in s.std_symbols.iter() {
            if params.query.is_empty() || name.contains(&params.query) {
                if let Some(loc) = locs.first() {
                    #[allow(deprecated)]
                    infos.push(SymbolInformation {
                        name: name.clone(),
                        kind: SymbolKind::FUNCTION,
                        location: Location { uri: loc.uri.clone(), range: Range { start: Position { line: loc.line, character: loc.character }, end: Position { line: loc.line, character: loc.character + 1 } } },
                        container_name: None,
                        deprecated: None,
                        tags: None,
                    });
                }
            }
        }
        Ok(Some(infos))
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = if let Some(t) = params.text { t } else { std::fs::read_to_string(uri.to_file_path().unwrap_or_default()).unwrap_or_default() };
        info!(uri=%uri, bytes=text.len(), "did_save");
        self.publish(uri, text).await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        info!(count=params.changes.len(), "did_change_watched_files");
        let mut to_recheck: Vec<(Url, String)> = Vec::new();
        {
            let s = self.state.0.lock().unwrap();
            for (u, d) in s.documents.iter() { to_recheck.push((u.clone(), d.text.clone())); }
        }
        for (u, t) in to_recheck { self.publish(u, t).await; }
    }
}

fn extract_quoted(msg: &str) -> Option<String> {
    if let Some(pos) = msg.find('\'') {
        let tail = &msg[pos+1..];
        if let Some(end) = tail.find('\'') { return Some(tail[..end].to_string()); }
    }
    if let Some(pos) = msg.find('"') {
        let tail = &msg[pos+1..];
        if let Some(end) = tail.find('"') { return Some(tail[..end].to_string()); }
    }
    None
}

fn find_store_decl(text: &str, name: &str) -> Option<(u32, u32)> {
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("store ") || trimmed.starts_with("@mut store ") {
            if let Some(_pos_name) = trimmed.find(&format!(" {}", name)) {
                // compute original column accounting for leading whitespace
                let leading = line.len() - trimmed.len();
                let pos_store = trimmed.find("store ").unwrap_or(0) as u32;
                return Some((i as u32, (leading as u32) + pos_store));
            }
        }
    }
    None
}