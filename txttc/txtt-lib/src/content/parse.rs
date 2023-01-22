use super::scan::{Action, ScanError, Scanner};
use crate::content::{ContentToken, ContentTokens, Ident};
use log::debug;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use unic_locale::Locale;

// template ::= <locale>? <item>+
pub fn template(scanner: &mut Scanner) -> Result<ContentTokens, UserError> {
    debug!("Starting template");

    let mut tokens = match locale(scanner) {
        Ok(locale) => ContentTokens::from(locale),
        Err(e) => {
            let mut tokens = ContentTokens::new();
            tokens.add_friendly(e); // Only raise a waring in case parsing the locale failed.
                                    // The warning is rasied, because one might have tried to
                                    // set the locale but failed. If the error was silent, the
                                    // user would not know about the fact that en-US will be used.
            tokens
        }
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
    let sequence = scanner.scan_seq(|sequence| match sequence {
        "${" => Some(Action::Return),
        "$" => Some(Action::Require('{')),
        "{" => Some(Action::Return),
        _ => Some(Action::Return),
    });
    scanner.abort();

    debug!("Sequence: {:?}", &sequence);

    match sequence {
        Ok(sequence) => match sequence.as_str() {
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
            },
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
    // Locale keyword
    const LOCALE_KEYWORD: &str = "locale";
    if let Err(e) = scanner.take_str(LOCALE_KEYWORD) {
        debug!("Didn't find locale keyword");
        let e = UserError {
            parse_error: ParseError::LexicalError(e),
            context: ContextMsg::InvalidContainedIn(format!("keyword {LOCALE_KEYWORD}")),
            possible: PossibleMsg::DidYouMean(LOCALE_KEYWORD.to_owned()),
        };
        return Err(e);
    }
    let ws = |scanner: &mut Scanner| {
        // Scan any number if whitespace characters. Nothing will
        // happen is none are encountered.
        scanner.begin();
        if scanner
            .scan(|symbol| match symbol {
                any if any.is_whitespace() => Some(Action::Request),
                _ => None,
            })
            .is_ok()
        {
            // The new scan layer at the start of this closure will not
            // be commit or destroyed if the callback was successful.
            // We commit here to continue after the last whitespace character.
            scanner.commit();
        }
    };

    // Colon delimiter with optional whitespace on both sides
    ws(scanner);
    if let Err(e) = scanner.take(':') {
        debug!("Failed to finish locale setting (Missing Colon)");
        let e = UserError {
            parse_error: ParseError::LexicalError(e),
            context: ContextMsg::InvalidContainedIn("locale setting".to_owned()),
            possible: PossibleMsg::DidYouForget(
                "to add a colon between the locale keyword and literal".to_owned(),
            ),
        };
        return Err(e);
    };
    ws(scanner);

    // Locale literal
    let input = match chars(scanner) {
        Ok(chars) => chars,
        Err(e) => {
            debug!("Didn't find potential locale");
            scanner.abort();
            return Err(e);
        }
    };
    let locale = match unic_locale::parser::parse_locale(input) {
        Ok(locale) => locale,
        Err(e) => {
            debug!("Found locale is invalid");
            scanner.abort();
            let e = UserError {
                parse_error: ParseError::LocaleError(e),
                context: ContextMsg::InvalidContainedIn("locale".to_owned()),
                possible: PossibleMsg::None,
            };
            return Err(e);
        }
    };
    // Terminating new-line character
    if let Err(e) = scanner.take('\n') {
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
        }
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
                possible: PossibleMsg::ForbiddenAre(
                    "'{', '}', '$' or whitespace characters".to_owned(),
                ),
            };
            return Err(e);
        }
    };
    scanner.commit();
    debug!("Successfully finished chars");
    Ok(chars)
}

// key ::= "{" <ident> "}"
pub fn key(scanner: &mut Scanner) -> Result<ContentToken, UserError> {
    debug!("Starting key");
    scanner.begin();
    if let Err(e) = scanner.take(Terminals::LBrace.into()) {
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
        }
    };
    let default = match default(scanner) {
        Ok(default) => default.map(Box::new),
        Err(e) => {
            debug!("Failed to finish key (incorrect default)");
            return Err(e);
        }
    };
    if let Err(e) = scanner.take(Terminals::RBrace.into()) {
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
    if scanner.take(':').is_err() {
        debug!("Failed to finish default (Missing colon)");
        return Ok(None);
    }
    let token = match item(scanner) {
        Ok(token) => token,
        Err(mut e) => {
            debug!("Failed to finish default (incorrect item)");
            e.context = ContextMsg::InvalidContainedIn("default for key".to_owned());
            return Err(e);
        }
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
    Ok(ident) // No `Ident::from` required because `Ident` is the same as `String`
}

// <option> ::= "$" <key>
pub fn option(scanner: &mut Scanner) -> Result<ContentToken, UserError> {
    debug!("Starting options");
    scanner.begin();
    if let Err(e) = scanner.take(Terminals::Cash.into()) {
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
        }
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
    if let Err(e) = scanner.take(Terminals::Cash.into()) {
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
}

impl From<Terminals> for char {
    fn from(from: Terminals) -> Self {
        from as u8 as Self
    }
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
        matches!(self, '{' | '}' | '$')
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UserError {
    parse_error: ParseError,
    context: ContextMsg,
    possible: PossibleMsg, // Info on the possible characters
}

impl std::error::Error for UserError {}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}: {}\n{}",
            self.context, self.parse_error, self.possible
        )
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
    InvalidContainedIn(String), // Invalid  character(s) conatined in {identifier for key}
    InvalidOpeningOf(String),   // Invalid opening character of {key}
    InvalidClosingOf(String),   // Invalid closing character of {key}
    EmptyInput,
    None,
}

