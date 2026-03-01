//! Shared catalog of KQL built-in functions, operators, and keyword metadata.
//! Single source of truth used by hover, completion, and signature help.

use crate::syntax::SyntaxKind;

/// A parameter in a function signature.
pub struct ParamInfo {
    pub label: &'static str,
}

/// Built-in function metadata.
pub struct BuiltinFunction {
    pub name: &'static str,
    pub signature: &'static str,
    pub description: &'static str,
    pub parameters: &'static [ParamInfo],
}

/// Tabular operator metadata (operators that appear after `|`).
pub struct TabularOperator {
    pub name: &'static str,
    pub description: &'static str,
}

/// String/logical operator metadata.
pub struct StringOperator {
    pub name: &'static str,
    pub description: &'static str,
}

// ---------------------------------------------------------------------------
// Built-in functions
// ---------------------------------------------------------------------------

pub const BUILTIN_FUNCTIONS: &[BuiltinFunction] = &[
    // Aggregation functions
    BuiltinFunction { name: "count", signature: "count() -> long", description: "Returns the number of rows in the group.", parameters: &[] },
    BuiltinFunction { name: "sum", signature: "sum(expr) -> numeric", description: "Returns the sum of expr across the group.", parameters: &[ParamInfo { label: "expr" }] },
    BuiltinFunction { name: "avg", signature: "avg(expr) -> double", description: "Returns the average of expr across the group.", parameters: &[ParamInfo { label: "expr" }] },
    BuiltinFunction { name: "min", signature: "min(expr) -> scalar", description: "Returns the minimum value of expr across the group.", parameters: &[ParamInfo { label: "expr" }] },
    BuiltinFunction { name: "max", signature: "max(expr) -> scalar", description: "Returns the maximum value of expr across the group.", parameters: &[ParamInfo { label: "expr" }] },
    BuiltinFunction { name: "dcount", signature: "dcount(expr) -> long", description: "Returns an estimate of the number of distinct values of expr.", parameters: &[ParamInfo { label: "expr" }] },
    BuiltinFunction { name: "countif", signature: "countif(predicate) -> long", description: "Returns a count of rows for which predicate evaluates to true.", parameters: &[ParamInfo { label: "predicate" }] },
    BuiltinFunction { name: "sumif", signature: "sumif(expr, predicate) -> numeric", description: "Returns a sum of expr for which predicate evaluates to true.", parameters: &[ParamInfo { label: "expr" }, ParamInfo { label: "predicate" }] },
    BuiltinFunction { name: "percentile", signature: "percentile(expr, percentile) -> scalar", description: "Returns the value of expr at the specified percentile.", parameters: &[ParamInfo { label: "expr" }, ParamInfo { label: "percentile" }] },
    BuiltinFunction { name: "make_list", signature: "make_list(expr) -> dynamic", description: "Returns a dynamic JSON array of all the values of expr in the group.", parameters: &[ParamInfo { label: "expr" }] },
    BuiltinFunction { name: "make_set", signature: "make_set(expr) -> dynamic", description: "Returns a dynamic JSON array of the set of distinct values of expr.", parameters: &[ParamInfo { label: "expr" }] },
    // DateTime functions
    BuiltinFunction { name: "ago", signature: "ago(timespan) -> datetime", description: "Subtracts the given timespan from the current UTC clock time.", parameters: &[ParamInfo { label: "timespan" }] },
    BuiltinFunction { name: "now", signature: "now() -> datetime", description: "Returns the current UTC clock time.", parameters: &[] },
    BuiltinFunction { name: "todatetime", signature: "todatetime(expr) -> datetime", description: "Converts the input to a datetime value.", parameters: &[ParamInfo { label: "expr" }] },
    BuiltinFunction { name: "totimespan", signature: "totimespan(expr) -> timespan", description: "Converts the input to a timespan value.", parameters: &[ParamInfo { label: "expr" }] },
    BuiltinFunction { name: "format_datetime", signature: "format_datetime(datetime, format) -> string", description: "Formats a datetime according to the provided format string.", parameters: &[ParamInfo { label: "datetime" }, ParamInfo { label: "format" }] },
    BuiltinFunction { name: "datetime_diff", signature: "datetime_diff(period, datetime1, datetime2) -> long", description: "Returns the difference between two datetimes.", parameters: &[ParamInfo { label: "period" }, ParamInfo { label: "datetime1" }, ParamInfo { label: "datetime2" }] },
    BuiltinFunction { name: "startofday", signature: "startofday(datetime) -> datetime", description: "Returns the start of the day for the given datetime.", parameters: &[ParamInfo { label: "datetime" }] },
    BuiltinFunction { name: "endofday", signature: "endofday(datetime) -> datetime", description: "Returns the end of the day for the given datetime.", parameters: &[ParamInfo { label: "datetime" }] },
    BuiltinFunction { name: "startofweek", signature: "startofweek(datetime) -> datetime", description: "Returns the start of the week for the given datetime.", parameters: &[ParamInfo { label: "datetime" }] },
    BuiltinFunction { name: "startofmonth", signature: "startofmonth(datetime) -> datetime", description: "Returns the start of the month for the given datetime.", parameters: &[ParamInfo { label: "datetime" }] },
    BuiltinFunction { name: "startofyear", signature: "startofyear(datetime) -> datetime", description: "Returns the start of the year for the given datetime.", parameters: &[ParamInfo { label: "datetime" }] },
    // String functions
    BuiltinFunction { name: "strcat", signature: "strcat(arg1, arg2, ...) -> string", description: "Concatenates between 1 and 64 arguments.", parameters: &[ParamInfo { label: "arg1" }, ParamInfo { label: "arg2" }] },
    BuiltinFunction { name: "strlen", signature: "strlen(source) -> long", description: "Returns the length, in characters, of the input string.", parameters: &[ParamInfo { label: "source" }] },
    BuiltinFunction { name: "substring", signature: "substring(source, startIndex, length) -> string", description: "Extracts a substring from the source string.", parameters: &[ParamInfo { label: "source" }, ParamInfo { label: "startIndex" }, ParamInfo { label: "length" }] },
    BuiltinFunction { name: "trim", signature: "trim(regex, source) -> string", description: "Removes leading and trailing matches of the specified regex.", parameters: &[ParamInfo { label: "regex" }, ParamInfo { label: "source" }] },
    BuiltinFunction { name: "toupper", signature: "toupper(source) -> string", description: "Converts a string to upper case.", parameters: &[ParamInfo { label: "source" }] },
    BuiltinFunction { name: "tolower", signature: "tolower(source) -> string", description: "Converts a string to lower case.", parameters: &[ParamInfo { label: "source" }] },
    BuiltinFunction { name: "replace_string", signature: "replace_string(text, lookup, rewrite) -> string", description: "Replaces all string matches with another string.", parameters: &[ParamInfo { label: "text" }, ParamInfo { label: "lookup" }, ParamInfo { label: "rewrite" }] },
    BuiltinFunction { name: "extract", signature: "extract(regex, captureGroup, source) -> string", description: "Get a match for a regular expression from a text string.", parameters: &[ParamInfo { label: "regex" }, ParamInfo { label: "captureGroup" }, ParamInfo { label: "source" }] },
    // Conversion functions
    BuiltinFunction { name: "tostring", signature: "tostring(expr) -> string", description: "Converts the input to a string representation.", parameters: &[ParamInfo { label: "expr" }] },
    BuiltinFunction { name: "toint", signature: "toint(expr) -> int", description: "Converts the input to an integer value.", parameters: &[ParamInfo { label: "expr" }] },
    BuiltinFunction { name: "tolong", signature: "tolong(expr) -> long", description: "Converts the input to a long value.", parameters: &[ParamInfo { label: "expr" }] },
    BuiltinFunction { name: "todouble", signature: "todouble(expr) -> double", description: "Converts the input to a double value.", parameters: &[ParamInfo { label: "expr" }] },
    BuiltinFunction { name: "toreal", signature: "toreal(expr) -> real", description: "Converts the input to a real (double) value.", parameters: &[ParamInfo { label: "expr" }] },
    BuiltinFunction { name: "todynamic", signature: "todynamic(expr) -> dynamic", description: "Converts the input to a dynamic value.", parameters: &[ParamInfo { label: "expr" }] },
    BuiltinFunction { name: "parse_json", signature: "parse_json(json) -> dynamic", description: "Interprets a string as a JSON value and returns the value as dynamic.", parameters: &[ParamInfo { label: "json" }] },
    // Math functions
    BuiltinFunction { name: "bin", signature: "bin(value, roundTo) -> scalar", description: "Rounds values down to an integer multiple of a given bin size.", parameters: &[ParamInfo { label: "value" }, ParamInfo { label: "roundTo" }] },
    BuiltinFunction { name: "floor", signature: "floor(value, roundTo) -> scalar", description: "Rounds values down to an integer multiple of a given bin size.", parameters: &[ParamInfo { label: "value" }, ParamInfo { label: "roundTo" }] },
    BuiltinFunction { name: "ceiling", signature: "ceiling(value, roundTo) -> scalar", description: "Rounds values up to an integer multiple of a given bin size.", parameters: &[ParamInfo { label: "value" }, ParamInfo { label: "roundTo" }] },
    BuiltinFunction { name: "round", signature: "round(value, precision) -> scalar", description: "Rounds the value to the specified precision.", parameters: &[ParamInfo { label: "value" }, ParamInfo { label: "precision" }] },
    // Conditional functions
    BuiltinFunction { name: "iif", signature: "iif(condition, ifTrue, ifFalse) -> scalar", description: "Returns ifTrue or ifFalse depending on the condition.", parameters: &[ParamInfo { label: "condition" }, ParamInfo { label: "ifTrue" }, ParamInfo { label: "ifFalse" }] },
    BuiltinFunction { name: "iff", signature: "iff(condition, ifTrue, ifFalse) -> scalar", description: "Returns ifTrue or ifFalse depending on the condition.", parameters: &[ParamInfo { label: "condition" }, ParamInfo { label: "ifTrue" }, ParamInfo { label: "ifFalse" }] },
    // Null/empty checks
    BuiltinFunction { name: "isempty", signature: "isempty(value) -> bool", description: "Returns true if the argument is an empty string or null.", parameters: &[ParamInfo { label: "value" }] },
    BuiltinFunction { name: "isnotempty", signature: "isnotempty(value) -> bool", description: "Returns true if the argument is not an empty string or null.", parameters: &[ParamInfo { label: "value" }] },
    BuiltinFunction { name: "isnull", signature: "isnull(value) -> bool", description: "Returns true if the argument is null.", parameters: &[ParamInfo { label: "value" }] },
    BuiltinFunction { name: "isnotnull", signature: "isnotnull(value) -> bool", description: "Returns true if the argument is not null.", parameters: &[ParamInfo { label: "value" }] },
    // Dynamic/array functions
    BuiltinFunction { name: "array_length", signature: "array_length(array) -> long", description: "Returns the number of elements in the array.", parameters: &[ParamInfo { label: "array" }] },
    BuiltinFunction { name: "pack", signature: "pack(key1, value1, ...) -> dynamic", description: "Creates a dynamic property bag from a list of names and values.", parameters: &[ParamInfo { label: "key1" }, ParamInfo { label: "value1" }] },
    BuiltinFunction { name: "pack_all", signature: "pack_all() -> dynamic", description: "Creates a dynamic property bag from all columns.", parameters: &[] },
];

