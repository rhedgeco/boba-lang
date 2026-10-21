/// The kinds of brackets a lexer recognises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Bracket {
    /// Round brackets: `( )`
    Round,
    /// Curly brackets: `{ }`
    Curly,
}

/// A string literal token.
///
/// The literal may be unterminated if the source ended before a closing quote
/// was found, which is tracked by [`closed`](StrLiteral::closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrLiteral {
    /// Whether the literal was terminated by a closing quote.
    pub closed: bool,
}

/// An identifier, either a reserved keyword or a user defined name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ident {
    /// A custom user defined name.
    Custom,
    /// The `pub` keyword.
    Pub,
    /// The `const` keyword.
    Const,
}

/// A single token produced by a lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Token {
    /// An unrecognized character.
    Invalid,
    /// An identifier or keyword.
    Ident(Ident),
    /// A string literal.
    StrLiteral,
    /// A string literal that is unclosed.
    UnclosedStringLiteral,
    /// An opening bracket.
    Open(Bracket),
    /// A closing bracket.
    Close(Bracket),
    /// An equals sign: `=`
    Equality,
    /// A colon: `:`
    Colon,
    /// A semicolon: `;`
    SemiColon,
    /// A comma: `,`
    Comma,
    /// A point: `.`
    Point,
}
