use crate::scan::{Scanner, ScanError, Action};
use crate::content::{ContentTokens, ContentToken, Ident};
use log::debug;
use unic_locale::Locale;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

// template ::= <locale>? <item>+
pub fn template(scanner: &mut Scanner) -> Result<ContentTokens, UserError> {
    debug!("Starting template");

    let mut tokens = match locale(scanner) {
        Ok(locale) => ContentTokens::from(locale),
        Err(e) => {
            let mut tokens = ContentTokens::new();
            tokens.add_friendly(e);
            tokens
        },
    };
    
    let e = loop {
        match item(scanner) {
            Ok(token) => tokens.push(token),
            Err(e) => break e,
        }
    };

    if tokens.len() > 0 && scanner.at_end() {
        Ok(tokens)
    } else {
        Err(e)
    }
}

// <item> ::= <key> | <option> | <constant> | <text>
pub fn item(scanner: &mut Scanner) -> Result<ContentToken, UserError> {
    scanner.begin();
    let sequence = scanner.scan_str(|sequence| match sequence {
        "${" => Some(Action::Return),
        "$" => Some(Action::Require('{')),
        "{" => Some(Action::Return),
        _ => Some(Action::Return),
    });
    scanner.abort();

    debug!("Sequence: {:?}", &sequence);

    match sequence {
        Ok(sequence) => {
            match sequence.as_str() {
                "${" => match option(scanner) {
                    Ok(token) => Ok(token),
                    Err(e) => Err(e),
                },
                "$" => match constant(scanner) {
                    Ok(token) => Ok(token),
                    Err(e) => Err(e),
                },
                "{" => match key(scanner) {
                    Ok(token) => Ok(token),
                    Err(e) => Err(e),
                },
                _ => match text(scanner) {
                    Ok(text) => Ok(ContentToken::Text(text)),
                    Err(e) => Err(e),
                }
            }
        },
        Err(e) => Err(UserError {
                parse_error: ParseError::LexicalError(e),
                context: ContextMsg::EmptyInput,
                possible: PossibleMsg::None,
        }),
    }
}

pub fn locale(scanner: &mut Scanner) -> Result<Locale, UserError> {
    debug!("Starting locale");
    scanner.begin();
    // Falls es eine locale gibt, muss sie valide sein. Sonst nicht

    let input = match chars(scanner) {
        Ok(chars) => chars,
        Err(e) => {
            debug!("Didn't find potential locale");
            scanner.abort();
            return Err(e);
        }
    };
    let locale: Locale = match input.parse() {
        Ok(locale) => locale,
        Err(e) => {
            debug!("Found locale is invalid");
            scanner.abort();
            let e = UserError {
                parse_error: ParseError::LocaleError(Box::new(e)),
                context: ContextMsg::InvalidContainedIn("locale".to_owned()),
                possible: PossibleMsg::None,
            };
            return Err(e);
        }
    };
    if let Err(e) = scanner.take(Terminals::Nl) {
        debug!("Failed to finish locale (Missing '\\n')");
        let e = UserError {
            parse_error: ParseError::LexicalError(e),
            context: ContextMsg::InvalidClosingOf("locale".to_owned()),
            possible: PossibleMsg::DidYouForget("a new line after the locale".to_owned()),
        };
        return Err(e);
    }
    scanner.commit();
    debug!("Successfully finished locale");
    Ok(locale)
}

// <text> ::= (<chars> | <ws>)+
// <ws>   ::= (" " | "\t" | "\n")+
// <chars> ::= ([A-Z] | [a-z])+
pub fn text(scanner: &mut Scanner) -> Result<String, UserError> {
    debug!("Starting text");
    scanner.begin();

    let text = match scanner.scan(|symbol| match symbol {
        any if !any.is_terminal() => Some(Action::Request),
        _ => None,
    }) {
        Ok(text) => text,
        Err(e) => {
            debug!("Failed to finish text ");
            let e = UserError {
                parse_error: ParseError::LexicalError(e),
                context: ContextMsg::InvalidContainedIn("text section".to_owned()),
                possible: PossibleMsg::ForbiddenAre("'{', '}' or '$'".to_owned()),
            };
            return Err(e);
        },
    };
    scanner.commit();
    debug!("Successfully finished text");
    Ok(text)
}

