use crate::content::*;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Template {
    tokens: ContentTokens,
    required: RequiredContent,
}

impl Template {
    /// Create a new `Template` instance by parsing the input string
    pub fn parse(s: &str) -> Result<Self, TemplateError> {
        let tokens: ContentTokens = s.parse()?;
        let required = tokens.draft();
        Ok(Self { tokens, required })
    }

    /// Compile the template using the default settings.
    pub fn fill_out(
        self,
        volatile_content: VolatileContent,
        content_state: ContentState,
    ) -> Result<String, TemplateError> {
        // Delegate the compilation to a `TemplateWithSettings` instance
        // with a default settings field.
        let with_settings = TemplateWithSettings {
            template: self,
            settings: CompilationSettings::default(),
        };
        with_settings.fill_out(volatile_content, content_state)
    }

    #[inline]
    pub const fn required(&self) -> &RequiredContent {
        &self.required
    }

    #[inline]
    pub fn with_settings(self, settings: CompilationSettings) -> TemplateWithSettings {
        TemplateWithSettings {
            template: self,
            settings,
        }
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

/// Settings for compiling a template.
#[derive(Debug)]
pub struct CompilationSettings {
    /// If set dynamic elements (i.e. meta constants) will be ignored
    /// and treated as regular elements.
    pub ignore_dynamics: bool,
}

impl std::default::Default for CompilationSettings {
    fn default() -> Self {
        Self {
            ignore_dynamics: false,
        }
    }
}

/// Combination of a template with the some compilation settings.
#[derive(Debug)]
pub struct TemplateWithSettings {
    template: Template,
    settings: CompilationSettings,
}

impl TemplateWithSettings {
    /// Compile a template, considering the given settings.
    /// This method is the only way to compile a template. The `Template::fill_out` method
    /// simply creates a new `TemplateWithSettings` instance (with the default settings)
    /// and then calls this method.
    pub fn fill_out(
        self,
        volatile_content: VolatileContent,
        content_state: ContentState,
    ) -> Result<String, TemplateError> {        
        let mut required = self.template.required;
        required.add_constants(content_state.constants);
        required.add_options(volatile_content.choices, content_state.options);
        required.add_keys(volatile_content.keys);

        let settings = self.settings;
        if settings.ignore_dynamics == false {
            // Evaluate all dynamic elements in the requirements.
            // `eval_dyn` does nothing if the "dyn" feature is disabled.
            required.eval_dyn();
        }

        let content: FullContent = required.try_into()?;
        Ok(self.template.tokens.fill_out(content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_examples() {
        {
            let mut cs = ContentState::new();
            cs.map_constant("name", "Paul");
            cs.map_option("SeeOff", choice!("CU", "See You"));
            let mut vc = VolatileContent::new();
            vc.map_key("other", "Leto");
            vc.map_choice("SeeOff", "CU");
            helper::test_fill_out(
                "Hello {other:Atreides}, I am $name.\n${SeeOff}",
                "Hello Leto, I am Paul.\nSee You",
                "Atreides example greeting",
                vc,
                cs,
            );
        }
        {
            let mut cs = ContentState::new();
            cs.map_constant("Ich", "Paul");
            cs.map_option("Anrede", choice!("m", "Sehr geehrter Herr"));
            cs.map_option("Anrede", choice!("w", "Sehr geehrte Frau"));
            cs.map_constant("Mfg", "Mit freundlichen Grüßen");
            let mut vc = VolatileContent::new();
            vc.map_key("Adressat", "Jessica");
            vc.map_key("nachricht", "ich bin tatsächlich der Kwisatz Haderach");
            vc.map_choice("Anrede", "w");
            helper::test_fill_out("${Anrede} {Adressat}, {nachricht}\n$Mfg\n$Ich",
                "Sehr geehrte Frau Jessica, ich bin tatsächlich der Kwisatz Haderach\nMit freundlichen Grüßen\nPaul",
                "Atreides example message",
                vc, cs);
        }
        #[cfg(feature = "dyn")]
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
                VolatileContent::new(),
                ContentState::new(),
            );
        }
    }

    #[test]
    fn idents_do_not_collide_outside_of_types() {
        let ident = "name"; // Same ident used once for each variable-element type
        {
            let mut cs = ContentState::new();
            cs.map_constant(ident, "constant-literal");
            cs.map_option(ident, choice!("only", "choice-literal"));
            let mut vc = VolatileContent::new();
            vc.map_key(ident, "key-literal");
            vc.map_choice(ident, "only");
            helper::test_fill_out(
                "{name} ${name} $name",
                "key-literal choice-literal constant-literal",
                "Identifiers of diffenent types do not collide",
                vc,
                cs,
            );
        }
        {
            let mut vc = VolatileContent::new();
            vc.map_key(ident, "initial key literal");
            vc.map_key(ident, "last key literal");
            helper::test_fill_out(
                "{name}",
                "last key literal",
                "Identifiers of same type overwrite each other",
                vc,
                ContentState::new(),
            );
        }
    }

    #[test]
    fn template_examples_are_filled_out_correctly() {
        // Random test cases
        {
            let mut vc = VolatileContent::new();
            vc.map_key("name", "Paul");
            helper::test_fill_out(
                "Hello, my name is {name}!",
                "Hello, my name is Paul!",
                "Simple key",
                vc,
                ContentState::new(),
            );
        }
        {
            let mut cs = ContentState::new();
            cs.map_constant("me", "Paul");
            helper::test_fill_out(
                "{from:$me}",
                "Paul",
                "Constant is resolved as a default",
                VolatileContent::new(),
                cs,
            );
        }
        {
            let mut vc = VolatileContent::new();
            vc.map_key("another", "another-literal");
            helper::test_fill_out(
                "{key:{another:default-literal}}",
                "another-literal",
                "Value of nested default correctly overwritten",
                vc,
                ContentState::new(),
            );
        }
        {
            let mut cs = ContentState::new();
            cs.map_option("opt", choice!("only", "choice-literal"));
            helper::test_fill_out(
                "${opt:default-literal}",
                "default-literal",
                "Value of default used for option without choice",
                VolatileContent::new(),
                cs,
            );
        }
        {
            let mut vc = VolatileContent::new();
            vc.map_key("key", "literal-content");
            helper::test_fill_out(
                "{key:default-literal}",
                "literal-content",
                "Key value overwrites default",
                vc,
                ContentState::new(),
            );
        }
        {
            let mut cs = ContentState::new();
            cs.map_constant("workemail", "im@work.com");
            cs.map_option("email", choice!("private", "im@home.com"));
            helper::test_fill_out(
                "${email:$workemail}",
                "im@work.com",
                "Default constant used for option without choice",
                VolatileContent::new(),
                cs,
            );
        }
    }

    // Test cases asserting all requirements for default *from the spec* are met

    #[test]
    fn defaults_are_used_if_value_is_not_specified() {
        {
            let mut cs = ContentState::new();
            cs.map_option("opt", choice!("only", "choice-literal"));
            helper::test_fill_out(
                "${opt:default-literal}",
                "default-literal",
                "Defaults are used for unspecified options; option becomes optional",
                VolatileContent::new(),
                cs,
            );
        }
        {
            helper::test_fill_out(
                "{key:default-literal}",
                "default-literal",
                "Defaults are used for unspecified keys; key becomes optional",
                VolatileContent::new(),
                ContentState::new(),
            );
        }
    }

    #[test] // "If, however a value is goven for the element, [it] will overwrite the default value"
    fn defaults_are_ignored_if_value_is_specified() {
        {
            let mut cs = ContentState::new();
            cs.map_option("opt", choice!("only", "choice-literal"));
            let mut vc = VolatileContent::new();
            vc.map_choice("opt", "only");
            helper::test_fill_out(
                "${opt:default-literal}",
                "choice-literal",
                "Defaults are NOT used for specified options; choice overwrites default",
                vc,
                cs,
            );
        }
        {
            let mut vc = VolatileContent::new();
            vc.map_key("key", "key-literal");
            helper::test_fill_out(
                "{key:default-literal}",
                "key-literal",
                "Defaults are NOT used for specified keys; key overwrites default",
                vc,
                ContentState::new(),
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
                VolatileContent::new(),
                ContentState::new(),
            );
        }
        {
            let mut cs = ContentState::new();
            cs.map_constant("constant", "constant-literal");
            helper::test_fill_out(
                "{key:$constant}",
                "constant-literal",
                "Constant elements can be used as defaults",
                VolatileContent::new(),
                cs,
            );
        }
        {
            let mut cs = ContentState::new();
            cs.map_option("opt", choice!("only", "choice-literal"));
            let mut vc = VolatileContent::new();
            vc.map_choice("opt", "only");
            helper::test_fill_out(
                "{key:${opt}}",
                "choice-literal",
                "Option elements can be used as defaults",
                vc,
                cs,
            );
        }
        {
            let mut vc = VolatileContent::new();
            vc.map_key("defaultKey", "default-key-literal");
            helper::test_fill_out(
                "{key:{defaultKey}}",
                "default-key-literal",
                "Key elements can be used as defaults",
                vc,
                ContentState::new(),
            );
        }
    }

    #[test] // "Defaults may also be nested"
    fn defaults_may_be_nested() {
        {
            let mut cs = ContentState::new();
            cs.map_option("option", choice!("only", "choice-literal"));
            helper::test_fill_out(
                "{key:${option:default-literal}}",
                "default-literal",
                "Simple nesting",
                VolatileContent::new(),
                cs,
            );
        }
        /*{  // This currently overflows the stack
            let mut cs = ContentState::new();
            cs.map_option("opt1", choice!("only", "choice-literal"));
            cs.map_option("opt2", choice!("only", "choice-literal"));
            helper::test_fill_out("{key:${opt1:${opt2:{key:default-literal}}}}", "default-literal", "Long nesting", VolatileContent::new(), cs);
        }*/
        // Chain of nested elements is stopped if a value was specified for any element in the chain
        {
            let mut cs = ContentState::new();
            cs.map_option("opt1", choice!("only", "choice-literal"));
            cs.map_option("opt2", choice!("only", "choice-literal"));
            let mut vc = VolatileContent::new();
            vc.map_choice("opt2", "only");
            helper::test_fill_out(
                "{key:${opt1:${opt2:{key:default-literal}}}}",
                "choice-literal",
                "Default chain stopped by value",
                vc,
                cs,
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
        volatile_content: VolatileContent,
        content_state: ContentState,
    ) {
        let result = Template::parse(input)
            .unwrap()
            .fill_out(volatile_content, content_state)
            .unwrap();
        assert_eq!(&result, expected, "Test case: {}", case);
    }
}
