//! Document formatting for KQL.
//! Rules:
//! - Pipe operators on new lines
//! - Spaces around binary operators
//! - Single space after keywords before their argument

use crate::lexer;
use crate::syntax::SyntaxKind;

/// A text edit for formatting.
#[derive(Debug, Clone)]
pub struct FormatEdit {
    pub offset: usize,
    pub len: usize,
    pub new_text: String,
}

/// Format KQL source text and return a list of edits.
pub fn format(text: &str) -> Vec<FormatEdit> {
    let tokens = lexer::lex(text);
    let mut edits = Vec::new();
    let mut offset = 0;

    for i in 0..tokens.len() {
        let token = &tokens[i];
        match token.kind {
            SyntaxKind::Pipe => {
                // Rule: pipe should be preceded by a newline (not inline)
                if i > 0 {
                    let prev = &tokens[i - 1];
                    let prev_offset = offset - prev.len;

                    if prev.kind == SyntaxKind::Whitespace {
                        let has_newline_before = if i >= 2 {
                            tokens[i - 2].kind == SyntaxKind::Newline
                        } else {
                            false
                        };

                        if !has_newline_before {
                            edits.push(FormatEdit {
                                offset: prev_offset,
                                len: prev.len,
                                new_text: "\n".to_string(),
                            });
                        }
                    } else if prev.kind != SyntaxKind::Newline {
                        edits.push(FormatEdit {
                            offset,
                            len: 0,
                            new_text: "\n".to_string(),
                        });
                    }
                }

                // Rule: pipe should be followed by a space
                if i + 1 < tokens.len() {
                    let next = &tokens[i + 1];
                    if next.kind != SyntaxKind::Whitespace && next.kind != SyntaxKind::Newline {
                        edits.push(FormatEdit {
                            offset: offset + token.len,
                            len: 0,
                            new_text: " ".to_string(),
                        });
                    }
                }
            }
            // Binary operators that should have spaces around them
            SyntaxKind::GreaterThan
            | SyntaxKind::LessThan
            | SyntaxKind::GreaterEqual
            | SyntaxKind::LessEqual
            | SyntaxKind::EqualEqual
            | SyntaxKind::NotEqual
            | SyntaxKind::Equals
            | SyntaxKind::Plus
            | SyntaxKind::Minus
            | SyntaxKind::Star
            | SyntaxKind::Slash
            | SyntaxKind::Percent => {
                // Check for space before operator
                if i > 0 {
                    let prev = &tokens[i - 1];
                    if prev.kind != SyntaxKind::Whitespace && prev.kind != SyntaxKind::Newline {
                        edits.push(FormatEdit {
                            offset,
                            len: 0,
                            new_text: " ".to_string(),
                        });
                    }
                }
                // Check for space after operator
                if i + 1 < tokens.len() {
                    let next = &tokens[i + 1];
                    if next.kind != SyntaxKind::Whitespace && next.kind != SyntaxKind::Newline {
                        edits.push(FormatEdit {
                            offset: offset + token.len,
                            len: 0,
                            new_text: " ".to_string(),
                        });
                    }
                }
            }
            _ => {}
        }

        offset += token.len;
    }

    // Deduplicate edits at the same offset (take the last one)
    edits.sort_by_key(|e| e.offset);
    edits.dedup_by_key(|e| e.offset);

    edits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_pipes_on_new_lines() {
        let text = "StormEvents|where X > 5|take 10";
        let edits = format(text);
        assert!(!edits.is_empty(), "Should have edits");

        // Apply edits to verify result
        let result = apply_edits(text, &edits);
        assert!(result.contains("\n|"), "Pipes should be on new lines: {}", result);
        assert!(result.contains("| where"), "Should preserve pipe + space + keyword: {}", result);
    }

    #[test]
    fn format_spaces_around_operators() {
        let text = "StormEvents | where X>5";
        let edits = format(text);
        let result = apply_edits(text, &edits);
        assert!(result.contains("X > 5"), "Should add spaces around >: {}", result);
    }

    #[test]
    fn no_edits_for_well_formatted() {
        let text = "StormEvents\n| where X > 5\n| take 10";
        let edits = format(text);
        assert!(edits.is_empty(), "Well-formatted text should have no edits: {:?}", edits);
    }

    fn apply_edits(text: &str, edits: &[FormatEdit]) -> String {
        let mut result = text.to_string();
        // Apply edits in reverse order to preserve offsets
        for edit in edits.iter().rev() {
            result.replace_range(edit.offset..edit.offset + edit.len, &edit.new_text);
        }
        result
    }
}
