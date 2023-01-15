mod parse;
mod scan;
pub use parse::UserError;
use scan::Scanner;

use unic_locale::{Locale, locale};
use crate::utils::LOGGING;
use once_cell::sync::Lazy;
use std::collections::HashMap;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use log::debug;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UserContentState {
    pub constants: HashMap<Ident, Content>,
    pub options: HashMap<Ident, HashMap<Ident, Content>>,
}

impl UserContentState {
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
            options: HashMap::new(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UserContent {
    pub keys: HashMap<Ident, Content>,
    pub choices: HashMap<Ident, Ident>,
}

impl UserContent {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            choices:  HashMap::new(),
        }
    }
}

// TODO: Add defaults field to `Content`

// Type containing ALL required content to  fill out a template
#[derive(Debug)]
pub struct FullContent(HashMap<Token, HashMap<Ident, Content>>);

impl FullContent {
    pub fn get(&self, token: TokenIdent) -> &Content {
        match self.0.get(&token.token) {
            Some(type_entries) => match type_entries.get(&token.ident) {
                Some(entry) => entry,
                None => panic!("Content was missing a requested entry {:?}", token),
            },
            None => panic!("Content was missing a requrest entry type {:?}", token),
        }
    }
}

// Map of all required tokens
// This struct directly maps identifers to chosen content values
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", serde_with::serde_as)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RequiredContent(
    #[cfg_attr(feature = "serde", serde_as(as = "Vec<(_, _)>"))]
    HashMap<Token, HashMap<Ident, Content>>,
);

impl RequiredContent {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, token: TokenIdent, content: &str) {
        let content = Content::new(content);
        match self.0.get_mut(&token.token) {
            Some(idents) => { idents.insert(token.ident, content); },
            None => {
                let mut map: HashMap<Ident, Content> = HashMap::new();
                map.insert(token.ident, content);
                self.0.insert(token.token, map);
            },
        };
    }

    pub fn add_constants(&mut self, mut constants: HashMap<Ident, Content>) {
        if let Some(entries) = self.0.get_mut(&Token::Constant) {
            // Move every piece of content for each required identifier into
            // the required constant entries.
            for (ident, value) in entries {
                if let Some(constant) = constants.remove(&ident) {
                    *value = constant;
                }
            }
        }
    }

    pub fn add_options(&mut self, choices: HashMap<Ident, Ident>, mut options: HashMap<Ident, HashMap<Ident, Content>>) {
        if let Some(entries) = self.0.get_mut(&Token::Option) {
            // Move every chosen piece of content for each required identifier into the
            // required constant entries.
            for (ident, value) in entries {
                // Get the option for the current identifier
                let option = match options.get_mut(&ident) {
                    Some(option) => option,
                    None => continue,
                };
                // Get the choosen option
                if let Some(choice) = choices.get(&ident) {
                    // The the content assoicates with the choice and move
                    // it into the required optin entries under the identifier
                    // for the option itself
                    if let Some(content) = option.remove(&choice) {
                        *value = content;
                    }
                }
            }
        }
    }

    pub fn add_keys(&mut self, mut keys: HashMap<Ident, Content>) {
        if let Some(entries) = self.0.get_mut(&Token::Key) {
            // Mov every piece of content for each required key
            // into the required key entries.
            for (ident, value) in entries {
                if let Some(key) = keys.remove(&ident) {
                    *value = key;
                }
            }
        }
    }
}

impl TryInto<FullContent> for RequiredContent {
    type Error = FillOutError;

