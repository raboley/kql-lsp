//! Find references for KQL let-bound variables.

use crate::catalog;
use crate::lexer;
use crate::syntax::SyntaxKind;

/// A reference location (byte offsets into the source text).
pub struct ReferenceLocation {
    pub offset: usize,
    pub len: usize,
}

/// Find all references to the identifier at the given byte offset.
/// Returns locations of the declaration and all usages.
pub fn find_references(text: &str, offset: usize) -> Vec<ReferenceLocation> {
    let tokens = lexer::lex(text);

    // Find the token at the given offset
    let mut token_offset = 0;
    let mut target_text: Option<&str> = None;

    for token in &tokens {
        let token_end = token_offset + token.len;
        if offset >= token_offset && offset < token_end {
            if token.kind == SyntaxKind::Identifier {
                target_text = Some(&text[token_offset..token_end]);
            }
            break;
        }
        token_offset = token_end;
    }

    let target_name = match target_text {
        Some(name) => name,
        None => return Vec::new(),
    };

    // Check if this name is actually let-bound
    let is_let_bound = is_let_defined(text, &tokens, target_name);
    if !is_let_bound {
        return Vec::new();
    }

    // Find all occurrences of this identifier name
    let mut refs = Vec::new();
    token_offset = 0;
    for token in &tokens {
        if token.kind == SyntaxKind::Identifier {
            let token_text = &text[token_offset..token_offset + token.len];
            if token_text == target_name {
                refs.push(ReferenceLocation {
                    offset: token_offset,
                    len: token.len,
                });
            }
        }
        token_offset += token.len;
    }

    refs
}

/// Check if a name is defined by a let statement.
fn is_let_defined(text: &str, tokens: &[lexer::Token], name: &str) -> bool {
    let mut token_offset = 0;
    for (i, token) in tokens.iter().enumerate() {
        if token.kind == SyntaxKind::LetKw {
            // Skip to next non-trivia token
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
                let id_text = &text[name_offset..name_offset + tokens[j].len];
                if id_text == name {
                    return true;
                }
            }
        }
        token_offset += token.len;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_all_references() {
        let text = "let x = 10;\nlet y = x + 1;\nStormEvents | where Col > x";
        // Find "x" at the end
        let offset = text.rfind('x').unwrap();
        let refs = find_references(text, offset);
        assert_eq!(refs.len(), 3, "Should find 3 references: declaration + 2 usages");
    }

    #[test]
    fn no_references_for_non_let() {
        let text = "StormEvents | where Damage > 100";
        let offset = text.find("Damage").unwrap();
        let refs = find_references(text, offset);
        assert_eq!(refs.len(), 0, "Should find no references for non-let identifier");
    }

    #[test]
    fn find_references_from_declaration() {
        let text = "let threshold = 100;\nStormEvents | where D > threshold";
        let offset = 4; // "threshold" in the let statement
        let refs = find_references(text, offset);
        assert_eq!(refs.len(), 2, "Should find 2 references from declaration site");
    }
}