// ---------------------------------------------------------------------------
// Tabular operators (after `|`)
// ---------------------------------------------------------------------------

pub const TABULAR_OPERATORS: &[TabularOperator] = &[
    TabularOperator { name: "where", description: "Filters rows based on a predicate expression." },
    TabularOperator { name: "project", description: "Selects columns to include, rename, or drop from the output." },
    TabularOperator { name: "extend", description: "Creates calculated columns and appends them to the result set." },
    TabularOperator { name: "summarize", description: "Produces a table that aggregates the content of the input table." },
    TabularOperator { name: "take", description: "Returns up to the specified number of rows." },
    TabularOperator { name: "limit", description: "Returns up to the specified number of rows." },
    TabularOperator { name: "top", description: "Returns the first N records sorted by the specified columns." },
    TabularOperator { name: "sort", description: "Sorts the rows of the input table by one or more columns." },
    TabularOperator { name: "order", description: "Sorts the rows of the input table by one or more columns." },
    TabularOperator { name: "count", description: "Returns the number of rows in the input table." },
    TabularOperator { name: "distinct", description: "Produces a table with the distinct combination of the provided columns." },
    TabularOperator { name: "join", description: "Merges the rows of two tables to form a new table." },
    TabularOperator { name: "union", description: "Takes two or more tables and returns all their rows." },
    TabularOperator { name: "render", description: "Renders results as a chart." },
    TabularOperator { name: "parse", description: "Evaluates a string expression and parses its value." },
    TabularOperator { name: "mv-expand", description: "Expands multi-value dynamic arrays or property bags." },
];

