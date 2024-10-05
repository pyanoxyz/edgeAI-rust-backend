
use actix_web::{ post, web, HttpRequest, HttpResponse, Error };
use crate::llm_stream::handle::stream_to_chat_client;
use serde::{ Deserialize, Serialize };
use crate::request_type::RequestType;
use std::sync::{Arc, Mutex};
use crate::session_manager::check_session;
use serde_json::json;
use super::utils::handle_stream_completion;

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
pub async fn chat_infill(data: web::Json<InfillRequest>, _req: HttpRequest) -> Result<HttpResponse, Error> {
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
        r#"
        Please suggest ONLY the new code or comments that would logically follow after this context:
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

    let system_prompt: &str =
        r#"
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
        handle_stream_completion(rx, accumulated_content, shared_session_id_clone, shared_prompt_clone, RequestType::Infill).await;
    });

    Ok(response)

}
