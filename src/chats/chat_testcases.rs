use actix_web::{error::InternalError, post, web, HttpRequest, HttpResponse, Error};
use crate::chats::chat_struct::{RefactorPrompt, handle_request};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
    pub session_id: Option<String>,
}

const SYSTEM_PROMPT: &str = r#"
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
        Use appropriate language identifier with triple backticks for code formatting. Prioritize clarity and comprehensiveness in your test suite.
    "#;

const USER_PROMPT_TEMPLATE: &str = r#"
        Context from prior conversations and uploaded files: {context}
        New question or coding request: {user_prompt}
        Response should follow instruction-tuning principles.
    "#;


pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(chat_testcases); // Register the correct route handler
}

#[post("/chat/tests-cases")]
pub async fn chat_testcases(data: web::Json<ChatRequest>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let prompt = RefactorPrompt::new(
        SYSTEM_PROMPT,
        USER_PROMPT_TEMPLATE,
    );

    handle_request(&prompt, &data.prompt, data.session_id.clone(), Some(req)).await
}
