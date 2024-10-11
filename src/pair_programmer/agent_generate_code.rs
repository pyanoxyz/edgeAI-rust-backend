

use crate::pair_programmer::agent::Agent;
use async_trait::async_trait;

pub struct GenerateCodeAgent {
    user_prompt: String,
    prompt_with_context: String,
}

impl GenerateCodeAgent {
    pub fn new(user_prompt: String, prompt_with_context: String) -> Self {
        GenerateCodeAgent {
            user_prompt,
            prompt_with_context,
        }
    }
}

#[async_trait]
impl Agent for GenerateCodeAgent {
    // Implementing the required trait methods for PlannedAgent
    fn get_name(&self) -> String {
        let name: &str = "system-code";
        return name.to_string()    }

    fn get_user_prompt(&self) -> String {
        self.user_prompt.clone()
    }

    fn get_system_prompt(&self) -> String {
        let system_prompt = r#"
        You are an AI pair programmer executing steps in a complex programming problem. 
        Your task is to generate code for the current step while maintaining context from recent steps

        Your Approach:
        1. Focus on the current step and the immediate context provided.
        2. Generate code that builds upon recent work without repeating unnecessary steps.
        3. Maintain consistency with established coding patterns and styles.
        4. Prepare for upcoming steps when appropriate.

        Output Format:

        Code Implementation
        [Your code here]

        Explanation: [Your detailed explanation]
        Remember to stay focused on the current_step while maintaining awareness of the overall_context and progress of the project including all_steps
        and steps_executed_so_far.
        Also consider the recent_discussion by the user before executing the step.
        If the steps has already been excuted in the previous steps, skip repeating code.
        If you think that the step is redudant, Feel free to skip the step.
        Do not include any explanations, suggestions, or additional information outside of the code itself and absolutely necessary comments.
        Do not include any 'Next Steps' in your response.
        "#;
        return system_prompt.to_string()
    }

    fn get_prompt_with_context(&self) -> String {
        self.prompt_with_context.clone()
    }
}