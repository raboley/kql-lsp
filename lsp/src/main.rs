mod catalog;
mod code_actions;
mod completion;
mod config;
mod definition;
mod diagnostics;
mod document;
mod folding;
mod formatting;
mod hover;
mod lexer;
mod parser;
mod references;
mod schema;
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

// ---------------------------------------------------------------------------
// Shared helpers to reduce boilerplate across handlers
// ---------------------------------------------------------------------------

/// Parse a JSON-RPC message, logging an error on failure.
fn parse_request(msg: &[u8], method: &str) -> Option<Value> {
    match serde_json::from_slice(msg) {
        Ok(v) => Some(v),
        Err(e) => {
            error!("couldn't parse {}: {}", method, e);
            None
        }
    }
}

/// Extract the JSON-RPC request ID (handles both string and numeric IDs).
fn get_request_id(req: &Value) -> Value {
    req.get("id").cloned().unwrap_or(Value::Number(0.into()))
}

/// Extract the textDocument URI string from request params.
fn get_uri_str<'a>(req: &'a Value) -> &'a str {
    req.get("params")
        .and_then(|p| p.get("textDocument"))
        .and_then(|td| td.get("uri"))
        .and_then(|u| u.as_str())
        .unwrap_or("unknown")
}

/// Extract cursor position from request params.
fn get_position(req: &Value) -> lsp_types::Position {
    let position = req.get("params").and_then(|p| p.get("position"));
    let line = position
        .and_then(|p| p.get("line"))
        .and_then(|l| l.as_u64())
        .unwrap_or(0) as u32;
    let character = position
        .and_then(|p| p.get("character"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;
    lsp_types::Position { line, character }
}

/// Get document text from the store by URI string.
fn get_document_text(state: &ServerState, uri_str: &str) -> String {
    if let Ok(uri) = Uri::from_str(uri_str) {
        state
            .documents
            .get(&uri)
            .map(|doc| doc.rope.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    }
}

/// Build a JSON-RPC response with a result value.
fn make_response(id: Value, result: Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

/// Convert byte offsets to an LSP range JSON value.
fn lsp_range_json(rope: &ropey::Rope, start: usize, end: usize) -> Value {
    let start_pos = document::DocumentStore::offset_to_position(rope, start);
    let end_pos = document::DocumentStore::offset_to_position(rope, end);
    serde_json::json!({
        "start": { "line": start_pos.line, "character": start_pos.character },
        "end": { "line": end_pos.line, "character": end_pos.character }
    })
}

// ---------------------------------------------------------------------------
// Message dispatch
// ---------------------------------------------------------------------------

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
        "textDocument/codeAction" => handle_code_action(writer, state, msg),
        "textDocument/formatting" => handle_formatting(writer, state, msg),
        "textDocument/foldingRange" => handle_folding_range(writer, state, msg),
        other => info!("Unhandled method: {}", other),
    }
}

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

fn handle_initialize<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "initialize") { Some(r) => r, None => return };
    let id = get_request_id(&req);

    if let Some(client_info) = req.get("params").and_then(|p| p.get("clientInfo")) {
        let name = client_info.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
        let version = client_info.get("version").and_then(|v| v.as_str()).unwrap_or("unknown");
        info!("connected to client: {}, version: {}", name, version);
    }

    // Parse rootUri for config file resolution
    let root_dir = req
        .get("params")
        .and_then(|p| p.get("rootUri"))
        .and_then(|u| u.as_str())
        .and_then(|u| {
            // rootUri is file:///path/to/dir — strip the scheme
            if let Some(path) = u.strip_prefix("file://") {
                Some(std::path::PathBuf::from(path))
            } else {
                Some(std::path::PathBuf::from(u))
            }
        });

    // Load config: initializationOptions > .kql-lsp.json in workspace root
    let init_opts = req.get("params").and_then(|p| p.get("initializationOptions"));
    let root_path = root_dir.as_deref();
    let mut lsp_config = if let Some(opts) = init_opts {
        config::LspConfig::from_init_options(opts, root_path)
    } else {
        config::LspConfig::default()
    };

    // Fall back to .kql-lsp.json if initializationOptions didn't provide schema config
    if lsp_config.schema_file.is_none() && lsp_config.adx.is_none() {
        if let Some(root) = root_path {
            if let Some(file_config) = config::LspConfig::from_file(root) {
                lsp_config = file_config;
            }
        }
    }

    // Load schema from static file if configured
    if let Some(ref schema_path) = lsp_config.schema_file {
        match schema::load_from_file(schema_path) {
            Ok(db_schema) => {
                info!("Loaded schema from file: {} ({} tables)", schema_path.display(), db_schema.tables.len());
                state.schema.load(db_schema, schema::SchemaSource::Static);
            }
            Err(e) => {
                info!("Failed to load schema file: {}", e);
            }
        }
    }

    let token_types: Vec<&str> = semantic_tokens::TOKEN_TYPES.to_vec();

    let response = make_response(id, serde_json::json!({
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
            "codeActionProvider": true,
            "documentFormattingProvider": true,
            "foldingRangeProvider": true,
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
    }));

    rpc::write_response(writer, &response, "initialize");
}

