// use actix_web::{ post, web, HttpRequest, HttpResponse, Error };
// use crate::chats::chat_struct::{ RefactorPrompt, handle_request };
// use serde::{ Deserialize, Serialize };
// use crate::request_type::RequestType;

// #[derive(Debug, Serialize, Deserialize)]
// pub struct ChatExplainRequest {
//     pub prompt: String,
//     pub session_id: Option<String>,
// }

// const SYSTEM_PROMPT: &str =
//     r#"
//    You are an expert code analyst specializing in breaking down code snippets and functions in a clear, step-by-step manner. \
//     Follow these guidelines for a detailed explanation:

//     1. **High-level overview**: Summarize the overall purpose of the code.
//     2. **Step-by-step breakdown**: Analyze each significant part, explaining its functionality and purpose.
//     3. **Algorithms/Design patterns**: Highlight key algorithms, data structures, or design patterns.
//     4. **Optimization/Improvements**: Suggest potential optimizations or improvements.
//     5. **Edge cases/Issues**: Identify possible issues or edge cases.
//     6. **Clarity**: Use clear, concise language suitable for all skill levels.

//     Structure your response with:
//     1. `ORIGINAL CODE` block.
//     2. **EXPLANATION**: Detailed breakdown.
//     3. **SUMMARY**: Brief conclusion.

//     For formatting:
//     - Use Gfm if necessary
//     - Use proper tabs spaces and indentation.
//     - Use single-line code blocks with `<code here>`.
//     - Use comments syntax of the programming language for comments in code blocks.
//     - Use multi-line blocks with:
//     ```<language>
//     <code here>
//     ```

//     "#;

// const USER_PROMPT_TEMPLATE: &str =
//     r#"
//         Context from prior conversations and uploaded files: {context}
//         New question or coding request: {user_prompt}
//     "#;

// pub fn register_routes(cfg: &mut web::ServiceConfig) {
//     cfg.service(chat_explain); // Register the correct route handler
// }

// #[post("/chat/explain")]
// pub async fn chat_explain(
//     data: web::Json<ChatExplainRequest>,
//     req: HttpRequest
// ) -> Result<HttpResponse, Error> {
//     let prompt = RefactorPrompt::new(SYSTEM_PROMPT, USER_PROMPT_TEMPLATE);

//     handle_request(
//         &prompt,
//         &data.prompt,
//         data.session_id.clone(),
//         Some(req),
//         RequestType::Explain
//     ).await
// }
