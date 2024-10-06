
use crate::database::db_config::DB_INSTANCE;
use log::{warn, error};


const QWEN_PROMPT_TEMPLATE: &str = r#"
    <|im_start|>system
    {system_prompt}<|im_end|>
    <|im_start|>user
    {user_prompt}<|im_end|>
    <|im_start|>assistant
    "#;

pub fn get_default_prompt_template() -> String {

    let sytem_prompt = match DB_INSTANCE.get_system_prompt() {
        Ok(system_prompt) => {
            warn!("Sytem prompt in DB {:?}", system_prompt);
            system_prompt},
        Err(error) => {
            error!("Couldnt get the system prompt out from the database {:?}", error);
            QWEN_PROMPT_TEMPLATE.to_owned()
        }
    };
    sytem_prompt

}