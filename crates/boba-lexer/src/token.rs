/// A single token produced by a lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Token {
    // bracing tokens and delimiters
    OpenScope,
    CloseScope,
    OpenGroup,
    CloseGroup,
    Delimiter,
    CloseExpr,

    // identifiers and literals
    InlineStrLiteral,
    MultiLineStrLiteral,
    Identifier,
    Constant,
    Variable,
    Public,

    // operators
    Assign,
    Divide,
    ScopeAccess,
    MemberAccess,
}
