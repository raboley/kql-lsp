//! Completion support for KQL.

use crate::catalog;
use crate::lexer;
use crate::schema::SchemaStore;
use crate::syntax::SyntaxKind;

/// A completion item to return to the client.
pub struct CompletionItem {
    pub label: String,
    pub kind: i32, // LSP CompletionItemKind
    pub detail: Option<String>,
}

/// LSP CompletionItemKind values.
const COMPLETION_KIND_KEYWORD: i32 = 14;
const COMPLETION_KIND_CLASS: i32 = 7;
const COMPLETION_KIND_FIELD: i32 = 5;

/// Compute completions at the given byte offset in the source text.
pub fn complete_at(text: &str, offset: usize, schema: &SchemaStore) -> Vec<CompletionItem> {
    // Check for column completion first (most specific context)
    if schema.is_loaded() {
        if let Some(table_name) = find_column_completion_context(text, offset) {
            let columns = schema.columns_for_table(&table_name);
            if !columns.is_empty() {
                return columns
                    .iter()
                    .map(|col| CompletionItem {
                        label: col.name.clone(),
                        kind: COMPLETION_KIND_FIELD,
                        detail: Some(col.column_type.clone()),
                    })
                    .collect();
            }
        }
    }

    // Determine context: are we after a pipe?
    if is_after_pipe(text, offset) {
        return tabular_operator_completions();
    }

    // At start of query (not after pipe): offer table names from schema
    if schema.is_loaded() && is_at_query_start(text, offset) {
        return table_name_completions(schema);
    }

    Vec::new()
}

/// Check if cursor is at the start of a query statement (not after a pipe).
fn is_at_query_start(text: &str, offset: usize) -> bool {
    let prefix = &text[..offset.min(text.len())];
    let trimmed = prefix.trim_start();

    // If prefix is empty or only whitespace, we're at the start
    if trimmed.is_empty() {
        return true;
    }

    // Check if we're at the start of a line (possibly with a partial identifier)
    // Walk backwards to find the start of the current line
    let last_newline = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let current_line = &prefix[last_newline..];
    let line_trimmed = current_line.trim_start();

    // If the current line has no pipe before the cursor, and is just an identifier
    // prefix (or empty), we're at a query start position
    if !line_trimmed.contains('|') {
        // Check there's no pipe on any previous line that's part of this query
        // (i.e., no pipe in the text between the last blank line and the cursor)
        let before_cursor = &prefix[..last_newline];
        let last_blank = before_cursor.rfind("\n\n").map(|i| i + 2).unwrap_or(0);
        let query_prefix = &prefix[last_blank..];
        !query_prefix.contains('|')
    } else {
        false
    }
}

/// Returns the table name if the cursor is in a column-completion position
/// (i.e., after a tabular operator that takes column arguments).
fn find_column_completion_context(text: &str, offset: usize) -> Option<String> {
    let prefix = &text[..offset.min(text.len())];
    let tokens = lexer::lex(prefix);
    let meaningful: Vec<_> = tokens
        .iter()
        .filter(|t| !catalog::is_trivia(t.kind))
        .collect();

    if meaningful.is_empty() {
        return None;
    }

    // Check if the last meaningful token (or second-to-last if last is a partial ident)
    // is a column-accepting operator keyword
    let is_column_operator = |kind: SyntaxKind| {
        matches!(
            kind,
            SyntaxKind::WhereKw
                | SyntaxKind::ProjectKw
                | SyntaxKind::ExtendKw
                | SyntaxKind::DistinctKw
        )
    };

    // Also match "summarize ... by" and "sort by" / "order by" / "top ... by"
    let is_by_keyword = |kind: SyntaxKind| kind == SyntaxKind::ByKw;

    let last = meaningful.last().unwrap();

    let in_column_position = is_column_operator(last.kind)
        || is_by_keyword(last.kind)
        // After a comma in column lists (e.g., "| project State, ")
        || last.kind == SyntaxKind::Comma
        // After a partial identifier following a column operator or comma
        || (meaningful.len() >= 2 && {
            let prev = meaningful[meaningful.len() - 2];
            (last.kind == SyntaxKind::Identifier || catalog::is_keyword(last.kind))
                && (is_column_operator(prev.kind) || is_by_keyword(prev.kind) || prev.kind == SyntaxKind::Comma)
        });

    if !in_column_position {
        return None;
    }

    // Find the table name: walk backward through all tokens to find the first
    // identifier before any pipe in the current query
    find_table_for_query(text, offset)
}

/// Walk backward through the query text to find the source table name.
/// The table name is the first identifier at the start of the current query statement.
fn find_table_for_query(text: &str, offset: usize) -> Option<String> {
    let prefix = &text[..offset.min(text.len())];

    // Find the start of the current query block (after last blank line or start of text)
    let query_start = prefix.rfind("\n\n").map(|i| i + 2).unwrap_or(0);
    let query_text = &prefix[query_start..];

    let tokens = lexer::lex(query_text);

    // Walk tokens to find the first meaningful token and its offset
    let mut pos = 0;
    for token in &tokens {
        if !catalog::is_trivia(token.kind) {
            if token.kind == SyntaxKind::Identifier {
                return Some(query_text[pos..pos + token.len].to_string());
            }
            break;
        }
        pos += token.len;
    }

    None
}

