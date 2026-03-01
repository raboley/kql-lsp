//! Convert parse errors and schema violations to LSP diagnostics.

use crate::catalog;
use crate::document::DocumentStore;
use crate::lexer;
use crate::parser::ParseError;
use crate::schema::{SchemaSource, SchemaStore};
use crate::syntax::SyntaxKind;
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

/// Generate diagnostics for unknown table/column references based on schema.
pub fn schema_diagnostics(text: &str, schema: &SchemaStore, rope: &Rope) -> Vec<Diagnostic> {
    if !schema.is_loaded() {
        return Vec::new();
    }

    let severity = match schema.source {
        SchemaSource::Live => DiagnosticSeverity::ERROR,
        SchemaSource::Static => DiagnosticSeverity::WARNING,
        SchemaSource::None => return Vec::new(),
    };

    let mut diagnostics = Vec::new();

    // Split text into query blocks (separated by blank lines or semicolons)
    for block in split_query_blocks(text) {
        let tokens = lexer::lex(block.text);
        let meaningful: Vec<TokenWithOffset> = tokens_with_offsets(&tokens, block.offset);

        if meaningful.is_empty() {
            continue;
        }

        // First meaningful identifier is the table name
        let first = &meaningful[0];
        if first.kind != SyntaxKind::Identifier {
            continue;
        }

        let table_name = &text[first.abs_offset..first.abs_offset + first.len];

        if !schema.has_table(table_name) {
            let start = DocumentStore::offset_to_position(rope, first.abs_offset);
            let end = DocumentStore::offset_to_position(rope, first.abs_offset + first.len);
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position { line: start.line, character: start.character },
                    end: Position { line: end.line, character: end.character },
                },
                severity: Some(severity),
                source: Some("kql-schema".to_string()),
                message: format!("Unknown table '{}'", table_name),
                ..Default::default()
            });
            continue; // Can't check columns for unknown table
        }

        // Check column references in column positions
        check_column_references(
            &meaningful,
            text,
            table_name,
            schema,
            severity,
            rope,
            &mut diagnostics,
        );
    }

    diagnostics
}

struct QueryBlock<'a> {
    text: &'a str,
    offset: usize, // byte offset in the original text
}

fn split_query_blocks(text: &str) -> Vec<QueryBlock<'_>> {
    let mut blocks = Vec::new();
    let mut start = 0;

    // Split on blank lines
    for (i, _) in text.match_indices("\n\n") {
        let block_text = &text[start..i];
        if !block_text.trim().is_empty() {
            blocks.push(QueryBlock { text: block_text, offset: start });
        }
        start = i + 2;
    }

    // Last block
    let remaining = &text[start..];
    if !remaining.trim().is_empty() {
        blocks.push(QueryBlock { text: remaining, offset: start });
    }

    blocks
}

struct TokenWithOffset {
    kind: SyntaxKind,
    abs_offset: usize, // absolute offset in original text
    len: usize,
}

fn tokens_with_offsets(tokens: &[lexer::Token], base_offset: usize) -> Vec<TokenWithOffset> {
    let mut result = Vec::new();
    let mut pos = 0;
    for token in tokens {
        if !catalog::is_trivia(token.kind) {
            result.push(TokenWithOffset {
                kind: token.kind,
                abs_offset: base_offset + pos,
                len: token.len,
            });
        }
        pos += token.len;
    }
    result
}

fn check_column_references(
    tokens: &[TokenWithOffset],
    text: &str,
    table_name: &str,
    schema: &SchemaStore,
    severity: DiagnosticSeverity,
    rope: &Rope,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Walk tokens looking for identifiers after column-accepting operators
    let is_column_operator = |kind: SyntaxKind| {
        matches!(
            kind,
            SyntaxKind::WhereKw
                | SyntaxKind::ProjectKw
                | SyntaxKind::ExtendKw
                | SyntaxKind::DistinctKw
        )
    };

    let mut in_column_context = false;

    for (i, token) in tokens.iter().enumerate() {
        // Enter column context after a column-accepting operator
        if is_column_operator(token.kind) || token.kind == SyntaxKind::ByKw {
            in_column_context = true;
            continue;
        }

        // Exit column context at pipe
        if token.kind == SyntaxKind::Pipe {
            in_column_context = false;
            continue;
        }

        // Check identifiers in column context
        if in_column_context && token.kind == SyntaxKind::Identifier {
            let name = &text[token.abs_offset..token.abs_offset + token.len];

            // Skip function calls (next token is LParen)
            if i + 1 < tokens.len() && tokens[i + 1].kind == SyntaxKind::LParen {
                continue;
            }

            // Skip if it's a known built-in function name
            if catalog::find_function(name).is_some() {
                continue;
            }

            // Skip if it matches the table name (e.g., used in join context)
            if name.eq_ignore_ascii_case(table_name) {
                continue;
            }

            // Skip string literal values (already filtered by being identifiers)
            // Check if column exists
            if !schema.has_column(table_name, name) {
                let start = DocumentStore::offset_to_position(rope, token.abs_offset);
                let end = DocumentStore::offset_to_position(rope, token.abs_offset + token.len);
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line: start.line, character: start.character },
                        end: Position { line: end.line, character: end.character },
                    },
                    severity: Some(severity),
                    source: Some("kql-schema".to_string()),
                    message: format!("Unknown column '{}' in table '{}'", name, table_name),
                    ..Default::default()
                });
            }
        }
    }
}
