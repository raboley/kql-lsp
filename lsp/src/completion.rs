//! Completion support for KQL.

use crate::catalog;
use crate::lexer;
use crate::syntax::SyntaxKind;

/// A completion item to return to the client.
pub struct CompletionItem {
    pub label: String,
    pub kind: i32, // LSP CompletionItemKind
    pub detail: Option<String>,
}

/// LSP CompletionItemKind values.
const COMPLETION_KIND_KEYWORD: i32 = 14;

/// Compute completions at the given byte offset in the source text.
pub fn complete_at(text: &str, offset: usize) -> Vec<CompletionItem> {
    // Determine context: are we after a pipe?
    if is_after_pipe(text, offset) {
        return tabular_operator_completions();
    }

    Vec::new()
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

    #[test]
    fn complete_after_pipe() {
        let items = complete_at("StormEvents | ", 14);
        assert!(items.len() >= 10, "Should have tabular operators");
        let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"where"));
        assert!(labels.contains(&"project"));
        assert!(labels.contains(&"summarize"));
        assert!(labels.contains(&"take"));
    }

    #[test]
    fn complete_after_pipe_no_space() {
        let items = complete_at("StormEvents |", 13);
        assert!(items.len() >= 10, "Should have tabular operators even without trailing space");
    }

    #[test]
    fn complete_after_pipe_partial_keyword() {
        let items = complete_at("StormEvents | wh", 16);
        assert!(items.len() >= 10, "Should return all tabular operators for partial input");
    }

    #[test]
    fn no_completion_at_start() {
        let items = complete_at("Storm", 5);
        assert!(items.is_empty(), "Should not complete at start of query");
    }
}
