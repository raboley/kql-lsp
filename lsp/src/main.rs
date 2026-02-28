mod diagnostics;
mod document;
mod lexer;
mod parser;
mod rpc;
mod semantic_tokens;
mod server;
mod syntax;

use env_logger::{Builder, Target};
use log::{error, info};
use lsp_types::Uri;
use serde_json::Value;
use server::ServerState;
use std::fs::OpenOptions;
use std::io::{self, Read, stdout, Write};
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdin = io::stdin();
    let mut buffer = Vec::new();

    // Configure logging to app.log (append mode to preserve logs across restarts)
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("app.log")?;

    let mut builder = Builder::new();
    if std::env::var("RUST_LOG").is_ok() {
        builder.parse_default_env();
    } else {
        builder.filter_level(log::LevelFilter::Info);
    }
    builder.target(Target::Pipe(Box::new(log_file))).init();

    info!("Starting KQL LSP");

    let mut state = ServerState::new();

    loop {
        let mut chunk = [0u8; 1024];
        match stdin.read(&mut chunk)? {
            0 => break, // EOF
            n => buffer.extend_from_slice(&chunk[..n]),
        }

        // Process complete messages from buffer
        while let Some(message_end) = rpc::find_complete_message(&buffer) {
            let message_bytes = buffer[..message_end].to_vec();
            match rpc::decode_message(&message_bytes) {
                Ok((method, json_bytes)) => {
                    handle_message(&mut stdout(), &mut state, &method, json_bytes);
                }
                Err(e) => error!("Failed to decode message: {}", e),
            }
            buffer.drain(..message_end);
        }
    }

    Ok(())
}

fn handle_message<W: Write>(writer: &mut W, state: &mut ServerState, method: &str, msg: &[u8]) {
    info!("Received msg with method: {}", method);

    match method {
        "initialize" => handle_initialize(writer, state, msg),
        "initialized" => {
            state.initialized = true;
            info!("Client sent initialized notification");
        }
        "shutdown" => handle_shutdown(writer, state, msg),
        "exit" => {
            info!("Received exit notification, shutting down");
            std::process::exit(0);
        }
        "textDocument/didOpen" => handle_did_open(writer, state, msg),
        "textDocument/didChange" => handle_did_change(writer, state, msg),
        "textDocument/didClose" => handle_did_close(state, msg),
        "textDocument/semanticTokens/full" => handle_semantic_tokens(writer, state, msg),
        other => info!("Unhandled method: {}", other),
    }
}

fn handle_initialize<W: Write>(writer: &mut W, _state: &mut ServerState, msg: &[u8]) {
    // Parse the request to get the ID (handles both string and numeric IDs)
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse initialize request: {}", e);
            return;
        }
    };

    let id = req.get("id").cloned().unwrap_or(Value::Number(0.into()));

    if let Some(client_info) = req.get("params").and_then(|p| p.get("clientInfo")) {
        let name = client_info.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
        let version = client_info.get("version").and_then(|v| v.as_str()).unwrap_or("unknown");
        info!("connected to client: {}, version: {}", name, version);
    }

    // Build semantic token types legend
    let token_types: Vec<&str> = semantic_tokens::TOKEN_TYPES.to_vec();

    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "capabilities": {
                "textDocumentSync": 1,
                "diagnosticProvider": {
                    "interFileDependencies": false,
                    "workspaceDiagnostics": false
                },
                "semanticTokensProvider": {
                    "legend": {
                        "tokenTypes": token_types,
                        "tokenModifiers": []
                    },
                    "full": true
                }
            },
            "serverInfo": {
                "name": "kql-lsp",
                "version": "0.1.0"
            }
        }
    });

    rpc::write_response(writer, &response, "initialize");
}

fn handle_shutdown<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse shutdown request: {}", e);
            return;
        }
    };

    let id = req.get("id").cloned().unwrap_or(Value::Number(0.into()));
    state.shutdown_requested = true;
    info!("Shutdown requested");

    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": null
    });

    rpc::write_response(writer, &response, "shutdown");
}

