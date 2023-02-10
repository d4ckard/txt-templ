use crate::content::*;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Template {
    tokens: ContentTokens,
    required: RequiredContent,
}

// TODO: Make dynamic/meta content a library feature.

impl Template {
    // Create a new `Template` instance by parsing the input string
    pub fn parse(s: &str) -> Result<Self, TemplateError> {
        let tokens: ContentTokens = s.parse()?;
        let required = tokens.draft();
        Ok(Self { tokens, required })
    }

    // Fill out the template
    pub fn fill_out(
        mut self,
        user_content: UserContent,
        user_content_state: UserContentState,
    ) -> Result<String, TemplateError> {
        self.required.add_constants(user_content_state.constants);
        self.required
            .add_options(user_content.choices, user_content_state.options);
        self.required.add_keys(user_content.keys);

        // Evaluate all dynamic elements in the requirements.
        // TODO: Add a switch to enable/disable dynamic content.
        self.required.eval_dyn();

        let content: FullContent = self.required.try_into()?;
        Ok(self.tokens.fill_out(content))
    }

    pub const fn required(&self) -> &RequiredContent {
        &self.required
    }
}

#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TemplateError {
    #[error(transparent)]
    UserError(#[from] UserError),
    #[error(transparent)]
    FillOutError(#[from] FillOutError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_examples() {
        {
            let mut ucs = UserContentState::new();
            ucs.map_constant("name", "Paul");
            ucs.map_option("SeeOff", choice!("CU", "See You"));
            let mut uc = UserContent::new();
            uc.map_key("other", "Leto");
            uc.map_choice("SeeOff", "CU");
            helper::test_fill_out(
                "Hello {other:Atreides}, I am $name.\n${SeeOff}",
                "Hello Leto, I am Paul.\nSee You",
                "Atreides example greeting",
                uc,
                ucs,
            );
        }
        {
            let mut ucs = UserContentState::new();
            ucs.map_constant("Ich", "Paul");
            ucs.map_option("Anrede", choice!("m", "Sehr geehrter Herr"));
            ucs.map_option("Anrede", choice!("w", "Sehr geehrte Frau"));
            ucs.map_constant("Mfg", "Mit freundlichen Grüßen");
            let mut uc = UserContent::new();
            uc.map_key("Adressat", "Jessica");
            uc.map_key("nachricht", "ich bin tatsächlich der Kwisatz Haderach");
            uc.map_choice("Anrede", "w");
            helper::test_fill_out("${Anrede} {Adressat}, {nachricht}\n$Mfg\n$Ich",
                "Sehr geehrte Frau Jessica, ich bin tatsächlich der Kwisatz Haderach\nMit freundlichen Grüßen\nPaul",
                "Atreides example message",
                uc, ucs);
        }
        {
            // This example uses meta constants to substitute dynamic information on
            // the current date in the template.
            use chrono::Utc;
            let now = Utc::now();
            let month = now.format("%B");
            let day = now.format("%d");
            let day_name = now.format("%A");
            helper::test_fill_out(
                "This template was compiled on $Month $DayNum which is a $Day",
                &format!("This template was compiled on {month} {day} which is a {day_name}"),
                "Dynamic meta content example",
                UserContent::new(),
                UserContentState::new(),
            );
        }
    }

    #[test]
    fn idents_do_not_collide_outside_of_types() {
        let ident = "name"; // Same ident used once for each variable-element type
        {
            let mut ucs = UserContentState::new();
            ucs.map_constant(ident, "constant-literal");
            ucs.map_option(ident, choice!("only", "choice-literal"));
            let mut uc = UserContent::new();
            uc.map_key(ident, "key-literal");
            uc.map_choice(ident, "only");
            helper::test_fill_out(
                "{name} ${name} $name",
                "key-literal choice-literal constant-literal",
                "Identifiers of diffenent types do not collide",
                uc,
                ucs,
            );
        }
        {
            let mut uc = UserContent::new();
            uc.map_key(ident, "initial key literal");
            uc.map_key(ident, "last key literal");
            helper::test_fill_out(
                "{name}",
                "last key literal",
                "Identifiers of same type overwrite each other",
                uc,
                UserContentState::new(),
            );
        }
    }

    #[test]
    fn template_examples_are_filled_out_correctly() {
        // Random test cases
        {
            let mut uc = UserContent::new();
            uc.map_key("name", "Paul");
            helper::test_fill_out(
                "Hello, my name is {name}!",
                "Hello, my name is Paul!",
                "Simple key",
                uc,
                UserContentState::new(),
            );
        }
        {
            let mut ucs = UserContentState::new();
            ucs.map_constant("me", "Paul");
            helper::test_fill_out(
                "{from:$me}",
                "Paul",
                "Constant is resolved as a default",
                UserContent::new(),
                ucs,
            );
        }
        {
            let mut uc = UserContent::new();
            uc.map_key("another", "another-literal");
            helper::test_fill_out(
                "{key:{another:default-literal}}",
                "another-literal",
                "Value of nested default correctly overwritten",
                uc,
                UserContentState::new(),
            );
        }
        {
            let mut ucs = UserContentState::new();
            ucs.map_option("opt", choice!("only", "choice-literal"));
            helper::test_fill_out(
                "${opt:default-literal}",
                "default-literal",
                "Value of default used for option without choice",
                UserContent::new(),
                ucs,
            );
        }
        {
            let mut uc = UserContent::new();
            uc.map_key("key", "literal-content");
            helper::test_fill_out(
                "{key:default-literal}",
                "literal-content",
                "Key value overwrites default",
                uc,
                UserContentState::new(),
            );
        }
        {
            let mut ucs = UserContentState::new();
            ucs.map_constant("workemail", "im@work.com");
            ucs.map_option("email", choice!("private", "im@home.com"));
            helper::test_fill_out(
                "${email:$workemail}",
                "im@work.com",
                "Default constant used for option without choice",
                UserContent::new(),
                ucs,
            );
        }
    }

    // Test cases asserting all requirements for default *from the spec* are met

    #[test]
    fn defaults_are_used_if_value_is_not_specified() {
        {
            let mut ucs = UserContentState::new();
            ucs.map_option("opt", choice!("only", "choice-literal"));
            helper::test_fill_out(
                "${opt:default-literal}",
                "default-literal",
                "Defaults are used for unspecified options; option becomes optional",
                UserContent::new(),
                ucs,
            );
        }
        {
            helper::test_fill_out(
                "{key:default-literal}",
                "default-literal",
                "Defaults are used for unspecified keys; key becomes optional",
                UserContent::new(),
                UserContentState::new(),
            );
        }
    }

    #[test] // "If, however a value is goven for the element, [it] will overwrite the default value"
    fn defaults_are_ignored_if_value_is_specified() {
        {
            let mut ucs = UserContentState::new();
            ucs.map_option("opt", choice!("only", "choice-literal"));
            let mut uc = UserContent::new();
            uc.map_choice("opt", "only");
            helper::test_fill_out(
                "${opt:default-literal}",
                "choice-literal",
                "Defaults are NOT used for specified options; choice overwrites default",
                uc,
                ucs,
            );
        }
        {
            let mut uc = UserContent::new();
            uc.map_key("key", "key-literal");
            helper::test_fill_out(
                "{key:default-literal}",
                "key-literal",
                "Defaults are NOT used for specified keys; key overwrites default",
                uc,
                UserContentState::new(),
            );
        }
    }

    #[test] // "Elements of any type can be used as defaults"
    fn elements_of_any_type_can_be_used_as_defaults() {
        {
            helper::test_fill_out(
                "{key:text-literal}",
                "text-literal",
                "Text literal elements can be used as defaults",
                UserContent::new(),
                UserContentState::new(),
            );
        }
        {
            let mut ucs = UserContentState::new();
            ucs.map_constant("constant", "constant-literal");
            helper::test_fill_out(
                "{key:$constant}",
                "constant-literal",
                "Constant elements can be used as defaults",
                UserContent::new(),
                ucs,
            );
        }
        {
            let mut ucs = UserContentState::new();
            ucs.map_option("opt", choice!("only", "choice-literal"));
            let mut uc = UserContent::new();
            uc.map_choice("opt", "only");
            helper::test_fill_out(
                "{key:${opt}}",
                "choice-literal",
                "Option elements can be used as defaults",
                uc,
                ucs,
            );
        }
        {
            let mut uc = UserContent::new();
            uc.map_key("defaultKey", "default-key-literal");
            helper::test_fill_out(
                "{key:{defaultKey}}",
                "default-key-literal",
                "Key elements can be used as defaults",
                uc,
                UserContentState::new(),
            );
        }
    }

    #[test] // "Defaults may also be nested"
    fn defaults_may_be_nested() {
        {
            let mut ucs = UserContentState::new();
            ucs.map_option("option", choice!("only", "choice-literal"));
            helper::test_fill_out(
                "{key:${option:default-literal}}",
                "default-literal",
                "Simple nesting",
                UserContent::new(),
                ucs,
            );
        }
        /*{  // This currently overflows the stack
            let mut ucs = UserContentState::new();
            ucs.map_option("opt1", choice!("only", "choice-literal"));
            ucs.map_option("opt2", choice!("only", "choice-literal"));
            helper::test_fill_out("{key:${opt1:${opt2:{key:default-literal}}}}", "default-literal", "Long nesting", UserContent::new(), ucs);
        }*/
        // Chain of nested elements is stopped if a value was specified for any element in the chain
        {
            let mut ucs = UserContentState::new();
            ucs.map_option("opt1", choice!("only", "choice-literal"));
            ucs.map_option("opt2", choice!("only", "choice-literal"));
            let mut uc = UserContent::new();
            uc.map_choice("opt2", "only");
            helper::test_fill_out(
                "{key:${opt1:${opt2:{key:default-literal}}}}",
                "choice-literal",
                "Default chain stopped by value",
                uc,
                ucs,
            );
        }
    }
}

/// Module with template test helpers which is public to the crate
/// so any test can use it
#[cfg(test)]
pub(crate) mod helper {
    use super::*;

    // Assert that filling out is correct
    pub fn test_fill_out(
        input: &str,
        expected: &str,
        case: &str,
        user_content: UserContent,
        user_content_state: UserContentState,
    ) {
        let result = Template::parse(input)
            .unwrap()
            .fill_out(user_content, user_content_state)
            .unwrap();
        assert_eq!(&result, expected, "Test case: {}", case);
    }
}
