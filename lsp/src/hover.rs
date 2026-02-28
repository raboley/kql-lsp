//! Hover documentation for KQL built-in functions and operators.

use crate::lexer;
use crate::syntax::SyntaxKind;

/// Built-in function documentation.
struct FunctionDoc {
    name: &'static str,
    signature: &'static str,
    description: &'static str,
}

const BUILTIN_FUNCTIONS: &[FunctionDoc] = &[
    FunctionDoc { name: "count", signature: "count() -> long", description: "Returns the number of rows in the group." },
    FunctionDoc { name: "sum", signature: "sum(expr) -> numeric", description: "Returns the sum of expr across the group." },
    FunctionDoc { name: "avg", signature: "avg(expr) -> double", description: "Returns the average of expr across the group." },
    FunctionDoc { name: "min", signature: "min(expr) -> scalar", description: "Returns the minimum value of expr across the group." },
    FunctionDoc { name: "max", signature: "max(expr) -> scalar", description: "Returns the maximum value of expr across the group." },
    FunctionDoc { name: "dcount", signature: "dcount(expr) -> long", description: "Returns an estimate of the number of distinct values of expr." },
    FunctionDoc { name: "countif", signature: "countif(predicate) -> long", description: "Returns a count of rows for which predicate evaluates to true." },
    FunctionDoc { name: "sumif", signature: "sumif(expr, predicate) -> numeric", description: "Returns a sum of expr for which predicate evaluates to true." },
    FunctionDoc { name: "ago", signature: "ago(timespan) -> datetime", description: "Subtracts the given timespan from the current UTC clock time." },
    FunctionDoc { name: "now", signature: "now() -> datetime", description: "Returns the current UTC clock time." },
    FunctionDoc { name: "strcat", signature: "strcat(arg1, arg2, ...) -> string", description: "Concatenates between 1 and 64 arguments." },
    FunctionDoc { name: "tostring", signature: "tostring(expr) -> string", description: "Converts the input to a string representation." },
    FunctionDoc { name: "toint", signature: "toint(expr) -> int", description: "Converts the input to an integer value." },
    FunctionDoc { name: "tolong", signature: "tolong(expr) -> long", description: "Converts the input to a long value." },
    FunctionDoc { name: "strlen", signature: "strlen(source) -> long", description: "Returns the length, in characters, of the input string." },
    FunctionDoc { name: "substring", signature: "substring(source, startIndex, length) -> string", description: "Extracts a substring from the source string." },
    FunctionDoc { name: "trim", signature: "trim(regex, source) -> string", description: "Removes leading and trailing matches of the specified regex." },
    FunctionDoc { name: "toupper", signature: "toupper(source) -> string", description: "Converts a string to upper case." },
    FunctionDoc { name: "tolower", signature: "tolower(source) -> string", description: "Converts a string to lower case." },
    FunctionDoc { name: "replace_string", signature: "replace_string(text, lookup, rewrite) -> string", description: "Replaces all string matches with another string." },
    FunctionDoc { name: "bin", signature: "bin(value, roundTo) -> scalar", description: "Rounds values down to an integer multiple of a given bin size." },
    FunctionDoc { name: "floor", signature: "floor(value, roundTo) -> scalar", description: "Rounds values down to an integer multiple of a given bin size." },
    FunctionDoc { name: "ceiling", signature: "ceiling(value, roundTo) -> scalar", description: "Rounds values up to an integer multiple of a given bin size." },
    FunctionDoc { name: "round", signature: "round(value, precision) -> scalar", description: "Rounds the value to the specified precision." },
    FunctionDoc { name: "iif", signature: "iif(condition, ifTrue, ifFalse) -> scalar", description: "Returns ifTrue or ifFalse depending on the condition." },
    FunctionDoc { name: "iff", signature: "iff(condition, ifTrue, ifFalse) -> scalar", description: "Returns ifTrue or ifFalse depending on the condition." },
    FunctionDoc { name: "isempty", signature: "isempty(value) -> bool", description: "Returns true if the argument is an empty string or null." },
    FunctionDoc { name: "isnotempty", signature: "isnotempty(value) -> bool", description: "Returns true if the argument is not an empty string or null." },
    FunctionDoc { name: "isnull", signature: "isnull(value) -> bool", description: "Returns true if the argument is null." },
    FunctionDoc { name: "isnotnull", signature: "isnotnull(value) -> bool", description: "Returns true if the argument is not null." },
    FunctionDoc { name: "extract", signature: "extract(regex, captureGroup, source) -> string", description: "Get a match for a regular expression from a text string." },
    FunctionDoc { name: "parse_json", signature: "parse_json(json) -> dynamic", description: "Interprets a string as a JSON value and returns the value as dynamic." },
    FunctionDoc { name: "todynamic", signature: "todynamic(expr) -> dynamic", description: "Converts the input to a dynamic value." },
    FunctionDoc { name: "todatetime", signature: "todatetime(expr) -> datetime", description: "Converts the input to a datetime value." },
    FunctionDoc { name: "totimespan", signature: "totimespan(expr) -> timespan", description: "Converts the input to a timespan value." },
    FunctionDoc { name: "todouble", signature: "todouble(expr) -> double", description: "Converts the input to a double value." },
    FunctionDoc { name: "toreal", signature: "toreal(expr) -> real", description: "Converts the input to a real (double) value." },
    FunctionDoc { name: "format_datetime", signature: "format_datetime(datetime, format) -> string", description: "Formats a datetime according to the provided format string." },
    FunctionDoc { name: "datetime_diff", signature: "datetime_diff(period, datetime1, datetime2) -> long", description: "Returns the difference between two datetimes." },
    FunctionDoc { name: "startofday", signature: "startofday(datetime) -> datetime", description: "Returns the start of the day for the given datetime." },
    FunctionDoc { name: "endofday", signature: "endofday(datetime) -> datetime", description: "Returns the end of the day for the given datetime." },
    FunctionDoc { name: "startofweek", signature: "startofweek(datetime) -> datetime", description: "Returns the start of the week for the given datetime." },
    FunctionDoc { name: "startofmonth", signature: "startofmonth(datetime) -> datetime", description: "Returns the start of the month for the given datetime." },
    FunctionDoc { name: "startofyear", signature: "startofyear(datetime) -> datetime", description: "Returns the start of the year for the given datetime." },
    FunctionDoc { name: "array_length", signature: "array_length(array) -> long", description: "Returns the number of elements in the array." },
    FunctionDoc { name: "pack", signature: "pack(key1, value1, ...) -> dynamic", description: "Creates a dynamic property bag from a list of names and values." },
    FunctionDoc { name: "pack_all", signature: "pack_all() -> dynamic", description: "Creates a dynamic property bag from all columns." },
    FunctionDoc { name: "percentile", signature: "percentile(expr, percentile) -> scalar", description: "Returns the value of expr at the specified percentile." },
    FunctionDoc { name: "make_list", signature: "make_list(expr) -> dynamic", description: "Returns a dynamic JSON array of all the values of expr in the group." },
    FunctionDoc { name: "make_set", signature: "make_set(expr) -> dynamic", description: "Returns a dynamic JSON array of the set of distinct values of expr." },
];

