
use actix_web::{ post, web, HttpRequest, HttpResponse, Error };
use crate::llm_stream::handle::stream_to_chat_client;
use serde::{ Deserialize, Serialize };
use crate::request_type::RequestType;
use std::sync::{Arc, Mutex};
use crate::session_manager::check_session;
use serde_json::json;
use super::utils::handle_stream_completion;

#[derive(Debug, Serialize, Deserialize)]
pub struct RefactorRequest {
    pub prompt: String,
    pub session_id: Option<String>,
}

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(chat_refactor); // Register the correct route handler
}

#[post("/chat/refactor")]
pub async fn chat_refactor(data: web::Json<RefactorRequest>, _req: HttpRequest) -> Result<HttpResponse, Error> {
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
        Response should follow instruction-tuning principles.
        "#,
        context = "empty context",
        user_prompt =  &data.prompt
    );
    let system_prompt: &str =
            r#"
            You are an expert software engineer specializing in code refactoring. 
            Use **re-reading and reflection** to improve the quality, readability, and efficiency of the given code.

            Your Approach:
            1. **Re-read the code** thoroughly, identifying areas for improvement.
            2. Reflect on multiple refactoring strategies, prioritizing readability, performance, maintainability, and design principles.
            3. **Refactor the code** using a clear strategy that builds upon previous steps.
            4. **Review and refine** your changes, ensuring alignment with best practices.

            Present your results as follows:

            **THOUGHT PROCESS**:
            [Brief outline of your analysis and decision-making process after re-reading]

            **REFACTORED CODE**:
            ```[Insert refactored code here]```

            **EXPLANATION**:
            [Concise explanation of key changes and their benefits]

            **REFLECTION**:
            [Brief reflection on the refactoring's impact, any trade-offs, and whether re-reading improved your reasoning]

            Ensure each section is clear and precise, focusing on improvements and efficiency.

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
        handle_stream_completion(rx, accumulated_content, shared_session_id_clone, shared_prompt_clone, RequestType::Refactor).await;
    });

    Ok(response)

}
