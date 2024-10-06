use actix_web::{ post, web, HttpRequest, HttpResponse, Error };
use crate::llm_stream::handle::stream_to_chat_client;
use serde::{ Deserialize, Serialize };
use super::chat_types::RequestType;
use std::sync::{Arc, Mutex};
use crate::session_manager::check_session;
use serde_json::json;
use super::utils::handle_stream_completion;


#[derive(Debug, Serialize, Deserialize)]
pub struct ChatExplainRequest {
    pub prompt: String,
    pub session_id: Option<String>,
}


pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(chat_explain); // Register the correct route handler
}



#[post("/chat/explain")]
pub async fn chat_explain(data: web::Json<ChatExplainRequest>, _req: HttpRequest) -> Result<HttpResponse, Error> {
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



    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    
    
    //TODO: Add context
    let prompt_with_context = format!(
        r#"
        Context from prior conversations and uploaded files: {context}
        New question or coding request: {user_prompt}
        "#,
        context = "empty context",
        user_prompt =  &data.prompt
    );
    let system_prompt: &str =
        r#"
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

    let response = stream_to_chat_client(
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