/// Tabular operator documentation (for when keywords appear after pipe).
struct OperatorDoc {
    name: &'static str,
    description: &'static str,
}

const TABULAR_OPERATORS: &[OperatorDoc] = &[
    OperatorDoc { name: "where", description: "Filters rows based on a predicate expression." },
    OperatorDoc { name: "project", description: "Selects columns to include, rename, or drop from the output." },
    OperatorDoc { name: "extend", description: "Creates calculated columns and appends them to the result set." },
    OperatorDoc { name: "summarize", description: "Produces a table that aggregates the content of the input table." },
    OperatorDoc { name: "take", description: "Returns up to the specified number of rows." },
    OperatorDoc { name: "limit", description: "Returns up to the specified number of rows." },
    OperatorDoc { name: "top", description: "Returns the first N records sorted by the specified columns." },
    OperatorDoc { name: "sort", description: "Sorts the rows of the input table by one or more columns." },
    OperatorDoc { name: "order", description: "Sorts the rows of the input table by one or more columns." },
    OperatorDoc { name: "count", description: "Returns the number of rows in the input table." },
    OperatorDoc { name: "distinct", description: "Produces a table with the distinct combination of the provided columns." },
    OperatorDoc { name: "join", description: "Merges the rows of two tables to form a new table." },
    OperatorDoc { name: "union", description: "Takes two or more tables and returns all their rows." },
];

