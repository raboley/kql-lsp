//! Folding ranges for KQL.
//! Multi-line queries and let blocks can be collapsed.

use crate::parser;
use crate::syntax::SyntaxKind;
use rowan::NodeOrToken;

/// A folding range (line-based).
#[derive(Debug)]
pub struct FoldingRange {
    pub start_line: u32,
    pub end_line: u32,
}

/// Compute folding ranges for KQL source text.
pub fn folding_ranges(text: &str) -> Vec<FoldingRange> {
    let parse_result = parser::parse(text);
    let root = parse_result.syntax();
    let rope = ropey::Rope::from_str(text);

    let mut ranges = Vec::new();

    // Walk top-level children of SourceFile
    for child in root.children() {
        let kind = child.kind();
        if kind == SyntaxKind::QueryStatement || kind == SyntaxKind::LetStatement {
            let range = child.text_range();
            let start_byte: usize = range.start().into();
            let end_byte: usize = range.end().into();

            // Only create folding range if it spans multiple lines
            let start_char = start_byte.min(rope.len_chars());
            let end_char = end_byte.min(rope.len_chars());
            let start_line = rope.char_to_line(start_char) as u32;
            let end_line = rope.char_to_line(end_char.max(1) - 1) as u32;

            if end_line > start_line {
                ranges.push(FoldingRange { start_line, end_line });
            }
        }
    }

    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_line_query_foldable() {
        let text = "StormEvents\n| where X > 5\n| take 10";
        let ranges = folding_ranges(text);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start_line, 0);
        assert_eq!(ranges[0].end_line, 2);
    }

    #[test]
    fn single_line_query_not_foldable() {
        let text = "StormEvents | take 10";
        let ranges = folding_ranges(text);
        assert!(ranges.is_empty());
    }
}
