use actix_web::{ HttpRequest, HttpResponse, Error };
use crate::{ request_type::RequestType, utils::local_llm_response, utils::remote_llm_response };
use serde_json::json;
use crate::session_manager::check_session;
use log::debug;
use crate::authentication::authorization::is_request_allowed;

#[derive(Debug)]
pub struct RefactorPrompt {
    pub system_prompt: &'static str,
    pub user_prompt_template: &'static str,
}

impl RefactorPrompt {
    pub fn new(system_prompt: &'static str, user_prompt_template: &'static str) -> Self {
        RefactorPrompt {
            system_prompt,
            user_prompt_template,
        }
    }
}
pub async fn handle_request(
    prompt: &RefactorPrompt,
    user_prompt: &str,
    session_id: Option<String>,
    req: Option<HttpRequest>,
    chat_request_type: RequestType
) -> Result<HttpResponse, Error> {
    // Check session and extract user ID from the request
    let session_id = match check_session(session_id) {
        Ok(id) => id,
        Err(e) => {
            return Err(
                actix_web::error::ErrorInternalServerError(
                    json!({
                "error": e.to_string()
            })
                )
            );
        }
    };

    let full_user_prompt = prompt.user_prompt_template
        .replace("{context}", "")
        .replace("{user_prompt}", user_prompt);

    // Raise error if req is None
    let req = match req {
        Some(req) => req,
        None => {
            return Err(
                actix_web::error::ErrorBadRequest(
                    json!({
                "error": "Request is required but not provided"
            })
                )
            );
        }
    };

    // Handle Cloud LLM or Local LLM responses based on user permission
    if let Ok(Some(user)) = is_request_allowed(req.clone()).await {
        debug!("Cloud LLM response for user ID: {}", user.user_id);

        remote_llm_response(
            prompt.system_prompt,
            user_prompt,
            &full_user_prompt,
            &session_id,
            &user.user_id,
            chat_request_type
        ).await.map_err(|e| {
            actix_web::error::ErrorInternalServerError(
                json!({
                "error": format!("LLM response error: {}", e)
            })
            )
        })
    } else {
        debug!("Local LLM response");

        local_llm_response(
            prompt.system_prompt,
            user_prompt,
            &full_user_prompt,
            &session_id,
            "user_id",
            chat_request_type
        ).await.map_err(|e| {
            actix_web::error::ErrorInternalServerError(
                json!({
                "error": format!("Local LLM response error: {}", e)
            })
            )
        })
    }
}

// TODO try adding this dynamically to all prompts
const FORMATTING_PROMPT: &str =
    r#"
    For formatting:
    - Use Gfm if necessary
    - Use proper tabs spaces and indentation.
    - Use single-line code blocks with `<code here>`.
    - Use comments syntax of the programming language for comments in code blocks.
    - Use multi-line blocks with:
    ```<language>
    <code here>
    ```
    "#;
