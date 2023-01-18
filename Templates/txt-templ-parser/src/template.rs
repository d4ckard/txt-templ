use crate::content::*;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub struct Template(ContentTokens);

impl Template {
    // Create a new `Template` instance by parsing the input string
    pub fn parse(s: &str) -> Result<Self, TemplateError> {
        Ok(Self(s.parse()?))
    }
    
    // Fill out the template
    pub fn fill_out(
        &self,
        user_content: UserContent,
        user_content_state: UserContentState
    ) -> Result<String, TemplateError> {
        let mut required = self.0.draft();
        required.add_constants(user_content_state.constants);
        required.add_options(user_content.choices, user_content_state.options);
        required.add_keys(user_content.keys);

        let content: FullContent = required.try_into()?;
        Ok(self.0.fill_out(content)?)
    }
}

#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TemplateError {
    #[error("transparent")]
    UserError(#[from] UserError),
    #[error("transparent")]
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
            ucs.map_option("SeeOff", new_choice("CU", "See You"));
            let mut uc = UserContent::new();
            uc.map_key("other", "Leto");
            uc.map_choice("SeeOff", "CU");
            helper::test_fill_out("Hello {other:Atreides}, I am $name.\n${SeeOff}", "Hello Leto, I am Paul.\nSee You", "Atreides example greeting",
                uc, ucs);
        }
    }

    #[test]
    fn idents_do_not_collide_outside_of_types() {
        let ident = "name";  // Same ident used once for each variable-element type
        {
            let mut ucs = UserContentState::new();
            ucs.map_constant(ident, "constant-literal");
            ucs.map_option(ident, new_choice("only", "choice-literal"));
            let mut uc = UserContent::new();
            uc.map_key(ident, "key-literal");
            uc.map_choice(ident, "only");
            helper::test_fill_out("{name} ${name} $name", "key-literal choice-literal constant-literal", "Identifiers of diffenent types do not collide",
                uc, ucs);
        }
        {
            let mut uc = UserContent::new();
            uc.map_key(ident, "initial key literal");
            uc.map_key(ident, "last key literal");
            helper::test_fill_out("{name}", "last key literal", "Identifiers of same type overwrite each other", uc, UserContentState::new());
        }
    }

    #[test]
    fn template_examples_are_filled_out_correctly() {
        // Random test cases
        {
            let mut uc = UserContent::new();
            uc.map_key("name", "Paul");
            helper::test_fill_out("Hello, my name is {name}!", "Hello, my name is Paul!", "Simple key", uc, UserContentState::new());
        }

        {
            let mut uc = UserContent::new();
            uc.map_key("another", "another-literal");
            helper::test_fill_out("{key:{another:default-literal}}", "another-literal", "Value of nested default correctly overwritten", uc, UserContentState::new());
        }
        {
            let mut ucs = UserContentState::new();
            ucs.map_option("opt", new_choice("only", "choice-literal"));
            helper::test_fill_out("${opt:default-literal}", "default-literal", "Value of default used for option without choice", UserContent::new(), ucs);
        }
        {
            let mut uc = UserContent::new();
            uc.map_key("key", "literal-content");
            helper::test_fill_out("{key:default-literal}", "literal-content", "Key value overwrites default", uc, UserContentState::new());
        }
        {
            let mut ucs = UserContentState::new();
            ucs.map_constant("workemail", "im@work.com");
            ucs.map_option("email", new_choice("private", "im@home.com"));
            helper::test_fill_out("${email:$workemail}", "im@work.com", "Default constant used for option without choice", UserContent::new(), ucs);
        }
    }
    
    // Test cases asserting all requirements for default *from the spec* are met
    #[test]
    fn defaults_are_used_if_value_is_not_specified() {
        {
            let mut ucs = UserContentState::new();
            ucs.map_option("opt", new_choice("only", "choice-literal"));
            helper::test_fill_out("${opt:default-literal}", "default-literal", "Defaults are used for unspecified options; option becomes optional",
                UserContent::new(), ucs);
        }
        {
            helper::test_fill_out("{key:default-literal}", "default-literal", "Defaults are used for unspecified keys; key becomes optional",
                UserContent::new(), UserContentState::new());
        }
    }

    #[test] // "If, however a value is goven for the element, [it] will overwrite the default value"
    fn defaults_are_ignored_if_value_is_specified() {
        {
            let mut ucs = UserContentState::new();
            ucs.map_option("opt", new_choice("only", "choice-literal"));
            let mut uc = UserContent::new();
            uc.map_choice("opt", "only");
            helper::test_fill_out("${opt:default-literal}", "choice-literal", "Defaults are NOT used for specified options; choice overwrites default",
                uc, ucs);
        }
        {
            let mut uc = UserContent::new();
            uc.map_key("key", "key-literal");
            helper::test_fill_out("{key:default-literal}", "key-literal", "Defaults are NOT used for specified keys; key overwrites default",
                uc, UserContentState::new());
        }
    }

    #[test] // "Elements of any type can be used as defaults"
    fn elements_of_any_type_can_be_used_as_defaults() {
        {
            helper::test_fill_out("{key:text-literal}", "text-literal", "Text literal elements can be used as defaults",
                UserContent::new(), UserContentState::new());
        }
        {
            let mut ucs = UserContentState::new();
            ucs.map_constant("constant", "constant-literal");
            helper::test_fill_out("{key:$constant}", "constant-literal", "Constant elements can be used as defaults",
                UserContent::new(), ucs);
        }
        {
            let mut ucs = UserContentState::new();
            ucs.map_option("opt", new_choice("only", "choice-literal"));
            let mut uc = UserContent::new();
            uc.map_choice("opt", "only");
            helper::test_fill_out("{key:${opt}}", "choice-literal", "Option elements can be used as defaults", uc, ucs);
        }
        {
            let mut uc = UserContent::new();
            uc.map_key("defaultKey", "default-key-literal");
            helper::test_fill_out("{key:{defaultKey}}", "default-key-literal", "Key elements can be used as defaults", uc, UserContentState::new());
        }
    }

    #[test] // "Defaults may also be nested"
    fn defaults_may_be_nested() {
        {
            let mut ucs = UserContentState::new();
            ucs.map_option("option", new_choice("only", "choice-literal"));
            helper::test_fill_out("{key:${option:default-literal}}", "default-literal", "Simple nesting", UserContent::new(), ucs);
        }
        /*{  // This currently overflows the stack
            let mut ucs = UserContentState::new();
            ucs.map_option("opt1", new_choice("only", "choice-literal"));
            ucs.map_option("opt2", new_choice("only", "choice-literal"));
            helper::test_fill_out("{key:${opt1:${opt2:{key:default-literal}}}}", "default-literal", "Long nesting", UserContent::new(), ucs);
        }*/
        // Chain of nested elements is stopped if a value was specified for any element in the chain
        {
            let mut ucs = UserContentState::new();
            ucs.map_option("opt1", new_choice("only", "choice-literal"));
            ucs.map_option("opt2", new_choice("only", "choice-literal"));
            let mut uc = UserContent::new();
            uc.map_choice("opt2", "only");
            helper::test_fill_out("{key:${opt1:${opt2:{key:default-literal}}}}", "choice-literal", "Default chain stopped by value", uc, ucs);
        }
    }

    mod helper {
        use super::*;
        
        // Assert that filling out is correct
        pub fn test_fill_out(
            input: &str,
            expected: &str,
            case: &str,
            user_content: UserContent,
            user_content_state: UserContentState,
        ) {
            let result =
                Template::parse(input).unwrap()
                .fill_out(user_content, user_content_state).unwrap();
            assert_eq!(&result, expected, "Test case: {}", case);
        }
    }
}
