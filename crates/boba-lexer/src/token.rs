use derive_more::Display;

/// The kinds of brackets a lexer recognises.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Bracket {
    /// Round brackets: `( )`
    Round,
    /// Curly brackets: `{ }`
    Curly,
}

/// An identifier, either a reserved keyword or a user supplied name.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ident<'a> {
    /// A user defined name, borrowed from the source.
    Custom(&'a str),
    /// The `pub` keyword.
    Pub,
    /// The `scope` keyword.
    Scope,
    /// The `type` keyword.
    Type,
    /// The `let` keyword.
    Let,
}

/// A single token produced by a lexer.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Token<'a> {
    /// An unrecognized character.
    Invalid(char),
    /// An identifier or keyword.
    Ident(Ident<'a>),
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

impl<'a> Ident<'a> {
    /// Builds an [`Ident`] from a string.
    /// Resolves it to a keyword if one matches, otherwise treats it as a custom identifier.
    pub fn build(ident: &'a str) -> Self {
        // phf keyword table matching strings to concrete identifiers
        static KEYWORDS: phf::Map<&'static str, Ident<'static>> = phf::phf_map! {
            "pub" => Ident::Pub,
            "scope" => Ident::Scope,
            "type" => Ident::Type,
            "let" => Ident::Let,
        };

        // try to map the ident to a keyword first, otherwise build a custom ident
        KEYWORDS.get(ident).copied().unwrap_or(Ident::Custom(ident))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ident_build_maps_known_keywords() {
        // keywords should resolve to their correct variants
        assert_eq!(Ident::build("pub"), Ident::Pub);
        assert_eq!(Ident::build("scope"), Ident::Scope);
        assert_eq!(Ident::build("type"), Ident::Type);
        assert_eq!(Ident::build("let"), Ident::Let);
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
