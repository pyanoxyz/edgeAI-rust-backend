use actix_web::{post, web, HttpRequest, HttpResponse, Error};
use crate::chats::chat_struct::{RefactorPrompt, handle_request};
use serde::{Deserialize, Serialize};
use crate::request_type::RequestType;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
    pub session_id: Option<String>,
}

const SYSTEM_PROMPT: &str = r#"
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

        Use triple backticks to format code blocks. Ensure each section is clear and precise, focusing on improvements and efficiency.
    "#;

const USER_PROMPT_TEMPLATE: &str = r#"
        Context from prior conversations and uploaded files: {context}
        New question or coding request: {user_prompt}
        Response should follow instruction-tuning principles.
    "#;


pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(chat_refactor); // Register the correct route handler
}

#[post("/chat/refactor")]
pub async fn chat_refactor(data: web::Json<ChatRequest>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let prompt = RefactorPrompt::new(
        SYSTEM_PROMPT,
        USER_PROMPT_TEMPLATE,
    );

    handle_request(&prompt, &data.prompt, data.session_id.clone(), Some(req), RequestType::Refactor).await
}