// <chars> ::= *any characters except for the terminals and whitespace*
pub fn chars(scanner: &mut Scanner) -> Result<String, UserError> {
    debug!("Starting characters");
    scanner.begin();

    let chars = match scanner.scan(|symbol| match symbol {
        any if any.is_whitespace() => None,
        any if !any.is_terminal() => Some(Action::Request),
        _ => None,
    }) {
        Ok(chars) => chars,
        Err(e) => {
            debug!("Failed to finish chars");
            let e = UserError {
                parse_error: ParseError::LexicalError(e),
                context: ContextMsg::InvalidContainedIn("characters section".to_owned()),
                possible: PossibleMsg::ForbiddenAre("'{', '}', '$' or whitespace characters".to_owned()),
            };
            return Err(e);
        },
    };
    scanner.commit();
    debug!("Successfully finished chars");
    Ok(chars)
}

// key ::= "{" <ident> "}"
pub fn key(scanner: &mut Scanner) -> Result<ContentToken, UserError> {
    debug!("Starting key");
    scanner.begin();
    if let Err(e) = scanner.take(Terminals::LBrace) {
        debug!("Failed to finish key (Missing LBrace)");
        let e = UserError {
            parse_error: ParseError::LexicalError(e),
            context: ContextMsg::InvalidOpeningOf("key".to_owned()),
            possible: PossibleMsg::DidYouMean("{".to_owned()),
        };
        return Err(e);
    }
    let ident = match ident(scanner) {
        Ok(ident) => ident,
        Err(e) => {
            debug!("Failed to finish key (incorrect ident)");
            let e = UserError {
                parse_error: e,
                context: ContextMsg::InvalidContainedIn("identifier of key".to_owned()),
                possible: PossibleMsg::AllowedAre("'A'-'Z', 'a'-'z' and '0'-'9'".to_owned()),
            };
            return Err(e);
        },
    };
    let default = match default(scanner) {
        Ok(default) => if let Some(token) = default {
            Some(Box::new(token))
        } else {
            None
        },
        Err(e) => {
            debug!("Failed to finish key (incorrect default)");
            return Err(e);
        },
    };
    if let Err(e) = scanner.take(Terminals::RBrace) {
        debug!("Failed to finish key (Missing RBrace)");
        let e = UserError {
            parse_error: ParseError::LexicalError(e),
            context: ContextMsg::InvalidClosingOf("key".to_owned()),
            possible: PossibleMsg::DidYouMean("}".to_owned()),
        };
        return Err(e);
    }
    scanner.commit();
    debug!("Successfully finished key");
    Ok(ContentToken::Key(ident, default))
}

// <default> ::= ":" <item>
pub fn default(scanner: &mut Scanner) -> Result<Option<ContentToken>, UserError> {
    debug!("Starting default");
    scanner.begin();
    if let Err(_) = scanner.take(Terminals::Colon) {
        debug!("Failed to finish default (Missing colon)");
        return Ok(None);
    }
    let token = match item(scanner) {
        Ok(token) => token,
        Err(mut e) => {
            debug!("Failed to finish default (incorrect item)");
            e.context = ContextMsg::InvalidContainedIn("default for key".to_owned());
            return Err(e);
        },
    };
    scanner.commit();
    debug!("Successfully finished default");
    Ok(Some(token))
}

// <ident> ::= (<char> | [0-9])+
// <char> ::= ([A-Z] | [a-z])   
pub fn ident(scanner: &mut Scanner) -> Result<Ident, ParseError> {
    debug!("Starting ident");
    let ident = match scanner.scan(|symbol| match symbol as u8 {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' => Some(Action::Request),
        _ => None,
    }) {
        Ok(ident) => ident,
        Err(e) => {
            debug!("Failed to finish ident");
            return Err(ParseError::LexicalError(e));
        }
    };
    debug!("Successfully finished ident");
    Ok(Ident::from(ident))  // at some point return the ident itself instead
}

// <option> ::= "$" <key>
pub fn option(scanner: &mut Scanner) -> Result<ContentToken, UserError> {
    debug!("Starting options");
    scanner.begin();
    if let Err(e) = scanner.take(Terminals::Cash) {
        debug!("Failed to finish options (Missing Cash)");
        let e = UserError {
            parse_error: ParseError::LexicalError(e),
            context: ContextMsg::InvalidOpeningOf("option".to_owned()),
            possible: PossibleMsg::DidYouMean("$".to_owned()),
        };
        return Err(e);
    }
    let key = match key(scanner) {
        Ok(ident) => ident,
        Err(mut e) => {
            debug!("Failed to finish options (incorrect ident)");
            e.context = ContextMsg::InvalidContainedIn("identifier of option".to_owned());
            return Err(e);
        },
    };
    scanner.commit();
    debug!("Successfully finished option");
    Ok(ContentToken::Option(Box::new(key)))
}

