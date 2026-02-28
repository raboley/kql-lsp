//! KQL lexer - tokenizes source text into SyntaxKind tokens.

use crate::syntax::SyntaxKind;

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: SyntaxKind,
    pub len: usize,
}

pub fn lex(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();

    while let Some(&(pos, ch)) = chars.peek() {
        let token = match ch {
            // Whitespace (not newlines)
            ' ' | '\t' | '\r' => {
                let start = pos;
                while let Some(&(_, c)) = chars.peek() {
                    if c == ' ' || c == '\t' || c == '\r' {
                        chars.next();
                    } else {
                        break;
                    }
                }
                let end = chars.peek().map_or(input.len(), |&(i, _)| i);
                Token { kind: SyntaxKind::Whitespace, len: end - start }
            }

            '\n' => {
                chars.next();
                Token { kind: SyntaxKind::Newline, len: 1 }
            }

            '|' => {
                chars.next();
                Token { kind: SyntaxKind::Pipe, len: 1 }
            }

            '(' => {
                chars.next();
                Token { kind: SyntaxKind::LParen, len: 1 }
            }

            ')' => {
                chars.next();
                Token { kind: SyntaxKind::RParen, len: 1 }
            }

            ',' => {
                chars.next();
                Token { kind: SyntaxKind::Comma, len: 1 }
            }

            '.' => {
                chars.next();
                Token { kind: SyntaxKind::Dot, len: 1 }
            }

            ';' => {
                chars.next();
                Token { kind: SyntaxKind::Semicolon, len: 1 }
            }

            '+' => {
                chars.next();
                Token { kind: SyntaxKind::Plus, len: 1 }
            }

            '-' => {
                chars.next();
                Token { kind: SyntaxKind::Minus, len: 1 }
            }

            '*' => {
                chars.next();
                Token { kind: SyntaxKind::Star, len: 1 }
            }

            '%' => {
                chars.next();
                Token { kind: SyntaxKind::Percent, len: 1 }
            }

            '=' => {
                chars.next();
                if chars.peek().map_or(false, |&(_, c)| c == '=') {
                    chars.next();
                    Token { kind: SyntaxKind::EqualEqual, len: 2 }
                } else {
                    Token { kind: SyntaxKind::Equals, len: 1 }
                }
            }

            '>' => {
                chars.next();
                if chars.peek().map_or(false, |&(_, c)| c == '=') {
                    chars.next();
                    Token { kind: SyntaxKind::GreaterEqual, len: 2 }
                } else {
                    Token { kind: SyntaxKind::GreaterThan, len: 1 }
                }
            }

            '<' => {
                chars.next();
                if chars.peek().map_or(false, |&(_, c)| c == '=') {
                    chars.next();
                    Token { kind: SyntaxKind::LessEqual, len: 2 }
                } else {
                    Token { kind: SyntaxKind::LessThan, len: 1 }
                }
            }

            '!' => {
                chars.next();
                if chars.peek().map_or(false, |&(_, c)| c == '=') {
                    chars.next();
                    Token { kind: SyntaxKind::NotEqual, len: 2 }
                } else {
                    Token { kind: SyntaxKind::Error, len: 1 }
                }
            }

            '/' => {
                chars.next();
                if chars.peek().map_or(false, |&(_, c)| c == '/') {
                    // Line comment
                    let start = pos;
                    while let Some(&(_, c)) = chars.peek() {
                        if c == '\n' {
                            break;
                        }
                        chars.next();
                    }
                    let end = chars.peek().map_or(input.len(), |&(i, _)| i);
                    Token { kind: SyntaxKind::Comment, len: end - start }
                } else {
                    Token { kind: SyntaxKind::Slash, len: 1 }
                }
            }

            '\'' | '"' => {
                let start = pos;
                let quote = ch;
                chars.next();
                while let Some(&(_, c)) = chars.peek() {
                    chars.next();
                    if c == quote {
                        break;
                    }
                    if c == '\\' {
                        chars.next(); // skip escaped char
                    }
                }
                let end = chars.peek().map_or(input.len(), |&(i, _)| i);
                Token { kind: SyntaxKind::StringLiteral, len: end - start }
            }

            '0'..='9' => {
                let start = pos;
                while let Some(&(_, c)) = chars.peek() {
                    if c.is_ascii_digit() {
                        chars.next();
                    } else {
                        break;
                    }
                }
                let end = chars.peek().map_or(input.len(), |&(i, _)| i);
                // Check for timespan suffix: d, h, m, s, ms, us, tick
                let suffix_start = end;
                let text_after = &input[suffix_start..];
                let timespan_suffix = if text_after.starts_with("tick") {
                    Some(4)
                } else if text_after.starts_with("ms") || text_after.starts_with("us") {
                    Some(2)
                } else if text_after.starts_with('d')
                    || text_after.starts_with('h')
                    || text_after.starts_with('m')
                    || text_after.starts_with('s')
                {
                    // Make sure it's not the start of an identifier (e.g. "10days")
                    let next_after = text_after.get(1..2).and_then(|s| s.chars().next());
                    if next_after.map_or(true, |c| !c.is_alphanumeric() && c != '_') {
                        Some(1)
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(suffix_len) = timespan_suffix {
                    // Consume the suffix characters
                    for _ in 0..suffix_len {
                        chars.next();
                    }
                    let total_end = chars.peek().map_or(input.len(), |&(i, _)| i);
                    Token { kind: SyntaxKind::TimespanLiteral, len: total_end - start }
                } else {
                    Token { kind: SyntaxKind::IntLiteral, len: end - start }
                }
            }

            c if c.is_alphabetic() || c == '_' => {
                let start = pos;
                while let Some(&(_, c)) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        chars.next();
                    } else {
                        break;
                    }
                }
                let end = chars.peek().map_or(input.len(), |&(i, _)| i);
                let text = &input[start..end];
                let kind = match text {
                    "where" => SyntaxKind::WhereKw,
                    "take" => SyntaxKind::TakeKw,
                    "limit" => SyntaxKind::LimitKw,
                    "let" => SyntaxKind::LetKw,
                    "by" => SyntaxKind::ByKw,
                    "project" => SyntaxKind::ProjectKw,
                    "extend" => SyntaxKind::ExtendKw,
                    "summarize" => SyntaxKind::SummarizeKw,
                    "sort" => SyntaxKind::SortKw,
                    "order" => SyntaxKind::OrderKw,
                    "top" => SyntaxKind::TopKw,
                    "count" => SyntaxKind::CountKw,
                    "distinct" => SyntaxKind::DistinctKw,
                    "join" => SyntaxKind::JoinKw,
                    "union" => SyntaxKind::UnionKw,
                    "and" => SyntaxKind::AndKw,
                    "or" => SyntaxKind::OrKw,
                    "not" => SyntaxKind::NotKw,
                    "contains" => SyntaxKind::ContainsKw,
                    "notcontains" | "!contains" => SyntaxKind::NotContainsKw,
                    "contains_cs" => SyntaxKind::ContainsCsKw,
                    "has" => SyntaxKind::HasKw,
                    "nothas" | "!has" => SyntaxKind::NotHasKw,
                    "has_cs" => SyntaxKind::HasCsKw,
                    "startswith" => SyntaxKind::StartswithKw,
                    "endswith" => SyntaxKind::EndswithKw,
                    "matches" => SyntaxKind::MatchesRegexKw,
                    "in" => SyntaxKind::InKw,
                    "between" => SyntaxKind::BetweenKw,
                    _ => SyntaxKind::Identifier,
                };
                Token { kind, len: end - start }
            }

            _ => {
                chars.next();
                Token { kind: SyntaxKind::Error, len: ch.len_utf8() }
            }
        };

        tokens.push(token);
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_simple_query() {
        let tokens = lex("StormEvents | take 10");
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(kinds, vec![
            SyntaxKind::Identifier,  // StormEvents
            SyntaxKind::Whitespace,  // " "
            SyntaxKind::Pipe,        // |
            SyntaxKind::Whitespace,  // " "
            SyntaxKind::TakeKw,      // take
            SyntaxKind::Whitespace,  // " "
            SyntaxKind::IntLiteral,  // 10
        ]);
    }

    #[test]
    fn lex_where_clause() {
        let tokens = lex("| where X > 5");
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(kinds, vec![
            SyntaxKind::Pipe,
            SyntaxKind::Whitespace,
            SyntaxKind::WhereKw,
            SyntaxKind::Whitespace,
            SyntaxKind::Identifier,
            SyntaxKind::Whitespace,
            SyntaxKind::GreaterThan,
            SyntaxKind::Whitespace,
            SyntaxKind::IntLiteral,
        ]);
    }

    #[test]
    fn lex_comment() {
        let tokens = lex("// comment\nStormEvents");
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(kinds, vec![
            SyntaxKind::Comment,
            SyntaxKind::Newline,
            SyntaxKind::Identifier,
        ]);
    }

    #[test]
    fn lex_string_literal() {
        let tokens = lex("'hello'");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, SyntaxKind::StringLiteral);
        assert_eq!(tokens[0].len, 7);
    }

    #[test]
    fn lex_operators() {
        let tokens = lex("== != >= <= > < =");
        let kinds: Vec<_> = tokens.iter().filter(|t| t.kind != SyntaxKind::Whitespace).map(|t| t.kind).collect();
        assert_eq!(kinds, vec![
            SyntaxKind::EqualEqual,
            SyntaxKind::NotEqual,
            SyntaxKind::GreaterEqual,
            SyntaxKind::LessEqual,
            SyntaxKind::GreaterThan,
            SyntaxKind::LessThan,
            SyntaxKind::Equals,
        ]);
    }
}
