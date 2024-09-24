use actix_web::{post, web, HttpRequest, HttpResponse, Error};
use crate::chats::chat_struct::{RefactorPrompt, handle_request};
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
    pub session_id: Option<String>,
}

const SYSTEM_PROMPT: &str = r#"
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
    - DETECTED LANGUAGE: [Insert detected language here]
    - CHOSEN DOCSTRING FORMAT: [Briefly describe the docstring format you selected and why]
    - GENERATED DOCSTRING:
    ```[language]
    [Insert the generated docstring here, using appropriate syntax for the language]
    ```

    EXPLANATION:
    [Provide a brief explanation of the choices made in generating the docstring, including your reasoning for selecting the particular format and any notable decisions in documentation strategies.]

    **Additional Notes**:
    - Always enclose code snippets and docstrings within triple backticks (```) and include the appropriate language identifier for correct formatting.
    - Ensure that the docstring is clear, concise, and follows the best practices of the selected format.
    "#;

const USER_PROMPT_TEMPLATE: &str = r#"
        Context from prior conversations and uploaded files: {context}
        New question or coding request: {user_prompt}
    "#;


pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(chat_docstring); // Register the correct route handler
}

#[post("/chat/docstring")]
pub async fn chat_docstring(data: web::Json<ChatRequest>, req: HttpRequest) -> Result<HttpResponse, Error> {
        let prompt = RefactorPrompt::new(
            SYSTEM_PROMPT,
            USER_PROMPT_TEMPLATE,
        );
        handle_request(&prompt, &data.prompt, data.session_id.clone(), Some(req)).await
    }
