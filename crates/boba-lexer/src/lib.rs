//! A character driven lexer for the boba language.

use std::num::NonZeroUsize;

use derive_more::From;

pub mod back_slash;
pub mod comment;
pub mod forward_slash;
pub mod identifier;
pub mod inline_string;
pub mod iter;
pub mod keyword;
pub mod multiline_string;
pub mod newline;

// re-export token for convenience
pub use boba_token::Token;

use crate::iter::PeekableChar;

/// Non lexical tokens produced by the lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Trivia {
    /// An invalid token.
    Invalid,
    /// A string that is missing its closing quote.
    UnclosedStr,
    /// A line comment introduced with `//`.
    Comment,
    /// A line break from either `\n`, `\r\n`, or just `\r`.
    Newline,
    /// A single space character.
    Space,
    /// A single tab character.
    Tab,
}

/// Either a lexical or trivia token.
#[derive(Debug, From, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LexOutput {
    /// A valid lexical token.
    Token(Token),
    /// A supporting trivia token.
    Trivia(Trivia),
}

/// The result of a single parsing step.
///
/// An [`Ok`] holds a completed token,
/// while an [`Err`] holds the state needed to resume parsing once more input is available.
pub type LexResult = Result<LexOutput, LexState>;

impl From<LexOutput> for LexResult {
    fn from(output: LexOutput) -> Self {
        Ok(output)
    }
}

impl From<LexState> for LexResult {
    fn from(lexer: LexState) -> Self {
        Err(lexer)
    }
}

/// A parsing step that is paused partway through a multi character token.
///
/// Each variant records where lexing left off,
/// so that the next character can be handled correctly when parsing resumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LexState {
    /// Parsing a back slash that may begin a multiline string.
    BackSlash,
    /// Consuming the body of a line comment.
    Comment,
    /// Parsing a forward slash that may begin a comment.
    ForwardSlash,
    /// Consuming the body of an identifier.
    Identifier,
    /// Parsing the character after an escape inside an inline string.
    InlineEscape,
    /// Consuming the body of an inline string.
    InlineString,
    /// Matching a possible `const` keyword, holding the matched length.
    KeywordConstant(NonZeroUsize),
    /// Matching a possible `pub` keyword, holding the matched length.
    KeywordPublic(NonZeroUsize),
    /// Matching a possible `var` keyword, holding the matched length.
    KeywordVariable(NonZeroUsize),
    /// Consuming the body of a multiline string.
    MultilineString,
    /// Parsing a `\r` newline that may be followed by a `\n`.
    Newline,
}

impl LexState {
    /// Resumes parsing from this state and always produces a token.
    ///
    /// If the input runs out before the token completes,
    /// the resumable state is finished in place with [`LexState::finish`].
    pub fn parse(self, input: impl PeekableChar) -> LexOutput {
        match self.try_parse(input) {
            Err(state) => state.finish(),
            Ok(output) => output,
        }
    }

    /// Completes this state at the end of input without consuming more.
    ///
    /// This is used to finish a token when the input has been exhausted while parsing was still in progress.
    pub fn finish(self) -> LexOutput {
        match self {
            Self::BackSlash => back_slash::finish(),
            Self::Comment => comment::finish(),
            Self::ForwardSlash => forward_slash::finish(),
            Self::Identifier => identifier::finish(),
            Self::InlineEscape => inline_string::finish_unclosed(),
            Self::InlineString => inline_string::finish_unclosed(),
            Self::KeywordConstant(length) => keyword::finish_constant(length),
            Self::KeywordPublic(length) => keyword::finish_public(length),
            Self::KeywordVariable(length) => keyword::finish_variable(length),
            Self::MultilineString => multiline_string::finish(),
            Self::Newline => newline::finish(),
        }
    }

    /// Resumes parsing from this state for a single step.
    ///
    /// Returns [`Ok`] with a completed token,
    /// or [`Err`] with resumable state if the token is still pending.
    pub fn try_parse(self, input: impl PeekableChar) -> LexResult {
        match self {
            Self::BackSlash => back_slash::parse_next(input),
            Self::Comment => comment::parse_next(input),
            Self::ForwardSlash => forward_slash::parse_next(input),
            Self::Identifier => identifier::parse_next(input),
            Self::InlineEscape => inline_string::parse_next_escape(input),
            Self::InlineString => inline_string::parse_next(input),
            Self::KeywordConstant(length) => keyword::parse_next_constant(length, input),
            Self::KeywordPublic(length) => keyword::parse_next_public(length, input),
            Self::KeywordVariable(length) => keyword::parse_next_variable(length, input),
            Self::MultilineString => multiline_string::parse_next(input),
            Self::Newline => newline::parse_next(input),
        }
    }
}

