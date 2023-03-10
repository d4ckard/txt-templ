#[cfg(feature = "dyn")]
mod meta;
mod parse;
mod scan;
#[cfg(feature = "dyn")]
use meta::MetaExt;
pub use parse::UserError;
use scan::Scanner;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use unic_locale::{locale, Locale};

/// Map identifiers to content
type IdentMap<C> = HashMap<Ident, C>;
// Map content type co map of content
type TypeMap<T> = HashMap<ContentType, T>;

type Ident = String;
type Content = String;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ContentState {
    // Map of constant identifiers to literal content
    pub constants: IdentMap<Content>,
    // Map of option identifiers to choice identifiers to literal content
    pub options: IdentMap<IdentMap<Content>>,
}

impl ContentState {
    pub fn new() -> Self {
        Self {
            constants: IdentMap::new(),
            options: IdentMap::new(),
        }
    }
    pub fn map_constant(&mut self, ident: &str, content: &str) {
        self.constants
            .insert(Ident::from(ident), Content::from(content));
    }

    /// Use this  method in combination with `choice!`: `map_option("opt-name", choice!("choice", "content"))`
    pub fn map_option(&mut self, option: &str, choice: (Ident, Content)) {
        let option = Ident::from(option);
        let (ident, content) = choice;
        match self.options.get_mut(&option) {
            Some(choices) => {
                choices.insert(ident, content);
            }
            None => {
                let mut choices = IdentMap::new();
                choices.insert(ident, content);
                self.options.insert(option, choices);
            }
        }
    }
}

impl Default for ContentState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VolatileContent {
    pub keys: IdentMap<Content>, // Map of key identifiers to content literals
    pub choices: IdentMap<Ident>, // Map of option identifiers to choice identifers
}

impl VolatileContent {
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
        self.choices
            .insert(Ident::from(option), Ident::from(choice));
    }
}

impl Default for VolatileContent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
macro_rules! choice {
    ( $x:expr, $y:expr ) => {{
        (String::from($x), String::from($y))
    }};
}
#[cfg(test)]
pub(crate) use choice;

// Type containing ALL required content to  fill out a template
#[derive(Debug)]
pub struct FullContent(TypeMap<IdentMap<Content>>);

impl FullContent {
    pub fn get(&self, idx: ContentIndex) -> &Content {
        &self.0[&idx.0][&idx.1]
    }
}

// Required content
#[derive(Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ContentRequirement {
    Literal(Content),      //  Either a piece of content
    Default(ContentIndex), // Or a reference to another piece of content
    None,
}

// Map of all required tokens
// This struct directly maps identifers to chosen content values
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", serde_with::serde_as)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RequiredContent(
    #[cfg_attr(feature = "serde", serde_as(as = "Vec<(_, _)>"))]
    TypeMap<IdentMap<ContentRequirement>>,
);

