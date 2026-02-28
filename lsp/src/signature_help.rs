//! Signature help for KQL built-in functions.

use crate::lexer;
use crate::syntax::SyntaxKind;

/// A parameter in a function signature.
pub struct ParamInfo {
    pub label: &'static str,
}

/// A function signature.
pub struct SignatureInfo {
    pub label: &'static str,
    pub documentation: &'static str,
    pub parameters: &'static [ParamInfo],
}

const SIGNATURES: &[SignatureInfo] = &[
    SignatureInfo {
        label: "count() -> long",
        documentation: "Returns the number of rows in the group.",
        parameters: &[],
    },
    SignatureInfo {
        label: "sum(expr) -> numeric",
        documentation: "Returns the sum of expr across the group.",
        parameters: &[ParamInfo { label: "expr" }],
    },
    SignatureInfo {
        label: "avg(expr) -> double",
        documentation: "Returns the average of expr across the group.",
        parameters: &[ParamInfo { label: "expr" }],
    },
    SignatureInfo {
        label: "min(expr) -> scalar",
        documentation: "Returns the minimum value of expr across the group.",
        parameters: &[ParamInfo { label: "expr" }],
    },
    SignatureInfo {
        label: "max(expr) -> scalar",
        documentation: "Returns the maximum value of expr across the group.",
        parameters: &[ParamInfo { label: "expr" }],
    },
    SignatureInfo {
        label: "dcount(expr) -> long",
        documentation: "Returns an estimate of the number of distinct values of expr.",
        parameters: &[ParamInfo { label: "expr" }],
    },
    SignatureInfo {
        label: "countif(predicate) -> long",
        documentation: "Returns a count of rows for which predicate evaluates to true.",
        parameters: &[ParamInfo { label: "predicate" }],
    },
    SignatureInfo {
        label: "sumif(expr, predicate) -> numeric",
        documentation: "Returns a sum of expr for which predicate evaluates to true.",
        parameters: &[ParamInfo { label: "expr" }, ParamInfo { label: "predicate" }],
    },
    SignatureInfo {
        label: "ago(timespan) -> datetime",
        documentation: "Subtracts the given timespan from the current UTC clock time.",
        parameters: &[ParamInfo { label: "timespan" }],
    },
    SignatureInfo {
        label: "now() -> datetime",
        documentation: "Returns the current UTC clock time.",
        parameters: &[],
    },
    SignatureInfo {
        label: "strcat(arg1, arg2, ...) -> string",
        documentation: "Concatenates between 1 and 64 arguments.",
        parameters: &[ParamInfo { label: "arg1" }, ParamInfo { label: "arg2" }],
    },
    SignatureInfo {
        label: "tostring(expr) -> string",
        documentation: "Converts the input to a string representation.",
        parameters: &[ParamInfo { label: "expr" }],
    },
    SignatureInfo {
        label: "toint(expr) -> int",
        documentation: "Converts the input to an integer value.",
        parameters: &[ParamInfo { label: "expr" }],
    },
    SignatureInfo {
        label: "tolong(expr) -> long",
        documentation: "Converts the input to a long value.",
        parameters: &[ParamInfo { label: "expr" }],
    },
    SignatureInfo {
        label: "strlen(source) -> long",
        documentation: "Returns the length, in characters, of the input string.",
        parameters: &[ParamInfo { label: "source" }],
    },
    SignatureInfo {
        label: "substring(source, startIndex, length) -> string",
        documentation: "Extracts a substring from the source string.",
        parameters: &[
            ParamInfo { label: "source" },
            ParamInfo { label: "startIndex" },
            ParamInfo { label: "length" },
        ],
    },
    SignatureInfo {
        label: "iif(condition, ifTrue, ifFalse) -> scalar",
        documentation: "Returns ifTrue or ifFalse depending on the condition.",
        parameters: &[
            ParamInfo { label: "condition" },
            ParamInfo { label: "ifTrue" },
            ParamInfo { label: "ifFalse" },
        ],
    },
    SignatureInfo {
        label: "iff(condition, ifTrue, ifFalse) -> scalar",
        documentation: "Returns ifTrue or ifFalse depending on the condition.",
        parameters: &[
            ParamInfo { label: "condition" },
            ParamInfo { label: "ifTrue" },
            ParamInfo { label: "ifFalse" },
        ],
    },
    SignatureInfo {
        label: "bin(value, roundTo) -> scalar",
        documentation: "Rounds values down to an integer multiple of a given bin size.",
        parameters: &[ParamInfo { label: "value" }, ParamInfo { label: "roundTo" }],
    },
    SignatureInfo {
        label: "extract(regex, captureGroup, source) -> string",
        documentation: "Get a match for a regular expression from a text string.",
        parameters: &[
            ParamInfo { label: "regex" },
            ParamInfo { label: "captureGroup" },
            ParamInfo { label: "source" },
        ],
    },
    SignatureInfo {
        label: "parse_json(json) -> dynamic",
        documentation: "Interprets a string as a JSON value and returns the value as dynamic.",
        parameters: &[ParamInfo { label: "json" }],
    },
    SignatureInfo {
        label: "todatetime(expr) -> datetime",
        documentation: "Converts the input to a datetime value.",
        parameters: &[ParamInfo { label: "expr" }],
    },
    SignatureInfo {
        label: "totimespan(expr) -> timespan",
        documentation: "Converts the input to a timespan value.",
        parameters: &[ParamInfo { label: "expr" }],
    },
    SignatureInfo {
        label: "todouble(expr) -> double",
        documentation: "Converts the input to a double value.",
        parameters: &[ParamInfo { label: "expr" }],
    },
    SignatureInfo {
        label: "replace_string(text, lookup, rewrite) -> string",
        documentation: "Replaces all string matches with another string.",
        parameters: &[
            ParamInfo { label: "text" },
            ParamInfo { label: "lookup" },
            ParamInfo { label: "rewrite" },
        ],
    },
    SignatureInfo {
        label: "format_datetime(datetime, format) -> string",
        documentation: "Formats a datetime according to the provided format string.",
        parameters: &[ParamInfo { label: "datetime" }, ParamInfo { label: "format" }],
    },
    SignatureInfo {
        label: "datetime_diff(period, datetime1, datetime2) -> long",
        documentation: "Returns the difference between two datetimes.",
        parameters: &[
            ParamInfo { label: "period" },
            ParamInfo { label: "datetime1" },
            ParamInfo { label: "datetime2" },
        ],
    },
];

