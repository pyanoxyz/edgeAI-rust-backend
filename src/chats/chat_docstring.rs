use actix_web::{ post, web, HttpRequest, HttpResponse, Error };
use crate::llm_stream::handle::stream_to_chat_client;
use serde::{ Deserialize, Serialize };
use crate::request_type::RequestType;
use std::sync::{Arc, Mutex};
use crate::session_manager::check_session;
use serde_json::json;
use super::utils::handle_stream_completion;

#[derive(Debug, Serialize, Deserialize)]
pub struct DocStringRequest {
    pub prompt: String,
    pub session_id: Option<String>,
}

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(chat_docstring); // Register the correct route handler
}


#[post("/chat/docstring")]
pub async fn chat_docstring(data: web::Json<DocStringRequest>, _req: HttpRequest) -> Result<HttpResponse, Error> {
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
             You are an expert programmer specializing in creating comprehensive and clear documentation. Your task is to generate 
            a docstring for the provided code snippet or function. You will also decide the most appropriate docstring format for the detected language.

            **Task Flow:**
            1. **Language Detection**: First, accurately detect the programming language of the provided code.
            2. **Docstring Format Selection**: Based on the detected language and your analysis, choose the most appropriate and widely used docstring format.
            3. **Code Analysis**:
            - Identify the function/method purpose
            - Extract and describe parameters (name, type, description)
            - Describe the return value (type, description)
            - Identify any exceptions raised (if applicable)
            - Provide examples of usage if they will aid understanding
            4. **Docstring Generation**:
            - Generate a comprehensive docstring that includes:
                - A concise description of the function/method's purpose
                - Detailed parameter descriptions
                - A description of the return value
                - Any exceptions that may be raised
                - Usage examples if they would be helpful
            5. **Optional**: If the function contains complex logic, briefly explain the algorithm or approach used.
            6. Include any important notes, warnings, or caveats about using the function.

            **Presentation Format**:
            - CHOSEN DOCSTRING FORMAT: [Briefly describe the docstring format you selected and why]
            - GENERATED DOCSTRING:
            ```<language>
            [Insert the generated docstring here, using appropriate syntax for the language]
            ```


            EXPLANATION:
            [Provide a brief explanation of the choices made in generating the docstring, including your reasoning for selecting the particular format and any notable decisions in documentation strategies.]

            **Additional Notes**:
            - Ensure that the docstring is clear, concise, and follows the best practices of the selected format.
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
        handle_stream_completion(rx, accumulated_content, shared_session_id_clone, shared_prompt_clone, RequestType::DocString).await;
    });

    Ok(response)

}