fn handle_shutdown<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "shutdown") { Some(r) => r, None => return };
    let id = get_request_id(&req);
    state.shutdown_requested = true;
    info!("Shutdown requested");
    rpc::write_response(writer, &make_response(id, Value::Null), "shutdown");
}

fn handle_did_open<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "textDocument/didOpen") { Some(r) => r, None => return };

    let text_document = match req.get("params").and_then(|p| p.get("textDocument")) {
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

    // Normalize \r\n to \n — IntelliJ's document model uses \n only,
    // and lsp4ij applies our byte-based semantic token positions to its
    // normalized document. If we keep \r bytes, positions will overshoot.
    let text = text.replace('\r', "");

    info!("Opened: {} (version {}, {} bytes)", uri_str, version, text.len());

    // Store document in rope-backed document store
    if let Ok(uri) = Uri::from_str(uri_str) {
        state.documents.open(uri, version, &text, language_id);
    }

    // Parse and publish diagnostics
    publish_diagnostics(writer, uri_str, &text, &state.schema);
}

fn handle_did_change<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "textDocument/didChange") { Some(r) => r, None => return };

    let uri_str = get_uri_str(&req);
    let version = req
        .get("params")
        .and_then(|p| p.get("textDocument"))
        .and_then(|td| td.get("version"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    // For full sync (textDocumentSync: 1), the full text is in contentChanges[0].text
    let new_text = req
        .get("params")
        .and_then(|p| p.get("contentChanges"))
        .and_then(|cc| cc.as_array())
        .and_then(|arr| arr.first())
        .and_then(|change| change.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("");

    // Normalize \r\n to \n (see didOpen comment)
    let new_text = new_text.replace('\r', "");

    info!("Changed: {} (version {}, {} bytes)", uri_str, version, new_text.len());

    // Update rope-backed document
    if let Ok(uri) = Uri::from_str(uri_str) {
        state.documents.change_full(&uri, version, &new_text);
    }

    // Parse and publish diagnostics
    publish_diagnostics(writer, uri_str, &new_text, &state.schema);
}

fn handle_did_close(state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "textDocument/didClose") { Some(r) => r, None => return };
    let uri_str = get_uri_str(&req);
    info!("Closed: {}", uri_str);

    if let Ok(uri) = Uri::from_str(uri_str) {
        state.documents.close(&uri);
    }
}

fn publish_diagnostics<W: Write>(writer: &mut W, uri_str: &str, text: &str, schema: &schema::SchemaStore) {
    let parse_result = parser::parse(text);
    let rope = ropey::Rope::from_str(text);
    let mut all_diagnostics = diagnostics::parse_errors_to_diagnostics(&parse_result.errors, &rope);
    all_diagnostics.extend(diagnostics::schema_diagnostics(text, schema, &rope));

    let diags_json: Vec<Value> = all_diagnostics
        .iter()
        .map(|d| {
            let severity = match d.severity {
                Some(lsp_types::DiagnosticSeverity::ERROR) => 1,
                Some(lsp_types::DiagnosticSeverity::WARNING) => 2,
                Some(lsp_types::DiagnosticSeverity::INFORMATION) => 3,
                Some(lsp_types::DiagnosticSeverity::HINT) => 4,
                _ => 1,
            };
            serde_json::json!({
                "range": {
                    "start": { "line": d.range.start.line, "character": d.range.start.character },
                    "end": { "line": d.range.end.line, "character": d.range.end.character }
                },
                "severity": severity,
                "source": d.source,
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
    let req = match parse_request(msg, "semanticTokens/full") { Some(r) => r, None => return };
    let id = get_request_id(&req);
    let text = get_document_text(state, get_uri_str(&req));

    let data = semantic_tokens::compute_semantic_tokens(&text);

    rpc::write_response(writer, &make_response(id, serde_json::json!({ "data": data })), "semanticTokens/full");
}

fn handle_document_symbols<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "textDocument/documentSymbol") { Some(r) => r, None => return };
    let id = get_request_id(&req);
    let text = get_document_text(state, get_uri_str(&req));

    let parse_result = parser::parse(&text);
    let doc_symbols = symbols::extract_symbols(&parse_result);
    let rope = ropey::Rope::from_str(&text);

    let lsp_symbols: Vec<Value> = doc_symbols
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "kind": s.kind,
                "range": lsp_range_json(&rope, s.range_start, s.range_end),
                "selectionRange": lsp_range_json(&rope, s.selection_start, s.selection_end)
            })
        })
        .collect();

    rpc::write_response(writer, &make_response(id, Value::Array(lsp_symbols)), "textDocument/documentSymbol");
}

