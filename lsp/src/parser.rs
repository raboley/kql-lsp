//! KQL parser - builds a rowan CST from a token stream.

use crate::lexer::{self, Token};
use crate::syntax::{KqlLanguage, SyntaxKind, SyntaxNode};
use rowan::GreenNodeBuilder;

/// A parse error with a message and text range.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub offset: usize,
    pub len: usize,
}

/// Result of parsing: a rowan green tree + errors.
pub struct ParseResult {
    pub green: rowan::GreenNode,
    pub errors: Vec<ParseError>,
}

impl ParseResult {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }
}

/// Parse KQL source text into a CST.
pub fn parse(input: &str) -> ParseResult {
    let tokens = lexer::lex(input);
    let mut parser = Parser::new(input, tokens);
    parser.parse_source_file();
    ParseResult {
        green: parser.builder.finish(),
        errors: parser.errors,
    }
}

struct Parser<'a> {
    input: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    offset: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str, tokens: Vec<Token>) -> Self {
        Self {
            input,
            tokens,
            pos: 0,
            offset: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
        }
    }

    fn current(&self) -> Option<SyntaxKind> {
        self.tokens.get(self.pos).map(|t| t.kind)
    }

    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == Some(kind)
    }

    fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        self.current().map_or(false, |k| kinds.contains(&k))
    }

    /// Consume current token and add it to the tree.
    fn bump(&mut self) {
        if let Some(token) = self.tokens.get(self.pos) {
            let text = &self.input[self.offset..self.offset + token.len];
            self.builder.token(token.kind.into(), text);
            self.offset += token.len;
            self.pos += 1;
        }
    }

    /// Consume token if it matches, return whether it matched.
    fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Expect a specific token kind, emit error if not found.
    fn expect(&mut self, kind: SyntaxKind) -> bool {
        if self.eat(kind) {
            true
        } else {
            let msg = format!("expected {:?}", kind);
            self.error_at_current(&msg);
            false
        }
    }

    fn skip_trivia(&mut self) {
        while self.at(SyntaxKind::Whitespace)
            || self.at(SyntaxKind::Newline)
            || self.at(SyntaxKind::Comment)
        {
            self.bump();
        }
    }

    fn error_at_current(&mut self, message: &str) {
        let offset = self.offset;
        let len = self.current_token().map_or(0, |t| t.len);
        self.errors.push(ParseError {
            message: message.to_string(),
            offset,
            len: len.max(1),
        });
    }

    fn error_recover(&mut self, message: &str) {
        self.error_at_current(message);
        self.builder.start_node(SyntaxKind::ErrorNode.into());
        // Consume tokens until we hit a recovery point (pipe, newline, or EOF)
        while let Some(kind) = self.current() {
            if kind == SyntaxKind::Pipe || kind == SyntaxKind::Newline || kind == SyntaxKind::Eof {
                break;
            }
            self.bump();
        }
        self.builder.finish_node();
    }

    // === Grammar rules ===

    fn parse_source_file(&mut self) {
        self.builder.start_node(SyntaxKind::SourceFile.into());

        self.skip_trivia();

        while self.pos < self.tokens.len() {
            if self.at(SyntaxKind::LetKw) {
                self.parse_let_statement();
            } else if self.at_any(&[SyntaxKind::Identifier, SyntaxKind::Pipe]) {
                self.parse_query_statement();
            } else if self.at_any(&[SyntaxKind::Whitespace, SyntaxKind::Newline, SyntaxKind::Comment]) {
                self.bump();
            } else if self.current().is_some() {
                self.error_recover("unexpected token");
            } else {
                break;
            }
        }

        self.builder.finish_node();
    }

    fn parse_let_statement(&mut self) {
        self.builder.start_node(SyntaxKind::LetStatement.into());

        self.bump(); // let keyword
        self.skip_trivia();

        // Parse name
        if self.at(SyntaxKind::Identifier) {
            self.builder.start_node(SyntaxKind::NameRef.into());
            self.bump();
            self.builder.finish_node();
        } else {
            self.error_at_current("expected identifier after 'let'");
        }

        self.skip_trivia();

        // Parse = sign
        if !self.eat(SyntaxKind::Equals) {
            self.error_at_current("expected '=' in let statement");
        }

        self.skip_trivia();

        // Parse value expression - consume everything until semicolon or end
        // For now, just parse a simple expression or consume tokens until semicolon
        if !self.at(SyntaxKind::Semicolon) && self.current().is_some() {
            self.parse_expression();
        }

        self.skip_trivia();

        // Parse optional semicolon
        self.eat(SyntaxKind::Semicolon);

        self.builder.finish_node();
    }

    fn parse_query_statement(&mut self) {
        self.builder.start_node(SyntaxKind::QueryStatement.into());

        // Parse initial table name (identifier)
        if self.at(SyntaxKind::Identifier) {
            self.builder.start_node(SyntaxKind::NameRef.into());
            self.bump();
            self.builder.finish_node();
        }

        self.skip_trivia();

        // Parse pipe expressions
        while self.at(SyntaxKind::Pipe) {
            self.parse_pipe_expression();
            self.skip_trivia();
        }

        self.builder.finish_node();
    }

    fn parse_pipe_expression(&mut self) {
        self.builder.start_node(SyntaxKind::PipeExpression.into());

        // Consume the pipe
        self.bump(); // |
        self.skip_trivia();

        // Parse the operator after the pipe
        match self.current() {
            Some(SyntaxKind::TakeKw) | Some(SyntaxKind::LimitKw) => {
                self.parse_take_clause();
            }
            Some(SyntaxKind::WhereKw) => {
                self.parse_where_clause();
            }
            Some(SyntaxKind::ProjectKw) => {
                self.parse_project_clause();
            }
            Some(SyntaxKind::ExtendKw) => {
                self.parse_extend_clause();
            }
            Some(SyntaxKind::SummarizeKw) => {
                self.parse_summarize_clause();
            }
            Some(SyntaxKind::SortKw) | Some(SyntaxKind::OrderKw) => {
                self.parse_sort_clause();
            }
            Some(SyntaxKind::TopKw) => {
                self.parse_top_clause();
            }
            Some(SyntaxKind::CountKw) => {
                self.parse_count_clause();
            }
            Some(SyntaxKind::DistinctKw)
            | Some(SyntaxKind::JoinKw)
            | Some(SyntaxKind::UnionKw) => {
                // Known keywords - consume loosely for now
                self.bump();
                self.skip_trivia();
                self.consume_until_pipe();
            }
            Some(SyntaxKind::Identifier) => {
                // Unknown operator - just consume the identifier and any following tokens
                // until the next pipe or end
                self.bump();
                self.skip_trivia();
                self.consume_until_pipe();
            }
            _ => {
                self.error_at_current("expected operator after '|'");
                self.builder.start_node(SyntaxKind::ErrorNode.into());
                self.builder.finish_node();
            }
        }

        self.builder.finish_node();
    }

    fn parse_take_clause(&mut self) {
        self.builder.start_node(SyntaxKind::TakeClause.into());

        self.bump(); // take/limit keyword
        self.skip_trivia();

        if self.at(SyntaxKind::IntLiteral) {
            self.builder.start_node(SyntaxKind::Literal.into());
            self.bump();
            self.builder.finish_node();
        } else {
            self.error_at_current("expected integer after 'take'");
        }

        self.builder.finish_node();
    }

    fn parse_where_clause(&mut self) {
        self.builder.start_node(SyntaxKind::WhereClause.into());

        self.bump(); // where keyword
        self.skip_trivia();

        // Parse the predicate expression
        if self.at_eof_or_pipe() {
            self.error_at_current("expected expression after 'where'");
        } else {
            self.parse_expression();
        }

        self.builder.finish_node();
    }

    fn at_eof_or_pipe(&self) -> bool {
        self.current().is_none() || self.at(SyntaxKind::Pipe)
    }

    fn consume_until_pipe(&mut self) {
        while self.current().is_some()
            && !self.at(SyntaxKind::Pipe)
            && !self.at(SyntaxKind::Newline)
        {
            self.bump();
        }
    }

    fn parse_project_clause(&mut self) {
        self.builder.start_node(SyntaxKind::ProjectClause.into());

        self.bump(); // project keyword
        self.skip_trivia();

        // Parse comma-separated column list
        if !self.at_eof_or_pipe() {
            self.parse_expression();
            self.skip_trivia();

            while self.eat(SyntaxKind::Comma) {
                self.skip_trivia();
                if !self.at_eof_or_pipe() {
                    self.parse_expression();
                    self.skip_trivia();
                }
            }
        }

        self.builder.finish_node();
    }

    fn parse_extend_clause(&mut self) {
        self.builder.start_node(SyntaxKind::ExtendClause.into());

        self.bump(); // extend keyword
        self.skip_trivia();

        // Parse comma-separated assignments: Name = Expr, ...
        if !self.at_eof_or_pipe() {
            self.parse_expression();
            self.skip_trivia();

            // Handle = assignment
            if self.eat(SyntaxKind::Equals) {
                self.skip_trivia();
                if !self.at_eof_or_pipe() {
                    self.parse_expression();
                    self.skip_trivia();
                }
            }

            while self.eat(SyntaxKind::Comma) {
                self.skip_trivia();
                if !self.at_eof_or_pipe() {
                    self.parse_expression();
                    self.skip_trivia();

                    if self.eat(SyntaxKind::Equals) {
                        self.skip_trivia();
                        if !self.at_eof_or_pipe() {
                            self.parse_expression();
                            self.skip_trivia();
                        }
                    }
                }
            }
        }

        self.builder.finish_node();
    }

    fn parse_summarize_clause(&mut self) {
        self.builder.start_node(SyntaxKind::SummarizeClause.into());

        self.bump(); // summarize keyword
        self.skip_trivia();

        // Parse aggregation expressions until 'by' or end
        if !self.at_eof_or_pipe() && !self.at(SyntaxKind::ByKw) {
            self.parse_expression();
            self.skip_trivia();

            // Handle = assignment (e.g., Count = count())
            if self.eat(SyntaxKind::Equals) {
                self.skip_trivia();
                if !self.at_eof_or_pipe() {
                    self.parse_expression();
                    self.skip_trivia();
                }
            }

            while self.eat(SyntaxKind::Comma) {
                self.skip_trivia();
                if !self.at_eof_or_pipe() && !self.at(SyntaxKind::ByKw) {
                    self.parse_expression();
                    self.skip_trivia();

                    if self.eat(SyntaxKind::Equals) {
                        self.skip_trivia();
                        if !self.at_eof_or_pipe() {
                            self.parse_expression();
                            self.skip_trivia();
                        }
                    }
                }
            }
        }

        // Parse optional 'by' clause
        if self.eat(SyntaxKind::ByKw) {
            self.skip_trivia();

            if !self.at_eof_or_pipe() {
                self.parse_expression();
                self.skip_trivia();

                while self.eat(SyntaxKind::Comma) {
                    self.skip_trivia();
                    if !self.at_eof_or_pipe() {
                        self.parse_expression();
                        self.skip_trivia();
                    }
                }
            }
        }

        self.builder.finish_node();
    }

    fn parse_sort_clause(&mut self) {
        self.builder.start_node(SyntaxKind::SortClause.into());

        self.bump(); // sort/order keyword
        self.skip_trivia();

        // Expect 'by' keyword
        self.eat(SyntaxKind::ByKw);
        self.skip_trivia();

        // Parse column list with optional asc/desc
        if !self.at_eof_or_pipe() {
            self.parse_expression();
            self.skip_trivia();
            // Consume optional asc/desc
            if self.at(SyntaxKind::Identifier) {
                if let Some(token) = self.current_token() {
                    let text = &self.input[self.offset..self.offset + token.len];
                    if text == "asc" || text == "desc" {
                        self.bump();
                        self.skip_trivia();
                    }
                }
            }

            while self.eat(SyntaxKind::Comma) {
                self.skip_trivia();
                if !self.at_eof_or_pipe() {
                    self.parse_expression();
                    self.skip_trivia();
                    // Optional asc/desc
                    if self.at(SyntaxKind::Identifier) {
                        if let Some(token) = self.current_token() {
                            let text = &self.input[self.offset..self.offset + token.len];
                            if text == "asc" || text == "desc" {
                                self.bump();
                                self.skip_trivia();
                            }
                        }
                    }
                }
            }
        }

        self.builder.finish_node();
    }

    fn parse_top_clause(&mut self) {
        self.builder.start_node(SyntaxKind::TopClause.into());

        self.bump(); // top keyword
        self.skip_trivia();

        // Parse count
        if self.at(SyntaxKind::IntLiteral) {
            self.builder.start_node(SyntaxKind::Literal.into());
            self.bump();
            self.builder.finish_node();
        }
        self.skip_trivia();

        // Expect 'by' keyword
        self.eat(SyntaxKind::ByKw);
        self.skip_trivia();

        // Parse sort expression
        if !self.at_eof_or_pipe() {
            self.parse_expression();
            self.skip_trivia();
            // Optional asc/desc
            if self.at(SyntaxKind::Identifier) {
                if let Some(token) = self.current_token() {
                    let text = &self.input[self.offset..self.offset + token.len];
                    if text == "asc" || text == "desc" {
                        self.bump();
                        self.skip_trivia();
                    }
                }
            }
        }

        self.builder.finish_node();
    }

    fn parse_count_clause(&mut self) {
        self.builder.start_node(SyntaxKind::CountClause.into());
        self.bump(); // count keyword
        self.builder.finish_node();
    }

    fn parse_expression(&mut self) {
        self.parse_binary_expr();
    }

    fn at_binary_operator(&self) -> bool {
        self.at_any(&[
            SyntaxKind::EqualEqual,
            SyntaxKind::NotEqual,
            SyntaxKind::GreaterThan,
            SyntaxKind::LessThan,
            SyntaxKind::GreaterEqual,
            SyntaxKind::LessEqual,
            SyntaxKind::Plus,
            SyntaxKind::Minus,
            SyntaxKind::Star,
            SyntaxKind::Slash,
            SyntaxKind::Percent,
            // String operators
            SyntaxKind::ContainsKw,
            SyntaxKind::NotContainsKw,
            SyntaxKind::ContainsCsKw,
            SyntaxKind::HasKw,
            SyntaxKind::NotHasKw,
            SyntaxKind::HasCsKw,
            SyntaxKind::StartswithKw,
            SyntaxKind::EndswithKw,
            SyntaxKind::MatchesRegexKw,
            SyntaxKind::InKw,
            SyntaxKind::BetweenKw,
            // Logical operators
            SyntaxKind::AndKw,
            SyntaxKind::OrKw,
        ])
    }

    fn parse_binary_expr(&mut self) {
        self.parse_primary();
        self.skip_trivia();

        // Check for binary operator
        while self.at_binary_operator() {
            // Wrap existing LHS in a BinaryExpr
            let checkpoint = self.builder.checkpoint();
            self.builder.start_node_at(checkpoint, SyntaxKind::BinaryExpr.into());

            self.bump(); // operator
            self.skip_trivia();

            if self.at_eof_or_pipe() {
                self.error_at_current("expected expression after operator");
            } else {
                self.parse_primary();
                self.skip_trivia();
            }

            self.builder.finish_node();
        }
    }

    fn parse_primary(&mut self) {
        match self.current() {
            Some(SyntaxKind::Identifier) | Some(SyntaxKind::CountKw) => {
                // Check if it's a function call: identifier followed by (
                let checkpoint = self.builder.checkpoint();
                self.builder.start_node(SyntaxKind::NameRef.into());
                self.bump();
                self.builder.finish_node();

                // Look ahead for function call
                if self.at(SyntaxKind::LParen) {
                    self.builder.start_node_at(checkpoint, SyntaxKind::FunctionCallExpr.into());
                    self.bump(); // (
                    self.skip_trivia();

                    // Parse arguments
                    if !self.at(SyntaxKind::RParen) && self.current().is_some() {
                        self.parse_expression();
                        self.skip_trivia();

                        while self.eat(SyntaxKind::Comma) {
                            self.skip_trivia();
                            if !self.at(SyntaxKind::RParen) && self.current().is_some() {
                                self.parse_expression();
                                self.skip_trivia();
                            }
                        }
                    }

                    self.expect(SyntaxKind::RParen);
                    self.builder.finish_node();
                }
            }
            Some(SyntaxKind::IntLiteral) | Some(SyntaxKind::StringLiteral) | Some(SyntaxKind::TimespanLiteral) => {
                self.builder.start_node(SyntaxKind::Literal.into());
                self.bump();
                self.builder.finish_node();
            }
            Some(SyntaxKind::NotKw) => {
                // Prefix not operator
                self.builder.start_node(SyntaxKind::BinaryExpr.into());
                self.bump(); // not
                self.skip_trivia();
                if !self.at_eof_or_pipe() {
                    self.parse_primary();
                }
                self.builder.finish_node();
            }
            Some(SyntaxKind::Minus) => {
                // Unary minus
                self.builder.start_node(SyntaxKind::BinaryExpr.into());
                self.bump(); // -
                self.skip_trivia();
                if !self.at_eof_or_pipe() {
                    self.parse_primary();
                }
                self.builder.finish_node();
            }
            Some(SyntaxKind::LParen) => {
                self.builder.start_node(SyntaxKind::ParenExpr.into());
                self.bump(); // (
                self.skip_trivia();
                self.parse_expression();
                self.skip_trivia();
                self.expect(SyntaxKind::RParen);
                self.builder.finish_node();
            }
            _ => {
                self.error_at_current("expected expression");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_take_query_no_errors() {
        let result = parse("StormEvents | take 10");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn parse_where_incomplete_has_error() {
        let result = parse("StormEvents | where");
        assert!(!result.errors.is_empty(), "Should have parse errors for incomplete where");
        assert!(
            result.errors.iter().any(|e| e.message.contains("expected expression")),
            "Should mention expected expression, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn parse_where_with_predicate_no_errors() {
        let result = parse("StormEvents | where DamageProperty > 100");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn parse_multi_pipe() {
        let result = parse("StormEvents | where State == 'TEXAS' | take 10");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn parse_where_missing_rhs() {
        let result = parse("StormEvents | where X >");
        assert!(!result.errors.is_empty(), "Should error on missing RHS");
    }

    #[test]
    fn parse_project_clause() {
        let result = parse("StormEvents | project State, EventType");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn parse_extend_clause() {
        let result = parse("StormEvents | extend Duration = EndTime - StartTime");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn parse_summarize_with_by() {
        let result = parse("StormEvents | summarize count() by State");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn parse_sort_by() {
        let result = parse("StormEvents | sort by State desc");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn parse_top_clause() {
        let result = parse("StormEvents | top 10 by Count desc");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn parse_count_clause() {
        let result = parse("StormEvents | count");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn parse_function_call() {
        let result = parse("StormEvents | summarize count()");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn parse_contains_operator() {
        let result = parse("StormEvents | where State contains \"TEX\"");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn parse_has_operator() {
        let result = parse("StormEvents | where Name has \"storm\"");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn parse_and_or_operators() {
        let result = parse("StormEvents | where State contains \"TEX\" and DamageProperty > 0");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn parse_timespan_literal() {
        let result = parse("StormEvents | where StartTime > ago(1h)");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }
}
