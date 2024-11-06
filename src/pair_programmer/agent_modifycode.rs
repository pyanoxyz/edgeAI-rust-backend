

use crate::pair_programmer::agent::Agent;
use async_trait::async_trait;

pub struct ModifyCodeAgent {
    user_prompt_with_context: String
}

impl ModifyCodeAgent {
    pub fn new(user_prompt_with_context: String) -> Self {
        ModifyCodeAgent {
            user_prompt_with_context
        }
    }
}

#[async_trait]
impl Agent for ModifyCodeAgent {
    // Implementing the required trait methods for PlannedAgent
    fn get_name(&self) -> String {
        let name: &str = "modify-code";
        return name.to_string()    }


    fn get_system_prompt(&self) -> String {
        let system_prompt = r#"
        Modify the code in RESPONSE based on the instructions in USER_PROMPT.
        Ensure the modified code remains coherent with prior steps in ALL_STEPS and references relevant details in CONTEXT_CODE to maintain consistency across the task.
        Based on this guidance, please implement the code for the current step, ensuring accuracy and coherence with the broader task and the codebase.

        Output Format:

        Code Implementation:
        [Your code here]

        Clarification Check:
        Before executing this step, if there are any ambiguous details or if further clarification is required, please prompt the user to confirm or clarify the specifics of the step.

        Important:

            Do not include any explanations, suggestions, or additional information outside of the code itself and absolutely necessary comments.
            Do not include any "Next Steps" in your response.
        "#;
        return system_prompt.to_string()
    }
    fn get_user_prompt_with_context(&self) -> String {
        self.user_prompt_with_context.clone()
    }
}