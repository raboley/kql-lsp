mod completion;
mod definition;
mod diagnostics;
mod document;
mod hover;
mod lexer;
mod parser;
mod references;
mod signature_help;
mod rpc;
mod semantic_tokens;
mod server;
mod symbols;
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
        "textDocument/documentSymbol" => handle_document_symbols(writer, state, msg),
        "textDocument/completion" => handle_completion(writer, state, msg),
        "textDocument/hover" => handle_hover(writer, state, msg),
        "textDocument/definition" => handle_definition(writer, state, msg),
        "textDocument/references" => handle_references(writer, state, msg),
        "textDocument/signatureHelp" => handle_signature_help(writer, state, msg),
        "textDocument/rename" => handle_rename(writer, state, msg),
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
                "completionProvider": {
                    "triggerCharacters": ["|", " "]
                },
                "signatureHelpProvider": {
                    "triggerCharacters": ["(", ","]
                },
                "hoverProvider": true,
                "definitionProvider": true,
                "referencesProvider": true,
                "renameProvider": true,
                "documentSymbolProvider": true,
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

fn handle_document_symbols<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse textDocument/documentSymbol: {}", e);
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

    let text = if let Ok(uri) = Uri::from_str(uri_str) {
        state
            .documents
            .get(&uri)
            .map(|doc| doc.rope.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let parse_result = parser::parse(&text);
    let doc_symbols = symbols::extract_symbols(&parse_result);
    let rope = ropey::Rope::from_str(&text);

    let lsp_symbols: Vec<Value> = doc_symbols
        .iter()
        .map(|s| {
            let range_start = document::DocumentStore::offset_to_position(&rope, s.range_start);
            let range_end = document::DocumentStore::offset_to_position(&rope, s.range_end);
            let sel_start = document::DocumentStore::offset_to_position(&rope, s.selection_start);
            let sel_end = document::DocumentStore::offset_to_position(&rope, s.selection_end);

            serde_json::json!({
                "name": s.name,
                "kind": s.kind,
                "range": {
                    "start": { "line": range_start.line, "character": range_start.character },
                    "end": { "line": range_end.line, "character": range_end.character }
                },
                "selectionRange": {
                    "start": { "line": sel_start.line, "character": sel_start.character },
                    "end": { "line": sel_end.line, "character": sel_end.character }
                }
            })
        })
        .collect();

    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": lsp_symbols
    });

    rpc::write_response(writer, &response, "textDocument/documentSymbol");
}

fn handle_completion<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse textDocument/completion: {}", e);
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

    let position = req
        .get("params")
        .and_then(|p| p.get("position"));

    let line = position
        .and_then(|p| p.get("line"))
        .and_then(|l| l.as_u64())
        .unwrap_or(0) as u32;
    let character = position
        .and_then(|p| p.get("character"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;

    let text = if let Ok(uri) = Uri::from_str(uri_str) {
        state
            .documents
            .get(&uri)
            .map(|doc| doc.rope.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let rope = ropey::Rope::from_str(&text);
    let pos = lsp_types::Position { line, character };
    let offset = document::DocumentStore::position_to_offset(&rope, pos);

    let items = completion::complete_at(&text, offset);

    let lsp_items: Vec<Value> = items
        .iter()
        .map(|item| {
            let mut json = serde_json::json!({
                "label": item.label,
                "kind": item.kind,
            });
            if let Some(detail) = &item.detail {
                json["detail"] = Value::String(detail.clone());
            }
            json
        })
        .collect();

    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": lsp_items
    });

    rpc::write_response(writer, &response, "textDocument/completion");
}

fn handle_hover<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse textDocument/hover: {}", e);
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

    let position = req
        .get("params")
        .and_then(|p| p.get("position"));

    let line = position
        .and_then(|p| p.get("line"))
        .and_then(|l| l.as_u64())
        .unwrap_or(0) as u32;
    let character = position
        .and_then(|p| p.get("character"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;

    let text = if let Ok(uri) = Uri::from_str(uri_str) {
        state
            .documents
            .get(&uri)
            .map(|doc| doc.rope.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let rope = ropey::Rope::from_str(&text);
    let pos = lsp_types::Position { line, character };
    let offset = document::DocumentStore::position_to_offset(&rope, pos);

    let response = if let Some(hover_result) = hover::hover_at(&text, offset) {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "contents": {
                    "kind": "markdown",
                    "value": hover_result.markdown
                }
            }
        })
    } else {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": null
        })
    };

    rpc::write_response(writer, &response, "textDocument/hover");
}