/// Hover result.
pub struct HoverResult {
    pub markdown: String,
}

/// Get hover documentation for the token at the given byte offset.
pub fn hover_at(text: &str, offset: usize) -> Option<HoverResult> {
    let tokens = lexer::lex(text);
    let mut token_offset = 0;

    for token in &tokens {
        let token_end = token_offset + token.len;

        if offset >= token_offset && offset < token_end {
            let token_text = &text[token_offset..token_end];
            return hover_for_token(token.kind, token_text);
        }

        token_offset = token_end;
    }

    None
}

fn hover_for_token(kind: SyntaxKind, text: &str) -> Option<HoverResult> {
    match kind {
        SyntaxKind::Identifier => {
            // Check if it's a built-in function
            if let Some(doc) = BUILTIN_FUNCTIONS.iter().find(|f| f.name == text) {
                return Some(HoverResult {
                    markdown: format!("```kql\n{}\n```\n\n{}", doc.signature, doc.description),
                });
            }
            None
        }
        SyntaxKind::CountKw => {
            // count can be both a function and a tabular operator
            if let Some(doc) = BUILTIN_FUNCTIONS.iter().find(|f| f.name == text) {
                return Some(HoverResult {
                    markdown: format!("```kql\n{}\n```\n\n{}", doc.signature, doc.description),
                });
            }
            if let Some(doc) = TABULAR_OPERATORS.iter().find(|o| o.name == text) {
                return Some(HoverResult {
                    markdown: format!("**{}** (tabular operator)\n\n{}", text, doc.description),
                });
            }
            None
        }
        SyntaxKind::WhereKw
        | SyntaxKind::ProjectKw
        | SyntaxKind::ExtendKw
        | SyntaxKind::SummarizeKw
        | SyntaxKind::TakeKw
        | SyntaxKind::LimitKw
        | SyntaxKind::SortKw
        | SyntaxKind::OrderKw
        | SyntaxKind::TopKw
        | SyntaxKind::DistinctKw
        | SyntaxKind::JoinKw
        | SyntaxKind::UnionKw => {
            if let Some(doc) = TABULAR_OPERATORS.iter().find(|o| o.name == text) {
                return Some(HoverResult {
                    markdown: format!("**{}** (tabular operator)\n\n{}", text, doc.description),
                });
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_count_function() {
        let result = hover_at("StormEvents | summarize count()", 24);
        assert!(result.is_some());
        let hover = result.unwrap();
        assert!(hover.markdown.contains("count"));
        assert!(hover.markdown.contains("long"));
    }

    #[test]
    fn hover_where_keyword() {
        let result = hover_at("StormEvents | where X > 5", 14);
        assert!(result.is_some());
        let hover = result.unwrap();
        assert!(hover.markdown.contains("where"));
        assert!(hover.markdown.contains("Filters"));
    }

    #[test]
    fn hover_unknown_identifier() {
        let result = hover_at("StormEvents | take 10", 3);
        assert!(result.is_none());
    }
}