impl RequiredContent {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, idx: &ContentIndex, content: ContentRequirement) {
        match self.0.get_mut(&idx.0) {
            Some(idents) => {
                idents.insert(idx.1.clone(), content);
            }
            None => {
                let mut map: HashMap<Ident, ContentRequirement> = HashMap::new();
                map.insert(idx.1.clone(), content);
                self.0.insert(idx.0, map);
            }
        };
    }

    pub fn add_constants(&mut self, mut constants: IdentMap<Content>) {
        if let Some(entries) = self.0.get_mut(&ContentType::Constant) {
            // Move every piece of content for each required identifier into
            // the required constant entries.
            for (ident, value) in entries {
                if let Some(constant) = constants.remove(ident) {
                    *value = ContentRequirement::Literal(constant);
                }
            }
        }
    }

    pub fn add_options(
        &mut self,
        choices: IdentMap<Ident>,
        mut options: IdentMap<IdentMap<Content>>,
    ) {
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
                        *value = ContentRequirement::Literal(content);
                    }
                }
            }
        }
    }

    pub fn add_keys(&mut self, mut keys: IdentMap<Content>) {
        if let Some(entries) = self.0.get_mut(&ContentType::Key) {
            // Move every piece of content for each required key
            // into the required key entries.
            for (ident, value) in entries {
                if let Some(key) = keys.remove(ident) {
                    *value = ContentRequirement::Literal(key);
                }
            }
        }
    }

    /// Evaluate all dynamic elements in `self`. At the current stage
    /// this method will evaluate all meta constants.
    /// This method will not do anything, if dynamic elements are diabled
    /// through the "dyn" flag.
    pub fn eval_dyn(&mut self) {
        #[cfg(feature = "dyn")]
        if let Some(required) = self.0.get_mut(&ContentType::Constant) {
            // For each entry, check if the entrie's identifier is the
            // identifier of a meta element. If it is, replace the current
            // content with the content of the meta element.
            for (ident, content_value) in required {
                if let Some(meta) = ident.as_meta() {
                    // Fill in the meta value.
                    let meta_content = Content::from(meta);
                    *content_value = ContentRequirement::Literal(meta_content);
                }
            }
        }
    }

    // Return an instance of volatile content which contains all required entires
    // and their respective content literals if there are some.
    pub fn draft_volatile_content(&self) -> VolatileContent {
        // Find the literal associated with the `ContentRequirement` instance or return an empty string
        fn get_literal(
            content: &ContentRequirement,
            map: &TypeMap<IdentMap<ContentRequirement>>,
        ) -> String {
            match content {
                ContentRequirement::None => "".to_owned(),
                ContentRequirement::Literal(its_lit) => its_lit.clone(),
                ContentRequirement::Default(default_idx) => {
                    // Here we need ownership of `default_idx` without moving it.
                    let default_idx = default_idx.clone();

                    // Get the identifier map for the type of the default.
                    match map.get(&default_idx.0) {
                        None => "".to_owned(),
                        // Get the content entry for the identifier of the default.
                        Some(ident_map) => match ident_map.get(&default_idx.1) {
                            None => "".to_owned(),
                            Some(content) => get_literal(content, map),
                        },
                    }
                }
            }
        }

        let mut vc = VolatileContent::new();
        // Add all key entries
        if let Some(key_entries) = self.0.get(&ContentType::Key) {
            for (ident, content) in key_entries {
                vc.map_key(ident, &get_literal(content, &self.0));
            }
        }
        // Add all choice entires
        if let Some(option_entries) = self.0.get(&ContentType::Option) {
            for (ident, content) in option_entries {
                vc.map_choice(ident, &get_literal(content, &self.0));
            }
        }

        vc
    }
}

impl Default for RequiredContent {
    fn default() -> Self {
        Self::new()
    }
}

impl TryInto<FullContent> for RequiredContent {
    type Error = FillOutError;

    fn try_into(self) -> Result<FullContent, Self::Error> {
        fn validate_content(
            idx: ContentIndex, // ContentIndex of current element; always passing this is kinda a waste
            content: &ContentRequirement,
            map: &TypeMap<IdentMap<ContentRequirement>>,
        ) -> Result<Content, FillOutError> {
            match content {
                ContentRequirement::None => Err(FillOutError::MissingElement(idx)),
                ContentRequirement::Literal(its_lit) => {
                    let its_lit = its_lit.clone();
                    match its_lit.is_empty() {
                        true => Err(FillOutError::EmptyContent(idx)),
                        false => Ok(its_lit), // <- only `Ok` path is returning a literal
                    }
                }
                ContentRequirement::Default(default_idx) => {
                    let default_idx = default_idx.clone();
                    let content_opt = match map.get(&default_idx.0) {
                        Some(entries) => entries.get(&default_idx.1),
                        None => return Err(FillOutError::MissingDefaultType(default_idx)),
                    };

                    match content_opt {
                        Some(content) => validate_content(default_idx, content, map),
                        None => Err(FillOutError::MissingDefault(default_idx)),
                    }
                }
            }
        }

        let mut full_content = HashMap::new();

        for (token_type, entries) in &self.0 {
            let mut full_type = HashMap::new();
            for (ident, content) in entries {
                let idx = ContentIndex::new(*token_type, ident);

                match validate_content(idx, content, &self.0) {
                    Ok(content) => full_type.insert(ident.clone(), content),
                    Err(e) => return Err(e),
                };
            }
            full_content.insert(*token_type, full_type);
        }

        Ok(FullContent(full_content))
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ContentIndex(ContentType, Ident);

impl ContentIndex {
    pub fn new(content_type: ContentType, ident: &str) -> Self {
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
            Self::Key => write!(f, "Key"),
            Self::Constant => write!(f, "Constant"),
            Self::Option => write!(f, "Option"),
        }
    }
}

#[derive(Debug)]
pub struct ContentTokens {
    tokens: Vec<ContentToken>,
    pub locale: Locale,
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