// ---------------------------------------------------------------------------
// String/logical operators
// ---------------------------------------------------------------------------

pub const STRING_OPERATORS: &[StringOperator] = &[
    StringOperator { name: "contains", description: "Returns true if the right-hand-side string occurs as a subsequence of the left-hand-side string (case-insensitive)." },
    StringOperator { name: "!contains", description: "Returns true if the right-hand-side string does NOT occur in the left-hand-side string (case-insensitive)." },
    StringOperator { name: "contains_cs", description: "Returns true if the right-hand-side string occurs as a subsequence of the left-hand-side string (case-sensitive)." },
    StringOperator { name: "has", description: "Returns true if the right-hand-side string is a whole term in the left-hand-side string (case-insensitive)." },
    StringOperator { name: "!has", description: "Returns true if the right-hand-side string is NOT a whole term in the left-hand-side string (case-insensitive)." },
    StringOperator { name: "has_cs", description: "Returns true if the right-hand-side string is a whole term in the left-hand-side string (case-sensitive)." },
    StringOperator { name: "startswith", description: "Returns true if the left-hand-side string starts with the right-hand-side string (case-insensitive)." },
    StringOperator { name: "endswith", description: "Returns true if the left-hand-side string ends with the right-hand-side string (case-insensitive)." },
    StringOperator { name: "matches regex", description: "Returns true if the left-hand-side string matches the right-hand-side regular expression." },
    StringOperator { name: "in", description: "Returns true if the value equals any of the elements in a list." },
    StringOperator { name: "between", description: "Returns true if the value is within an inclusive range." },
    StringOperator { name: "and", description: "Logical AND operator. Returns true if both operands are true." },
    StringOperator { name: "or", description: "Logical OR operator. Returns true if either operand is true." },
    StringOperator { name: "not", description: "Logical NOT operator. Negates the boolean expression." },
];