// <constant> ::= "$" <ident>
pub fn constant(scanner: &mut Scanner) -> Result<ContentToken, UserError> {
    debug!("Starting constant");
    debug!("Scanner is at: {}", scanner.current_char().unwrap());
    scanner.begin();
    if let Err(e) = scanner.take(Terminals::Cash) {
        debug!("Failed to finish constant (Missing Cash)");
        let e = UserError {
            parse_error: ParseError::LexicalError(e),
            context: ContextMsg::InvalidOpeningOf("constant".to_owned()),
            possible: PossibleMsg::DidYouMean("$".to_owned()),
        };
        return Err(e);
    }
    let ident = match ident(scanner) {
        Ok(ident) => ident,
        Err(e) => {
            debug!("Failed to finish constant (incorrect ident)");
            let e = UserError {
                parse_error: e,
                context: ContextMsg::InvalidContainedIn("identifer of constant".to_owned()),
                possible: PossibleMsg::AllowedAre("'A'-'Z', 'a'-'z' and '0'-'9'".to_owned()),
            };
            return Err(e);
        }
    };
    scanner.commit();
    debug!("Successfully finished constant");
    Ok(ContentToken::Constant(ident))    
}

// Terminal-symbol representation
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Terminals {
    LBrace = b'{',
    RBrace = b'}',
    Cash = b'$',
    Colon = b':',
    Nl = b'\n',
}

// Trait which can be implementend on any potential terminal or non-terminal symbol
pub trait Symbol {
    fn is_terminal(&self) -> bool;
    fn is_non_terminal(&self) -> bool {
        !self.is_terminal()
    }
}

impl Symbol for char {
    fn is_terminal(&self) -> bool {
        match self {
            // Don't consider ':' here because ':' is only a terminal
            // in the context of a key
            '{' | '}' | '$'=> true,
            _ => false,
        }
    }
}


#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UserError {
    parse_error: ParseError,
    context: ContextMsg,
    possible: PossibleMsg,  // Info on the possible characters
}

impl std::error::Error for UserError {}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}\n{}", self.context, self.parse_error, self.possible)
    }
}

impl From<ParseError> for UserError {
    fn from(parse_error: ParseError) -> Self {
        Self {
            parse_error,
            context: ContextMsg::None,
            possible: PossibleMsg::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
enum ContextMsg {
    InvalidContainedIn(String),  // Invalid  character(s) conatined in {identifier for key}
    InvalidOpeningOf(String),  // Invalid opening character of {key}
    InvalidClosingOf(String),  // Invalid closing character of {key}
    EmptyInput,
    None,
}

impl std::fmt::Display for ContextMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ContextMsg::InvalidContainedIn(target) => {
                write!(f, "Found invalid character(s) contained in {}", target)
            },
            ContextMsg::InvalidOpeningOf(target) => {
                write!(f, "Found invalid opening character for {}", target)
            },
            ContextMsg::InvalidClosingOf(target) => {
                write!(f, "Found invalid closing character for {}", target)
            },
            ContextMsg::EmptyInput => {
                write!(f, "Cannot process an empty input")
            }
            ContextMsg::None => {
                write!(f, "")
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
enum PossibleMsg {
    DidYouMean(String),
    DidYouForget(String),
    AllowedAre(String),
    ForbiddenAre(String),
    None,
}

impl std::fmt::Display for PossibleMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PossibleMsg::DidYouMean(maybe) => {
                write!(f, "Did you maybe mean '{}'?", maybe)
            },
            PossibleMsg::DidYouForget(maybe) => {
                write!(f, "Did you maybe forget {}?", maybe)
            }
            PossibleMsg::AllowedAre(allowed) => {
                write!(f, "Allowed characters are {}", allowed)
            },
            PossibleMsg::ForbiddenAre(forbidden) => {
                write!(f, "Forbidden characters are {}", forbidden)
            },
            PossibleMsg::None => {
                write!(f, "")
            },
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ParseError {
    #[error(transparent)]
    LexicalError(#[from] ScanError),
    #[error("Locale Error")]
    #[serde(skip_serializing, skip_deserializing)]
    LocaleError(#[source] Box<dyn std::error::Error>),    
}

