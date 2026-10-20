use std::fmt::Display;

use derive_more::Display;

/// The kinds of brackets a lexer recognises.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
pub struct StrLiteral<'a> {
    /// The literal's contents, borrowed from the source, excluding the
    /// surrounding quotes.
    pub content: &'a str,
    /// Whether the literal was terminated by a closing quote.
    pub closed: bool,
}

impl<'a> Display for StrLiteral<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\"{}{}",
            self.content,
            match self.closed {
                true => "\"",
                false => "",
            }
        )
    }
}

/// An identifier, either a reserved keyword or a user supplied name.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ident<'a> {
    /// A user defined name, borrowed from the source.
    Custom(&'a str),
    /// The `pub` keyword.
    Pub,
    /// The `const` keyword.
    Const,
    /// The `use` keyword.
    Use,
}

/// A single token produced by a lexer.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Token<'a> {
    /// An unrecognized character.
    Invalid(char),
    /// An identifier or keyword.
    Ident(Ident<'a>),
    /// A string literal.
    StrLiteral(StrLiteral<'a>),
    /// An opening bracket.
    #[display("Open {_0}")]
    Open(Bracket),
    /// A closing bracket.
    #[display("Close {_0}")]
    Close(Bracket),
    /// An equals sign: `=`
    Equality,
    /// A colon: `:`
    Colon,
    /// A double colon: `::`
    DoubleColon,
    /// A semicolon: `;`
    SemiColon,
    /// A comma: `,`
    Comma,
    /// A point: `.`
    Point,
}

impl<'a> Ident<'a> {
    /// Builds an [`Ident`] from a string.
    /// Resolves it to a keyword if one matches, otherwise treats it as a custom identifier.
    pub fn build(ident: &'a str) -> Self {
        // phf keyword table matching strings to concrete identifiers
        static KEYWORDS: phf::Map<&'static str, Ident<'static>> = phf::phf_map! {
            "pub" => Ident::Pub,
            "const" => Ident::Const,
            "use" => Ident::Use,
        };

        // try to map the ident to a keyword first, otherwise build a custom ident
        KEYWORDS.get(ident).copied().unwrap_or(Ident::Custom(ident))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_literal_display_closed() {
        // a closed literal renders with both quotes
        let lit = StrLiteral {
            content: "hello",
            closed: true,
        };
        assert_eq!(lit.to_string(), "\"hello\"");
    }

    #[test]
    fn str_literal_display_unclosed() {
        // an unclosed literal omits the trailing quote
        let lit = StrLiteral {
            content: "hello",
            closed: false,
        };
        assert_eq!(lit.to_string(), "\"hello");
    }

    #[test]
    fn ident_build_maps_known_keywords() {
        // keywords should resolve to their correct variants
        assert_eq!(Ident::build("pub"), Ident::Pub);
        assert_eq!(Ident::build("const"), Ident::Const);
        assert_eq!(Ident::build("use"), Ident::Use);
    }

    #[test]
    fn ident_build_falls_back_to_custom() {
        // anything not in the keyword table becomes a custom ident
        assert_eq!(Ident::build("foo"), Ident::Custom("foo"));
    }

    #[test]
    fn ident_build_is_case_sensitive() {
        // keyword matching is exact, so `Pub` is not the keyword
        assert_eq!(Ident::build("Pub"), Ident::Custom("Pub"));
    }
}
