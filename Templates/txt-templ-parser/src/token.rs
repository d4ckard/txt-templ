use unic_locale::{Locale, locale};

#[derive(Clone, Debug)]
pub struct ContentTokens {
    tokens: Vec<ContentToken>,
    locale: Locale,
}

impl ContentTokens {
    pub fn new() -> Self {
        Self {
            tokens: vec![],
            locale: locale!("en-US"),
        }
    }
    pub fn from(locale: Locale) -> Self {
        Self {
            tokens: vec![],
            locale,
        }
    }

    pub fn push(&mut self, token: ContentToken) {
        self.tokens.push(token)
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn tokens_ref(&self) -> &Vec<ContentToken> {
        &self.tokens
    }

    pub fn locale_ref(&self) -> &Locale {
        &self.locale
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContentToken {
    Text(String),
    Key(Ident, Option<Box::<ContentToken>>),
    Constant(Ident),
    Option(Box::<ContentToken>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ident(String);

impl Ident {
    pub fn new(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl From<String> for Ident {
    fn from(s: String) -> Self {
        Self(s)
    }
}