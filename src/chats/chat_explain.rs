use actix_web::{error::InternalError, post, web, HttpRequest, HttpResponse, Error};
use crate::{request_type::RequestType, utils::handle_llm_response};
use serde_json::json;
use crate::session_manager::check_session;
use log::{debug, error};
use crate::authentication::authorization::is_request_allowed;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatExplainRequest {
    pub prompt: String,
    pub session_id: Option<String>,
}

const SYSTEM_PROMPT: &str = r#"
   You are an expert code analyst specializing in breaking down code snippets and functions in a clear, step-by-step manner. \
    Follow these guidelines for a detailed explanation:

    1. **High-level overview**: Summarize the overall purpose of the code.
    2. **Step-by-step breakdown**: Analyze each significant part, explaining its functionality and purpose.
    3. **Algorithms/Design patterns**: Highlight key algorithms, data structures, or design patterns.
    4. **Optimization/Improvements**: Suggest potential optimizations or improvements.
    5. **Edge cases/Issues**: Identify possible issues or edge cases.
    6. **Clarity**: Use clear, concise language suitable for all skill levels.

    Structure your response with:
    1. `ORIGINAL CODE` block.
    2. **EXPLANATION**: Detailed breakdown.
    3. **SUMMARY**: Brief conclusion.

    Use triple backticks (```) for all code blocks.

    "#;

const USER_PROMPT_TEMPLATE: &str = r#"
        Context from prior conversations and uploaded files: {context}
        New question or coding request: {user_prompt}
    "#;


pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(chat_explain); // Register the correct route handler
}

#[post("/chat/explain")]
pub async fn chat_explain(data: web::Json<ChatExplainRequest>, req: HttpRequest) -> Result<HttpResponse, Error> {
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
            error!("chat explain request failed {:?}", e);
            let err_response = InternalError::from_response("Request failed", e).into();
            Err(err_response)
        }
    }
}
