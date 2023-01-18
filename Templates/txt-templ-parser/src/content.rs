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

/// Map identifiers to content
type IdentMap<C> = HashMap<Ident, C>;
// Map content type co map of content
type TypeMap<T> = HashMap<ContentType, T>;

// TODO: Introduce validity checking to `Ident` (maybe)
// `parse::ident` is used any time a user input is converted
// into an identifier. This mean the only time `Ident` could be
// invalid is when setting an invalid identifier internally.
type Ident = String;
type Content = String;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UserContentState {
    pub constants: IdentMap<Content>,
    pub options: IdentMap<IdentMap<Content>>,
}

impl UserContentState {
    pub fn new() -> Self {
        Self {
            constants: IdentMap::new(),
            options: IdentMap::new(),
        }
    }
    pub fn map_constant(&mut self, ident: &str, content: &str) {
        self.constants.insert(Ident::from(ident), Content::from(content));
    }

    /// Use this  method in combination with `new_choice`: `map_option("opt-name", new_choice("choice", "content"))`
    pub fn map_option(
        &mut self,
        option: &str, 
        choice: (Ident, Content)
    ) {
        let option = Ident::from(option);
        let (ident, content) = choice;
        match self.options.get_mut(&option) {
            Some(choices) => { choices.insert(ident, content); },
            None => {
                let mut choices = IdentMap::new();
                choices.insert(ident, content);
                self.options.insert(option, choices);
            },
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UserContent {
    pub keys: IdentMap<Content>,
    pub choices: IdentMap<Ident>,
}

impl UserContent {
    pub fn new() -> Self {
        Self {
            keys: IdentMap::new(),
            choices: IdentMap::new(),
        }
    }

    pub fn map_key(&mut self, ident: &str, content: &str) {
        self.keys.insert(Ident::from(ident), Content::from(content));
    }

    pub fn map_choice(&mut self, option: &str, choice: &str) {
        self.choices.insert(Ident::from(option), Ident::from(choice));
    }
}

pub fn new_choice(ident: &str, content: &str) -> (Ident, Content) {
    (Ident::from(ident), Content::from(content))
}

// Type containing ALL required content to  fill out a template
#[derive(Debug)]
pub struct FullContent(TypeMap<IdentMap<Content>>);

impl FullContent {
    pub fn get(&self, idx: ContentIndex) -> &Content {
        match self.0.get(&idx.0) {
            Some(type_entries) => match type_entries.get(&idx.1) {
                Some(entry) => entry,
                None => panic!("Content was missing a requested entry {:?}", idx),
            },
            None => panic!("Content was missing a requrest entry type {:?}", idx),
        }
    }
}

// Required content
#[derive(Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ReqContent {
    Literal(Content),  //  Either a piece of content
    Default(ContentIndex),  // Or a reference to another piece of content
    None,
}

// Map of all required tokens
// This struct directly maps identifers to chosen content values
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", serde_with::serde_as)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RequiredContent(
    #[cfg_attr(feature = "serde", serde_as(as = "Vec<(_, _)>"))]
    TypeMap<IdentMap<ReqContent>>,
);

impl RequiredContent {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, idx: &ContentIndex, content: ReqContent) {
        match self.0.get_mut(&idx.0) {
            Some(idents) => {
                idents.insert(idx.1.clone(), content);
            },
            None => {
                let mut map: HashMap<Ident, ReqContent> = HashMap::new();
                map.insert(idx.1.clone(), content);
                self.0.insert(idx.0, map);
            },
        };
    }

    pub fn add_constants(&mut self, mut constants: IdentMap<Content>) {
        if let Some(entries) = self.0.get_mut(&ContentType::Constant) {
            // Move every piece of content for each required identifier into
            // the required constant entries.
            for (ident, value) in entries {
                if let Some(constant) = constants.remove(ident) {
                    *value = ReqContent::Literal(constant);
                }
            }
        }
    }

    pub fn add_options(&mut self, choices: IdentMap<Ident>, mut options: IdentMap<IdentMap<Content>>) {
        if let Some(entries) = self.0.get_mut(&ContentType::Option) {
            // Move every chosen piece of content for each required identifier into the
            // required constant entries.
            for (ident, value) in entries {
                // Get the option for the current identifier
                let option = match options.get_mut(ident) {
                    Some(option) => option,
                    None => continue,
                };
                // Get the choosen option
                if let Some(choice) = choices.get(ident) {
                    // The the content assoicates with the choice and move
                    // it into the required optin entries under the identifier
                    // for the option itself
                    if let Some(content) = option.remove(choice) {
                        *value = ReqContent::Literal(content);
                    }
                }
            }
        }
    }