// ---------------------------------------------------------------------------
// SyntaxKind classification helpers
// ---------------------------------------------------------------------------

/// Returns true if the SyntaxKind is a keyword.
pub fn is_keyword(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::WhereKw
            | SyntaxKind::TakeKw
            | SyntaxKind::LimitKw
            | SyntaxKind::LetKw
            | SyntaxKind::ByKw
            | SyntaxKind::ProjectKw
            | SyntaxKind::ExtendKw
            | SyntaxKind::SummarizeKw
            | SyntaxKind::SortKw
            | SyntaxKind::OrderKw
            | SyntaxKind::TopKw
            | SyntaxKind::CountKw
            | SyntaxKind::DistinctKw
            | SyntaxKind::JoinKw
            | SyntaxKind::UnionKw
            | SyntaxKind::AndKw
            | SyntaxKind::OrKw
            | SyntaxKind::NotKw
            | SyntaxKind::ContainsKw
            | SyntaxKind::NotContainsKw
            | SyntaxKind::ContainsCsKw
            | SyntaxKind::HasKw
            | SyntaxKind::NotHasKw
            | SyntaxKind::HasCsKw
            | SyntaxKind::StartswithKw
            | SyntaxKind::EndswithKw
            | SyntaxKind::MatchesRegexKw
            | SyntaxKind::InKw
            | SyntaxKind::BetweenKw
    )
}

/// Returns true if the SyntaxKind is trivia (whitespace, newlines, comments).
pub fn is_trivia(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Whitespace | SyntaxKind::Newline | SyntaxKind::Comment
    )
}

/// Look up a built-in function by name.
pub fn find_function(name: &str) -> Option<&'static BuiltinFunction> {
    BUILTIN_FUNCTIONS.iter().find(|f| f.name == name)
}

/// Look up a tabular operator by name.
pub fn find_tabular_operator(name: &str) -> Option<&'static TabularOperator> {
    TABULAR_OPERATORS.iter().find(|o| o.name == name)
}

/// Look up a string/logical operator by name.
pub fn find_string_operator(name: &str) -> Option<&'static StringOperator> {
    STRING_OPERATORS.iter().find(|o| o.name == name)
}
