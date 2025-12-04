//! Production-ready NLang Language Server and Static Analyzer
//! 
//! Provides LSP server capabilities and command-line diagnostics
//! with comprehensive error handling and recovery.

use clap::Parser;
use serde::Serialize;
use std::path::{Path, PathBuf};
use nlang::lexer::tokenize;
use nlang::parser::parse;
use nlang::semantic::analyzer::SemanticAnalyzer;
use nlang::diagnostics;
use tower_lsp::{LspService, Server};
use tracing_subscriber::EnvFilter;
use anyhow::{Context, Result};

mod state;
mod lsp;
mod formatter;
mod symbols;
use state::State;

#[derive(Parser)]
#[command(name = "nscan")]
#[command(version, about = "Production-ready static analyzer and LSP server for NLang")] 
#[command(long_about = "\nProvides comprehensive diagnostics, code completion, goto-definition,\nhover info, code actions, workspace symbols, rename, and formatting.")]
struct NscanCli {
    /// Path to NLang source file to analyze
    #[arg(long, value_name = "FILE")]
    code: Option<PathBuf>,
    
    /// Output diagnostics in JSON format (LSP protocol)
    #[arg(long)]
    json: bool,
    
    /// Start LSP server mode (default if no --code)
    #[arg(long)]
    lsp: bool,
    
    /// TCP port for LSP server
    #[arg(long, default_value_t = 9257)]
    port: u16,
    
    /// Verbose logging output
    #[arg(short, long)]
    verbose: bool,
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

/// Convert file path to proper URI format
/// 
/// Handles Windows UNC paths and ensures consistent formatting
fn file_uri(path: &Path) -> Result<String> {
    let p = path.canonicalize()
        .with_context(|| format!("Failed to canonicalize path: {}", path.display()))?;
    let mut s = p.to_string_lossy().to_string();
    
    // Remove Windows UNC prefix if present
    if s.starts_with("\\\\?\\") {
        s = s.trim_start_matches("\\\\?\\").to_string();
    }
    if s.starts_with("//?/") {
        s = s.trim_start_matches("//?/").to_string();
    }
    
    // Normalize path separators
    let s = s.replace("\\", "/");
    Ok(format!("file:///{}", s))
}



fn main() -> Result<()> {
    let cli = NscanCli::parse();
    
    // Setup logging based on verbosity
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("debug")))
            .with_target(true)
            .init();
    }
    
    // Start LSP server mode if requested or no code file provided
    if cli.lsp || cli.code.is_none() {
        if !cli.verbose {
            tracing_subscriber::fmt()
                .with_env_filter(EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new("info")))
                .with_target(false)
                .init();
        }
        
        let rt = tokio::runtime::Runtime::new()
            .context("Failed to create Tokio runtime")?;
            
        rt.block_on(async move {
            start_lsp_server(cli.port).await
        })?;
        
        return Ok(());
    }
    
    // CLI mode: analyze single file
    analyze_file(&cli)
}

/// Start LSP server on specified port
async fn start_lsp_server(port: u16) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await
        .with_context(|| format!("Failed to bind to {}", addr))?;
        
    tracing::info!(%addr, "nscan LSP listening");
    println!("nscan LSP server v{} listening at {}", env!("CARGO_PKG_VERSION"), addr);
    println!("Capabilities: diagnostics, completion, goto-definition, hover, code-actions, workspace-symbols, rename, formatting");
    println!("Transport: TCP JSON-RPC");
    
    let state = State::new();
    
    loop {
        println!("Waiting for client connection...");
        
        let (stream, peer) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error=%e, "accept failed");
                continue;
            }
        };
        
        tracing::info!(?peer, "client connected");
        println!("Client connected: {:?}", peer);
        
        let (stdin, stdout) = stream.into_split();
        let (service, socket) = LspService::new(|client| lsp::Backend {
            client,
            state: state.clone(),
        });
        
        tokio::spawn(async move {
            Server::new(stdin, stdout, socket).serve(service).await;
            tracing::info!("client session ended");
        });
    }
}

/// Analyze a single file and output diagnostics
fn analyze_file(cli: &NscanCli) -> Result<()> {
    let input = cli.code.as_ref()
        .ok_or_else(|| anyhow::anyhow!("--code <FILE> required unless --lsp"))?;
        
    if !input.extension().map_or(false, |e| e == "nlang") {
        anyhow::bail!("Input must be a .nlang file");
    }
    
    let source = std::fs::read_to_string(input)
        .with_context(|| format!("Failed to read file: {}", input.display()))?;
    let uri = file_uri(input)?;
    
    // Check if this is a module file
    let is_export = input.file_name()
        .and_then(|n| n.to_str())
        .map_or(false, |n| n == "export.nlang");
    let is_submodule = source.lines()
        .take(5)
        .any(|l| l.trim().starts_with("sub mod") || l.trim().starts_with("sub"));
    
    // export.nlang files use special module syntax - just validate lexer
    if is_export {
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
            Ok(_) => {
                // export.nlang lexer passed - no errors
                if cli.json {
                    let env = LspEnvelope { jsonrpc: "2.0", method: "textDocument/publishDiagnostics", params: PublishDiagnosticsParams { uri, diagnostics: vec![] } };
                    println!("{}", serde_json::to_string(&env)?);
                } else {
                    println!("No issues found");
                }
                return Ok(());
            }
        }
    }

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
                    // Skip semantic analysis for submodule files
                    // They use module system features not supported by standalone analyzer
                    if is_submodule {
                        if cli.json {
                            let env = LspEnvelope { jsonrpc: "2.0", method: "textDocument/publishDiagnostics", params: PublishDiagnosticsParams { uri, diagnostics: vec![] } };
                            println!("{}", serde_json::to_string(&env)?);
                        } else {
                            println!("No issues found");
                        }
                        return Ok(());
                    }
                    
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