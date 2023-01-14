use crate::content::*;

#[derive(Debug)]
pub struct Template {
    source: String,
    tokens: ContentTokens,
}

impl Template {
    // Create a new `Template` instance by parsing the input string
    pub fn parse(s: &str) -> Result<Self, TemplateError> {
        Ok(Self {
            source: s.to_owned(),
            tokens: s.parse()?,
        })
    }
    
    // Fill out the template
    pub fn fill_out(
        &self,
        user_content: UserContent,
        user_content_state: UserContentState
    ) -> Result<String, TemplateError> {
        let mut required = self.tokens.draft();
        required.add_constants(user_content_state.constants);
        required.add_options(user_content.choices, user_content_state.options);
        required.add_keys(user_content.keys);

        let content: Content = required.try_into()?;
        Ok(self.tokens.fill_out(content)?)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum TemplateError {
    #[error("transparent")]
    UserError(#[from] UserError),
    #[error("transparent")]
    FillOutError(#[from] FillOutError),
}


#[cfg(test)]
mod tests {
    use super::*;
    use unic_locale::Locale;
    use std::collections::HashMap;

    #[test]
    fn template_api_works() {
        let input = "Hallo {name:A default literal}, ich bin $name.\n${SeeOff}";
        let user_content = {
            let mut content = UserContent {
                keys: HashMap::new(),
                choices: HashMap::new(),
            };
            content.keys.insert(Ident::new("name"), "Leto".to_owned());
            content.choices.insert(Ident::new("SeeOff"), Ident::new("CU"));
            content
        };
        let user_content_state = {
            let mut content  = UserContentState {
                constants: HashMap::new(),
                options: HashMap::new(),
            };
            content.constants.insert(Ident::new("name"), "Paul".to_owned());
            let mut choices = HashMap::new();
            choices.insert(Ident::new("CU"), "See You".to_owned());
            content.options.insert(Ident::new("SeeOff"), choices);
            content
        };

        let output = Template::parse(input).unwrap().fill_out(user_content, user_content_state).unwrap();
        assert_eq!(&output, "Hallo Leto, ich bin Paul.\nSee You");
    }

    mod correct {
        use super::*;

        /*#[test]
        fn fill_out_works() {
            let variants = vec![
                ("Hallo Paul", "Hallo {name}".parse::<ContentTokens>().unwrap(), vec![(TokenIdent::new("name", Token::Key), "Paul")]),
                ("a Leto b Paul", "a {other} b $name".parse().unwrap(), vec![
                    (TokenIdent::new("other", Token::Key), "Leto"),
                    (TokenIdent::new("name", Token::Constant), "Paul"),
                ]),
                ("a Leto b Paul", "a {other:Leto} b $name".parse().unwrap(), vec![
                    (TokenIdent::new("name", Token::Constant), "Paul")
                ]),
                ("a Leto b Paul", "a {other:{othername:Leto}} b $name".parse().unwrap(), vec![
                    (TokenIdent::new("name", Token::Constant), "Paul")
                ]),
            ];
                                         
            for (expected, tokens, pairs) in variants {
                let content = helper::content_map_from_vec(pairs);
                let output = tokens.fill_out(content);
                assert_eq!(&output.unwrap(), expected);
            }
        }*/

        #[test]
        fn draft_works() {
            let variants = vec![
                ("a {name} b $Bye".parse::<ContentTokens>().unwrap(), vec![
                    (TokenIdent::new("name", Token::Key), ""),
                    (TokenIdent::new("Bye", Token::Constant), ""),
                ]),
                ("{other:{othername:Leto}}".parse::<ContentTokens>().unwrap(), vec![
                    (TokenIdent::new("other", Token::Key), "Leto"),
                    (TokenIdent::new("othername", Token::Key), "Leto"),
                ]),
            ];
            for (tokens, pairs) in variants {
                let expected = helper::content_map_from_vec(pairs);
                let output = tokens.draft();
                assert_eq!(expected, output);
            }
        }

    }

    mod incorrect {
        /*
        use super::*;
        #[test]
        fn fill_out_rejects_ummodified_drafs() {
            let tokens: ContentTokens = "a {name} b $Const".parse().unwrap();
            let draft = tokens.draft();
            // While a draft contains all required keys, it's missing any content!
            // Therefore `fill_out` should always reject a raw draft.
            assert!(tokens.fill_out(draft).is_err());
        }*/
    }

    #[test]
    fn templates_are_parsed_correctly() {
        // Lenghts of literal text and idents in decreased so tests are more consice
        // Other tests assert that any idents/text passes
        let pairs = vec![
            ("fr-FR\n{key}$Constant${Option}", vec![
                ContentToken::Key(Ident::new("key"), None),
                ContentToken::Constant(Ident::new("Constant")),
                ContentToken::Option(Box::new(ContentToken::Key(Ident::new("Option"), None))),
            ], Some("fr-FR")),
            ("S ${Anrede} {name}\n{n}\n$M\n$S", vec![
                ContentToken::Text("S ".into()),
                ContentToken::Option(Box::new(ContentToken::Key(Ident::new("Anrede"), None))),
                ContentToken::Text(" ".into()),
                ContentToken::Key(Ident::new("name"), None),
                ContentToken::Text("\n".into()),
                ContentToken::Key(Ident::new("n"), None),
                ContentToken::Text("\n".into()),
                ContentToken::Constant(Ident::new("M")),
                ContentToken::Text("\n".into()),
                ContentToken::Constant(Ident::new("S")),
            ], None),
            ("Sehr geehrte Frau {name}\n{nachricht}\nMit freundlichen Grüßen\nBar", vec![
                ContentToken::Text("Sehr geehrte Frau ".into()),
                ContentToken::Key(Ident::new("name"), None),
                ContentToken::Text("\n".into()),
                ContentToken::Key(Ident::new("nachricht"), None),
                ContentToken::Text("\nMit freundlichen Grüßen\nBar".into()),
            ], None),
            ("{name:Peter} bla ${bye:{mfg:MfG}}", vec![
                ContentToken::Key(Ident::new("name"), Some(Box::new(ContentToken::Text("Peter".into())))),
                ContentToken::Text(" bla ".into()),
                ContentToken::Option(Box::new(
                    ContentToken::Key(Ident::new("bye"), Some(Box::new(
                        ContentToken::Key(Ident::new("mfg"), Some(Box::new(
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

    #[test]
    fn recursive_template_is_processed_correctly() {
        let input = "a {name:{another:default literal}}";
        let expected = "a default literal";
        let user_content =  UserContent{keys: HashMap::new(), choices: HashMap::new()};
        let user_content_state =  UserContentState{constants: HashMap::new(), options: HashMap::new()};
        let output = Template::parse(input).unwrap().fill_out(user_content, user_content_state).unwrap();
        assert_eq!(&output, expected);
    }

    mod helper {
        use super::*;

        pub fn content_map_from_vec(v: Vec<(TokenIdent, &str)>) -> RequiredContent {
            let mut map = RequiredContent::new();
            for (ident, value) in v {
                map.insert(ident, value.to_owned());
            }
            map
        }
    }
}
