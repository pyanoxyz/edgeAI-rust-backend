use actix_web::{ post, web, HttpRequest, HttpResponse, Error };
use crate::llm_stream::handle::stream_to_chat_client;
use serde::{ Deserialize, Serialize };
use crate::request_type::RequestType;
use std::sync::{Arc, Mutex};
use crate::session_manager::check_session;
use serde_json::json;
use super::utils::handle_stream_completion;

#[derive(Debug, Serialize, Deserialize)]
pub struct TestCasesRequest {
    pub prompt: String,
    pub session_id: Option<String>,
}

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(chat_testcases); // Register the correct route handler
}

#[post("/chat/tests-cases")]
pub async fn chat_testcases(data: web::Json<TestCasesRequest>, _req: HttpRequest) -> Result<HttpResponse, Error> {
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
        You are an expert software tester. Create a comprehensive test suite for the given code snippet or function. Follow this process:
        1. Analyze the code:
            - Identify the programming language and appropriate testing framework.
            - Determine key functionalities and potential edge cases.
        
        2. Plan test strategy:
            - Consider normal cases, edge cases, error handling, and performance aspects.
            - Reflect on potential challenges in testing this specific code.

        3. Design test cases:
            - Develop a mix of test types (normal, edge, error, performance).
            - Consider how to mock dependencies if necessary.

        4. Implement test suite:
            - Write test code using the chosen framework.
            - Include setup/teardown methods if needed.

        5. Review and refine:
            - Assess test coverage and effectiveness.
            - Consider any additional tools or techniques that could enhance testing.

        Present your results as follows:

        THOUGHT PROCESS:
        [Brief outline of your analysis and test strategy]

        TEST SUITE:
        ```[language]
        [Complete test suite code, including imports and all test methods]
        EXPLANATION:
        [Concise explanation of key test cases and testing approach]
        REFLECTION:
        [Brief reflection on test coverage, challenges, and potential improvements]
        
        Ensure each section is clear and precise, focusing on improvements and efficiency.
        Prioritize clarity and comprehensiveness in your test suite.

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
        handle_stream_completion(rx, accumulated_content, shared_session_id_clone, shared_prompt_clone, RequestType::TestCases).await;
    });
    Ok(response)
}
