use wasm_bindgen::prelude::*;
extern crate console_error_panic_hook;
use txt_templ_compiler::template;
use txt_templ_compiler::content::{UserContent, UserContentState};

#[wasm_bindgen]
extern {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
pub struct Template(template::Template);

#[wasm_bindgen]
impl Template {
    pub fn parse(s: &str) -> Result<Template, String> {
        console_error_panic_hook::set_once();
        match template::Template::parse(s) {
            Ok(templ) => Ok(Self(templ)),
            Err(e) => Err(serde_json::to_string(&e).unwrap()),
        }
    }

    pub fn fill_out(
        &self,
        user_content: JsValue,
        user_content_state: JsValue,
    ) -> Result<String, String> {
        let user_content: UserContent = serde_wasm_bindgen::from_value(user_content).unwrap();
        let user_content_state: UserContentState = serde_wasm_bindgen::from_value(user_content_state).unwrap();
        match self.0.fill_out(user_content, user_content_state) {
            Ok(result) => Ok(result),
            Err(e) => Err(serde_json::to_string(&e).unwrap()),
        }
    }
}
