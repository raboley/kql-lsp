//! Hover documentation for KQL built-in functions and operators.

use crate::catalog;
use crate::lexer;
use crate::syntax::SyntaxKind;

/// Hover result.
pub struct HoverResult {
    pub markdown: String,
}

/// Get hover documentation for the token at the given byte offset.
pub fn hover_at(text: &str, offset: usize) -> Option<HoverResult> {
    let tokens = lexer::lex(text);
    let mut token_offset = 0;

    for token in &tokens {
        let token_end = token_offset + token.len;

        if offset >= token_offset && offset < token_end {
            let token_text = &text[token_offset..token_end];
            return hover_for_token(token.kind, token_text);
        }

        token_offset = token_end;
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

    #[test]
    fn hover_count_function() {
        let result = hover_at("StormEvents | summarize count()", 24);
        assert!(result.is_some());
        let hover = result.unwrap();
        assert!(hover.markdown.contains("count"));
        assert!(hover.markdown.contains("long"));
    }

    #[test]
    fn hover_where_keyword() {
        let result = hover_at("StormEvents | where X > 5", 14);
        assert!(result.is_some());
        let hover = result.unwrap();
        assert!(hover.markdown.contains("where"));
        assert!(hover.markdown.contains("Filters"));
    }

    #[test]
    fn hover_unknown_identifier() {
        let result = hover_at("StormEvents | take 10", 3);
        assert!(result.is_none());
    }
}