/// Result of signature help.
pub struct SignatureHelpResult {
    pub signature: &'static SignatureInfo,
    pub active_parameter: u32,
}

/// Get signature help at the given byte offset.
pub fn signature_help_at(text: &str, offset: usize) -> Option<SignatureHelpResult> {
    let tokens = lexer::lex(text);
    let prefix = &text[..offset.min(text.len())];

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
                        if prev_token.kind != SyntaxKind::Whitespace
                            && prev_token.kind != SyntaxKind::Newline
                            && prev_token.kind != SyntaxKind::Comment
                        {
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

    // Look up the function name in signatures
    // Extract just the function name from the signature label
    let sig = SIGNATURES.iter().find(|s| {
        s.label.starts_with(name)
            && s.label.as_bytes().get(name.len()) == Some(&b'(')
    })?;

    Some(SignatureHelpResult {
        signature: sig,
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
        assert!(help.signature.label.contains("ago"), "Should be ago signature");
        assert_eq!(help.active_parameter, 0, "First parameter should be active");
    }

    #[test]
    fn signature_help_strcat_second_param() {
        let result = signature_help_at("strcat(\"a\", ", 11);
        assert!(result.is_some(), "Should provide signature help for strcat after comma");
        let help = result.unwrap();
        assert!(help.signature.label.contains("strcat"), "Should be strcat signature");
        assert_eq!(help.active_parameter, 1, "Second parameter should be active");
    }

    #[test]
    fn no_signature_help_outside_parens() {
        let result = signature_help_at("StormEvents | take 10", 5);
        assert!(result.is_none(), "Should not provide signature help outside function call");
    }
}
