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
            Some(SyntaxKind::Identifier) => {
                // Unknown operator - just consume the identifier and any following tokens
                // until the next pipe or end
                self.bump();
                self.skip_trivia();
                while self.current().is_some()
                    && !self.at(SyntaxKind::Pipe)
                    && !self.at(SyntaxKind::Newline)
                {
                    self.bump();
                }
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

    fn parse_expression(&mut self) {
        self.parse_binary_expr();
    }

    fn parse_binary_expr(&mut self) {
        self.parse_primary();
        self.skip_trivia();

        // Check for binary operator
        while self.at_any(&[
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
        ]) {
            // Wrap existing LHS in a BinaryExpr
            let checkpoint = self.builder.checkpoint();
            self.builder.start_node_at(checkpoint, SyntaxKind::BinaryExpr.into());

            // Note: LHS was already parsed above, we need to restructure.
            // For now, just bump the operator and parse RHS.
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
            Some(SyntaxKind::Identifier) => {
                self.builder.start_node(SyntaxKind::NameRef.into());
                self.bump();
                self.builder.finish_node();
            }
            Some(SyntaxKind::IntLiteral) | Some(SyntaxKind::StringLiteral) => {
                self.builder.start_node(SyntaxKind::Literal.into());
                self.bump();
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
}
