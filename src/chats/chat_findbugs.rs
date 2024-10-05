use actix_web::{ post, web, HttpRequest, HttpResponse, Error };
use crate::llm_stream::handle::stream_to_chat_client;
use serde::{ Deserialize, Serialize };
use crate::request_type::RequestType;
use std::sync::{Arc, Mutex};
use crate::session_manager::check_session;
use serde_json::json;
use super::utils::handle_stream_completion;


#[derive(Debug, Serialize, Deserialize)]
pub struct FindBugsRequest {
    pub prompt: String,
    pub session_id: Option<String>,
}

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(chat_find_bugs); // Register the correct route handler
}

#[post("/chat/find-bugs")]
pub async fn chat_find_bugs(data: web::Json<FindBugsRequest>, _req: HttpRequest) -> Result<HttpResponse, Error> {
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
        Analyze the code based on the guidelines provided in the system prompt. Identify any bugs, 
        issues, or potential improvements, and present your findings in the specified format.
        "#,
        context = "empty context",
        user_prompt =  &data.prompt
    );
    let system_prompt: &str =
        r#"
        You are an expert code reviewer and debugger specializing in identifying bugs, performance issues, and vulnerabilities. 
       Your task is to analyze the GIVEN CODE and CONTEXT, following these steps:
        1. **Code Review**: Examine the code for logical, syntax, or runtime errors, security vulnerabilities, and performance bottlenecks.
        2. **Contextual Analysis**: Use the CONTEXT for insights but prioritize the GIVEN CODE.
        3. **Detailed Issue Reporting**:
        - Description of the issue
        - Code location (line number or snippet)
        - Impact or potential consequence
        - Suggested fix or optimization

        4. **Edge Case Consideration**: Analyze potential edge cases.
        5. **Library-Specific Pitfalls**: Watch for errors related to libraries or frameworks.
        6. **Code Smells**: Identify anti-patterns or code smells.
        7. **Final Review**: If no major bugs are found, suggest minor improvements.

        Structure your response using:
        1. **Bug Description**
        2. **Location**
        3. **Impact**
        4. **Suggested Fix** (within triple backticks for code)

        For formatting:
        - Use Gfm if necessary
        - Use proper tabs spaces and indentation.
        - Use single-line code blocks with `<code here>`.
        - Use comments syntax of the programming language for comments in code blocks.
        - Use multi-line blocks with:
        ```<language>
        <code here>
        ```
        Provide any general recommendations at the end for improving quality or performance.

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
        handle_stream_completion(rx, accumulated_content, shared_session_id_clone, shared_prompt_clone, RequestType::FindBugs).await;
    });
    Ok(response)
}
