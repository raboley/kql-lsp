//! Signature help for KQL built-in functions.

use crate::catalog;
use crate::lexer;
use crate::syntax::SyntaxKind;

/// Result of signature help.
pub struct SignatureHelpResult {
    pub function: &'static catalog::BuiltinFunction,
    pub active_parameter: u32,
}

/// Get signature help at the given byte offset.
pub fn signature_help_at(text: &str, offset: usize) -> Option<SignatureHelpResult> {
    let tokens = lexer::lex(text);
    // Walk backwards from the cursor position to find if we're inside function parens
    // Count unmatched open parens and commas
    let mut paren_depth: i32 = 0;
    let mut comma_count: u32 = 0;
    let mut func_name: Option<&str> = None;

    // We need to walk through tokens up to the offset position
    let mut token_offset = 0;
    let mut tokens_before_cursor = Vec::new();

    for token in &tokens {
        let token_end = token_offset + token.len;
        if token_offset >= offset {
            break;
        }
        tokens_before_cursor.push((token_offset, token));
        token_offset = token_end;
    }

    // Walk backwards through tokens to find enclosing function call
    for (tok_offset, token) in tokens_before_cursor.iter().rev() {
        match token.kind {
            SyntaxKind::RParen => {
                paren_depth += 1;
            }
            SyntaxKind::LParen => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                } else {
                    // We found our unmatched open paren
                    // Look for the function name before it (skip whitespace)
                    let paren_idx = tokens_before_cursor
                        .iter()
                        .position(|(o, _)| o == tok_offset)
                        .unwrap();

                    // Walk backwards past trivia to find the identifier
                    let mut j = paren_idx;
                    while j > 0 {
                        j -= 1;
                        let (prev_offset, prev_token) = &tokens_before_cursor[j];
                        if !catalog::is_trivia(prev_token.kind) {
                            if prev_token.kind == SyntaxKind::Identifier
                                || prev_token.kind == SyntaxKind::CountKw
                            {
                                func_name =
                                    Some(&text[*prev_offset..*prev_offset + prev_token.len]);
                            }
                            break;
                        }
                    }
                    break;
                }
            }
            SyntaxKind::Comma => {
                if paren_depth == 0 {
                    comma_count += 1;
                }
            }
            _ => {}
        }
    }

    let name = func_name?;
    let func = catalog::find_function(name)?;

    Some(SignatureHelpResult {
        function: func,
        active_parameter: comma_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_help_ago() {
        let result = signature_help_at("StormEvents | where StartTime > ago(", 36);
        assert!(result.is_some(), "Should provide signature help for ago(");
        let help = result.unwrap();
        assert!(help.function.signature.contains("ago"), "Should be ago signature");
        assert_eq!(help.active_parameter, 0, "First parameter should be active");
    }

    #[test]
    fn signature_help_strcat_second_param() {
        let result = signature_help_at("strcat(\"a\", ", 11);
        assert!(result.is_some(), "Should provide signature help for strcat after comma");
        let help = result.unwrap();
        assert!(help.function.signature.contains("strcat"), "Should be strcat signature");
        assert_eq!(help.active_parameter, 1, "Second parameter should be active");
    }

    #[test]
    fn no_signature_help_outside_parens() {
        let result = signature_help_at("StormEvents | take 10", 5);
        assert!(result.is_none(), "Should not provide signature help outside function call");
    }
}
