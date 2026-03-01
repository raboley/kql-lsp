//! Syntax kinds for the KQL CST (Concrete Syntax Tree).
//! Uses rowan for lossless syntax trees.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    // Tokens
    Whitespace = 0,
    Newline,
    Identifier,
    IntLiteral,
    StringLiteral,
    Pipe,
    Equals,
    EqualEqual,
    GreaterThan,
    LessThan,
    GreaterEqual,
    LessEqual,
    NotEqual,
    LParen,
    RParen,
    Comma,
    Dot,
    Semicolon,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Comment,
    Error,
    Eof,

    TimespanLiteral,

    // Keywords
    WhereKw,
    TakeKw,
    LimitKw,
    LetKw,
    ByKw,
    ProjectKw,
    ExtendKw,
    SummarizeKw,
    SortKw,
    OrderKw,
    TopKw,
    CountKw,
    DistinctKw,
    JoinKw,
    UnionKw,
    AndKw,
    OrKw,
    NotKw,
    ContainsKw,
    NotContainsKw,
    ContainsCsKw,
    HasKw,
    NotHasKw,
    HasCsKw,
    StartswithKw,
    EndswithKw,
    MatchesRegexKw,
    InKw,
    BetweenKw,

    // Composite nodes
    SourceFile,
    LetStatement,
    QueryStatement,
    PipeExpression,
    TakeClause,
    WhereClause,
    ProjectClause,
    ExtendClause,
    SummarizeClause,
    SortClause,
    TopClause,
    CountClause,
    ColumnAssignment,
    FunctionCallExpr,
    BinaryExpr,
    ParenExpr,
    NameRef,
    Literal,
    ManagementCommand,
    ErrorNode,
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KqlLanguage {}

impl rowan::Language for KqlLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        assert!(raw.0 <= SyntaxKind::ErrorNode as u16, "SyntaxKind out of range: {} (max: {})", raw.0, SyntaxKind::ErrorNode as u16);
        // SAFETY: SyntaxKind is repr(u16) and we checked bounds
        unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0) }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<KqlLanguage>;
#[allow(dead_code)]
pub type SyntaxToken = rowan::SyntaxToken<KqlLanguage>;
