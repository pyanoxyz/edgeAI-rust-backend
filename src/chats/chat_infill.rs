use actix_web::{error::InternalError, get, post, web, HttpRequest, HttpResponse, Error};
use std::env;
use crate::{request_type::RequestType, utils::handle_llm_response};
use serde_json::json;
use crate::session_manager::check_session;
use dotenv::dotenv;
use log::{info, debug, error};
use crate::authentication::authorization::is_request_allowed;
use serde::{Deserialize, Serialize};



#[derive(Debug, Serialize, Deserialize)]
pub struct InfillRequest {
    pub code_before: String,
    pub code_after: String,
    pub session_id: Option<String>,
}

const SYSTEM_PROMPT: &str = r#"
    You are an expert code assistant specializing in predicting and generating code based on context. Analyze the given code context, 
    determine the programming language, and suggest the most appropriate continuation. Adhere strictly to the following instructions:
    1. Examine the CODE BEFORE section carefully and determine the programming language. Focus on understanding the code structure and logic.
    2. Generate only new code that logically follows the CODE BEFORE section.
    3. Do NOT generate any comments, explanations, or extra text. Your output should contain only executable code.
    4. Avoid repeating any part of the CODE BEFORE section. If CODE AFTER is provided, ensure the new code fits logically between CODE BEFORE and CODE AFTER.
    5. If CODE AFTER is empty, generate a logically complete structure (function, class, etc.) following best practices, ensuring correctness and optimal performance.
    6. Prioritize readability and maintain consistent code style and indentation.
    7. Re-assess incomplete patterns in the CODE BEFORE section and ensure the output forms a syntactically correct and executable code snippet.
    8. Apply patterns for progressive task-solving by incrementally refining the solution and following a "least-to-most" decompositional approach when needed (as described in the research paper).
    "#;

const USER_PROMPT_TEMPLATE: &str = r#"
    Please suggest ONLY the new code or comments that would logically follow after this context:
    CODE BEFORE:
    {code_before}

    CODE AFTER:
    {code_after}
"#;


pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(infill_code); // Register the correct route handler
}

#[post("/chat/infill")]
pub async fn infill_code(data: web::Json<InfillRequest>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let code_before = &data.code_before;
    let code_after = &data.code_after;
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
        .replace("{code_before}", code_before)
        .replace("{code_after}", code_after);

    match is_request_allowed(req.clone()).await {
        Ok(Some(user)) => {
            debug!("Ok reached here");

            // Cloud LLM response with actual user ID
            handle_llm_response(
                Some(req),
                SYSTEM_PROMPT,
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
                &full_user_prompt,
                &session_id,
                "user_id", // Placeholder user_id for local execution
                RequestType::Infill,
            )
            .await
        }
        Err(e) => {
            // Error handling with InternalError response
            let err_response = InternalError::from_response("Request failed", e).into();
            Err(err_response)
        }
    }
}
