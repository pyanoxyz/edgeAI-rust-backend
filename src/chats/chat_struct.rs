
use actix_web::{error::InternalError, post, web, HttpRequest, HttpResponse, Error};
use crate::{request_type::RequestType, utils::handle_llm_response};
use serde_json::json;
use crate::session_manager::check_session;
use log::{debug, error};
use crate::authentication::authorization::is_request_allowed;
use serde::{Deserialize, Serialize};



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

pub async  fn handle_request(
    prompt: &RefactorPrompt,
    user_prompt: &str,
    session_id: Option<String>,
    req: Option<HttpRequest>,
) -> Result<HttpResponse, Error> {
    // Check session and extract user ID from the request
    let session_id = match check_session(session_id) {
        Ok(id) => id,
        Err(e) => {
            return Err(actix_web::error::ErrorInternalServerError(json!({
                "error": e.to_string()
            })));
        }
    };

    let full_user_prompt = prompt
        .user_prompt_template
        .replace("{context}", "")
        .replace("{user_prompt}", user_prompt);

    match req {
        Some(req) => {
            if let Ok(Some(user)) = is_request_allowed(req.clone()).await {
                debug!("Ok reached here");

                // Cloud LLM response with actual user ID
                handle_llm_response(
                    Some(req),
                    prompt.system_prompt,
                    &user_prompt,
                    &full_user_prompt,
                    &session_id,
                    &user.user_id,
                    RequestType::Refactor,
                )
                .await
            } else {
                // Local LLM response
                handle_llm_response(
                    None,
                    prompt.system_prompt,
                    &user_prompt,
                    &full_user_prompt,
                    &session_id,
                    "user_id",
                    RequestType::Refactor,
                )
                .await
            }
        }
        None => {
            // Local LLM response without user info
            handle_llm_response(
                None,
                prompt.system_prompt,
                &user_prompt,
                &full_user_prompt,
                &session_id,
                "user_id",
                RequestType::Refactor,
            )
            .await
        }
    }
}
