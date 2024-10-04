use actix_web::{ post, web, HttpRequest, HttpResponse, Error };
use crate::chats::chat_struct::{ RefactorPrompt, handle_request };
use serde::{ Deserialize, Serialize };
use crate::request_type::RequestType;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatExplainRequest {
    pub prompt: String,
    pub session_id: Option<String>,
}

const SYSTEM_PROMPT: &str =
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

const USER_PROMPT_TEMPLATE: &str =
    r#"
        Context from prior conversations and uploaded files: {context}
        New question or coding request: {user_prompt}
        Analyze the code based on the guidelines provided in the system prompt. Identify any bugs, 
        issues, or potential improvements, and present your findings in the specified format.
    "#;

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(chat_find_bugs); // Register the correct route handler
}

#[post("/chat/find-bugs")]
pub async fn chat_find_bugs(
    data: web::Json<ChatExplainRequest>,
    req: HttpRequest
) -> Result<HttpResponse, Error> {
    let prompt = RefactorPrompt::new(SYSTEM_PROMPT, USER_PROMPT_TEMPLATE);

    handle_request(
        &prompt,
        &data.prompt,
        data.session_id.clone(),
        Some(req),
        RequestType::FindBugs
    ).await
}