fn handle_completion<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "textDocument/completion") { Some(r) => r, None => return };
    let id = get_request_id(&req);
    let pos = get_position(&req);
    let text = get_document_text(state, get_uri_str(&req));

    let rope = ropey::Rope::from_str(&text);
    let offset = document::DocumentStore::position_to_offset(&rope, pos);
    let items = completion::complete_at(&text, offset, &state.schema);

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

    rpc::write_response(writer, &make_response(id, Value::Array(lsp_items)), "textDocument/completion");
}

fn handle_hover<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "textDocument/hover") { Some(r) => r, None => return };
    let id = get_request_id(&req);
    let pos = get_position(&req);
    let text = get_document_text(state, get_uri_str(&req));

    let rope = ropey::Rope::from_str(&text);
    let offset = document::DocumentStore::position_to_offset(&rope, pos);

    let result = match hover::hover_at(&text, offset) {
        Some(hover_result) => serde_json::json!({
            "contents": {
                "kind": "markdown",
                "value": hover_result.markdown
            }
        }),
        None => Value::Null,
    };

    rpc::write_response(writer, &make_response(id, result), "textDocument/hover");
}

fn handle_definition<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "textDocument/definition") { Some(r) => r, None => return };
    let id = get_request_id(&req);
    let uri_str = get_uri_str(&req);
    let pos = get_position(&req);
    let text = get_document_text(state, uri_str);

    let rope = ropey::Rope::from_str(&text);
    let offset = document::DocumentStore::position_to_offset(&rope, pos);

    let result = match definition::find_definition(&text, offset) {
        Some(def) => serde_json::json!({
            "uri": uri_str,
            "range": lsp_range_json(&rope, def.range_start, def.range_end)
        }),
        None => Value::Null,
    };

    rpc::write_response(writer, &make_response(id, result), "textDocument/definition");
}

fn handle_references<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "textDocument/references") { Some(r) => r, None => return };
    let id = get_request_id(&req);
    let uri_str = get_uri_str(&req);
    let pos = get_position(&req);
    let text = get_document_text(state, uri_str);

    let rope = ropey::Rope::from_str(&text);
    let offset = document::DocumentStore::position_to_offset(&rope, pos);
    let refs = references::find_references(&text, offset);

    let lsp_locations: Vec<Value> = refs
        .iter()
        .map(|r| {
            serde_json::json!({
                "uri": uri_str,
                "range": lsp_range_json(&rope, r.offset, r.offset + r.len)
            })
        })
        .collect();

    rpc::write_response(writer, &make_response(id, Value::Array(lsp_locations)), "textDocument/references");
}

fn handle_signature_help<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "textDocument/signatureHelp") { Some(r) => r, None => return };
    let id = get_request_id(&req);
    let pos = get_position(&req);
    let text = get_document_text(state, get_uri_str(&req));

    let rope = ropey::Rope::from_str(&text);
    let offset = document::DocumentStore::position_to_offset(&rope, pos);

    let result = match signature_help::signature_help_at(&text, offset) {
        Some(help) => {
            let params: Vec<Value> = help
                .function
                .parameters
                .iter()
                .map(|p| serde_json::json!({ "label": p.label }))
                .collect();

            serde_json::json!({
                "signatures": [{
                    "label": help.function.signature,
                    "documentation": help.function.description,
                    "parameters": params
                }],
                "activeSignature": 0,
                "activeParameter": help.active_parameter
            })
        }
        None => Value::Null,
    };

    rpc::write_response(writer, &make_response(id, result), "textDocument/signatureHelp");
}

