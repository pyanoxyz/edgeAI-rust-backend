use actix_web::{post, web, HttpRequest, HttpResponse, Error};
use crate::chats::chat_struct::{RefactorPrompt, handle_request};
use serde::{Deserialize, Serialize};
use crate::request_type::RequestType;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
    pub session_id: Option<String>,
}

const SYSTEM_PROMPT: &str = r#"
   You are an AI coding assistant specializing in programming questions, code generation, and multi-turn conversations. 
        Your responses should be concise, context-aware, and instruction-tuned, leveraging both past interactions 
        and user-provided data to offer relevant, step-by-step guidance.

        When offering code solutions:
        - Provide code examples or modifications to enhance clarity.
        - Use context to suggest optimizations or anticipate common issues.
        - Handle complex requests across multiple turns, remembering prior context.

        For formatting:
        - Use single-line code blocks with `<code here>`.
        - Use multi-line blocks with:
        <language>
        <code>
        Reflect, verify correctness, and explain concisely.
    "#;

const USER_PROMPT_TEMPLATE: &str = r#"
        Context from prior conversations and uploaded files: {context}
        New question or coding request: {user_prompt}
        Response should follow instruction-tuning principles.
    "#;


pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(chat); // Register the correct route handler
}

#[post("/chat")]
pub async fn chat(data: web::Json<ChatRequest>, req: HttpRequest) -> Result<HttpResponse, Error> {
        let prompt = RefactorPrompt::new(
            SYSTEM_PROMPT,
            USER_PROMPT_TEMPLATE,
        );
    
        handle_request(&prompt, &data.prompt, data.session_id.clone(), Some(req), RequestType::Chat).await
    }