    fn try_into(self) -> Result<FullContent, Self::Error> {
        debug!("{:?}", &self);
        // Check that there are entires without content
        for (token_type, entries) in &self.0 {
            for (ident, content) in entries {
                if content.is_empty() {
                    return match token_type {
                        Token::Key => Err(FillOutError::MissingKey(ident.clone())),
                        Token::Constant => Err(FillOutError::MissingConstant(ident.clone())),
                        Token::Option => Err(FillOutError::MissingOption(ident.clone())),
                    };
                }
            }
        }
        // Move all the entires into a new Content instance
        Ok(FullContent( self.0 ))
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TokenIdent {
    pub ident: Ident,
    pub token: Token,
}

impl TokenIdent {
    pub fn new(ident: &str, token: Token) -> Self {
        Self {
            ident: Ident::new(ident),
            token
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Token {
    Key,
    Constant,
    Option,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Token::Key => write!(f, "Key"),
            Token::Constant => write!(f, "Constant"),
            Token::Option => write!(f, "Option"),
        }
    }
}

#[derive(Debug)]
pub struct ContentTokens {
    tokens: Vec<ContentToken>,
    locale: Locale,
    friendly_errors: Vec<UserError>,
}

impl ContentTokens {
    pub fn new() -> Self {
        Self {
            tokens: vec![],
            locale: locale!("en-US"),
            friendly_errors: vec![],
        }
    }
    pub fn from(locale: Locale) -> Self {
        Self {
            tokens: vec![],
            locale,
            friendly_errors: vec![],
        }
    }

    // Add a friendly error to the `ContentTokens` instance
    pub fn add_friendly(&mut self, e: UserError) {
        self.friendly_errors.push(e);
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

    // Use the content map to substitue all values in `tokens` until
    // the entire template has been filled out.
    pub fn fill_out(&self, content: FullContent) -> Result<String, FillOutError> {
        Lazy::force(&LOGGING);

        let mut output = String::new();

        // Try to add the content for `token` to `output`
        fn fill_out_token(token: &ContentToken, content: &FullContent, output: &mut String) -> Result<(), FillOutError> {
            match token {
                ContentToken::Text(text) => output.push_str(&text),
                ContentToken::Constant(ident) => {
                    let content = content.get(TokenIdent::new(ident.as_ref(), Token::Constant));
                    output.push_str(content.as_ref());
                },
                ContentToken::Key(ident, _) => {
                    output.push_str(
                        content.get(TokenIdent::new(ident.as_ref(), Token::Key)).as_ref()
                    );
                    /*if let Some(default_box) = default_box {
                        return fill_out_token(*default_box, content, output);
                    }*/
                },
                ContentToken::Option(key_box) => {
                    let (ident, _) = match &**key_box {
                        ContentToken::Key(ident, default_box) => (ident, default_box),
                        _ => panic!("ContentToken::Option did not contain a ContentToken::Key instance. \
                            `parse::option` should not allow this!"),
                    };
                    output.push_str(
                        content.get(TokenIdent::new(ident.as_ref(), Token::Option)).as_ref()
                    );
                    /*if let Some(default_box) = default_box {
                        return fill_out_token(*default_box, content, output);
                    }*/
                },
            }
            Ok(())
        }
    
        for token in &self.tokens {
            fill_out_token(token, &content, &mut output)?;
        }
        Ok(output)
    }

    // Return a half-empty `RequiredContent` instance containing the identifiers and 
    // token-types of all the empty entries in the template
    // If there is a default value for a key or an option which is a text literal,
    // then this literal will be entered into the content table draft under this
    // key or option entry. If the user selects a value for this entry, the default 
    // will be overwritten.
    pub fn draft(&self) -> RequiredContent {
        let mut map = RequiredContent::new();

        // Recursively store default values
        fn get_default(token: &ContentToken, map: &mut RequiredContent) -> String {
            match token {
                ContentToken::Text(text) => text.clone(),
                ContentToken::Constant(ident) => {
                    map.insert(TokenIdent::new(ident.as_ref(), Token::Constant), "");
                    "".to_owned()
                },
                ContentToken::Key(ident, default_box) => {
                    let default = match default_box {
                        Some(default_box) => get_default(&*default_box, map),
                        None => "".to_owned(),
                    };
                    map.insert(TokenIdent::new(ident.as_ref(), Token::Key), &default);
                    default  // Propagate the default literal up
                },
                ContentToken::Option(key_box) => {
                    let (ident, default_box) = match &**key_box {
                        ContentToken::Key(ident, default_box) => (ident, default_box),
                        _ => panic!("ContentToken::Option did not contain a ContentToken::Key \
                            instance. `parse::option` should not allow this!"),
                    };
                    let default = match default_box {
                        Some(default_box) => get_default(&*default_box, map),
                        None => "".to_owned(),
                    };
                    map.insert(TokenIdent::new(ident.as_ref(), Token::Option), &default);
                    default  // Propagate the default literal up
                },
            }
        }

        for token in &self.tokens {
            match token {
                ContentToken::Text(_) => continue,  // `text` values are not representet as keys in the content map
                ContentToken::Constant(ident) => {
                    map.insert(TokenIdent::new(ident.as_ref(), Token::Constant), "");
                },
                ContentToken::Key(ident, default_box) => {
                    let default = match default_box {
                        Some(default_box) => get_default(&*default_box, &mut map),
                        None => String::new(),
                    };
                    map.insert(TokenIdent::new(ident.as_ref(), Token::Key), &default);
                },
                ContentToken::Option(key_box) => {
                    let (ident, default_box) = match &**key_box {
                        ContentToken::Key(ident, default_box) => (ident, default_box),
                        _ => panic!("ContentToken::Option did not contain a ContentToken::Key \
                            instance. `parse::option` should not allow this!"),
                    };
                    let default = match default_box {
                        Some(default_box) => get_default(&*default_box, &mut map),
                        None => String::new(),
                    };
                    map.insert(TokenIdent::new(ident.as_ref(), Token::Option), &default);
                },
            }
        }

        map
     }
}

impl std::str::FromStr for ContentTokens {
    type Err = UserError;
    
    // Attempt to parse the given string into a `ContentTokens` instance
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Lazy::force(&LOGGING);
        let mut scanner = Scanner::new(s);
        parse::template(&mut scanner)
    }
}

#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FillOutError {
    #[error("The given content is missing an option with the name {0}")]
    MissingOption(Ident),
    #[error("The given content is missing a constant with the name {0}")]
    MissingConstant(Ident),
    #[error("The given content is missing a key with the name {0}")]
    MissingKey(Ident),
    #[error("The chosen option for ident {0} is invalid. Valid option ident are {1}")]
    InvalidOption(Ident, Idents),
    #[error("The chosen constant for ident {0} is invalid. Valid constant ident are {1}")]
    InvalidConstant(Ident, Idents),
    #[error("The given content for the entry with the identifier {0} is empty")]
    EmptyContent(Ident),
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Idents(Vec<Ident>);

impl std::fmt::Display for Idents {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for ident in self.0.iter() {
            write!(f, "{}", ident)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContentToken {
    Text(String),
    Key(Ident, Option<Box::<ContentToken>>),
    Constant(Ident),
    Option(Box::<ContentToken>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

impl AsRef<str> for Ident {
    fn as_ref<'a>(&'a self) -> &'a str {
        &self.0
    }
}

impl std::fmt::Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Content(String);

impl Content {
    pub fn new(s: &str) -> Self {
        Self(s.to_owned())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl AsRef<str> for Content {
    fn as_ref<'a>(&'a self) -> &'a str {
        &self.0
    }
}
