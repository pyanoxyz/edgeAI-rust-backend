

use crate::pair_programmer::agent::Agent;
use async_trait::async_trait;

pub struct RethinkerAgent {
    user_prompt: String,
    prompt_with_context: String,
}

impl RethinkerAgent {
    pub fn new(user_prompt: String, prompt_with_context: String) -> Self {
        RethinkerAgent {
            user_prompt,
            prompt_with_context,
        }
    }
}
#[async_trait]
impl Agent for RethinkerAgent {
    // Implementing the required trait methods for PlannedAgent
    fn get_name(&self) -> String {
        let name: &str = "system-code";
        return name.to_string()    }

    fn get_user_prompt(&self) -> String {
        self.user_prompt.clone()
    }

    fn get_system_prompt(&self) -> String {
        let system_prompt = r#"
            You are a problem-solving assistant responsible for updating the current step of an ongoing solution based on the user’s feedback from recent chats.

            Instructions:
            Context Awareness: Review the entire plan, the steps executed so far, and the chats exchanged between the user and the LLM. Use this context to revise the current step's goal or approach.

            User Feedback Integration: Modify the current step according to the users chats, ensuring that any changes reflect their requests and suggestions precisely. 
            Do not introduce new information unless requested by the user.

            Tool Selection: For executing changes, continue using only the designated tools (llm, generate-code, or system-code). 
            Use the tools in the same structured format, focusing on modifying the current step's goal.

            Strict Formatting: Adhere to the original output format. No extra explanations or comments—only changes as per the users request should be reflected.

            Tool Guidelines:
            llm: For reasoning or adjusting the current task.
            generate-code: For generating any new code needed to address changes.
            system-code: For executing Unix commands or handling file operations.

            Output Format (Strict):
            Step Modification: Clearly describe the updated goal of the current step.
            Tool: [llm | generate-code | system-code]  
            Action: <function=[chosen_tool]>{{"problem": "Updated problem or goal", "language": "Programming language"}}</function>

            Ensure:
            - The user feedback is fully incorporated into the step.
            - Only specified tools are used for task execution.
            - No general-purpose advice or extra information—solve the problem based on the feedback.
            - Strict adherence to the format, without deviation.

        "#;
        return system_prompt.to_string()
    }

    fn get_prompt_with_context(&self) -> String {
        self.prompt_with_context.clone()
    }
}