fn table_name_completions(schema: &SchemaStore) -> Vec<CompletionItem> {
    schema
        .table_names()
        .iter()
        .map(|name| CompletionItem {
            label: name.to_string(),
            kind: COMPLETION_KIND_CLASS,
            detail: Some("Table".to_string()),
        })
        .collect()
}

fn is_after_pipe(text: &str, offset: usize) -> bool {
    let prefix = &text[..offset.min(text.len())];
    // Walk backwards past whitespace to find a pipe
    let trimmed = prefix.trim_end();
    trimmed.ends_with('|')
        || is_pipe_then_partial_ident(prefix)
}

/// Check if we're in a position like "| wh" where we started typing an operator
fn is_pipe_then_partial_ident(prefix: &str) -> bool {
    let tokens = lexer::lex(prefix);
    // Walk backwards through tokens (skipping whitespace) to find what's before the cursor
    let meaningful: Vec<_> = tokens
        .iter()
        .filter(|t| !catalog::is_trivia(t.kind))
        .collect();

    if meaningful.len() >= 2 {
        let last = meaningful[meaningful.len() - 1];
        let second_last = meaningful[meaningful.len() - 2];
        // Pattern: ... | <identifier>  (cursor is at/after the identifier)
        if second_last.kind == SyntaxKind::Pipe
            && (last.kind == SyntaxKind::Identifier
                || catalog::is_keyword(last.kind))
        {
            return true;
        }
    }

    // Also check if the last meaningful token is a pipe
    if let Some(last) = meaningful.last() {
        if last.kind == SyntaxKind::Pipe {
            return true;
        }
    }

    false
}

fn tabular_operator_completions() -> Vec<CompletionItem> {
    catalog::TABULAR_OPERATORS
        .iter()
        .map(|op| CompletionItem {
            label: op.name.to_string(),
            kind: COMPLETION_KIND_KEYWORD,
            detail: Some(op.description.to_string()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_schema() -> SchemaStore {
        SchemaStore::new()
    }

    #[test]
    fn complete_after_pipe() {
        let items = complete_at("StormEvents | ", 14, &empty_schema());
        assert!(items.len() >= 10, "Should have tabular operators");
        let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"where"));
        assert!(labels.contains(&"project"));
        assert!(labels.contains(&"summarize"));
        assert!(labels.contains(&"take"));
    }

    #[test]
    fn complete_after_pipe_no_space() {
        let items = complete_at("StormEvents |", 13, &empty_schema());
        assert!(items.len() >= 10, "Should have tabular operators even without trailing space");
    }

    #[test]
    fn complete_after_pipe_partial_keyword() {
        let items = complete_at("StormEvents | wh", 16, &empty_schema());
        assert!(items.len() >= 10, "Should return all tabular operators for partial input");
    }

    #[test]
    fn no_completion_at_start_without_schema() {
        let items = complete_at("Storm", 5, &empty_schema());
        assert!(items.is_empty(), "Should not complete at start without schema");
    }

    fn test_schema() -> SchemaStore {
        let mut schema = SchemaStore::new();
        schema.load(
            crate::schema::DatabaseSchema {
                database: "TestDB".to_string(),
                tables: vec![
                    crate::schema::Table {
                        name: "StormEvents".to_string(),
                        columns: vec![
                            crate::schema::Column {
                                name: "State".to_string(),
                                column_type: "string".to_string(),
                            },
                            crate::schema::Column {
                                name: "EventType".to_string(),
                                column_type: "string".to_string(),
                            },
                            crate::schema::Column {
                                name: "DamageProperty".to_string(),
                                column_type: "long".to_string(),
                            },
                        ],
                    },
                    crate::schema::Table {
                        name: "PopulationData".to_string(),
                        columns: vec![],
                    },
                ],
            },
            crate::schema::SchemaSource::Static,
        );
        schema
    }

    #[test]
    fn complete_columns_after_where() {
        let schema = test_schema();
        let items = complete_at("StormEvents | where ", 20, &schema);
        let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"State"), "Should contain State column");
        assert!(labels.contains(&"EventType"), "Should contain EventType column");
        assert!(labels.contains(&"DamageProperty"), "Should contain DamageProperty column");
    }

    #[test]
    fn complete_columns_after_project() {
        let schema = test_schema();
        let items = complete_at("StormEvents | project ", 22, &schema);
        let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"State"));
        assert!(labels.contains(&"EventType"));
    }

    #[test]
    fn no_columns_for_unknown_table() {
        let schema = test_schema();
        let items = complete_at("UnknownTable | where ", 21, &schema);
        let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
        // Should get tabular operators, not columns
        assert!(!labels.contains(&"State"));
        assert!(labels.contains(&"where") || labels.contains(&"project"),
            "Should get tabular operators for unknown table");
    }

    #[test]
    fn complete_table_names_at_start() {
        let mut schema = SchemaStore::new();
        schema.load(
            crate::schema::DatabaseSchema {
                database: "TestDB".to_string(),
                tables: vec![
                    crate::schema::Table {
                        name: "StormEvents".to_string(),
                        columns: vec![],
                    },
                    crate::schema::Table {
                        name: "PopulationData".to_string(),
                        columns: vec![],
                    },
                ],
            },
            crate::schema::SchemaSource::Static,
        );
        let items = complete_at("Storm", 5, &schema);
        let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"StormEvents"));
        assert!(labels.contains(&"PopulationData"));
    }
}
