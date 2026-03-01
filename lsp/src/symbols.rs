//! Document symbols extraction from the CST.

use crate::parser::ParseResult;
use crate::syntax::{SyntaxKind, SyntaxNode};

/// LSP SymbolKind numeric values.
const SYMBOL_KIND_VARIABLE: i32 = 13;
const SYMBOL_KIND_EVENT: i32 = 24;
const SYMBOL_KIND_MODULE: i32 = 2; // Management commands

/// A document symbol extracted from the CST.
pub struct DocSymbol {
    pub name: String,
    pub kind: i32,
    pub range_start: usize,
    pub range_end: usize,
    pub selection_start: usize,
    pub selection_end: usize,
}

/// Extract document symbols from a parse result.
pub fn extract_symbols(parse_result: &ParseResult) -> Vec<DocSymbol> {
    let root = parse_result.syntax();
    let mut symbols = Vec::new();

    for child in root.children() {
        match child.kind() {
            SyntaxKind::LetStatement => {
                if let Some(sym) = extract_let_symbol(&child) {
                    symbols.push(sym);
                }
            }
            SyntaxKind::QueryStatement => {
                if let Some(sym) = extract_query_symbol(&child) {
                    symbols.push(sym);
                }
            }
            SyntaxKind::ManagementCommand => {
                if let Some(sym) = extract_management_symbol(&child) {
                    symbols.push(sym);
                }
            }
            _ => {}
        }
    }

    symbols
}

fn extract_let_symbol(node: &SyntaxNode) -> Option<DocSymbol> {
    // Find the NameRef child to get the variable name
    let name_ref = node.children().find(|c| c.kind() == SyntaxKind::NameRef)?;
    let name_token = name_ref.first_token()?;
    let name = name_token.text().to_string();

    let range_start = node.text_range().start().into();
    let range_end = node.text_range().end().into();
    let selection_start: usize = name_ref.text_range().start().into();
    let selection_end: usize = name_ref.text_range().end().into();

    Some(DocSymbol {
        name,
        kind: SYMBOL_KIND_VARIABLE,
        range_start,
        range_end,
        selection_start,
        selection_end,
    })
}

fn extract_query_symbol(node: &SyntaxNode) -> Option<DocSymbol> {
    // Find the first NameRef child (the table name)
    let name_ref = node.children().find(|c| c.kind() == SyntaxKind::NameRef)?;
    let name_token = name_ref.first_token()?;
    let name = name_token.text().to_string();

    let range_start = node.text_range().start().into();
    let range_end = node.text_range().end().into();
    let selection_start: usize = name_ref.text_range().start().into();
    let selection_end: usize = name_ref.text_range().end().into();

    Some(DocSymbol {
        name,
        kind: SYMBOL_KIND_EVENT,
        range_start,
        range_end,
        selection_start,
        selection_end,
    })
}

fn extract_management_symbol(node: &SyntaxNode) -> Option<DocSymbol> {
    // Build the command name from all tokens (e.g., ".show tables")
    let text = node.text().to_string().trim().to_string();
    let name = if text.len() > 40 {
        format!("{}...", &text[..40])
    } else {
        text
    };

    let range_start = node.text_range().start().into();
    let range_end = node.text_range().end().into();

    Some(DocSymbol {
        name,
        kind: SYMBOL_KIND_MODULE,
        range_start,
        range_end,
        selection_start: range_start,
        selection_end: range_end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn extract_let_and_query_symbols() {
        let result = parser::parse("let threshold = 100;\nStormEvents | take 10");
        let symbols = extract_symbols(&result);
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "threshold");
        assert_eq!(symbols[0].kind, SYMBOL_KIND_VARIABLE);
        assert_eq!(symbols[1].name, "StormEvents");
        assert_eq!(symbols[1].kind, SYMBOL_KIND_EVENT);
    }

    #[test]
    fn extract_query_only() {
        let result = parser::parse("StormEvents | take 10");
        let symbols = extract_symbols(&result);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "StormEvents");
    }

    #[test]
    fn extract_let_only() {
        let result = parser::parse("let x = 42;");
        let symbols = extract_symbols(&result);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "x");
        assert_eq!(symbols[0].kind, SYMBOL_KIND_VARIABLE);
    }
}
