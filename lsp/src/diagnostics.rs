//! Convert parse errors to LSP diagnostics.

use crate::document::DocumentStore;
use crate::parser::ParseError;
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use ropey::Rope;

/// Convert parse errors to LSP diagnostics using rope for position conversion.
pub fn parse_errors_to_diagnostics(errors: &[ParseError], rope: &Rope) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|err| {
            let start = DocumentStore::offset_to_position(rope, err.offset);
            let end = DocumentStore::offset_to_position(rope, err.offset + err.len);

            Diagnostic {
                range: Range {
                    start: Position {
                        line: start.line,
                        character: start.character,
                    },
                    end: Position {
                        line: end.line,
                        character: end.character,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("kql".to_string()),
                message: err.message.clone(),
                ..Default::default()
            }
        })
        .collect()
}
