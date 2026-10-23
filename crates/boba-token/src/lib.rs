use derive_more::Display;

/// A generic token for constructing the boba language
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Token {
    // bracing tokens and delimiters
    OpenScope,
    CloseScope,
    OpenGroup,
    CloseGroup,
    Delimiter,
    CloseExpr,

    // identifiers and literals
    FloatLiteral,
    IntegerLiteral,
    InlineStrLiteral,
    MultiLineStrLiteral,
    Identifier,
    Constant,
    Variable,
    Public,

    // operators
    Assign,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    AccessScope,
    AccessMember,
}
