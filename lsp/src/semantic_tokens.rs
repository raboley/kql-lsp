//! Semantic tokens for KQL syntax highlighting.

use crate::lexer;
use crate::syntax::SyntaxKind;

/// Token type indices matching the legend order.
pub const TOKEN_TYPE_KEYWORD: u32 = 0;
pub const TOKEN_TYPE_NUMBER: u32 = 1;
pub const TOKEN_TYPE_STRING: u32 = 2;
pub const TOKEN_TYPE_COMMENT: u32 = 3;
pub const TOKEN_TYPE_OPERATOR: u32 = 4;
pub const TOKEN_TYPE_PROPERTY: u32 = 5;
pub const TOKEN_TYPE_VARIABLE: u32 = 6;

/// The token type legend (order must match the constants above).
pub const TOKEN_TYPES: &[&str] = &[
    "keyword",    // 0
    "number",     // 1
    "string",     // 2
    "comment",    // 3
    "operator",   // 4
    "property",   // 5
    "variable",   // 6
];

/// Map a SyntaxKind to a semantic token type index, or None if it shouldn't be highlighted.
fn token_type_for(kind: SyntaxKind) -> Option<u32> {
    match kind {
        SyntaxKind::WhereKw
        | SyntaxKind::TakeKw
        | SyntaxKind::LimitKw
        | SyntaxKind::LetKw
        | SyntaxKind::ByKw
        | SyntaxKind::ProjectKw
        | SyntaxKind::ExtendKw
        | SyntaxKind::SummarizeKw
        | SyntaxKind::SortKw
        | SyntaxKind::OrderKw
        | SyntaxKind::TopKw
        | SyntaxKind::CountKw
        | SyntaxKind::DistinctKw
        | SyntaxKind::JoinKw
        | SyntaxKind::UnionKw => Some(TOKEN_TYPE_KEYWORD),

        SyntaxKind::IntLiteral => Some(TOKEN_TYPE_NUMBER),

        SyntaxKind::StringLiteral => Some(TOKEN_TYPE_STRING),

        SyntaxKind::Comment => Some(TOKEN_TYPE_COMMENT),

        SyntaxKind::Pipe
        | SyntaxKind::EqualEqual
        | SyntaxKind::NotEqual
        | SyntaxKind::GreaterThan
        | SyntaxKind::LessThan
        | SyntaxKind::GreaterEqual
        | SyntaxKind::LessEqual
        | SyntaxKind::Plus
        | SyntaxKind::Minus
        | SyntaxKind::Star
        | SyntaxKind::Slash
        | SyntaxKind::Percent => Some(TOKEN_TYPE_OPERATOR),

        SyntaxKind::Identifier => Some(TOKEN_TYPE_PROPERTY),

        _ => None,
    }
}

/// Compute semantic tokens data for the given source text.
/// Returns the encoded data array (groups of 5 integers per token).
pub fn compute_semantic_tokens(text: &str) -> Vec<u32> {
    let tokens = lexer::lex(text);
    let mut data = Vec::new();
    let mut prev_line: u32 = 0;
    let mut prev_col: u32 = 0;
    let mut offset: usize = 0;

    // Pre-compute line starts for offset-to-line/col conversion
    let line_starts = compute_line_starts(text);

    for token in &tokens {
        if let Some(token_type) = token_type_for(token.kind) {
            let (line, col) = offset_to_line_col(&line_starts, offset);

            let delta_line = line - prev_line;
            let delta_start = if delta_line == 0 {
                col - prev_col
            } else {
                col
            };

            data.push(delta_line);
            data.push(delta_start);
            data.push(token.len as u32);
            data.push(token_type);
            data.push(0); // no modifiers

            prev_line = line;
            prev_col = col;
        }
        offset += token.len;
    }

    data
}

fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn offset_to_line_col(line_starts: &[usize], offset: usize) -> (u32, u32) {
    let line = line_starts.partition_point(|&start| start <= offset) - 1;
    let col = offset - line_starts[line];
    (line as u32, col as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_tokens_simple_query() {
        let data = compute_semantic_tokens("StormEvents | take 10");
        // Should have tokens for: StormEvents(property), |(operator), take(keyword), 10(number)
        assert_eq!(data.len() % 5, 0, "Data should be groups of 5");
        let token_count = data.len() / 5;
        assert_eq!(token_count, 4, "Expected 4 tokens, got {}", token_count);
    }

    #[test]
    fn semantic_tokens_keyword_type() {
        let data = compute_semantic_tokens("take");
        // Single keyword token
        assert_eq!(data.len(), 5);
        assert_eq!(data[3], TOKEN_TYPE_KEYWORD); // token type
    }

    #[test]
    fn semantic_tokens_multiline() {
        let data = compute_semantic_tokens("StormEvents\n| take 10");
        assert!(data.len() >= 20, "Should have at least 4 tokens");
        // Second token (|) should have deltaLine = 1
        assert_eq!(data[5], 1, "Pipe should be on line 1 (delta_line=1)");
    }
}
