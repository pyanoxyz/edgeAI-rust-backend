use actix_web::{ post, web, HttpRequest, HttpResponse, Error };
use crate::llm_stream::handle::stream_to_chat_client;
use serde::{ Deserialize, Serialize };
use super::chat_types::RequestType;
use std::sync::{Arc, Mutex};
use crate::session_manager::check_session;
use serde_json::json;
use super::utils::handle_stream_completion;
use crate::context::make_context::make_context;
use reqwest::Client;


#[derive(Debug, Serialize, Deserialize)]
pub struct ChatExplainRequest {
    pub prompt: String,
    pub session_id: Option<String>,
}

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(chat_explain); // Register the correct route handler
}

#[post("/chat/explain")]
pub async fn chat_explain(data: web::Json<ChatExplainRequest>, client: web::Data<Client>, _req: HttpRequest) -> Result<HttpResponse, Error> {
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
    
    let accumulated_content = Arc::new(Mutex::new(String::new()));
    let accumulated_content_clone = Arc::clone(&accumulated_content);

    // Wrap your data in a Mutex or RwLock to ensure thread safety
    let shared_session_id = Arc::new(Mutex::new(session_id.clone()));
    let shared_session_id_clone: Arc<Mutex<String>> = Arc::clone(&shared_session_id);

    // Wrap your data in a Mutex or RwLock to ensure thread safety
    let shared_prompt = Arc::new(Mutex::new(data.prompt.clone()));
    let shared_prompt_clone = Arc::clone(&shared_prompt);
    let context = make_context(&session_id, &data.prompt, 3).await?;

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    
    
    let prompt_with_context = format!(r#"
        Context from prior conversations and uploaded files (separated by '----------CONTEXT----------'): 
        {context}
        New question or coding request: {user_prompt}

        Please provide your response following instruction-tuning principles.
        "#,
            context = context,
            user_prompt = &data.prompt
        );

    let system_prompt: &str = r#"
        You are an expert code analyst. Provide a step-by-step breakdown of code snippets, following these steps:

        1. **Overview**: Summarize the purpose of the code.
        2. **Breakdown**: Explain each significant part of the code.
        3. **Algorithms/Patterns**: Highlight key algorithms, data structures, or design patterns.
        4. **Optimizations**: Suggest potential improvements.
        5. **Edge cases**: Identify possible edge cases or issues.
        6. **Clarity**: Ensure your explanation is easy to understand.

        Structure:
        1. `ORIGINAL CODE`: Display the code.
        2. **EXPLANATION**: Provide a detailed breakdown.
        3. **SUMMARY**: Conclude with a brief summary.

        Formatting:
        - Use GFM when needed.
        - Ensure proper indentation, comments, and single/multi-line code blocks.
        "#;

    let response = stream_to_chat_client(
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
        handle_stream_completion(rx, accumulated_content, shared_session_id_clone, shared_prompt_clone, RequestType::Explain).await;
    });

    Ok(response)

}
