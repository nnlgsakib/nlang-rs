use clap::Parser;
use serde::Serialize;
use std::path::{Path, PathBuf};
use nlang::lexer::tokenize;
use nlang::parser::parse;
use nlang::semantic::analyzer::SemanticAnalyzer;
use nlang::diagnostics;
use tower_lsp::{LspService, Server};
use tracing_subscriber::EnvFilter;
mod state;
mod lsp;
mod formatter;
mod symbols;
use state::State;

#[derive(Parser)]
#[command(name = "nscan")]
#[command(about = "Static analyzer and diagnostics for Nlang code")] 
struct NscanCli {
    #[arg(long, value_name = "FILE")]
    code: Option<PathBuf>,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    lsp: bool,
    #[arg(long, default_value_t=9257)]
    port: u16,
}

#[derive(Serialize)]
struct LspPosition { line: u32, character: u32 }

#[derive(Serialize)]
struct LspRange { start: LspPosition, end: LspPosition }

#[derive(Serialize)]
struct LspDiagnostic {
    range: LspRange,
    severity: u8,
    code: String,
    source: String,
    message: String,
}

#[derive(Serialize)]
struct PublishDiagnosticsParams {
    uri: String,
    diagnostics: Vec<LspDiagnostic>,
}

#[derive(Serialize)]
struct LspEnvelope {
    jsonrpc: &'static str,
    method: &'static str,
    params: PublishDiagnosticsParams,
}

fn file_uri(path: &Path) -> String {
    let p = path.canonicalize().unwrap_or(path.to_path_buf());
    let mut s = p.to_string_lossy().to_string();
    if s.starts_with("\\\\?\\") { s = s.trim_start_matches("\\\\?\\").to_string(); }
    if s.starts_with("//?/") { s = s.trim_start_matches("//?/").to_string(); }
    let s = s.replace("\\", "/");
    format!("file:///{}", s)
}



fn main() -> anyhow::Result<()> {
    let cli = NscanCli::parse();
    if cli.lsp || cli.code.is_none() {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
            .with_target(false)
            .init();
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async move {
            let addr = format!("127.0.0.1:{}", cli.port);
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
            tracing::info!(%addr, "nscan LSP listening");
            println!("nscan LSP server v{} listening at {}", env!("CARGO_PKG_VERSION"), addr);
            println!("Capabilities: diagnostics, completion, goto-definition, hover, code-actions, workspace-symbols, rename, formatting");
            println!("Transport: TCP JSON-RPC");
            let state = State::new();
            loop {
                println!("Waiting for client connection...");
                let (stream, peer) = match listener.accept().await { Ok(v) => v, Err(e) => { tracing::error!(error=%e, "accept failed"); continue } };
                tracing::info!(?peer, "client connected");
                println!("Client connected: {:?}", peer);
                let (stdin, stdout) = stream.into_split();
                let (service, socket) = LspService::new(|client| lsp::Backend { client, state: state.clone() });
                tokio::spawn(async move {
                    Server::new(stdin, stdout, socket).serve(service).await;
                    tracing::info!("client session ended");
                });
            }
        });
        return Ok(());
    }
    let input = cli.code.clone().ok_or_else(|| anyhow::anyhow!("--code <FILE> required unless --lsp"))?;
    if !input.extension().map_or(false, |e| e == "nlang") { anyhow::bail!("Input must be a .nlang file"); }
    let source = std::fs::read_to_string(&input)?;
    let uri = file_uri(&input);

    match tokenize(&source) {
        Err(le) => {
            if cli.json {
                let base_span = diagnostics::Span { line: le.line.max(1), column: 0 };
                let rs = diagnostics::resolve_span("Lexer error", &source, base_span, None, &le.message);
                let diag = LspDiagnostic {
                    range: LspRange { start: LspPosition { line: (rs.line - 1) as u32, character: (rs.column - 1) as u32 }, end: LspPosition { line: (rs.line - 1) as u32, character: (rs.column) as u32 } },
                    severity: 1,
                    code: "lexer".to_string(),
                    source: "nscan".to_string(),
                    message: format!("Lexer error on line {}: {}", le.line, le.message),
                };
                let env = LspEnvelope { jsonrpc: "2.0", method: "textDocument/publishDiagnostics", params: PublishDiagnosticsParams { uri, diagnostics: vec![diag] } };
                println!("{}", serde_json::to_string(&env)?);
            } else {
                let err = nlang::execution_engine::ExecutionError::LexerError(le);
                let text = diagnostics::from_execution_error(&input, &source, &err);
                println!("{}", text);
            }
            return Ok(());
        }
        Ok(tokens) => {
            match parse(&tokens) {
                Err(pe) => {
                    if cli.json {
                        let base_span = diagnostics::Span { line: pe.line.max(1), column: 0 };
                        let rs = diagnostics::resolve_span("Parser error", &source, base_span, None, &pe.message);
                        let msg = format!("Parse error on line {}: {}", pe.line, pe.message);
                        let diag = LspDiagnostic {
                            range: LspRange { start: LspPosition { line: (rs.line - 1) as u32, character: (rs.column - 1) as u32 }, end: LspPosition { line: (rs.line - 1) as u32, character: (rs.column) as u32 } },
                            severity: 1,
                            code: "parser".to_string(),
                            source: "nscan".to_string(),
                            message: msg,
                        };
                        let env = LspEnvelope { jsonrpc: "2.0", method: "textDocument/publishDiagnostics", params: PublishDiagnosticsParams { uri, diagnostics: vec![diag] } };
                        println!("{}", serde_json::to_string(&env)?);
                    } else {
                        let err = nlang::execution_engine::ExecutionError::ParserError(pe);
                        let text = diagnostics::from_execution_error(&input, &source, &err);
                        println!("{}", text);
                    }
                    return Ok(());
                }
                Ok(program) => {
                    let mut analyzer = SemanticAnalyzer::new_with_file_path(Some(&input));
                    match analyzer.analyze_program(program, false) {
                        Err(se) => {
                            if cli.json {
                                let msg = se.to_string();
                                let rs = diagnostics::semantic_span(&source, &msg)
                                    .unwrap_or(diagnostics::Span { line: 1, column: 1 });
                                let mut full = msg.clone();
                                if let Some(extra) = diagnostics::semantic_extra_help(&source, &msg) {
                                    full.push_str(&format!("\n{}", extra));
                                }
                                let diag = LspDiagnostic {
                                    range: LspRange { start: LspPosition { line: (rs.line - 1) as u32, character: (rs.column - 1) as u32 }, end: LspPosition { line: (rs.line - 1) as u32, character: (rs.column) as u32 } },
                                    severity: 1,
                                    code: "semantic".to_string(),
                                    source: "nscan".to_string(),
                                    message: full,
                                };
                                let env = LspEnvelope { jsonrpc: "2.0", method: "textDocument/publishDiagnostics", params: PublishDiagnosticsParams { uri, diagnostics: vec![diag] } };
                                println!("{}", serde_json::to_string(&env)?);
                            } else {
                                let err = nlang::execution_engine::ExecutionError::SemanticError(se);
                                let text = diagnostics::from_execution_error(&input, &source, &err);
                                println!("{}", text);
                            }
                            return Ok(());
                        }
                        Ok(_) => {
                            if cli.json {
                                let env = LspEnvelope { jsonrpc: "2.0", method: "textDocument/publishDiagnostics", params: PublishDiagnosticsParams { uri, diagnostics: vec![] } };
                                println!("{}", serde_json::to_string(&env)?);
                            } else {
                                println!("No issues found");
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}