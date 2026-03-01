//! Code actions for KQL (quick fixes, refactorings).

use crate::lexer;
use crate::syntax::SyntaxKind;

/// A code action result.
#[derive(Debug)]
pub struct CodeAction {
    pub title: String,
    pub edit_offset: usize,
    pub edit_text: String,
}

/// Find code actions applicable at the given byte offset range.
pub fn code_actions_at(text: &str, start_offset: usize, end_offset: usize) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Check for missing semicolons on let statements
    if let Some(action) = check_missing_semicolon(text, start_offset, end_offset) {
        actions.push(action);
    }

    actions
}

/// Check if a let statement in the given range is missing a semicolon.
fn check_missing_semicolon(text: &str, start_offset: usize, end_offset: usize) -> Option<CodeAction> {
    let tokens = lexer::lex(text);

    // Walk tokens to find let statements
    let mut offset = 0;
    let mut i = 0;

    while i < tokens.len() {
        let token = &tokens[i];

        if token.kind == SyntaxKind::LetKw {
            let let_start = offset;

            // Find the end of this let statement: scan forward past the value expression
            // until we hit a newline, another 'let', or EOF
            let mut let_end = offset + token.len;
            let mut j = i + 1;
            let mut has_semicolon = false;

            while j < tokens.len() {
                let t = &tokens[j];
                if t.kind == SyntaxKind::Semicolon {
                    has_semicolon = true;
                    let_end += t.len;
                    break;
                }
                if t.kind == SyntaxKind::Newline {
                    // End of let statement (no semicolon before newline)
                    break;
                }
                let_end += t.len;
                j += 1;
            }

            // Check if this let statement overlaps with the requested range
            if let_start < end_offset && let_end > start_offset && !has_semicolon {
                return Some(CodeAction {
                    title: "Add missing semicolon".to_string(),
                    edit_offset: let_end,
                    edit_text: ";".to_string(),
                });
            }
        }

        offset += token.len;
        i += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_semicolon_detected() {
        let text = "let x = 10\nStormEvents | take 5";
        let actions = code_actions_at(text, 0, 10);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Add missing semicolon");
        assert_eq!(actions[0].edit_offset, 10); // after "let x = 10"
        assert_eq!(actions[0].edit_text, ";");
    }

    #[test]
    fn no_action_when_semicolon_present() {
        let text = "let x = 10;\nStormEvents | take 5";
        let actions = code_actions_at(text, 0, 11);
        assert!(actions.is_empty());
    }

    #[test]
    fn no_action_on_non_let_range() {
        let text = "StormEvents | take 5";
        let actions = code_actions_at(text, 0, 20);
        assert!(actions.is_empty());
    }
}