    pub fn add_keys(&mut self, mut keys: IdentMap<Content>) {
        if let Some(entries) = self.0.get_mut(&ContentType::Key) {
            // Mov every piece of content for each required key
            // into the required key entries.
            for (ident, value) in entries {
                if let Some(key) = keys.remove(ident) {
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
            idx: ContentIndex,  // ContentIndex of current element; always passing this is kinda a waste
            content: &ReqContent,
            map: &TypeMap<IdentMap<ReqContent>>,
        ) -> Result<Content, FillOutError> {
            match content {
                ReqContent::None => {
                    return Err(FillOutError::MissingElement(idx));
                }
                ReqContent::Literal(its_lit) => {
                    let its_lit = its_lit.clone();
                    if its_lit.is_empty() {
                        return Err(FillOutError::EmptyContent(idx));
                    } else {
                        return Ok(its_lit);  // <- only ok path is literal content
                    }
                },
                ReqContent::Default(default_idx) => {
                    let default_idx = ContentIndex::new(default_idx.1.as_ref(), default_idx.0);
                    let content_opt = match map.get(&default_idx.0) {
                        Some(entries) => entries.get(&default_idx.1),
                        None => return Err(FillOutError::MissingDefaultType(default_idx)),
                    };

                    match content_opt {
                        Some(content) => return validate_content(default_idx, &content, map),
                        None => return Err(FillOutError::MissingDefault(default_idx)),
                    }
                },
            }
        }

        let mut full_content = HashMap::new(); 
        
        for (token_type, entries) in &self.0 {
            let mut full_type = HashMap::new();
            for (ident, content) in entries {
                match validate_content(ContentIndex::new(ident.as_ref(), *token_type), content, &self.0) {
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
pub struct ContentIndex(ContentType, Ident);

impl ContentIndex {
    pub fn new(ident: &str, content_type: ContentType) -> Self {
        Self(content_type, Ident::from(ident))
    }
}

impl std::fmt::Display for ContentIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Type: {}, Ident: {}", self.0, self.1)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ContentType {
    Key,
    Constant,
    Option,
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ContentType::Key => write!(f, "Key"),
            ContentType::Constant => write!(f, "Constant"),
            ContentType::Option => write!(f, "Option"),
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
                    let content = content.get(ContentIndex::new(ident.as_ref(), ContentType::Constant));
                    output.push_str(content.as_ref());
                },
                ContentToken::Key(ident, _) => {
                    output.push_str(
                        content.get(ContentIndex::new(ident.as_ref(), ContentType::Key)).as_ref()
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
                        content.get(ContentIndex::new(ident.as_ref(), ContentType::Option)).as_ref()
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
                ContentToken::Text(text) => ReqContent::Literal(Content::from(text)),
                ContentToken::Constant(ident) => {
                    let token_idx = ContentIndex::new(ident.as_ref(), ContentType::Constant);
                    map.insert(&token_idx, ReqContent::None);
                    ReqContent::Default(token_idx)
                },
                ContentToken::Key(ident, default) => {
                    let token_idx = ContentIndex::new(ident.as_ref(), ContentType::Key);
                    match default {
                        Some(default_box) => {
                            let default = draft_token(&**default_box, map);
                            map.insert(&token_idx, default);
                        },
                        None => map.insert(&token_idx, ReqContent::None),
                    }
                    ReqContent::Default(token_idx)
                },
                ContentToken::Option(key_box) => {
                    // Extract the key box from the option
                    let (ident, default) = match &**key_box {
                        ContentToken::Key(ident, default) => (ident, default),
                        _ => panic!("ContentToken::Option did not contain a ContentToken::Key \
                            instance. `parse::option` should not allow this!"),
                    };

                    let token_idx = ContentIndex::new(ident.as_ref(), ContentType::Option);
                    match default {
                        Some(default_box) => {
                            let default = draft_token(&**default_box, map);
                            map.insert(&token_idx, default);
                        },
                        None => map.insert(&token_idx, ReqContent::None),
                    }
                    ReqContent::Default(token_idx)
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
    MissingElement(ContentIndex),
    #[error("The given content for the entry {0} is empty")]
    EmptyContent(ContentIndex),
    #[error("The type of a requested default {0} does not exist")]
    MissingDefaultType(ContentIndex),
    #[error("The identifier of a requested default {0} does not exitst")]
    MissingDefault(ContentIndex),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContentToken {
    Text(String),
    Key(Ident, Option<Box::<ContentToken>>),
    Constant(Ident),
    Option(Box::<ContentToken>),
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_works() {
        let variants = vec![
            ("a {name} b $Bye".parse::<ContentTokens>().unwrap(), vec![
                (ContentIndex::new("name", ContentType::Key), ReqContent::None),
                (ContentIndex::new("Bye", ContentType::Constant), ReqContent::None),
            ]),
            ("{other:{othername:Leto}}".parse::<ContentTokens>().unwrap(), vec![
                (ContentIndex::new("other", ContentType::Key), ReqContent::Default(ContentIndex::new("othername", ContentType::Key))),
                (ContentIndex::new("othername", ContentType::Key), ReqContent::Literal("Leto".into())),
            ]),
        ];
        for (tokens, pairs) in variants {
            let expected = helper::content_map_from_vec(pairs);
            let output = tokens.draft();
            assert_eq!(expected, output);
        }
    }

    #[test]
    fn templates_are_parsed_correctly() {
        // Lenghts of literal text and idents in decreased so tests are more consice
        // Other tests assert that any idents/text passes
        let pairs = vec![
            ("fr-FR\n{key}$Constant${Option}", vec![
                ContentToken::Key(Ident::from("key"), None),
                ContentToken::Constant(Ident::from("Constant")),
                ContentToken::Option(Box::new(ContentToken::Key(Ident::from("Option"), None))),
            ], Some("fr-FR")),
            ("S ${Anrede} {name}\n{n}\n$M\n$S", vec![
                ContentToken::Text("S ".into()),
                ContentToken::Option(Box::new(ContentToken::Key(Ident::from("Anrede"), None))),
                ContentToken::Text(" ".into()),
                ContentToken::Key(Ident::from("name"), None),
                ContentToken::Text("\n".into()),
                ContentToken::Key(Ident::from("n"), None),
                ContentToken::Text("\n".into()),
                ContentToken::Constant(Ident::from("M")),
                ContentToken::Text("\n".into()),
                ContentToken::Constant(Ident::from("S")),
            ], None),
            ("Sehr geehrte Frau {name}\n{nachricht}\nMit freundlichen Grüßen\nBar", vec![
                ContentToken::Text("Sehr geehrte Frau ".into()),
                ContentToken::Key(Ident::from("name"), None),
                ContentToken::Text("\n".into()),
                ContentToken::Key(Ident::from("nachricht"), None),
                ContentToken::Text("\nMit freundlichen Grüßen\nBar".into()),
            ], None),
            ("{name:Peter} bla ${bye:{mfg:MfG}}", vec![
                ContentToken::Key(Ident::from("name"), Some(Box::new(ContentToken::Text("Peter".into())))),
                ContentToken::Text(" bla ".into()),
                ContentToken::Option(Box::new(
                    ContentToken::Key(Ident::from("bye"), Some(Box::new(
                        ContentToken::Key(Ident::from("mfg"), Some(Box::new(
                            ContentToken::Text("MfG".into())   
                        )))   
                    )))
                ))
            ], None)
        ];
        for (template, tokens, locale_str) in pairs {
            let result: ContentTokens = template.parse().unwrap();
            if let Some(locale_str) = locale_str {
                let locale: Locale = locale_str.parse().unwrap();
                assert_eq!(*result.locale_ref(), locale);
            }
            for (idx, token) in result.tokens_ref().iter().enumerate() {
                assert_eq!(token, tokens.get(idx).unwrap());
            }
        }
    }


    mod helper {
        use super::*;

        pub fn content_map_from_vec(v: Vec<(ContentIndex, ReqContent)>) -> RequiredContent {
            let mut map = RequiredContent::new();
            for (idx, value) in v {
                map.insert(&idx, value);
            }
            map
        }
    }
}
