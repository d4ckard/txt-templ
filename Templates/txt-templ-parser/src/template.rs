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
    fn template_api_works() {
        let input = "Hallo {name:A default literal}, ich bin $name.\n${SeeOff}";
        let user_content = {
            let mut c = UserContent::new();
            c.map_key("name", "Leto");
            c.map_choice("SeeOff", "CU");
            c
        };
        let user_content_state = {
            let mut c = UserContentState::new();
            c.map_constant("name", "Paul");
            c.map_option("SeeOff", new_choice("CU", "See You"));
            c            
        };

        let output = Template::parse(input).unwrap()
            .fill_out(user_content, user_content_state).unwrap();
        assert_eq!(&output, "Hallo Leto, ich bin Paul.\nSee You");
    }

    #[test]
    fn recursive_template_is_processed_correctly() {
        let input = "a {name:{another:default literal}}";
        let expected = "a default literal";

        let output = Template::parse(input).unwrap()
            .fill_out(UserContent::new(), UserContentState::new()).unwrap();
        assert_eq!(&output, expected);
    }

    #[test]
    fn constant_default_for_option() {
        let input = "${email:$workemail}";

        let user_content = UserContent::new();
        let user_content_state = {
            let mut c = UserContentState::new();
            c.map_constant("workemail", "im@work.com");
            c.map_option("email", new_choice("private", "im@home.com"));
            c
        };

        let output = Template::parse(input).unwrap()
            .fill_out(user_content, user_content_state).unwrap();
        assert_eq!(&output, "im@work.com");
    }

}