    pub const fn from(locale: Locale) -> Self {
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

    // Use the content map to substitue all values in `tokens` until
    // the entire template has been filled out.
    pub fn fill_out(&self, content: FullContent) -> String {
        let mut output = String::new();

        // Try to add the content for `token` to `output`
        fn fill_out_token(token: &ContentToken, content: &FullContent, output: &mut String) {
            match token {
                ContentToken::Text(text) => output.push_str(text),
                ContentToken::Constant(ident) => {
                    let content = content.get(ContentIndex::new(ContentType::Constant, ident));
                    output.push_str(content);
                }
                ContentToken::Key(ident, _) => {
                    output.push_str(content.get(ContentIndex::new(ContentType::Key, ident)));
                }
                ContentToken::Option(key_box) => {
                    let (ident, _) = match &**key_box {
                        ContentToken::Key(ident, default_box) => (ident, default_box),
                        _ => panic!(
                            "ContentToken::Option did not contain a ContentToken::Key instance. \
                            `parse::option` should not allow this!"
                        ),
                    };
                    output.push_str(
                        content.get(ContentIndex::new(ContentType::Option, ident.as_ref())),
                    );
                }
            }
        }

        for token in &self.tokens {
            fill_out_token(token, &content, &mut output);
        }

        output
    }

    // Return a half-empty `RequiredContent` instance containing the identifiers and
    // token-types of all the empty entries in the template
    // If there is a default value for a key or an option which is a text literal,
    // then this literal will be entered into the content table draft under this
    // key or option entry. If the user selects a value for this entry, the default
    // will be overwritten.
    pub fn draft(&self) -> RequiredContent {
        let mut map = RequiredContent::new();

        fn draft_token(token: &ContentToken, map: &mut RequiredContent) -> ContentRequirement {
            match token {
                ContentToken::Text(text) => ContentRequirement::Literal(Content::from(text)),
                ContentToken::Constant(ident) => {
                    let token_idx = ContentIndex::new(ContentType::Constant, ident);
                    map.insert(&token_idx, ContentRequirement::None);
                    ContentRequirement::Default(token_idx)
                }
                ContentToken::Key(ident, default) => {
                    let token_idx = ContentIndex::new(ContentType::Key, ident);
                    match default {
                        Some(default_box) => {
                            let default = draft_token(default_box, map);
                            map.insert(&token_idx, default);
                        }
                        None => map.insert(&token_idx, ContentRequirement::None),
                    }
                    ContentRequirement::Default(token_idx)
                }
                ContentToken::Option(key_box) => {
                    // Extract the key box from the option
                    let (ident, default) = match &**key_box {
                        ContentToken::Key(ident, default) => (ident, default),
                        _ => panic!(
                            "ContentToken::Option did not contain a ContentToken::Key \
                            instance. `parse::option` should not allow this!"
                        ),
                    };

                    let token_idx = ContentIndex::new(ContentType::Option, ident);
                    match default {
                        Some(default_box) => {
                            let default = draft_token(default_box, map);
                            map.insert(&token_idx, default);
                        }
                        None => map.insert(&token_idx, ContentRequirement::None),
                    }
                    ContentRequirement::Default(token_idx)
                }
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
    Key(Ident, Option<Box<ContentToken>>),
    Constant(Ident),
    Option(Box<ContentToken>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_works() {
        let variants = vec![
            (
                "a {name} b $Bye".parse::<ContentTokens>().unwrap(),
                vec![
                    (
                        ContentIndex::new(ContentType::Key, "name"),
                        ContentRequirement::None,
                    ),
                    (
                        ContentIndex::new(ContentType::Constant, "Bye"),
                        ContentRequirement::None,
                    ),
                ],
            ),
            (
                "{other:{othername:Leto}}".parse::<ContentTokens>().unwrap(),
                vec![
                    (
                        ContentIndex::new(ContentType::Key, "other"),
                        ContentRequirement::Default(ContentIndex::new(
                            ContentType::Key,
                            "othername",
                        )),
                    ),
                    (
                        ContentIndex::new(ContentType::Key, "othername"),
                        ContentRequirement::Literal("Leto".into()),
                    ),
                ],
            ),
        ];
        for (tokens, pairs) in variants {
            let expected = helper::content_map_from_vec(pairs);
            let output = tokens.draft();
            assert_eq!(expected, output);
        }
    }

    #[test] // Ensure the `RequiredContent::volatile_content_draft` methods works as expected
    fn volatile_content_drafts_work() {
        let volatile_content_draft = |input: &str| {
            input
                .parse::<ContentTokens>()
                .unwrap()
                .draft()
                .draft_volatile_content()
        };

        {
            // Options and keys are entered into the volatile content instance
            let vc = volatile_content_draft("{key}${option}");
            let mut expected_vc = VolatileContent::new();
            expected_vc.map_key("key", "");
            expected_vc.map_choice("option", "");
            assert_eq!(vc, expected_vc);
        }
        {
            // Defaults are copied into the volatile content instance
            let vc =
                volatile_content_draft("{key:key-default-literal}${option:option-default-literal}");
            let mut expected_vc = VolatileContent::new();
            expected_vc.map_key("key", "key-default-literal");
            expected_vc.map_choice("option", "option-default-literal");
            assert_eq!(vc, expected_vc);
        }
        {
            // Nested key defaults are entered into the volatile content instance
            let vc = volatile_content_draft("{key:{defaultKey:default-literal}}");
            let mut expected_vc = VolatileContent::new();
            expected_vc.map_key("key", "default-literal"); // Default literals are propagated
            expected_vc.map_key("defaultKey", "default-literal");
            assert_eq!(vc, expected_vc);
        }
        {
            // Nested option defaults are entered into the volatile content instance
            let vc = volatile_content_draft("${option:${defaultOption:default-literal}}");
            let mut expected_vc = VolatileContent::new();
            expected_vc.map_choice("option", "default-literal"); // Default literals are propagated
            expected_vc.map_choice("defaultOption", "default-literal");
            assert_eq!(vc, expected_vc);
        }
        {
            // Default constants are skipped/not entered as defaults into the volatile content instance
            let vc = volatile_content_draft("{key:$constant}${option:$constant}");
            let mut expected_vc = VolatileContent::new();
            expected_vc.map_key("key", "");
            expected_vc.map_choice("option", "");
            assert_eq!(vc, expected_vc);
        }
        {
            // Constants and text literals are not entered into the volatile content instance
            let vc = volatile_content_draft("$constant some funny text literal! $anotherConstant");
            assert_eq!(vc, VolatileContent::new());
        }
    }

    #[test]
    fn templates_are_parsed_correctly() {
        // Lenghts of literal text and idents in decreased so tests are more consice
        // Other tests assert that any idents/text passes
        let pairs = vec![
            (
                "locale:fr-FR\n{key}$Constant${Option}",
                vec![
                    ContentToken::Key(Ident::from("key"), None),
                    ContentToken::Constant(Ident::from("Constant")),
                    ContentToken::Option(Box::new(ContentToken::Key(Ident::from("Option"), None))),
                ],
                Some("fr-FR"),
            ),
            (
                "S ${Anrede} {name}\n{n}\n$M\n$S",
                vec![
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
                ],
                None,
            ),
            (
                "Sehr geehrte Frau {name}\n{nachricht}\nMit freundlichen Gr????en\nBar",
                vec![
                    ContentToken::Text("Sehr geehrte Frau ".into()),
                    ContentToken::Key(Ident::from("name"), None),
                    ContentToken::Text("\n".into()),
                    ContentToken::Key(Ident::from("nachricht"), None),
                    ContentToken::Text("\nMit freundlichen Gr????en\nBar".into()),
                ],
                None,
            ),
            (
                "{name:Peter} bla ${bye:{mfg:MfG}}",
                vec![
                    ContentToken::Key(
                        Ident::from("name"),
                        Some(Box::new(ContentToken::Text("Peter".into()))),
                    ),
                    ContentToken::Text(" bla ".into()),
                    ContentToken::Option(Box::new(ContentToken::Key(
                        Ident::from("bye"),
                        Some(Box::new(ContentToken::Key(
                            Ident::from("mfg"),
                            Some(Box::new(ContentToken::Text("MfG".into()))),
                        ))),
                    ))),
                ],
                None,
            ),
        ];
        for (template, tokens, locale_str) in pairs {
            let result: ContentTokens = template.parse().unwrap();
            if let Some(locale_str) = locale_str {
                let locale: Locale = locale_str.parse().unwrap();
                assert_eq!(result.locale, locale);
            }
            for (idx, token) in result.tokens.iter().enumerate() {
                assert_eq!(token, tokens.get(idx).unwrap());
            }
        }
    }

    mod helper {
        use super::*;

        pub fn content_map_from_vec(v: Vec<(ContentIndex, ContentRequirement)>) -> RequiredContent {
            let mut map = RequiredContent::new();
            for (idx, value) in v {
                map.insert(&idx, value);
            }
            map
        }
    }
}
