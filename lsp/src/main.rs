mod rpc;

use env_logger::{Builder, Target};
use log::{error, info};
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::{self, Read, stdout, Write};

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
                    handle_message(&mut stdout(), &method, json_bytes);
                }
                Err(e) => error!("Failed to decode message: {}", e),
            }
            buffer.drain(..message_end);
        }
    }

    Ok(())
}

fn handle_message<W: Write>(writer: &mut W, method: &str, msg: &[u8]) {
    info!("Received msg with method: {}", method);

    match method {
        "initialize" => handle_initialize(writer, msg),
        "initialized" => info!("Client sent initialized notification"),
        "shutdown" => handle_shutdown(writer, msg),
        "exit" => {
            info!("Received exit notification, shutting down");
            std::process::exit(0);
        }
        "textDocument/didOpen" => handle_did_open(writer, msg),
        "textDocument/didChange" => handle_did_change(writer, msg),
        "textDocument/didClose" => info!("textDocument/didClose received"),
        other => info!("Unhandled method: {}", other),
    }
}

fn handle_initialize<W: Write>(writer: &mut W, msg: &[u8]) {
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

    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "capabilities": {
                "textDocumentSync": 1,
                "diagnosticProvider": {
                    "interFileDependencies": false,
                    "workspaceDiagnostics": false
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

fn handle_shutdown<W: Write>(writer: &mut W, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse shutdown request: {}", e);
            return;
        }
    };

    let id = req.get("id").cloned().unwrap_or(Value::Number(0.into()));
    info!("Shutdown requested");

    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": null
    });

    rpc::write_response(writer, &response, "shutdown");
}

fn handle_did_open<W: Write>(writer: &mut W, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse textDocument/didOpen: {}", e);
            return;
        }
    };

    let uri = req
        .get("params")
        .and_then(|p| p.get("textDocument"))
        .and_then(|td| td.get("uri"))
        .and_then(|u| u.as_str())
        .unwrap_or("unknown");

    info!("Opened: {}", uri);

    // For now, publish empty diagnostics (features come later via TDD)
    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "diagnostics": []
        }
    });

    rpc::write_response(writer, &notification, "didOpen diagnostics");
}

fn handle_did_change<W: Write>(writer: &mut W, msg: &[u8]) {
    let req: Value = match serde_json::from_slice(msg) {
        Ok(v) => v,
        Err(e) => {
            error!("couldn't parse textDocument/didChange: {}", e);
            return;
        }
    };

    let uri = req
        .get("params")
        .and_then(|p| p.get("textDocument"))
        .and_then(|td| td.get("uri"))
        .and_then(|u| u.as_str())
        .unwrap_or("unknown");

    info!("Changed: {}", uri);

    // For now, publish empty diagnostics (features come later via TDD)
    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "diagnostics": []
        }
    });

    rpc::write_response(writer, &notification, "didChange diagnostics");
}