impl std::fmt::Display for ContextMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidContainedIn(target) => {
                write!(f, "Found invalid character(s) contained in {target}")
            }
            Self::InvalidOpeningOf(target) => {
                write!(f, "Found invalid opening character for {target}")
            }
            Self::InvalidClosingOf(target) => {
                write!(f, "Found invalid closing character for {target}")
            }
            Self::EmptyInput => {
                write!(f, "Cannot process an empty input")
            }
            Self::None => {
                write!(f, "")
            }
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
            Self::DidYouMean(maybe) => {
                write!(f, "Did you maybe mean '{maybe}'?")
            }
            Self::DidYouForget(maybe) => {
                write!(f, "Did you maybe forget {maybe}?")
            }
            Self::AllowedAre(allowed) => {
                write!(f, "Allowed characters are {allowed}")
            }
            Self::ForbiddenAre(forbidden) => {
                write!(f, "Forbidden characters are {forbidden}")
            }
            Self::None => {
                write!(f, "")
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ParseError {
    #[error(transparent)]
    LexicalError(#[from] ScanError),
    #[error(transparent)]
    #[cfg_attr(feature = "serde", serde(skip_serializing, skip_deserializing))]
    LocaleError(#[from] unic_locale::parser::ParserError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;

    mod correct {
        use super::*;

        #[test]
        fn colon_terminal_symbol_may_stand_alone_in_text() {
            let variants = vec!["This is some text with the message: colons are cool"];
            helper::test_correct_variants(template, variants);
        }

        #[test]
        fn locales_are_accepted_and_correct() {
            let cases = vec![
                (
                    "locale \n :  \t en_us\n",
                    "Delimiter can be surrounded by whitespaces",
                    "en-US",
                ),
                ("locale:fr-FR\n", "Whitespaces are optional", "fr-FR"),
            ];
            for (variant, case, locale_str) in cases {
                let mut scanner = Scanner::new(variant);
                let locale_result = locale(&mut scanner).expect(&format!(
                    "Valid locale setting was falsely rejected. Case: {}",
                    case
                ));
                let locale_expected: Locale = locale_str.parse().unwrap();
                assert_eq!(
                    locale_result, locale_expected,
                    "Accepted locale setting did not return the expected locale. Case: {}",
                    case
                );
            }

            // Ensure the locale setting itself is optional
            let content_tokens: ContentTokens = "example text literal".parse().unwrap();
            let expected_locale: Locale = "en-US".parse().unwrap();
            assert_eq!(expected_locale, content_tokens.locale);
        }

        #[test]
        fn defaults_are_accepted() {
            Lazy::force(&helper::LOGGING);
            let key_defaults = vec![
                "{name:hallo}",              // `text` default for key
                "{name:$Me}",                // `constant` default for key
                "{name:${Someone}}",         // `option` default for key
                "{name:${Kontake:Müller}}", // `text` default for `option` default for `key`
            ];
            helper::test_correct_variants(key, key_defaults);
            let opt_defaults = vec![
                "${Someone:{name}}", // `key` default for option
            ];
            helper::test_correct_variants(option, opt_defaults);
        }

        #[test]
        fn keys_are_accepted() {
            let keys = vec!["{name}", "{NAME}", "{NaMe}", "{n}", "{N}", "{08nsf}"];
            helper::test_correct_variants(key, keys);
        }

        #[test]
        fn idents_are_accepted() {
            let idents = vec!["hallo", "HALLO", "hAlLO", "h4ll0", "823480", "H4LLO"];
            helper::test_correct_variants(ident, idents);

            let all_symbols = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            let mut scanner = Scanner::new(&all_symbols);
            assert!(ident(&mut scanner).is_ok());
        }

        #[test]
        fn options_are_accepted() {
            let options = vec!["${Adressat}", "${addressat}", "${NAME}"];
            helper::test_correct_variants(option, options);
        }

        #[test]
        fn constants_are_accepted() {
            Lazy::force(&helper::LOGGING);
            let options = vec!["$MyName", "$myname", "$me13", "$3.141"];
            helper::test_correct_variants(constant, options);
        }

        #[test]
        fn templates_are_accepted() {
            Lazy::force(&helper::LOGGING);
            let templates = vec![
                "{key}$Constant${Option}",
                "Sehr ${Anrede} {name}\n{nachricht}\n$Mfg\n$Sender",
                "Sehr geehrte Frau {name}\n{nachricht}\nMit freundlichen Grüßen\nBar",
                "Hallo Herr {name:${Kontake:Müller}}, ich wollte ...",
            ];
            helper::test_correct_variants(template, templates);
        }

        #[test]
        fn texts_are_accepted() {
            let texts = vec![
                "Sehr geehrter Herr Foo \n\t iblbl",
                "\nHallo",
                "h",
                "\nllsf\n",
                ")_!_&_)*@#*^+_[]0=082q5-=8';,m;,.<''\"",
                "\n \t ",
            ];
            helper::test_correct_variants(text, texts);
        }
    }

    mod incorrect {
        use super::*;

        #[test]
        fn locales_are_rejected() {
            let cases = vec![
                ("en_US\n", "Locales require `locale` keyword"),
                (
                    "locale:en-US",
                    "Locales require newline behind locale string",
                ),
                ("locale en-US\n", "Locales require colon delimiter"),
                (
                    " anything locale:en-US\n",
                    "Locale keyword has to be at the very start of the file",
                ),
            ];
            helper::test_incorrect_cases(locale, cases);
        }

        #[test]
        fn keys_are_rejected() {
            let cases = vec![
                ("name", "is missing braces"),
                ("{name", "is missing right brace"),
                ("name}", "is missing left brace"),
                ("{&*(^)}", "contains invalid characters"),
                ("{ /t\n}", "only contains whitespace charactes"),
                ("{ /tsf\n}", "contains whitespace charactes"),
            ];
            helper::test_incorrect_cases(key, cases);
        }

        #[test]
        fn idents_are_rejected() {
            let cases = vec![
                (" \n \t", "only contains whitespace characters"),
                ("*)&%%_)+|", "only contains invalid characters"),
                ("&*!abc", "starts out with invalid characters"),
            ];
            helper::test_incorrect_cases(ident, cases);
        }

        #[test]
        fn options_are_rejected() {
            let cases = vec![
                ("$name", "is missing the braces"),
                ("{name}", "is missing the dollar sign"),
                ("${}", "is missing an identifier"),
                (
                    "$ {name}",
                    "has a whitespace between the dollar sign and the first brace",
                ),
            ];
            helper::test_incorrect_cases(option, cases);
        }

        #[test]
        fn constants_are_rejected() {
            let cases = vec![
                (
                    "$ name",
                    "has a whitespace between the dollar sign and the ident",
                ),
                ("${name}", "has braces around it's ident"),
            ];
            helper::test_incorrect_cases(constant, cases);
        }

        #[test]
        fn texts_are_rejected() {
            let cases = vec![
                ("{}\nsf{dsf}$", "contains invalid characters"),
                ("$$}}{}$", "only contains invalid characters"),
            ];
            helper::test_incorrect_cases(text, cases);
        }
    }

    mod helper {
        use super::*;

        // Initialize logging for a test
        pub static LOGGING: Lazy<()> = Lazy::new(|| {
            env_logger::init();
        });

        pub fn test_correct_variants<T, E>(
            parse_fn: fn(&mut Scanner) -> Result<T, E>,
            variants: Vec<&str>,
        ) where
            T: std::fmt::Debug,
            E: std::error::Error,
        {
            for variant in variants {
                let mut scanner = Scanner::new(&variant);
                assert!(parse_fn(&mut scanner).is_ok());
            }
        }

        /*pub fn test_correct_cases<T, E>(
            parse_fn: fn(&mut Scanner) -> Result<T, E>,
            cases: Vec<(&str, &str)>,
        )
        where
            T: std::fmt::Debug, E: std::error::Error,
        {
            for (variant, case) in cases {
                let mut scanner = Scanner::new(&variant);
                assert!(
                    parse_fn(&mut scanner).is_ok(),
                    "A valid variant: '{}' was falsely rejected. Case: {}",
                    variant,
                    case,
                );
            }
        }*/

        pub fn test_incorrect_cases<T, E>(
            parse_fn: fn(&mut Scanner) -> Result<T, E>,
            cases: Vec<(&str, &str)>,
        ) where
            T: std::fmt::Debug,
            E: std::error::Error,
        {
            for (variant, case) in cases {
                let mut scanner = Scanner::new(&variant);
                assert!(
                    parse_fn(&mut scanner).is_err(),
                    "An invalid variant: '{}' was falsely accepted! Case: {}",
                    variant,
                    case,
                );
            }
        }
    }
}
