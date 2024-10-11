
use actix_web::{ post, web, HttpRequest, HttpResponse, Error };
use crate::llm_stream::handle::stream_to_chat_client;
use serde::{ Deserialize, Serialize };
use super::chat_types::RequestType;
use std::sync::{Arc, Mutex};
use crate::session_manager::check_session;
use serde_json::json;
use super::utils::handle_stream_completion;
use reqwest::Client;

#[derive(Debug, Serialize, Deserialize)]
pub struct InfillRequest {
    pub code_before: String,
    pub code_after: String,
    pub session_id: Option<String>,
}

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(chat_infill); // Register the correct route handler
}

#[post("/chat/infill")]
pub async fn chat_infill(data: web::Json<InfillRequest>, client: web::Data<Client>, _req: HttpRequest) -> Result<HttpResponse, Error> {
    let session_id = match check_session(data.session_id.clone()) {
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
    //     let code_before = &data.code_before;
//     let code_after = &data.code_after;
    
    let accumulated_content = Arc::new(Mutex::new(String::new()));
    let accumulated_content_clone = Arc::clone(&accumulated_content);

    // Wrap your data in a Mutex or RwLock to ensure thread safety
    let shared_session_id = Arc::new(Mutex::new(session_id.clone()));
    let shared_session_id_clone: Arc<Mutex<String>> = Arc::clone(&shared_session_id);

    let prompt_with_context = format!(
        r#":
        CODE BEFORE:
        {code_before}

        CODE AFTER:
        {code_after}
            "#,
        code_before =  &data.code_before,
        code_after = &data.code_after
    );

    // Wrap your data in a Mutex or RwLock to ensure thread safety
    let shared_prompt = Arc::new(Mutex::new(prompt_with_context.clone()));
    let shared_prompt_clone = Arc::clone(&shared_prompt);

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    
    //TODO: Add context

    let system_prompt: &str = r#"
    You are an expert code assistant. Based on CODE BEFORE and CODE AFTER, generate only the missing code in between. Follow these rules:

    1. Output the executable code that logically fits between CODE BEFORE and CODE AFTER.
    2. MUST not repeat any part of CODE BEFORE. 
    3. If CODE AFTER is empty, generate a complete, logically correct continuation beyond CODE BEFORE.
    4. Do not stop prematurely; generate enough code to complete the function logically.
    5. Ensure the generated code maintains consistent style, syntax, and indentation.
    "#;

    let response = stream_to_chat_client(
        RequestType::Infill,
        &client,
        &session_id,
        system_prompt,
        &prompt_with_context,
        accumulated_content_clone,
        tx,
    ).await?;
    // Spawn a separate task to handle the stream completion
    // Ensure the main async task is spawned correctly
    tokio::spawn(async move {
        handle_stream_completion(rx, accumulated_content, shared_session_id_clone, shared_prompt_clone, RequestType::Infill).await;
    });
    Ok(response)
}