fn handle_did_open<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse textDocument/didOpen: {}", e);
            return;
        }
    };

    let params = match req.get("params") {
        Some(p) => p,
        None => return,
    };

    let text_document = match params.get("textDocument") {
        Some(td) => td,
        None => return,
    };

    let uri_str = text_document
        .get("uri")
        .and_then(|u| u.as_str())
        .unwrap_or("unknown");
    let text = text_document
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("");
    let version = text_document
        .get("version")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;
    let language_id = text_document
        .get("languageId")
        .and_then(|l| l.as_str())
        .unwrap_or("kql");

    info!("Opened: {} (version {}, {} bytes)", uri_str, version, text.len());

    // Store document in rope-backed document store
    if let Ok(uri) = Uri::from_str(uri_str) {
        state.documents.open(uri, version, text, language_id);
    }

    // Parse and publish diagnostics
    publish_diagnostics(writer, uri_str, text);
}

fn handle_did_change<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse textDocument/didChange: {}", e);
            return;
        }
    };

    let params = match req.get("params") {
        Some(p) => p,
        None => return,
    };

    let uri_str = params
        .get("textDocument")
        .and_then(|td| td.get("uri"))
        .and_then(|u| u.as_str())
        .unwrap_or("unknown");
    let version = params
        .get("textDocument")
        .and_then(|td| td.get("version"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    // For full sync (textDocumentSync: 1), the full text is in contentChanges[0].text
    let new_text = params
        .get("contentChanges")
        .and_then(|cc| cc.as_array())
        .and_then(|arr| arr.first())
        .and_then(|change| change.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("");

    info!("Changed: {} (version {}, {} bytes)", uri_str, version, new_text.len());

    // Update rope-backed document
    if let Ok(uri) = Uri::from_str(uri_str) {
        state.documents.change_full(&uri, version, new_text);
    }

    // Parse and publish diagnostics
    publish_diagnostics(writer, uri_str, new_text);
}

fn publish_diagnostics<W: Write>(writer: &mut W, uri_str: &str, text: &str) {
    let parse_result = parser::parse(text);
    let rope = ropey::Rope::from_str(text);
    let lsp_diagnostics = diagnostics::parse_errors_to_diagnostics(&parse_result.errors, &rope);

    // Convert to JSON
    let diags_json: Vec<serde_json::Value> = lsp_diagnostics
        .iter()
        .map(|d| {
            serde_json::json!({
                "range": {
                    "start": { "line": d.range.start.line, "character": d.range.start.character },
                    "end": { "line": d.range.end.line, "character": d.range.end.character }
                },
                "severity": 1, // Error
                "source": "kql",
                "message": d.message
            })
        })
        .collect();

    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri_str,
            "diagnostics": diags_json
        }
    });

    rpc::write_response(writer, &notification, "diagnostics");
}

fn handle_semantic_tokens<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse semanticTokens/full: {}", e);
            return;
        }
    };

    let id = req.get("id").cloned().unwrap_or(Value::Number(0.into()));

    let uri_str = req
        .get("params")
        .and_then(|p| p.get("textDocument"))
        .and_then(|td| td.get("uri"))
        .and_then(|u| u.as_str())
        .unwrap_or("unknown");

    // Get document text from store
    let text = if let Ok(uri) = Uri::from_str(uri_str) {
        state
            .documents
            .get(&uri)
            .map(|doc| doc.rope.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let data = semantic_tokens::compute_semantic_tokens(&text);

    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "data": data
        }
    });

    rpc::write_response(writer, &response, "semanticTokens/full");
}

fn handle_did_close(state: &mut ServerState, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse textDocument/didClose: {}", e);
            return;
        }
    };

    let uri_str = req
        .get("params")
        .and_then(|p| p.get("textDocument"))
        .and_then(|td| td.get("uri"))
        .and_then(|u| u.as_str())
        .unwrap_or("unknown");

    info!("Closed: {}", uri_str);

    if let Ok(uri) = Uri::from_str(uri_str) {
        state.documents.close(&uri);
    }
}
