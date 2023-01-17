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

// Required content
#[derive(Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ReqContent {
    Literal(Content),  //  Either a piece of content
    Default(TokenIdent),  // Or a reference to another piece of content
    // TODO: Rename TokenIdent to ContentIdx
    None,
}

// Map of all required tokens
// This struct directly maps identifers to chosen content values
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", serde_with::serde_as)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RequiredContent(
    #[cfg_attr(feature = "serde", serde_as(as = "Vec<(_, _)>"))]
    HashMap<Token, HashMap<Ident, ReqContent>>,
);

impl RequiredContent {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, token: &TokenIdent, content: ReqContent) {
        match self.0.get_mut(&token.token) {
            Some(idents) => {
                idents.insert(token.ident.clone(), content);
            },
            None => {
                let mut map: HashMap<Ident, ReqContent> = HashMap::new();
                map.insert(token.ident.clone(), content);
                self.0.insert(token.token, map);
            },
        };
    }

    // TODO: Create types for reused *types*

    fn get(&self, token_ident: &TokenIdent) -> Option<&ReqContent> {
        match self.0.get(&token_ident.token) {
            Some(idents) => idents.get(&token_ident.ident),
            None => None,
        }
    }

    pub fn add_constants(&mut self, mut constants: HashMap<Ident, Content>) {
        if let Some(entries) = self.0.get_mut(&Token::Constant) {
            // Move every piece of content for each required identifier into
            // the required constant entries.
            for (ident, value) in entries {
                if let Some(constant) = constants.remove(&ident) {
                    *value = ReqContent::Literal(constant);
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
                        *value = ReqContent::Literal(content);
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
                    *value = ReqContent::Literal(key);
                }
            }
        }
    }
}

impl TryInto<FullContent> for RequiredContent {
    type Error = FillOutError;

    // TODO: This function is really messing with loads of clones! Clean this up.
    // Also maybe make `validate_content` an associated function of `Content`
    fn try_into(self) -> Result<FullContent, Self::Error> {
        fn validate_content(
            token_ident: TokenIdent,  // TokenIdent of current element; always passing this is kinda a waste
            content: &ReqContent,
            map: &HashMap<Token, HashMap<Ident, ReqContent>>,
        ) -> Result<Content, FillOutError> {
            match content {
                ReqContent::None => {
                    return Err(FillOutError::MissingElement(token_ident));
                }
                ReqContent::Literal(its_lit) => {
                    let its_lit = its_lit.clone();
                    if its_lit.is_empty() {
                        return Err(FillOutError::EmptyContent(token_ident));
                    } else {
                        return Ok(its_lit);  // <- only ok path is literal content
                    }
                },
                ReqContent::Default(default_id) => {
                    let default_id = TokenIdent::new(default_id.ident.as_ref(), default_id.token);
                    let content_opt = match map.get(&default_id.token) {
                        Some(entries) => entries.get(&default_id.ident),
                        None => return Err(FillOutError::MissingDefaultType(default_id)),
                    };

                    match content_opt {
                        Some(content) => return validate_content(default_id, &content, map),
                        None => return Err(FillOutError::MissingDefault(default_id)),
                    }
                },
            }
        }

        let mut full_content = HashMap::new(); 
        
        for (token_type, entries) in &self.0 {
            let mut full_type = HashMap::new();
            for (ident, content) in entries {
                match validate_content(TokenIdent::new(ident.as_ref(), *token_type), content, &self.0) {
                    Ok(content) => full_type.insert(ident.clone(), content),
                    Err(e) => return Err(e),
                };
            }
            full_content.insert(*token_type, full_type);
        }

        Ok(FullContent( full_content ))
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

impl std::fmt::Display for TokenIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Ident: {}, Token type: {}", self.ident, self.token)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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

        fn draft_token(token: &ContentToken, map: &mut RequiredContent) -> ReqContent {
            match token {
                ContentToken::Text(text) => ReqContent::Literal(Content::new(text)),
                ContentToken::Constant(ident) => {
                    let token_id = TokenIdent::new(ident.as_ref(), Token::Constant);
                    map.insert(&token_id, ReqContent::None);
                    ReqContent::Default(token_id)
                },
                ContentToken::Key(ident, default) => {
                    let token_id = TokenIdent::new(ident.as_ref(), Token::Key);
                    match default {
                        Some(default_box) => {
                            let default = draft_token(&**default_box, map);
                            map.insert(&token_id, default);
                        },
                        None => map.insert(&token_id, ReqContent::None),
                    }
                    ReqContent::Default(token_id)
                },
                ContentToken::Option(key_box) => {
                    // Extract the key box from the option
                    let (ident, default) = match &**key_box {
                        ContentToken::Key(ident, default) => (ident, default),
                        _ => panic!("ContentToken::Option did not contain a ContentToken::Key \
                            instance. `parse::option` should not allow this!"),
                    };

                    let token_id = TokenIdent::new(ident.as_ref(), Token::Option);
                    match default {
                        Some(default_box) => {
                            let default = draft_token(&**default_box, map);
                            map.insert(&token_id, default);
                        },
                        None => map.insert(&token_id, ReqContent::None),
                    }
                    ReqContent::Default(token_id)
                },
            }
        }

        for token in &self.tokens {
            draft_token(token, &mut map);
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
    #[error("The given content is missing an element {0}")]
    MissingElement(TokenIdent),
    #[error("The given content for the entry {0} is empty")]
    EmptyContent(TokenIdent),
    #[error("The type of a requested default {0} does not exist")]
    MissingDefaultType(TokenIdent),
    #[error("The identifier of a requested default {0} does not exitst")]
    MissingDefault(TokenIdent),
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
        Content(s.to_owned())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Content(s)
    }
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

// TODO: Remove the default  from as ref
impl AsRef<str> for Content {
    fn as_ref<'a>(&'a self) -> &'a str {
        &self.0
    }
}
