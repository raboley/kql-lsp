//! Go-to-definition support for KQL let-bound variables.

use crate::catalog;
use crate::lexer;
use crate::syntax::SyntaxKind;

/// A definition location (byte offsets into the source text).
#[allow(dead_code)]
pub struct DefinitionResult {
    /// Start of the let statement.
    pub range_start: usize,
    /// End of the let statement.
    pub range_end: usize,
    /// Start of the name identifier in the let statement.
    pub name_start: usize,
    /// End of the name identifier.
    pub name_end: usize,
}

/// Find the definition of the identifier at the given byte offset.
/// Currently supports let-bound variable references.
pub fn find_definition(text: &str, offset: usize) -> Option<DefinitionResult> {
    let tokens = lexer::lex(text);

    // Find the token at the given offset and its position
    let mut token_offset = 0;
    let mut cursor_token_start = 0;
    let mut target_text: Option<&str> = None;

    for token in &tokens {
        let token_end = token_offset + token.len;
        if offset >= token_offset && offset < token_end {
            if token.kind == SyntaxKind::Identifier {
                target_text = Some(&text[token_offset..token_end]);
                cursor_token_start = token_offset;
            }
            break;
        }
        token_offset = token_end;
    }

    let target_name = target_text?;

    // Search for a let statement that defines this name
    // Pattern: let <whitespace> <identifier> ...
    token_offset = 0;
    for (i, token) in tokens.iter().enumerate() {
        if token.kind == SyntaxKind::LetKw {
            let let_offset = token_offset;

            // Skip to the next non-trivia token (should be the identifier name)
            let mut j = i + 1;
            let mut name_offset = token_offset + token.len;

            while j < tokens.len() {
                let t = &tokens[j];
                if !catalog::is_trivia(t.kind) {
                    break;
                }
                name_offset += t.len;
                j += 1;
            }

            if j < tokens.len() && tokens[j].kind == SyntaxKind::Identifier {
                let name_token = &tokens[j];
                let name = &text[name_offset..name_offset + name_token.len];

                if name == target_name && cursor_token_start != name_offset {
                    // Find the end of the let statement
                    let mut end_offset = name_offset + name_token.len;
                    let mut k = j + 1;
                    while k < tokens.len() {
                        end_offset += tokens[k].len;
                        if tokens[k].kind == SyntaxKind::Semicolon
                            || tokens[k].kind == SyntaxKind::Newline
                        {
                            break;
                        }
                        k += 1;
                    }

                    return Some(DefinitionResult {
                        range_start: let_offset,
                        range_end: end_offset,
                        name_start: name_offset,
                        name_end: name_offset + name_token.len,
                    });
                }
            }
        }
        token_offset += token.len;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_let_definition() {
        let text = "let threshold = 100;\nStormEvents | where Damage > threshold";
        let offset = text.rfind("threshold").unwrap();
        let result = find_definition(text, offset);
        assert!(result.is_some(), "Should find definition");
        let def = result.unwrap();
        assert_eq!(def.range_start, 0, "Definition should start at offset 0");
        assert_eq!(def.name_start, 4, "Name should start at offset 4");
    }

    #[test]
    fn no_definition_for_unknown() {
        let text = "StormEvents | where Damage > 100";
        let offset = text.find("Damage").unwrap();
        let result = find_definition(text, offset);
        assert!(result.is_none(), "Should not find definition for unknown identifier");
    }

    #[test]
    fn no_definition_on_definition_itself() {
        let text = "let threshold = 100;\nStormEvents | where Damage > threshold";
        let offset = 4; // "threshold" in the let statement
        let result = find_definition(text, offset);
        assert!(result.is_none(), "Should not jump to self");
    }
}