fn handle_definition<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse textDocument/definition: {}", e);
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

    let position = req.get("params").and_then(|p| p.get("position"));

    let line = position
        .and_then(|p| p.get("line"))
        .and_then(|l| l.as_u64())
        .unwrap_or(0) as u32;
    let character = position
        .and_then(|p| p.get("character"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;

    let text = if let Ok(uri) = Uri::from_str(uri_str) {
        state
            .documents
            .get(&uri)
            .map(|doc| doc.rope.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let rope = ropey::Rope::from_str(&text);
    let pos = lsp_types::Position { line, character };
    let offset = document::DocumentStore::position_to_offset(&rope, pos);

    let response = if let Some(def) = definition::find_definition(&text, offset) {
        let start = document::DocumentStore::offset_to_position(&rope, def.name_start);
        let end = document::DocumentStore::offset_to_position(&rope, def.name_end);
        let range_start = document::DocumentStore::offset_to_position(&rope, def.range_start);
        let range_end = document::DocumentStore::offset_to_position(&rope, def.range_end);

        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "uri": uri_str,
                "range": {
                    "start": { "line": range_start.line, "character": range_start.character },
                    "end": { "line": range_end.line, "character": range_end.character }
                }
            }
        })
    } else {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": null
        })
    };

    rpc::write_response(writer, &response, "textDocument/definition");
}

fn handle_references<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse textDocument/references: {}", e);
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

    let position = req.get("params").and_then(|p| p.get("position"));

    let line = position
        .and_then(|p| p.get("line"))
        .and_then(|l| l.as_u64())
        .unwrap_or(0) as u32;
    let character = position
        .and_then(|p| p.get("character"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;

    let text = if let Ok(uri) = Uri::from_str(uri_str) {
        state
            .documents
            .get(&uri)
            .map(|doc| doc.rope.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let rope = ropey::Rope::from_str(&text);
    let pos = lsp_types::Position { line, character };
    let offset = document::DocumentStore::position_to_offset(&rope, pos);

    let refs = references::find_references(&text, offset);

    let lsp_locations: Vec<Value> = refs
        .iter()
        .map(|r| {
            let start = document::DocumentStore::offset_to_position(&rope, r.offset);
            let end = document::DocumentStore::offset_to_position(&rope, r.offset + r.len);
            serde_json::json!({
                "uri": uri_str,
                "range": {
                    "start": { "line": start.line, "character": start.character },
                    "end": { "line": end.line, "character": end.character }
                }
            })
        })
        .collect();

    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": lsp_locations
    });

    rpc::write_response(writer, &response, "textDocument/references");
}

fn handle_signature_help<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse textDocument/signatureHelp: {}", e);
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

    let position = req.get("params").and_then(|p| p.get("position"));

    let line = position
        .and_then(|p| p.get("line"))
        .and_then(|l| l.as_u64())
        .unwrap_or(0) as u32;
    let character = position
        .and_then(|p| p.get("character"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;

    let text = if let Ok(uri) = Uri::from_str(uri_str) {
        state
            .documents
            .get(&uri)
            .map(|doc| doc.rope.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let rope = ropey::Rope::from_str(&text);
    let pos = lsp_types::Position { line, character };
    let offset = document::DocumentStore::position_to_offset(&rope, pos);

    let response = if let Some(help) = signature_help::signature_help_at(&text, offset) {
        let params: Vec<Value> = help
            .signature
            .parameters
            .iter()
            .map(|p| {
                serde_json::json!({
                    "label": p.label
                })
            })
            .collect();

        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "signatures": [{
                    "label": help.signature.label,
                    "documentation": help.signature.documentation,
                    "parameters": params
                }],
                "activeSignature": 0,
                "activeParameter": help.active_parameter
            }
        })
    } else {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": null
        })
    };

    rpc::write_response(writer, &response, "textDocument/signatureHelp");
}

fn handle_rename<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse textDocument/rename: {}", e);
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

    let position = req.get("params").and_then(|p| p.get("position"));
    let new_name = req
        .get("params")
        .and_then(|p| p.get("newName"))
        .and_then(|n| n.as_str())
        .unwrap_or("");

    let line = position
        .and_then(|p| p.get("line"))
        .and_then(|l| l.as_u64())
        .unwrap_or(0) as u32;
    let character = position
        .and_then(|p| p.get("character"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;

    let text = if let Ok(uri) = Uri::from_str(uri_str) {
        state
            .documents
            .get(&uri)
            .map(|doc| doc.rope.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let rope = ropey::Rope::from_str(&text);
    let pos = lsp_types::Position { line, character };
    let offset = document::DocumentStore::position_to_offset(&rope, pos);

    let refs = references::find_references(&text, offset);

    let response = if refs.is_empty() {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": null
        })
    } else {
        let edits: Vec<Value> = refs
            .iter()
            .map(|r| {
                let start = document::DocumentStore::offset_to_position(&rope, r.offset);
                let end = document::DocumentStore::offset_to_position(&rope, r.offset + r.len);
                serde_json::json!({
                    "range": {
                        "start": { "line": start.line, "character": start.character },
                        "end": { "line": end.line, "character": end.character }
                    },
                    "newText": new_name
                })
            })
            .collect();

        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "changes": {
                    uri_str: edits
                }
            }
        })
    };

    rpc::write_response(writer, &response, "textDocument/rename");
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