/// Parses a single token from the start of the input.
///
/// Returns [`None`] if the input is completely exhausted.
/// Any token that is still pending when the input ends is finished and returned.
pub fn parse(input: impl PeekableChar) -> Option<LexOutput> {
    match try_parse(input)? {
        Err(state) => state.finish().into(),
        Ok(output) => output.into(),
    }
}

/// Parses a single step from the start of the input.
///
/// Returns [`None`] if the input is completely exhausted.
/// Otherwise it parses characters and either produces a token or resumable state.
pub fn try_parse(mut input: impl PeekableChar) -> Option<LexResult> {
    Some(match input.next()? {
        // --- SINGLE-CHAR TOKENS ---
        // some simple characters can be processed immediately
        ' ' => Ok(Trivia::Space.into()),
        '.' => Ok(Token::AccessMember.into()),
        ',' => Ok(Token::Delimiter.into()),
        '=' => Ok(Token::Assign.into()),
        '+' => Ok(Token::Add.into()),
        '-' => Ok(Token::Subtract.into()),
        '*' => Ok(Token::Multiply.into()),
        '%' => Ok(Token::Modulo.into()),
        ':' => Ok(Token::AccessScope.into()),
        ';' => Ok(Token::CloseExpr.into()),
        '(' => Ok(Token::OpenGroup.into()),
        ')' => Ok(Token::CloseGroup.into()),
        '{' => Ok(Token::OpenScope.into()),
        '}' => Ok(Token::CloseScope.into()),
        '\t' => Ok(Trivia::Tab.into()),
        '\n' => Ok(Trivia::Newline.into()),

        // --- MULTI-CHAR TOKENS ---
        // a back slash could result in a multiline string
        '\\' => back_slash::parse_next(input),
        // a forward slash could result in a comment or division symbol
        '/' => forward_slash::parse_next(input),
        // a quote opens an inline string literal
        '"' => inline_string::parse_next(input),
        // '\r' newlines might be followed by a '\n'. '\r\n' is a single newline
        '\r' => newline::parse_next(input),

        // --- KEYWORDS / IDENTIFIERS ---
        // keywords start with a specific letter
        // the start length is 1 since the initial letter is included
        'p' => keyword::parse_next_public(NonZeroUsize::MIN, input),
        'v' => keyword::parse_next_variable(NonZeroUsize::MIN, input),
        'c' => keyword::parse_next_constant(NonZeroUsize::MIN, input),
        // any other letter is lastly assumed to be an ident
        // identifiers have a range of valid start characters
        c if identifier::is_valid_start(c) => identifier::parse_next(input),

        // --- INVALID CHARACTERS ---
        // any other character is invalid for the language
        _ => Ok(Trivia::Invalid.into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // parses every token from a string until the input is exhausted
    fn parse_tokens(text: &str) -> Vec<LexOutput> {
        let mut input = text.chars().peekable();
        let mut tokens = Vec::new();
        while let Some(token) = parse(&mut input) {
            tokens.push(token);
        }
        tokens
    }

    #[test]
    fn single_char_tokens_dispatch() {
        assert_eq!(parse_tokens(" "), vec![Trivia::Space.into()]);
        assert_eq!(parse_tokens("."), vec![Token::AccessMember.into()]);
        assert_eq!(parse_tokens(","), vec![Token::Delimiter.into()]);
        assert_eq!(parse_tokens("="), vec![Token::Assign.into()]);
        assert_eq!(parse_tokens("+"), vec![Token::Add.into()]);
        assert_eq!(parse_tokens("-"), vec![Token::Subtract.into()]);
        assert_eq!(parse_tokens("*"), vec![Token::Multiply.into()]);
        assert_eq!(parse_tokens("%"), vec![Token::Modulo.into()]);
        assert_eq!(parse_tokens(":"), vec![Token::AccessScope.into()]);
        assert_eq!(parse_tokens(";"), vec![Token::CloseExpr.into()]);
        assert_eq!(parse_tokens("("), vec![Token::OpenGroup.into()]);
        assert_eq!(parse_tokens(")"), vec![Token::CloseGroup.into()]);
        assert_eq!(parse_tokens("{"), vec![Token::OpenScope.into()]);
        assert_eq!(parse_tokens("}"), vec![Token::CloseScope.into()]);
        assert_eq!(parse_tokens("\t"), vec![Trivia::Tab.into()]);
        assert_eq!(parse_tokens("\n"), vec![Trivia::Newline.into()]);
    }

    #[test]
    fn invalid_character_is_trivia() {
        assert_eq!(parse_tokens("§"), vec![Trivia::Invalid.into()]);
    }

    #[test]
    fn empty_input_yields_none() {
        assert_eq!(parse_tokens(""), vec![]);
    }

    #[test]
    fn multi_char_dispatch() {
        // a lone forward slash is division
        assert_eq!(parse_tokens("/"), vec![Token::Divide.into()]);
        // a double forward slash is a comment
        assert_eq!(parse_tokens("//comment"), vec![Trivia::Comment.into()]);
        // a double back slash is a multiline string
        assert_eq!(
            parse_tokens("\\\\text"),
            vec![Token::MultilineString.into()]
        );
        // a lone back slash is invalid
        assert_eq!(parse_tokens("\\"), vec![Trivia::Invalid.into()]);
        // a quote opens an inline string
        assert_eq!(parse_tokens("\"closed\""), vec![Token::InlineString.into()]);
        // a carriage return is a newline
        assert_eq!(parse_tokens("\r"), vec![Trivia::Newline.into()]);
    }

    #[test]
    fn keyword_dispatch() {
        // a complete keyword followed by a boundary is the keyword token
        assert_eq!(
            parse_tokens("pub "),
            vec![Token::Public.into(), Trivia::Space.into()]
        );
        assert_eq!(
            parse_tokens("var "),
            vec![Token::Variable.into(), Trivia::Space.into()]
        );
        assert_eq!(
            parse_tokens("const "),
            vec![Token::Constant.into(), Trivia::Space.into()]
        );
    }

    #[test]
    fn keyword_at_eof_is_finished_as_keyword() {
        // a keyword that ends the input parks as pending, then finishes as the keyword
        assert_eq!(parse_tokens("pub"), vec![Token::Public.into()]);
        assert_eq!(parse_tokens("var"), vec![Token::Variable.into()]);
        assert_eq!(parse_tokens("const"), vec![Token::Constant.into()]);
    }

    #[test]
    fn keyword_prefix_is_identifier() {
        // starts like a keyword but continues, so it is an identifier
        assert_eq!(
            parse_tokens("public "),
            vec![Token::Identifier.into(), Trivia::Space.into()]
        );
        // a non keyword letter is a plain identifier
        assert_eq!(
            parse_tokens("name "),
            vec![Token::Identifier.into(), Trivia::Space.into()]
        );
    }

    #[test]
    fn pending_state_is_finished_at_eof() {
        // a forward slash with no following char finishes as division
        assert_eq!(parse_tokens("/"), vec![Token::Divide.into()]);
        // a bare keyword prefix at eof finishes as the keyword
        assert_eq!(parse_tokens("pub"), vec![Token::Public.into()]);
    }

    #[test]
    fn try_parse_returns_resumable_state() {
        // a quote with no closing quote leaves the string pending
        let mut input = "\"open".chars().peekable();
        assert_eq!(try_parse(&mut input), Some(Err(LexState::InlineString)));
    }

    #[test]
    fn whole_string_lexes_into_token_sequence() {
        // keywords, trivia, and identifiers all advance across their boundaries
        assert_eq!(
            parse_tokens("pub var"),
            vec![
                Token::Public.into(),
                Trivia::Space.into(),
                Token::Variable.into(),
            ]
        );
    }

    #[test]
    fn boundary_char_begins_the_next_token() {
        // a keyword ends on its boundary, which is then lexed as its own token
        assert_eq!(
            parse_tokens("pub.name"),
            vec![
                Token::Public.into(),
                Token::AccessMember.into(),
                Token::Identifier.into(),
            ]
        );
    }

    #[test]
    fn trailing_pending_token_is_finished() {
        // the final keyword ends the input while pending, then finishes as its token
        assert_eq!(
            parse_tokens("name const"),
            vec![
                Token::Identifier.into(),
                Trivia::Space.into(),
                Token::Constant.into(),
            ]
        );
    }

    #[test]
    fn lexer_state_finish_covers_all_variants() {
        assert_eq!(LexState::BackSlash.finish(), Trivia::Invalid.into());
        assert_eq!(LexState::Comment.finish(), Trivia::Comment.into());
        assert_eq!(LexState::ForwardSlash.finish(), Token::Divide.into());
        assert_eq!(LexState::Identifier.finish(), Token::Identifier.into());
        assert_eq!(LexState::InlineEscape.finish(), Trivia::UnclosedStr.into());
        assert_eq!(LexState::InlineString.finish(), Trivia::UnclosedStr.into());
        assert_eq!(
            LexState::MultilineString.finish(),
            Token::MultilineString.into()
        );
        assert_eq!(LexState::Newline.finish(), Trivia::Newline.into());
        // keywords finish as their token only at the full length
        assert_eq!(
            LexState::KeywordPublic(keyword::PUBLIC.len()).finish(),
            Token::Public.into()
        );
        assert_eq!(
            LexState::KeywordPublic(NonZeroUsize::MIN).finish(),
            Token::Identifier.into()
        );
    }

    #[test]
    fn lexer_state_try_parse_resumes() {
        // resuming a comment state consumes the body until the newline
        let mut input = "body\n".chars().peekable();
        let result = LexState::Comment.try_parse(&mut input);
        assert_eq!(result, Ok(Trivia::Comment.into()));
    }
}
