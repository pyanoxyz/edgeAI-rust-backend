
use crate::utils::get_total_ram;

const LLAMA_PROMPT_TEMPLATE: &str = r#"
    <|begin_of_text|><|start_header_id|>system<|end_header_id|>{system_prompt}<|eot_id|>
            <|start_header_id|>user
            <|end_header_id|>
            {user_prompt}
            <|eot_id|>
            <|start_header_id|>assistant<|end_header_id|>
        "#;

const QWEN_PROMPT_TEMPLATE: &str = r#"
    <|im_start|>system
    {system_prompt}<|im_end|>
    <|im_start|>user
    {user_prompt}<|im_end|>
    <|im_start|>assistant
    "#;

pub fn get_default_prompt_template() -> String {
    let gb_in_bytes: u64 = 8 * 1024 * 1024 * 1024; // 8GB in bytes

    // Assuming get_total_ram() returns the total system RAM in bytes
    if get_total_ram() < gb_in_bytes as f64 {
        LLAMA_PROMPT_TEMPLATE.to_string() // Return the Llama template if RAM is less than 8GB
    } else {
        QWEN_PROMPT_TEMPLATE.to_string() // Return the Qwen template if RAM is 8GB or more
    }
}