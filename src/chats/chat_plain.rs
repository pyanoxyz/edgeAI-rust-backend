use actix_web::{error::InternalError, post, web, HttpRequest, HttpResponse, Error};
use crate::{request_type::RequestType, utils::handle_llm_response};
use serde_json::json;
use crate::session_manager::check_session;
use log::{debug, error};
use crate::authentication::authorization::is_request_allowed;
use serde::{Deserialize, Serialize};

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
    let user_prompt = &data.prompt;
    // Check session and extract user ID from the request
    let session_id = match check_session(data.session_id.clone()) {
        Ok(id) => id,
        Err(e) => {
            return Err(actix_web::error::ErrorInternalServerError(json!({
                "error": e.to_string()
            })));
        }
    };

    let full_user_prompt = USER_PROMPT_TEMPLATE
        .replace("{context}", "")
        .replace("{user_prompt}", user_prompt);

    match is_request_allowed(req.clone()).await {
        Ok(Some(user)) => {
            debug!("Ok reached here");

            // Cloud LLM response with actual user ID
            handle_llm_response(
                Some(req),
                SYSTEM_PROMPT,
                &user_prompt,
                &full_user_prompt,
                &session_id,
                &user.user_id, // Using actual user ID from request
                RequestType::Infill,
            )
            .await
        }
        Ok(None) => {
            debug!("Local llm being executed with session_id {}", session_id);

            // Local LLM response (no user info available)
            handle_llm_response(
                None,
                SYSTEM_PROMPT,
                &user_prompt,
                &full_user_prompt,
                &session_id,
                "user_id", // Placeholder user_id for local execution
                RequestType::Infill,
            )
            .await
        }
        Err(e) => {
            // Error handling with InternalError response
            error!("chat  request failed {:?}", e);
            let err_response = InternalError::from_response("Request failed", e).into();
            Err(err_response)
        }
    }
}
