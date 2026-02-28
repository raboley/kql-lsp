use std::io::Write;

use log::*;
use serde::{Deserialize, Serialize};

/// Find the end of a complete LSP message in the buffer.
/// Returns `Some(end_index)` if a full message is available, `None` otherwise.
pub fn find_complete_message(buffer: &[u8]) -> Option<usize> {
    let separator = b"\r\n\r\n";
    let pos = buffer
        .windows(separator.len())
        .position(|window| window == separator)?;

    let content_length = get_content_length(buffer, pos).ok()?;
    let message_end = pos + separator.len() + content_length;

    if message_end <= buffer.len() {
        Some(message_end)
    } else {
        None
    }
}

/// Decode an LSP message: extract the method name and the JSON body.
pub fn decode_message(msg: &[u8]) -> Result<(String, &[u8]), String> {
    let separator = b"\r\n\r\n";
    let Some(pos) = msg
        .windows(separator.len())
        .position(|window| window == separator)
    else {
        return Err(r"Failed to find \r\n\r\n separator in bytes".to_owned());
    };

    let content_length: usize = get_content_length(msg, pos)?;
    let json_start = pos + separator.len();
    let json_end = json_start + content_length;

    if json_end > msg.len() {
        return Err("Content-Length exceeds message length".to_owned());
    }

    let json_bytes = &msg[json_start..json_end];

    // Extract just the method field
    let result = serde_json::from_slice::<BaseMessage>(json_bytes);
    match result {
        Ok(t) => Ok((t.method.unwrap_or_default(), json_bytes)),
        Err(e) => Err(format!("Could not parse json: {}", e)),
    }
}

/// Encode an LSP message with Content-Length header.
/// CRITICAL: Always flush stdout after writing! Without flush, pipe-based
/// communication (used by lsp4ij and Neovim) will buffer indefinitely.
pub fn encode_message(msg: impl Serialize) -> Result<String, serde_json::Error> {
    let content = serde_json::to_string(&msg)?;
    Ok(format!(
        "Content-Length: {}\r\n\r\n{}",
        content.as_bytes().len(),
        content
    ))
}

/// Write an LSP response to the writer and FLUSH.
/// The flush is critical for pipe-based communication.
pub fn write_response<W: Write>(writer: &mut W, msg: impl Serialize, label: &str) {
    let Ok(response) = encode_message(&msg) else {
        error!("Could not encode message for: {}", label);
        return;
    };
    debug!("sent: {}", response);
    if let Err(e) = write!(writer, "{}", response) {
        error!("Could not write message for: {}, error: {}", label, e);
    };
    // CRITICAL: flush() is required for pipe-based LSP communication
    if let Err(e) = writer.flush() {
        error!("Could not flush writer for: {}, error: {}", label, e);
    }
}

fn get_content_length(msg: &[u8], pos: usize) -> Result<usize, String> {
    let content_length_bytes = &msg["Content-Length: ".as_bytes().len()..pos];
    let content_length_str = std::str::from_utf8(content_length_bytes)
        .map_err(|_| "Invalid UTF-8 in content length".to_owned())?;

    let content_length: usize = content_length_str
        .trim()
        .parse()
        .map_err(|_| "Failed to parse content length".to_owned())?;

    Ok(content_length)
}

/// Base message for extracting the method field.
/// `method` is optional because responses don't have one.
#[derive(Debug, Deserialize, Serialize)]
struct BaseMessage {
    method: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_message_works() {
        #[derive(Serialize)]
        struct Msg {
            method: String,
        }
        let msg = Msg {
            method: "something".to_owned(),
        };
        let result = encode_message(&msg).unwrap();
        assert_eq!(result, "Content-Length: 22\r\n\r\n{\"method\":\"something\"}");
    }

    #[test]
    fn decode_message_works() {
        let msg = b"Content-Length: 23\r\n\r\n{\"method\":\"initialize\"}";
        let (method, _body) = decode_message(msg).unwrap();
        assert_eq!(method, "initialize");
    }

    #[test]
    fn find_complete_message_works() {
        let msg = b"Content-Length: 23\r\n\r\n{\"method\":\"initialize\"}";
        assert_eq!(find_complete_message(msg), Some(msg.len()));
    }

    #[test]
    fn find_complete_message_incomplete() {
        let msg = b"Content-Length: 100\r\n\r\n{\"method\":\"initialize\"}";
        assert_eq!(find_complete_message(msg), None);
    }
}
