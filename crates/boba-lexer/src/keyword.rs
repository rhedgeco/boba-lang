use std::marker::PhantomData;

use boba_token::Token;

use crate::{
    LexerBuilder, LexerOutput, LexerResult,
    ident::{is_valid_ident_body, is_valid_ident_start},
};

pub struct KeywordStr<'a> {
    value: &'a str,
}

impl<'a> KeywordStr<'a> {
    pub const fn value(&self) -> &'a str {
        self.value
    }

    pub const fn len(&self) -> usize {
        self.value.as_bytes().len()
    }

    pub const fn get(&self, index: usize) -> Option<char> {
        let bytes = self.value.as_bytes();
        if index >= bytes.len() {
            return None;
        }

        Some(bytes[index] as char)
    }

    pub const fn build(keyword: &'a str) -> Option<Self> {
        // convert the keyword into bytes
        // all keywords should be valid ascii <255
        let bytes = keyword.as_bytes();

        // ensure the bytes are not empty
        if bytes.is_empty() {
            return None;
        }

        // check the first byte to ensure it is a valid ident start char
        let first_char = bytes[0] as char;
        if !is_valid_ident_start(first_char) {
            return None;
        }

        // check each consecutive byte to see if it is a valid body char
        let mut index = 1;
        while index < bytes.len() {
            let next_char = bytes[index] as char;
            if !is_valid_ident_body(next_char) {
                return None;
            }
            index += 1;
        }

        // if all pass, then we have a valid keyword
        Some(Self { value: keyword })
    }
}

pub trait Keyword {
    const KEY: KeywordStr<'static>;
    const TOKEN: Token;
}

/// An auto implemented trait for keywords that provides helpful const info
pub trait KeywordInfo: Keyword {
    const PRIMER: char;
    const LENGTH: usize;
}
impl<K: Keyword> KeywordInfo for K {
    const PRIMER: char = K::KEY.get(0).unwrap();
    const LENGTH: usize = K::KEY.len();
}

#[macro_export]
macro_rules! keyword {
    ($token:ident => $key:literal) => {
        $crate::keyword!($token: $token => $key);
    };
    ($name:ident: $token:ident => $key:literal) => {
        #[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name;
        impl $crate::keyword::Keyword for $name {
            const KEY: $crate::keyword::KeywordStr<'static> = $crate::keyword::KeywordStr::<'static>::build($key).unwrap();
            const TOKEN: $crate::Token = $crate::Token::$token;
        }
    };
}

keyword!(Public => "pub");
keyword!(Variable => "var");
keyword!(Constant => "const");

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeywordLexer<K> {
    _keyword: PhantomData<K>,
    length: usize,
}

impl<K: Keyword> KeywordLexer<K> {
    pub const fn primed() -> Self {
        Self {
            _keyword: PhantomData,
            length: 1,
        }
    }

    fn build_token(&self) -> Token {
        if self.length == K::LENGTH {
            return K::TOKEN;
        }

        Token::Identifier
    }
}

impl<K: Keyword> LexerBuilder for KeywordLexer<K> {
    type NextState = Self;

    fn finish(self) -> Option<LexerOutput> {
        Some(self.build_token().into())
    }

    fn build(mut self, c: char) -> LexerResult<Self::NextState> {
        // check if we should bail immediately
        if !is_valid_ident_body(c) {
            return LexerResult::Bail(self.build_token().into());
        }

        // if the length is already too long, just return next
        if self.length > K::LENGTH {
            return LexerResult::Next(self);
        }

        // otherwise update the keyword checker
        match K::KEY.get(self.length) {
            // if the character is valid, update the length
            Some(key) if c == key => self.length += 1,
            // if the character is not valid, overshoot the length
            // this means the next check wont even get here
            _ => self.length = K::LENGTH + 1,
        }

        LexerResult::Next(self)
    }
}
