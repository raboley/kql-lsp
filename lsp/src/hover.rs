//! Hover documentation for KQL built-in functions and operators.

use crate::catalog;
use crate::lexer;
use crate::schema::SchemaStore;
use crate::syntax::SyntaxKind;

/// Hover result.
pub struct HoverResult {
    pub markdown: String,
}

/// Get hover documentation for the token at the given byte offset.
pub fn hover_at(text: &str, offset: usize, schema: &SchemaStore) -> Option<HoverResult> {
    let tokens = lexer::lex(text);
    let mut token_offset = 0;

    for token in &tokens {
        let token_end = token_offset + token.len;

        if offset >= token_offset && offset < token_end {
            let token_text = &text[token_offset..token_end];
            // Try built-in hover first
            if let Some(result) = hover_for_token(token.kind, token_text) {
                return Some(result);
            }
            // Try schema-aware hover for identifiers
            if token.kind == SyntaxKind::Identifier && schema.is_loaded() {
                return hover_for_schema_identifier(text, token_text, token_offset, schema);
            }
            return None;
        }

        token_offset = token_end;
    }

    None
}

/// Provide hover for identifiers that might be table or column names.
fn hover_for_schema_identifier(
    text: &str,
    token_text: &str,
    token_offset: usize,
    schema: &SchemaStore,
) -> Option<HoverResult> {
    // Check if it's a table name
    if schema.has_table(token_text) {
        let columns = schema.columns_for_table(token_text);
        let col_lines: Vec<String> = columns
            .iter()
            .map(|c| format!("  {}: {}", c.name, c.column_type))
            .collect();
        let markdown = format!(
            "**{}** (table)\n\n**Columns:**\n```\n{}\n```",
            token_text,
            col_lines.join("\n")
        );
        return Some(HoverResult { markdown });
    }

    // Check if it's a column name by finding the table for this query
    let table_name = catalog::find_table_for_query(text, token_offset)?;
    if schema.has_column(&table_name, token_text) {
        let columns = schema.columns_for_table(&table_name);
        let col = columns.iter().find(|c| c.name.eq_ignore_ascii_case(token_text))?;
        let markdown = format!("**{}**: {}\n\n(column in {})", col.name, col.column_type, table_name);
        return Some(HoverResult { markdown });
    }

    None
}


fn hover_for_token(kind: SyntaxKind, text: &str) -> Option<HoverResult> {
    match kind {
        SyntaxKind::Identifier => {
            if let Some(func) = catalog::find_function(text) {
                return Some(HoverResult {
                    markdown: format!("```kql\n{}\n```\n\n{}", func.signature, func.description),
                });
            }
            None
        }
        SyntaxKind::CountKw => {
            // count can be both a function and a tabular operator
            if let Some(func) = catalog::find_function(text) {
                return Some(HoverResult {
                    markdown: format!("```kql\n{}\n```\n\n{}", func.signature, func.description),
                });
            }
            if let Some(op) = catalog::find_tabular_operator(text) {
                return Some(HoverResult {
                    markdown: format!("**{}** (tabular operator)\n\n{}", text, op.description),
                });
            }
            None
        }
        SyntaxKind::WhereKw
        | SyntaxKind::ProjectKw
        | SyntaxKind::ExtendKw
        | SyntaxKind::SummarizeKw
        | SyntaxKind::TakeKw
        | SyntaxKind::LimitKw
        | SyntaxKind::SortKw
        | SyntaxKind::OrderKw
        | SyntaxKind::TopKw
        | SyntaxKind::DistinctKw
        | SyntaxKind::JoinKw
        | SyntaxKind::UnionKw => {
            if let Some(op) = catalog::find_tabular_operator(text) {
                return Some(HoverResult {
                    markdown: format!("**{}** (tabular operator)\n\n{}", text, op.description),
                });
            }
            None
        }
        SyntaxKind::ContainsKw
        | SyntaxKind::NotContainsKw
        | SyntaxKind::ContainsCsKw
        | SyntaxKind::HasKw
        | SyntaxKind::NotHasKw
        | SyntaxKind::HasCsKw
        | SyntaxKind::StartswithKw
        | SyntaxKind::EndswithKw
        | SyntaxKind::MatchesRegexKw
        | SyntaxKind::InKw
        | SyntaxKind::BetweenKw
        | SyntaxKind::AndKw
        | SyntaxKind::OrKw
        | SyntaxKind::NotKw => {
            if let Some(op) = catalog::find_string_operator(text) {
                return Some(HoverResult {
                    markdown: format!("**{}** (operator)\n\n{}", text, op.description),
                });
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_schema() -> SchemaStore {
        SchemaStore::new()
    }

    #[test]
    fn hover_count_function() {
        let result = hover_at("StormEvents | summarize count()", 24, &empty_schema());
        assert!(result.is_some());
        let hover = result.unwrap();
        assert!(hover.markdown.contains("count"));
        assert!(hover.markdown.contains("long"));
    }

    #[test]
    fn hover_where_keyword() {
        let result = hover_at("StormEvents | where X > 5", 14, &empty_schema());
        assert!(result.is_some());
        let hover = result.unwrap();
        assert!(hover.markdown.contains("where"));
        assert!(hover.markdown.contains("Filters"));
    }

    #[test]
    fn hover_unknown_identifier() {
        let result = hover_at("StormEvents | take 10", 3, &empty_schema());
        assert!(result.is_none());
    }

    #[test]
    fn hover_table_name_with_schema() {
        let mut schema = SchemaStore::new();
        schema.load(
            crate::schema::DatabaseSchema {
                database: "TestDB".to_string(),
                tables: vec![crate::schema::Table {
                    name: "StormEvents".to_string(),
                    columns: vec![
                        crate::schema::Column { name: "State".to_string(), column_type: "string".to_string() },
                        crate::schema::Column { name: "DamageProperty".to_string(), column_type: "long".to_string() },
                    ],
                }],
            },
            crate::schema::SchemaSource::Static,
        );
        let result = hover_at("StormEvents | take 10", 3, &schema);
        assert!(result.is_some());
        let hover = result.unwrap();
        assert!(hover.markdown.contains("StormEvents"), "Should mention table name");
        assert!(hover.markdown.contains("State"), "Should list columns");
        assert!(hover.markdown.contains("DamageProperty"), "Should list columns");
    }

    #[test]
    fn hover_column_name_with_schema() {
        let mut schema = SchemaStore::new();
        schema.load(
            crate::schema::DatabaseSchema {
                database: "TestDB".to_string(),
                tables: vec![crate::schema::Table {
                    name: "StormEvents".to_string(),
                    columns: vec![
                        crate::schema::Column { name: "State".to_string(), column_type: "string".to_string() },
                    ],
                }],
            },
            crate::schema::SchemaSource::Static,
        );
        let result = hover_at("StormEvents | where State == \"TX\"", 20, &schema);
        assert!(result.is_some());
        let hover = result.unwrap();
        assert!(hover.markdown.contains("State"), "Should mention column name");
        assert!(hover.markdown.contains("string"), "Should show column type");
    }
}
