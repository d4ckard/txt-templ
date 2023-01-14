use crate::parse::UserError;
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