fn handle_rename<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "textDocument/rename") { Some(r) => r, None => return };
    let id = get_request_id(&req);
    let uri_str = get_uri_str(&req);
    let pos = get_position(&req);
    let new_name = req
        .get("params")
        .and_then(|p| p.get("newName"))
        .and_then(|n| n.as_str())
        .unwrap_or("");

    let text = get_document_text(state, uri_str);

    let rope = ropey::Rope::from_str(&text);
    let offset = document::DocumentStore::position_to_offset(&rope, pos);
    let refs = references::find_references(&text, offset);

    let result = if refs.is_empty() {
        Value::Null
    } else {
        let edits: Vec<Value> = refs
            .iter()
            .map(|r| {
                serde_json::json!({
                    "range": lsp_range_json(&rope, r.offset, r.offset + r.len),
                    "newText": new_name
                })
            })
            .collect();

        serde_json::json!({
            "changes": {
                uri_str: edits
            }
        })
    };

    rpc::write_response(writer, &make_response(id, result), "textDocument/rename");
}

fn handle_code_action<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "textDocument/codeAction") { Some(r) => r, None => return };
    let id = get_request_id(&req);
    let uri_str = get_uri_str(&req);

    let range = req.get("params").and_then(|p| p.get("range"));
    let start_line = range
        .and_then(|r| r.get("start"))
        .and_then(|s| s.get("line"))
        .and_then(|l| l.as_u64())
        .unwrap_or(0) as u32;
    let start_char = range
        .and_then(|r| r.get("start"))
        .and_then(|s| s.get("character"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;
    let end_line = range
        .and_then(|r| r.get("end"))
        .and_then(|s| s.get("line"))
        .and_then(|l| l.as_u64())
        .unwrap_or(0) as u32;
    let end_char = range
        .and_then(|r| r.get("end"))
        .and_then(|s| s.get("character"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;

    let text = get_document_text(state, uri_str);

    let rope = ropey::Rope::from_str(&text);
    let start_offset = document::DocumentStore::position_to_offset(
        &rope,
        lsp_types::Position { line: start_line, character: start_char },
    );
    let end_offset = document::DocumentStore::position_to_offset(
        &rope,
        lsp_types::Position { line: end_line, character: end_char },
    );

    let actions = code_actions::code_actions_at(&text, start_offset, end_offset);

    let lsp_actions: Vec<Value> = actions
        .iter()
        .map(|action| {
            serde_json::json!({
                "title": action.title,
                "kind": "quickfix",
                "edit": {
                    "changes": {
                        uri_str: [{
                            "range": lsp_range_json(&rope, action.edit_offset, action.edit_offset),
                            "newText": action.edit_text
                        }]
                    }
                }
            })
        })
        .collect();

    rpc::write_response(writer, &make_response(id, Value::Array(lsp_actions)), "textDocument/codeAction");
}

fn handle_formatting<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "textDocument/formatting") { Some(r) => r, None => return };
    let id = get_request_id(&req);
    let text = get_document_text(state, get_uri_str(&req));

    let rope = ropey::Rope::from_str(&text);
    let format_edits = formatting::format(&text);

    let lsp_edits: Vec<Value> = format_edits
        .iter()
        .map(|edit| {
            serde_json::json!({
                "range": lsp_range_json(&rope, edit.offset, edit.offset + edit.len),
                "newText": edit.new_text
            })
        })
        .collect();

    rpc::write_response(writer, &make_response(id, Value::Array(lsp_edits)), "textDocument/formatting");
}

fn handle_folding_range<W: Write>(writer: &mut W, state: &mut ServerState, msg: &[u8]) {
    let req = match parse_request(msg, "textDocument/foldingRange") { Some(r) => r, None => return };
    let id = get_request_id(&req);
    let text = get_document_text(state, get_uri_str(&req));

    let ranges = folding::folding_ranges(&text);

    let lsp_ranges: Vec<Value> = ranges
        .iter()
        .map(|r| {
            serde_json::json!({
                "startLine": r.start_line,
                "endLine": r.end_line,
                "kind": "region"
            })
        })
        .collect();

    rpc::write_response(writer, &make_response(id, Value::Array(lsp_ranges)), "textDocument/foldingRange");
}
